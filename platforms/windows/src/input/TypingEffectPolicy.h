#pragma once
#include <algorithm>
#include <cstdint>

namespace msime::windows {
// The events msime_client_typing_effect takes beyond the key_sound classes 0-3, and the flag that keeps its tier-up sound quiet.
constexpr uint32_t typing_effect_commit_event = 4u;
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

// The event for a key the Server handled, from its key_sound class. Muted while sounds must stay quiet, so the library neither queues nor reports the tier-up sound.
inline uint32_t typing_effect_key_event(uint32_t key_class, bool sound_allowed) {
  return key_class | (sound_allowed ? 0u : typing_effect_muted_flag);
}
inline uint32_t typing_effect_commit(bool sound_allowed) {
  return typing_effect_key_event(typing_effect_commit_event, sound_allowed);
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

// Opacity of the flash `elapsed` milliseconds after the key, 0 once it has faded or when nothing is drawn. Windows draws every style as a flash of the candidate card (it has no particle overlay); the stronger styles and a new tier flash brighter. effect_intensity 50 is the nominal strength, 100 doubles it.
inline float typing_effect_flash_alpha(const TypingEffect &effect, uint32_t intensity, uint64_t elapsed) {
  if (effect.style == TypingEffectStyle::off || intensity == 0 || elapsed >= typing_effect_flash_millis)
    return 0.0f;
  float base = 0.35f;
  if (effect.style == TypingEffectStyle::sparks)
    base = 0.5f;
  else if (effect.style == TypingEffectStyle::power_mode)
    base = 0.7f;
  if (effect.tier_up)
    base += 0.3f;
  const float strength = static_cast<float>((std::min)(intensity, 100u)) / 50.0f;
  const float remaining = 1.0f - static_cast<float>(elapsed) / static_cast<float>(typing_effect_flash_millis);
  return (std::min)(1.0f, base * strength) * remaining;
}

// Whether the card shows the combo count: a streak of at least two keys, reported within the library's idle window. A count of one is every first key, not a combo.
inline bool typing_effect_shows_combo(uint32_t combo, uint64_t elapsed) {
  return combo >= 2 && elapsed < typing_effect_combo_millis;
}
} // namespace msime::windows
