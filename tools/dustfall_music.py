#!/usr/bin/env python3
"""Dustfall's live score: the one-bar phrases the conductor plays.

Writes the `music/score/...` and `music/colony/...` sounds into
sound-specs/dustfall.json, leaving every other sound in the spec as it is,
then `python3 tools/render_sounds.py dustfall` renders them on blal1's
genny.

The conductor (`games/dustfall/src/music.rs`) plays one bar at a time in
D minor at 72 beats a minute, four beats a bar, over an eight-bar chord
cycle. Each bar it starts one phrase for every part that is playing, all
on the same sample, and the last bar rings out under the new one. The
parts and when they play:

- bass (cello, centre): from the first turn.
- pad (warm pad, wide): while a raider is at row 4 or nearer.
- pluck (kalimba, a little right): while a raider is at row 2 or nearer.
- danger (a high glass note with a slow pulse, centre): while a charge is
  pending, or while the shield is at 0 with a raider at row 2 or nearer.
  It fits every chord of the cycle.
- heavy (a brass stab, a little left): as a hulk, a mothership or the
  carrier steps.
- run (a marimba run, a little right): as a skitter pair steps.

The colony phase has its own bed at half tempo from the same chords:
bass and pad, the pad dropped while the colony is hungry.

Every phrase is `music/score/<part>-<chord>-<variant>` or
`music/colony/<part>-<chord>`; music.rs names them the same way, and a
test checks each key is rendered. Light room reverb, parts panned apart,
bass and pad in the centre, levelled near the collection's pieces.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "dustfall.json"

BPM = 72
COLONY_BPM = 36
BEATS = 4
TAIL = 2.0
ROOM = [{"type": "reverb", "mix": 0.1, "size": 0.5}]

BED_LUFS = -30.0
PLUCK_LUFS = -29.0
DANGER_LUFS = -31.0
STAB_LUFS = -26.0

# The chord cycle. Must match music.rs.
CYCLE = ["Dm", "Bb", "F", "C", "Dm", "Bb", "Gm", "A"]
CHORDS = sorted(set(CYCLE))

# Chord: bass root, bass fifth, pad voicing, higher voicing, arpeggio notes
# (root, third, fifth, octave) for the pluck and the run.
CHORD = {
    "Dm": ("D2", "A1", "[D3,F3,A3]", "[F3,A3,D4]", ("D4", "F4", "A4", "D5")),
    "Bb": ("Bb1", "F2", "[Bb2,D3,F3]", "[D3,F3,Bb3]", ("Bb3", "D4", "F4", "Bb4")),
    "F": ("F2", "C2", "[F2,A2,C3]", "[A2,C3,F3]", ("F4", "A4", "C5", "F5")),
    "C": ("C2", "G1", "[C3,E3,G3]", "[E3,G3,C4]", ("C4", "E4", "G4", "C5")),
    "Gm": ("G1", "D2", "[G2,Bb2,D3]", "[Bb2,D3,G3]", ("G3", "Bb3", "D4", "G4")),
    "A": ("A1", "E2", "[A2,C#3,E3]", "[C#3,E3,A3]", ("A3", "C#4", "E4", "A4")),
}

# Each part's variants: a phrase pattern with chord slots. Must match the
# variant counts in music.rs.
BASS = ["R:4", "R:2 V:2", "R:1.5 R:0.5 V:1 R:1"]
PAD = ["P:4", "P:2 Q:2"]
PLUCK = ["a:1 b:1 c:1 b:1", "a:0.5 b:0.5 c:0.5 b:0.5 a:0.5 c:0.5 b:1", "c:1.5 b:0.5 a:2"]


def fill(pattern: str, chord: str) -> str:
    root, fifth, voicing, higher, arp = CHORD[chord]
    a, b, c, d = arp
    slots = {"R": root, "V": fifth, "P": voicing, "Q": higher, "a": a, "b": b, "c": c, "d": d}
    out = []
    for tok in pattern.split():
        slot, _, length = tok.partition(":")
        out.append(f"{slots[slot]}:{length}")
    return " ".join(out)


def seq(inst: str, steps: str, gain: float, vel: float = 0.5, pan: float = 0.0,
        params: dict | None = None, width: float | None = None) -> dict:
    layer = {"type": "seq", "inst": inst, "step": 1, "steps": steps, "gain": gain, "vel": vel}
    if params:
        layer["params"] = params
    fx = []
    if pan:
        fx.append({"type": "pan", "pos": pan})
    if width:
        fx.append({"type": "width", "amount": width})
    if fx:
        layer["fx"] = fx
    return layer


def phrase(out: str, about: str, layers: list[dict], lufs: float, bpm: int = BPM,
           beats: float = BEATS, tail: float = TAIL) -> dict:
    return {
        "out": out,
        "_about": about,
        "bpm": bpm,
        "beats": beats,
        "tail": tail,
        "layers": layers,
        "fx": ROOM,
        "loudness": lufs,
    }


def score() -> list[dict]:
    out = []
    for chord in CHORDS:
        for i, pattern in enumerate(BASS):
            out.append(phrase(
                f"music/score/bass-{chord}-{i}.wav",
                f"Score: the bass on {chord}, variant {i}, one bar at {BPM}, centre.",
                [seq("cello", fill(pattern, chord), 0.34, 0.45, params={"attack": 0.25})],
                BED_LUFS))
        for i, pattern in enumerate(PAD):
            out.append(phrase(
                f"music/score/pad-{chord}-{i}.wav",
                f"Score: the pad on {chord}, variant {i}, one bar at {BPM}, wide.",
                [seq("warm_pad", fill(pattern, chord), 0.26, 0.5,
                     params={"attack": 0.6, "release": 1.6, "cutoff": 850}, width=0.5)],
                BED_LUFS))
        for i, pattern in enumerate(PLUCK):
            out.append(phrase(
                f"music/score/pluck-{chord}-{i}.wav",
                f"Score: the plucked line on {chord}, variant {i}, one bar at {BPM}, a little right.",
                [seq("kalimba", fill(pattern, chord), 0.36, 0.5, pan=0.3)],
                PLUCK_LUFS))
        root, _, voicing, _, arp = CHORD[chord]
        out.append(phrase(
            f"music/score/heavy-{chord}.wav",
            f"Score: a brass stab on {chord} as a hulk, a mothership or the carrier steps, a little left.",
            [seq("brass", f"{voicing}:0.75", 0.3, 0.7, pan=-0.25, params={"attack": 0.04}),
             seq("cello", f"{root}:0.75", 0.25, 0.6, params={"attack": 0.05})],
            STAB_LUFS, beats=1, tail=1.5))
        a, b, c, d = arp
        out.append(phrase(
            f"music/score/run-{chord}.wav",
            f"Score: a marimba run up {chord} as a skitter pair steps, a little right.",
            [seq("marimba", f"{a}:0.25 {b}:0.25 {c}:0.25 {d}:0.25", 0.34, 0.55, pan=0.3)],
            STAB_LUFS, beats=1, tail=1.0))
        out.append(phrase(
            f"music/colony/bass-{chord}.wav",
            f"Colony: the bass on {chord}, one bar at {COLONY_BPM}, centre.",
            [seq("cello", f"{root}:4", 0.3, 0.4, params={"attack": 0.6})],
            BED_LUFS, bpm=COLONY_BPM, tail=3.0))
        out.append(phrase(
            f"music/colony/pad-{chord}.wav",
            f"Colony: the pad on {chord}, one bar at {COLONY_BPM}, wide.",
            [seq("warm_pad", f"{voicing}:4", 0.24, 0.45,
                 params={"attack": 1.2, "release": 2.0, "cutoff": 750}, width=0.5)],
            BED_LUFS, bpm=COLONY_BPM, tail=3.0))
    out.append(phrase(
        "music/score/danger.wav",
        "Score: the danger layer, a high glass A held with a slow pulse, while a charge is pending "
        "or the shield is at 0 with a raider close. Fits every chord of the cycle.",
        [seq("glass_pad", "A4:4", 0.16, 0.4, params={"attack": 0.4, "release": 1.2}),
         seq("glass", "A5:0.5 -:0.5 A5:0.5 -:0.5 A5:0.5 -:0.5 A5:0.5 -:0.5", 0.12, 0.35)],
        DANGER_LUFS))
    return out


def main() -> None:
    spec = json.loads(SPEC.read_text(encoding="utf-8"))
    kept = [s for s in spec["sounds"]
            if not s["out"].startswith(("music/score/", "music/colony/"))]
    new = score()
    spec["sounds"] = kept + new
    SPEC.write_text(json.dumps(spec, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {SPEC.relative_to(ROOT)}: {len(kept)} sounds kept, {len(new)} score phrases")


if __name__ == "__main__":
    main()
