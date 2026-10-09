#!/usr/bin/env python3
"""豆包识别直接复用共享 JSON 布尔策略。"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/voice/DoubaoAsrPolicy.java"
RECOGNIZER = ROOT / "platforms/android/java/app/msime/android/voice/DoubaoRecognizer.java"
SMOKE = ROOT / "platforms/android/tests/voice/DoubaoAsrPolicySmoke.java"


def main() -> int:
    source = SOURCE.read_text(encoding="utf-8")
    recognizer = RECOGNIZER.read_text(encoding="utf-8")
    smoke = SMOKE.read_text(encoding="utf-8")
    if "strictBoolean(Object value)" in source:
        print(f"{SOURCE}: 不应保留 strictBoolean 转发方法", file=sys.stderr)
        return 1
    if "DoubaoAsrPolicy.strictBoolean" in recognizer:
        print(f"{RECOGNIZER}: 不应继续通过策略类调用 strictBoolean", file=sys.stderr)
        return 1
    if "getDeclaredMethod(\"strictBoolean\"" in smoke:
        print(f"{SMOKE}: 不应反射检查已移除的转发方法", file=sys.stderr)
        return 1
    if "JsonPolicy.strictBoolean" not in recognizer or "JsonPolicy.strictBoolean" not in smoke:
        print(f"{SMOKE}: 缺少 JsonPolicy.strictBoolean 合同检查", file=sys.stderr)
        return 1
    print("Android Doubao uses the shared JSON boolean policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
