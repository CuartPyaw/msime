#include "HandwritingWindow.h"
#include "../../compat/WindowsDpi.h"
#include <shellapi.h>
#include <cwchar>

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR, int show) {
  int count = 0;
  wchar_t** arguments = CommandLineToArgvW(GetCommandLineW(), &count);
  if (!arguments) return 1;
  // 独立开发目标目前只承载手写；不把其他面板路由误开成手写。
  bool supported = count == 1 || (count == 2 && std::wcscmp(arguments[1], L"--route=handwriting") == 0);
  LocalFree(arguments);
  if (count == 1) {
    wchar_t route[80]{};
    const DWORD length = GetEnvironmentVariableW(L"MSIME_CLIENT_ROUTE", route, 80);
    supported = !length || (length < 80 && std::wcscmp(route, L"handwriting") == 0);
  }
  if (!supported) return 2;
  msime::windows::compat::process_context(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
  msime::windows::win7::HandwritingWindow window;
  if (!window.create(instance)) return 1;
  return window.run(show);
}
