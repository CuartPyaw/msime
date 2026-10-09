#!/usr/bin/env python3
"""遥测状态直接复用共享 JSON 布尔策略。"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "platforms/android/java/app/msime/android/core/Telemetry.java"
SMOKE = ROOT / "platforms/android/tests/core/TelemetryHandlerSmoke.java"


def main() -> int:
    source = SOURCE.read_text(encoding="utf-8")
    smoke = SMOKE.read_text(encoding="utf-8")
    if "booleanValue(" in source:
        print(f"{SOURCE}: 不应保留或调用 booleanValue 转发方法", file=sys.stderr)
        return 1
    if "Telemetry.booleanValue" in smoke:
        print(f"{SMOKE}: 不应继续通过遥测类检查 JSON 布尔值", file=sys.stderr)
        return 1
    if source.count("JsonPolicy.strictBoolean") < 2:
        print(f"{SOURCE}: 遥测状态应直接调用 JsonPolicy.strictBoolean", file=sys.stderr)
        return 1
    if "import app.msime.android.JsonPolicy;" not in smoke:
        print(f"{SMOKE}: 应直接导入 JsonPolicy", file=sys.stderr)
        return 1
    if smoke.count("JsonPolicy.strictBoolean") < 2:
        print(f"{SMOKE}: 缺少共享 JSON 布尔策略合同检查", file=sys.stderr)
        return 1
    print("Android 遥测状态已复用共享 JSON 布尔策略")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
