#!/usr/bin/env python3
"""Android 社区和词库页面复用共享的条数展示策略。"""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
NUMBER = ROOT / "platforms/android/java/app/msime/android/NumberPolicy.java"
COMMUNITY = ROOT / "platforms/android/java/app/msime/android/community/CommunityRequest.java"
COLLECTIONS = ROOT / "platforms/android/java/app/msime/android/dictionary/DictionaryCollectionsStore.java"


def main() -> int:
    number = NUMBER.read_text(encoding="utf-8")
    errors = []
    if "public static String groupedCount(long count)" not in number:
        errors.append(f"{NUMBER}: 缺少共享条数展示方法")
    for source in (COMMUNITY, COLLECTIONS):
        text = source.read_text(encoding="utf-8")
        if text.count("NumberPolicy.groupedCount(") != 1:
            errors.append(f"{source}: 没有调用共享条数展示方法")
        if "NumberPolicy.grouped(BoundsPolicy.nonNegative(count)) + \" 条\"" in text:
            errors.append(f"{source}: 仍保留重复的条数拼接逻辑")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("Android count labels share the grouped count policy")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
