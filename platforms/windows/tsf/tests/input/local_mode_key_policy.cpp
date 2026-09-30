#include "../../Global/LocalModeKeyPolicy.h"
#include <cstdio>
#include <string>

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

    // The V mode is a buffer that starts with the V that opened it, and only while the Server reports the mode on.
    const std::wstring expression = L"V1+2";
    check(IsExpressionModeComposition(expression.c_str(), expression.size(), true), "V buffer is the mode");
    check(!IsExpressionModeComposition(expression.c_str(), expression.size(), false), "switched off, V is a capital");
    check(!IsExpressionModeComposition(L"U4e2d", 5, true), "U buffer is not V");
    check(!IsExpressionModeComposition(L"nihao", 5, true), "pinyin is not V");
    check(!IsExpressionModeComposition(L"", 0, true), "empty buffer is no mode");
    check(!IsExpressionModeComposition(nullptr, 3, true), "no buffer is no mode");

    // Its digits and operators are input, whichever key typed them.
    for (wchar_t symbol : std::wstring(L"0123456789+-*/.()%^"))
        check(IsExpressionSpellingSymbol(symbol), "every V symbol is spelled");
    for (wchar_t other : std::wstring(L"=!@#$&,;'aV "))
        check(!IsExpressionSpellingSymbol(other), "other text is not spelled");
    check(!IsExpressionSpellingSymbol(L'\0'), "no text is not spelled");
    check(ClassifyExpressionKey(L'4', L'4') == ExpressionKey::Input, "digit composes");
    check(ClassifyExpressionKey(0x64, L'4') == ExpressionKey::Input, "numpad digit composes");
    check(ClassifyExpressionKey(0xBD, L'-') == ExpressionKey::Input, "minus composes instead of paging");
    check(ClassifyExpressionKey(0xBB, L'+') == ExpressionKey::Input, "Shift+= plus composes");
    check(ClassifyExpressionKey(L'8', L'*') == ExpressionKey::Input, "Shift+8 star composes, not the eighth row");
    check(ClassifyExpressionKey(L'9', L'(') == ExpressionKey::Input, "Shift+9 paren composes");
    check(ClassifyExpressionKey(L'5', L'%') == ExpressionKey::Input, "Shift+5 percent composes");
    check(ClassifyExpressionKey(0xBE, L'.') == ExpressionKey::Input, "period composes instead of paging");
    check(ClassifyExpressionKey(0xBF, L'/') == ExpressionKey::Input, "slash composes instead of punctuation");
    // A digit key printing anything else selects its row, with Shift on a US layout or bare on one whose digit row needs Shift.
    check(ClassifyExpressionKey(L'1', L'!') == ExpressionKey::SelectByNumber, "Shift+1 selects");
    check(ClassifyExpressionKey(L'2', L'@') == ExpressionKey::SelectByNumber, "Shift+2 selects");
    check(ClassifyExpressionKey(L'1', L'&') == ExpressionKey::SelectByNumber, "AZERTY bare 1 selects");
    // Everything else keeps its usual route.
    check(ClassifyExpressionKey(L'0', L')') == ExpressionKey::Input, "Shift+0 paren composes");
    check(ClassifyExpressionKey(L'0', 0x00E0) == ExpressionKey::Unclaimed, "zero key printing a letter is not claimed");
    check(ClassifyExpressionKey(0xBB, L'=') == ExpressionKey::Unclaimed, "equals keeps paging");
    check(ClassifyExpressionKey(0xBC, L',') == ExpressionKey::Unclaimed, "comma keeps paging");
    check(ClassifyExpressionKey(L'A', L'a') == ExpressionKey::Unclaimed, "letters keep their route");

    // "/" and "@" open their modes only on an empty composition, with Chinese punctuation, and for a mode that is on.
    check(OpensLocalMode(L'/', false, true, true, false), "slash opens commands");
    check(OpensLocalMode(L'@', false, true, false, true), "at opens mentions");
    check(!OpensLocalMode(L'/', false, true, false, true), "slash needs the command mode");
    check(!OpensLocalMode(L'@', false, true, true, false), "at needs the mention mode");
    check(!OpensLocalMode(L'/', true, true, true, true), "not inside a composition");
    check(!OpensLocalMode(L'/', false, false, true, true), "not with ASCII punctuation");
    check(!OpensLocalMode(L'#', false, true, true, true), "no other mark opens a mode");

    // The Server's frame: three '0'/'1' flags; anything else turns every mode off.
    const auto parse = [](const wchar_t *payload) {
        wchar_t buffer[8] = {};
        for (size_t index = 0; payload[index] && index < 7; ++index)
            buffer[index] = payload[index];
        return ParseLocalModeTriggers(buffer, 8);
    };
    auto triggers = parse(L"101");
    check(triggers.expression && !triggers.command && triggers.mention, "flags in V, /, @ order");
    triggers = parse(L"010");
    check(!triggers.expression && triggers.command && !triggers.mention, "command alone");
    triggers = parse(L"111");
    check(triggers.expression && triggers.command && triggers.mention, "all on");
    for (const wchar_t *invalid : {L"", L"1", L"11", L"1111", L"1x1", L"abc"}) {
        triggers = parse(invalid);
        check(!triggers.expression && !triggers.command && !triggers.mention, "malformed frame is all off");
    }
    const wchar_t short_buffer[3] = {L'1', L'1', L'1'};
    triggers = ParseLocalModeTriggers(short_buffer, 3);
    check(!triggers.expression && !triggers.command && !triggers.mention, "unterminated frame is all off");
    triggers = ParseLocalModeTriggers(nullptr, 8);
    check(!triggers.expression && !triggers.command && !triggers.mention, "no frame is all off");

    if (failures != 0) {
        std::fprintf(stderr, "%d local mode key policy check(s) failed\n", failures);
        return 1;
    }
    std::puts("TSF local mode key policy checks passed");
    return 0;
}
