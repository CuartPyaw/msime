#!/usr/bin/env python3
"""Synthesize the built-in sound packs' samples into resources/sound-packs.

Every sample is computed here from sines, decaying envelopes and seeded noise; nothing is recorded or downloaded, so the output is this project's own work and is dedicated to the public domain under CC0-1.0, as each pack's plugin.toml says. The manifests are committed beside the samples and are not written by this script.

Run it after changing a voice below, then commit the regenerated files:

    python3 scripts/generate_sound_packs.py

`scripts/test-sound-packs.py` regenerates the samples into a temporary directory and compares them with the committed ones, so a sample that no longer comes from this script fails the checks.
"""

from __future__ import annotations

import argparse
import math
import random
import struct
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "resources/sound-packs"
RATE = 44_100
# Peak level of every sample, about -3 dBFS, so the host's volume setting starts from the same loudness for each.
PEAK = 0.7


def silence(seconds: float) -> list[float]:
    return [0.0] * int(seconds * RATE)


def add_click(buffer: list[float], at: float, pitch: float, body: float, noise: float, seed: int, level: float = 1.0) -> None:
    """A key press: a burst of high-passed noise for the contact and a short decaying sine for the body."""
    generator = random.Random(seed)
    start = int(at * RATE)
    previous_input = previous_output = 0.0
    for index in range(start, len(buffer)):
        t = (index - start) / RATE
        sample = generator.uniform(-1.0, 1.0)
        # One-pole high-pass around 2 kHz: keeps the click crisp instead of hissy.
        filtered = 0.75 * (previous_output + sample - previous_input)
        previous_input, previous_output = sample, filtered
        contact = filtered * math.exp(-t / noise)
        thock = math.sin(2 * math.pi * pitch * t) * math.exp(-t / body)
        buffer[index] += level * (0.45 * contact + 0.8 * thock)


def add_tone(buffer: list[float], at: float, frequency: float, partials: list[tuple[float, float, float]], attack: float = 0.004) -> None:
    """A plucked or struck note: partials of (frequency ratio, amplitude, decay seconds) under a short linear attack."""
    start = int(at * RATE)
    for index in range(start, len(buffer)):
        t = (index - start) / RATE
        envelope = min(1.0, t / attack)
        value = 0.0
        for ratio, amplitude, decay in partials:
            value += amplitude * math.sin(2 * math.pi * frequency * ratio * t) * math.exp(-t / decay)
        buffer[index] += envelope * value


def finish(buffer: list[float]) -> list[int]:
    """Fade the last 8 ms to silence, normalize to PEAK and quantize to 16 bits."""
    fade = int(0.008 * RATE)
    for offset in range(fade):
        buffer[len(buffer) - fade + offset] *= 1.0 - (offset + 1) / fade
    peak = max(abs(value) for value in buffer) or 1.0
    return [round(value / peak * PEAK * 32767) for value in buffer]


def key(pitch: float, body: float, noise: float, seconds: float, seed: int) -> list[int]:
    buffer = silence(seconds)
    add_click(buffer, 0.0, pitch, body, noise, seed)
    return finish(buffer)


def enter() -> list[int]:
    # A heavier key that lands twice: the stem, then the stabilizer bar.
    buffer = silence(0.11)
    add_click(buffer, 0.0, 140.0, 0.022, 0.006, 3)
    add_click(buffer, 0.018, 180.0, 0.018, 0.004, 4, level=0.55)
    return finish(buffer)


# A plucked string: harmonics that fade faster the higher they are.
PLUCKED = [(1.0, 1.0, 0.42), (2.0, 0.45, 0.2), (3.0, 0.22, 0.13), (4.0, 0.1, 0.09), (5.0, 0.05, 0.07)]


def commit() -> list[int]:
    # A small bell: inharmonic partials of E6.
    buffer = silence(0.32)
    add_tone(buffer, 0.0, 1318.51, [(1.0, 1.0, 0.11), (2.76, 0.35, 0.05), (5.4, 0.15, 0.025)], attack=0.002)
    return finish(buffer)


def achievement() -> list[int]:
    # C major arpeggio, C5 E5 G5 C6, the last note held.
    buffer = silence(1.25)
    for onset, frequency in [(0.0, 523.25), (0.11, 659.26), (0.22, 783.99), (0.33, 1046.5)]:
        partials = [(ratio, amplitude, decay * (1.6 if frequency > 1000 else 0.8)) for ratio, amplitude, decay in PLUCKED]
        add_tone(buffer, onset, frequency, partials)
    return finish(buffer)


def melody_tone() -> list[int]:
    # C5, the pitch the melody's semitone offsets are counted from.
    buffer = silence(0.8)
    add_tone(buffer, 0.0, 523.25, PLUCKED)
    return finish(buffer)


PACKS: dict[str, dict[str, object]] = {
    "default": {
        "key.wav": lambda: key(180.0, 0.016, 0.005, 0.055, 1),
        "space.wav": lambda: key(110.0, 0.03, 0.008, 0.09, 2),
        "enter.wav": enter,
        "backspace.wav": lambda: key(250.0, 0.012, 0.004, 0.05, 5),
        "commit.wav": commit,
        "achievement.wav": achievement,
    },
    "twinkle": {
        "tone.wav": melody_tone,
    },
}


def write(path: Path, samples: list[int]) -> None:
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(RATE)
        output.writeframes(struct.pack(f"<{len(samples)}h", *samples))


def generate(output: Path) -> list[Path]:
    written = []
    for pack, files in PACKS.items():
        directory = output / pack
        directory.mkdir(parents=True, exist_ok=True)
        for name, render in files.items():
            path = directory / name
            write(path, render())
            written.append(path)
    return written


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, default=OUTPUT)
    arguments = parser.parse_args()
    for path in generate(arguments.out):
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
