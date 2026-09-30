#!/usr/bin/env python3
"""The generated punctuation header matches the JSON it is generated from.

`shared/contracts/punctuation/policy.json` is the authority for the Chinese punctuation map; `policy.h`, which the Windows hosts compile against, is generated from it. A hand edit of the header, or a JSON edit without regenerating, would give the C++ hosts and the Rust engine (whose own test compares its table with both files) different punctuation, and nothing but this would notice before a user typed it.
"""

from __future__ import annotations

import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
GENERATOR = ROOT / "shared/contracts/punctuation/generate.py"

result = subprocess.run([sys.executable, str(GENERATOR), "--check"], cwd=ROOT, capture_output=True, text=True)
if result.returncode != 0:
    sys.stderr.write(result.stdout + result.stderr)
    sys.stderr.write("punctuation contract: policy.h does not match policy.json; run python3 shared/contracts/punctuation/generate.py\n")
    sys.exit(1)
print("punctuation contract: policy.h matches policy.json")
