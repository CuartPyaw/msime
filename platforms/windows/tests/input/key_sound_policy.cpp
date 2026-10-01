#include "KeySoundPolicy.h"
#include "TypingEffectPolicy.h"

#include <iostream>
#include <stdexcept>
#include <string>

using namespace msime::windows;
namespace {
void require(bool value, int line) {
  if (!value)
    throw std::runtime_error("key sound policy failed at line " + std::to_string(line));
}
#define REQUIRE(value) require((value), __LINE__)

FanyImeNamedpipeData key(unsigned code, unsigned modifiers = 0) {
  FanyImeNamedpipeData packet{};
  packet.event_type = FanyImePipeEventType::KeyEvent;
  packet.keycode = code;
  packet.modifiers_down = modifiers;
  return packet;
}
} // namespace

int main() {
  try {
    // The classes msime_client_key_sound numbers: 1 space, 2 enter, 3 backspace, 0 any other key.
    REQUIRE(key_sound_class(key(0x20)) == 1u);
    REQUIRE(key_sound_class(key(0x0D)) == 2u);
    REQUIRE(key_sound_class(key(0x08)) == 3u);
    REQUIRE(key_sound_class(key('A')) == 0u);
    REQUIRE(key_sound_class(key('1')) == 0u);
    REQUIRE(key_sound_class(key(0xBC)) == 0u);
    REQUIRE(key_sound_class(key(0x1B)) == 0u);
    // Shift is typing: a capital, an operator, Shift+digit.
    REQUIRE(key_sound_class(key('V', 1)) == 0u);
    // The pipe's own flags are not modifiers.
    REQUIRE(key_sound_class(key(0x20, FanyImePipeFlags::UiLess | PipeMetadata::CandidateActive)) == 1u);
    // Chords with Ctrl or Alt are shortcuts, and a bare modifier the TIP forwards to cancel a composition is no key at all.
    REQUIRE(!key_sound_class(key('F', 3)));
    REQUIRE(!key_sound_class(key(0x08, 2)));
    REQUIRE(!key_sound_class(key('A', 4)));
    for (unsigned modifier : {0x10u, 0x11u, 0x12u, 0x14u, 0x5Bu, 0x5Cu, 0xA0u, 0xA1u, 0xA2u, 0xA3u, 0xA4u, 0xA5u, 0u})
      REQUIRE(!key_sound_class(key(modifier)));
    // Only key events sound.
    auto activated = key('A');
    activated.event_type = FanyImePipeEventType::ClientActivated;
    REQUIRE(!key_sound_class(activated));

    // The typing effect takes the same classes, muted (0x200) in a full-screen application, and 4 for a commit.
    REQUIRE(typing_effect_key_event(2u, true) == 2u);
    REQUIRE(typing_effect_key_event(0u, false) == 0x200u);
    REQUIRE(typing_effect_commit(true) == 4u);
    REQUIRE(typing_effect_commit(false) == 0x204u);
    // Its packed answer: bits 0-15 the combo, bit 16 a new tier, bits 17-19 the style.
    const auto power = decode_typing_effect(25u | 0x10000u | (3u << 17) | 0x100000u);
    REQUIRE(power.combo == 25u && power.tier_up && power.style == TypingEffectStyle::power_mode);
    const auto flash = decode_typing_effect(7u | (1u << 17));
    REQUIRE(flash.combo == 7u && !flash.tier_up && flash.style == TypingEffectStyle::flash);
    REQUIRE(decode_typing_effect(0u).style == TypingEffectStyle::off);
    REQUIRE(decode_typing_effect(7u << 17).style == TypingEffectStyle::power_mode);
    // The flash fades out over 150 ms, scales with effect_intensity, and is nothing with the style off or the intensity at 0.
    REQUIRE(typing_effect_flash_alpha(flash, 50u, 0u) > typing_effect_flash_alpha(flash, 50u, 100u));
    REQUIRE(typing_effect_flash_alpha(flash, 50u, 150u) == 0.0f);
    REQUIRE(typing_effect_flash_alpha(flash, 0u, 0u) == 0.0f);
    REQUIRE(typing_effect_flash_alpha(flash, 100u, 0u) > typing_effect_flash_alpha(flash, 50u, 0u));
    REQUIRE(typing_effect_flash_alpha(power, 50u, 0u) > typing_effect_flash_alpha(flash, 50u, 0u));
    REQUIRE(typing_effect_flash_alpha(power, 100u, 0u) <= 1.0f);
    REQUIRE(typing_effect_flash_alpha(decode_typing_effect(9u), 100u, 0u) == 0.0f);
    // The count shows from two keys on, until the library's 3 s idle window would have ended the combo.
    REQUIRE(!typing_effect_shows_combo(1u, 0u));
    REQUIRE(typing_effect_shows_combo(2u, 2999u));
    REQUIRE(!typing_effect_shows_combo(2u, 3000u));

    std::cout << "Windows key sound policy checks passed\n";
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
