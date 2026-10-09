#!/usr/bin/env python3
"""诊断接口直接复用共享 JSON 字符串和整数策略。"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/account/DiagnosticsApi.java"
SMOKE = ROOT / "platforms/android/tests/core/DiagnosticsApiSmoke.java"


def main() -> int:
    source = SOURCE.read_text(encoding="utf-8")
    smoke = SMOKE.read_text(encoding="utf-8")
    if "strictString(Object value)" in source:
        print(f"{SOURCE}: 不应保留 strictString 转发方法", file=sys.stderr)
        return 1
    if "strictInteger(Object value)" in source:
        print(f"{SOURCE}: 不应保留 strictInteger 转发方法", file=sys.stderr)
        return 1
    if "DiagnosticsApi.class.getDeclaredMethod(\"strictString\"" in smoke:
        print(f"{SMOKE}: 不应反射检查已删除的字符串转发方法", file=sys.stderr)
        return 1
    if "DiagnosticsApi.strictInteger" in smoke:
        print(f"{SMOKE}: 不应继续通过诊断类调用 strictInteger", file=sys.stderr)
        return 1
    if "JsonPolicy.strictString" not in smoke:
        print(f"{SMOKE}: 缺少 JsonPolicy.strictString 合同检查", file=sys.stderr)
        return 1
    if "JsonPolicy.strictLong" not in source or "JsonPolicy.strictLong" not in smoke:
        print(f"{SMOKE}: 应直接检查 JsonPolicy.strictLong", file=sys.stderr)
        return 1
    print("Android diagnostics use the shared JSON string and integer policies")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
