#pragma once

#include "../../../shared/contracts/windows_ipc.h"

namespace msime::windows::PipeMetadata {
// Set by the TSF while its original candidate list is active. This metadata
// is distinct from keyboard modifiers: VK_RETURN has different semantics for
// an original candidate list and an incremental/raw composition.
inline constexpr std::uint32_t CandidateActive = 0x40000000u;
// Set by the TSF on a key-down that is the auto-repeat of a held key (bit 30 of its lParam). The Server still handles the key as it always did; only the typing effect's combo leaves it uncounted.
inline constexpr std::uint32_t AutoRepeat = 0x20000000u;

inline constexpr std::uint32_t key_modifiers(std::uint32_t value) {
  return value & ~(FanyImePipeFlags::UiLess | CandidateActive | AutoRepeat);
}
} // namespace msime::windows::PipeMetadata
