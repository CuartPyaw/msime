#pragma once
#include "CandidatePalette.h"
#include <algorithm>
#include <optional>

namespace msime::windows {
// The user's candidate window style: candidate_scale_percent, candidate_opacity_percent and candidate_corner_radius. Kept free of JSON so the settings window, which has no JSON library, draws its preview through the same rules the card is drawn with.
struct CandidateWindowStyle {
  unsigned scale_percent = 100;
  unsigned opacity_percent = 100;
  // None follows the skin package's radius, else the palette's own.
  std::optional<unsigned> corner_radius;
  bool valid() const {
    return scale_percent >= 50 && scale_percent <= 200 &&
           opacity_percent >= 50 && opacity_percent <= 100 &&
           (!corner_radius || *corner_radius <= 32);
  }
  // Multiplies the DPI scale: every DIP of the card, its fonts included, grows by this factor.
  double scale() const { return static_cast<double>(scale_percent) / 100.0; }
  float opacity() const { return static_cast<float>(opacity_percent) / 100.0f; }
  bool operator==(const CandidateWindowStyle &other) const {
    return scale_percent == other.scale_percent &&
           opacity_percent == other.opacity_percent &&
           corner_radius == other.corner_radius;
  }
  bool operator!=(const CandidateWindowStyle &other) const {
    return !(*this == other);
  }
};
// The card radius: the user's value, else the skin package's, else the palette's.
inline float candidate_card_radius(const CandidateWindowStyle &style,
                                   std::optional<float> skin, float palette) {
  if (style.corner_radius)
    return static_cast<float>(*style.corner_radius);
  return skin.value_or(palette);
}
// A row's highlight never rounds more than a card radius the user chose, so a square card keeps square rows. A skin package's radius leaves the rows at the palette's own shape, as they were drawn before the setting existed.
inline float candidate_row_radius(const CandidateWindowStyle &style,
                                  float palette_row, float card) {
  if (!style.corner_radius)
    return palette_row;
  return (std::max)(0.0f, (std::min)(palette_row, card));
}
// The card's surface, border and background image fade; text, numbers and the selection keep their own alpha.
inline CandidateColor candidate_faded(CandidateColor color,
                                      const CandidateWindowStyle &style) {
  color.a *= style.opacity();
  return color;
}
} // namespace msime::windows
