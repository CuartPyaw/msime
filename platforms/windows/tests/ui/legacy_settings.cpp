#include "../../settings/legacy/SettingsWindow.h"
#include "../../settings/legacy/PreferencesStore.h"
#include "../../../../shared/contracts/msime_edition.h"

#include <commctrl.h>
#include <cassert>
#include <cstdio>
#include <filesystem>
#include <functional>
#include <string>

namespace {
using namespace msime::settings::legacy;

struct TemporaryDirectory {
  TemporaryDirectory() {
    wchar_t root[MAX_PATH]{};
    wchar_t unique[MAX_PATH]{};
    const DWORD length = GetTempPathW(MAX_PATH, root);
    assert(length && length < MAX_PATH);
    assert(GetTempFileNameW(root, L"m7s", 0, unique));
    assert(DeleteFileW(unique));
    assert(CreateDirectoryW(unique, nullptr));
    path = unique;
  }
  ~TemporaryDirectory() {
    std::error_code error;
    std::filesystem::remove_all(path, error);
  }
  std::filesystem::path path;
};

struct InjectedDirectory {
  explicit InjectedDirectory(const std::filesystem::path& path) {
    const DWORD required = GetEnvironmentVariableW(L"MSIME_CLIENT_STATE_DIR", nullptr, 0);
    present = required != 0;
    if (present) {
      previous.resize(required);
      const DWORD length = GetEnvironmentVariableW(L"MSIME_CLIENT_STATE_DIR", previous.data(), required);
      assert(length < required);
      previous.resize(length);
    }
    assert(SetEnvironmentVariableW(L"MSIME_CLIENT_STATE_DIR", path.c_str()));
  }
  ~InjectedDirectory() {
    SetEnvironmentVariableW(L"MSIME_CLIENT_STATE_DIR", present ? previous.c_str() : nullptr);
  }
  bool present = false;
  std::wstring previous;
};

void pump_until(const std::function<bool()>& condition) {
  const ULONGLONG deadline = GetTickCount64() + 15000;
  for (;;) {
    MSG message{};
    while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
      assert(message.message != WM_QUIT);
      TranslateMessage(&message);
      DispatchMessageW(&message);
    }
    if (condition()) return;
    assert(GetTickCount64() < deadline);
    MsgWaitForMultipleObjects(0, nullptr, FALSE, 25, QS_ALLINPUT);
  }
}

std::wstring caption(HWND window) {
  std::wstring value(static_cast<size_t>(GetWindowTextLengthW(window)) + 1, L'\0');
  const int copied = GetWindowTextW(window, value.data(), static_cast<int>(value.size()));
  value.resize(static_cast<size_t>(copied));
  return value;
}

} // namespace

int main() {
  using namespace msime::settings::legacy;
  TemporaryDirectory temporary;
  InjectedDirectory injected(temporary.path);
  PreferencesStore store(temporary.path.u8string());
  auto loaded = store.load();
  assert(loaded.outcome == Outcome::loaded && loaded.snapshot);
  auto tokens = loaded.snapshot->preferences().at("ai_assistant").at("tokens");
  tokens["https://compat.example.invalid"] = "synthetic-token-not-a-real-credential";
  auto seeded = store.save(*loaded.snapshot, {
      {{"ai_assistant", "tokens"}, tokens},
      {{"navigation", "tab"}, false}, {{"navigation", "page_up_down"}, false}});
  assert(seeded.outcome == Outcome::saved && seeded.snapshot);
  auto peer = store.load();
  assert(peer.snapshot && peer.snapshot->revision() == seeded.snapshot->revision());
  const auto unchanged = seeded.snapshot->document;
  auto changed = store.save(*seeded.snapshot, {{{"keybindings", "switch_language_caps_lock"}, true}});
  assert(changed.outcome == Outcome::saved && changed.snapshot);
  assert(seeded.snapshot->document == unchanged);
  auto conflict = store.save(*peer.snapshot, {{{"keybindings", "switch_language_caps_lock"}, false}});
  assert(conflict.outcome == Outcome::conflict && !conflict.snapshot);
  assert(store.load().snapshot->revision() == changed.snapshot->revision());
  assert(store.load().snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  auto invalid = store.save(*changed.snapshot, {{{"keybindings", "synthetic_unknown_key"}, true}});
  assert(invalid.outcome == Outcome::invalid_change);
  auto rejected = store.save(*changed.snapshot, {{{"keybindings", "switch_language_caps_lock"}, "invalid"}});
  assert(rejected.outcome == Outcome::storage);
  assert(store.load().snapshot->revision() == changed.snapshot->revision());

  // 真实 HWND、系统控件通知和工作线程贯通到同源 Rust，不触碰测试目录之外的数据。
  SettingsWindow window;
  assert(window.create(GetModuleHandleW(nullptr)));
  const HWND host = window.handle();
  const HWND body = GetDlgItem(host, page_body);
  const HWND language = GetDlgItem(body, language_switch);
  const HWND caps = GetDlgItem(body, caps_lock);
  assert(body && language && caps);
#if MSIME_EDITION_IS_FULL
  assert(GetDlgItem(body, japanese_toggle) != nullptr);
#else
  assert(GetDlgItem(body, japanese_toggle) == nullptr);
#endif
  pump_until([&] { return IsWindowEnabled(language) != FALSE; });
  assert(SendMessageW(caps, BM_GETCHECK, 0, 0) == BST_CHECKED);
  const auto before_controls = store.load().snapshot->preferences();
  const auto select_page = [&](int index) {
    const HWND selector = GetDlgItem(host, page_selector);
    assert(selector);
    SendMessageW(selector, CB_SETCURSEL, index, 0);
    SendMessageW(host, WM_COMMAND, MAKEWPARAM(page_selector, CBN_SELCHANGE), reinterpret_cast<LPARAM>(selector));
  };
  SendMessageW(language, CB_SETCURSEL, 3, 0);
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(language_switch, CBN_SELCHANGE), reinterpret_cast<LPARAM>(language));
  assert(!IsWindowEnabled(language));
  pump_until([&] { return IsWindowEnabled(language) != FALSE; });
  auto saved = store.load();
  assert(saved.snapshot);
  const auto& keys = saved.snapshot->preferences().at("keybindings");
  assert(!keys.at("switch_language_shift").get<bool>());
  assert(!keys.at("switch_language_ctrl").get<bool>());
  assert(!keys.at("switch_language_ctrl_alt_space").get<bool>());
  assert(saved.snapshot->preferences().at("ai_assistant") == before_controls.at("ai_assistant"));

  SendMessageW(caps, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(caps) != FALSE; });
  saved = store.load();
  assert(!saved.snapshot->preferences().at("keybindings").at("switch_language_caps_lock").get<bool>());
  const bool previous_width = saved.snapshot->preferences().at("keybindings").at("toggle_fullwidth_option_shift_h").get<bool>();
  const auto external = store.save(*saved.snapshot, {{{"keybindings", "toggle_fullwidth_option_shift_h"}, !previous_width}});
  assert(external.outcome == Outcome::saved);
  SendMessageW(caps, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(caps) != FALSE; });
  assert(caption(GetDlgItem(host, status_text)).find(L"其他窗口") != std::wstring::npos);
  assert(SendMessageW(caps, BM_GETCHECK, 0, 0) == BST_UNCHECKED);
  saved = store.load();
  assert(saved.snapshot->revision() == external.snapshot->revision());
  assert(saved.snapshot->preferences().at("keybindings").at("toggle_fullwidth_option_shift_h").get<bool>() != previous_width);

  const HWND tabs = GetDlgItem(host, page_tabs);
  TabCtrl_SetCurSel(tabs, 1);
  NMHDR notification{tabs, page_tabs, TCN_SELCHANGE};
  SendMessageW(host, WM_NOTIFY, page_tabs, reinterpret_cast<LPARAM>(&notification));
  const HWND reference = GetDlgItem(body, reference_text);
  assert((GetWindowLongPtrW(reference, GWL_STYLE) & ES_READONLY) != 0);
  assert(caption(reference).find(L"Page Up / Page Down") != std::wstring::npos);
  assert(caption(reference).find(L"Shift+Tab / Tab") != std::wstring::npos);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  // 原有自定义尺寸只在用户选择标准档位后才改变；两个尺寸字段原子保存。
  select_page(1);
  const HWND size = GetDlgItem(body, toolbar_size);
  assert((GetWindowLongPtrW(size, GWL_STYLE) & WS_VISIBLE) != 0);
  assert((GetWindowLongPtrW(tabs, GWL_STYLE) & WS_VISIBLE) == 0);
#if MSIME_EDITION_HANDWRITING
  assert(GetDlgItem(body, toolbar_handwriting) != nullptr);
#else
  assert(GetDlgItem(body, toolbar_handwriting) == nullptr);
#endif
  auto custom = store.save(*saved.snapshot, {{{"floating_toolbar", "scale_percent"}, 110},
      {{"floating_toolbar", "font_size"}, 20}});
  assert(custom.outcome == Outcome::saved);
  SendMessageW(host, WM_COMMAND, MAKEWPARAM(reload_settings, BN_CLICKED), 0);
  pump_until([&] { return IsWindowEnabled(size) != FALSE; });
  assert(SendMessageW(size, CB_GETCOUNT, 0, 0) == 6);
  assert(SendMessageW(size, CB_GETCURSEL, 0, 0) == 5);
  assert(store.load().snapshot->revision() == custom.snapshot->revision());
  SendMessageW(size, CB_SETCURSEL, 4, 0);
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(toolbar_size, CBN_SELCHANGE), reinterpret_cast<LPARAM>(size));
  pump_until([&] { return IsWindowEnabled(size) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->revision() == custom.snapshot->revision() + 1);
  assert(saved.snapshot->preferences().at("floating_toolbar").at("scale_percent") == 150);
  assert(saved.snapshot->preferences().at("floating_toolbar").at("font_size") == 28);
  assert(SendMessageW(size, CB_GETCOUNT, 0, 0) == 5);
  const HWND emoji = GetDlgItem(body, toolbar_emoji);
  const bool previous_emoji = saved.snapshot->preferences().at("floating_toolbar").at("emoji").get<bool>();
  SendMessageW(emoji, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(emoji) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->preferences().at("floating_toolbar").at("emoji").get<bool>() != previous_emoji);
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  select_page(0);
  TabCtrl_SetCurSel(tabs, 0);
  SendMessageW(host, WM_NOTIFY, page_tabs, reinterpret_cast<LPARAM>(&notification));
  SendMessageW(language, CB_SETCURSEL, 1, 0);
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(language_switch, CBN_SELCHANGE), reinterpret_cast<LPARAM>(language));
  assert(!IsWindowEnabled(language));
  // 关闭会完成刚受理的修改并等待工作线程退出，不丢掉排队写入。
  DestroyWindow(host);
  assert(window.handle() == nullptr);
  saved = store.load();
  assert(saved.snapshot->preferences().at("keybindings").at("switch_language_ctrl").get<bool>());
  assert(!saved.snapshot->preferences().at("keybindings").at("switch_language_shift").get<bool>());
  assert(!saved.snapshot->preferences().at("keybindings").at("switch_language_ctrl_alt_space").get<bool>());
  std::puts("Legacy settings: real CAS, native control saves, conflicts and reference passed");
}
