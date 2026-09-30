#pragma once
#include "../../third_party/miniaudio/miniaudio.h"

#include <string>
#include <string_view>

namespace msime::windows {
// A WASAPI endpoint identity as the opaque ASCII id saved preferences hold: "wasapi:" and then every UTF-16 unit of the terminated native id as four lowercase hex digits. The settings list reads the same form from crates/host-api/src/voice_capture.rs (wasapi_device_id), so the two encoders must stay byte-for-byte identical. An empty or unterminated native id has no identity.
inline std::string wasapi_capture_device_id(const ma_device_id &id) {
  constexpr char hex[] = "0123456789abcdef";
  constexpr std::size_t capacity = sizeof(id.wasapi) / sizeof(id.wasapi[0]);
  std::string result("wasapi:");
  for (std::size_t index = 0; index < capacity; ++index) {
    const auto value = static_cast<unsigned>(static_cast<unsigned short>(id.wasapi[index]));
    if (!value)
      return index ? result : std::string{};
    for (unsigned digit = 4; digit > 0; --digit)
      result.push_back(hex[(value >> ((digit - 1) * 4)) & 15]);
  }
  return {};
}

// Whether a saved id is well formed for this host, checked before any device is opened. Ids other hosts write (coreaudio:, pulseaudio:, alsa:) are refused rather than falling back to the default microphone.
inline bool is_wasapi_capture_device_id(std::string_view id) {
  constexpr std::string_view prefix = "wasapi:";
  constexpr std::size_t capacity = sizeof(ma_device_id::wasapi) / sizeof(ma_device_id::wasapi[0]);
  if (id.substr(0, prefix.size()) != prefix)
    return false;
  const auto payload = id.substr(prefix.size());
  if (payload.empty() || payload.size() % 4 || payload.size() >= 4 * capacity)
    return false;
  for (const char ch : payload)
    if (!((ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f')))
      return false;
  return true;
}

// The one enumerated device whose identity is the requested id. Two devices reporting the same identity are ambiguous, and picking either would record from a microphone the person may not have chosen.
inline const ma_device_info *select_capture_device(const ma_device_info *devices, ma_uint32 count,
                                                   std::string_view requested) {
  if (!devices || requested.empty())
    return nullptr;
  const ma_device_info *selected = nullptr;
  for (ma_uint32 index = 0; index < count; ++index)
    if (wasapi_capture_device_id(devices[index].id) == requested) {
      if (selected)
        return nullptr;
      selected = &devices[index];
    }
  return selected;
}
} // namespace msime::windows
