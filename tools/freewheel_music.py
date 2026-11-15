"""Write Freewheel's title music into `sound-specs/freewheel.json`.

Freewheel's title is a modern retro track: chiptune voices from genny's
sound-chip models over a modern kit, pad and sub. A NES pulse plays the
tune a little right, a Game Boy pulse runs arpeggios a little left, and
the NES triangle bounces octaves under a sine sub in the middle. The
bridge hands the tune to a Commodore 64 SID saw, and the last part has
the NES pulse with the Game Boy echoing an octave up. Like all of
Wicket's newer music it is made for blal1's genny from short chord and
tune tables, with a light room (reverb mix 0.1) and the parts panned
apart. The piece is original.

It replaces `music/title` by its `out` and leaves the rest of the spec
as written. Then render with:

    python tools/freewheel_music.py
    python tools/render_sounds.py freewheel

- music/title: 96 beats at 140 a minute, about 41 seconds in G, looping
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
# tunes a little right, bass, pad and drums in the middle.
PAN = {
    "gb_pulse": -0.35, "nes_pulse": 0.3, "sid": 0.3, "hat808": 0.15,
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

# --- the changes, in G ---------------------------------------------------------

# Each chord as its notes, low to high, for the arpeggios and the pad.
CHORD = {
    "G": ["G3", "B3", "D4"], "Bm7": ["F#3", "A3", "B3", "D4"], "Cmaj7": ["E3", "G3", "B3", "C4"],
    "D": ["F#3", "A3", "D4"], "Em7": ["E3", "G3", "B3", "D4"], "Am7": ["E3", "G3", "A3", "C4"],
    "C": ["E3", "G3", "C4"], "Bm": ["F#3", "B3", "D4"], "Em": ["E3", "G3", "B3"],
    "D7": ["F#3", "A3", "C4", "D4"],
}
A_CHORDS = ["G", "Bm7", "Cmaj7", "D", "Em7", "Am7", "Cmaj7", "D"]
B_CHORDS = ["C", "D", "Bm", "Em", "Am7", "D", "G", "D7"]
ROOT_NOTE = {"G": "G", "Bm7": "B", "Cmaj7": "C", "D": "D", "Em7": "E", "Am7": "A", "C": "C",
             "Bm": "B", "Em": "E", "D7": "D"}
# The bass octave each root sits in, kept between E1 and D2.
LOW = {"G": "G1", "A": "A1", "B": "B1", "C": "C2", "D": "D2", "E": "E2"}

# The title's tune: leaping up and coasting down.
A_TUNE = bars("D5:.5 G5:.5 B5:1 A5:.5 G5:.5 D5:1", "F#5:1.5 E5:.5 D5:1 A5:1",
              "G5:.5 E5:.5 G5:.5 B5:.5 A5:1 G5:1", "F#5:3 -:1",
              "B5:.5 A5:.5 G5:.5 E5:.5 G5:1 B5:1", "C6:1.5 B5:.5 A5:1 E5:1",
              "G5:.5 A5:.5 B5:.5 G5:.5 E5:1 D5:1", "E5:1 F#5:1 A5:1 -:1")
# The bridge's own tune, for the SID.
B_TUNE = bars("E5:1 G5:.5 E5:.5 C5:2", "F#5:1 A5:.5 F#5:.5 D5:2",
              "D5:.5 F#5:.5 B5:1 A5:1 F#5:1", "G5:3 E5:1",
              "C5:1 E5:1 G5:1 A5:1", "F#5:1.5 E5:.5 D5:1 C5:1",
              "B4:1 D5:1 G5:2", "F#5:1 E5:1 D5:1 C5:1")


def up(note: str, octaves: int) -> str:
    return note[:-1] + str(int(note[-1]) + octaves)


def arps(chords: list[str]) -> str:
    """Sixteenths up and down each chord an octave up, the chip way."""
    out = []
    for name in chords:
        notes = [up(n, 1) for n in CHORD[name]]
        ring = notes + [up(notes[0], 1)] + notes[-2:0:-1] if len(notes) == 3 else notes + notes[-2:0:-1]
        run = [ring[i % len(ring)] for i in range(16)]
        out.append(" ".join(f"{n}:.25" for n in run))
    return bars(*out)


def octaves(chords: list[str]) -> str:
    """Triangle bass bouncing root and octave in eighths."""
    out = []
    for name in chords:
        low = LOW[ROOT_NOTE[name]]
        hi = up(low, 1)
        out.append(" ".join(f"{n}:.5" for n in [low, low, hi, low, low, hi, low, hi]))
    return bars(*out)


def subs(chords: list[str]) -> str:
    """The sine sub on each bar's root, pushed into the and of two."""
    return bars(*(f"{LOW[ROOT_NOTE[c]]}:1.5 {LOW[ROOT_NOTE[c]]}:2.5" for c in chords))


def pads(chords: list[str]) -> str:
    return bars(*(f"[{','.join(CHORD[c])}]:4" for c in chords))


KICK = "x-----x---x-----"
SNARE = "----x-------x---"
HATS = "x-xxx-x-x-xxx-x-"

LEAD = {"duty": 1, "vib": 0.25, "vib_delay": 0.2}
ECHO = {"duty": 2, "vib": 0.15}
ARP = {"duty": 0}
SID = {"wave": "saw", "a": 0, "d": 9, "s": 8, "r": 6, "cutoff": 1800, "res": 6, "vib": 0.2}
PAD = {"attack": 0.6, "release": 1.0, "cutoff": 1400}


def rhythm(chords: list[str], n_bars: int, arp_gain: float = 0.1) -> list[dict]:
    return [
        seq("nes_triangle", octaves(chords), 0.4, 0.6),
        seq("sub", subs(chords), 0.3, 0.5),
        pattern("kick", KICK, n_bars, 0.36),
        pattern("snare808", SNARE, n_bars, 0.2),
        pattern("clap", SNARE, n_bars, 0.08),
        pattern("hat808", HATS, n_bars, 0.06),
        seq("gb_pulse", arps(chords), arp_gain, 0.4, params=ARP),
        seq("pad", pads(chords), 0.1, 0.4, params=PAD, fx=[{"type": "width", "amount": 0.6}]),
    ]


def title() -> dict:
    return {
        "out": "music/title.wav",
        "genny": "blal1",
        "loudness": -26.5,
        "bpm": 140,
        "beats": 96,
        "loop": "fold",
        "_arrangement": (
            "96 beats in G, modern retro: NES triangle octaves over a sine sub, a modern kick, 808 snare "
            "and clap and 808 hats in the middle, Game Boy pulse arpeggios on the left and a wide pad. "
            "A the title's own tune on a NES pulse on the right; B the bridge's tune on a C64 SID saw; A2 "
            "the NES pulse with the Game Boy an octave up."
        ),
        "layers": [
            group("A", 0, *rhythm(A_CHORDS, 8),
                  seq("nes_pulse", A_TUNE, 0.22, 0.6, params=LEAD)),
            group("B", 32, *rhythm(B_CHORDS, 8, arp_gain=0.08),
                  seq("sid", B_TUNE, 0.2, 0.6, params=SID)),
            group("A2", 64, *rhythm(A_CHORDS, 8),
                  seq("nes_pulse", A_TUNE, 0.2, 0.6, params=LEAD),
                  seq("gb_pulse", A_TUNE, 0.08, 0.5, pan=0.4, transpose=12, params=ECHO)),
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
