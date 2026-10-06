#pragma once
#include "CandidateWindowStyle.h"
#include <cstdint>
#include <mutex>
#include <nlohmann/json.hpp>
#include <optional>

namespace msime::windows {
// Projects the three style fields out of the preferences document, as candidate_font_settings does for the fonts. An absent field is its default; a field of the wrong type or out of range rejects the whole projection, which leaves the card as it is.
inline std::optional<CandidateWindowStyle>
candidate_window_style(const nlohmann::json &preferences) {
  try {
    if (!preferences.is_object())
      return std::nullopt;
    CandidateWindowStyle result;
    for (const auto &[key, target] :
         {std::pair{"candidate_scale_percent", &result.scale_percent},
          std::pair{"candidate_opacity_percent", &result.opacity_percent}}) {
      if (!preferences.contains(key))
        continue;
      if (!preferences.at(key).is_number_integer())
        return std::nullopt;
      const auto value = preferences.at(key).get<int64_t>();
      if (value < 0 || value > 1000)
        return std::nullopt;
      *target = static_cast<unsigned>(value);
    }
    if (preferences.contains("candidate_corner_radius") &&
        !preferences.at("candidate_corner_radius").is_null()) {
      const auto &radius = preferences.at("candidate_corner_radius");
      if (!radius.is_number_integer())
        return std::nullopt;
      const auto value = radius.get<int64_t>();
      if (value < 0 || value > 32)
        return std::nullopt;
      result.corner_radius = static_cast<unsigned>(value);
    }
    return result.valid() ? std::optional{result} : std::nullopt;
  } catch (...) {
    return std::nullopt;
  }
}

// Latest style by preference revision, handed from the monitor thread to the UI thread like CandidateFontMailbox.
class CandidateWindowStyleMailbox {
public:
  bool publish(uint64_t revision, CandidateWindowStyle value) {
    if (!value.valid())
      return false;
    std::lock_guard lock(mutex_);
    if (revision_ && revision <= *revision_)
      return false;
    revision_ = revision;
    pending_ = value;
    return true;
  }
  std::optional<CandidateWindowStyle> take() {
    std::lock_guard lock(mutex_);
    auto result = pending_;
    pending_.reset();
    return result;
  }

private:
  std::mutex mutex_;
  std::optional<uint64_t> revision_;
  std::optional<CandidateWindowStyle> pending_;
};
} // namespace msime::windows
