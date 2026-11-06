"""Write every Little Rapids sound into `sound-specs/little-rapids.json`.

Little Rapids builds its in-game song live (see
`games/little-rapids/src/music.rs`). The song is in D major at 96 beats a
minute, one bar (4 beats, 2.5 seconds) at a time. Each bar the game picks a
one-bar phrase for every part that is playing, over the bar's chord:

- bass: upright bass, in the middle
- pluck: harp, a little to the left
- pad: a warm pad, wide
- counter: a flute tune, a little to the right
- drums: brushes and a shaker, near the middle (no chord)

Each catch plays a note from the bar's chord on the kalimba, climbing four
steps as the combo grows (`song/note/<chord>_<k>`).

The menus keep fixed renders: the title theme (16 bars, about 40 seconds),
the pause theme (about 24 seconds), and short pieces for the river mouth
and for sinking. The room is light everywhere (reverb mix 0.08 to 0.15),
and the music's instruments are panned apart.

Every sound the boat hears on the water is mono, so the game can place it.

The same seed always writes the same file. Run, then render:

    python tools/rapids_music.py
    python tools/render_sounds.py little-rapids

If you change how many phrases a part has, change `variants` in
`games/little-rapids/src/music.rs` to match; a test checks every key the
game can ask for has an Ogg.
"""

from __future__ import annotations

import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "little-rapids.json"
SEED = 2026_11_06

BPM = 96
TAIL = 1.2
ROOM = {"type": "reverb", "mix": 0.12, "size": 0.5}
MONO = {"type": "mono"}

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
SCALE = {2, 4, 6, 7, 9, 11, 1}  # D major's pitch classes, C = 0

# Chord: (file name, root pitch class, chord tones)
CHORDS = [
    ("d", 2, (2, 6, 9)),
    ("bm", 11, (11, 2, 6)),
    ("g", 7, (7, 11, 2)),
    ("a", 9, (9, 1, 4)),
    ("em", 4, (4, 7, 11)),
]

# Phrases per part; music.rs `Part::variants` must match.
VARIANTS = {"bass": 3, "pluck": 4, "pad": 2, "counter": 3, "drums": 3}
NOTES = 4
PAN = {"bass": 0.0, "pluck": -0.35, "counter": 0.35, "drums": 0.1}


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


def sfx(kind: str, params: dict, gain: float = 1.0, at: float = 0, fx: list | None = None, **extra) -> dict:
    layer = {"type": "sfx", "kind": kind, "at": at, "gain": gain, "params": params}
    if fx:
        layer["fx"] = fx
    layer.update(extra)
    return layer


def drum(kind: str, params: dict, gain: float = 0.6, at: float = 0, vel: float = 0.5) -> dict:
    return {"type": "drum", "kind": kind, "gain": gain, "at": at, "vel": vel, "params": params}


def sound(out: str, layers: list, fx: list | None = None, normalize: float = -16, **extra) -> dict:
    s = {"genny": "blal1", "out": out, "layers": layers, "fx": fx if fx is not None else [ROOM], "normalize": normalize}
    s.update(extra)
    return s


def pan(pos: float) -> dict:
    return {"type": "pan", "pos": pos}


# ---------------------------------------------------------------- the song

BASS = [
    [(0, 2), (2, 1), (0, 1)],
    [(0, 1.5), (0, 0.5), (2, 1), (1, 1)],
    [(0, 1), (None, 0.5), (0, 0.5), (2, 1.5), (3, 0.5)],
]
PLUCK = [
    [(0, .5), (1, .5), (2, .5), (3, .5), (2, .5), (1, .5), (2, .5), (1, .5)],
    [(0, 1), (2, .5), (3, .5), (2, 1), (1, 1)],
    [(2, .5), (1, .5), (0, .5), (1, .5), (2, .5), (3, .5), (4, 1)],
    [(0, .5), (2, .5), (1, .5), (3, .5), (2, .5), (4, .5), (3, 1)],
]
RHYTHMS = [
    [1.5, .5, 2],
    [1, 1, 2],
    [2, 1, 1],
    [1, .5, .5, 2],
    [.5, .5, 1, 2],
]
TUNE_LOW, TUNE_HIGH, TUNE_HOME = midi("D5"), midi("D6"), midi("A5")
DRUMS = [
    "x---x---x---x---",
    "x-----x-x---x---",
    "x---x-x-x---x-x-",
]
SHAKE = [
    "--x---x---x---x-",
    "x-x-x-x-x-x-x-x-",
    "--x---x---x-x-x-",
]


def scale_step(m: int, by: int) -> int:
    step = 1 if by > 0 else -1
    for _ in range(abs(by)):
        m += step
        while m % 12 not in SCALE:
            m += step
    return m


def nearest_tone(chord: tuple, target: int) -> int:
    best = None
    for m in range(TUNE_LOW, TUNE_HIGH + 1):
        if m % 12 in chord[2] and (best is None or abs(m - target) < abs(best - target)):
            best = m
    return best


def counter_phrase(chord: tuple, rng: random.Random) -> str:
    """One bar of flute: a chord tone on each strong beat, scale steps between."""
    rhythm = rng.choice(RHYTHMS)
    m = nearest_tone(chord, TUNE_HOME + rng.randint(-4, 4))
    cells, at = [], 0.0
    for i, beats in enumerate(rhythm):
        if i > 0:
            if at in (0.0, 2.0):
                m = nearest_tone(chord, m + rng.choice([-3, -2, 2, 3]))
            else:
                by = rng.choice([-2, -1, 1, 2])
                if not TUNE_LOW <= scale_step(m, by) <= TUNE_HIGH:
                    by = -by
                m = scale_step(m, by)
        cells.append((m, beats))
        at += beats
    return steps(cells)


def phrase(out: str, layers: list, part: str, normalize: float) -> dict:
    fx = [ROOM]
    if part in PAN:
        fx = [pan(PAN[part]), ROOM]
    return sound(out, layers, fx, normalize, bpm=BPM, beats=4, tail=TAIL)


def song() -> list[dict]:
    rng = random.Random(SEED)
    out = []
    for chord in CHORDS:
        name = chord[0]
        for n, pat in enumerate(BASS):
            out.append(phrase(f"song/bass/{name}_{n}.wav",
                              [seq("upright_bass", by_degree(stacked(chord, "D2", 4), pat), 0.8, 0.6)], "bass", -16))
        for n, pat in enumerate(PLUCK):
            out.append(phrase(f"song/pluck/{name}_{n}.wav",
                              [seq("harp", by_degree(stacked(chord, "D4", 5), pat), 0.45, 0.5)], "pluck", -17))
        pad = tones_from(chord, "F#3", 3)
        out.append(phrase(f"song/pad/{name}_0.wav",
                          [seq("warm_pad", f"[{','.join(note(m) for m in pad)}]:4", 0.4, 0.65,
                               {"attack": 0.4, "release": 1.0, "cutoff": 900}), ],
                          "pad", -19) | {"fx": [{"type": "width", "amount": 0.5}, ROOM]})
        high = tones_from(chord, "A3", 3)
        out.append(phrase(f"song/pad/{name}_1.wav",
                          [seq("warm_pad", f"[{','.join(note(m) for m in high)}]:2 [{','.join(note(m) for m in high)}]:2",
                               0.4, 0.6, {"attack": 0.3, "release": 0.9, "cutoff": 1000})],
                          "pad", -19) | {"fx": [{"type": "width", "amount": 0.5}, ROOM]})
        for n in range(VARIANTS["counter"]):
            out.append(phrase(f"song/counter/{name}_{n}.wav",
                              [seq("flute", counter_phrase(chord, rng), 0.45, 0.5, {"breath": 0.2})], "counter", -19))
        for k, m in enumerate(tones_from(chord, "D5", NOTES)):
            out.append(sound(f"song/note/{name}_{k}.wav", [seq("kalimba", f"{note(m)}:1", 0.6, 0.6)],
                             [ROOM], -17, bpm=BPM, beats=1, tail=0.6))
    for n in range(VARIANTS["drums"]):
        out.append(phrase(f"song/drums/{n}.wav", [
            {"type": "pattern", "kind": "brush", "steps": DRUMS[n], "bars": 1, "gain": 0.35},
            {"type": "pattern", "kind": "shaker", "steps": SHAKE[n], "bars": 1, "gain": 0.18},
        ], "drums", -18))
    return out


# ----------------------------------------------------------- menu music

def title_music() -> dict:
    prog = ["[D3,F#3,A3]", "[B2,D3,F#3]", "[G2,B2,D3]", "[A2,C#3,E3]"]
    bass = "D2:2 A2:2 | B1:2 F#2:2 | G1:2 D2:2 | A1:2 E2:2"
    harp = ("D4:.5 F#4:.5 A4:.5 D5:.5 A4:.5 F#4:.5 A4:.5 F#4:.5 | B3:.5 D4:.5 F#4:.5 B4:.5 F#4:.5 D4:.5 F#4:.5 D4:.5 | "
            "G3:.5 B3:.5 D4:.5 G4:.5 D4:.5 B3:.5 D4:.5 B3:.5 | A3:.5 C#4:.5 E4:.5 A4:.5 E4:.5 C#4:.5 E4:.5 C#4:.5")
    tune_a = "F#5:1.5 E5:.5 D5:2 | B4:1 D5:1 F#5:2 | G5:1 F#5:1 D5:2 | E5:3 -:1"
    tune_b = "A5:1.5 G5:.5 F#5:2 | D5:1 F#5:1 B5:2 | B5:1 A5:1 G5:1 F#5:1 | E5:2 C#5:1 E5:1"
    rest = "-:4 | -:4 | -:4 | -:4"
    pad = " | ".join(f"{c}:4" for c in prog)
    return sound("music/title.wav", [
        seq("warm_pad", " | ".join([pad] * 4), 0.25, 0.55, {"attack": 1.0, "release": 1.5, "cutoff": 800},
            fx=[{"type": "width", "amount": 0.5}]),
        seq("upright_bass", " | ".join([bass] * 4), 0.45, 0.45),
        seq("harp", " | ".join([harp] * 4), 0.3, 0.4, fx=[pan(-0.35)]),
        seq("flute", " | ".join([rest, tune_a, tune_b, tune_a]), 0.3, 0.45, {"breath": 0.2}, fx=[pan(0.35)]),
        {"type": "pattern", "kind": "brush", "steps": "x---x---x---x---", "bars": 16, "gain": 0.1, "swing": 0.1},
        sfx("stream", {"dur": 40.0, "seed": 4}, 0.12),
    ], [ROOM, {"type": "lowpass", "cutoff": 6000}], -16, bpm=BPM, beats=64, loop="fold")


def pause_music() -> dict:
    # 8 bars at 80 a minute: 24 seconds.
    prog = ["[D3,F#3,A3]", "[G2,B2,D3]", "[B2,D3,F#3]", "[A2,C#3,E3]"]
    pad = " | ".join(f"{c}:4" for c in prog)
    kal = ("A4:1 F#4:1 D4:2 | B4:1 G4:1 D4:2 | F#4:1 D4:1 B3:2 | E4:1 C#4:1 A3:2 | "
           "A4:1 D5:1 F#4:2 | G4:1 B4:1 D5:2 | F#4:1 B4:1 D5:2 | C#5:2 E4:2")
    return sound("music/pause.wav", [
        seq("warm_pad", pad + " | " + pad, 0.3, 0.5, {"attack": 1.0, "release": 1.5, "cutoff": 700},
            fx=[{"type": "width", "amount": 0.5}]),
        seq("kalimba", kal, 0.35, 0.4, fx=[pan(-0.25)]),
        seq("upright_bass", "D2:4 | G1:4 | B1:4 | A1:4 | D2:4 | G1:4 | B1:4 | A1:4", 0.35, 0.4),
    ], [ROOM], -18, bpm=80, beats=32, loop="fold")


def mouth_music() -> dict:
    return sound("music/mouth.wav", [
        seq("hand_chime", "D5:.5 F#5:.5 A5:.5 D6:2.5", 0.4, 0.55, fx=[pan(0.25)]),
        seq("harp", "D4:.25 F#4:.25 A4:.25 D5:.25 F#5:.25 A5:.25 D6:2.5", 0.35, 0.45, fx=[pan(-0.3)]),
        seq("warm_pad", "[D3,F#3,A3,D4]:4", 0.3, 0.6, {"attack": 0.2, "release": 1.5}),
        seq("upright_bass", "D2:4", 0.4, 0.5),
    ], [ROOM], -16, bpm=BPM, beats=4, tail=2.0)


def sank_music() -> dict:
    return sound("music/sank.wav", [
        seq("kalimba", "A4:1 F#4:1 E4:1 D4:2", 0.4, 0.45, fx=[pan(-0.2)]),
        seq("warm_pad", "[G2,B2,D3]:2 [D3,F#3,A3]:3", 0.3, 0.5, {"attack": 0.3, "release": 1.5, "cutoff": 700}),
        seq("upright_bass", "G1:2 D2:3", 0.4, 0.45),
    ], [ROOM], -17, bpm=80, beats=5, tail=2.0)


# ------------------------------------------------------- on the water

def loops() -> list[dict]:
    beat = 60 / BPM
    out = [
        sound("ambience/water.wav", [
            sfx("stream", {"dur": 12.0, "seed": 2}, 0.6),
            sfx("babble", {"dur": 12.0, "flow": 0.5, "size": 1.4, "turbulence": 0.3, "seed": 3}, 0.25),
        ], [{"type": "width", "amount": 0.4}], -24, duration=12.0, loop=0.8),
        sound("loop/bank.wav", [
            sfx("babble", {"dur": 6.0, "flow": 0.6, "size": 0.8, "turbulence": 0.6, "seed": 5}, 0.7),
            sfx("drops", {"dur": 6.0, "rate": 2.5, "jitter": 0.6, "size": 0.5, "seed": 6}, 0.25),
        ], [MONO], -18, duration=6.0, loop=0.5),
        sound("loop/current.wav", [
            sfx("noise", {"dur": 6.0, "color": "pink", "attack": 0.3}, 0.5,
                fx=[{"type": "bandpass", "center": 1800, "q": 0.6}]),
            sfx("stream", {"dur": 6.0, "seed": 8}, 0.5),
        ], [MONO], -22, duration=6.0, loop=0.6),
    ]
    bar = 4 * beat
    floats = {
        # A soft bell on every beat, high on the water.
        "petal": [seq("bell", "A5:1 -:1 A5:1 -:1", 0.35, 0.35, {"decay": 0.6, "bright": 0.4})],
        "leaf": [seq("marimba", "D4:1 -:1 D4:1 -:1", 0.6, 0.45)],
        "scrap": [sfx("paper", {"kind": "rustle", "dur": 0.5, "weight": 0.4, "seed": n}, 0.7, at=n * beat)
                  for n in range(4)],
        "trinket": [sfx("magic", {"dur": bar, "density": 5, "freq": "E6"}, 0.4)],
        "lantern": [seq("glass_harmonica", "D5:2 A5:2", 0.4, 0.4, {"ring": 1.0})],
        "stone": [sfx("gurgle", {"dur": bar, "rate": 1.4, "size": 0.9, "seed": 4}, 0.8,
                      fx=[{"type": "lowpass", "cutoff": 900}])],
        "gate": [sfx("babble", {"dur": bar, "flow": 0.8, "size": 0.6, "seed": 9}, 0.5),
                 seq("wind_chime", "F#6:1 -:1 D6:1 -:1", 0.25, 0.3, {"decay": 1.0})],
        "falls": [sfx("waterfall", {"dur": bar, "height": 2.0, "distance": 3.0, "seed": 1}, 0.9)],
        "mill": [sfx("creak", {"material": "wood", "force": "ramp", "dur": 1.0, "size": 1.6, "seed": 2}, 0.6),
                 sfx("thud", {"body": "box", "size": 1.4, "force": 0.7, "seed": 3}, 0.8, at=beat * 2),
                 sfx("splash", {"size": 0.4, "dur": 0.6, "droplets": 3, "seed": 2}, 0.4, at=beat * 2)],
        "duck": [sfx("animal", {"size": 0.5, "pitch": 420.0, "contour": "fall", "excursion": 0.2, "roughness": 0.7,
                                "vowels": "ae-E", "dur": 0.22, "seed": 1}, 0.7),
                 sfx("animal", {"size": 0.5, "pitch": 400.0, "contour": "fall", "excursion": 0.2, "roughness": 0.7,
                                "vowels": "ae-E", "dur": 0.2, "seed": 2}, 0.6, at=0.3),
                 sfx("drops", {"dur": bar, "rate": 3.0, "jitter": 0.3, "size": 0.6, "seed": 4}, 0.3)],
    }
    # Peak levels that make the sparse loops as loud as the busy ones.
    level = {"leaf": -15, "scrap": -14, "stone": -14, "mill": -14, "duck": -13}
    for kind, layers in floats.items():
        out.append(sound(f"float/{kind}.wav", layers, [MONO, {"type": "reverb", "mix": 0.08, "size": 0.4}, MONO],
                         level.get(kind, -17), duration=bar, loop="fold" if kind in ("petal", "leaf", "scrap", "lantern", "gate", "mill", "duck") else 0.3))
    return out


def cues() -> list[dict]:
    m = [MONO, {"type": "reverb", "mix": 0.08, "size": 0.4}, MONO]
    return [
        # Bumps: mono, placed where they happen.
        sound("bump/bank.wav", [sfx("knock", {"count": 1, "material": "hollow", "size": 0.8, "force": 0.7, "seed": 2}),
                                sfx("splash", {"size": 0.3, "dur": 0.5, "droplets": 3, "seed": 1}, 0.4)], m, -14),
        sound("bump/stone.wav", [sfx("knock", {"count": 1, "material": "wood", "size": 0.6, "force": 0.9, "seed": 4}),
                                 sfx("thud", {"body": "box", "size": 0.5, "force": 0.6, "seed": 1}, 0.6)], m, -14),
        sound("bump/mill.wav", [sfx("thud", {"body": "box", "size": 1.6, "force": 1.0, "seed": 5}),
                                sfx("creak", {"material": "wood", "force": "ramp", "dur": 0.6, "size": 1.4, "seed": 7}, 0.5),
                                sfx("splash", {"size": 0.6, "dur": 0.8, "droplets": 5, "seed": 3}, 0.5)], m, -14),
        sound("bump/duck.wav", [sfx("animal", {"size": 0.5, "pitch": 470.0, "contour": "arch", "excursion": 0.4,
                                               "roughness": 0.8, "vowels": "ae-E", "dur": 0.3, "seed": 6}),
                                sfx("splash", {"size": 0.4, "dur": 0.6, "droplets": 4, "seed": 5}, 0.6)], m, -14),
        sound("bump/falls.wav", [sfx("splash", {"size": 0.8, "dur": 1.0, "droplets": 7, "seed": 8}),
                                 sfx("knock", {"count": 1, "material": "wood", "size": 0.7, "force": 0.8, "seed": 9}, 0.7)], m, -14),
        # Catches: the note comes from the song; these add on top.
        sound("catch/clean.wav", [seq("glockenspiel", "D7:1", 0.4, 0.4)], m, -20, bpm=BPM, beats=1, tail=0.4),
        sound("catch/scrap.wav", [sfx("paper", {"kind": "page", "dur": 0.3, "weight": 0.3, "seed": 1}, 0.8)], m, -18),
        sound("catch/trinket.wav", [sfx("magic", {"dur": 0.8, "density": 10, "freq": "D6"}, 0.6),
                                    seq("celesta", "D6:.25 F#6:.25 A6:1", 0.4, 0.45)], m, -16, bpm=BPM, beats=1.5, tail=0.6),
        sound("through/gate.wav", [seq("wind_chime", "D6:.25 F#6:.25 A6:1", 0.4, 0.45, {"decay": 1.2}),
                                   sfx("whoosh", {"dur": 0.4, "low": 400, "high": 2500, "direction": "up"}, 0.3)],
              m, -16, bpm=BPM, beats=1.5, tail=0.6),
        sound("through/falls.wav", [sfx("splash", {"size": 0.7, "dur": 1.0, "droplets": 6, "seed": 4}, 0.7),
                                    sfx("whoosh", {"dur": 0.7, "low": 300, "high": 3000, "direction": "down"}, 0.5),
                                    seq("hand_chime", "A5:.5 D6:1", 0.35, 0.4, at=0.2)], m, -15, bpm=BPM, beats=1.5, tail=0.6),
        # Score cues: stereo, not placed.
        sound("score/multiplier.wav", [seq("glockenspiel", "A6:.25 D7:.25 F#7:.75", 0.4, 0.45, fx=[pan(0.2)])],
              [ROOM], -17, bpm=BPM, beats=1.25, tail=0.6),
        sound("score/best.wav", [seq("celesta", "D5:.25 F#5:.25 A5:.25 D6:1", 0.45, 0.45, fx=[pan(-0.2)]),
                                 seq("hand_chime", "-:.75 D6:1", 0.3, 0.4, fx=[pan(0.2)])], [ROOM], -16, bpm=BPM, beats=1.75, tail=0.8),
        sound("score/ribbon.wav", [seq("handbell", "A5:.5 D6:.5 F#6:1", 0.4, 0.45, fx=[pan(0.2)]),
                                   seq("harp", "D4:.25 A4:.25 D5:.25 F#5:.25 A5:1", 0.3, 0.4, fx=[pan(-0.3)])],
              [ROOM], -16, bpm=BPM, beats=2, tail=0.8),
        sound("move/launch.wav", [sfx("splash", {"size": 0.3, "dur": 0.6, "droplets": 3, "seed": 11}, 0.6),
                                  sfx("paper", {"kind": "rustle", "dur": 0.3, "weight": 0.5, "seed": 12}, 0.5)], [ROOM], -17),
        sound("move/mouth.wav", [seq("hand_chime", "D5:.25 F#5:.25 A5:.25 D6:.25 F#6:1.5", 0.4, 0.5, {"decay": 1.6}),
                                 sfx("babble", {"dur": 2.5, "flow": 0.4, "size": 1.6, "seed": 13}, 0.25)],
              [ROOM], -16, bpm=BPM, beats=2.5, tail=1.0),
        sound("move/sank.wav", [sfx("bubbles", {"dur": 1.8, "rate": 14, "size": 2.5, "rise": 1.0, "seed": 14}, 0.6),
                                sfx("whoosh", {"dur": 1.0, "low": 200, "high": 1200, "direction": "down"}, 0.3),
                                seq("kalimba", "A4:.5 F#4:.5 D4:1", 0.35, 0.4)], [ROOM], -17, bpm=BPM, beats=2, tail=0.8),
        sound("move/current_in.wav", [sfx("whoosh", {"dur": 0.35, "low": 500, "high": 3500, "direction": "up"}, 0.6)],
              [ROOM], -22),
        sound("move/current_out.wav", [sfx("whoosh", {"dur": 0.35, "low": 250, "high": 1400, "direction": "down"}, 0.5,
                                           fx=[{"type": "lowpass", "cutoff": 1200}])], [ROOM], -24),
        sound("shelf/music_box.wav", [seq("music_box",
                                          "D6:.5 F#6:.5 A6:1 F#6:.5 E6:.5 D6:1 | B5:.5 D6:.5 G6:1 F#6:.5 E6:.5 A6:1 | "
                                          "D6:.5 F#6:.5 A6:1 B6:.5 A6:.5 F#6:1 | E6:.5 C#6:.5 E6:1 D6:2",
                                          0.5, 0.45)], [ROOM], -17, bpm=88, beats=16, tail=1.5),
        sound("shelf/fold.wav", [sfx("paper", {"kind": "page", "dur": 0.4, "weight": 0.6, "seed": 21}, 0.7),
                                 sfx("paper", {"kind": "page", "dur": 0.35, "weight": 0.6, "seed": 22}, 0.6, at=0.35),
                                 sfx("crumple", {"kind": "paper", "energy": 0.15, "density": 0.6, "size": 0.3, "dur": 0.3,
                                                 "seed": 23}, 0.4, at=0.75)], [ROOM], -17),
    ]


def menu() -> list[dict]:
    end = [{"type": "mono"}, {"type": "fade", "out": 0.04}]
    return [
        {"genny": "blal1", "out": "menu/move.wav", "_about": "Menu move: one small water drop.", "duration": 0.18,
         "layers": [sfx("bubble", {"freq": 700}, 0.5)],
         "fx": [{"type": "lowpass", "cutoff": 2500}] + end, "normalize": -9, "max_loudness": -28},
        {"genny": "blal1", "out": "menu/select.wav", "_about": "Menu select: a splash of a drop and a harp note.",
         "duration": 0.45,
         "layers": [sfx("bubble", {"freq": 1000}, 0.35), seq("harp", "D5:1", 0.5, 0.45, at=0.03)],
         "fx": [{"type": "lowpass", "cutoff": 4500}, {"type": "reverb", "mix": 0.12, "size": 0.3}] + end,
         "bpm": 160, "normalize": -6, "max_loudness": -22},
        {"genny": "blal1", "out": "menu/back.wav", "_about": "Menu back: a soft paper fold.", "duration": 0.3,
         "layers": [sfx("paper", {"kind": "page", "dur": 0.25, "weight": 0.4, "seed": 31}, 0.6)],
         "fx": [{"type": "highpass", "cutoff": 200}, {"type": "lowpass", "cutoff": 3000}] + end,
         "normalize": -9, "max_loudness": -26},
    ]


def main() -> None:
    spec = {
        "genny": "blal1",
        "_about": "Little Rapids. Written by tools/rapids_music.py: edit that, not this file.",
        "sounds": [title_music(), pause_music(), mouth_music(), sank_music()] + loops() + cues() + menu() + song(),
    }
    SPEC.write_text(json.dumps(spec, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {len(spec['sounds'])} sounds to {SPEC.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
