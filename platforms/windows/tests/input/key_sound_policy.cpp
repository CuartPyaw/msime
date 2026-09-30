#include "KeySoundPolicy.h"

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

    std::cout << "Windows key sound policy checks passed\n";
    return 0;
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
