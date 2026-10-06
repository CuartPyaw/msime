#include "SoundPackRoot.h"

#include <chrono>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>

namespace {
void require(bool value, const char *message) {
  if (!value)
    throw std::runtime_error(message);
}
} // namespace

int main() {
  namespace fs = std::filesystem;
  const auto root = fs::temp_directory_path() /
                    ("msime-sound-pack-root-" +
                     std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
  if (!fs::create_directory(root))
    return 1;
  struct Cleanup {
    fs::path path;
    ~Cleanup() {
      std::error_code error;
      fs::remove_all(path, error);
    }
  } cleanup{root};
  try {
    using msime::windows::name_builtin_sound_packs;
    const nlohmann::json base = {{"api_version", 1},
                                 {"resources", (root / "resources").u8string()},
                                 {"preferences_directory", root.u8string()}};
    // A state root without the packs (a development run) leaves the options alone, so the library keeps its own guess beside resources.
    auto options = base;
    name_builtin_sound_packs(options, root);
    require(options == base, "A missing pack directory was named");

    // The installer's layout: DataDir\sound-packs beside the audio cues.
    fs::create_directory(root / "sound-packs");
    name_builtin_sound_packs(options, root);
    require(options.at("sound_packs") == (root / "sound-packs").u8string(),
            "The staged pack directory was not named");

    // A name already in the options wins; a relative root and a document that is not an object are left alone.
    auto named = base;
    named["sound_packs"] = "C:\\elsewhere";
    name_builtin_sound_packs(named, root);
    require(named.at("sound_packs") == "C:\\elsewhere", "An explicit name was replaced");
    auto relative = base;
    name_builtin_sound_packs(relative, fs::path("state"));
    require(!relative.contains("sound_packs"), "A relative state root was named");
    nlohmann::json not_object = nlohmann::json::array();
    name_builtin_sound_packs(not_object, root);
    require(not_object.is_array() && not_object.empty(), "A non-object document changed");

    // A plain file of that name is not a pack directory.
    const auto other = root / "other";
    fs::create_directory(other);
    { std::ofstream(other / "sound-packs") << "not a directory"; }
    auto file = base;
    name_builtin_sound_packs(file, other);
    require(!file.contains("sound_packs"), "A file was named as the pack directory");
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
  std::cout << "Windows built-in sound pack root checks passed\n";
  return 0;
}
