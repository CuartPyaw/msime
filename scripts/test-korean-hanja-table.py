#!/usr/bin/env python3
"""Check the committed Korean Hanja table (crates/engine/src/korean/hanja.tsv) against the rules msime-dict-build hanja writes it by.

The table is generated from libhangul's hanja.txt, which the sources lock pins, and committed so the engine can embed it. Nothing rebuilds it in CI, so a hand edit or a generator change committed without regenerating would otherwise ship unnoticed. The invariants are checked on every run without network. When the pinned source is already in the dict-builder cache (target/dictionary-sources/hanja/hanja.txt, the --cache path the release workflow uses) and matches the lock's size and SHA-256, the table is also recomputed from it and compared byte for byte; without the cache that part prints a skip line, because fetching 6 MB is not something a contract check should do.
"""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TABLE = ROOT / "crates/engine/src/korean/hanja.tsv"
LOCK = ROOT / "resources/dictionary-sources.lock.json"
LICENSE = ROOT / "resources/licenses/libhangul-hanja-BSD-3-Clause.txt"
CACHE = ROOT / "target/dictionary-sources"
SOURCE = "hanja/hanja.txt"
COMMIT = "717409ce61524bb3d8426060a384822f21354c62"
failures = []


def check(condition: bool, message: str) -> None:
    if not condition:
        failures.append(message)


def syllable(character: str) -> bool:
    return len(character) == 1 and 0xAC00 <= ord(character) <= 0xD7A3


def kept_hanja(character: str) -> bool:
    if len(character) != 1:
        return False
    point = ord(character)
    return 0x4E00 <= point <= 0x9FFF or 0x3400 <= point <= 0x4DBF


def generate(source: str) -> str:
    """crates/dict-builder/src/hanja.rs `build` then `tsv`, restated."""
    groups: dict[str, list[tuple[str, str]]] = {}
    seen = set()
    # str::lines in Rust: only \n and \r\n end a line, unlike str.splitlines.
    for line in source.split("\n"):
        line = line.removesuffix("\r")
        if not line or line.startswith("#"):
            continue
        key, value, comment = line.split(":", 2)
        if not syllable(key) or not kept_hanja(value) or (key, value) in seen:
            continue
        seen.add((key, value))
        groups.setdefault(key, []).append((value, comment.strip()))
    return "".join(f"{key}\t{value}\t{gloss}\n" for key in sorted(groups) for value, gloss in groups[key])


def main() -> int:
    if not TABLE.is_file():
        print(f"FAIL: {TABLE.relative_to(ROOT)} is missing")
        return 1
    table = TABLE.read_text(encoding="utf-8")
    check(table.endswith("\n"), "the table does not end with a newline")
    order = []
    seen = set()
    for number, line in enumerate(table.splitlines(), 1):
        fields = line.split("\t")
        if len(fields) != 3:
            check(False, f"line {number} has {len(fields)} fields, expected syllable, hanja and gloss")
            continue
        key, value, gloss = fields
        check(syllable(key), f"line {number}: {key!r} is not one precomposed Hangul syllable")
        check(kept_hanja(value), f"line {number}: {value!r} is not one CJK Unified or Extension A ideograph")
        check(gloss == gloss.strip(), f"line {number}: the gloss has surrounding whitespace")
        check((key, value) not in seen, f"line {number}: {key} {value} is listed twice")
        seen.add((key, value))
        if not order or order[-1] != key:
            check(key not in order, f"line {number}: the readings of {key} are not contiguous")
            order.append(key)
    check(order == sorted(order), "syllables are not in code point order")
    rows = table.splitlines()
    check(rows[:2] == ["가\t可\t옳을 가", "가\t家\t집 가"], "the table no longer starts with the source's first readings of 가")
    han = [row for row in rows if row.startswith("한\t")]
    check(han[:2] == ["한\t韓\t나라 이름 한, 한나라 한", "한\t漢\t한수 한"], "한 no longer starts with 韓 and 漢 in source order")

    lock = json.loads(LOCK.read_text(encoding="utf-8"))
    pinned = [entry for entry in lock["files"] if entry["path"] == SOURCE]
    check(len(pinned) == 1, f"{SOURCE} is not pinned exactly once in the sources lock")
    check(LICENSE.is_file() and "Choe Hwanjin" in LICENSE.read_text(encoding="utf-8"), "the libhangul BSD-3-Clause text is missing from resources/licenses")
    if len(pinned) == 1:
        entry = pinned[0]
        check(f"/libhangul/libhangul/{COMMIT}/data/hanja/hanja.txt" in entry["url"], f"the lock no longer pins libhangul {COMMIT}; update this check together with the table")
        cached = CACHE / SOURCE
        if not cached.is_file():
            print(f"skipped: regeneration, {cached.relative_to(ROOT)} is not cached (msime-dict-build hanja --cache target/dictionary-sources fetches it)")
        else:
            data = cached.read_bytes()
            if len(data) != entry["size"] or hashlib.sha256(data).hexdigest() != entry["sha256"]:
                print(f"skipped: regeneration, {cached.relative_to(ROOT)} does not match the lock")
            else:
                check(generate(data.decode("utf-8")) == table, "the committed table differs from what the pinned source generates; rerun msime-dict-build hanja")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print(f"korean hanja table: {len(rows)} readings of {len(order)} syllables")
    return 0


if __name__ == "__main__":
    sys.exit(main())
