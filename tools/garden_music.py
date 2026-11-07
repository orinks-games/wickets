"""Write Rooftop Garden's made-up song phrases into `sound-specs/rooftop-garden.json`.

The garden's own song is four bars in G major (G, E minor, C, D) at 80 beats
a minute, one loop per plant. Those loops were made with Oriol's genny and
stay as shipped. This script adds a library of one-bar phrases for every
part, over five chords (G, E minor, C, D and A minor), with the same
instruments, so the game can string them into a new tune every few verses
(see `games/rooftop-garden/src/song.rs`).

Each phrase is one bar (4 beats, 3 seconds) plus a short ring-out, played
once at the start of its bar. Every part keeps the voice it has in the
garden's own song:

- tomato: kalimba arpeggios
- squash: upright bass
- pumpkin: the squash's lines on a bassoon, an octave up
- bean: plucked strings
- sunflower: the flute tune, made up here from chord tones and steps
- lavender: glockenspiel chimes
- mint: a glass pad
- base: the warm pad under everything

Each sound plays from its own planter, so the phrases are not panned: the
planter places them. The room is light (reverb mix 0.12). Each part's
`normalize` is set so its phrases sit at the loudness of its loop in the
garden's own song (measured with ffmpeg's ebur128). The same seed
always writes the same phrases. It replaces every phrase it made before
(outs under `song/made/`) and keeps every other sound where it is. Then
render with:

    python tools/garden_music.py
    python tools/render_sounds.py rooftop-garden

If you change how many phrases a part has, change `VARIANTS` in
`games/rooftop-garden/src/song.rs` to match; a test checks every phrase the
game can ask for has an Ogg.
"""

from __future__ import annotations

import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "rooftop-garden.json"
PREFIX = "song/made/"
SEED = 2026_10_24

BPM = 80
TAIL = 1.5
ROOM = [{"type": "reverb", "mix": 0.12, "size": 0.5}]

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
SCALE = {0, 2, 4, 6, 7, 9, 11}  # G major's pitch classes, C = 0

# Chord: (file name, root pitch class, chord tones as pitch classes)
CHORDS = [
    ("g", 7, (7, 11, 2)),
    ("em", 4, (4, 7, 11)),
    ("c", 0, (0, 4, 7)),
    ("d", 2, (2, 6, 9)),
    ("am", 9, (9, 0, 4)),
]


def midi(name: str) -> int:
    pc, octave = name[:-1], int(name[-1])
    return NAMES.index(pc) + 12 * (octave + 1)


def note(m: int) -> str:
    return f"{NAMES[m % 12]}{m // 12 - 1}"


def root_in(pc: int, low: str) -> int:
    """The chord's root in the octave starting at `low`."""
    lo = midi(low)
    return lo + (pc - lo) % 12


def tones_from(chord: tuple, low: str, count: int) -> list[int]:
    """Chord tones going up from the first one at or above `low`."""
    out, m = [], midi(low)
    while len(out) < count:
        if m % 12 in chord[2]:
            out.append(m)
        m += 1
    return out


def stacked(chord: tuple, low: str, count: int) -> list[int]:
    """The chord from its root at or above `low`: root, third, fifth, octave..."""
    root = root_in(chord[1], low)
    return [t for t in tones_from(chord, note(root), count)]


def steps(cells: list[tuple[int | None, float]]) -> str:
    return " ".join(f"{'-' if m is None else note(m)}:{d:g}" for m, d in cells)


def by_degree(tones: list[int], pattern: list[tuple[int | None, float]]) -> str:
    return steps([(None if d is None else tones[d], beats) for d, beats in pattern])


# Each part: rhythm patterns of chord degrees (0 root, 1 third, 2 fifth,
# 3 octave...; None a rest). Pattern 0 is the shape of the garden's own song.
BASS = [
    [(0, 3), (2, 1)],
    [(0, 2), (2, 1), (1, 1)],
    [(0, 1.5), (0, 0.5), (2, 2)],
]
BEAN = [
    [(0, 1), (None, 0.5), (2, 0.5), (0, 1), (2, 1)],
    [(0, 0.5), (2, 0.5), (3, 1), (2, 1), (1, 1)],
    [(0, 1), (2, 1), (None, 0.5), (2, 0.5), (3, 1)],
]
TOMATO = [
    [(0, .5), (1, .5), (2, .5), (1, .5), (0, .5), (1, .5), (2, .5), (1, .5)],
    [(0, .5), (2, .5), (3, .5), (2, .5), (1, .5), (2, .5), (3, .5), (2, .5)],
    [(2, .5), (1, .5), (0, .5), (1, .5), (2, .5), (3, .5), (2, .5), (1, .5)],
    [(0, .5), (1, .5), (2, .5), (3, .5), (4, .5), (3, .5), (2, .5), (1, .5)],
]
LAVENDER = [
    [(None, 1), (2, .5), (1, .5), (None, 1), (0, 1)],
    [(None, .5), (0, .5), (1, .5), (2, .5), (None, 1), (3, 1)],
    [(None, 2), (3, .5), (2, .5), (1, 1)],
]

# The flute's rhythms, four beats each. The first is the resting bar that
# ends a tune: one long note.
RESTING = [3, 1]
RHYTHMS = [
    [1.5, .5, 1, 1],
    [2, 1, 1],
    [1, 1, 1, 1],
    [1, .5, .5, 2],
    [.5, .5, 1, 1, 1],
    [1, 1, 2],
]
TUNE_LOW, TUNE_HIGH = midi("D4"), midi("E5")
TUNE_HOME = midi("B4")


def scale_step(m: int, by: int) -> int:
    """`by` steps along G major from `m`."""
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


def flute_phrase(chord: tuple, rng: random.Random, resting: bool) -> str:
    """One bar of tune: chord tones on the beats that land on 1 and 3,
    steps along the scale between, kept near the middle of the flute."""
    if resting:
        tone = nearest_tone(chord, TUNE_HOME + rng.choice([-4, 0, 3]))
        return steps([(tone, 3), (None, 1)])
    rhythm = rng.choice(RHYTHMS)
    m = nearest_tone(chord, TUNE_HOME + rng.randint(-5, 4))
    cells, at = [], 0.0
    for i, beats in enumerate(rhythm):
        if i > 0:
            if at in (0.0, 2.0):
                pull = TUNE_HOME - m
                m = nearest_tone(chord, m + rng.choice([-3, -2, 2, 3]) + (2 if pull > 4 else -2 if pull < -4 else 0))
            else:
                by = rng.choice([-2, -1, -1, 1, 1, 2])
                if m + by * 2 > TUNE_HIGH or m + by * 2 < TUNE_LOW:
                    by = -by
                m = scale_step(m, by)
        cells.append((m, beats))
        at += beats
    return steps(cells)


def phrase(part: str, chord_name: str, n: int, inst: str, steps_text: str, gain: float,
           normalize: float, vel: float = 0.55, params: dict | None = None,
           layer_fx: list | None = None) -> dict:
    layer = {"type": "seq", "inst": inst, "steps": steps_text, "gain": gain, "at": 0, "vel": vel, "step": 1}
    if params:
        layer["params"] = params
    if layer_fx:
        layer["fx"] = layer_fx
    return {
        "genny": "blal1",
        "out": f"{PREFIX}{part}/{chord_name}-{n}.wav",
        "bpm": BPM,
        "beats": 4,
        "tail": TAIL,
        "layers": [layer],
        "fx": ROOM,
        "normalize": normalize,
    }


def phrases() -> list[dict]:
    rng = random.Random(SEED)
    out = []
    for chord in CHORDS:
        name = chord[0]
        for n, pat in enumerate(TOMATO):
            out.append(phrase("tomato", name, n, "kalimba", by_degree(stacked(chord, "C4", 5), pat), 0.4, -15.3))
        for n, pat in enumerate(BASS):
            out.append(phrase("squash", name, n, "upright_bass", by_degree(stacked(chord, "C2", 4), pat), 0.8, -15, vel=0.6))
        for n, pat in enumerate(BASS):
            out.append(phrase("pumpkin", name, n, "bassoon", by_degree(stacked(chord, "C3", 4), pat), 0.7, -17.6, vel=0.5,
                              layer_fx=[{"type": "lowpass", "cutoff": 1400}]))
        for n, pat in enumerate(BEAN):
            out.append(phrase("bean", name, n, "pizzicato", by_degree(stacked(chord, "C3", 4), pat), 0.45, -14.6))
        for n in range(4):
            out.append(phrase("sunflower", name, n, "flute", flute_phrase(chord, rng, n == 0), 0.45, -18.9, vel=0.5,
                              params={"breath": 0.3}))
        for n, pat in enumerate(LAVENDER):
            out.append(phrase("lavender", name, n, "glockenspiel", by_degree(tones_from(chord, "G5", 4), pat), 0.3, -15.5, vel=0.5))
        pad = tones_from(chord, "A3", 3)
        out.append(phrase("mint", name, 0, "glass_pad", f"[{','.join(note(m) for m in pad)}]:4", 0.4, -17.4, vel=0.7,
                          params={"attack": 0.3, "release": 1.2}))
        root = root_in(chord[1], "A2")
        out.append(phrase("base", name, 0, "warm_pad", f"[{note(root)},{note(root + 7)}]:4", 0.45, -18.3, vel=0.7,
                          params={"attack": 0.3, "release": 1.2, "cutoff": 800}))
    return out + toned(out)


# Each plant's other instruments, for seed varieties (tone 1 and tone 2;
# tone 0 is the part's own, above): the instrument, a shift in semitones,
# any params, and a change to `normalize` in dB that brings it level with
# the part's own phrases (measured with ffmpeg's ebur128). They play the same phrases at the same loudness, under
# `song/made/<part>-<tone>/`. Must match `breeding::tone_name`.
TONES = {
    "tomato": [("marimba", 0, None, 0.5), ("music_box", 12, None, -1.3)],
    "squash": [("finger_bass", 0, None, -2.9), ("tuba", 12, None, -5.8)],
    "bean": [("harp", 0, None, -4.0), ("banjo", 0, None, -0.8)],
    "sunflower": [("clarinet", -12, None, 0.3), ("ocarina", 0, None, 1.2)],
    "lavender": [("celesta", -12, None, -2.9), ("vibraphone", -12, None, -2.9)],
    "mint": [("string_machine", 0, {"attack": 0.3, "release": 1.2}, 2.1), ("choir", 0, {"vowel": "o"}, 0.8)],
}


def shifted(steps_text: str, by: int) -> str:
    """Moves every note of a step string `by` semitones."""
    if by == 0:
        return steps_text

    def one(n: str) -> str:
        return n if n == "-" else note(midi(n) + by)

    out = []
    for cell in steps_text.split(" "):
        notes, beats = cell.rsplit(":", 1)
        if notes.startswith("["):
            notes = "[" + ",".join(one(n) for n in notes[1:-1].split(",")) + "]"
        else:
            notes = one(notes)
        out.append(f"{notes}:{beats}")
    return " ".join(out)


def toned(own: list[dict]) -> list[dict]:
    out = []
    for spec in own:
        rest = spec["out"][len(PREFIX):]
        part, file = rest.split("/", 1)
        for tone, (inst, by, params, level) in enumerate(TONES.get(part, []), start=1):
            layer = dict(spec["layers"][0])
            layer["inst"] = inst
            layer["steps"] = shifted(layer["steps"], by)
            layer.pop("params", None)
            layer.pop("fx", None)
            if params:
                layer["params"] = params
            made = dict(spec)
            made["layers"] = [layer]
            made["out"] = f"{PREFIX}{part}-{tone}/{file}"
            made["normalize"] = round(spec["normalize"] + level, 1)
            out.append(made)
    return out


def main() -> None:
    # Edit the text, not the parsed JSON, so every other sound keeps its
    # layout: each phrase is one line, and a run drops the lines it wrote.
    text = SPEC.read_text(encoding="utf-8")
    lines = [line for line in text.splitlines() if f'"out": "{PREFIX}' not in line]
    end = max(i for i, line in enumerate(lines) if line == "  ]")
    lines[end - 1] = lines[end - 1].rstrip(",") + ","
    made = phrases()
    lines[end:end] = [f"    {json.dumps(s)}" + ("," if i < len(made) - 1 else "") for i, s in enumerate(made)]
    SPEC.write_text("\n".join(lines) + "\n", encoding="utf-8")
    json.loads(SPEC.read_text(encoding="utf-8"))
    print(f"wrote {len(made)} phrases to {SPEC.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
