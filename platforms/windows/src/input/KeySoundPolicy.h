#pragma once
#include "PipeMetadata.h"
#include "windows_ipc.h"
#include <cstdint>
#include <optional>

namespace msime::windows {
// The class msime_client_key_sound plays for a key the Server handled: 1 space, 2 enter, 3 backspace, 0 any other key. The TIP forwards only the keys the input method takes, so a key that reaches the Server is typing; the exceptions are a bare modifier, which the TIP forwards to cancel a composition, and a chord with Ctrl or Alt, which is a shortcut. Neither makes a sound.
inline std::optional<uint32_t> key_sound_class(const FanyImeNamedpipeData &packet) {
  if (packet.event_type != FanyImePipeEventType::KeyEvent)
    return std::nullopt;
  if (PipeMetadata::key_modifiers(packet.modifiers_down) & ~1u)
    return std::nullopt;
  switch (packet.keycode) {
  case 0:
  case 0x10: // Shift
  case 0x11: // Ctrl
  case 0x12: // Alt
  case 0x14: // Caps Lock
  case 0x5B: // left Windows
  case 0x5C: // right Windows
  case 0xA0:
  case 0xA1:
  case 0xA2:
  case 0xA3:
  case 0xA4:
  case 0xA5:
    return std::nullopt;
  case 0x20:
    return 1u;
  case 0x0D:
    return 2u;
  case 0x08:
    return 3u;
  default:
    return 0u;
  }
}
} // namespace msime::windows
