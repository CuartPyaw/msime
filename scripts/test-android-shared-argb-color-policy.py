#!/usr/bin/env python3
"""验证 Android 颜色输出统一复用共享 ARGB/RGB 策略。"""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
COLOR = ROOT / "platforms/android/java/app/msime/android/ColorPolicy.java"
PALETTE = ROOT / "platforms/android/java/app/msime/android/settings/AppThemePalette.java"
SKIN = ROOT / "platforms/android/java/app/msime/android/keyboard/KeyboardSkin.java"


def main() -> None:
    color = COLOR.read_text()
    palette = PALETTE.read_text()
    skin = SKIN.read_text()
    errors = []
    if "public static String hexArgb(int color)" not in color:
        errors.append(f"{COLOR}: 缺少共享 ARGB 颜色格式化方法")
    if "ColorPolicy.hexArgb(color)" not in palette:
        errors.append(f"{PALETTE}: 没有调用共享 ARGB 颜色策略")
    if "ColorPolicy.hexRgb(argb & 0xFFFFFF)" not in skin:
        errors.append(f"{SKIN}: 不透明颜色没有调用共享 RGB 策略")
    if "ColorPolicy.hexArgb(argb)" not in skin:
        errors.append(f"{SKIN}: 带 alpha 颜色没有调用共享 ARGB 策略")
    if 'String.format(Locale.ROOT, "#%08X"' in palette or 'String.format(Locale.ROOT, "#%06X"' in skin:
        errors.append("颜色宿主仍保留重复的十六进制格式化逻辑")
    if errors:
        for error in errors:
            print(error)
        raise SystemExit(1)
    print("Android palette and keyboard skins share the ARGB color policy")


if __name__ == "__main__":
    main()
