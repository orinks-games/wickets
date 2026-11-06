"""Write every Larkfield sound into `sound-specs/larkfield.json`.

Larkfield builds its in-game tune live (see `games/larkfield/src/music.rs`).
The tune is in G major at 84 beats a minute, one bar (4 beats, about 2.9
seconds) at a time. Each bar the game picks a one-bar phrase for every
part that is playing, over the bar's chord:

- guitar: you, fingerpicked, always playing, in the middle
- fiddle, flute, accordion, harp, cello, clarinet: one per settler, by the
  order they arrived; each is mono, so the game can play it from wherever
  that settler is working on the lane

The menus keep fixed renders: the title theme (16 bars), the supper tune
heard while supper's pages are read, and the pause theme. The room is light
everywhere (reverb mix 0.08 to 0.15), and the menu music's instruments are
panned apart.

Every sound heard on the lane is mono, so the game can place it.

The same seed always writes the same file. Run, then render:

    python tools/larkfield_music.py
    python tools/render_sounds.py larkfield

If you change how many phrases a part has, change `VARIANTS` in
`games/larkfield/src/music.rs` to match; a test checks every key the game
can ask for has an Ogg.
"""

from __future__ import annotations

import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "larkfield.json"
SEED = 2026_11_07

BPM = 84
TAIL = 1.2
ROOM = {"type": "reverb", "mix": 0.12, "size": 0.5}
MONO = {"type": "mono"}
PLACED = [MONO, {"type": "reverb", "mix": 0.08, "size": 0.4}, MONO]

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
SCALE = {7, 9, 11, 0, 2, 4, 6}  # G major's pitch classes, C = 0

# Chord: (file name, root pitch class, chord tones)
CHORDS = [
    ("g", 7, (7, 11, 2)),
    ("c", 0, (0, 4, 7)),
    ("d", 2, (2, 6, 9)),
    ("em", 4, (4, 7, 11)),
    ("am", 9, (9, 0, 4)),
]

# Phrases per part; music.rs `VARIANTS` must match.
VARIANTS = 2


def midi(name: str) -> int:
    pc, octave = name[:-1], int(name[-1])
    return NAMES.index(pc) + 12 * (octave + 1)


def note(m: int) -> str:
    return f"{NAMES[m % 12]}{m // 12 - 1}"


def tones_from(chord: tuple, low: str, count: int) -> list[int]:
    """Chord tones going up from the first one at or above `low`."""
    out, m = [], midi(low)
    while len(out) < count:
        if m % 12 in chord[2]:
            out.append(m)
        m += 1
    return out


def stacked(chord: tuple, low: str, count: int) -> list[int]:
    """The chord from its root at or above `low`."""
    lo = midi(low)
    root = lo + (chord[1] - lo) % 12
    return tones_from(chord, note(root), count)


def steps(cells) -> str:
    return " ".join(f"{'-' if m is None else note(m)}:{d:g}" for m, d in cells)


def by_degree(tones: list[int], pattern) -> str:
    return steps([(None if d is None else tones[d], beats) for d, beats in pattern])


def seq(inst: str, text: str, gain: float, vel: float = 0.5, params: dict | None = None,
        fx: list | None = None, at: float = 0) -> dict:
    layer = {"type": "seq", "inst": inst, "steps": text, "gain": gain, "at": at, "vel": vel, "step": 1}
    if params:
        layer["params"] = params
    if fx:
        layer["fx"] = fx
    return layer


def sfx(kind: str, params: dict, gain: float = 1.0, at: float = 0, fx: list | None = None) -> dict:
    layer = {"type": "sfx", "kind": kind, "at": at, "gain": gain, "params": params}
    if fx:
        layer["fx"] = fx
    return layer


def sound(out: str, layers: list, fx: list | None = None, normalize: float = -16, **extra) -> dict:
    s = {"genny": "blal1", "out": out, "layers": layers, "fx": fx if fx is not None else [ROOM], "normalize": normalize}
    s.update(extra)
    return s


def pan(pos: float) -> dict:
    return {"type": "pan", "pos": pos}


def scale_step(m: int, by: int) -> int:
    step = 1 if by > 0 else -1
    for _ in range(abs(by)):
        m += step
        while m % 12 not in SCALE:
            m += step
    return m


def nearest_tone(chord: tuple, target: int, low: int, high: int) -> int:
    best = None
    for m in range(low, high + 1):
        if m % 12 in chord[2] and (best is None or abs(m - target) < abs(best - target)):
            best = m
    return best


RHYTHMS = [
    [1.5, .5, 2],
    [1, 1, 2],
    [2, 1, 1],
    [1, .5, .5, 2],
    [.5, .5, 1, 2],
    [1, 1, 1, 1],
]


def tune(chord: tuple, rng: random.Random, low: str, high: str, rhythms=RHYTHMS) -> str:
    """One bar of tune: a chord tone on each strong beat, scale steps between."""
    lo, hi = midi(low), midi(high)
    rhythm = rng.choice(rhythms)
    m = nearest_tone(chord, (lo + hi) // 2 + rng.randint(-3, 3), lo, hi)
    cells, at = [], 0.0
    for i, beats in enumerate(rhythm):
        if i > 0:
            if at in (0.0, 2.0):
                m = nearest_tone(chord, m + rng.choice([-3, -2, 2, 3]), lo, hi)
            else:
                by = rng.choice([-2, -1, 1, 2])
                if not lo <= scale_step(m, by) <= hi:
                    by = -by
                m = scale_step(m, by)
        cells.append((m, beats))
        at += beats
    return steps(cells)


def phrase(out: str, layers: list, normalize: float, placed: bool = True) -> dict:
    return sound(out, layers, PLACED if placed else [{"type": "width", "amount": 0.3}, ROOM], normalize,
                 bpm=BPM, beats=4, tail=TAIL)


# ---------------------------------------------------------------- the band

GUITAR = [
    [(0, .5), (2, .5), (3, .5), (2, .5), (1, .5), (2, .5), (3, .5), (2, .5)],
    [(0, 1), (2, .5), (3, .5), (1, 1), (2, .5), (3, .5)],
]
HARP = [
    [(0, .5), (1, .5), (2, .5), (3, .5), (4, .5), (3, .5), (2, .5), (1, .5)],
    [(0, .25), (1, .25), (2, .5), (3, 1), (2, .5), (4, .5), (3, 1)],
]
CELLO = [
    [(0, 2), (2, 2)],
    [(0, 1), (1, 1), (2, 1.5), (1, .5)],
]


def band() -> list[dict]:
    rng = random.Random(SEED)
    out = []
    for chord in CHORDS:
        name = chord[0]
        for n, pat in enumerate(GUITAR):
            out.append(phrase(f"band/guitar/{name}_{n}.wav",
                              [seq("guitar", by_degree(stacked(chord, "E3", 4), pat), 0.6, 0.45)], -17, placed=False))
        for n in range(VARIANTS):
            out.append(phrase(f"band/fiddle/{name}_{n}.wav",
                              [seq("violin", tune(chord, rng, "G4", "G5"), 0.5, 0.5, {"vibrato": 0.6})], -18))
            long_notes = [[2, 2], [3, 1], [1, 1, 2], [4]]
            out.append(phrase(f"band/flute/{name}_{n}.wav",
                              [seq("flute", tune(chord, rng, "D5", "D6", long_notes), 0.45, 0.45, {"breath": 0.2})], -19))
            c = tones_from(chord, "B3", 3)
            ch = f"[{','.join(note(m) for m in c)}]"
            accordion = [f"-:1 {ch}:1 -:1 {ch}:1", f"-:.5 {ch}:.5 -:.5 {ch}:.5 -:1 {ch}:1"][n]
            out.append(phrase(f"band/accordion/{name}_{n}.wav",
                              [seq("accordion", accordion, 0.35, 0.4)], -19))
            out.append(phrase(f"band/harp/{name}_{n}.wav",
                              [seq("harp", by_degree(stacked(chord, "G3", 5), HARP[n]), 0.45, 0.45)], -18))
            out.append(phrase(f"band/cello/{name}_{n}.wav",
                              [seq("cello", by_degree(stacked(chord, "C2", 3), CELLO[n]), 0.55, 0.5)], -17))
            out.append(phrase(f"band/clarinet/{name}_{n}.wav",
                              [seq("clarinet", tune(chord, rng, "G3", "G4"), 0.45, 0.45)], -19))
    return out


# ----------------------------------------------------------- menu music

PROG = ["[G3,B3,D4]", "[C3,E3,G3]", "[E3,G3,B3]", "[D3,F#3,A3]"]
PROG_B = ["[C3,E3,G3]", "[G3,B3,D4]", "[A3,C4,E4]", "[D3,F#3,A3]"]


def title_music() -> dict:
    bass = "G2:2 D2:2 | C2:2 G2:2 | E2:2 B1:2 | D2:2 A2:2"
    bass_b = "C2:2 G2:2 | G2:2 D2:2 | A2:2 E2:2 | D2:2 A2:2"
    guitar = ("G3:.5 B3:.5 D4:.5 B3:.5 G3:.5 B3:.5 D4:.5 B3:.5 | C3:.5 E3:.5 G3:.5 E3:.5 C3:.5 E3:.5 G3:.5 E3:.5 | "
              "E3:.5 G3:.5 B3:.5 G3:.5 E3:.5 G3:.5 B3:.5 G3:.5 | D3:.5 F#3:.5 A3:.5 F#3:.5 D3:.5 F#3:.5 A3:.5 F#3:.5")
    guitar_b = ("C3:.5 E3:.5 G3:.5 E3:.5 C3:.5 E3:.5 G3:.5 E3:.5 | G3:.5 B3:.5 D4:.5 B3:.5 G3:.5 B3:.5 D4:.5 B3:.5 | "
                "A3:.5 C4:.5 E4:.5 C4:.5 A3:.5 C4:.5 E4:.5 C4:.5 | D3:.5 F#3:.5 A3:.5 F#3:.5 D3:.5 F#3:.5 A3:.5 F#3:.5")
    fiddle_a = "B4:1.5 A4:.5 G4:2 | E5:1 D5:1 C5:2 | B4:1 G4:1 E4:2 | F#4:2 A4:2"
    fiddle_b = "E5:1.5 D5:.5 C5:2 | D5:1 B4:1 G4:2 | A4:1 C5:1 E5:2 | D5:3 -:1"
    flute_a = "-:4 | G5:2 E5:2 | -:4 | A5:2 F#5:2"
    rest = "-:4 | -:4 | -:4 | -:4"
    acc = " | ".join(f"-:1 {c}:1 -:1 {c}:1" for c in PROG)
    acc_b = " | ".join(f"-:1 {c}:1 -:1 {c}:1" for c in PROG_B)
    return sound("music/title.wav", [
        seq("guitar", " | ".join([guitar, guitar, guitar_b, guitar]), 0.4, 0.45),
        seq("cello", " | ".join([bass, bass, bass_b, bass]), 0.35, 0.45),
        seq("violin", " | ".join([rest, fiddle_a, fiddle_b, fiddle_a]), 0.3, 0.45, {"vibrato": 0.6}, fx=[pan(-0.3)]),
        seq("flute", " | ".join([rest, rest, flute_a, flute_a]), 0.25, 0.4, {"breath": 0.2}, fx=[pan(0.35)]),
        seq("accordion", " | ".join([rest, acc, acc_b, acc]), 0.18, 0.4, fx=[pan(0.2)]),
        sfx("dawn_chorus", {"birds": 4, "dur": 45.0, "seed": 3}, 0.08),
    ], [ROOM, {"type": "lowpass", "cutoff": 7000}], -16, bpm=BPM, beats=64, loop="fold")


def supper_music() -> dict:
    # 8 bars, a little slower: the table after a day's work.
    harp = ("G3:.5 B3:.5 D4:.5 G4:.5 D4:.5 B3:.5 D4:.5 B3:.5 | C3:.5 E3:.5 G3:.5 C4:.5 G3:.5 E3:.5 G3:.5 E3:.5 | "
            "E3:.5 G3:.5 B3:.5 E4:.5 B3:.5 G3:.5 B3:.5 G3:.5 | D3:.5 F#3:.5 A3:.5 D4:.5 A3:.5 F#3:.5 A3:.5 F#3:.5")
    clar = "D4:2 B3:2 | E4:2 G4:2 | G4:1 F#4:1 E4:2 | F#4:3 -:1 | B4:2 G4:2 | C5:2 G4:2 | B4:1 A4:1 G4:2 | A4:2 F#4:2"
    return sound("music/supper.wav", [
        seq("harp", harp + " | " + harp, 0.35, 0.4, fx=[pan(-0.3)]),
        seq("clarinet", clar, 0.3, 0.4, fx=[pan(0.3)]),
        seq("cello", "G2:4 | C2:4 | E2:4 | D2:4 | G2:4 | C2:4 | E2:4 | D2:4", 0.3, 0.4),
        sfx("fire", {"dur": 26.0, "size": 0.3, "crackle": 6.0, "seed": 4}, 0.08),
    ], [ROOM], -17, bpm=76, beats=32, loop="fold")


def pause_music() -> dict:
    # 8 bars at 80 a minute: 24 seconds.
    pad = " | ".join(f"{c}:4" for c in PROG)
    kal = ("D5:1 B4:1 G4:2 | E5:1 C5:1 G4:2 | B4:1 G4:1 E4:2 | A4:1 F#4:1 D4:2 | "
           "D5:1 G5:1 B4:2 | C5:1 E5:1 G5:2 | B4:1 E5:1 G5:2 | F#5:2 A4:2")
    return sound("music/pause.wav", [
        seq("accordion", pad + " | " + pad, 0.15, 0.35, fx=[{"type": "width", "amount": 0.4}]),
        seq("harp", kal, 0.35, 0.4, fx=[pan(-0.25)]),
        seq("cello", "G2:4 | C2:4 | E2:4 | D2:4 | G2:4 | C2:4 | E2:4 | D2:4", 0.3, 0.4),
    ], [ROOM], -18, bpm=80, beats=32, loop="fold")


# ------------------------------------------------------- on the lane

def ambience() -> list[dict]:
    return [
        sound("ambience/day.wav", [
            sfx("dawn_chorus", {"birds": 5, "dur": 12.0, "seed": 2}, 0.5),
            sfx("wind", {"dur": 12.0, "scale": 0.4, "seed": 3}, 0.2),
        ], [{"type": "width", "amount": 0.5}], -26, duration=12.0, loop=0.8),
        sound("ambience/rain.wav", [
            sfx("rain", {"dur": 12.0, "intensity": 0.6, "seed": 4}, 0.7),
            sfx("wind", {"dur": 12.0, "scale": 0.6, "seed": 5}, 0.2),
        ], [{"type": "width", "amount": 0.5}], -25, duration=12.0, loop=0.8),
        sound("ambience/winter.wav", [
            sfx("wind", {"dur": 12.0, "scale": 1.2, "seed": 6}, 0.5),
            sfx("gust", {"dur": 6.0, "through": "gap", "strength": 0.3, "seed": 7}, 0.2, at=3.0),
        ], [{"type": "width", "amount": 0.5}], -27, duration=12.0, loop=0.8),
        sound("loop/river.wav", [
            sfx("stream", {"dur": 8.0, "seed": 8}, 0.6),
            sfx("babble", {"dur": 8.0, "flow": 0.5, "size": 1.2, "turbulence": 0.3, "seed": 9}, 0.3),
        ], [MONO], -20, duration=8.0, loop=0.6),
    ]


def cues() -> list[dict]:
    m = PLACED
    step = lambda ground, seed: [sfx("footsteps", {"ground": ground, "steps": 1, "weight": 0.8, "seed": seed}, 0.8)]
    animal = lambda **p: sfx("animal", p)
    return [
        sound("move/lane.wav", step("gravel", 1), [MONO], -18),
        sound("move/inside.wav", step("grass", 2), [MONO], -18),
        sound("move/field.wav", step("mud", 3), [MONO], -18),
        # A soft knock on a fence post: the end of the lane or the field.
        sound("move/edge.wav", [sfx("thud", {"body": "box", "size": 0.4, "force": 0.4, "seed": 21}, 0.7),
                                sfx("creak", {"material": "wood", "force": "ramp", "dur": 0.2, "size": 0.5, "seed": 22}, 0.2, at=0.03)], [MONO], -19),
        # Work, placed where it happens.
        sound("work/water.wav", [sfx("pour", {"dur": 0.9, "flow": 0.7, "material": "metal", "seed": 1}, 0.8)], m, -17),
        sound("work/dig.wav", [sfx("thud", {"body": "box", "size": 0.5, "force": 0.6, "seed": 2}, 0.7),
                               sfx("shake", {"preset": "gravel", "energy": 0.4, "dur": 0.4, "seed": 3}, 0.6, at=0.05)], m, -16),
        sound("work/sow.wav", [sfx("shake", {"preset": "sandpaper", "energy": 0.3, "dur": 0.5, "seed": 4}, 0.6)], m, -18),
        sound("work/weed.wav", [sfx("cloth", {"kind": "rustle", "dur": 0.5, "weight": 0.3, "seed": 5}, 0.7),
                                sfx("crumple", {"kind": "paper", "energy": 0.2, "density": 0.5, "size": 0.3, "dur": 0.3, "seed": 6}, 0.4)], m, -17),
        sound("work/harvest.wav", [sfx("cloth", {"kind": "rustle", "dur": 0.4, "weight": 0.5, "seed": 7}, 0.6),
                                   sfx("thud", {"body": "box", "size": 0.3, "force": 0.4, "seed": 8}, 0.6, at=0.3),
                                   seq("harp", "G5:.25 B5:.5", 0.25, 0.4, at=0.35)], m, -16, bpm=BPM),
        sound("work/cook.wav", [sfx("sizzle", {"dur": 1.0, "intensity": 0.5, "seed": 9}, 0.6),
                                sfx("clink", {"object": "bottle", "size": 0.8, "force": 0.4, "seed": 10}, 0.4)], m, -18),
        sound("work/hammer.wav", [sfx("hammering", {"strikes": 3, "pace": 3.0, "seed": 11}, 0.8)], m, -16),
        sound("work/chop.wav", [sfx("impact", {"material": "wood", "shape": "bar", "size": 0.6, "hardness": 0.9,
                                               "velocity": 3.0, "seed": 12}, 0.8),
                                sfx("creak", {"material": "wood", "force": "ramp", "dur": 0.3, "size": 0.8, "seed": 13}, 0.3, at=0.05)], m, -15),
        sound("work/pick.wav", [sfx("clang", {"object": "sword", "size": 0.6, "force": 0.9, "damping": 4.0, "dur": 0.5, "seed": 14}, 0.6),
                                sfx("shake", {"preset": "gravel", "energy": 0.5, "dur": 0.4, "seed": 15}, 0.5, at=0.05)], m, -16),
        sound("work/hens.wav", [animal(size=0.35, pitch=520.0, contour="fall", excursion=0.3, roughness=0.6,
                                       vowels="ae-E", dur=0.14, seed=16),
                                {**animal(size=0.35, pitch=560.0, contour="fall", excursion=0.3, roughness=0.6,
                                          vowels="ae-E", dur=0.12, seed=17), "at": 0.22},
                                {**animal(size=0.35, pitch=500.0, contour="arch", excursion=0.4, roughness=0.6,
                                          vowels="E-ae", dur=0.3, seed=18), "at": 0.42}], m, -15),
        sound("work/goat.wav", [animal(size=0.8, pitch=330.0, contour="arch", excursion=0.25, roughness=0.9,
                                       vowels="E-ae", dur=0.6, seed=19)], m, -15),
        sound("work/feed.wav", [sfx("shake", {"preset": "gravel", "energy": 0.3, "dur": 0.6, "seed": 20}, 0.6)], m, -18),
        sound("work/cast.wav", [sfx("whoosh", {"dur": 0.4, "low": 500, "high": 2500, "direction": "up"}, 0.5),
                                sfx("splash", {"size": 0.15, "dur": 0.4, "droplets": 2, "seed": 21}, 0.6, at=0.45)], m, -17),
        sound("work/splash.wav", [sfx("splash", {"size": 0.3, "dur": 0.5, "droplets": 4, "seed": 22}, 0.7)], m, -17),
        sound("work/reel.wav", [sfx("ratchet", {"clicks": 12, "rate": 24.0, "size": 0.6, "seed": 23}, 0.5),
                                sfx("splash", {"size": 0.35, "dur": 0.6, "droplets": 5, "seed": 24}, 0.7, at=0.4)], m, -16),
        sound("work/forage.wav", [sfx("cloth", {"kind": "rustle", "dur": 0.6, "weight": 0.4, "seed": 25}, 0.6),
                                  sfx("gust", {"dur": 0.6, "through": "leaves", "strength": 0.3, "seed": 26}, 0.4)], m, -18),
        sound("work/talk.wav", [sfx("cloth", {"kind": "rustle", "dur": 0.3, "weight": 0.3, "seed": 27}, 0.4)], m, -24),
        # The fish bites: placed at the river.
        sound("fish/bite.wav", [sfx("splash", {"size": 0.2, "dur": 0.3, "droplets": 3, "seed": 28}, 0.7),
                                seq("handbell", "E6:.5", 0.5, 0.5)], m, -13, bpm=BPM, tail=0.4),
        # The day: not placed.
        sound("day/morning.wav", [sfx("birdsong", {"species": "rooster", "seed": 29}, 0.6, fx=[pan(-0.4)])], [ROOM], -20),
        sound("day/supper_bell.wav", [seq("handbell", "G5:.5 D5:.5 G5:1.5", 0.5, 0.5, {"decay": 2.0})],
              [ROOM], -15, bpm=BPM, tail=1.2),
        sound("day/built.wav", [seq("harp", "G4:.25 B4:.25 D5:.25 G5:1", 0.4, 0.45),
                                sfx("hammering", {"strikes": 2, "pace": 3.0, "seed": 30}, 0.4)], PLACED, -16, bpm=BPM, tail=0.8),
        sound("day/arrived.wav", [sfx("knock", {"count": 2, "pace": 4.0, "material": "wood", "size": 1.0, "force": 0.6, "seed": 31}, 0.6),
                                  seq("celesta", "D5:.5 G5:1", 0.35, 0.4, at=0.5)], [ROOM], -17, bpm=BPM, tail=0.8),
        sound("day/keepsake.wav", [seq("music_box", "G6:.5 B6:.5 D7:1 B6:.5 A6:.5 G6:1.5", 0.5, 0.45)],
              [ROOM], -17, bpm=96, tail=1.2),
        sound("day/goal.wav", [seq("celesta", "G5:.25 B5:.25 D6:.25 G6:1", 0.45, 0.45, fx=[pan(-0.2)]),
                               seq("handbell", "-:.75 G6:1", 0.3, 0.4, fx=[pan(0.2)])], [ROOM], -16, bpm=BPM, tail=0.8),
    ]


def menu() -> list[dict]:
    end = [{"type": "mono"}, {"type": "fade", "out": 0.04}]
    return [
        {"genny": "blal1", "out": "menu/move.wav", "_about": "Menu move: one soft step on grass.", "duration": 0.2,
         "layers": [sfx("footsteps", {"ground": "grass", "steps": 1, "weight": 0.5, "seed": 41}, 0.6)],
         "fx": [{"type": "lowpass", "cutoff": 3000}] + end, "normalize": -9, "max_loudness": -28},
        {"genny": "blal1", "out": "menu/select.wav", "_about": "Menu select: a guitar note.",
         "duration": 0.45,
         "layers": [seq("guitar", "G4:1", 0.6, 0.5)],
         "fx": [{"type": "reverb", "mix": 0.12, "size": 0.3}] + end,
         "bpm": 160, "normalize": -6, "max_loudness": -22},
        {"genny": "blal1", "out": "menu/back.wav", "_about": "Menu back: a soft rustle.", "duration": 0.3,
         "layers": [sfx("cloth", {"kind": "rustle", "dur": 0.25, "weight": 0.4, "seed": 42}, 0.6)],
         "fx": [{"type": "highpass", "cutoff": 200}, {"type": "lowpass", "cutoff": 3000}] + end,
         "normalize": -9, "max_loudness": -26},
    ]


def main() -> None:
    spec = {
        "genny": "blal1",
        "_about": "Larkfield. Written by tools/larkfield_music.py: edit that, not this file.",
        "sounds": [title_music(), supper_music(), pause_music()] + ambience() + cues() + menu() + band(),
    }
    SPEC.write_text(json.dumps(spec, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {len(spec['sounds'])} sounds to {SPEC.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
