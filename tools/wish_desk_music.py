"""Write the Wish Desk's sounds into `sound-specs/winter-wonderland.json`.

The Wish Desk builds its music live (see
`games/winter-wonderland/src/desk_music.rs`). The piece is in F major at 76
beats a minute, one bar (4 beats) at a time. Each bar the game picks a
one-bar phrase for every part that is playing, over the bar's chord:

- pad: a warm pad, wide, in the middle
- keys: music box arpeggios, a little to the left
- bass: cello, in the middle
- tune: a clarinet tune, a little to the right

The room is light (reverb mix 0.12), and the instruments are panned
apart. The desk's cues (steps along the shelf, straw, wrapping paper, the
ribbon, the stamp) are mono, so the game can place them on the shelf.

Only the Wish Desk's sounds (`desk/...`) are written; the rest of the
spec is kept as it is. The same seed always writes the same file. Run,
then render:

    python tools/wish_desk_music.py
    python tools/render_sounds.py winter-wonderland

If you change how many phrases a part has, change `Part::variants` in
`desk_music.rs` to match; a test checks every key the game can ask for has
an Ogg.
"""

from __future__ import annotations

import json
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "winter-wonderland.json"
SEED = 2026_11_07

BPM = 76
TAIL = 1.4
ROOM = {"type": "reverb", "mix": 0.12, "size": 0.5}
MONO = {"type": "mono"}
CUE_FX = [MONO, {"type": "reverb", "mix": 0.08, "size": 0.3}, MONO]

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
SCALE = {5, 7, 9, 10, 0, 2, 4}  # F major's pitch classes, C = 0

# Chord: (file name, root pitch class, chord tones). desk_music.rs `Chord`.
CHORDS = [
    ("f", 5, (5, 9, 0)),
    ("dm", 2, (2, 5, 9)),
    ("bb", 10, (10, 2, 5)),
    ("c", 0, (0, 4, 7)),
    ("gm", 7, (7, 10, 2)),
    ("am", 9, (9, 0, 4)),
]

# Phrases per part; desk_music.rs `Part::variants` must match.
VARIANTS = {"pad": 2, "keys": 4, "bass": 3, "tune": 4}
PAN = {"keys": -0.35, "tune": 0.35}


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


def drum(kind: str, params: dict, gain: float = 0.6, at: float = 0, vel: float = 0.5) -> dict:
    return {"type": "drum", "kind": kind, "gain": gain, "at": at, "vel": vel, "params": params}


def sound(out: str, layers: list, fx: list | None = None, normalize: float = -16, about: str = "",
          **extra) -> dict:
    s = {"genny": "blal1", "out": out, "layers": layers, "fx": fx if fx is not None else [ROOM],
         "normalize": normalize}
    if about:
        s["_about"] = about
    s.update(extra)
    return s


def pan(pos: float) -> dict:
    return {"type": "pan", "pos": pos}


# ---------------------------------------------------------------- the song

BASS = [
    [(0, 2), (2, 2)],
    [(0, 3), (1, 1)],
    [(0, 1.5), (2, 0.5), (1, 2)],
]
KEYS = [
    [(0, .5), (1, .5), (2, .5), (3, .5), (2, .5), (1, .5), (2, .5), (1, .5)],
    [(0, 1), (2, .5), (3, .5), (4, 1), (2, 1)],
    [(4, .5), (3, .5), (2, .5), (1, .5), (2, .5), (3, .5), (2, 1)],
    [(0, .5), (2, .5), (1, .5), (3, .5), (2, 1), (None, 1)],
]
RHYTHMS = [
    [2, 1, 1],
    [1.5, .5, 2],
    [1, 1, 2],
    [3, 1],
    [1, .5, .5, 2],
]
TUNE_LOW, TUNE_HIGH, TUNE_HOME = midi("F4"), midi("F5"), midi("C5")


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


def tune_phrase(chord: tuple, rng: random.Random) -> str:
    """One bar of clarinet: a chord tone on each strong beat, scale steps between."""
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
    if part == "pad":
        fx = [{"type": "width", "amount": 0.5}, ROOM]
    return sound(out, layers, fx, normalize, bpm=BPM, beats=4, tail=TAIL)


def song() -> list[dict]:
    rng = random.Random(SEED)
    out = []
    for chord in CHORDS:
        name = chord[0]
        low = tones_from(chord, "C3", 3)
        high = tones_from(chord, "A3", 3)
        out.append(phrase(f"desk/song/pad/{name}_0.wav", [
            seq("warm_pad", f"[{','.join(note(m) for m in low)}]:4", 0.4, 0.6,
                {"attack": 0.6, "release": 1.2, "cutoff": 800})], "pad", -21))
        out.append(phrase(f"desk/song/pad/{name}_1.wav", [
            seq("warm_pad", f"[{','.join(note(m) for m in high)}]:2 [{','.join(note(m) for m in high)}]:2",
                0.4, 0.55, {"attack": 0.4, "release": 1.0, "cutoff": 900})], "pad", -21))
        for n, pat in enumerate(KEYS):
            out.append(phrase(f"desk/song/keys/{name}_{n}.wav",
                              [seq("music_box", by_degree(stacked(chord, "F5", 5), pat), 0.45, 0.45)],
                              "keys", -19))
        for n, pat in enumerate(BASS):
            out.append(phrase(f"desk/song/bass/{name}_{n}.wav",
                              [seq("cello", by_degree(stacked(chord, "F2", 4), pat), 0.6, 0.45)],
                              "bass", -19))
        for n in range(VARIANTS["tune"]):
            out.append(phrase(f"desk/song/tune/{name}_{n}.wav",
                              [seq("clarinet", tune_phrase(chord, rng), 0.45, 0.45)], "tune", -20))
    return out


# ---------------------------------------------------------------- the desk

def cues() -> list[dict]:
    return [
        sound("desk/step.wav", [
            sfx("footstep", {"tone": 140, "ground": "wood", "variation": 2}, 0.5),
            sfx("knock", {"count": 1, "material": "hollow", "size": 0.45, "force": 0.35, "seed": 3}, 0.6, at=0.05),
        ], CUE_FX, -20, "A step along the shelf and a light tap on the box there."),
        sound("desk/end.wav", [
            sfx("thud", {"body": "crate", "size": 0.9, "force": 0.6, "seed": 2}, 0.9),
        ], CUE_FX, -18, "The end of the shelf: a dull wooden bump."),
        sound("desk/empty.wav", [
            sfx("knock", {"count": 2, "pace": 7.0, "material": "wood", "size": 1.4, "force": 0.3, "seed": 5}, 0.8,
                fx=[{"type": "lowpass", "cutoff": 1400}]),
        ], CUE_FX, -21, "An empty place: two soft knocks on the bare shelf."),
        sound("desk/straw.wav", [
            sfx("crumple", {"kind": "bag", "energy": 0.2, "density": 0.9, "size": 0.4, "dur": 0.6, "seed": 7}, 0.7),
            sfx("cloth", {"kind": "rustle", "dur": 0.6, "rate": 6.0, "weight": 0.9, "seed": 8}, 0.6),
        ], CUE_FX, -18, "Only straw: a dry rustle with nothing under it."),
        sound("desk/wrap.wav", [
            sfx("paper", {"kind": "rustle", "dur": 1.2, "weight": 0.35, "seed": 11}, 0.8),
            sfx("crumple", {"kind": "paper", "energy": 0.15, "density": 0.7, "size": 0.35, "dur": 1.1, "seed": 12},
                0.4, at=0.1),
        ], CUE_FX, -18, "Wrapping paper folded round the box while Space is held."),
        sound("desk/snap.wav", [
            drum("snap", {}, 0.6),
            sfx("paper", {"kind": "page", "dur": 0.2, "weight": 0.6, "seed": 13}, 0.4, at=0.03),
        ], CUE_FX, -17, "The ribbon pulled tight."),
        sound("desk/stamp.wav", [
            sfx("thud", {"body": "crate", "size": 0.6, "force": 0.8, "seed": 14}, 0.8),
            sfx("knock", {"count": 1, "material": "wood", "size": 0.6, "force": 0.7, "seed": 15}, 0.5),
            seq("handbell", "-:.5 A5:.5 C6:1", 0.25, 0.4),
        ], CUE_FX, -16, "The stamp comes down, and a small bell for a gift done.", bpm=BPM, beats=2, tail=0.8),
        sound("desk/wrong.wav", [
            sfx("paper", {"kind": "tear", "dur": 0.35, "weight": 0.4, "seed": 16}, 0.6),
            seq("bassoon", "E2:.5 D2:1", 0.4, 0.4, at=0.15),
        ], CUE_FX, -17, "The wrong box: paper pulled off, and a low sigh.", bpm=BPM, beats=1.5, tail=0.6),
        sound("desk/letter.wav", [
            sfx("paper", {"kind": "page", "dur": 0.4, "weight": 0.5, "seed": 17}, 0.7),
            sfx("paper", {"kind": "page", "dur": 0.3, "weight": 0.5, "seed": 18}, 0.5, at=0.25),
        ], CUE_FX, -20, "A letter unfolded."),
        sound("desk/late.wav", [
            sfx("book_drop", {"size": 1.6, "height": 0.4, "seed": 19}, 0.8),
            seq("handbell", "-:.5 F5:.5 A5:.5 C6:1", 0.3, 0.45),
        ], CUE_FX, -16, "Late post: a sack set down on the desk.", bpm=BPM, beats=2.5, tail=0.8),
        sound("desk/clear.wav", [
            seq("music_box", "F5:.5 A5:.5 C6:.5 F6:1.5", 0.5, 0.45, fx=[pan(-0.25)]),
            seq("clarinet", "-:.5 C5:.5 F5:2", 0.3, 0.4, fx=[pan(0.25)]),
        ], [ROOM], -17, "A stack answered.", bpm=BPM, beats=3, tail=1.2),
    ]


def main() -> None:
    # The private overlay keeps a fuller copy of the spec: pass its path to
    # write the same sounds there, in the file's own indent.
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else SPEC
    text = path.read_text(encoding="utf-8")
    indent = 2 if text.startswith('{\n  "') else 1
    spec = json.loads(text)
    kept = [s for s in spec["sounds"] if not s["out"].startswith("desk/")]
    spec["sounds"] = kept + song() + cues()
    path.write_text(json.dumps(spec, indent=indent) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
