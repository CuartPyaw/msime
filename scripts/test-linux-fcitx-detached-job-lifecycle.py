#!/usr/bin/env python3
"""Fcitx detached workers must finish before the addon shared object unloads."""

import re
from pathlib import Path

source = Path(__file__).resolve().parents[1] / "platforms/linux/fcitx5/FcitxEngine.cpp"
text = source.read_text()

if "class DetachedJobTracker" not in text:
    raise SystemExit("missing detached-job tracker")
if "std::condition_variable idle_" not in text:
    raise SystemExit("detached-job tracker has no idle wait")
if not re.search(r"template <class F> std::shared_future<Json> detachedJob\(F work\)\s*\{\s*return fcitx_detached_jobs\.start", text):
    raise SystemExit("detachedJob does not register workers with the tracker")

destructor = re.search(r"~FcitxEngine\(\) override \{(?P<body>.*?)\n  \}", text, re.S)
if not destructor:
    raise SystemExit("missing FcitxEngine destructor")
if "fcitx_detached_jobs.wait_idle();" not in destructor.group("body"):
    raise SystemExit("FcitxEngine destructor does not wait for detached workers")

print("linux Fcitx detached jobs: all workers are tracked until addon unload")
