#pragma once

#include <string_view>

namespace msime::linux_host {

// What the panel shows for the input method, following the Windows language bar (LanguageBar.cpp): CapsLock outranks everything, because letters then reach the editor as capitals whichever mode is on; Japanese and Korean are shown only while conversion is on, since direct input with either scheme selected is plain English typing. Each host draws these with its own symbols.
enum class InputModeIndicator { Chinese, English, Japanese, Korean, CapsLock };

// `scheme` is the preferences scheme id in use ("quanpin", "japanese", "korean", ...); every id that is not Japanese or Korean is one of the Chinese schemes.
inline InputModeIndicator input_mode_indicator(bool input_enabled, std::string_view scheme,
                                               bool caps_lock) {
  if (caps_lock) return InputModeIndicator::CapsLock;
  if (!input_enabled) return InputModeIndicator::English;
  if (scheme == "japanese") return InputModeIndicator::Japanese;
  if (scheme == "korean") return InputModeIndicator::Korean;
  return InputModeIndicator::Chinese;
}

}  // namespace msime::linux_host
