#pragma once

#include <windows.h>
#include <filesystem>
#include <functional>
#include <memory>
#include <optional>

namespace msime::settings::win7 {

// 控件标识也用于原生验收，操作实际 HWND 和工作线程，不绕过窗口事件。
enum ControlId {
  page_body = 99, language_switch = 100, caps_lock, japanese_toggle, traditional_toggle, fullwidth_toggle,
  reload_settings, page_tabs, status_text, reference_text, page_selector,
  toolbar_enabled = 200, toolbar_language, toolbar_scheme, toolbar_fullwidth,
  toolbar_punctuation, toolbar_character_set, toolbar_emoji, toolbar_handwriting,
  toolbar_keyboard, toolbar_voice, toolbar_settings, toolbar_size, toolbar_summary,
  diagnostic_server = 300, diagnostic_tsf, restart_service, open_data_directory,
  open_logs_directory, export_settings, import_settings, data_directory_text,
  word_character_keys = 400, backspace_reverts, navigation_minus_equal, navigation_comma_period,
  navigation_brackets, navigation_mouse_wheel, navigation_arrows, navigation_left_right,
  traditional_output = 500, output_description, chinese_english_spacing, fullwidth_output,
  punctuation_mode = 600, quote_style, bracket_style, paired_punctuation, smart_punctuation,
  smart_description, smart_repeat, smart_space_convert, smart_direct_digit, smart_direct_letter,
  cloud_candidates = 700, cloud_source, cloud_endpoint, cloud_method, cloud_headers, cloud_body,
  cloud_response_format, cloud_response_path, cloud_timeout, cloud_cache, cloud_save,
  mixed_enabled = 750, mixed_english, mixed_first_page_count, mixed_emoji, mixed_symbols,
  mixed_kaomoji, mixed_japanese, learn_candidates, single_character_only, word_lattice,
};

class SettingsWindow {
public:
  // 默认使用系统文件选择器；原生集成测试可注入合成路径，继续走正式文件与存储流程。
  using FilePicker = std::function<std::optional<std::filesystem::path>(HWND, bool)>;
  explicit SettingsWindow(FilePicker file_picker = {});
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

} // namespace msime::settings::win7
