#pragma once
#include <windows.h>
#include <atomic>
#include <cstdint>
#include <optional>

namespace msime::windows {
// Posted to the candidate window when a typing effect is waiting. The message wakes the Server's UI loop at once rather than at its next 50 ms poll, which is a third of the flash.
constexpr UINT typing_effect_message = WM_APP + 0x45;

// Hands the typing effect of each key from the Server's input thread to the candidate window on the UI thread. Only the latest value is kept: a flash superseded before the UI thread reads it is one the user could not have seen. A new tier is the exception and is carried over until read, so a key that follows it at once does not swallow the tier flash. Lock free and allocation free, as the key path requires.
class TypingEffectSignal final {
public:
  static TypingEffectSignal &instance() {
    static TypingEffectSignal signal;
    return signal;
  }
  // UI thread: the window that receives typing_effect_message. Detach before that window is destroyed.
  void attach(HWND target) { target_.store(target, std::memory_order_release); }
  void detach(HWND target) {
    HWND expected = target;
    target_.compare_exchange_strong(expected, nullptr, std::memory_order_acq_rel);
  }
  // Input thread. 0 is a real answer: with the flash style off it is the combo counter reporting a reset (Backspace, or the idle window), which has to clear a count still on the card. A run of zeros after it is skipped, so with effects and the counter both off the UI thread is not woken on every key.
  void publish(uint32_t packed) {
    if (packed == 0 && !reported_)
      return;
    reported_ = packed != 0;
    uint32_t previous = latest_.load(std::memory_order_relaxed);
    while (!latest_.compare_exchange_weak(previous, packed | waiting_bit | (previous & tier_up_bit),
                                          std::memory_order_acq_rel, std::memory_order_relaxed)) {
    }
    if (const HWND target = target_.load(std::memory_order_acquire))
      (void)PostMessageW(target, typing_effect_message, 0, 0);
  }
  // UI thread: the waiting value, which may be 0, or nothing when an earlier message already took it.
  std::optional<uint32_t> take() {
    const uint32_t value = latest_.exchange(0, std::memory_order_acq_rel);
    if ((value & waiting_bit) == 0)
      return std::nullopt;
    return value & ~waiting_bit;
  }

private:
  static constexpr uint32_t tier_up_bit = 0x10000u;
  // Marks a published value, so a published 0 is told apart from nothing waiting. The answer never uses bit 31.
  static constexpr uint32_t waiting_bit = 0x80000000u;
  // Input thread only: whether the last value published was not 0.
  bool reported_ = false;
  TypingEffectSignal() = default;
  std::atomic<uint32_t> latest_{0};
  std::atomic<HWND> target_{nullptr};
};
} // namespace msime::windows
