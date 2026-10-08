#!/usr/bin/env python3
"""检查 Android 递归控件启用策略是否集中在共享 ViewPolicy。"""
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VIEW_POLICY = ROOT / "platforms/android/java/app/msime/android/ViewPolicy.java"
LOGIN_SHEET = ROOT / "platforms/android/java/app/msime/android/home/LoginSheet.java"
INPUT_SERVICE = ROOT / "platforms/android/java/app/msime/android/core/MSIMEInputService.java"


def main() -> None:
    view_policy = VIEW_POLICY.read_text(encoding="utf-8")
    login_sheet = LOGIN_SHEET.read_text(encoding="utf-8")
    input_service = INPUT_SERVICE.read_text(encoding="utf-8")
    required = (
        "public static void setEnabledRecursively(ViewGroup group, boolean enabled, float inactiveAlpha)",
        "if (child instanceof ViewGroup nested && !child.isClickable())",
        "setEnabledRecursively(nested, enabled, inactiveAlpha);",
        "setEnabledWithAlpha(child, enabled, inactiveAlpha);",
        "public static void setFixedHeight(View view, int height)",
        "if (params == null || params.height == height) return;",
        "public static boolean isVisible(View view)",
        "return view != null && view.getVisibility() == View.VISIBLE;",
    )
    missing = [snippet for snippet in required if snippet not in view_policy]
    if missing:
        raise AssertionError("ViewPolicy 缺少递归启用策略：" + ", ".join(missing))
    if "private static void setEnabled(ViewGroup group, boolean enabled)" in login_sheet:
        raise AssertionError("LoginSheet 仍保留重复的递归启用实现")
    if "ViewPolicy.setEnabledRecursively(options, enabled, 0.6f);" not in login_sheet:
        raise AssertionError("LoginSheet 没有调用共享递归启用策略")
    if "private static void setFixedHeight(View view, int height)" in input_service:
        raise AssertionError("MSIMEInputService 仍保留重复的固定高度实现")
    if "ViewPolicy.setFixedHeight(candidateLine, pixels(line));" not in input_service:
        raise AssertionError("MSIMEInputService 没有调用共享固定高度策略")
    if "private static boolean shown(View view)" in input_service:
        raise AssertionError("MSIMEInputService 仍保留重复的可见性判断")
    if "ViewPolicy.isVisible(" not in input_service:
        raise AssertionError("MSIMEInputService 没有调用共享可见性策略")
    print("android view policy: recursive enabled state is shared")


if __name__ == "__main__":
    main()
