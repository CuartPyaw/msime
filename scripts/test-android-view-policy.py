#!/usr/bin/env python3
"""检查 Android 递归控件启用策略是否集中在共享 ViewPolicy。"""
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VIEW_POLICY = ROOT / "platforms/android/java/app/msime/android/ViewPolicy.java"
LOGIN_SHEET = ROOT / "platforms/android/java/app/msime/android/home/LoginSheet.java"
INPUT_SERVICE = ROOT / "platforms/android/java/app/msime/android/core/MSIMEInputService.java"
BOTTOM_BAR = ROOT / "platforms/android/java/app/msime/android/core/ImeBottomBar.java"
UI = ROOT / "platforms/android/java/app/msime/android/home/Ui.java"


def main() -> None:
    view_policy = VIEW_POLICY.read_text(encoding="utf-8")
    login_sheet = LOGIN_SHEET.read_text(encoding="utf-8")
    input_service = INPUT_SERVICE.read_text(encoding="utf-8")
    bottom_bar = BOTTOM_BAR.read_text(encoding="utf-8")
    ui = UI.read_text(encoding="utf-8")
    required = (
        "public static void setEnabledRecursively(ViewGroup group, boolean enabled, float inactiveAlpha)",
        "if (child instanceof ViewGroup nested && !child.isClickable())",
        "setEnabledRecursively(nested, enabled, inactiveAlpha);",
        "setEnabledWithAlpha(child, enabled, inactiveAlpha);",
        "public static void setFixedHeight(View view, int height)",
        "if (params == null || params.height == height) return;",
        "public static boolean isVisible(View view)",
        "return view != null && view.getVisibility() == View.VISIBLE;",
        "public static void setPaddingIfChanged(View view, int left, int top, int right, int bottom)",
        "if (view.getPaddingLeft() == left && view.getPaddingTop() == top",
        "public static void setVisibleIfChanged(View view, boolean visible)",
        "if (view.getVisibility() == visibility) return;",
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
    if "private static void setPadding(View view, int left, int top, int right, int bottom)" in bottom_bar:
        raise AssertionError("ImeBottomBar 仍保留重复的条件内边距实现")
    if "ViewPolicy.setPaddingIfChanged(keyboard, 0, 0, 0, 0);" not in bottom_bar:
        raise AssertionError("ImeBottomBar 没有调用共享条件内边距策略")
    if "if (bar != null && (bar.getVisibility() == View.VISIBLE) != shown) ViewPolicy.setVisible(bar, shown);" in bottom_bar:
        raise AssertionError("ImeBottomBar 仍保留重复的条件可见性实现")
    if "ViewPolicy.setVisibleIfChanged(bar, shown);" not in bottom_bar:
        raise AssertionError("ImeBottomBar 没有调用共享条件可见性策略")
    if "view.setMinHeight(dp(context, heightDp));" in ui:
        raise AssertionError("Ui 仍直接实现文本最小高度策略")
    if "ViewPolicy.setTextMinHeight(view, dp(context, heightDp));" not in ui:
        raise AssertionError("Ui 没有调用共享文本最小高度策略")
    if "view.setMinWidth(dp(context, widthDp));" in ui:
        raise AssertionError("Ui 仍直接实现文本最小宽度策略")
    if "ViewPolicy.setTextMinWidth(view, dp(context, widthDp));" not in ui:
        raise AssertionError("Ui 没有调用共享文本最小宽度策略")
    print("android view policy: recursive enabled state is shared")


if __name__ == "__main__":
    main()
