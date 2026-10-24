#pragma once
#include "PipeTicket.h"
#include <windows.h>
#include <optional>
#include <utility>

namespace msime::windows {
// 桌面句柄不能按数值或名字比较：不同窗口站可以有同名桌面。
inline bool same_desktop_object(HDESK first, HDESK second) {
  using Compare = BOOL(WINAPI *)(HANDLE, HANDLE);
  static const auto compare = reinterpret_cast<Compare>(reinterpret_cast<void *>(
      GetProcAddress(GetModuleHandleW(L"kernelbase.dll"), "CompareObjectHandles")));
  return first && second && compare && compare(first, second);
}

// 只接受来自当前焦点租约的已认证 PipeTicket：PipePeer 已核对高 32 位与真实管道对端。
// 这里再核对低 32 位的线程归属，持有线程对象，并拥有供 UI 线程绑定的桌面句柄。
class ClientDesktop final {
public:
  static std::optional<ClientDesktop> resolve(const PipeTicket &ticket) {
    const DWORD pid = client_pid(ticket);
    const DWORD tid = static_cast<DWORD>(ticket.client);
    if (!pid || !tid)
      return std::nullopt;
    ClientDesktop result;
    result.thread_ = OpenThread(THREAD_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, FALSE, tid);
    if (!result.thread_ || GetProcessIdOfThread(result.thread_) != pid || !result.alive())
      return std::nullopt;
    // GetThreadDesktop 返回借用句柄，不能 CloseDesktop；也不能直接据它跨窗口站绑定线程。
    const HDESK assigned = GetThreadDesktop(tid);
    wchar_t name[256]{};
    DWORD needed = 0;
    if (!assigned || !GetUserObjectInformationW(assigned, UOI_NAME, name, sizeof(name), &needed) ||
        !needed || needed > sizeof(name) || !name[0])
      return std::nullopt;
    result.desktop_ = OpenDesktopW(name, 0, FALSE,
        DESKTOP_CREATEWINDOW | DESKTOP_CREATEMENU | DESKTOP_ENUMERATE |
            DESKTOP_READOBJECTS | DESKTOP_WRITEOBJECTS);
    if (!same_desktop_object(assigned, result.desktop_) || !result.alive())
      return std::nullopt;
    result.tid_ = tid;
    return result;
  }
  ClientDesktop(ClientDesktop &&other) noexcept
      : thread_(std::exchange(other.thread_, nullptr)),
        desktop_(std::exchange(other.desktop_, nullptr)), tid_(other.tid_) {}
  ClientDesktop &operator=(ClientDesktop &&other) noexcept {
    if (this != &other) {
      close();
      thread_ = std::exchange(other.thread_, nullptr);
      desktop_ = std::exchange(other.desktop_, nullptr);
      tid_ = other.tid_;
    }
    return *this;
  }
  ClientDesktop(const ClientDesktop &) = delete;
  ClientDesktop &operator=(const ClientDesktop &) = delete;
  ~ClientDesktop() { close(); }
  HDESK handle() const { return desktop_; }
  // 线程退出或换了桌面时，缓存的归属立即失效，不回退到 Server 的默认桌面。
  bool current() const {
    return alive() && same_desktop_object(GetThreadDesktop(tid_), desktop_);
  }

private:
  ClientDesktop() = default;
  bool alive() const { return thread_ && WaitForSingleObject(thread_, 0) == WAIT_TIMEOUT; }
  void close() {
    if (desktop_)
      CloseDesktop(std::exchange(desktop_, nullptr));
    if (thread_)
      CloseHandle(std::exchange(thread_, nullptr));
  }
  HANDLE thread_ = nullptr;
  HDESK desktop_ = nullptr;
  DWORD tid_ = 0;
};
} // namespace msime::windows
