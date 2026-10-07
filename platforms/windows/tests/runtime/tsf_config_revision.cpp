#include "../../src/system/TsfConfigRevision.h"
#include <iostream>
#include <stdexcept>
#include <string>

using namespace msime::windows;
namespace {
[[noreturn]] void require_failed(int line) {
  throw std::runtime_error("TSF config revision test failed at line " +
                           std::to_string(line));
}
#define require(value)                                                         \
  do {                                                                         \
    if (!(value))                                                              \
      require_failed(__LINE__);                                                \
  } while (false)
} // namespace

int main() {
  try {
    TsfConfigRevision revision;
    const auto first = revision.snapshot();
    require(first != 0);

    // A preference publication that arrives while an older snapshot is being
    // sent must keep that newer revision pending.
    revision.mark_changed();
    const auto sent = revision.snapshot();
    revision.mark_changed();
    require(!revision.is_current(sent));
    require(revision.is_current(revision.snapshot()));

    std::cout << "TSF config revisions preserve newer publications\n";
  } catch (const std::exception &failure) {
    std::cerr << failure.what() << '\n';
    return 1;
  } catch (...) {
    std::cerr << "TSF config revision test failed with an unknown error\n";
    return 1;
  }
}
