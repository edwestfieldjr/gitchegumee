#!/usr/bin/env python3
"""Generate the game's original chiptune music and sound effects.

Usage: tools/gen_audio.py OUTPUT_DIR
Writes WAV files to OUTPUT_DIR. Encode them to assets/audio/*.ogg with ffmpeg:
    ffmpeg -i in.wav -c:a libvorbis -q:a 3 out.ogg

Uses only the standard library. Output is the same on every run.
"""

import math
import random
import struct
import sys
import wave
from pathlib import Path

RATE = 22050
BEAT = 0.5  # seconds per quarter note, 120 bpm


def midi_hz(note):
    return 440.0 * 2 ** ((note - 69) / 12)


def square(phase, duty=0.5):
    return 1.0 if phase % 1.0 < duty else -1.0


def triangle(phase):
    p = phase % 1.0
    return 4 * p - 1 if p < 0.5 else 3 - 4 * p


def add_note(buf, start, length, note, wave_fn, level, attack=0.005, release=0.06):
    hz = midi_hz(note)
    first = int(start * RATE)
    count = int(length * RATE)
    for i in range(count):
        t = i / RATE
        env = min(1.0, t / attack) * min(1.0, (length - t) / release)
        index = (first + i) % len(buf)
        buf[index] += wave_fn(t * hz) * level * max(env, 0.0)


def add_noise(buf, start, length, level, rng, hold=4):
    first = int(start * RATE)
    count = int(length * RATE)
    value = 0.0
    for i in range(count):
        if i % hold == 0:
            value = rng.uniform(-1, 1)
        env = 1.0 - i / count
        buf[(first + i) % len(buf)] += value * level * env


def save(path, buf):
    peak = max(1.0, max(abs(s) for s in buf))
    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(
            b"".join(struct.pack("<h", int(s / peak * 30000)) for s in buf)
        )


def empty(seconds):
    return [0.0] * int(seconds * RATE)


# D minor, slow and plain. One bar is four beats; the loop is eight bars.
BASS = [38, 38, 34, 34, 41, 41, 33, 33]
CHORDS = [
    (50, 53, 57),
    (50, 53, 57),
    (46, 50, 53),
    (46, 50, 53),
    (48, 53, 57),
    (48, 53, 57),
    (45, 49, 52),
    (45, 49, 52),
]
TUNE = [
    (0, 2, 62), (2, 1, 65), (3, 1, 64),
    (4, 3, 62), (7, 1, 60),
    (8, 2, 58), (10, 2, 62),
    (12, 4, 65),
    (16, 2, 64), (18, 1, 65), (19, 1, 67),
    (20, 3, 69), (23, 1, 65),
    (24, 2, 64), (26, 2, 61),
    (28, 4, 62),
]
BARS = len(BASS)
LOOP = BARS * 4 * BEAT


def calm_layer():
    buf = empty(LOOP)
    for bar, root in enumerate(BASS):
        start = bar * 4 * BEAT
        add_note(buf, start, 2 * BEAT, root + 12, triangle, 0.5, release=0.2)
        add_note(buf, start + 2 * BEAT, 2 * BEAT, root + 12, triangle, 0.4, release=0.2)
    for beat, length, note in TUNE:
        add_note(
            buf,
            beat * BEAT,
            length * BEAT * 0.95,
            note + 12,
            lambda p: square(p, 0.25),
            0.16,
            release=0.15,
        )
    return buf


def storm_layer():
    rng = random.Random(1975)
    buf = empty(LOOP)
    for bar, (root, chord) in enumerate(zip(BASS, CHORDS)):
        start = bar * 4 * BEAT
        for step in range(8):
            at = start + step * BEAT / 2
            add_note(buf, at, BEAT / 2 * 0.9, root, square, 0.2)
            add_note(
                buf,
                at,
                BEAT / 2 * 0.8,
                chord[step % 3] + 12,
                lambda p: square(p, 0.125),
                0.07,
            )
        for step in range(4):
            add_noise(buf, start + step * BEAT, BEAT * 0.4, 0.3 if step % 2 else 0.18, rng)
    return buf


def wind_layer():
    """Filtered noise, looped, for the top of the storm."""
    rng = random.Random(29)
    buf = empty(LOOP)
    low = 0.0
    for i in range(len(buf)):
        swell = 0.55 + 0.45 * math.sin(2 * math.pi * 3 * i / len(buf))
        cut = 0.02 + 0.04 * swell
        low += (rng.uniform(-1, 1) - low) * cut
        buf[i] = low * swell * 5.0
    return buf


def blip(notes, step, duty=0.5, level=0.5):
    buf = empty(step * len(notes) + 0.05)
    for i, note in enumerate(notes):
        add_note(buf, i * step, step, note, lambda p: square(p, duty), level, release=0.02)
    return buf


def noise_hit(seconds, level, hold, seed):
    rng = random.Random(seed)
    buf = empty(seconds)
    add_noise(buf, 0, seconds, level, rng, hold)
    return buf


def bell():
    """A struck bell: inharmonic partials with a long decay."""
    seconds = 3.0
    buf = empty(seconds)
    partials = [(1.0, 1.0), (2.0, 0.55), (2.4, 0.4), (3.0, 0.3), (4.2, 0.18), (5.4, 0.1)]
    base = 196.0
    for i in range(len(buf)):
        t = i / RATE
        sample = 0.0
        for ratio, level in partials:
            sample += math.sin(2 * math.pi * base * ratio * t) * level * math.exp(
                -t * (0.9 + ratio * 0.5)
            )
        buf[i] = sample
    return buf


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    save(out / "music_calm.wav", calm_layer())
    save(out / "music_storm.wav", storm_layer())
    save(out / "music_wind.wav", wind_layer())
    save(out / "sfx_select.wav", blip([72], 0.05, level=0.4))
    save(out / "sfx_radio.wav", blip([84, 79, 84, 79], 0.05, 0.25, 0.3))
    save(out / "sfx_sea.wav", noise_hit(0.5, 0.8, 6, 1))
    save(out / "sfx_debris.wav", noise_hit(0.15, 0.8, 2, 2))
    save(out / "sfx_shoal.wav", noise_hit(0.7, 1.0, 14, 3))
    save(out / "sfx_rogue.wav", noise_hit(1.2, 1.0, 10, 4))
    save(out / "sfx_bell.wav", bell())


if __name__ == "__main__":
    main()
