#pragma once

#include <windows.h>
#include <memory>

namespace msime::settings::legacy {

// 控件标识也用于原生验收，操作实际 HWND 和工作线程，不绕过窗口事件。
enum ControlId {
  page_body = 99, language_switch = 100, caps_lock, japanese_toggle, traditional_toggle, fullwidth_toggle,
  reload_settings, page_tabs, status_text, reference_text, page_selector,
  toolbar_enabled = 200, toolbar_language, toolbar_scheme, toolbar_fullwidth,
  toolbar_punctuation, toolbar_character_set, toolbar_emoji, toolbar_handwriting,
  toolbar_keyboard, toolbar_voice, toolbar_settings, toolbar_size, toolbar_summary,
};

class SettingsWindow {
public:
  SettingsWindow();
  ~SettingsWindow();
  SettingsWindow(const SettingsWindow&) = delete;
  SettingsWindow& operator=(const SettingsWindow&) = delete;
  bool create(HINSTANCE instance);
  int run(int show);
  HWND handle() const;

private:
  struct State;
  std::unique_ptr<State> state_;
};

} // namespace msime::settings::legacy
