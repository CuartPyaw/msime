#!/usr/bin/env python3
"""Android 设置组件直接复用共享几何换算并移除死转发。"""

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
HOME = ROOT / "platforms/android/java/app/msime/android/home"
UI = HOME / "Ui.java"
EXPECTED_CALLERS = (
    "AiSkinPage.java",
    "FeedbackPage.java",
    "InputDialog.java",
    "LoginSheet.java",
    "OnboardingActivity.java",
    "SkinsPage.java",
)
HEIGHT_PX_CALLERS = (
    "FeedbackPage.java",
    "OnboardingActivity.java",
    "SkinsPage.java",
)


def main() -> int:
    errors = []
    ui = UI.read_text(encoding="utf-8")
    if "public static int atLeastOnePx(Context context, float value)" in ui:
        errors.append(f"{UI}: 不应保留 atLeastOnePx 转发方法")
    if "public static float sp(Context context, float value)" in ui:
        errors.append(f"{UI}: 不应保留无调用方的 sp 转发方法")
    if "public static int hairlinePx(Context context)" in ui:
        errors.append(f"{UI}: 不应保留 hairlinePx 转发方法")
    if "public static LinearLayout.LayoutParams matchWidthHeightPx(" in ui:
        errors.append(f"{UI}: 不应保留 matchWidthHeightPx 转发方法")
    if re.search(r"(?<![.\w])matchWidthHeightPx\(", ui):
        errors.append(f"{UI}: 不应调用已移除的 matchWidthHeightPx")
    if "public static LinearLayout.LayoutParams weightedWidth(" in ui:
        errors.append(f"{UI}: 不应保留 weightedWidth 转发方法")
    if re.search(r"(?<![.\w])weightedWidth\(", ui):
        errors.append(f"{UI}: 不应调用已移除的 weightedWidth")

    for path in HOME.glob("*.java"):
        source = path.read_text(encoding="utf-8")
        if "Ui.atLeastOnePx(" in source:
            errors.append(f"{path}: 应直接调用 KeyboardGeometry.atLeastOnePixel")
        if "Ui.sp(" in source:
            errors.append(f"{path}: 不应调用已移除的 Ui.sp")
        if "Ui.hairlinePx(" in source:
            errors.append(f"{path}: 应直接调用 KeyboardGeometry.atLeastOnePixel")
        if "Ui.matchWidthHeightPx(" in source:
            errors.append(f"{path}: 应直接调用 KeyboardGeometry.matchWidthHeightPx")
        if "Ui.weightedWidth(" in source:
            errors.append(f"{path}: 应直接调用 KeyboardGeometry.weightedWidthParams")

    for name in EXPECTED_CALLERS:
        path = HOME / name
        if "KeyboardGeometry.atLeastOnePixel(" not in path.read_text(encoding="utf-8"):
            errors.append(f"{path}: 未直接复用 KeyboardGeometry.atLeastOnePixel")

    for name in HEIGHT_PX_CALLERS:
        path = HOME / name
        if "KeyboardGeometry.matchWidthHeightPx(" not in path.read_text(encoding="utf-8"):
            errors.append(f"{path}: 未直接复用 KeyboardGeometry.matchWidthHeightPx")

    option_sheet = HOME / "OptionSheet.java"
    if "KeyboardGeometry.weightedWidthParams(" not in option_sheet.read_text(encoding="utf-8"):
        errors.append(f"{option_sheet}: 未直接复用 KeyboardGeometry.weightedWidthParams")

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("Android home components use shared geometry without forwarding methods")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
