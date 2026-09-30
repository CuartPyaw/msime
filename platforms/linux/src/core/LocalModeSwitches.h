#pragma once

#include <string_view>

namespace msime::linux_host {

// The value a local-mode switch has when the preference document leaves it out, for the status menus of both hosts. client-core's LocalModePreferences is the authority: the Engine's own modes are on by default, while the expression (V), command (/) and mention (@) modes are off until the user turns them on, and the document omits them while they are off. Reading a missing one as on would tick a menu entry for a mode the Engine is not running.
inline bool local_mode_enabled_by_default(std::string_view key) {
  return key != "expression" && key != "command" && key != "mention";
}

} // namespace msime::linux_host
