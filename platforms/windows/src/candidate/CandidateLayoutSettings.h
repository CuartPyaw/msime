#pragma once
#include <nlohmann/json.hpp>
#include <optional>
#include <string>

namespace msime::windows {
struct CandidateLayoutSettings {
  bool horizontal = false;
  bool show_preedit = true;
  // The shared `wubi_code_hint`, which is on when the document does not say.
  bool wubi_code_hint = true;
  // 共享偏好 `show_app_logo`：首行左端画不画水杉 logo。新装默认关，文档里没有这个键时也按关处理，和 macOS 一致。
  bool show_app_logo = false;
  // 横排候选为释义预留几行（0 到 2）：打开候选翻译或离线英文释义时按目标语言数预留，最多两行，和 macOS 的 reservedGlossHeightForFont 一样。
  unsigned reserved_gloss_lines = 0;
  // The switches travel in one atomic value between monitor and UI threads.
  constexpr unsigned encode() const {
    return (horizontal ? 1u : 0u) | (show_preedit ? 2u : 0u) | (wubi_code_hint ? 4u : 0u) |
           (show_app_logo ? 8u : 0u) | ((reserved_gloss_lines > 2u ? 2u : reserved_gloss_lines) << 4);
  }
  static constexpr CandidateLayoutSettings decode(unsigned value) {
    return {(value & 1u) != 0, (value & 2u) != 0, (value & 4u) != 0, (value & 8u) != 0,
            (value >> 4) & 3u};
  }
};

// 释义要占几行：候选翻译或离线英文释义打开时，每种目标语言一行，最少一行、最多两行。目标语言的取法和 macOS 的 MSIMETranslationTargetsFromPreferences 一样：第一种缺值时按英语，只认支持的七种语言，第二种语言不支持、没有设置或与第一种相同时只算一行。
inline unsigned candidate_reserved_gloss_lines(const nlohmann::json &preferences) {
  const auto flag = [&](const char *key) {
    const auto found = preferences.find(key);
    return found != preferences.end() && found->is_boolean() && found->get<bool>();
  };
  if (!flag("candidate_translations") && !flag("candidate_english_gloss"))
    return 0;
  const auto supported = [](const std::string &language) {
    for (const char *known : {"en", "fr", "ja", "es", "ru", "de", "ko"})
      if (language == known)
        return true;
    return false;
  };
  const auto primary = preferences.find("translation_target_language");
  const std::string first =
      primary != preferences.end() && primary->is_string() ? primary->get<std::string>() : "en";
  const auto secondary = preferences.find("translation_secondary_language");
  const bool two = supported(first) && secondary != preferences.end() && secondary->is_string() &&
                   supported(secondary->get<std::string>()) &&
                   secondary->get<std::string>() != first;
  return two ? 2u : 1u;
}

inline std::optional<CandidateLayoutSettings>
candidate_layout_settings(const nlohmann::json &preferences) {
  try {
    if (!preferences.is_object())
      return std::nullopt;
    const auto layout =
        preferences.value("candidate_layout", std::string("vertical"));
    const auto preedit =
        preferences.value("candidate_preedit_style", std::string("pinyin"));
    if ((layout != "horizontal" && layout != "vertical") ||
        (preedit != "pinyin" && preedit != "empty"))
      return std::nullopt;
    const auto hint = preferences.find("wubi_code_hint");
    if (hint != preferences.end() && !hint->is_boolean() && !hint->is_null())
      return std::nullopt;
    const auto logo = preferences.find("show_app_logo");
    if (logo != preferences.end() && !logo->is_boolean() && !logo->is_null())
      return std::nullopt;
    return CandidateLayoutSettings{layout == "horizontal", preedit != "empty",
                                   hint == preferences.end() || hint->is_null() || hint->get<bool>(),
                                   logo != preferences.end() && logo->is_boolean() && logo->get<bool>(),
                                   candidate_reserved_gloss_lines(preferences)};
  } catch (...) {
    return std::nullopt;
  }
}
} // namespace msime::windows
