#pragma once
#include <algorithm>
#include <cstdint>
#include <optional>
#include <string_view>

namespace msime::windows {
// The events msime_client_typing_effect takes beyond the key_sound classes 0-3, the flag for an auto-repeated key (drawn but not counted toward the combo), and the flag that keeps its tier-up sound quiet.
constexpr uint32_t typing_effect_commit_event = 4u;
constexpr uint32_t typing_effect_repeat_flag = 0x100u;
constexpr uint32_t typing_effect_muted_flag = 0x200u;
// How long one flash takes to fade out, and how long a combo count stays on the card after the last key: the library ends a combo after 3000 ms without a counted key, so the count it last reported is stale from then on.
constexpr uint32_t typing_effect_flash_millis = 150u;
constexpr uint32_t typing_effect_combo_millis = 3000u;

enum class TypingEffectStyle : uint32_t { off = 0, flash = 1, sparks = 2, power_mode = 3 };

struct TypingEffect {
  uint32_t combo = 0;
  bool tier_up = false;
  TypingEffectStyle style = TypingEffectStyle::off;
};

// The event for a key the Server handled, from its key_sound class. An auto-repeat of a held key is flagged so it does not count toward the combo or reach a new tier. Muted while sounds must stay quiet, so the library neither queues nor reports the tier-up sound.
inline uint32_t typing_effect_key_event(uint32_t key_class, bool sound_allowed, bool auto_repeat) {
  return key_class | (auto_repeat ? typing_effect_repeat_flag : 0u) | (sound_allowed ? 0u : typing_effect_muted_flag);
}
inline uint32_t typing_effect_commit(bool sound_allowed) {
  return typing_effect_key_event(typing_effect_commit_event, sound_allowed, false);
}

// Unpacks msime_client_typing_effect's return value: bits 0-15 the combo count, bit 16 a new tier, bits 17-19 the style. A style number this host does not know is drawn as the strongest one it does.
inline TypingEffect decode_typing_effect(uint32_t packed) {
  TypingEffect effect;
  effect.combo = packed & 0xFFFFu;
  effect.tier_up = (packed & 0x10000u) != 0;
  const uint32_t style = (packed >> 17) & 0x7u;
  effect.style = static_cast<TypingEffectStyle>((std::min)(style, 3u));
  return effect;
}

// The parts of msime_client_typing_effect_settings Windows draws: the intensity, the flash length (the pack's duration_ms, else the host's own 150 ms) and the flash colour (the pack's first colour, else the theme accent). Particles have nothing to drive on a card that only flashes, and the other colours of a pack are ignored.
struct TypingEffectSettings {
  uint32_t intensity = 50;
  uint32_t flash_millis = typing_effect_flash_millis;
  std::optional<uint32_t> color;
  friend bool operator==(const TypingEffectSettings &left, const TypingEffectSettings &right) {
    return left.intensity == right.intensity && left.flash_millis == right.flash_millis && left.color == right.color;
  }
};

// "#RRGGBB" as 0xRRGGBB, the only form the settings carry; anything else is no colour.
inline std::optional<uint32_t> typing_effect_rgb(std::string_view text) {
  if (text.size() != 7 || text.front() != '#')
    return std::nullopt;
  uint32_t value = 0;
  for (const char ch : text.substr(1)) {
    uint32_t digit = 0;
    if (ch >= '0' && ch <= '9')
      digit = static_cast<uint32_t>(ch - '0');
    else if (ch >= 'a' && ch <= 'f')
      digit = static_cast<uint32_t>(ch - 'a' + 10);
    else if (ch >= 'A' && ch <= 'F')
      digit = static_cast<uint32_t>(ch - 'A' + 10);
    else
      return std::nullopt;
    value = (value << 4) | digit;
  }
  return value;
}

// The settings clamped to the ranges the library documents (intensity 0-100, flash 60-1500 ms), so a value out of range from any producer still draws.
inline TypingEffectSettings resolve_typing_effect_settings(uint32_t intensity, std::optional<uint32_t> duration_ms, std::optional<uint32_t> color) {
  TypingEffectSettings settings;
  settings.intensity = (std::min)(intensity, 100u);
  if (duration_ms)
    settings.flash_millis = (std::max)(60u, (std::min)(*duration_ms, 1500u));
  if (color)
    settings.color = *color & 0xFFFFFFu;
  return settings;
}

// Packs the settings into one word the input thread hands the UI thread without a lock: bits 0-7 the intensity, 8-19 the flash length, 20 whether a colour is set, 32-55 the colour, 63 that settings were published at all.
inline uint64_t pack_typing_effect_settings(const TypingEffectSettings &settings) {
  return (1ull << 63) | (static_cast<uint64_t>(settings.color.value_or(0) & 0xFFFFFFu) << 32) |
         (settings.color ? (1ull << 20) : 0ull) | (static_cast<uint64_t>(settings.flash_millis & 0xFFFu) << 8) |
         static_cast<uint64_t>(settings.intensity & 0xFFu);
}
// Nothing for a word nobody published, which keeps the host's own preference-derived intensity.
inline std::optional<TypingEffectSettings> unpack_typing_effect_settings(uint64_t packed) {
  if ((packed & (1ull << 63)) == 0)
    return std::nullopt;
  std::optional<uint32_t> color;
  if (packed & (1ull << 20))
    color = static_cast<uint32_t>((packed >> 32) & 0xFFFFFFu);
  return resolve_typing_effect_settings(static_cast<uint32_t>(packed & 0xFFu), static_cast<uint32_t>((packed >> 8) & 0xFFFu), color);
}

// Opacity of the flash `elapsed` milliseconds after the key, 0 once it has faded or when nothing is drawn. Windows draws every style as a flash of the candidate card (it has no particle overlay); the stronger styles and a new tier flash brighter. effect_intensity 50 is the nominal strength, 100 doubles it. `flash_millis` is how long the flash takes to fade, an effect pack's duration_ms.
inline float typing_effect_flash_alpha(const TypingEffect &effect, uint32_t intensity, uint64_t elapsed,
                                       uint32_t flash_millis = typing_effect_flash_millis) {
  if (effect.style == TypingEffectStyle::off || intensity == 0 || flash_millis == 0 || elapsed >= flash_millis)
    return 0.0f;
  float base = 0.35f;
  if (effect.style == TypingEffectStyle::sparks)
    base = 0.5f;
  else if (effect.style == TypingEffectStyle::power_mode)
    base = 0.7f;
  if (effect.tier_up)
    base += 0.3f;
  const float strength = static_cast<float>((std::min)(intensity, 100u)) / 50.0f;
  const float remaining = 1.0f - static_cast<float>(elapsed) / static_cast<float>(flash_millis);
  return (std::min)(1.0f, base * strength) * remaining;
}

// Whether the card shows the combo count: a streak of at least two keys, reported within the library's idle window. A count of one is every first key, not a combo.
inline bool typing_effect_shows_combo(uint32_t combo, uint64_t elapsed) {
  return combo >= 2 && elapsed < typing_effect_combo_millis;
}
} // namespace msime::windows
