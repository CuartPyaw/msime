#pragma once

#include <cstdint>
#include <string>

namespace msime::linux_host {

// The key class msime_client_key_sound takes for a key press: 1 Space, 2 Enter, 3 Backspace, 0 any other key. IBus and Fcitx5 both report X11 keysyms, so one table serves both hosts.
constexpr std::uint32_t key_sound_class(std::uint32_t keysym) {
  switch (keysym) {
  case 0x0020: // space
  case 0xff80: // KP_Space
    return 1;
  case 0xff0d: // Return
  case 0xff8d: // KP_Enter
    return 2;
  case 0xff08: // BackSpace
    return 3;
  default:
    return 0;
  }
}

// Whether a key press makes a key sound. A release, a bare modifier and a shortcut (a key with Control, Alt, Super, Hyper or Meta held) are silent: the sound is for typing, and a chord the application handles is not typing. Shift and the level-3 shift only choose which character a key types, so they do not silence it.
constexpr bool key_press_sounds(bool release, bool modifier_key, bool shortcut) {
  return !release && !modifier_key && !shortcut;
}

// msime_client_typing_effect's events beyond the key classes above, and its flag for an auto-repeated key, which is drawn but not counted (msime_client.h).
constexpr std::uint32_t kTypingEffectCommit = 4;
constexpr std::uint32_t kTypingEffectRepeat = 0x100;

// The combo count in a msime_client_typing_effect answer: its low 16 bits, zero while the combo counter is off or the call was refused.
constexpr std::uint32_t typing_effect_combo(std::uint32_t answer) { return answer & 0xffffu; }

// The combo as the Linux hosts show it, as one more segment of the candidate aux line; empty while there is none. Linux draws no flash or sparks: neither IBus nor Fcitx5 gives an input method a reliable place on screen for an overlay under Wayland, so the count in the text the panel already shows is the whole effect.
inline std::string typing_combo_label(std::uint32_t combo) {
  return combo == 0 ? std::string{} : "连击 ×" + std::to_string(combo);
}

// Whether a press is an auto-repeat of the key still held. Neither IBus nor Fcitx5 tells an input method that a press repeats, but both pass it releases, and where a held key arrives as more presses of the same keysym with no release between them (Wayland, and X11 clients with detectable auto-repeat, which GDK turns on) that is the repeat; where X11 sends a fake release before each repeat, a repeat counts as a press. A release lost to a focus change misreads at most the next press of that key, and reset() at focus out avoids even that.
class KeyRepeat {
public:
  bool press(std::uint32_t keysym) {
    const bool repeat = keysym == held_;
    held_ = keysym;
    return repeat;
  }
  void release(std::uint32_t keysym) {
    if (keysym == held_) held_ = 0;
  }
  void reset() { held_ = 0; }

private:
  std::uint32_t held_ = 0;
};

// The host's half of msime_client_music_set_active. The player keeps the answer it was told last for the whole process, whichever session told it, so a host tells it only when the answer changes, and counts it told only once a call was accepted: a call made while no sound is switched on starts no player and is not remembered, so the next one repeats it once music is switched on.
class MusicActivity {
public:
  // Tell the player whether music may play now: the input method is active in a field that is not a secure one. Nothing is sent without a session.
  template <class Call> void sync(std::uint64_t session, bool active, Call &&call) {
    if (session == 0 || active == told_) return;
    if (call(session, active)) told_ = active;
  }
  // Before the session goes away. A player still told the input method is active would go on playing with no session left to say otherwise, so it hears the end now; the next session starts from silence.
  template <class Call> void release(std::uint64_t session, Call &&call) {
    if (session != 0 && told_) call(session, false);
    told_ = false;
  }
  bool told() const { return told_; }

private:
  bool told_ = false;
};

} // namespace msime::linux_host
