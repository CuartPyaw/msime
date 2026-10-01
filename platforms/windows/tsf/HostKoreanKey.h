#pragma once
#include "../common/KoreanHanjaKey.h"
#include <cstdint>

namespace msime::tsf {
// What a key does while the Korean scheme is active and the keyboard is open.
//
// Korean is a syllable automaton (see the MsimeCommand comment in msime_client.h): letters compose, and every other key ends the open syllable and then does its ordinary work. The TIP inserts that syllable itself, so no key here depends on the Chinese punctuation table or on full-width conversion. Its only candidates are the Hanja of the composing syllable, listed after the Hanja key; while that list is open the keys in KoreanHanjaKey.h choose from it, and every other key keeps the meaning it has without a list.
enum class KoreanKeyAction {
    // Not Korean-specific: the ordinary classification decides. Backspace and Escape while a syllable is open take this path, as do keys that produce no character.
    Default,
    // A letter is a jamo. The TIP eats it and sends the letter in the case Shift gives it.
    Compose,
    // A printable non-letter (space, digit, punctuation) while a syllable is open. The TIP eats it, commits the syllable and inserts the key's ASCII character right after it, so the character can never land ahead of the syllable it ended.
    CommitWithText,
    // A caret or editing key while a syllable is open. The syllable is committed and the key goes on to the application, which inserts the newline or tab, moves the caret or deletes.
    CommitAndPass,
    // The application gets the key untouched: a printable non-letter, or the Hanja key, with nothing composing, so punctuation stays half-width ASCII.
    Pass,
    // The Hanja key while a syllable is open: the TIP eats it and sends MSIME_CONVERT_HANJA, which lists the syllable's Hanja or closes an open list. A syllable with no Hanja (a lone jamo) keeps composing and the key is still eaten, so it never reaches the application in the middle of a syllable.
    ConvertHanja,
    // A key the open Hanja list takes (KoreanHanjaKey.h): a digit, Space, Enter, an arrow, Page Up/Down, Home/End, Escape or Backspace. The TIP eats it and applies it to the list.
    HanjaList,
};

inline constexpr uint32_t kVirtualKeyHangul = 0x15;
inline constexpr uint32_t kVirtualKeyHanja = msime::windows::kVirtualKeyHanja;

constexpr bool is_korean_letter_key(uint32_t vk, wchar_t wch) {
    return vk >= 'A' && vk <= 'Z' && ((wch >= L'a' && wch <= L'z') || (wch >= L'A' && wch <= L'Z'));
}

// Printable ASCII other than a letter: the characters a Korean syllable is followed by in the document.
constexpr bool is_korean_text_key(wchar_t wch) {
    return wch >= 0x20 && wch <= 0x7E && !((wch >= L'a' && wch <= L'z') || (wch >= L'A' && wch <= L'Z'));
}

constexpr bool is_korean_caret_or_edit_key(uint32_t vk) {
    switch (vk) {
    case 0x09: // Tab
    case 0x0D: // Enter
    case 0x21: // Page Up
    case 0x22: // Page Down
    case 0x23: // End
    case 0x24: // Home
    case 0x25: // Left
    case 0x26: // Up
    case 0x27: // Right
    case 0x28: // Down
    case 0x2D: // Insert
    case 0x2E: // Delete
        return true;
    default:
        return false;
    }
}

// A key the open Hanja list takes, other than the Hanja key itself.
constexpr bool is_korean_hanja_list_key(uint32_t vk, wchar_t wch) {
    return msime::windows::is_korean_hanja_list_key(vk, static_cast<uint32_t>(wch));
}

// Modifier chords are resolved before this: Ctrl, Alt and Windows combinations belong to the application. `hanja_list_open` is whether the composing syllable's Hanja list is open; with it false every key behaves as it did before Hanja conversion existed.
constexpr KoreanKeyAction korean_key_action(uint32_t vk, wchar_t wch, bool composing, bool hanja_list_open = false) {
    if (is_korean_letter_key(vk, wch))
        return KoreanKeyAction::Compose;
    if (vk == kVirtualKeyHanja)
        return composing ? KoreanKeyAction::ConvertHanja : KoreanKeyAction::Pass;
    if (!composing)
        return is_korean_text_key(wch) ? KoreanKeyAction::Pass : KoreanKeyAction::Default;
    if (hanja_list_open && is_korean_hanja_list_key(vk, wch))
        return KoreanKeyAction::HanjaList;
    if (vk == 0x08 || vk == 0x1B)
        return KoreanKeyAction::Default;
    // Numpad keys carry their character too, so they are text; Enter and Tab carry control characters and are not.
    if (is_korean_text_key(wch))
        return KoreanKeyAction::CommitWithText;
    if (is_korean_caret_or_edit_key(vk))
        return KoreanKeyAction::CommitAndPass;
    return KoreanKeyAction::Default;
}

// The letter the Engine receives: Shift decides the case, and with it the tense consonants and ㅒ ㅖ, whatever Caps Lock says.
constexpr wchar_t korean_letter(wchar_t wch, bool shift) {
    const wchar_t lower = (wch >= L'A' && wch <= L'Z') ? static_cast<wchar_t>(wch - L'A' + L'a') : wch;
    return shift && lower >= L'a' && lower <= L'z' ? static_cast<wchar_t>(lower - L'a' + L'A') : lower;
}
} // namespace msime::tsf
