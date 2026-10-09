#!/usr/bin/env python3
"""本地语音识别直接复用共享 JSON 布尔策略。"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/voice/LocalAsrPolicy.java"
SMOKE = ROOT / "platforms/android/tests/voice/LocalAsrPolicySmoke.java"


def main() -> int:
    source = SOURCE.read_text(encoding="utf-8")
    smoke = SMOKE.read_text(encoding="utf-8")
    if "strictBoolean(Object value)" in source:
        print(f"{SOURCE}: 不应保留 strictBoolean 转发方法", file=sys.stderr)
        return 1
    if "getDeclaredMethod(\"strictBoolean\"" in smoke:
        print(f"{SMOKE}: 不应反射检查已移除的转发方法", file=sys.stderr)
        return 1
    if "JsonPolicy.strictBoolean" not in smoke:
        print(f"{SMOKE}: 缺少 JsonPolicy.strictBoolean 合同检查", file=sys.stderr)
        return 1
    print("Android local ASR uses the shared JSON boolean policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
