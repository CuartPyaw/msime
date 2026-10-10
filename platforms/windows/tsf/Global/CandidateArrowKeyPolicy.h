#pragma once
#include <string_view>

namespace Global
{
// 横排候选窗里 ←/→ 沿着候选的排列方向，所以用来移动候选高亮，与 macOS 相同；竖排时 ←/→ 仍然移动组字光标，↑/↓ 移动高亮。只有打开了「方向键翻选」（navigation.arrows，文档里没有这一项时为开，与 Server 的 preference_navigation 一致）才这样做：关掉时方向键归组字光标，macOS 也是如此。
inline bool CandidateArrowsFollowHorizontalLayout(std::string_view candidateLayout, bool arrowsNavigation)
{
    return arrowsNavigation && candidateLayout == "horizontal";
}

// 这次按下的 ←/→ 是否移动候选高亮。`candidatesShown` 是候选窗开着，`horizontalArrows` 是上面那条的结果，`inlineScheme` 是组字一直写在行内的方案（韩文、注音、越南文、藏文，scheme::AlwaysInlinePreedit）：它们没有常驻的候选窗，←/→ 照旧交给组字。
inline bool ArrowKeyMovesCandidateHighlight(unsigned virtualKey, bool candidatesShown, bool horizontalArrows,
                                            bool inlineScheme)
{
    constexpr unsigned left = 0x25;
    constexpr unsigned right = 0x27;
    return (virtualKey == left || virtualKey == right) && candidatesShown && horizontalArrows && !inlineScheme;
}

// 发给 Server 的键码。Server 的导航只认 ↑/↓ 是上一个 / 下一个候选（src/input/NavigationPolicy.h），所以移动高亮的 ← 按 ↑、→ 按 ↓ 发过去；其他键和不移动高亮的方向键原样发送。
inline unsigned CandidateNavigationWireKey(unsigned virtualKey, bool movesHighlight)
{
    if (!movesHighlight)
        return virtualKey;
    if (virtualKey == 0x25)
        return 0x26;
    if (virtualKey == 0x27)
        return 0x28;
    return virtualKey;
}
} // namespace Global
