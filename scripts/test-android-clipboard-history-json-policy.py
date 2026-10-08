#!/usr/bin/env python3
"""剪贴板历史存储直接复用共享策略层。"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/clipboard/ClipboardHistoryStore.java"
SMOKE = ROOT / "platforms/android/tests/dictionary/ClipboardHistoryPolicySmoke.java"


def main() -> int:
    source = SOURCE.read_text(encoding="utf-8")
    smoke = SMOKE.read_text(encoding="utf-8")
    if "strictBoolean(Object value)" in source:
        print(f"{SOURCE}: 不应保留 strictBoolean 转发方法", file=sys.stderr)
        return 1
    if "ClipboardHistoryStore.class.getDeclaredMethod" in smoke:
        print(f"{SMOKE}: 不应反射检查已删除的存储转发方法", file=sys.stderr)
        return 1
    if "ClipboardHistoryPolicy.strictBoolean" not in smoke:
        print(f"{SMOKE}: 缺少共享布尔策略合同检查", file=sys.stderr)
        return 1
    print("Android clipboard history uses the shared boolean policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
