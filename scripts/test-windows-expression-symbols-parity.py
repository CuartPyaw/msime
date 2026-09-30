#!/usr/bin/env python3
"""Keep the Windows TIP's copy of the V mode's symbols equal to the Engine's.

The Engine decides which keys the V (expression) mode spells, and publishes them in View.spelling_symbols; the Windows Server reads them from there. The TSF DLL cannot: it decides whether a key is composition input or a candidate selection before the Server has answered, so `platforms/windows/tsf/Global/LocalModeKeyPolicy.h` keeps its own copy. A symbol added on one side alone splits the two: the TIP would put an operator into its keystroke buffer that the Engine never saw, or send a digit as a selection the Engine expected as input, and the composition the user sees stops matching the one that commits.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ENGINE = ROOT / "crates/engine/src/local/expression.rs"
TIP = ROOT / "platforms/windows/tsf/Global/LocalModeKeyPolicy.h"


def main() -> int:
    engine = re.search(r'pub const SPELLING_SYMBOLS: &str = "([^"]*)";', ENGINE.read_text(encoding="utf-8"))
    if not engine:
        print(f"{ENGINE.relative_to(ROOT)}: SPELLING_SYMBOLS not found", file=sys.stderr)
        return 1
    tip = re.search(r'inline constexpr wchar_t ExpressionSpellingSymbols\[\] = L"([^"]*)";', TIP.read_text(encoding="utf-8"))
    if not tip:
        print(f"{TIP.relative_to(ROOT)}: ExpressionSpellingSymbols not found", file=sys.stderr)
        return 1
    if engine.group(1) != tip.group(1):
        print(
            f"expression symbols differ: Engine {engine.group(1)!r}, Windows TIP {tip.group(1)!r}",
            file=sys.stderr,
        )
        return 1
    print(f"Windows TIP spells the Engine's V symbols: {engine.group(1)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
