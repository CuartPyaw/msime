#!/usr/bin/env python3
"""Harmony private text readers must validate and read through one descriptor."""
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
ACCOUNT = ROOT / "platforms/harmony/entry/src/main/ets/account/HarmonyAccountTransport.ets"
SESSION = ROOT / "platforms/harmony/entry/src/main/ets/keyboard/KeyboardSession.ets"


def main() -> int:
    account = ACCOUNT.read_text(encoding="utf-8")
    account_region = account[account.index("  load(): string | null {") : account.index("\n  save(value: string): void", account.index("  load(): string | null {"))]
    session = SESSION.read_text(encoding="utf-8")
    helper_marker = "  private static readPrivateText("
    helper_start = session.find(helper_marker)
    helper_end = session.find("\n  /** Generates bounded reply candidates", helper_start)
    helper = session[helper_start:helper_end] if helper_start >= 0 and helper_end >= 0 else ""
    required_helper = ("fs.openSync", "fs.statSync(handle.fd)", "fs.readSync", "fs.closeSync", "isFile")
    missing = [token for token in required_helper if token not in helper]
    if not helper:
        missing.append("readPrivateText helper")
    if "fs.readTextSync(this.file)" in account_region:
        missing.append("account descriptor read")
    for call in (
        "return KeyboardFeedback.parse(fs.readTextSync(path));",
        "return KeyboardFeedback.parse(fs.readTextSync(this.feedbackFile()));",
        "return EmojiCatalogModel.parseRecents(fs.readTextSync(file));",
        "return CommunityReplyLibraryPolicy.parse(fs.readTextSync(file));",
    ):
        if call in session:
            missing.append(call)
    if missing:
        print("Harmony private text readers missing " + ", ".join(missing), file=sys.stderr)
        return 1
    print("Harmony private text readers use descriptor validation")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
