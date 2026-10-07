#!/usr/bin/env python3
"""iOS/macOS bounded readers must reject special files without blocking."""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]


def region(path: Path, start: str, end: str) -> str:
    text = path.read_text(encoding="utf-8")
    begin = text.index(start)
    return text[begin : text.index(end, begin)]


CHECKS = (
    (
        ROOT / "platforms/ios/App/Services/BoundedFileReader.swift",
        "static func read(from url: URL",
        "  }\n}",
        ("O_NONBLOCK", "fstat", "S_IFREG"),
    ),
    (
        ROOT / "platforms/ios/SharedUI/voice/VoiceTextHandoffStore.swift",
        "static func readBounded(_ file: URL)",
        "  }\n\n  func read(now:",
        ("O_NONBLOCK", "fstat", "S_IFREG"),
    ),
    (
        ROOT / "platforms/ios/SharedUI/dictionary/DictionarySnapshotQueue.swift",
        "private func read(_ root: URL)",
        "  }\n  func read() throws",
        ("O_NONBLOCK", "fstat", "S_IFREG"),
    ),
    (
        ROOT / "platforms/ios/SharedUI/dictionary/DictionarySnapshotQueue.swift",
        "private func openSnapshotSource(_ file: URL)",
        "  }\n\n  // A killed worker",
        ("O_NONBLOCK",),
    ),
    (
        ROOT / "platforms/macos/src/core/BoundedFileReader.mm",
        "NSData *MSIMEReadFileUpTo",
        "    return data;\n}",
        ("O_NONBLOCK",),
    ),
)


def main() -> int:
    failed = False
    for path, start, end, required in CHECKS:
        body = region(path, start, end)
        missing = [token for token in required if token not in body]
        if missing:
            print(f"{path}: missing {', '.join(missing)}", file=sys.stderr)
            failed = True
    if not failed:
        print("iOS and macOS bounded readers reject special files without blocking")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
