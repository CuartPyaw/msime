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


def finish(buffer: list[float], rate: int = RATE, level: float = PEAK) -> list[int]:
    """Fade the last 8 ms to silence, normalize to `level` and quantize to 16 bits."""
    fade = int(0.008 * rate)
    for offset in range(fade):
        buffer[len(buffer) - fade + offset] *= 1.0 - (offset + 1) / fade
    return normalize(buffer, level)


def normalize(buffer: list[float], level: float) -> list[int]:
    """Scale so the loudest sample sits at `level` of full scale and quantize to 16 bits."""
    peak = max(abs(value) for value in buffer) or 1.0
    return [round(value / peak * level * 32767) for value in buffer]


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


# ---- Packs at 22.05 kHz ----

# The packs after `default` and `twinkle` are synthesized at half the rate. None of their voices has anything above 10 kHz worth keeping, and half the rate is half the bytes in every bundle that ships them.
LITE = 22_050
# Peak level of the background loops, below PEAK: a loop plays under typing for minutes at a time, and its sustained notes sound louder than a click that reaches the same peak.
MUSIC_PEAK = 0.5
# Partials at or above this frequency are left out rather than aliased.
LITE_CEILING = 0.45 * LITE


def lite_silence(seconds: float) -> list[float]:
    return [0.0] * int(seconds * LITE)


def lite_finish(buffer: list[float], level: float = PEAK) -> list[int]:
    """Ramp the first millisecond up from silence, so a sample never starts on a step, then fade, normalize and quantize like `finish`. A voice denser than a click, such as a pulse wave, passes a lower `level` so it sounds as loud as the others rather than peaking as high."""
    ramp = int(0.001 * LITE)
    for index in range(ramp):
        buffer[index] *= index / ramp
    return finish(buffer, LITE, level)


def lite_tone(buffer: list[float], at: float, frequency: float, partials: list[tuple[float, float, float]], attack: float = 0.002, level: float = 1.0) -> None:
    """`add_tone` at LITE: partials of (frequency ratio, amplitude, decay seconds) under a short linear attack."""
    start = int(at * LITE)
    for ratio, amplitude, decay in partials:
        if frequency * ratio >= LITE_CEILING:
            continue
        step = 2 * math.pi * frequency * ratio / LITE
        for index in range(start, len(buffer)):
            t = (index - start) / LITE
            buffer[index] += level * min(1.0, t / attack) * amplitude * math.sin(step * (index - start)) * math.exp(-t / decay)


def lite_noise(buffer: list[float], at: float, seed: int, decay: float, level: float = 1.0, highpass: float = 0.75, lowpass: float = 1.0) -> None:
    """A burst of seeded noise through a one-pole high-pass and a one-pole low-pass, decaying exponentially after a 1 ms attack."""
    generator = random.Random(seed)
    start = int(at * LITE)
    previous_input = high = low = 0.0
    for index in range(start, len(buffer)):
        t = (index - start) / LITE
        sample = generator.uniform(-1.0, 1.0)
        high = highpass * (high + sample - previous_input)
        previous_input = sample
        low += lowpass * (high - low)
        buffer[index] += level * min(1.0, t / 0.001) * low * math.exp(-t / decay)


def lite_glide(buffer: list[float], at: float, start_hz: float, end_hz: float, glide: float, decay: float, level: float = 1.0) -> None:
    """A sine whose pitch slides exponentially from `start_hz` towards `end_hz`: the ring of a bubble as it closes, or of a drop falling into water."""
    start = int(at * LITE)
    phase = 0.0
    for index in range(start, len(buffer)):
        t = (index - start) / LITE
        buffer[index] += level * min(1.0, t / 0.0015) * math.sin(phase) * math.exp(-t / decay)
        phase += 2 * math.pi * (end_hz + (start_hz - end_hz) * math.exp(-t / glide)) / LITE


def lite_pulse(buffer: list[float], at: float, notes: list[tuple[float, float, float]], duty: float = 0.25, level: float = 1.0) -> None:
    """A band-limited pulse wave, the voice of an 8-bit sound chip, playing (seconds, start Hz, end Hz) notes back to back with linear pitch slides. Each note sinks to two thirds of its level and is cut with 1 ms ramps, the way a chip retriggers its envelope."""
    highest = max(max(start_hz, end_hz) for _, start_hz, end_hz in notes)
    harmonics = [(k, 2 * math.sin(math.pi * k * duty) / (k * math.pi)) for k in range(1, int(LITE_CEILING / highest) + 1)]
    index = int(at * LITE)
    phase = 0.0
    for seconds, start_hz, end_hz in notes:
        count = int(seconds * LITE)
        ramp = int(0.001 * LITE)
        for offset in range(min(count, len(buffer) - index)):
            envelope = (1.0 - offset / count / 3) * min(1.0, offset / ramp, (count - offset) / ramp)
            value = sum(weight * math.cos(k * phase) for k, weight in harmonics)
            buffer[index + offset] += level * envelope * value
            phase += 2 * math.pi * (start_hz + (end_hz - start_hz) * offset / count) / LITE
        index += count


# ---- Typewriter ----


def typebar(buffer: list[float], at: float, thud: float, metal: float, seed: int, level: float = 1.0) -> None:
    """A type bar striking the platen: the contact noise, the ring of the steel bar and the thud of the roller behind the paper."""
    lite_noise(buffer, at, seed, 0.004, level=0.8 * level, highpass=0.8)
    lite_tone(buffer, at, 2350.0, [(1.0, 0.3 * metal, 0.012), (1.47, 0.22 * metal, 0.008), (2.31, 0.12 * metal, 0.005)], attack=0.0005, level=level)
    lite_tone(buffer, at, thud, [(1.0, 0.9, 0.02), (2.1, 0.25, 0.008)], attack=0.0005, level=level)


def typewriter_key(thud: float, metal: float, seconds: float, seed: int, level: float = 1.0) -> list[int]:
    buffer = lite_silence(seconds)
    typebar(buffer, 0.0, thud, metal, seed, level)
    return lite_finish(buffer)


# The margin bell: inharmonic partials of a small struck bell around A6.
TYPEWRITER_BELL = [(1.0, 1.0, 0.32), (2.32, 0.3, 0.12), (4.25, 0.12, 0.05)]


def typewriter_return() -> list[int]:
    # The margin bell, the ratchet of the carriage running back and the clunk of it stopping.
    buffer = lite_silence(0.6)
    lite_tone(buffer, 0.0, 1760.0, TYPEWRITER_BELL, attack=0.001, level=0.6)
    for step, onset in enumerate([0.08, 0.115, 0.145, 0.17, 0.192, 0.212, 0.23]):
        lite_noise(buffer, onset, 20 + step, 0.003, level=0.35, highpass=0.85)
    typebar(buffer, 0.255, 90.0, 0.4, 30, level=1.0)
    return lite_finish(buffer)


def typewriter_commit() -> list[int]:
    buffer = lite_silence(0.4)
    lite_tone(buffer, 0.0, 2093.0, [(ratio, amplitude, decay * 0.6) for ratio, amplitude, decay in TYPEWRITER_BELL], attack=0.001)
    return lite_finish(buffer, 0.8 * PEAK)


# ---- Bubble ----


def bubble(pops: list[tuple[float, float, float, float]], seconds: float) -> list[int]:
    """Bubbles of (onset, start Hz, end Hz, decay seconds); a pitch that rises is a bubble closing, one that falls is a drop."""
    buffer = lite_silence(seconds)
    for onset, start_hz, end_hz, decay in pops:
        lite_glide(buffer, onset, start_hz, end_hz, 0.018, decay)
    return lite_finish(buffer)


# ---- 8-bit ----


def chip(notes: list[tuple[float, float, float]], tail: float = 0.01) -> list[int]:
    buffer = lite_silence(sum(seconds for seconds, _, _ in notes) + tail)
    lite_pulse(buffer, 0.0, notes)
    return lite_finish(buffer, 0.55 * PEAK)


# ---- Wood block ----


def wood(hits: list[tuple[float, float, float]], seconds: float) -> list[int]:
    """Hits of (onset, pitch, decay seconds) on a hollow wood block: a few strongly damped inharmonic modes and a dull knock of noise."""
    buffer = lite_silence(seconds)
    for number, (onset, pitch, decay) in enumerate(hits):
        lite_tone(buffer, onset, pitch, [(1.0, 1.0, decay), (2.57, 0.35, decay / 3), (4.18, 0.12, decay / 6)], attack=0.0008)
        lite_noise(buffer, onset, 40 + number, 0.003, level=0.25, highpass=0.6, lowpass=0.5)
    return lite_finish(buffer)


def claves() -> list[int]:
    # Two sticks of hardwood: an almost pure ring around 2.5 kHz.
    buffer = lite_silence(0.3)
    lite_tone(buffer, 0.0, 2490.0, [(1.0, 1.0, 0.06), (2.71, 0.06, 0.015)], attack=0.0005)
    return lite_finish(buffer)


# ---- Melody voices ----

# A kalimba tine: a clean fundamental with the tine's own inharmonic chirp near the sixth partial, gone within a few milliseconds.
KALIMBA = [(1.0, 1.0, 0.5), (2.0, 0.06, 0.2), (5.95, 0.22, 0.035)]
# A music box comb tooth: bright, with a slightly stretched fourth partial.
MUSIC_BOX = [(1.0, 1.0, 0.6), (2.0, 0.35, 0.28), (3.0, 0.1, 0.15), (4.13, 0.12, 0.06)]
# An electric piano tine: a round fundamental, a little second harmonic and a short bell partial for the attack.
ELECTRIC_PIANO = [(1.0, 1.0, 0.8), (2.0, 0.25, 0.35), (3.0, 0.07, 0.18), (7.0, 0.05, 0.04)]


def lite_melody_tone(frequency: float, partials: list[tuple[float, float, float]], seconds: float, level: float = PEAK) -> list[int]:
    buffer = lite_silence(seconds)
    lite_tone(buffer, 0.0, frequency, partials, attack=0.003)
    return lite_finish(buffer, level)


# ---- Background loops ----

# The loops are built in a ring the length of the track: a note still ringing at the end carries on from the start, and nothing else runs across the seam, so the last sample leads into the first as any two neighbours do and the track repeats without a gap or a click.


def midi(note: int) -> float:
    return 440.0 * 2 ** ((note - 69) / 12)


def ring_add(ring: list[float], at: float, values: list[float]) -> None:
    """Mix `values` into `ring` from `at` seconds, wrapping past the end back to the start."""
    size = len(ring)
    position = int(at * LITE) % size
    done = 0
    while done < len(values):
        span = min(size - position, len(values) - done)
        ring[position : position + span] = [a + b for a, b in zip(ring[position : position + span], values[done : done + span])]
        done += span
        position = 0


def voice(seconds: float, frequency: float, partials: list[tuple[float, float, float | None]], attack: float, release: float, level: float) -> list[float]:
    """One note of (frequency ratio, amplitude, decay seconds or None to sustain) partials, eased in over `attack` and out over `release` along a sine-squared curve, so it starts and ends at silence."""
    count = int(seconds * LITE)
    values = [0.0] * count
    for ratio, amplitude, decay in partials:
        if frequency * ratio >= LITE_CEILING:
            continue
        step = 2 * math.pi * frequency * ratio / LITE
        if decay is None:
            values = [value + amplitude * math.sin(step * index) for index, value in enumerate(values)]
        else:
            fall = -1.0 / (decay * LITE)
            values = [value + amplitude * math.sin(step * index) * math.exp(fall * index) for index, value in enumerate(values)]
    rise = max(1, int(attack * LITE))
    for index in range(min(rise, count)):
        values[index] *= math.sin(0.5 * math.pi * index / rise) ** 2
    fade = max(1, int(release * LITE))
    for index in range(min(fade, count)):
        values[count - 1 - index] *= math.sin(0.5 * math.pi * index / fade) ** 2
    return [level * value for value in values]


def drum_fade(values: list[float]) -> list[float]:
    """Bring the last 10 ms of a drum hit down to silence."""
    fade = int(0.01 * LITE)
    count = len(values)
    for index in range(fade):
        values[count - 1 - index] *= index / fade
    return values


def drum_kick() -> list[float]:
    values = []
    phase = 0.0
    for index in range(int(0.42 * LITE)):
        t = index / LITE
        values.append(min(1.0, t / 0.001) * math.sin(phase) * math.exp(-t / 0.15))
        phase += 2 * math.pi * (46.0 + 74.0 * math.exp(-t / 0.03)) / LITE
    return drum_fade(values)


def drum_noise(seed: int, seconds: float, decay: float, highpass: float, lowpass: float) -> list[float]:
    buffer = [0.0] * int(seconds * LITE)
    lite_noise(buffer, 0.0, seed, decay, highpass=highpass, lowpass=lowpass)
    return drum_fade(buffer)


def drum_snare(seed: int) -> list[float]:
    # A brushed, muted snare: band-limited noise over a short drum-head tone.
    values = drum_noise(seed, 0.28, 0.07, 0.7, 0.45)
    body = voice(0.28, 185.0, [(1.0, 0.6, 0.035), (1.6, 0.2, 0.02)], 0.001, 0.01, 1.0)
    return [a + b for a, b in zip(values, body)]


def lofi_loop() -> list[int]:
    # 80 beats a minute, four to the bar, twelve bars: ii-V-I-vi in C, three times round, the last two times with a tune on top. Eighth notes swing, the second of each pair landing late.
    beat = 0.75
    ring = [0.0] * int(12 * 4 * beat * LITE)
    kick = drum_kick()
    snares = [drum_snare(60 + number) for number in range(2)]
    hats = [drum_noise(70 + number, 0.09, 0.018, 0.45, 1.0) for number in range(3)]

    def at(bar: int, position: float) -> float:
        whole = math.floor(position)
        swung = whole + (0.58 if position - whole >= 0.5 else position - whole)
        return (bar * 4 + swung) * beat

    # (bass root, voicing) per bar of the cycle: Dm9, G13, Cmaj9, Am9.
    chords = [
        (38, [53, 57, 60, 64]),
        (43, [53, 59, 64, 69]),
        (36, [52, 55, 59, 62]),
        (45, [55, 59, 60, 64]),
    ]
    electric_piano = [(1.0, 1.0, 1.6), (2.0, 0.2, 0.5), (3.0, 0.06, 0.25), (7.0, 0.03, 0.05)]
    bass = [(1.0, 1.0, 1.4), (2.0, 0.3, 0.4)]
    for bar in range(12):
        root, voicing = chords[bar % 4]
        # The chord, rolled a little from the bottom, and a soft stab on the and of three.
        for number, note in enumerate(voicing):
            ring_add(ring, at(bar, 0) + 0.012 * number, voice(2.9, midi(note), electric_piano, 0.006, 0.25, 0.22))
            ring_add(ring, at(bar, 2.5) + 0.008 * number, voice(0.4, midi(note), electric_piano, 0.006, 0.12, 0.09))
        ring_add(ring, at(bar, 0), voice(1.35, midi(root), bass, 0.01, 0.08, 0.55))
        ring_add(ring, at(bar, 1.5), voice(0.5, midi(root), bass, 0.01, 0.06, 0.4))
        ring_add(ring, at(bar, 2.5), voice(0.9, midi(root + 7), bass, 0.01, 0.08, 0.45))
        kicks = [0, 2.5] if bar % 4 == 3 else [0, 1.5]
        for position in kicks:
            ring_add(ring, at(bar, position), [0.85 * value for value in kick])
        for number, position in enumerate([1, 3]):
            ring_add(ring, at(bar, position), [0.3 * value for value in snares[(bar + number) % 2]])
        for step in range(8):
            loudness = 0.16 if step % 2 == 0 else 0.1
            ring_add(ring, at(bar, step / 2), [loudness * value for value in hats[(bar * 8 + step) % 3]])
    # (bar, beat, note, beats long). The last note rings over the seam into the first bar.
    tune = [
        (1, 2, 71, 1.0), (3, 2, 76, 2.0),
        (4, 0.5, 69, 0.5), (4, 1, 72, 0.5), (4, 1.5, 76, 1.0), (4, 3, 74, 1.0),
        (5, 0.5, 71, 0.5), (5, 1, 74, 1.5), (5, 3, 69, 1.0),
        (6, 0, 76, 1.5), (6, 2, 74, 0.5), (6, 2.5, 71, 1.4),
        (7, 1, 72, 1.0), (7, 2, 69, 2.0),
        (8, 0.5, 77, 0.5), (8, 1, 76, 0.5), (8, 1.5, 72, 0.5), (8, 2, 74, 1.5),
        (9, 0, 71, 1.0), (9, 1.5, 74, 0.5), (9, 2, 77, 1.0), (9, 3, 76, 1.0),
        (10, 0, 79, 2.0), (10, 2.5, 76, 1.4),
        (11, 0.5, 74, 0.5), (11, 1, 72, 1.0), (11, 2, 71, 0.5), (11, 2.5, 69, 1.6),
    ]
    lead = [(1.0, 1.0, 0.9), (2.0, 0.12, 0.4), (3.0, 0.05, 0.25)]
    for bar, position, note, length in tune:
        ring_add(ring, at(bar, position), voice(length * beat + 0.3, midi(note), lead, 0.012, 0.15, 0.3))
    return normalize(ring, MUSIC_PEAK)


def ambient_loop() -> list[int]:
    # Three slow pads, Cmaj9, Am9 and Fmaj9, twelve seconds each and crossfading into one another, under a sparse pentatonic glass chime with two echoes.
    span = 12.0
    ring = [0.0] * int(3 * span * LITE)
    chords = [[48, 55, 59, 62, 64], [45, 52, 55, 59, 60], [41, 48, 52, 55, 57]]
    # Two sines a few cents apart beat slowly against each other, which is what keeps a held pad alive.
    pad = [(1.0, 1.0, None), (1.0016, 0.8, None), (2.0, 0.12, None)]
    for number, chord in enumerate(chords):
        for position, note in enumerate(chord):
            level = 0.2 if position == 0 else 0.13
            ring_add(ring, number * span - 3.0, voice(span + 6.0, midi(note), pad, 5.0, 6.0, level))
    generator = random.Random(80)
    pentatonic = [72, 74, 76, 79, 81, 84, 86, 88]
    glass = [(1.0, 1.0, 1.4), (2.76, 0.12, 0.45), (5.4, 0.04, 0.18)]
    moment = 0.7
    previous = None
    while moment < 3 * span - 1.0:
        note = generator.choice([candidate for candidate in pentatonic if candidate != previous])
        previous = note
        loudness = generator.uniform(0.1, 0.18)
        chime = voice(3.5, midi(note), glass, 0.003, 0.4, 1.0)
        for delay, echo in [(0.0, 1.0), (0.48, 0.35), (0.96, 0.15)]:
            ring_add(ring, moment + delay, [loudness * echo * value for value in chime])
        moment += generator.uniform(1.6, 3.4)
    return normalize(ring, MUSIC_PEAK)


# Pack id to (sample rate, file name to renderer).
PACKS: dict[str, tuple[int, dict[str, object]]] = {
    "default": (
        RATE,
        {
            "key.wav": lambda: key(180.0, 0.016, 0.005, 0.055, 1),
            "space.wav": lambda: key(110.0, 0.03, 0.008, 0.09, 2),
            "enter.wav": enter,
            "backspace.wav": lambda: key(250.0, 0.012, 0.004, 0.05, 5),
            "commit.wav": commit,
            "achievement.wav": achievement,
        },
    ),
    "twinkle": (
        RATE,
        {
            "tone.wav": melody_tone,
        },
    ),
    "msime-typewriter": (
        LITE,
        {
            "key.wav": lambda: typewriter_key(150.0, 1.0, 0.09, 11),
            "space.wav": lambda: typewriter_key(95.0, 0.35, 0.12, 12),
            "enter.wav": typewriter_return,
            "backspace.wav": lambda: typewriter_key(210.0, 0.6, 0.07, 13, level=0.8),
            "commit.wav": typewriter_commit,
        },
    ),
    "msime-bubble": (
        LITE,
        {
            "key.wav": lambda: bubble([(0.0, 520.0, 860.0, 0.03)], 0.12),
            "space.wav": lambda: bubble([(0.0, 300.0, 480.0, 0.045)], 0.16),
            "enter.wav": lambda: bubble([(0.0, 380.0, 620.0, 0.035), (0.06, 620.0, 980.0, 0.03)], 0.2),
            "backspace.wav": lambda: bubble([(0.0, 900.0, 480.0, 0.03)], 0.12),
            "commit.wav": lambda: bubble([(0.0, 600.0, 900.0, 0.03), (0.05, 800.0, 1200.0, 0.03), (0.1, 1050.0, 1580.0, 0.04)], 0.3),
        },
    ),
    "msime-8bit": (
        LITE,
        {
            "key.wav": lambda: chip([(0.045, 880.0, 880.0)]),
            "space.wav": lambda: chip([(0.07, 440.0, 440.0)]),
            "enter.wav": lambda: chip([(0.035, 1046.5, 1046.5), (0.035, 1318.51, 1318.51), (0.07, 1567.98, 1567.98)]),
            "backspace.wav": lambda: chip([(0.08, 880.0, 330.0)]),
            # The coin: B5, then E6 held.
            "commit.wav": lambda: chip([(0.06, 987.77, 987.77), (0.25, 1318.51, 1318.51)]),
        },
    ),
    "msime-woodblock": (
        LITE,
        {
            "key.wav": lambda: wood([(0.0, 720.0, 0.035)], 0.12),
            "space.wav": lambda: wood([(0.0, 470.0, 0.05)], 0.16),
            "enter.wav": lambda: wood([(0.0, 560.0, 0.045), (0.07, 840.0, 0.04)], 0.22),
            "backspace.wav": lambda: wood([(0.0, 1050.0, 0.025)], 0.09),
            "commit.wav": claves,
        },
    ),
    "msime-pentatonic": (
        LITE,
        {
            # C5.
            "tone.wav": lambda: lite_melody_tone(523.25, KALIMBA, 1.1),
        },
    ),
    "msime-canon": (
        LITE,
        {
            # A5.
            "tone.wav": lambda: lite_melody_tone(880.0, MUSIC_BOX, 1.2),
        },
    ),
    "msime-ode-to-joy": (
        LITE,
        {
            # C5.
            "tone.wav": lambda: lite_melody_tone(523.25, ELECTRIC_PIANO, 1.2, 0.82 * PEAK),
        },
    ),
    "msime-music-lofi": (
        LITE,
        {
            "lofi.wav": lofi_loop,
        },
    ),
    "msime-music-ambient": (
        LITE,
        {
            "ambient.wav": ambient_loop,
        },
    ),
}


def write(path: Path, samples: list[int], rate: int) -> None:
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(rate)
        output.writeframes(struct.pack(f"<{len(samples)}h", *samples))


def generate(output: Path) -> list[Path]:
    written = []
    for pack, (rate, files) in PACKS.items():
        directory = output / pack
        directory.mkdir(parents=True, exist_ok=True)
        for name, render in files.items():
            path = directory / name
            write(path, render(), rate)
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
