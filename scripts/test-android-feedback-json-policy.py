#!/usr/bin/env python3
"""Android 键盘反馈设置直接复用共享的严格布尔解析策略。"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
STORE = ROOT / "platforms/android/java/app/msime/android/KeyboardFeedbackStore.java"
SMOKE = ROOT / "platforms/android/tests/keyboard/KeyboardFeedbackStoreSmoke.java"


def main() -> int:
    store = STORE.read_text(encoding="utf-8")
    smoke = SMOKE.read_text(encoding="utf-8")
    errors = []
    if "static Boolean strictBoolean(" in store:
        errors.append(f"{STORE}: 仍保留 strictBoolean 局部包装")
    if "static boolean booleanValue(" in store:
        errors.append(f"{STORE}: 仍保留 booleanValue 局部包装")
    if store.count("JsonPolicy.strictBoolean(") != 2:
        errors.append(f"{STORE}: fromValues 没有直接调用共享 strictBoolean 两次")
    if "KeyboardFeedbackStore.booleanValue(" in smoke:
        errors.append(f"{SMOKE}: smoke 仍调用已删除的局部 booleanValue")
    if "KeyboardFeedbackStore.strictBoolean(" in smoke:
        errors.append(f"{SMOKE}: smoke 仍调用已删除的局部 strictBoolean")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("Android feedback settings use the shared strict JSON boolean policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
