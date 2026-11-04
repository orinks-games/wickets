"""Write the lobby's newer pieces into `sound-specs/parlor.json`.

The launcher's lobby plays a different set of pieces at each part of the
day (see `crates/wickets/src/music.rs`). The first three, the lounge, the
bossa and the rag, were made with Oriol's genny and stay as shipped. This
script builds the rest for blal1's genny from short chord and tune tables,
in the same way as the story mode's soundtrack: a light room, instruments
spread across the stereo field (accompaniment a little left, tunes a
little right, bass and pads in the middle) and no heavy effects. It
replaces each piece it makes, by its `out`, keeps every other sound where
it is, and puts new pieces after the rag. Then render with:

    python tools/lobby_music.py
    python tools/render_sounds.py parlor

Pieces (all quiet under speech, looping cleanly with "loop": "fold"):

- lobby_morning: dawn and morning. Fingerpicked guitar, a flute tune and
  a chiptune pulse in the middle eight, in D.
- lobby_afternoon: the afternoon. A tea-room piece on pizzicato strings,
  oboe and harp, in G.
- lobby_evening: evening and night. A slow ballad on felt piano and sax
  with brushes, in E flat.
- lobby_small_hours: 1 to 5 in the morning. Rolled felt piano chords and a
  cello, in A minor, very sparse.
- lobby_swing: any time but the small hours. A light swing on rhythm
  guitar, chiptune pulse and vibraphone, in F.
- lobby_waltz: any time but the small hours. A musette waltz on
  accordion and mandolin, in A minor.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "parlor.json"
AFTER = "music/lobby_rag.wav"


def bars(*items: str) -> str:
    return " | ".join(items)


# Where each instrument sits, -1 left to 1 right: accompaniment a little
# left, tunes a little right, bass, pads and strings in the middle. The
# same placing as the story music.
PAN = {
    "harp": -0.35, "guitar": -0.35, "muted_guitar": -0.35, "felt_piano": -0.2, "piano": -0.2,
    "pizzicato": -0.3,
    "flute": 0.35, "chip": 0.3, "oboe": 0.3, "sax": 0.3, "vibraphone": 0.3, "accordion": 0.3,
    "mandolin": 0.35, "marimba": 0.3, "cello": 0.15,
    "brush": 0.2, "shaker": 0.3,
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


def piece(out: str, bpm: int, beats: int, normalize: float, about: str,
          sections: list[dict], fx: list[dict]) -> dict:
    return {
        "out": f"music/{out}.wav",
        "genny": "blal1",
        "bpm": bpm,
        "beats": beats,
        "loop": "fold",
        "normalize": normalize,
        "_arrangement": about,
        "layers": sections,
        "fx": fx,
    }


# Light reverb only: the space comes from the panning.
ROOM = [{"type": "reverb", "mix": 0.08, "size": 0.45}, {"type": "compressor", "threshold": -18, "ratio": 2}]
SOFT = [{"type": "reverb", "mix": 0.12, "size": 0.6}, {"type": "lowpass", "cutoff": 6000}]


def pick(chords: list[list[str]]) -> str:
    """Fingerpicked: bass, then the upper notes, twice a bar."""
    return bars(*(" ".join(f"{n}:.5" for n in [c[0], c[2], c[3], c[2], c[1], c[2], c[3], c[2]])
                  for c in chords))


def root_fifth(chords: list[list[str]]) -> str:
    """The bass on one and three: the root, then the chord's next note."""
    return bars(*(f"{c[0]}:2 {c[1]}:2" for c in chords))


def comp(chords: list[str]) -> str:
    """A soft piano comp: the chord on one and on the and of two."""
    return bars(*(f"{c}:1.5 {c}:2.5" for c in chords))


def rolled(chords: list[list[str]]) -> str:
    """Each chord's notes in turn, one a beat, held under the next."""
    return bars(*(" ".join(f"{n}:1" for n in c) for c in chords))


def oom(chords: list[tuple[str, str]]) -> str:
    """The oom of a waltz: the bass on one."""
    return bars(*(f"{b}:1 -:2" for b, _ in chords))


def pah(chords: list[tuple[str, str]]) -> str:
    """The pah-pah of a waltz: the chord on two and three."""
    return bars(*(f"-:1 {c}:1 {c}:1" for _, c in chords))


def held(chords: list[str], beats: float) -> str:
    return bars(*(f"{c}:{beats:g}" for c in chords))


# --- lobby_morning: D major, 128 beats -------------------------------------

M_D = ["D3", "A3", "D4", "F#4"]
M_G = ["G2", "D3", "G3", "B3"]
M_BM = ["B2", "F#3", "B3", "D4"]
M_A = ["A2", "E3", "A3", "C#4"]
M_EM = ["E2", "B2", "E3", "G3"]
M_FSM = ["F#2", "C#3", "F#3", "A3"]
MORN_A = [M_D, M_G, M_BM, M_A, M_D, M_G, M_EM, M_A]
MORN_B = [M_G, M_A, M_FSM, M_BM, M_G, M_A, M_D, M_A]
MORN_TUNE = bars("A4:1 D5:1 F#5:1.5 E5:.5", "D5:1 B4:1 G4:2", "F#4:1 B4:1 D5:1.5 C#5:.5", "E5:2 C#5:1 A4:1",
                 "A4:1 D5:1 F#5:1 A5:1", "B5:1.5 A5:.5 G5:1 D5:1", "E5:1 G5:1 F#5:1 E5:1", "E5:3 -:1")
MORN_B_TUNE = bars("B4:1.5 D5:.5 G5:2", "A5:1 G5:1 E5:2", "F#5:1.5 E5:.5 C#5:2", "D5:3 -:1",
                   "B4:1 D5:1 G5:1 B5:1", "A5:1.5 G5:.5 E5:1 C#5:1", "D5:1 F#5:1 A5:2", "G5:1 F#5:1 E5:2")


def lobby_morning() -> dict:
    tick = pattern("shaker", "x-x-x-x-x-x-x-x-", 8, 0.07)
    return piece(
        "lobby_morning", 104, 128, -13.4,
        "128 beats in D: A on fingerpicked guitar and bass with the tune on marimba; A2 the tune on flute "
        "with a shaker; B on chiptune pulse over G, A, F sharp minor and B minor; A3 the flute again.",
        [
            group("A", 0,
                  seq("guitar", pick(MORN_A), 0.42, 0.5),
                  seq("upright_bass", root_fifth(MORN_A), 0.5),
                  seq("marimba", MORN_TUNE, 0.3, 0.5)),
            group("A2", 32,
                  seq("guitar", pick(MORN_A), 0.38, 0.5),
                  seq("upright_bass", root_fifth(MORN_A), 0.5),
                  dict(tick),
                  seq("flute", MORN_TUNE, 0.26, 0.45)),
            group("B", 64,
                  seq("guitar", pick(MORN_B), 0.38, 0.5),
                  seq("upright_bass", root_fifth(MORN_B), 0.5),
                  dict(tick),
                  seq("chip", MORN_B_TUNE, 0.2, 0.45)),
            group("A3", 96,
                  seq("guitar", pick(MORN_A), 0.38, 0.5),
                  seq("upright_bass", root_fifth(MORN_A), 0.5),
                  seq("flute", MORN_TUNE, 0.26, 0.45),
                  seq("marimba", MORN_TUNE, 0.14, 0.4, transpose=-12, pan=-0.1)),
        ],
        ROOM,
    )


# --- lobby_afternoon: G major, 128 beats -----------------------------------

def tea(chords: list[tuple[str, str, str]]) -> tuple[str, str]:
    """A tea-room two-step: the bass on one and three, the chord on two and
    four. Each bar is (bass, chord) or, split in half, (bass, chord, bass2/chord2)."""
    lo, hi = [], []
    for bar in chords:
        if len(bar) == 2:
            b, c = bar
            lo.append(f"{b}:1 -:1 {b}:1 -:1")
            hi.append(f"-:1 {c}:1 -:1 {c}:1")
        else:
            b, c, rest = bar
            b2, c2 = rest.split("/")
            lo.append(f"{b}:1 -:1 {b2}:1 -:1")
            hi.append(f"-:1 {c}:1 -:1 {c2}:1")
    return bars(*lo), bars(*hi)


T_G = ("G2", "[B3,D4,G4]")
T_EM = ("E2", "[B3,E4,G4]")
T_AM = ("A2", "[A3,C4,E4]")
T_D7 = ("D2", "[A3,C4,F#4]")
T_C = ("C3", "[G3,C4,E4]")
T_D = ("D2", "[A3,D4,F#4]")
T_BM = ("B2", "[B3,D4,F#4]")
TEA_A = [T_G, T_EM, T_AM, T_D7, T_G, T_EM, ("A2", "[A3,C4,E4]", "D2/[A3,C4,F#4]"), T_G]
TEA_B = [T_C, T_D, T_BM, T_EM, T_AM, T_D, T_G, T_D7]
TEA_TUNE = bars("D5:1 B4:1 G4:1 B4:1", "E5:2 D5:1 B4:1", "C5:1 E5:1 A5:1 G5:1", "F#5:2 D5:2",
                "D5:1 B4:1 G5:1.5 F#5:.5", "E5:1 G5:1 B4:2", "C5:1 E5:1 D5:1 C5:1", "B4:3 -:1")
TEA_B_TUNE = bars("E5:1.5 D5:.5 C5:1 G4:1", "A4:1 D5:1 F#5:2", "F#5:1 D5:1 B4:2", "G5:1.5 F#5:.5 E5:2",
                  "C5:1 E5:1 A5:1.5 G5:.5", "F#5:1 A5:1 D5:2", "B4:1 D5:1 G5:2", "F#5:1 E5:1 D5:1 C5:1")


def harp_arp(chords: list[tuple]) -> str:
    """The chord's notes rising in eighths, from the bass up, twice a bar."""
    out = []
    for bar in chords:
        b, c = bar[0], bar[1]
        notes = [b] + c.strip("[]").split(",")
        out.append(" ".join(f"{n}:.5" for n in notes * 2))
    return bars(*out)


def lobby_afternoon() -> dict:
    a_lo, a_hi = tea(TEA_A)
    b_lo, b_hi = tea(TEA_B)
    return piece(
        "lobby_afternoon", 100, 128, -14.6,
        "128 beats in G, a tea-room two-step: A on pizzicato and bass with the tune on harp; A2 the tune "
        "on oboe over soft strings; B on chiptune pulse through C, D, B minor and E minor; A3 the oboe over "
        "rising harp.",
        [
            group("A", 0,
                  seq("upright_bass", a_lo, 0.5),
                  seq("pizzicato", a_hi, 0.32, 0.5),
                  seq("harp", TEA_TUNE, 0.34, 0.5, pan=0.3)),
            group("A2", 32,
                  seq("upright_bass", a_lo, 0.5),
                  seq("pizzicato", a_hi, 0.3, 0.5),
                  seq("strings", held([c[1] for c in TEA_A], 4), 0.12, 0.35),
                  seq("oboe", TEA_TUNE, 0.24, 0.45)),
            group("B", 64,
                  seq("upright_bass", b_lo, 0.5),
                  seq("pizzicato", b_hi, 0.3, 0.5),
                  seq("chip", TEA_B_TUNE, 0.2, 0.45)),
            group("A3", 96,
                  seq("upright_bass", a_lo, 0.5),
                  seq("harp", harp_arp(TEA_A), 0.26, 0.45),
                  seq("oboe", TEA_TUNE, 0.24, 0.45)),
        ],
        ROOM,
    )


# --- lobby_evening: a ballad in E flat, 128 beats --------------------------

EV_A = [("Eb2", "[G3,Bb3,D4]"), ("C2", "[G3,Bb3,Eb4]"), ("F2", "[Ab3,C4,Eb4]"), ("Bb1", "[Ab3,D4,F4]"),
        ("G2", "[F3,Bb3,D4]"), ("C2", "[E3,Bb3,D4]"), ("F2", "[Ab3,C4,Eb4]", "Bb1/[Ab3,D4,F4]"),
        ("Eb2", "[G3,Bb3,D4]")]
EV_B = [("Ab1", "[G3,C4,Eb4]"), ("Db2", "[F3,B3,Eb4]"), ("G2", "[G3,Bb3,D4]"), ("C2", "[E3,Bb3,D4]"),
        ("F2", "[Ab3,C4,Eb4]"), ("Bb1", "[Ab3,D4,F4]"), ("Eb2", "[G3,Bb3,D4]"), ("Bb1", "[Ab3,D4,F4]")]
# The bass's second note in each bar: the fifth, or the next root in a split bar.
EV_A_BASS2 = ["Bb1", "G2", "C2", "F2", "D2", "G2", "Bb1", "Bb1"]
EV_B_BASS2 = ["Eb2", "Ab1", "D2", "G2", "C2", "F2", "Bb1", "F2"]
EV_TUNE = bars("G4:1 Bb4:1 D5:2", "C5:1.5 Bb4:.5 G4:2", "Ab4:1 C5:1 Eb5:1.5 D5:.5", "D5:1.5 C5:.5 Ab4:2",
               "Bb4:1 D5:1 F5:2", "E5:1.5 D5:.5 Bb4:2", "C5:1 Ab4:1 D5:1 F4:1", "Eb5:3 -:1")
EV_B_TUNE = bars("Eb4:1 G4:1 C5:2", "B4:1.5 Ab4:.5 F4:2", "Bb4:1 G4:1 D5:2", "E5:1.5 D5:.5 Bb4:1 G4:1",
                 "Ab4:1 C5:1 F5:2", "D5:1 C5:1 Ab4:1 F4:1", "G4:1 Bb4:1 D5:2", "Ab4:2 F4:2")


def ballad_comp(chords: list[tuple]) -> str:
    out = []
    for bar in chords:
        if len(bar) == 2:
            out.append(f"{bar[1]}:1.5 {bar[1]}:2.5")
        else:
            out.append(f"{bar[1]}:2 {bar[2].split('/')[1]}:2")
    return bars(*out)


def ballad_bass(chords: list[tuple], second: list[str]) -> str:
    return bars(*(f"{bar[0]}:2 {n}:2" for bar, n in zip(chords, second)))


def lobby_evening() -> dict:
    brush = pattern("brush", "x---x---x---x---", 8, 0.16, swing=0.2)
    return piece(
        "lobby_evening", 70, 128, -13.5,
        "128 beats in E flat, a slow ballad: A on felt piano, bass and brushes with the tune in the "
        "piano's right hand; A2 the tune on sax; B through A flat, D flat 7 and C7 with soft strings; "
        "A3 the sax again.",
        [
            group("A", 0,
                  seq("felt_piano", ballad_comp(EV_A), 0.4, 0.45),
                  seq("upright_bass", ballad_bass(EV_A, EV_A_BASS2), 0.55),
                  dict(brush),
                  seq("felt_piano", EV_TUNE, 0.32, 0.5, transpose=12, pan=0.25)),
            group("A2", 32,
                  seq("felt_piano", ballad_comp(EV_A), 0.36, 0.45),
                  seq("upright_bass", ballad_bass(EV_A, EV_A_BASS2), 0.55),
                  dict(brush),
                  seq("sax", EV_TUNE, 0.24, 0.45)),
            group("B", 64,
                  seq("felt_piano", ballad_comp(EV_B), 0.36, 0.45),
                  seq("upright_bass", ballad_bass(EV_B, EV_B_BASS2), 0.55),
                  seq("strings", held([b[1] for b in EV_B], 4), 0.12, 0.35),
                  dict(brush),
                  seq("sax", EV_B_TUNE, 0.24, 0.45)),
            group("A3", 96,
                  seq("felt_piano", ballad_comp(EV_A), 0.36, 0.45),
                  seq("upright_bass", ballad_bass(EV_A, EV_A_BASS2), 0.55),
                  dict(brush),
                  seq("sax", EV_TUNE, 0.24, 0.45)),
        ],
        ROOM,
    )


# --- lobby_small_hours: A minor, 64 beats ----------------------------------

SH = [["A2", "E3", "B3", "C4"], ["F2", "C3", "E3", "A3"], ["C3", "G3", "D4", "E4"], ["G2", "D3", "A3", "B3"],
      ["A2", "E3", "B3", "C4"], ["F2", "C3", "E3", "A3"], ["D3", "A3", "C4", "F4"], ["E2", "B2", "E3", "G#3"]]
SH_TUNE = bars("-:1 E5:2 D5:1", "C5:3 -:1", "-:1 G5:2 E5:1", "D5:3 -:1",
               "-:1 C5:1 E5:1 A5:1", "G5:2 E5:2", "F5:1.5 E5:.5 D5:1 C5:1", "B4:3 -:1")


def lobby_small_hours() -> dict:
    return piece(
        "lobby_small_hours", 60, 64, -17.5,
        "64 beats in A minor, for 1 to 5 in the morning: rolled felt piano chords with a sparse tune "
        "high in the piano; then the tune an octave down on cello over soft strings.",
        [
            group("A", 0,
                  seq("felt_piano", rolled(SH), 0.4, 0.4),
                  seq("felt_piano", SH_TUNE, 0.28, 0.4, pan=0.25)),
            group("A2", 32,
                  seq("felt_piano", rolled(SH), 0.36, 0.4),
                  seq("strings", held([f"[{c[1]},{c[2]}]" for c in SH], 4), 0.1, 0.3),
                  seq("cello", SH_TUNE, 0.3, 0.45, transpose=-12)),
        ],
        SOFT,
    )


# --- lobby_swing: F major, 128 beats ---------------------------------------

def chop(chords: list[str]) -> str:
    """Rhythm guitar, four to the bar; "a/b" splits the bar in half."""
    out = []
    for c in chords:
        if "/" in c:
            a, b = c.split("/")
            out.append(f"{a}:1 {a}:1 {b}:1 {b}:1")
        else:
            out.append(f"{c}:1 {c}:1 {c}:1 {c}:1")
    return bars(*out)


S_F6, S_F = "[A3,C4,D4]", "[A3,C4,F4]"
S_D7, S_GM7, S_C7, S_AM7 = "[F#3,A3,C4]", "[F3,Bb3,D4]", "[E3,G3,Bb3]", "[G3,C4,E4]"
S_A7, S_G7 = "[G3,C#4,E4]", "[F3,B3,D4]"
SW_A = [S_F6, S_D7, S_GM7, S_C7, f"{S_AM7}/{S_D7}", f"{S_GM7}/{S_C7}", S_F, S_C7]
SW_B = [S_A7, S_A7, S_D7, S_D7, S_G7, S_G7, S_C7, S_C7]
SW_A_BASS = bars("F2:1 G2:1 A2:1 C3:1", "D3:1 C3:1 A2:1 F#2:1", "G2:1 A2:1 Bb2:1 B2:1", "C3:1 Bb2:1 G2:1 E2:1",
                 "A2:1 C3:1 D3:1 F#2:1", "G2:1 Bb2:1 C3:1 E2:1", "F2:1 A2:1 C3:1 A2:1", "C3:1 Bb2:1 G2:1 E2:1")
SW_B_BASS = bars("A2:1 C#3:1 E3:1 C#3:1", "A2:1 G2:1 E2:1 C#2:1", "D2:1 F#2:1 A2:1 F#2:1", "D2:1 E2:1 F#2:1 A2:1",
                 "G2:1 B2:1 D3:1 B2:1", "G2:1 F2:1 D2:1 B1:1", "C2:1 E2:1 G2:1 E2:1", "C2:1 D2:1 E2:1 G2:1")
# Swung quavers are written long-short, 2/3 and 1/3 of a beat.
L, S = ".667", ".333"
SW_TUNE = bars("A4:1 C5:1 D5:1.5 C5:.5", f"D5:{L} F#5:{S} A5:1 F#5:1 D5:1", "G5:1 F5:1 D5:1 Bb4:1",
               f"C5:{L} E5:{S} G5:1 E5:2", "A4:1 C5:1 F#5:1 D5:1", "G4:1 Bb4:1 E5:1 C5:1",
               "A5:1 F5:1 C5:2", f"-:1 G4:{L} A4:{S} Bb4:{L} B4:{S} C5:1")
SW_B_TUNE = bars("E5:1.5 C#5:.5 A4:2", f"-:1 G4:{L} A4:{S} C#5:{L} E5:{S} G5:1",
                 "F#5:1.5 D5:.5 A4:2", f"-:1 F#4:{L} A4:{S} C5:{L} D5:{S} F#5:1",
                 "D5:1.5 B4:.5 G4:2", f"-:1 F4:{L} G4:{S} B4:{L} D5:{S} F5:1",
                 "E5:1.5 C5:.5 G4:2", "Bb4:1 G4:1 E4:1 -:1")


def lobby_swing() -> dict:
    brush = pattern("brush", "x---x---x---x---", 8, 0.18, swing=0.3)
    return piece(
        "lobby_swing", 132, 128, -8.6,
        "128 beats in F, a light swing: A on rhythm guitar, walking bass and brushes with a chiptune "
        "tune; A2 the tune on piano; B through A7, D7, G7 and C7 on vibraphone; A3 the chiptune again.",
        [
            group("A", 0,
                  seq("muted_guitar", chop(SW_A), 0.36, 0.45),
                  seq("upright_bass", SW_A_BASS, 0.55),
                  dict(brush),
                  seq("chip", SW_TUNE, 0.2, 0.45)),
            group("A2", 32,
                  seq("muted_guitar", chop(SW_A), 0.34, 0.45),
                  seq("upright_bass", SW_A_BASS, 0.55),
                  dict(brush),
                  seq("piano", SW_TUNE, 0.3, 0.5, pan=0.25)),
            group("B", 64,
                  seq("muted_guitar", chop(SW_B), 0.34, 0.45),
                  seq("upright_bass", SW_B_BASS, 0.55),
                  dict(brush),
                  seq("vibraphone", SW_B_TUNE, 0.3, 0.5)),
            group("A3", 96,
                  seq("muted_guitar", chop(SW_A), 0.34, 0.45),
                  seq("upright_bass", SW_A_BASS, 0.55),
                  dict(brush),
                  seq("chip", SW_TUNE, 0.2, 0.45)),
        ],
        ROOM,
    )


# --- lobby_waltz: a musette in A minor, 96 beats of 3 ----------------------

W_AM, W_E7, W_A7 = ("A2", "[A3,C4,E4]"), ("E2", "[G#3,B3,D4]"), ("A2", "[G3,C#4,E4]")
W_DM, W_C, W_G7, W_F = ("D2", "[A3,D4,F4]"), ("C3", "[G3,C4,E4]"), ("G2", "[B3,D4,F4]"), ("F2", "[A3,C4,F4]")
WZ_A = [W_AM, W_E7, W_AM, W_A7, W_DM, W_AM, W_E7, W_AM]
WZ_B = [W_C, W_G7, W_G7, W_C, W_F, W_C, W_G7, W_E7]
WZ_TUNE = bars("A4:1 C5:1 E5:1", "D5:1.5 C5:.5 B4:1", "C5:1 E5:1 A5:1", "G5:2 E5:1",
               "F5:1 E5:1 D5:1", "C5:1 E5:1 A4:1", "B4:1 D5:1 G#4:1", "A4:3")
WZ_B_TUNE = bars("G4:1 C5:1 E5:1", "F5:1.5 E5:.5 D5:1", "B4:1 D5:1 G5:1", "E5:3",
                 "A4:1 C5:1 F5:1", "E5:1.5 D5:.5 C5:1", "D5:1 F5:1 B4:1", "G#4:2 B4:1")


def lobby_waltz() -> dict:
    return piece(
        "lobby_waltz", 150, 96, -11.3,
        "96 beats of 3/4 in A minor, a musette: A the tune on mandolin over guitar and bass; A2 on "
        "accordion; B in C major on accordion; A3 accordion and mandolin together.",
        [
            group("A", 0,
                  seq("guitar", pah(WZ_A), 0.34, 0.45),
                  seq("upright_bass", oom(WZ_A), 0.55),
                  seq("mandolin", WZ_TUNE, 0.3, 0.5)),
            group("A2", 24,
                  seq("guitar", pah(WZ_A), 0.32, 0.45),
                  seq("upright_bass", oom(WZ_A), 0.55),
                  seq("accordion", WZ_TUNE, 0.22, 0.45)),
            group("B", 48,
                  seq("guitar", pah(WZ_B), 0.32, 0.45),
                  seq("upright_bass", oom(WZ_B), 0.55),
                  seq("accordion", WZ_B_TUNE, 0.22, 0.45)),
            group("A3", 72,
                  seq("guitar", pah(WZ_A), 0.32, 0.45),
                  seq("upright_bass", oom(WZ_A), 0.55),
                  seq("accordion", WZ_TUNE, 0.2, 0.45),
                  seq("mandolin", WZ_TUNE, 0.2, 0.45, transpose=12, pan=-0.1)),
        ],
        ROOM,
    )


PIECES = [lobby_morning, lobby_afternoon, lobby_evening, lobby_small_hours, lobby_swing, lobby_waltz]


def check_bars(p: dict) -> None:
    """Every bar of every part fills its time signature, so the parts stay
    together: 3 beats in the waltz, else 4."""
    per_bar = 3 if "3/4" in p["_arrangement"] else 4
    for g in p["layers"]:
        for layer in g["layers"]:
            if layer["type"] != "seq":
                continue
            for i, bar in enumerate(layer["steps"].split("|")):
                total = sum(float(t.rsplit(":", 1)[1]) for t in bar.split())
                if abs(total - per_bar) > 0.01:
                    raise SystemExit(f"{p['out']} {g['_section']} {layer['inst']} bar {i + 1}: "
                                     f"{total:g} beats, not {per_bar}")
            if not re.fullmatch(r"[\w#\[\],:.\- |]+", layer["steps"]):
                raise SystemExit(f"{p['out']} {g['_section']}: odd characters in {layer['inst']}")


def main() -> None:
    spec = json.loads(SPEC.read_text(encoding="utf-8"))
    made = [p() for p in PIECES]
    for p in made:
        check_bars(p)
    have = {s["out"] for s in spec["sounds"]}
    by_out = {p["out"]: p for p in made}
    sounds = []
    for s in spec["sounds"]:
        sounds.append(by_out.get(s["out"], s))
        if s["out"] == AFTER:
            # New pieces go after the rag, in the order above.
            sounds.extend(p for p in made if p["out"] not in have)
    spec["sounds"] = sounds
    SPEC.write_text(json.dumps(spec, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(made)} lobby pieces to {SPEC.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
