#!/usr/bin/env python3
"""Offline check of build_pronunciations.py: phonetic normalisation, word selection and the database.

The rows are real ECDICT cells, including the irregular ones the normaliser exists for — Cyrillic
schwa, ASCII stress and length marks, several variants in one cell, Chinese notes. Everything runs on
a temporary CSV; nothing is downloaded, so --quick can run it offline.
"""
import csv
import sqlite3
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import build_pronunciations as generator  # noqa: E402

failures = []


def check(condition: bool, message: str) -> None:
    if not condition:
        failures.append(message)


# (ECDICT cell, expected IPA or None)
PHONETICS = [
    ("lʌv", "lʌv"),
    ("hә'lәu", "həˈləu"),  # Cyrillic schwa, ASCII primary stress
    ("kәm'pju:tә", "kəmˈpjuːtə"),  # ASCII length mark
    (",skrɑ:vən'hɑ:ɡə", "ˌskrɑːvənˈhɑːɡə"),  # leading comma is secondary stress
    ("ðem; ðəm", "ðem"),  # several variants: the first
    ("ɪg'zem(p)t; eg-", "ɪɡˈzem(p)t"),  # optional sound kept, ASCII g made IPA
    ("ˈteliˌprɔmptə(璻)", None),  # stray CJK
    ("kɑ:'lɔtə; 意大利语kɑ:r'lɔ:ttɑ:", "kɑːˈlɔtə"),  # the note is in the dropped variant
    ("[ˈɔ:ltmən", "ˈɔːltmən"),  # a stray bracket at the edge
    ("ˈæmfθətəz", None),  # private-use characters
    (".eibi:si:'dєәriәn", ".eibiːsiːˈdɛəriən"),  # Cyrillic ie; the dot is an IPA syllable break
    ("", None),
    ("ˈbæb(ə", None),  # unbalanced
]

WORDS = [
    ("love", True),
    ("well-known", True),
    ("don't", True),
    ("-ing", False),
    ("a cappella", False),
    ("café", False),
    ("3D", False),
]


def main() -> int:
    for raw, expected in PHONETICS:
        found = generator.clean_phonetic(raw)
        check(found == expected, f"clean_phonetic({raw!r}) = {found!r}, expected {expected!r}")
    for word, expected in WORDS:
        check(generator.usable_word(word) is expected, f"usable_word({word!r}) should be {expected}")

    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "ecdict.csv"
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(["word", "phonetic", "translation"])
            writer.writerow(["May", "mei", "n. 五月"])
            writer.writerow(["may", "meɪ", "aux. 可以"])
            writer.writerow(["June", "dʒu:n", "n. 六月"])
            writer.writerow(["love", "lʌv", "n. 爱"])
            writer.writerow(["nophonetic", "", "n. 无"])
            writer.writerow(["-ing", "iŋ", "suf."])
        rows = generator.collect(path)
        check(rows.get("may") == "meɪ", f"the lowercase headword wins a shared key: {rows.get('may')!r}")
        check(rows.get("june") == "dʒuːn", f"a capitalised headword is keyed lowercase: {rows.get('june')!r}")
        check("nophonetic" not in rows and "-ing" not in rows, "rows without a usable word or IPA are dropped")

        database = Path(directory) / "en-phonetic.db"
        generator.write_database(database, rows, {"kind": "en_phonetic"})
        first = database.read_bytes()
        generator.write_database(database, rows, {"kind": "en_phonetic"})
        check(first == database.read_bytes(), "the same rows give the same bytes")
        connection = sqlite3.connect(database)
        try:
            check(connection.execute("PRAGMA user_version").fetchone() == (1,), "user_version is 1")
            meta = dict(connection.execute("SELECT key, value FROM meta"))
            check(meta.get("kind") == "en_phonetic", f"meta.kind: {meta.get('kind')!r}")
            check(meta.get("key_count") == str(len(rows)), f"meta.key_count: {meta.get('key_count')!r}")
            check(
                connection.execute("SELECT phonetic FROM en_phonetics WHERE word = 'love'").fetchone() == ("lʌv",),
                "love is looked up by its lowercase key",
            )
        finally:
            connection.close()

    for failure in failures:
        print(f"FAIL {failure}")
    print(f"pronunciations: {'ok' if not failures else f'{len(failures)} failure(s)'}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
