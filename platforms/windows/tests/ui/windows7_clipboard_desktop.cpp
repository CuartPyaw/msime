#include "../../panels/win7/ClipboardWindow.h"
#include "../../panels/win7/CloudClipboardWindow.h"
#include "../../settings/win7/PreferencesStore.h"
#include "../../settings/win7/ControlText.h"
#include "../../settings/win7/ThemeData.h"
#include <cassert>
#include <cstdio>

namespace {
using namespace msime::settings::win7;
using namespace msime::windows::win7;
void pump(const std::function<bool()>& condition) {
  const auto deadline = GetTickCount64() + 10000;
  for (;;) {
    MSG message{}; while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) { assert(message.message != WM_QUIT); TranslateMessage(&message); DispatchMessageW(&message); }
    if (condition()) return;
    assert(GetTickCount64() < deadline); MsgWaitForMultipleObjects(0, nullptr, FALSE, 20, QS_ALLINPUT);
  }
}
LRESULT CALLBACK editor_proc(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  if (message == WM_SETFOCUS) { if (const auto editor = GetDlgItem(window, 1)) SetFocus(editor); return 0; }
  return DefWindowProcW(window, message, wparam, lparam);
}
void activate(HWND window) {
  const auto foreground = GetWindowThreadProcessId(GetForegroundWindow(), nullptr), current = GetCurrentThreadId();
  // 只为测试自己的窗口取得前台权限；产品代码不连接其他进程的输入队列。
  const bool attached = foreground && foreground != current && AttachThreadInput(current, foreground, TRUE);
  ShowWindow(window, SW_SHOW); SetForegroundWindow(window); BringWindowToTop(window);
  if (const auto editor = GetDlgItem(window, 1)) SetFocus(editor);
  if (attached) AttachThreadInput(current, foreground, FALSE);
  pump([&] { return GetForegroundWindow() == window; });
}
struct TemporaryState {
  std::filesystem::path path;
  std::wstring previous, previous_options;
  bool present = false, options_present = false;
  TemporaryState() {
    wchar_t root[MAX_PATH]{}, file[MAX_PATH]{}; assert(GetTempPathW(MAX_PATH, root) && GetTempFileNameW(root, L"m7d", 0, file));
    assert(DeleteFileW(file) && CreateDirectoryW(file, nullptr)); path = file;
    const auto remember = [](const wchar_t* key, std::wstring& value, bool& exists) {
      const auto length = GetEnvironmentVariableW(key, nullptr, 0); exists = length != 0;
      if (exists) { value.resize(length); value.resize(GetEnvironmentVariableW(key, value.data(), length)); }
    };
    remember(L"MSIME_CLIENT_STATE_DIR", previous, present); remember(L"MSIME_CLIENT_HOST_OPTIONS", previous_options, options_present);
    assert(SetEnvironmentVariableW(L"MSIME_CLIENT_STATE_DIR", path.c_str()));
    assert(SetEnvironmentVariableW(L"MSIME_CLIENT_HOST_OPTIONS", (path / L"missing-options.json").c_str()));
  }
  ~TemporaryState() {
    SetEnvironmentVariableW(L"MSIME_CLIENT_STATE_DIR", present ? previous.c_str() : nullptr);
    SetEnvironmentVariableW(L"MSIME_CLIENT_HOST_OPTIONS", options_present ? previous_options.c_str() : nullptr);
    std::error_code error; std::filesystem::remove_all(path, error);
  }
};
}
int main(int count, char** arguments) {
  if (count != 2 || std::string_view(arguments[1]) != "--desktop") { std::puts("请在交互桌面使用 --desktop；此程序会向它自己创建的编辑器输入合成文字。"); return 2; }
  USEROBJECTFLAGS flags{}; DWORD bytes = 0;
  const auto desktop = OpenInputDesktop(0, FALSE, DESKTOP_READOBJECTS);
  const bool visible = GetUserObjectInformationW(GetProcessWindowStation(), UOI_FLAGS, &flags, sizeof(flags), &bytes) && (flags.dwFlags & WSF_VISIBLE);
  if (desktop) CloseDesktop(desktop);
  if (!desktop || !visible) { std::puts("需要可见交互桌面；服务会话不能执行真实前台输入验收。"); return 2; }
  TemporaryState temporary; PreferencesStore store(temporary.path.u8string()); const auto initial = store.load(); assert(initial.snapshot);
  assert(store.save(*initial.snapshot, {{{"clipboard_history"}, true}}).outcome == Outcome::saved);
  const std::string sample = "合成🙂", id(64, 'a');
  assert(theme_data::value(theme_data::call(msime_client_capture_clipboard_history, {{"directory", temporary.path.u8string()}, {"text", sample}})));
  assert(theme_data::value(theme_data::call(msime_client_typing_statistics, {{"directory", temporary.path.u8string()}, {"action", {{"operation", "set_enabled"}, {"enabled", true}}}})));
  const auto instance = GetModuleHandleW(nullptr);
  WNDCLASSEXW type{sizeof(type)}; type.hInstance = instance; type.lpfnWndProc = editor_proc; type.lpszClassName = L"MSIME.Windows7Clipboard.TestEditor";
  assert(RegisterClassExW(&type));
  const auto editor_host = CreateWindowExW(0, type.lpszClassName, L"合成文字验收编辑器", WS_OVERLAPPEDWINDOW, 40, 40, 420, 200, nullptr, nullptr, instance, nullptr);
  const auto editor = CreateWindowExW(0, L"EDIT", L"", WS_CHILD | WS_VISIBLE | WS_BORDER | ES_MULTILINE, 10, 10, 380, 140, editor_host, reinterpret_cast<HMENU>(1), instance, nullptr);
  const auto other = CreateWindowExW(0, type.lpszClassName, L"合成的其他窗口", WS_OVERLAPPEDWINDOW, 480, 40, 320, 180, nullptr, nullptr, instance, nullptr);
  assert(editor_host && editor && other); activate(editor_host);
  const auto total = [&] { const auto result = theme_data::value(theme_data::call(msime_client_typing_statistics, {{"directory", temporary.path.u8string()}, {"action", {{"operation", "load"}}}})); assert(result); return result->at("total").get<unsigned>(); };
  const auto chosen = control_text::wide(sample);
  {
    ClipboardWindow window; assert(window.create(instance)); const auto host = window.handle(); ShowWindow(host, SW_SHOWNOACTIVATE);
    const auto wait = [&] { pump([&] { return IsWindowEnabled(GetDlgItem(host, clipboard_panel_refresh)) != FALSE; }); }; wait();
    assert(GetForegroundWindow() == editor_host);
    const auto list = GetDlgItem(host, clipboard_panel_entries);
    SendMessageW(list, LB_SETCURSEL, 0, 0); SendMessageW(host, WM_COMMAND, MAKEWPARAM(clipboard_panel_entries, LBN_SELCHANGE), reinterpret_cast<LPARAM>(list));
    activate(other); SendMessageW(GetDlgItem(host, clipboard_panel_insert), BM_CLICK, 0, 0); wait();
    assert(control_text::read(editor).value().empty() && GetForegroundWindow() == other && total() == 0);
    activate(editor_host); SendMessageW(GetDlgItem(host, clipboard_panel_insert), BM_CLICK, 0, 0); wait();
    pump([&] { return control_text::read(editor).value() == sample; });
    assert(GetForegroundWindow() == editor_host && total() > 0);
    DestroyWindow(host);
  }
  const auto local_total = total(); assert(SetWindowTextW(editor, L"")); activate(editor_host);
  {
    CloudClipboardWindow::Services services;
    services.account = [&](const std::string& request) {
      const auto action = Json::parse(request).at("action").at("operation"); Json value;
      if (action == "list") value = {{"enabled", true}, {"retention_days", 0}, {"items", Json::array({{{"id", id}, {"text", sample}, {"pinned", false}, {"device", "合成设备"}, {"updated_at", "2026-10-24T00:00:00Z"}}})},
          {"e2e", {{"status", "ready"}, {"code", "0123 4567 8901 2345"}, {"pending_devices", 0u}, {"undecryptable", 0u}}}};
      else { assert(action == "devices"); value = {{"enabled", true}, {"status", "ready"}, {"code", "0123 4567 8901 2345"}, {"device_id", std::string(32, 'b')}, {"devices", Json::array()}}; }
      return Json{{"ok", true}, {"value", value}}.dump();
    };
    services.relay = [](const std::string&) -> std::string { assert(false); return {}; };
    CloudClipboardWindow window({}, {}, services, [](const std::string&) { return Json{{"ok", true}, {"value", Json::object()}}.dump(); });
    assert(window.create(instance)); const auto host = window.handle(), body = GetDlgItem(host, cloud_clipboard_panel_body);
    const auto wait = [&] { pump([&] { return IsWindowEnabled(GetDlgItem(body, cloud_clipboard_refresh)) != FALSE; }); }; wait();
    const auto select = [&] { const auto list = GetDlgItem(body, cloud_clipboard_items); SendMessageW(list, LB_SETCURSEL, 0, 0); SendMessageW(body, WM_COMMAND, MAKEWPARAM(cloud_clipboard_items, LBN_SELCHANGE), reinterpret_cast<LPARAM>(list)); };
    activate(host); select(); activate(other);
    SendMessageW(GetDlgItem(host, cloud_clipboard_panel_insert), BM_CLICK, 0, 0); wait();
    assert(control_text::read(editor).value().empty() && GetForegroundWindow() == other && total() == local_total);
    activate(host); select(); SendMessageW(GetDlgItem(host, cloud_clipboard_panel_insert), BM_CLICK, 0, 0); wait();
    pump([&] { return control_text::read(editor).value() == sample; });
    assert(GetForegroundWindow() == editor_host && total() == local_total * 2);
    DestroyWindow(host);
  }
  DestroyWindow(other); DestroyWindow(editor_host);
  std::puts("独立剪贴板面板真实上屏与焦点验收通过：仅写自己的合成编辑器，第三个窗口在前台时拒绝，成功输入各统计一次。");
}
