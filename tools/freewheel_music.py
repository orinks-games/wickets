"""Write Freewheel's title music into `sound-specs/freewheel.json`.

Freewheel is a courier bike ride through a city, so its title is a bright
city-pop tune for a sunny morning on the bike: fingerstyle bass and a
light kit in the middle, a palm-muted guitar and an electric piano a
little left, and the tune a little right, whistled the first time, on a
clean electric guitar in the bridge, then whistle and glockenspiel
together. Like all of Wicket's newer music it is made for blal1's genny
from short chord and tune tables, with a light room (reverb mix 0.1).
The piece is original.

It replaces `music/title` by its `out` and leaves the rest of the spec
as written. Then render with:

    python tools/freewheel_music.py
    python tools/render_sounds.py freewheel

- music/title: 96 beats at 132 a minute, about 44 seconds in G, looping
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


# Where each part sits, -1 left to 1 right: accompaniment a little left,
# tunes a little right, bass and drums in the middle.
PAN = {
    "muted_guitar": -0.35, "rhodes": -0.25,
    "whistle": 0.3, "electric_guitar": 0.3, "glockenspiel": 0.4,
    "shaker": 0.15,
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
        {"type": "lowpass", "cutoff": 8000}]

# --- the changes, in G ---------------------------------------------------------

G, BM7, CM7, D = "[B3,D4,G4]", "[A3,D4,F#4]", "[B3,E4,G4]", "[A3,D4,F#4,A4]"
EM7, AM7, C, BM = "[G3,B3,D4,E4]", "[G3,C4,E4]", "[G3,C4,E4,G4]", "[F#3,B3,D4]"
EM, D7, GG = "[G3,B3,E4]", "[F#3,C4,D4]", "[G3,B3,D4,G4]"

A_CHORDS = [G, BM7, CM7, D, EM7, AM7, CM7, D]
B_CHORDS = [C, D, BM, EM, AM7, D, GG, D7]

# Fingerstyle bass: root on one, a push into three, the fifth, a step on.
A_BASS = bars("G2:1.5 G2:.5 D3:1 G2:1", "B1:1.5 B1:.5 F#2:1 A2:1", "C2:1.5 C2:.5 G2:1 E2:1",
              "D2:1.5 D2:.5 A2:1 F#2:1", "E2:1.5 E2:.5 B2:1 E2:1", "A1:1.5 A1:.5 E2:1 G2:1",
              "C2:1.5 C2:.5 G2:1 C2:1", "D2:1 D2:1 E2:1 F#2:1")
B_BASS = bars("C2:1.5 C2:.5 G2:1 E2:1", "D2:1.5 D2:.5 A2:1 F#2:1", "B1:1.5 B1:.5 F#2:1 D2:1",
              "E2:1.5 E2:.5 B2:1 G2:1", "A1:1.5 A1:.5 E2:1 C2:1", "D2:1.5 D2:.5 A2:1 C2:1",
              "G1:1.5 G1:.5 D2:1 B1:1", "D2:1 E2:1 F#2:1 A2:1")

# The title's tune: whistled, leaping up and coasting down.
A_TUNE = bars("D5:.5 G5:.5 B5:1 A5:.5 G5:.5 D5:1", "F#5:1.5 E5:.5 D5:1 A5:1",
              "G5:.5 E5:.5 G5:.5 B5:.5 A5:1 G5:1", "F#5:3 -:1",
              "B5:.5 A5:.5 G5:.5 E5:.5 G5:1 B5:1", "C6:1.5 B5:.5 A5:1 E5:1",
              "G5:.5 A5:.5 B5:.5 G5:.5 E5:1 D5:1", "E5:1 F#5:1 A5:1 -:1")
# The bridge's own tune, for the electric guitar.
B_TUNE = bars("E5:1 G5:.5 E5:.5 C5:2", "F#5:1 A5:.5 F#5:.5 D5:2",
              "D5:.5 F#5:.5 B5:1 A5:1 F#5:1", "G5:3 E5:1",
              "C5:1 E5:1 G5:1 A5:1", "F#5:1.5 E5:.5 D5:1 C5:1",
              "B4:1 D5:1 G5:2", "F#5:1 E5:1 D5:1 C5:1")


def chop(chords: list[str]) -> str:
    """Muted guitar on every off beat."""
    return bars(*(f"-:.5 {c}:.5 -:.5 {c}:.5 -:.5 {c}:.5 -:.5 {c}:.5" for c in chords))


def keys(chords: list[str]) -> str:
    """Electric piano on one and pushed into three."""
    return bars(*(f"{c}:1.5 {c}:2.5" for c in chords))


KICK = "x-------x-x-----"
SNARE = "----x-------x---"
HATS = "x-x-x-x-x-x-x-x-"
SHAKE = "-x-x-x-x-x-x-x-x"

RHODES = {"tremolo": 0.15, "decay": 1.2}
GUITAR = {"tone": 3200}


def rhythm(chords: list[str], bass: str, n_bars: int, hats: float = 0.07) -> list[dict]:
    return [
        seq("finger_bass", bass, 0.5, 0.55),
        pattern("kick", KICK, n_bars, 0.32),
        pattern("snare", SNARE, n_bars, 0.16),
        pattern("hihat", HATS, n_bars, hats),
        pattern("shaker", SHAKE, n_bars, 0.06),
        seq("muted_guitar", chop(chords), 0.2, 0.4),
        seq("rhodes", keys(chords), 0.16, 0.4, params=RHODES),
    ]


def title() -> dict:
    return {
        "out": "music/title.wav",
        "genny": "blal1",
        "loudness": -26.5,
        "bpm": 132,
        "beats": 96,
        "loop": "fold",
        "_arrangement": (
            "96 beats in G, bright city pop: fingerstyle bass, kick, snare, hats and a shaker in the "
            "middle, muted guitar on the off beats and an electric piano on the left. A the title's own "
            "tune whistled on the right; B the bridge's tune on a clean electric guitar; A2 whistle and "
            "glockenspiel together."
        ),
        "layers": [
            group("A", 0, *rhythm(A_CHORDS, A_BASS, 8),
                  seq("whistle", A_TUNE, 0.28, 0.5)),
            group("B", 32, *rhythm(B_CHORDS, B_BASS, 8, hats=0.05),
                  seq("electric_guitar", B_TUNE, 0.24, 0.5, params=GUITAR)),
            group("A2", 64, *rhythm(A_CHORDS, A_BASS, 8),
                  seq("whistle", A_TUNE, 0.26, 0.5),
                  seq("glockenspiel", A_TUNE, 0.1, 0.4, transpose=12)),
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
