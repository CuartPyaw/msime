#include "../../panels/win7/HandwritingWindow.h"
#include "msime_client.h"
#include "../../../../shared/contracts/msime_edition.h"

#include <windowsx.h>
#include <nlohmann/json.hpp>
#include <cassert>
#include <cstdio>
#include <functional>
#include <filesystem>
#include <string>
#include <utility>
#include <vector>

namespace {
using namespace msime::windows::win7;
#if MSIME_EDITION_HANDWRITING
struct SyntheticState {
  std::filesystem::path path;
  std::vector<std::pair<std::wstring, std::wstring>> previous;
  SyntheticState() {
    wchar_t root[MAX_PATH]{};
    wchar_t unique[MAX_PATH]{};
    assert(GetTempPathW(MAX_PATH, root));
    assert(GetTempFileNameW(root, L"m7h", 0, unique));
    assert(DeleteFileW(unique));
    assert(CreateDirectoryW(unique, nullptr));
    path = unique;
    for (const auto* name : {L"MSIME_CLIENT_STATE_DIR", L"MSIME_CLIENT_HOST_OPTIONS", L"MSIME_HANDWRITING_MODEL"}) {
      const DWORD size = GetEnvironmentVariableW(name, nullptr, 0);
      std::wstring value(size, L'\0');
      if (size) value.resize(GetEnvironmentVariableW(name, value.data(), size));
      previous.emplace_back(name, std::move(value));
    }
    assert(SetEnvironmentVariableW(L"MSIME_CLIENT_STATE_DIR", path.c_str()));
    assert(SetEnvironmentVariableW(L"MSIME_CLIENT_HOST_OPTIONS", (path / L"runtime-options.json").c_str()));
    assert(SetEnvironmentVariableW(L"MSIME_HANDWRITING_MODEL", nullptr));
  }
  ~SyntheticState() {
    for (const auto& [name, value] : previous) SetEnvironmentVariableW(name.c_str(), value.empty() ? nullptr : value.c_str());
    std::error_code error;
    std::filesystem::remove_all(path, error);
  }
  nlohmann::json statistics(nlohmann::json action) const {
    const auto request = nlohmann::json{{"directory", path.u8string()}, {"action", std::move(action)}}.dump();
    char* answer = msime_client_typing_statistics(reinterpret_cast<const uint8_t*>(request.data()), request.size());
    assert(answer);
    auto response = nlohmann::json::parse(answer);
    msime_client_string_free(answer);
    assert(response.at("ok").get<bool>());
    return response.at("value");
  }
};
void pump_until(const std::function<bool()>& condition) {
  const ULONGLONG deadline = GetTickCount64() + 20000;
  for (;;) {
    MSG message{};
    while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
      assert(message.message != WM_QUIT);
      TranslateMessage(&message);
      DispatchMessageW(&message);
    }
    if (condition()) return;
    assert(GetTickCount64() < deadline);
    MsgWaitForMultipleObjects(0, nullptr, FALSE, 20, QS_ALLINPUT);
  }
}
void activate(HWND host, HWND editor = nullptr) {
  const DWORD current = GetCurrentThreadId();
  const DWORD foreground = GetWindowThreadProcessId(GetForegroundWindow(), nullptr);
  // 计划任务不一定继承前台权限；只在测试内连接当前前台队列，验收仍检查实际前台窗口。
  const bool attached = foreground && foreground != current && AttachThreadInput(current, foreground, TRUE);
  ShowWindow(host, SW_SHOW);
  SetForegroundWindow(host);
  BringWindowToTop(host);
  if (editor) SetFocus(editor);
  if (attached) AttachThreadInput(current, foreground, FALSE);
  pump_until([&] { return GetForegroundWindow() == host && (!editor || GetFocus() == editor); });
}
std::wstring caption(HWND window) {
  std::wstring result(static_cast<size_t>(GetWindowTextLengthW(window)) + 1, L'\0');
  result.resize(static_cast<size_t>(GetWindowTextW(window, result.data(), static_cast<int>(result.size()))));
  return result;
}
void stroke(HWND canvas, int left, int top, int right, int bottom) {
  SendMessageW(canvas, WM_LBUTTONDOWN, MK_LBUTTON, MAKELPARAM(left, top));
  SendMessageW(canvas, WM_MOUSEMOVE, MK_LBUTTON, MAKELPARAM((left + right) / 2, (top + bottom) / 2));
  SendMessageW(canvas, WM_LBUTTONUP, 0, MAKELPARAM(right, bottom));
}
bool busy(HWND host) {
  const auto status = caption(GetDlgItem(host, handwriting_status));
  return status == L"正在识别…" || status == L"正在查询中文手写识别器…";
}
HWND cross_candidate(HWND host) {
  for (int i = 0; i < 12; ++i) {
    HWND button = GetDlgItem(host, handwriting_candidate + i);
    if (caption(button) == L"十") return button;
  }
  return nullptr;
}
void draw_cross(HWND canvas) {
  RECT bounds{};
  assert(GetClientRect(canvas, &bounds));
  // 通过实际窗口坐标输入，验证 DPI 缩放到共享的 420 DIP 画布。
  stroke(canvas, bounds.right * 2 / 7, bounds.bottom / 2, bounds.right * 5 / 7, bounds.bottom / 2);
  stroke(canvas, bounds.right / 2, bounds.bottom * 2 / 7, bounds.right / 2, bounds.bottom * 5 / 7);
}
#endif
} // namespace

int main(int argc, char** argv) {
#ifdef _MSC_VER
  _set_error_mode(_OUT_TO_STDERR);
#endif
  const bool desktop = argc == 2 && std::string(argv[1]) == "--desktop";
  assert(argc == 1 || desktop);
  if (desktop) {
    DWORD session = 0;
    if (!ProcessIdToSessionId(GetCurrentProcessId(), &session) || session == 0 || !GetForegroundWindow()) {
      std::fputs("桌面验收未执行：需要已解锁的用户交互会话；SSH 服务会话不能验证前台上屏。\n", stderr);
      return 77;
    }
  }
#if !MSIME_EDITION_HANDWRITING
  const auto probe = std::string("{\"operation\":\"status\"}");
  char* buffer = msime_client_windows_handwriting(reinterpret_cast<const uint8_t*>(probe.data()), probe.size());
  assert(buffer);
  const auto status = nlohmann::json::parse(buffer);
  msime_client_string_free(buffer);
  assert(!status.at("ok").get<bool>());
  assert(status.at("error") == "handwriting_not_offered");
  HandwritingWindow unavailable;
  assert(!unavailable.create(GetModuleHandleW(nullptr)));
  return 0;
#else
  SyntheticState synthetic;
  synthetic.statistics({{"operation", "set_enabled"}, {"enabled", true}});
  const auto probe = std::string("{\"operation\":\"status\"}");
  char* buffer = msime_client_windows_handwriting(reinterpret_cast<const uint8_t*>(probe.data()), probe.size());
  assert(buffer);
  const auto status = nlohmann::json::parse(buffer);
  msime_client_string_free(buffer);
  assert(status.at("ok").get<bool>());
  const bool system = status.at("value").at("system_recognizer").get<bool>();
  HWND editor_host = nullptr;
  HWND editor = nullptr;
  if (desktop) {
    assert(system);
    editor_host = CreateWindowExW(0, L"STATIC", L"合成手写上屏验收", WS_OVERLAPPEDWINDOW,
        40, 40, 380, 220, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
    assert(editor_host);
    editor = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT", L"", WS_CHILD | WS_VISIBLE | ES_MULTILINE,
        10, 10, 340, 150, editor_host, nullptr, GetModuleHandleW(nullptr), nullptr);
    assert(editor);
    activate(editor_host, editor);
  }
  std::wstring inserted;
  bool accept = false;
  HandwritingWindow::SubmitCandidate sink;
  if (!desktop) sink = [&](std::wstring_view candidate) {
    if (accept) inserted.assign(candidate);
    return accept;
  };
  HandwritingWindow window(std::move(sink));
  assert(window.create(GetModuleHandleW(nullptr)));
  HWND host = window.handle();
  HWND canvas = GetDlgItem(host, handwriting_canvas);
  assert(canvas);
  assert((GetWindowLongPtrW(host, GWL_EXSTYLE) & WS_EX_NOACTIVATE) != 0);
  assert(SendMessageW(host, WM_MOUSEACTIVATE, 0, MAKELPARAM(HTCLIENT, WM_LBUTTONDOWN)) == MA_NOACTIVATE);
  ShowWindow(host, SW_SHOWNOACTIVATE);
  pump_until([&] { return !busy(host); });
  assert(!IsWindowEnabled(GetDlgItem(host, handwriting_undo)));
  draw_cross(canvas);
  pump_until([&] { return !busy(host); });
  if (system) {
    assert(cross_candidate(host));
    if (!desktop) {
      SendMessageW(cross_candidate(host), BM_CLICK, 0, 0);
      assert(inserted.empty());
      assert(caption(GetDlgItem(host, handwriting_status)).find(L"未能输入") != std::wstring::npos);
      assert(cross_candidate(host));
      accept = true;
    } else {
      HWND other = CreateWindowExW(0, L"STATIC", L"合成的其他窗口", WS_OVERLAPPEDWINDOW,
          460, 40, 300, 180, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
      assert(other);
      activate(other);
      SendMessageW(cross_candidate(host), BM_CLICK, 0, 0);
      assert(caption(editor).empty());
      assert(GetForegroundWindow() == other);
      assert(cross_candidate(host));
      assert(caption(GetDlgItem(host, handwriting_status)).find(L"未能输入") != std::wstring::npos);
      activate(editor_host, editor);
      DestroyWindow(other);
    }
    SendMessageW(cross_candidate(host), BM_CLICK, 0, 0);
    if (desktop) {
      pump_until([&] { return caption(editor) == L"十"; });
      assert(GetForegroundWindow() == editor_host);
    } else assert(inserted == L"十");
    assert(!IsWindowEnabled(GetDlgItem(host, handwriting_clear)));
    assert(!cross_candidate(host));
  } else {
    std::puts("未检测到可用系统中文识别器：只验证窗口与识别不可用的状态，未验收实际中文识别。");
    const auto notice = caption(GetDlgItem(host, handwriting_status));
    assert(notice.find(L"未安装") != std::wstring::npos || notice.find(L"未能完成") != std::wstring::npos);
  }
  // 清空后即便前一批异步结果到达，也不能重新显示旧候选。
  draw_cross(canvas);
  SendMessageW(GetDlgItem(host, handwriting_clear), BM_CLICK, 0, 0);
  assert(!IsWindowEnabled(GetDlgItem(host, handwriting_undo)));
  stroke(canvas, 40, 60, 80, 60);
  SendMessageW(GetDlgItem(host, handwriting_undo), BM_CLICK, 0, 0);
  assert(!IsWindowEnabled(GetDlgItem(host, handwriting_clear)));
  assert(!cross_candidate(host));
  // 关闭时等待识别线程退出，不泄漏线程或把结果投递给已经销毁的窗口。
  draw_cross(canvas);
  SendMessageW(host, WM_CLOSE, 0, 0);
  assert(!IsWindow(host));
  if (system) {
    const auto statistics = synthetic.statistics({{"operation", "load"}});
    assert(statistics.at("total") == 1);
    assert(statistics.at("detail").at("sources").at("handwriting") == 1);
  }
  if (editor_host) DestroyWindow(editor_host);
  MSG message{};
  while (PeekMessageW(&message, nullptr, WM_QUIT, WM_QUIT, PM_REMOVE)) {}
  std::puts(desktop ? "真实编辑器接收合成手写候选，面板未夺取焦点。" : "原生手写画布、识别、撤销、清空、候选提交与关闭通过。");
  return 0;
#endif
}
