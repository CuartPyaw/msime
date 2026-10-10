#pragma once
#include "KeySoundClass.h"
#include "PipeMetadata.h"
#include "windows_ipc.h"
#include <cstdint>
#include <optional>

namespace msime::windows {
// The class msime_client_key_sound plays for a key the Server handled: 1 space, 2 enter, 3 backspace, 0 any other key. A key that reaches the Server is typing; the exceptions are a bare modifier, which the TIP forwards to cancel a composition, and a chord with Ctrl or Alt, which is a shortcut. Neither makes a sound. The keys the TIP hands to the application sound through the Aux pipe instead (passthrough_key_sound_class), with the same table.
inline std::optional<uint32_t> key_sound_class(const FanyImeNamedpipeData &packet) {
  if (packet.event_type != FanyImePipeEventType::KeyEvent)
    return std::nullopt;
  if (PipeMetadata::key_modifiers(packet.modifiers_down) & ~1u)
    return std::nullopt;
  return key_sound_class_for_virtual_key(packet.keycode);
}
} // namespace msime::windows
