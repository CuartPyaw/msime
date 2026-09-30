#!/usr/bin/env python3
"""The built-in sound packs are exactly what scripts/generate_sound_packs.py synthesizes.

Their CC0-1.0 dedication rests on every sample being computed by that script rather than recorded or downloaded. This regenerates the samples into a temporary directory and compares them with the committed ones, so a sample edited by hand, or one dropped in from elsewhere, fails here instead of shipping under a licence nobody can vouch for. Samples are compared to within one step of 16-bit quantization, since the sines and exponentials come from the platform's libm.
"""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import tomllib
import wave
from array import array
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKS = ROOT / "resources/sound-packs"


def load_generator():
    spec = importlib.util.spec_from_file_location("generate_sound_packs", ROOT / "scripts/generate_sound_packs.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def samples(path: Path) -> tuple[tuple[int, int, int], array]:
    with wave.open(str(path), "rb") as source:
        shape = (source.getnchannels(), source.getsampwidth(), source.getframerate())
        frames = array("h", source.readframes(source.getnframes()))
    return shape, frames


def main() -> int:
    generator = load_generator()
    failures = []
    with tempfile.TemporaryDirectory() as scratch:
        expected = {path.relative_to(scratch).as_posix(): path for path in generator.generate(Path(scratch))}
        committed = {path.relative_to(PACKS).as_posix(): path for path in PACKS.rglob("*.wav")}
        for name in sorted(set(committed) - set(expected)):
            failures.append(f"{name} is not produced by generate_sound_packs.py")
        for name in sorted(set(expected) - set(committed)):
            failures.append(f"{name} is missing; run scripts/generate_sound_packs.py")
        for name in sorted(set(expected) & set(committed)):
            want_shape, want = samples(expected[name])
            have_shape, have = samples(committed[name])
            if want_shape != have_shape or len(want) != len(have):
                failures.append(f"{name} differs in format or length from the generator's output")
            elif any(abs(a - b) > 1 for a, b in zip(want, have)):
                failures.append(f"{name} differs from the generator's output")
    for directory in sorted(path for path in PACKS.iterdir() if path.is_dir()):
        manifest = tomllib.loads((directory / "plugin.toml").read_text(encoding="utf-8"))
        if manifest.get("license") != "CC0-1.0" or manifest.get("id") != directory.name:
            failures.append(f"{directory.name}/plugin.toml must name its folder and CC0-1.0")
    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    if failures:
        return 1
    print(f"sound packs: {len(committed)} samples match the generator")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
