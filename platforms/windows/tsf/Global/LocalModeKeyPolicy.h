#pragma once
#include <cstddef>

namespace Global
{
// The digits and operators of the V mode: crates/engine/src/local/expression.rs SPELLING_SYMBOLS. The Server reads them from View.spelling_symbols, but the TIP classifies a key before the Server answers, so it keeps this copy; scripts/test-windows-expression-symbols-parity.py keeps the two equal.
inline constexpr wchar_t ExpressionSpellingSymbols[] = L"0123456789+-*/.()%^";

inline bool IsExpressionSpellingSymbol(wchar_t wch)
{
    if (wch == L'\0')
    {
        return false;
    }
    for (const wchar_t *symbol = ExpressionSpellingSymbols; *symbol; ++symbol)
    {
        if (*symbol == wch)
        {
            return true;
        }
    }
    return false;
}

// A V composition: the keystroke buffer starts with the V that opened it, and the Server reports the mode on. With the mode off, Shift+V is an ordinary capital and its digits keep selecting.
inline bool IsExpressionModeComposition(const wchar_t *buffer, size_t length, bool expressionMode)
{
    return expressionMode && buffer && length > 0 && buffer[0] == L'V';
}

enum class ExpressionKey
{
    Unclaimed,
    Input,
    SelectByNumber,
};

// What a key without Ctrl or Alt does inside a V composition. The text the mode spells is input, including the operators typed with Shift ('*' is Shift+8) and the minus key that pages elsewhere; a digit key printing anything else (Shift+1's '!' on a US layout, or a bare digit key on a layout whose digit row needs Shift) picks the row in its slot. The Server's edit_kind and digit_selects_candidate (src/input/EditPolicy.h) decide the same key the same way, so the two never disagree about whether it was a selection.
inline ExpressionKey ClassifyExpressionKey(unsigned code, wchar_t wch)
{
    if (IsExpressionSpellingSymbol(wch))
    {
        return ExpressionKey::Input;
    }
    if (code >= L'1' && code <= L'9')
    {
        return ExpressionKey::SelectByNumber;
    }
    return ExpressionKey::Unclaimed;
}

// Whether this key opens the "/" or "@" mode: on an empty composition, with Chinese punctuation in force (with ASCII punctuation the key is the literal mark the user chose, and the Engine opens nothing), and only for a mode the Server reports on. The key then starts the composition, and the Engine takes it as the mode's first character.
inline bool OpensLocalMode(wchar_t wch, bool composing, bool chinesePunctuation, bool commandMode, bool mentionMode)
{
    if (composing || !chinesePunctuation)
    {
        return false;
    }
    return (wch == L'/' && commandMode) || (wch == L'@' && mentionMode);
}

// The Server's LocalModeTriggersChanged payload: three '0'/'1' flags, V then "/" then "@". Anything else turns all three off, which routes every key as it was routed before the modes existed.
struct LocalModeTriggers
{
    bool expression = false;
    bool command = false;
    bool mention = false;
};

inline LocalModeTriggers ParseLocalModeTriggers(const wchar_t *payload, size_t capacity)
{
    LocalModeTriggers triggers;
    if (!payload || capacity < 4 || payload[3] != L'\0')
    {
        return triggers;
    }
    for (size_t index = 0; index < 3; ++index)
    {
        if (payload[index] != L'0' && payload[index] != L'1')
        {
            return triggers;
        }
    }
    triggers.expression = payload[0] == L'1';
    triggers.command = payload[1] == L'1';
    triggers.mention = payload[2] == L'1';
    return triggers;
}
} // namespace Global
