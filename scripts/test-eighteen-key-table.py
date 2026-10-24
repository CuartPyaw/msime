#!/usr/bin/env python3
"""校验 Android 18 键与共享解码的分组及跨端统计白名单，防止宿主键面与引擎漂移。"""

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
ROWS = ["q we rt y u io p", "a sd fg h jk l", "z xc v bn m"]
GROUPS = " ".join(ROWS).split()


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def check() -> None:
    engine = read("crates/engine/src/nine_key.rs")
    block = re.search(r"const EIGHTEEN_KEY_GROUPS: \[&str; 18\] = \[(.*?)\];", engine, re.S)
    assert block, "引擎缺少 18 键分组"
    groups = re.findall(r'"([a-z;]+)"', block[1])
    assert groups == GROUPS, "引擎分组与附件不符"
    assert sorted("".join(GROUPS)) == list("abcdefghijklmnopqrstuvwxyz"), "字母必须完整且不重复"
    android = read("platforms/android/java/app/msime/android/keyboard/EighteenKeyLayout.java")
    block = re.search(r"ROWS = List\.of\((.*?)\);", android, re.S)
    assert block, "Android 缺少 18 键分组"
    rows = [re.findall(r'key\("([a-z]+)"\)', line) for line in block[1].splitlines()]
    assert [row for row in rows if row] == [row.split() for row in ROWS], "Android 分组与附件不符"
    expected_ids = ["Eighteen" + group.upper() for group in GROUPS]
    for path in [
        "crates/client-services/src/typing_statistics.rs",
        "platforms/android/java/app/msime/android/statistics/KeyPressIds.java",
        "platforms/harmony/entry/src/main/ets/keyboard/KeyIdPolicy.ts",
    ]:
        assert re.findall(r'"(Eighteen[A-Z]+)"', read(path)) == expected_ids, f"{path} 的统计标识不一致"
    ios = read("platforms/ios/SharedUI/core/TypingStatistics.swift")
    block = re.search(r'let eighteenKey: \[String\] = \[(.*?)\]\.map \{ "Eighteen', ios)
    assert block, "iOS 缺少 18 键统计白名单"
    assert re.findall(r'"([A-Z]+)"', block[1]) == [group.upper() for group in GROUPS], "iOS 统计标识不一致"
    print("18 键双拼：7＋6＋5 分组、26 字母与四处统计白名单一致")


if __name__ == "__main__":
    check()
