#!/usr/bin/env python3
"""验证 AI polish 响应直接复用共享文本策略。"""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESPONSE = ROOT / "platforms/android/java/app/msime/android/AiProviderResponse.java"
TRANSPORT = ROOT / "platforms/android/java/app/msime/android/voice/AiPolishHttpTransport.java"
POLISHER = ROOT / "platforms/android/java/app/msime/android/voice/VoicePolisher.java"
TRANSPORT_SMOKE = ROOT / "platforms/android/tests/voice/AiPolishHttpTransportSmoke.java"
POLISHER_SMOKE = ROOT / "platforms/android/tests/voice/VoicePolisherSmoke.java"


def main() -> None:
    response = RESPONSE.read_text()
    transport = TRANSPORT.read_text()
    polisher = POLISHER.read_text()
    transport_smoke = TRANSPORT_SMOKE.read_text()
    polisher_smoke = POLISHER_SMOKE.read_text()
    errors = []
    if "strictContent(Object value)" in response:
        errors.append(f"{RESPONSE}: 仍保留重复的内容字符串方法")
    if "static String strictContent(Object value)" in transport:
        errors.append(f"{TRANSPORT}: 仍保留内容字符串转发方法")
    if "static String strictContent(Object value)" in polisher:
        errors.append(f"{POLISHER}: 仍保留内容字符串转发方法")
    if "AiProviderResponse.strictText" not in transport or "AiProviderResponse.strictText" not in polisher:
        errors.append("AI polish 响应没有直接复用共享文本策略")
    if 'getDeclaredMethod("strictContent"' in polisher_smoke or "AiPolishHttpTransport.strictContent" in transport_smoke:
        errors.append("smoke 仍通过已删除的内容字符串转发方法验证")
    if "JsonPolicy.strictStringOrEmpty" not in transport_smoke or "JsonPolicy.strictStringOrEmpty" not in polisher_smoke:
        errors.append("smoke 没有直接验证共享 JSON 文本策略")
    if errors:
        for error in errors:
            print(error)
        raise SystemExit(1)
    print("Android AI polish providers share the response text policy")


if __name__ == "__main__":
    main()
