#pragma once

#include "msime_client.h"
#include "../../../common/HostApiString.h"

#include <nlohmann/json.hpp>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace msime::settings::legacy {

using Json = nlohmann::json;

struct Snapshot {
  Json document;
  uint64_t revision() const { return document.at("revision").get<uint64_t>(); }
  const Json& preferences() const { return document.at("preferences"); }
};

enum class Outcome { loaded, saved, conflict, storage, invalid_response, invalid_change };

struct Result {
  Outcome outcome = Outcome::storage;
  std::optional<Snapshot> snapshot;
};

struct Change {
  std::vector<std::string> path;
  Json value;
};

// 只转换 C ABI 的文档与所有权；默认值、版本过滤、校验和持久化仍归共享 Rust。
class PreferencesStore {
public:
  explicit PreferencesStore(std::string directory) : directory_(std::move(directory)) {}

  Result load() const {
    return response(msime_client_load_preferences(bytes(directory_), directory_.size()), Outcome::loaded);
  }

  Result save(const Snapshot& previous, const std::vector<Change>& changes) const {
    if (!valid(previous.document) || changes.empty()) return {Outcome::invalid_change, std::nullopt};
    Json next = previous.document;
    for (const auto& change : changes) {
      if (change.path.empty()) return {Outcome::invalid_change, std::nullopt};
      Json* parent = &next.at("preferences");
      // 修改明确存在的字段，不能因拼错路径而补造一棵配置树。
      for (const auto& segment : change.path) {
        if (!parent->is_object()) return {Outcome::invalid_change, std::nullopt};
        const auto child = parent->find(segment);
        if (child == parent->end()) return {Outcome::invalid_change, std::nullopt};
        parent = &*child;
      }
      *parent = change.value;
    }
    const auto body = next.dump();
    return response(msime_client_save_preferences(bytes(directory_), directory_.size(), previous.revision(),
        bytes(body), body.size()), Outcome::saved);
  }

private:
  static const uint8_t* bytes(const std::string& value) {
    return reinterpret_cast<const uint8_t*>(value.data());
  }

  static bool valid(const Json& value) {
    if (!value.is_object()) return false;
    const auto version = value.find("format_version");
    const auto revision = value.find("revision");
    const auto preferences = value.find("preferences");
    return version != value.end() && version->is_number_unsigned() && *version == 1 &&
        revision != value.end() && revision->is_number_unsigned() &&
        preferences != value.end() && preferences->is_object();
  }

  static Result response(char* raw, Outcome success) {
    auto owned = msime::host_api::own_string(raw);
    if (!owned) return {Outcome::invalid_response, std::nullopt};
    const auto body = Json::parse(owned.get(), nullptr, false);
    if (!body.is_object()) return {Outcome::invalid_response, std::nullopt};
    const auto ok = body.find("ok");
    if (ok == body.end() || !ok->is_boolean()) return {Outcome::invalid_response, std::nullopt};
    if (!ok->get<bool>()) {
      const auto error = body.find("error");
      // 此 C ABI 返回 PreferencesError 的描述，尚未提供独立错误码字段。
      return {error != body.end() && *error == "preferences changed; reload before saving" ?
          Outcome::conflict : Outcome::storage, std::nullopt};
    }
    const auto value = body.find("value");
    if (value == body.end() || !valid(*value)) return {Outcome::invalid_response, std::nullopt};
    return {success, Snapshot{*value}};
  }

  std::string directory_;
};

} // namespace msime::settings::legacy
