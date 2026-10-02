#pragma once

namespace msime::mac
{
enum class MaintenanceShortcutAction
{
    None,
    ClearCache,
    Restart,
    Terminate,
};

// InputMethodKit only receives events for the active input context.  Preserve
// the Windows maintenance keys while replacing Alt with macOS Option and using
// physical ANSI key codes so keyboard-layout characters cannot change them.
constexpr MaintenanceShortcutAction PhysicalMaintenanceShortcut(unsigned short keyCode, bool control, bool shift,
                                                                  bool option, bool command)
{
    if (!control || !shift || !option || command)
        return MaintenanceShortcutAction::None;
    switch (keyCode)
    {
    case 8: return MaintenanceShortcutAction::ClearCache; // C
    case 15: return MaintenanceShortcutAction::Restart;   // R
    case 17: return MaintenanceShortcutAction::Terminate; // T
    default: return MaintenanceShortcutAction::None;
    }
}

// Main-row ANSI digit key codes used for candidate selection.  These helpers
// live in the Engine-facing namespace so InputController.mm can include them
// alongside CandidateSkin.h, which reserves metasequoia::mac as an alias.
constexpr int PhysicalCandidateDigitSlot(unsigned short keyCode)
{
    switch (keyCode)
    {
    case 18: return 0; // 1
    case 19: return 1; // 2
    case 20: return 2; // 3
    case 21: return 3; // 4
    case 23: return 4; // 5
    case 22: return 5; // 6
    case 26: return 6; // 7
    case 28: return 7; // 8
    case 25: return 8; // 9
    // AppKit reports the physical ANSI keypad digits separately from the
    // number row. Windows normalizes VK_NUMPAD1..9 before candidate routing;
    // keep the same selection contract on macOS without accepting keypad 0.
    case 83: return 0; // kVK_ANSI_Keypad1
    case 84: return 1; // kVK_ANSI_Keypad2
    case 85: return 2; // kVK_ANSI_Keypad3
    case 86: return 3; // kVK_ANSI_Keypad4
    case 87: return 4; // kVK_ANSI_Keypad5
    case 88: return 5; // kVK_ANSI_Keypad6
    case 89: return 6; // kVK_ANSI_Keypad7
    case 91: return 7; // kVK_ANSI_Keypad8
    case 92: return 8; // kVK_ANSI_Keypad9
    default: return -1;
    }
}

// Candidate digits are a controller shortcut only for an ordinary candidate panel. A digit the Engine lists in the view's spelling_symbols is input - the code point being typed in Unicode mode, the number in expression mode - while nine-key mode and modified chords belong to the Engine.
constexpr bool ShouldRoutePhysicalCandidateDigit(bool candidatePanelVisible, bool nineKeyMode, bool digitIsSpelling,
                                                   bool modified)
{
    return candidatePanelVisible && !nineKeyMode && !digitIsSpelling && !modified;
}

// A mode that spells with digits still has to let the user reach the second candidate.
//
// Its digits are what is being typed, so the reference moves selection onto Shift+digit, which cannot be part of a code point: `Shift + 数字 选其他候选` in the Unicode mode's own documentation. Without it the only candidate a keyboard can commit there is the first one. Expression mode also spells some shifted digits - ( ) % ^ * on a US layout - and a Shift+digit whose character the mode takes stays input; only the others select.
constexpr bool ShouldRouteSpellingShiftCandidateDigit(bool candidatePanelVisible, bool digitIsSpelling, bool shiftOnly,
                                                        bool shiftedIsSpelling)
{
    return candidatePanelVisible && digitIsSpelling && shiftOnly && !shiftedIsSpelling;
}

// The digit a candidate slot's physical key types, so it can be looked up in spelling_symbols whatever the keyboard layout puts on that key.
constexpr char PhysicalCandidateDigitCharacter(int slot)
{
    return slot >= 0 && slot <= 8 ? static_cast<char>('1' + slot) : '\0';
}

// The key class msime_client_key_sound takes: 1 space, 2 enter (main or keypad), 3 backspace, 0 any other key.
constexpr unsigned PhysicalKeySoundClass(unsigned short keyCode)
{
    switch (keyCode)
    {
    case 49: return 1; // kVK_Space
    case 36: // kVK_Return
    case 76: // kVK_ANSI_KeypadEnter
        return 2;
    case 51: return 3; // kVK_Delete
    default: return 0;
    }
}

constexpr bool IsKeypadDecimal(unsigned short keyCode)
{
    return keyCode == 65; // kVK_ANSI_KeypadDecimal
}

// Physical macOS keypad punctuation.  Keep this independent of the active
// keyboard layout so keypad operators cannot fall into the main-row paging
// shortcuts (notably '-' and '=').
constexpr char KeypadPunctuation(unsigned short keyCode)
{
    switch (keyCode)
    {
    case 65: return '.'; // kVK_ANSI_KeypadDecimal
    case 67: return '*'; // kVK_ANSI_KeypadMultiply
    case 69: return '+'; // kVK_ANSI_KeypadPlus
    case 75: return '/'; // kVK_ANSI_KeypadDivide
    case 78: return '-'; // kVK_ANSI_KeypadMinus
    case 81: return '='; // kVK_ANSI_KeypadEquals
    case 95: return ','; // kVK_JIS_KeypadComma / keypad separator
    default: return '\0';
    }
}

constexpr bool IsJapaneseMinusEqualInput(int scheme, bool temporaryJapanese, char character)
{
    return (scheme == 3 || temporaryJapanese) && (character == '-' || character == '=');
}

// Candidate paging is configured by characters, but Japanese input owns the
// physical ANSI minus/equal keys.  AppKit's charactersIgnoringModifiers can
// change with the active keyboard layout, so retain the character fallback for
// synthetic/older events while preferring the physical key codes in real input.
constexpr bool IsJapaneseMinusEqualKey(int scheme, bool temporaryJapanese, unsigned short keyCode, char character)
{
    if (scheme != 3 && !temporaryJapanese) return false;
    // Real AppKit events must identify the physical ANSI key.  Keep the
    // keyCode==0 character fallback for older synthetic tests/events only;
    // accepting a punctuation character from another physical key would
    // diverge from the Windows virtual-key contract.
    return keyCode == 24 || keyCode == 27 ||
           (keyCode == 0 && (character == '-' || character == '='));
}

// Candidate paging follows Windows' physical virtual-key policy.  AppKit's
// charactersIgnoringModifiers varies with the active keyboard layout, so the
// key code—not the produced glyph—selects the binding.  A zero result means
// that the key is not one of the paging keys; -1 is previous and +1 is next.
constexpr int PhysicalCandidatePageDirection(unsigned short keyCode)
{
    switch (keyCode)
    {
    case 27: // ANSI '-'
    case 43: // ANSI ','
    case 33: // ANSI '['
    case 116: // Page Up
        return -1;
    case 24: // ANSI '='
    case 47: // ANSI '.'
    case 30: // ANSI ']'
    case 121: // Page Down
        return 1;
    default:
        return 0;
    }
}

// Word-to-character is stricter than paging: both the physical key and its
// expected unshifted punctuation must match.  This prevents a keyboard layout
// from making an unrelated physical key with the same glyph select an edge.
constexpr bool IsPhysicalWordCharacterKey(unsigned short keyCode, bool brackets, char character)
{
    if (brackets)
        return (keyCode == 33 && character == '[') || (keyCode == 30 && character == ']');
    return (keyCode == 27 && character == '-') || (keyCode == 24 && character == '=');
}

// The Engine's `SchemeType::Korean`, as it appears in a view's `scheme`.
constexpr int KoreanScheme = 4;

// Dubeolsik binds jamo to letters by case: Shift+Q/W/E/R/T/O/P type ㅃ ㅉ ㄸ ㄲ ㅆ ㅒ ㅖ, and every other letter types the same jamo either way. So the case the Engine receives is Shift's alone. AppKit folds Caps Lock into the typed character, and passing that on would turn r (ㄱ) into R (ㄲ) for a user who only left Caps Lock on. Anything that is not an ASCII letter comes back unchanged.
constexpr char KoreanKeyLetter(char character, bool shift)
{
    if (character >= 'A' && character <= 'Z') character = static_cast<char>(character - 'A' + 'a');
    else if (character < 'a' || character > 'z') return character;
    return shift ? static_cast<char>(character - 'a' + 'A') : character;
}

// The Engine's `CandidateSource::Fallback`, as it appears in a view candidate's `source`.
constexpr int CandidateSourceFallback = 9;

// A lone Fallback row is the raw composition the Engine shows when there is nothing to convert (a bare Shift+R prefix, or romaji it cannot read). Windows commits it on the first Space, so Japanese Space must not arm a conversion on it.
constexpr bool JapaneseSpaceCommitsFallback(unsigned long candidateCount, int firstCandidateSource)
{
    return candidateCount == 1 && firstCandidateSource == CandidateSourceFallback;
}
} // namespace msime::mac
