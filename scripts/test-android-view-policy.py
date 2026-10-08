#!/usr/bin/env python3
"""检查 Android 递归控件启用策略是否集中在共享 ViewPolicy。"""
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VIEW_POLICY = ROOT / "platforms/android/java/app/msime/android/ViewPolicy.java"
LOGIN_SHEET = ROOT / "platforms/android/java/app/msime/android/home/LoginSheet.java"


def main() -> None:
    view_policy = VIEW_POLICY.read_text(encoding="utf-8")
    login_sheet = LOGIN_SHEET.read_text(encoding="utf-8")
    required = (
        "public static void setEnabledRecursively(ViewGroup group, boolean enabled, float inactiveAlpha)",
        "if (child instanceof ViewGroup nested && !child.isClickable())",
        "setEnabledRecursively(nested, enabled, inactiveAlpha);",
        "setEnabledWithAlpha(child, enabled, inactiveAlpha);",
    )
    missing = [snippet for snippet in required if snippet not in view_policy]
    if missing:
        raise AssertionError("ViewPolicy 缺少递归启用策略：" + ", ".join(missing))
    if "private static void setEnabled(ViewGroup group, boolean enabled)" in login_sheet:
        raise AssertionError("LoginSheet 仍保留重复的递归启用实现")
    if "ViewPolicy.setEnabledRecursively(options, enabled, 0.6f);" not in login_sheet:
        raise AssertionError("LoginSheet 没有调用共享递归启用策略")
    print("android view policy: recursive enabled state is shared")


if __name__ == "__main__":
    main()
