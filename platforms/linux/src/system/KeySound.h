#pragma once

#include <cstdint>

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
