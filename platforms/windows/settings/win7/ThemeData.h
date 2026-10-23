#pragma once

#include "PreferencesStore.h"
#include <filesystem>
#include <vector>

namespace msime::settings::win7 {

namespace theme_data {
using Api = char* (*)(const uint8_t*, size_t);
inline Json response(char* raw) {
  auto owned = msime::host_api::own_string(raw);
  return owned ? Json::parse(owned.get(), nullptr, false) : Json();
}
inline Json call(Api api, const Json& request) {
  const auto body = request.dump();
  return response(api(reinterpret_cast<const uint8_t*>(body.data()), body.size()));
}
inline std::optional<Json> value(const Json& response) {
  if (!response.is_object() || !response.contains("ok") || response.at("ok") != true || !response.contains("value"))
    return std::nullopt;
  return response.at("value");
}
struct Image {
  uint32_t width = 0;
  uint32_t height = 0;
  std::vector<uint8_t> bgra;
  static int32_t accept(void* context, const uint8_t* rgba, size_t length, uint32_t width, uint32_t height) noexcept {
    try {
      if (!width || !height || width > 2048 || height > 2048 || length != static_cast<size_t>(width) * height * 4) return 0;
      auto& image = *static_cast<Image*>(context);
      image.bgra.resize(length);
      // GDI AlphaBlend 接受预乘 BGRA；回调内复制，不保留 Rust 借用指针。
      for (size_t index = 0; index < length; index += 4) {
        const auto alpha = rgba[index + 3];
        image.bgra[index] = static_cast<uint8_t>((rgba[index + 2] * alpha + 127) / 255);
        image.bgra[index + 1] = static_cast<uint8_t>((rgba[index + 1] * alpha + 127) / 255);
        image.bgra[index + 2] = static_cast<uint8_t>((rgba[index] * alpha + 127) / 255);
        image.bgra[index + 3] = alpha;
      }
      image.width = width; image.height = height;
      return 1;
    } catch (...) { return 0; }
  }
};
struct Preview {
  Json resolved;
  Json package = Json::object();
  Image background;
  Image decoration;
  bool dark = false;
  bool horizontal = false;
  bool image_failed = false;
};
inline bool system_dark() {
  DWORD light = 1, length = sizeof(light);
  return RegGetValueW(HKEY_CURRENT_USER, L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
      L"AppsUseLightTheme", RRF_RT_REG_DWORD, nullptr, &light, &length) == ERROR_SUCCESS && !light;
}
inline bool candidate_dark(const Json& preferences) {
  const auto mode = preferences.value("candidate_theme", std::string("follow"));
  if (mode != "follow") return mode == "dark";
  const auto theme = preferences.value("theme", std::string("system"));
  return theme == "dark" || (theme == "system" && system_dark());
}
inline Json catalog(const std::filesystem::path& root) {
  const auto directory = root.u8string();
  return response(msime_client_skin_catalog(reinterpret_cast<const uint8_t*>(directory.data()), directory.size()));
}
inline Image image(const std::filesystem::path& root, const std::string& id, const std::string& relative, bool& failed) {
  Image result;
  if (relative.empty()) return result;
  const auto resource = value(call(msime_client_skin_resource,
      {{"directory", root.u8string()}, {"id", id}, {"relative", relative}, {"kind", "image"}}));
  if (resource && resource->is_object() && resource->contains("bytes") && resource->at("bytes").is_array()) {
    const auto bytes = resource->at("bytes").get<std::vector<uint8_t>>();
    if (msime_client_skin_image_decode(bytes.data(), bytes.size(), Image::accept, &result) == 1) return result;
  }
  failed = true;
  return {};
}
inline Preview preview(const Json& preferences, const std::filesystem::path& root, int mode) {
  Preview result;
  result.dark = mode == 0 ? candidate_dark(preferences) : mode == 2;
  result.horizontal = preferences.value("candidate_layout", std::string("vertical")) == "horizontal";
  auto custom = preferences.value("custom_theme", Json::object());
  if (custom.contains("candidate_colors") && custom.at("candidate_colors").is_object()) {
    // 与候选宿主相同：Windows 的编号色和边框色跟随主题，不使用旧配置中的覆盖。
    custom["candidate_colors"].erase("number"); custom["candidate_colors"].erase("border");
  }
  const auto resolved = value(call(msime_client_resolve_theme,
      {{"global_theme", preferences.value("global_theme", std::string("system"))},
       {"custom_theme", std::move(custom)}, {"dark", result.dark},
       {"layout", result.horizontal ? "horizontal" : "vertical"}, {"skins_directory", root.u8string()}}));
  if (!resolved || !resolved->is_object()) return result;
  result.resolved = *resolved;
  if (resolved->contains("appearance") && resolved->at("appearance").is_string()) result.dark = resolved->at("appearance") == "dark";
  if (resolved->contains("candidate_skin") && resolved->at("candidate_skin").is_string()) {
    const auto id = resolved->at("candidate_skin").get<std::string>();
    const auto package = value(call(msime_client_skin_package, {{"directory", root.u8string()}, {"id", id}}));
    if (package && package->is_object()) {
      result.package = *package;
      if (package->contains("background") && package->at("background").is_object())
        result.background = image(root, id, package->at("background").value("image", std::string{}), result.image_failed);
      if (package->contains("decorationImage") && package->at("decorationImage").is_string())
        result.decoration = image(root, id, package->at("decorationImage").get<std::string>(), result.image_failed);
    }
  }
  return result;
}
} // namespace theme_data
} // namespace msime::settings::win7
