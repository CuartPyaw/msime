#!/usr/bin/env python3
"""验证语音配置直接复用共享 JSON 字符串策略。"""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "platforms/android/java/app/msime/android/voice/VoiceConfiguration.java"
SMOKE = ROOT / "platforms/android/tests/voice/VoiceConfigurationSmoke.java"


def main() -> None:
    config = CONFIG.read_text()
    smoke = SMOKE.read_text()
    errors = []
    if "static String strictString(Object value)" in config:
        errors.append(f"{CONFIG}: 仍保留 JSON 字符串转发方法")
    if "JsonPolicy.strictString(value)" not in config:
        errors.append(f"{CONFIG}: 没有直接使用共享 JSON 字符串策略")
    if 'getDeclaredMethod("strictString"' in smoke:
        errors.append(f"{SMOKE}: 仍通过语音配置转发方法验证字符串策略")
    if "JsonPolicy.strictString" not in smoke:
        errors.append(f"{SMOKE}: 没有直接验证共享 JSON 字符串策略")
    if errors:
        for error in errors:
            print(error)
        raise SystemExit(1)
    print("Android voice configuration uses the shared JSON string policy")


if __name__ == "__main__":
    main()
