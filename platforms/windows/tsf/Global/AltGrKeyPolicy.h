#pragma once

namespace Global {
// The modifiers a key is classified and sent with, from the IPC bits (Shift 1, Ctrl 2, Alt 4) and the character the keyboard layout produced with the full key state. AltGr is reported as Ctrl+Alt (Windows synthesizes a left Ctrl for the right Alt), so a Ctrl+Alt chord that still produces a printable character is AltGr typing that character, as AltGr+0 types '@' on AZERTY, not an application shortcut: Ctrl and Alt are dropped so the key is input and the Server takes it as typing. Shift stays. A Ctrl+Alt chord with no character, a control character (Ctrl+letter) or a space (the Ctrl+Alt+Space input hotkey) keeps both and stays a shortcut.
inline unsigned CharacterModifiers(unsigned modifiers, wchar_t character)
{
    const bool altGr = (modifiers & 0b110u) == 0b110u;
    const bool printable = character > 0x20 && character != 0x7F;
    return altGr && printable ? (modifiers & ~0b110u) : modifiers;
}
} // namespace Global
