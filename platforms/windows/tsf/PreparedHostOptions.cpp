#include "HostOptionsPaths.h"
#include "../src/system/StateRootLease.h"
#include <array>
#include <windows.h>

namespace msime::tsf {
std::string read_prepared_host_options(const std::filesystem::path &file) {
  // Match msime_client_create's limit; an extra byte detects truncation.
  constexpr std::size_t limit = 16384;
  try {
    msime::windows::reject_reparse_ancestors(file.parent_path());
  } catch (...) {
    return {};
  }
  HANDLE handle = CreateFileW(
      file.c_str(), GENERIC_READ,
      FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr,
      OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT, nullptr);
  if (handle == INVALID_HANDLE_VALUE || !msime::windows::handle_is_trusted_file(handle)) {
    if (handle != INVALID_HANDLE_VALUE)
      CloseHandle(handle);
    return {};
  }
  std::string document;
  document.reserve(limit + 1);
  std::array<char, 4096> buffer{};
  for (;;) {
    DWORD count = 0;
    if (!ReadFile(handle, buffer.data(), static_cast<DWORD>(buffer.size()), &count, nullptr)) {
      CloseHandle(handle);
      return {};
    }
    if (count == 0)
      break;
    if (document.size() > limit - static_cast<std::size_t>(count)) {
      CloseHandle(handle);
      return {};
    }
    document.append(buffer.data(), count);
  }
  CloseHandle(handle);
  return document.empty() ? std::string{} : document;
}
}
