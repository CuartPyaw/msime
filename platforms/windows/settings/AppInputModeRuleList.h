#pragma once
#include <cstddef>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

// 「输入」页「中英文」组的应用例外（共享偏好 `app_input_mode_rules`）在写入前的规范化与校验，规则与 crates/client-core/src/preferences.rs 的 valid_app_input_mode_rules 和共享设置页的 app-input-mode-rules.ts 一致，另外要求 `.exe` 结尾，因为 Windows 只拿进程基名去比。先在这里查一遍是为了就地说出原因：保存被拒时窗口只能给一句笼统的提示。不依赖 Windows 和 WinRT 头文件，方便单独测试。
namespace msime::settings {

inline constexpr std::size_t app_input_mode_rule_limit = 32;
inline constexpr std::size_t app_input_mode_rule_bytes = 64;

enum class AppInputModeRuleError { Empty, TooLong, InvalidCharacter, NotExe, Duplicate, TooMany };

// 只把 ASCII 字母转成小写，非 ASCII 字符原样保留。
inline std::wstring app_rule_ascii_lowercase(std::wstring_view value) {
  std::wstring result(value);
  for (auto &c : result)
    if (c >= L'A' && c <= L'Z')
      c = static_cast<wchar_t>(c - L'A' + L'a');
  return result;
}

// 用户填的程序名：去掉首尾空白（含不换行空格和全角空格）；粘贴了完整路径时只取文件名；再转 ASCII 小写。
inline std::wstring normalize_app_input_mode_rule(std::wstring_view raw) {
  constexpr std::wstring_view blanks = L" \t\r\n\v\f\u00A0\u3000";
  auto trim = [&](std::wstring_view value) {
    const auto first = value.find_first_not_of(blanks);
    if (first == std::wstring_view::npos)
      return std::wstring_view{};
    const auto last = value.find_last_not_of(blanks);
    return value.substr(first, last - first + 1);
  };
  auto name = trim(raw);
  const auto separator = name.find_last_of(L"\\/");
  if (separator != std::wstring_view::npos)
    name = trim(name.substr(separator + 1));
  return app_rule_ascii_lowercase(name);
}

// UTF-16 换算成 UTF-8 的字节数，偏好库按字节限制标识的长度。
inline std::size_t app_rule_utf8_bytes(std::wstring_view value) {
  std::size_t bytes = 0;
  for (std::size_t i = 0; i < value.size(); ++i) {
    const auto c = static_cast<unsigned>(value[i]);
    if (c < 0x80)
      bytes += 1;
    else if (c < 0x800)
      bytes += 2;
    else if (c >= 0xD800 && c <= 0xDBFF && i + 1 < value.size()) {
      bytes += 4;
      ++i;
    } else
      bytes += 3;
  }
  return bytes;
}

// 规范化后的程序名能否加进现有的规则表；重复按 ASCII 不分大小写比较。
inline std::optional<AppInputModeRuleError>
validate_app_input_mode_rule(std::wstring const &name, std::vector<std::wstring> const &existing) {
  if (name.empty())
    return AppInputModeRuleError::Empty;
  if (app_rule_utf8_bytes(name) > app_input_mode_rule_bytes)
    return AppInputModeRuleError::TooLong;
  // 与 Rust 的 char::is_control 相同（U+0000–U+001F、U+007F–U+009F），再加上两个路径分隔符。
  for (const wchar_t c : name)
    if (c < 0x20 || (c >= 0x7F && c <= 0x9F) || c == L'\\' || c == L'/')
      return AppInputModeRuleError::InvalidCharacter;
  if (name.size() < 4 ||
      app_rule_ascii_lowercase(std::wstring_view(name).substr(name.size() - 4)) != L".exe")
    return AppInputModeRuleError::NotExe;
  const auto folded = app_rule_ascii_lowercase(name);
  for (auto const &rule : existing)
    if (app_rule_ascii_lowercase(rule) == folded)
      return AppInputModeRuleError::Duplicate;
  if (existing.size() >= app_input_mode_rule_limit)
    return AppInputModeRuleError::TooMany;
  return std::nullopt;
}

} // namespace msime::settings
