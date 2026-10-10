#include "../../Global/CandidateArrowKeyPolicy.h"
#include <cstdio>
#include <cstdlib>

namespace {
int failures = 0;

void check(bool condition, const char *what) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", what);
        ++failures;
    }
}

constexpr unsigned kLeft = 0x25;
constexpr unsigned kUp = 0x26;
constexpr unsigned kRight = 0x27;
constexpr unsigned kDown = 0x28;
constexpr unsigned kTab = 0x09;
} // namespace

int main() {
    using namespace Global;

    // 横排且打开方向键翻选时 ←/→ 才归候选；竖排或关闭翻选时归组字光标。
    check(CandidateArrowsFollowHorizontalLayout("horizontal", true), "horizontal layout with arrow navigation");
    check(!CandidateArrowsFollowHorizontalLayout("vertical", true), "vertical layout keeps the caret keys");
    check(!CandidateArrowsFollowHorizontalLayout("horizontal", false), "arrow navigation off keeps the caret keys");
    check(!CandidateArrowsFollowHorizontalLayout("", true), "a missing layout is the vertical default");

    check(ArrowKeyMovesCandidateHighlight(kLeft, true, true, false), "left moves the highlight across a horizontal list");
    check(ArrowKeyMovesCandidateHighlight(kRight, true, true, false), "right moves the highlight across a horizontal list");
    check(!ArrowKeyMovesCandidateHighlight(kLeft, false, true, false), "no candidate list: left moves the caret");
    check(!ArrowKeyMovesCandidateHighlight(kRight, true, false, false), "vertical list: right moves the caret");
    check(!ArrowKeyMovesCandidateHighlight(kLeft, true, true, true), "inline schemes keep their own left");
    check(!ArrowKeyMovesCandidateHighlight(kUp, true, true, false), "up is already a candidate key");
    check(!ArrowKeyMovesCandidateHighlight(kTab, true, true, false), "other keys are untouched");

    // Server 只认 ↑/↓ 作为上一个 / 下一个候选，所以 ← 记作 ↑、→ 记作 ↓。
    check(CandidateNavigationWireKey(kLeft, true) == kUp, "left goes to the Server as up");
    check(CandidateNavigationWireKey(kRight, true) == kDown, "right goes to the Server as down");
    check(CandidateNavigationWireKey(kLeft, false) == kLeft, "a caret move keeps its key");
    check(CandidateNavigationWireKey(kDown, true) == kDown, "down stays down");
    check(CandidateNavigationWireKey(kTab, true) == kTab, "tab stays tab");

    return failures == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
