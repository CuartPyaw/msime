#!/usr/bin/env python3
"""POSIX local ASR manifest and token readers must reject special path inputs."""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "shared/voice/LocalAsr.cpp"


def main() -> int:
    source = SOURCE.read_text(encoding="utf-8")
    regions = []
    for name, end in (
        ("read_manifest", "// ---- runtime loading ----"),
        ("read_token_set", "std::vector<std::string> utf8_characters"),
    ):
        start = source.index(f"{name}(")
        regions.append((name, source[start : source.index(end, start)]))
    missing = []
    for name, region in regions:
        for token in ("O_NOFOLLOW", "O_NONBLOCK", "fstat", "S_ISREG"):
            if token not in region:
                missing.append(f"{name}: {token}")
        if "std::ifstream" in region and "#if defined(_WIN32)" not in region:
            missing.append(f"{name}: path ifstream")
    if missing:
        print(f"{SOURCE}: reader contract missing {', '.join(missing)}", file=sys.stderr)
        return 1
    print("Local ASR manifest and token readers reject special files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
