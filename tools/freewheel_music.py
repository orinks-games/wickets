"""Write Freewheel's title music into `sound-specs/freewheel.json`.

Freewheel is a courier's ride through a city in the morning, so its
title is city pop: the sunny, quick 80s city sound that also ran under
the racing and arcade games of the time. The set is chosen for that:

- fm_bass, a tight, rubbery FM bass, in the middle, the engine of it;
- epiano, an FM electric piano, on chords a little left;
- clav, a clavinet chopping the off beats further left;
- string_machine, a 70s string ensemble, wide and soft behind;
- square_lead, a classic game-melody lead, on the tune a little right;
- sax, breathy and husky, taking the bridge's tune a little right;
- brass, synth brass stabs that push into the section turns;
- a kit: kick, snare, clap, hats and an open hat.

Like all of Wicket's newer music it is made for Oriol's genny from short
chord and tune tables, with a light room (reverb mix 0.1) and the parts
panned apart. The piece and its tune are original.

It replaces `music/title` by its `out` and leaves the rest of the spec
as written. Then render with:

    python tools/freewheel_music.py
    python tools/render_sounds.py freewheel

- music/title: 96 beats at 124 a minute, about 46 seconds in D, looping
  cleanly with "loop": "fold" at about -26.5 LUFS, like the other titles.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "freewheel.json"


def bars(*items: str) -> str:
    return " | ".join(items)


# Where each part sits, -1 left to 1 right: accompaniment left, tunes
# right, bass, pad and drums in the middle.
PAN = {
    "epiano": -0.25, "clav": -0.45, "square_lead": 0.3, "sax": 0.3, "brass": 0.15,
    "hihat": 0.2, "openhat": 0.2,
}


def panned(layer: dict, name: str, pan: float | None) -> dict:
    pos = PAN.get(name, 0.0) if pan is None else pan
    if pos:
        layer["fx"] = list(layer.get("fx") or []) + [{"type": "pan", "pos": pos}]
    return layer


def seq(inst: str, steps: str, gain: float, vel: float = 0.6, pan: float | None = None, **extra) -> dict:
    layer = {"type": "seq", "inst": inst, "step": 1, "steps": steps, "gain": gain, "vel": vel}
    layer.update(extra)
    return panned(layer, inst, pan)


def pattern(kind: str, steps: str, n_bars: int, gain: float, **extra) -> dict:
    layer = {"type": "pattern", "kind": kind, "steps": steps, "bars": n_bars, "gain": gain}
    layer.update(extra)
    return panned(layer, kind, None)


def group(name: str, at: float, *layers: dict) -> dict:
    return {"type": "group", "_section": name, "at": at, "layers": list(layers)}


# Light reverb only: the space comes from the panning.
ROOM = [{"type": "reverb", "mix": 0.1, "size": 0.45}, {"type": "compressor", "threshold": -18, "ratio": 2},
        {"type": "lowpass", "cutoff": 9000}]

# --- the changes, in D ---------------------------------------------------------

# Rootless voicings, as city pop keys play them, and each chord's bass
# root and fifth.
CHORD = {
    "Gmaj7": (["B3", "D4", "F#4"], "G1", "D2"),
    "F#m7": (["A3", "C#4", "E4"], "F#1", "C#2"),
    "Em7": (["G3", "B3", "D4"], "E1", "B1"),
    "Em9": (["G3", "B3", "D4", "F#4"], "E1", "B1"),
    "A7sus": (["G3", "D4", "E4"], "A1", "E2"),
    "A7": (["G3", "C#4", "E4"], "A1", "E2"),
    "B7": (["A3", "D#4", "F#4"], "B1", "F#2"),
    "Dmaj7": (["A3", "C#4", "F#4"], "D2", "A2"),
    "Bm7": (["A3", "D4", "F#4"], "B1", "F#2"),
    "F#7": (["A#3", "C#4", "E4"], "F#1", "C#2"),
}
# A bar is a chord, or "a/b" for two chords, two beats each.
A_CHORDS = ["Gmaj7", "F#m7", "Em7", "A7sus/A7", "Gmaj7", "F#m7/B7", "Em7/A7", "Dmaj7"]
B_CHORDS = ["Bm7", "Gmaj7", "Em7", "F#7", "Bm7", "Gmaj7", "Em9", "A7sus"]

# The tune: off-beat starts and a climb each phrase, landing on D.
A_TUNE = bars("-:.5 F#5:.5 A5:.5 B5:1 A5:.5 F#5:1", "E5:.5 F#5:.5 A5:1 C#6:1.5 -:.5",
              "-:.5 D6:.5 C#6:.5 B5:.5 A5:.5 G5:.5 F#5:1", "E5:2 -:.5 A4:.5 C#5:.5 E5:.5",
              "-:.5 F#5:.5 A5:.5 D6:1 C#6:.5 B5:1", "A5:1 F#5:.5 A5:.5 D#6:1.5 -:.5",
              "-:.5 E6:.5 D6:.5 B5:.5 G5:.5 E6:.5 C#6:1", "D6:2.5 -:1.5")
# The bridge's own tune, in B minor, for the sax.
B_TUNE = bars("F#5:1.5 A5:.5 B5:1 D6:1", "D6:1.5 B5:.5 F#5:2",
              "G5:1 B5:1 D6:1 E6:1", "C#6:1.5 A#5:.5 F#5:2",
              "F#5:1.5 A5:.5 B5:1 D6:1", "F#6:1.5 E6:.5 D6:1 B5:1",
              "-:.5 G5:.5 B5:.5 D6:.5 F#6:1 E6:1", "E6:1.5 D6:.5 C#6:1 A5:1")


def split(bar: str) -> tuple[str, str]:
    return tuple(bar.split("/")) if "/" in bar else (bar, bar)


def voiced(name: str, octave: int = 0) -> str:
    notes = CHORD[name][0]
    if octave:
        notes = [n[:-1] + str(int(n[-1]) + octave) for n in notes]
    return f"[{','.join(notes)}]"


def up(note: str) -> str:
    return note[:-1] + str(int(note[-1]) + 1)


def bass(chords: list[str]) -> str:
    """FM bass: the root with a ghost, the octave, then root, fifth and
    octave, two beats for each half bar."""
    out = []
    for bar in chords:
        half = []
        for name in split(bar):
            root, fifth = CHORD[name][1], CHORD[name][2]
            half.append(f"{root}:.75 {root}:.25 -:.5 {up(root)}:.5")
        a, b = split(bar)
        if a == b:
            root, fifth = CHORD[a][1], CHORD[a][2]
            half[1] = f"-:.5 {root}:.5 {fifth}:.5 {up(root)}:.5"
        out.append(" ".join(half))
    return bars(*out)


def keys(chords: list[str]) -> str:
    """Electric piano on one and pushed into the and of two, each half."""
    out = []
    for bar in chords:
        a, b = split(bar)
        if a == b:
            out.append(f"{voiced(a)}:1.5 {voiced(a)}:1 -:1.5")
        else:
            out.append(f"{voiced(a)}:1.5 -:.5 {voiced(b)}:1.5 -:.5")
    return bars(*out)


def chops(chords: list[str]) -> str:
    """Clavinet on every off beat, an octave up."""
    out = []
    for bar in chords:
        a, b = split(bar)
        out.append(" ".join(f"-:.5 {voiced(c, 1)}:.5" for c in (a, a, b, b)))
    return bars(*out)


def strings(chords: list[str]) -> str:
    out = []
    for bar in chords:
        a, b = split(bar)
        out.append(f"{voiced(a)}:4" if a == b else f"{voiced(a)}:2 {voiced(b)}:2")
    return bars(*out)


def stabs(chords: list[str]) -> str:
    """Brass on the downbeat of each phrase and a push into the next."""
    out = []
    for i, bar in enumerate(chords):
        a, b = split(bar)
        if i % 4 == 0:
            out.append(f"{voiced(a, 1)}:.5 -:3.5")
        elif i % 4 == 3:
            out.append(f"-:3 {voiced(b, 1)}:.5 -:.5")
        else:
            out.append("-:4")
    return bars(*out)


KICK = "x-----x-x-------"
SNARE = "----x-------x---"
HATS = "x-x-x-x-x-x-x-x-"
OPEN = "--x---x---x---x-"

EPIANO = {"tremolo": 0.25, "decay": 1.2}
CLAV = {"decay": 0.4}
LEAD = {"vibrato": 0.15}
SAX = {"breath": 0.15}
STRINGS = {"attack": 0.4, "release": 0.8}


def band(chords: list[str], n_bars: int) -> list[dict]:
    return [
        seq("fm_bass", bass(chords), 0.27, 0.6),
        pattern("kick", KICK, n_bars, 0.16),
        pattern("snare", SNARE, n_bars, 0.32),
        pattern("clap", SNARE, n_bars, 0.11),
        pattern("hihat", HATS, n_bars, 0.09),
        pattern("openhat", OPEN, n_bars, 0.06),
        seq("epiano", keys(chords), 0.28, 0.45, params=EPIANO),
        seq("clav", chops(chords), 0.28, 0.4, params=CLAV),
        seq("string_machine", strings(chords), 0.11, 0.4, params=STRINGS,
            fx=[{"type": "width", "amount": 0.6}]),
        seq("brass", stabs(chords), 0.19, 0.55),
    ]


def title() -> dict:
    return {
        "out": "music/title.wav",
        "genny": "oriol",
        "loudness": -26.5,
        "bpm": 124,
        "beats": 96,
        "loop": "fold",
        "_arrangement": (
            "96 beats in D, city pop: FM bass and a kit in the middle, FM electric piano and a clavinet "
            "on the left, a string machine wide behind and brass stabs into each phrase. A the title's "
            "tune on a square lead on the right; B, in B minor, the bridge's tune on sax; A2 the square "
            "lead again with the sax an octave down under it."
        ),
        "layers": [
            group("A", 0, *band(A_CHORDS, 8),
                  seq("square_lead", A_TUNE, 0.44, 0.55, params=LEAD)),
            group("B", 32, *band(B_CHORDS, 8),
                  seq("sax", B_TUNE, 0.38, 0.55, transpose=-12, params=SAX)),
            group("A2", 64, *band(A_CHORDS, 8),
                  seq("square_lead", A_TUNE, 0.4, 0.55, params=LEAD),
                  seq("sax", A_TUNE, 0.22, 0.45, transpose=-12, pan=0.35, params=SAX)),
        ],
        "fx": ROOM,
    }


def check_bars(p: dict) -> None:
    """Every bar of every part fills its four beats, so the parts stay
    together."""
    for g in p["layers"]:
        for layer in g["layers"]:
            if layer["type"] != "seq":
                continue
            for i, bar in enumerate(layer["steps"].split("|")):
                total = sum(float(t.rsplit(":", 1)[1]) for t in bar.split())
                if abs(total - 4) > 0.01:
                    raise SystemExit(f"{p['out']} {g['_section']} {layer['inst']} bar {i + 1}: "
                                     f"{total:g} beats, not 4")
            if not re.fullmatch(r"[\w#\[\],:.\- |]+", layer["steps"]):
                raise SystemExit(f"{p['out']} {g['_section']}: odd characters in {layer['inst']}")


def sound_spans(text: str) -> list[tuple[int, int, dict]]:
    """Where each sound sits in the spec's text, so the rest stays as written."""
    dec = json.JSONDecoder()
    i = text.index("[", text.index('"sounds"')) + 1
    spans = []
    while True:
        while text[i] in " \n\r\t,":
            i += 1
        if text[i] == "]":
            return spans
        s, end = dec.raw_decode(text, i)
        spans.append((i, end, s))
        i = end


def main() -> None:
    made = [title()]
    for p in made:
        check_bars(p)
    new = {p["out"]: p for p in made}
    text = SPEC.read_text(encoding="utf-8")
    spans = sound_spans(text)
    missing = set(new) - {s["out"] for _, _, s in spans}
    if missing:
        raise SystemExit(f"not in the spec: {sorted(missing)}")
    for start, end, s in reversed(spans):
        if s["out"] in new:
            text = text[:start] + json.dumps(new[s["out"]], ensure_ascii=False) + text[end:]
    json.loads(text)
    SPEC.write_text(text, encoding="utf-8")
    print(f"wrote {len(new)} piece to {SPEC.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
