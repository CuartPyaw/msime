// The per-session file the MSIME bar widget for Omarchy reads the input mode from (src/core/InputStatus.h, data/omarchy/plugin).
#include "../src/core/InputStatus.h"

#include <cassert>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>
#include <unistd.h>

int main() {
  using namespace msime::linux_host;
  assert(!input_status_file(nullptr));
  assert(!input_status_file("relative"));
  assert(*input_status_file("/run/user/1000") == std::filesystem::path("/run/user/1000/msime-client/input-status.json"));

  // The widget parses these keys; the label is UTF-8 as the tray shows it.
  assert(input_status_document(true, "中", "quanpin") == "{\"active\":true,\"label\":\"中\",\"scheme\":\"quanpin\"}\n");
  assert(input_status_document(false, "英", "shuangpin") == "{\"active\":false,\"label\":\"英\",\"scheme\":\"shuangpin\"}\n");
  // A scheme id comes from the user's preferences document and is escaped rather than trusted.
  assert(input_status_document(true, "中", "a\"b") == "{\"active\":true,\"label\":\"中\",\"scheme\":\"a\\\"b\"}\n");

  const auto root = std::filesystem::temp_directory_path() / ("msime-input-status-" + std::to_string(::getpid()));
  std::filesystem::remove_all(root);
  std::filesystem::create_directories(root);
  const auto read = [&] {
    std::ifstream in(*input_status_file(root.c_str()), std::ios::binary);
    return std::string(std::istreambuf_iterator<char>(in), {});
  };
  const auto chinese = input_status_document(true, "中", "quanpin");
  publish_input_status(root.c_str(), chinese);
  assert(read() == chinese);
  // An unchanged document is not written again, even when the file changed underneath.
  { std::ofstream(*input_status_file(root.c_str())) << "edited"; }
  publish_input_status(root.c_str(), chinese);
  assert(read() == "edited");
  const auto english = input_status_document(true, "英", "quanpin");
  publish_input_status(root.c_str(), english);
  assert(read() == english);
  // Without a usable runtime directory nothing is written and nothing is remembered.
  publish_input_status(nullptr, chinese);
  publish_input_status(root.c_str(), chinese);
  assert(read() == chinese);
  std::filesystem::remove_all(root);
  return 0;
}
