#include "../../compat/WindowsDpi.h"

#include <cassert>
#include <cstdio>

namespace {
int legacy_calls = 0;
DPI_AWARENESS_CONTEXT received_context = nullptr;
HWND received_window = nullptr;

UINT WINAPI native_window_dpi(HWND window) {
  received_window = window;
  SetLastError(ERROR_ACCESS_DENIED);
  return 0;
}
UINT WINAPI native_system_dpi() { return 168; }
DPI_AWARENESS_CONTEXT WINAPI native_thread_context(DPI_AWARENESS_CONTEXT context) {
  received_context = context;
  return DPI_AWARENESS_CONTEXT_SYSTEM_AWARE;
}
DPI_AWARENESS_CONTEXT WINAPI failed_thread_context(DPI_AWARENESS_CONTEXT) {
  SetLastError(ERROR_ACCESS_DENIED);
  return nullptr;
}
BOOL WINAPI native_process_context(DPI_AWARENESS_CONTEXT context) {
  received_context = context;
  SetLastError(ERROR_ACCESS_DENIED);
  return FALSE;
}
BOOL WINAPI legacy_process() {
  ++legacy_calls;
  return TRUE;
}
DPI_AWARENESS_CONTEXT WINAPI native_window_context(HWND window) {
  received_window = window;
  return DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2;
}
DPI_AWARENESS WINAPI native_awareness(DPI_AWARENESS_CONTEXT context) {
  received_context = context;
  return DPI_AWARENESS_PER_MONITOR_AWARE;
}
} // namespace

int main() {
  using msime::windows::compat::DpiApi;

  // 真实 HWND 验证无新 API 的路径，包括无效窗口和 GDI 查询；查询结果不能把旧系统固定为 96 DPI。
  const HWND window = CreateWindowExW(0, L"STATIC", L"dpi-fixture", WS_POPUP,
                                      0, 0, 64, 64, nullptr, nullptr,
                                      GetModuleHandleW(nullptr), nullptr);
  assert(window);
  DpiApi legacy;
  legacy.set_legacy_process = legacy_process;
  assert(legacy.window_dpi(nullptr) == 0);
  assert(legacy.window_context(nullptr) == nullptr);
  const HDC dc = GetDC(window);
  assert(dc);
  const UINT expected = static_cast<UINT>(GetDeviceCaps(dc, LOGPIXELSX));
  ReleaseDC(window, dc);
  assert(expected > 0);
  assert(legacy.window_dpi(window) == expected);
  assert(legacy.system_dpi() > 0);
  assert(legacy.window_context(window) ==
         (IsProcessDPIAware() ? DPI_AWARENESS_CONTEXT_SYSTEM_AWARE
                             : DPI_AWARENESS_CONTEXT_UNAWARE));
  const BOOL original_awareness = IsProcessDPIAware();
  assert(legacy.thread_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == nullptr);
  assert(GetLastError() == ERROR_CALL_NOT_IMPLEMENTED);
  assert(IsProcessDPIAware() == original_awareness);
  assert(legacy_calls == 0);
  {
    msime::windows::compat::ThreadDpiScope scope(legacy);
    assert(IsProcessDPIAware() == original_awareness);
    assert(legacy_calls == 0);
  }
  assert(legacy.process_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2));
  assert(legacy_calls == 1);
  assert(!legacy.process_context(nullptr));
  assert(legacy_calls == 1);
  assert(legacy.awareness(nullptr) == DPI_AWARENESS_INVALID);
  assert(legacy.awareness(DPI_AWARENESS_CONTEXT_UNAWARE) == DPI_AWARENESS_UNAWARE);
  assert(legacy.awareness(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE) == DPI_AWARENESS_SYSTEM_AWARE);
  assert(legacy.awareness(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == DPI_AWARENESS_PER_MONITOR_AWARE);

  // 新系统分支必须保留错误、零结果及上下文令牌；新 API 调用失败时不能再改一次进程状态。
  DpiApi modern;
  modern.query_window = native_window_dpi;
  modern.query_system = native_system_dpi;
  modern.set_thread = native_thread_context;
  modern.set_process = native_process_context;
  modern.query_context = native_window_context;
  modern.query_awareness = native_awareness;
  modern.set_legacy_process = legacy_process;
  assert(modern.window_dpi(window) == 0);
  assert(received_window == window);
  assert(GetLastError() == ERROR_ACCESS_DENIED);
  assert(modern.system_dpi() == 168);
  assert(modern.thread_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) ==
         DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
  assert(received_context == DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
  assert(!modern.process_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2));
  assert(GetLastError() == ERROR_ACCESS_DENIED);
  assert(legacy_calls == 1);
  assert(modern.window_context(window) == DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
  assert(received_window == window);
  assert(modern.awareness(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE) == DPI_AWARENESS_PER_MONITOR_AWARE);
  assert(received_context == DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
  {
    msime::windows::compat::ThreadDpiScope scope(modern);
    assert(received_context == DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
  }
  assert(received_context == DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
  modern.set_thread = failed_thread_context;
  bool failed = false;
  try {
    msime::windows::compat::ThreadDpiScope scope(modern);
  } catch (const std::runtime_error &) {
    failed = true;
  }
  assert(failed);

  // 系统实际导出也经同一入口调用，并核对线程作用域恢复；Win7 缺失入口时仍验证回退。
  const auto &actual = msime::windows::compat::dpi_api();
  assert(actual.window_dpi(window) > 0);
  assert(actual.system_dpi() > 0);
  const auto previous = actual.thread_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
  if (previous)
    assert(actual.thread_context(previous) != nullptr);
  else
    assert(GetLastError() == ERROR_CALL_NOT_IMPLEMENTED ||
           GetLastError() == ERROR_INVALID_PARAMETER);

  assert(DestroyWindow(window));
  assert(legacy.window_dpi(window) == 0);
  std::puts("Windows DPI native forwarding and legacy fallback passed");
}
