#include "../../Global/AltGrKeyPolicy.h"
#include <cstdio>

namespace {
int failures = 0;

void check(bool condition, const char *what) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", what);
        ++failures;
    }
}
} // namespace

int main() {
    using namespace Global;

    // AltGr arrives as Ctrl+Alt. A character it produces is typing, not a shortcut.
    check(CharacterModifiers(0b110u, L'@') == 0u, "AZERTY AltGr+0 types @");
    check(CharacterModifiers(0b110u, L'{') == 0u, "German AltGr+7 types {");
    check(CharacterModifiers(0b110u, L'\u20AC') == 0u, "AltGr+E types the euro sign");
    check(CharacterModifiers(0b111u, L'|') == 1u, "Shift stays with an AltGr character");

    // Without a printable character the chord is still a shortcut.
    check(CharacterModifiers(0b110u, L'\0') == 0b110u, "Ctrl+Alt+letter with no character is a shortcut");
    check(CharacterModifiers(0b110u, L' ') == 0b110u, "Ctrl+Alt+Space is the input hotkey");
    check(CharacterModifiers(0b110u, wchar_t(0x01)) == 0b110u, "a control character is a shortcut");
    check(CharacterModifiers(0b110u, wchar_t(0x7F)) == 0b110u, "DEL is a shortcut");

    // Ctrl or Alt alone is never AltGr, whatever the layout produced.
    check(CharacterModifiers(0b010u, L'a') == 0b010u, "Ctrl alone stays");
    check(CharacterModifiers(0b100u, L'a') == 0b100u, "Alt alone stays");
    check(CharacterModifiers(0b011u, L'A') == 0b011u, "Ctrl+Shift stays");
    check(CharacterModifiers(0u, L'a') == 0u, "a plain key has no modifiers");
    check(CharacterModifiers(1u, L'A') == 1u, "Shift alone stays");

    if (failures != 0)
        return 1;
    std::puts("altgr key policy: ok");
    return 0;
}
