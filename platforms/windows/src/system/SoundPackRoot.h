#pragma once
#include <filesystem>
#include <nlohmann/json.hpp>
#include <system_error>

namespace msime::windows {
// Name the built-in sound packs in the options sessions are created from. Without a name the shared library looks for them beside the resources directory, where the macOS bundle and the Linux install put them. The Windows installer stages them in DataDir\sound-packs instead, beside the audio cues (installer/Prepare-PackageFiles.ps1), and DataDir is the Server's state root. A state root without them, as in a development run, leaves the options alone, and a name already in the options wins.
inline void name_builtin_sound_packs(nlohmann::json &options,
                                     const std::filesystem::path &state_root) {
  if (!options.is_object() || options.contains("sound_packs") ||
      !state_root.is_absolute())
    return;
  const auto packs = state_root / "sound-packs";
  std::error_code error;
  if (std::filesystem::is_directory(packs, error))
    options["sound_packs"] = packs.u8string();
}
} // namespace msime::windows
