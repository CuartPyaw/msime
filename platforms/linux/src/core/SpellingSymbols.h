#pragma once

#include <nlohmann/json.hpp>

#include <string>

namespace msime::linux_host {

// The Engine lists in View.spelling_symbols the non-letter characters it takes as input in its current state: the operators and digits of the expression mode, the digits of the unicode mode, and with nothing composed the keys that open a mode ("/" and "@"). Shared by the IBus and Fcitx5 hosts, so neither keeps its own list of which mode spells with what; a host that did would drift the first time the Engine adds a mode.
inline bool spelling_symbol(const nlohmann::json &view, char32_t character) {
  if (!view.is_object() || character < 0x21 || character > 0x7e) return false;
  const auto symbols = view.find("spelling_symbols");
  return symbols != view.end() && symbols->is_string() &&
         symbols->get_ref<const std::string &>().find(static_cast<char>(character)) !=
             std::string::npos;
}

// A character a local mode is spelling with, which the host sends to the Engine as a character before any of its own key bindings can claim it: in the expression mode "-" and "=" are not page keys, "." is not smart punctuation, "(" is not an auto-paired bracket and a digit is not a candidate shortcut. The idle mode-entry keys are left to the ordinary punctuation route, where the shared runtime decides whether a mark opens a mode (it does not after a held phrase, or when the host keeps the mark ASCII beside a digit).
inline bool local_mode_spelling(const nlohmann::json &view, char32_t character) {
  if (!view.is_object()) return false;
  const auto mode = view.find("local_mode");
  return mode != view.end() && mode->is_string() && *mode != "none" &&
         spelling_symbol(view, character);
}

// Whether the bare number row is input rather than a candidate shortcut. The candidates are then picked with Shift and the number row, as Windows does in its Unicode mode, except where Shift and a digit key type a symbol the mode also spells with (the expression mode's % ^ * ( ) on a US layout).
inline bool spelling_digits(const nlohmann::json &view) {
  return spelling_symbol(view, U'0');
}

} // namespace msime::linux_host
