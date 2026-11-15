"""Write Fireside Patience's music into `sound-specs/fireside-patience.json`.

Fireside Patience is a game of cards by the fire on an autumn or winter
evening, so its music is a late-night jazz trio playing softly in the
corner: slow, swung and warm. The set:

- rhodes, a Rhodes electric piano comping the chords, a little left;
- upright_bass, walking, in the middle;
- vibraphone, the tune, a little right;
- sax, the bridge's tune, and long notes under the tune near the win, right;
- strings, a soft section holding the chords wide behind, near the win;
- a kit played with brushes: a brushed snare swinging with its accent on
  two and four, and the hi-hat's foot.

Like all of Wicket's newer music it is made for blal1's genny from short
chord and tune tables, with a light room (reverb mix 0.12) and the parts
panned apart. The pieces and their tunes are original.

- music/title: 96 beats at 76 a minute, about 76 seconds in F: the
  vibraphone's tune, then the sax's bridge, looping with "loop": "fold".
- score/base, score/melody, score/pulse, score/lift: the table's music,
  160 beats at 72 a minute, about 133 seconds, four stems of one piece that
  start together and loop. The base (Rhodes and bass) plays from the deal;
  the tune joins as the first cards reach the foundations; the brushes as they
  fill; the sax and strings near the win. Stems share one
  gain, so together they make the full piece.
- music/pause: 64 beats at 66 a minute, about 58 seconds: the Rhodes, the
  bass and a soft vibraphone tune, with no drums.

Then render with:

    python tools/fireside_music.py
    python tools/render_sounds.py fireside-patience

Pass a spec path to write another copy of the spec, such as the private
tree's.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "fireside-patience.json"

BEATS = 4
SWING = 1 / 3  # the brushes play on swung eighth-note triplets


def bars(*items: str) -> str:
    return " | ".join(items)


# Where each part sits, -1 left to 1 right: accompaniment left, tunes
# right, bass, strings and snare in the middle.
PAN = {"rhodes": -0.3, "vibraphone": 0.3, "sax": 0.35, "hihat": -0.2, "brush": -0.05}


def seq(inst: str, steps: str, gain: float, vel: float = 0.5, pan: float | None = None, **extra) -> dict:
    layer = {"type": "seq", "inst": inst, "step": 1, "steps": steps, "gain": gain, "vel": vel}
    layer.update(extra)
    pos = PAN.get(inst, 0.0) if pan is None else pan
    if pos:
        layer["fx"] = list(layer.get("fx") or []) + [{"type": "pan", "pos": pos}]
    return layer


def pattern(kind: str, steps: str, n_bars: int, gain: float) -> dict:
    layer = {"type": "pattern", "kind": kind, "steps": steps, "bars": n_bars, "gain": gain, "step": SWING}
    if PAN.get(kind):
        layer["fx"] = [{"type": "pan", "pos": PAN[kind]}]
    return layer


def kit(n_bars: int) -> list[dict]:
    """Brushes: a swung brushed snare with its accent on two and four, and
    the hat's foot with it."""
    return [
        pattern("brush", "o--x-oo--x-o", n_bars, 0.3),
        pattern("hihat", "---o-----o--", n_bars, 0.05),
    ]


def group(name: str, at: float, *layers: dict) -> dict:
    return {"type": "group", "_section": name, "at": at, "layers": list(layers)}


# Light reverb only: the space comes from the panning.
ROOM = [{"type": "reverb", "mix": 0.12, "size": 0.5}, {"type": "compressor", "threshold": -18, "ratio": 2},
        {"type": "lowpass", "cutoff": 9000}]

# --- the chords: the Rhodes voicing, the bass's root, third and fifth, and a
# guide tone for the long notes under the tune
CHORD = {
    "Fmaj7": (["A3", "C4", "E4"], ["F2", "A2", "C3"], "A4"),
    "Dm7": (["F3", "A3", "C4"], ["D2", "F2", "A2"], "C5"),
    "Gm7": (["F3", "Bb3", "D4"], ["G2", "Bb2", "D3"], "Bb4"),
    "C7": (["E3", "Bb3", "D4"], ["C2", "E2", "G2"], "Bb4"),
    "Am7": (["G3", "C4", "E4"], ["A2", "C3", "E3"], "G4"),
    "D7": (["F#3", "C4", "E4"], ["D2", "F#2", "A2"], "F#4"),
    "G7": (["F3", "B3", "E4"], ["G2", "B2", "D3"], "F4"),
    "Bbmaj7": (["A3", "D4", "F4"], ["Bb2", "D3", "F3"], "A4"),
    "Bbm6": (["G3", "Db4", "F4"], ["Bb2", "Db3", "F3"], "Db5"),
    "F7": (["A3", "Eb4", "G4"], ["F2", "A2", "C3"], "Eb5"),
    "Eb7": (["G3", "Db4", "F4"], ["Eb2", "G2", "Bb2"], "Db5"),
    "Em7b5": (["G3", "Bb3", "D4"], ["E2", "G2", "Bb2"], "Bb4"),
    "A7": (["G3", "C#4", "E4"], ["A2", "C#3", "E3"], "C#5"),
    "Cm7": (["G3", "Bb3", "Eb4"], ["C2", "Eb2", "G2"], "Eb5"),
}

NAMES = ["C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B"]
STEP = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def midi(note: str) -> int:
    m = re.fullmatch(r"([A-G])([#b]?)(-?\d)", note)
    letter, acc, octave = m.groups()
    return 12 * (int(octave) + 1) + STEP[letter] + {"#": 1, "b": -1, "": 0}[acc]


def name(m: int) -> str:
    return f"{NAMES[m % 12]}{m // 12 - 1}"


LOWEST = 28  # E1, the upright bass's lowest string


def halves(bar: str) -> list[str]:
    return bar.split("/") if "/" in bar else [bar, bar]


def voiced(c: str) -> str:
    return "[" + ",".join(CHORD[c][0]) + "]"


def comp(chords: list[str]) -> str:
    """The Rhodes: soft pushes on the and of one and of three."""
    out = []
    for bar in chords:
        a, b = halves(bar)
        out.append(f"-:.67 {voiced(a)}:1.33 -:.67 {voiced(b)}:1.33")
    return bars(*out)


def pause_comp(chords: list[str]) -> str:
    """The pause's Rhodes: the chord held on one, touched again on three."""
    out = []
    for bar in chords:
        a, b = halves(bar)
        out.append(f"{voiced(a)}:2 {voiced(b)}:2")
    return bars(*out)


def walk(chords: list[str]) -> str:
    """The walking bass: root, third, fifth, then a half step into the next
    bar's root; a bar of two chords walks root and fifth of each."""
    out = []
    for i, bar in enumerate(chords):
        a, b = halves(bar)
        nxt = CHORD[halves(chords[(i + 1) % len(chords)])[0]][1][0]
        if a == b:
            root, third, fifth = (midi(n) for n in CHORD[a][1])
            target = midi(nxt)
            # Climb root, third, fifth toward a higher next root; step down
            # through the fifth below toward a lower one.
            if target >= root:
                two, three = third, fifth
            else:
                two, three = third, max(fifth - 12, LOWEST)
            lead = min((target - 1, target + 1), key=lambda m: abs(m - three))
            out.append(f"{name(root)}:1 {name(two)}:1 {name(three)}:1 {name(lead)}:1")
        else:
            out.append(f"{CHORD[a][1][0]}:1 {CHORD[a][1][2]}:1 {CHORD[b][1][0]}:1 {CHORD[b][1][2]}:1")
    return bars(*out)


def walk_into(chords: list[str], nxt: str) -> str:
    """A section's walk, leading into the chord that follows it."""
    line = walk(chords + [nxt])
    return line[: line.rindex(" | ")]


def two_feel(chords: list[str]) -> str:
    """The pause's bass: the root and the fifth, two beats each."""
    out = []
    for bar in chords:
        a, b = halves(bar)
        out.append(f"{CHORD[a][1][0]}:2 {CHORD[b][1][2]}:2")
    return bars(*out)


def guide(chords: list[str]) -> str:
    out = []
    for bar in chords:
        a, b = halves(bar)
        out.append(f"{CHORD[a][2]}:4" if a == b else f"{CHORD[a][2]}:2 {CHORD[b][2]}:2")
    return bars(*out)


def held(chords: list[str]) -> str:
    out = []
    for bar in chords:
        a, b = halves(bar)
        out.append(f"{voiced(a)}:4" if a == b else f"{voiced(a)}:2 {voiced(b)}:2")
    return bars(*out)


# --- the title -------------------------------------------------------------------
TITLE_A = ["Fmaj7", "Dm7", "Gm7", "C7", "Am7", "Dm7", "Gm7", "C7",
           "Bbmaj7", "Em7b5/A7", "Dm7", "Gm7/C7", "Fmaj7", "Dm7", "Gm7/C7", "Fmaj7"]
TITLE_A_TUNE = bars(
    "-:1 A4:.67 C5:.33 E5:1 D5:1", "C5:2 A4:2", "-:1 Bb4:.67 D5:.33 F5:1 E5:1", "E5:3 -:1",
    "-:1 G4:.67 C5:.33 E5:1.5 G5:.5", "F5:2 D5:2", "-:.67 D5:.33 F5:.67 E5:.33 D5:1 Bb4:1", "C5:4",
    "D5:1.5 C5:.5 A4:1 F4:1", "G4:2 C#5:2", "D5:1 F5:1 A5:1.5 G5:.5", "F5:2 E5:2",
    "-:1 A4:.67 C5:.33 E5:1 D5:1", "C5:1 A4:1 F5:2", "E5:1.5 D5:.5 Bb4:1 G4:1", "F4:4")

BRIDGE = ["Cm7", "F7", "Bbmaj7", "Bbm6", "Am7", "D7", "G7", "Gm7/C7"]
BRIDGE_TUNE = bars(
    "G4:1.5 Bb4:.5 Eb5:1 D5:1", "C5:2 A4:2", "-:1 F4:.67 A4:.33 D5:1 C5:1", "Db5:3 -:1",
    "E5:1.5 D5:.5 C5:1 A4:1", "F#4:1 A4:1 C5:1 D5:1", "B4:1.5 D5:.5 F5:2", "E5:1 D5:1 C5:1 -:1")

# --- the table -------------------------------------------------------------------
TABLE_A = ["Fmaj7", "Dm7", "Gm7", "C7", "Am7", "D7", "Gm7", "C7",
           "Fmaj7", "F7", "Bbmaj7", "Bbm6", "Am7", "D7", "Gm7/C7", "Fmaj7"]
TABLE_A_TUNE = bars(
    "A4:1.5 C5:.5 E5:2", "D5:.67 C5:.33 A4:1 F4:2", "-:1 Bb4:.67 D5:.33 F5:1 D5:1", "E5:3 -:1",
    "-:.67 G4:.33 C5:.67 E5:.33 G5:2", "F#5:1.5 E5:.5 C5:2", "Bb4:.67 D5:.33 F5:.67 A5:.33 G5:1 F5:1", "E5:4",
    "-:1 A4:.67 C5:.33 F5:1 E5:1", "Eb5:2 C5:2", "D5:1.5 F5:.5 A5:2", "G5:1 F5:1 Db5:2",
    "C5:1.5 E5:.5 G5:1 E5:1", "F#5:1 A5:1 C6:1 A5:1", "G5:1 F5:1 E5:1 Bb4:1", "A4:3 -:1")
TABLE_A2 = ["Fmaj7", "Dm7", "Gm7", "C7", "Am7", "D7", "Gm7", "C7",
            "Fmaj7", "F7", "Bbmaj7", "Eb7", "Am7", "D7", "Gm7/C7", "Fmaj7"]
TABLE_A2_TUNE = bars(
    "A4:1.5 C5:.5 E5:2", "D5:.67 C5:.33 A4:1 F4:2", "-:1 Bb4:.67 D5:.33 F5:1 D5:1", "E5:2 G5:1 E5:1",
    "-:.67 G4:.33 C5:.67 E5:.33 G5:2", "F#5:1.5 E5:.5 C5:2", "Bb4:.67 D5:.33 F5:.67 A5:.33 G5:1 F5:1", "E5:2 -:1 C5:1",
    "F5:1.5 E5:.5 C5:1 A4:1", "Eb5:1 C5:1 A4:1 F4:1", "D5:2 F5:1 A5:1", "G5:1.5 F5:.5 Db5:2",
    "E5:1 G5:1 C6:2", "A5:1.5 F#5:.5 D5:2", "Bb4:.67 D5:.33 G5:1 E5:.67 C5:.33 Bb4:1", "F5:4")
TABLE = [("A", TABLE_A, TABLE_A_TUNE), ("B", BRIDGE, BRIDGE_TUNE), ("A2", TABLE_A2, TABLE_A2_TUNE)]

# --- the pause -------------------------------------------------------------------
PAUSE = ["Bbmaj7", "Am7", "Gm7", "C7", "Fmaj7", "Dm7", "Gm7", "C7",
         "Bbmaj7", "Bbm6", "Am7", "D7", "Gm7", "C7", "Fmaj7", "Fmaj7"]
PAUSE_TUNE = bars(
    "D5:2 C5:1 A4:1", "G4:2 E4:2", "F4:1 A4:1 D5:2", "E5:3 -:1",
    "A4:1.5 C5:.5 E5:2", "F5:2 D5:2", "Bb4:1 D5:1 F5:1 A5:1", "G5:3 -:1",
    "F5:2 D5:1 A4:1", "Db5:2 Bb4:2", "C5:1.5 E5:.5 G5:2", "F#5:2 C5:2",
    "Bb4:1 D5:1 F5:2", "E5:2 Bb4:2", "A4:4", "-:4")

RHODES = {"tremolo": 0.2, "decay": 1.4}
SAX = {"breath": 0.15}
STRINGS = {"attack": 0.8, "release": 1.2}


def title() -> dict:
    return {
        "out": "music/title.wav",
        "genny": "blal1",
        "loudness": -26.5,
        "bpm": 76,
        "beats": 96,
        "loop": "fold",
        "_arrangement": (
            "96 beats in F, a late-night jazz trio: Rhodes comping on the left, a walking upright bass, "
            "the vibraphone's tune on the right, brushes. The bridge, from C minor, is the sax's."
        ),
        "layers": [
            group("A", 0,
                  seq("rhodes", comp(TITLE_A), 0.27, 0.4, params=RHODES),
                  seq("upright_bass", walk_into(TITLE_A, BRIDGE[0]), 0.23, 0.5),
                  seq("vibraphone", TITLE_A_TUNE, 0.34, 0.5),
                  *kit(16)),
            group("B", 64,
                  seq("rhodes", comp(BRIDGE), 0.27, 0.4, params=RHODES),
                  seq("upright_bass", walk_into(BRIDGE, "Fmaj7"), 0.23, 0.5),
                  seq("sax", BRIDGE_TUNE, 0.3, 0.5, params=SAX),
                  *kit(8)),
        ],
        "fx": ROOM,
    }


# One gain for every stem, so together they make the full piece at about
# the title's loudness and the base alone sits well under it.
STEM_GAIN = 0.49


def stem_layers(stem: str, section: str, chords: list[str], tune: str, nxt: str) -> list[dict]:
    bridge = section == "B"
    if stem == "base":
        return [seq("rhodes", comp(chords), 0.27, 0.4, params=RHODES),
                seq("upright_bass", walk_into(chords, nxt), 0.23, 0.5)]
    if stem == "melody":
        if bridge:
            return [seq("sax", tune, 0.3, 0.5, params=SAX)]
        return [seq("vibraphone", tune, 0.34, 0.5)]
    if stem == "pulse":
        return kit(len(chords))
    if stem == "lift":
        layers = [seq("strings", held(chords), 0.05, 0.35, params=STRINGS)]
        if not bridge:
            layers.append(seq("sax", guide(chords), 0.13, 0.4, transpose=-12, params=SAX))
        return layers
    raise ValueError(stem)


def score(stem: str) -> dict:
    at, groups = 0, []
    for i, (section, chords, tune) in enumerate(TABLE):
        nxt = TABLE[(i + 1) % len(TABLE)][1][0]
        groups.append(group(section, at, *stem_layers(stem, section, chords, tune, nxt)))
        at += BEATS * len(chords)
    return {
        "out": f"score/{stem}.wav",
        "genny": "blal1",
        "bpm": 72,
        "beats": at,
        "loop": "fold",
        "normalize": None,
        "gain": STEM_GAIN,
        "_arrangement": f"The table's trio, its {stem} stem: A in F, the bridge from C minor, A again with a new ending.",
        "layers": groups,
        "fx": ROOM,
    }


def pause() -> dict:
    return {
        "out": "music/pause.wav",
        "genny": "blal1",
        "loudness": -30,
        "bpm": 66,
        "beats": 64,
        "loop": "fold",
        "_about": "Pause music: the trio's Rhodes and bass, and a soft vibraphone tune, with no drums.",
        "layers": [
            seq("rhodes", pause_comp(PAUSE), 0.24, 0.35, params=RHODES),
            seq("upright_bass", two_feel(PAUSE), 0.2, 0.4),
            seq("vibraphone", PAUSE_TUNE, 0.3, 0.4),
        ],
        "fx": ROOM,
    }


def pieces() -> list[dict]:
    return [title(), *(score(s) for s in ("base", "melody", "pulse", "lift")), pause()]


def check_bars(p: dict) -> None:
    """Every bar of every part fills its four beats, so the parts stay
    together."""
    layers = [l for g in p["layers"] for l in (g["layers"] if g["type"] == "group" else [g])]
    for layer in layers:
        if layer["type"] != "seq":
            continue
        for i, bar in enumerate(layer["steps"].split("|")):
            total = sum(float(t.rsplit(":", 1)[1]) for t in bar.split())
            if abs(total - BEATS) > 0.01:
                raise SystemExit(f"{p['out']} {layer['inst']} bar {i + 1}: {total:g} beats, not {BEATS}")
        if not re.fullmatch(r"[\w#\[\],:.\- |]+", layer["steps"]):
            raise SystemExit(f"{p['out']}: odd characters in {layer['inst']}")


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


def write(spec: Path) -> None:
    made = pieces()
    for p in made:
        check_bars(p)
    new = {p["out"]: p for p in made}
    text = spec.read_text(encoding="utf-8")
    spans = sound_spans(text)
    for start, end, s in reversed(spans):
        if s["out"] in new:
            text = text[:start] + json.dumps(new.pop(s["out"]), ensure_ascii=False) + text[end:]
    if new:
        # Pieces the spec does not have yet go at the end of its list.
        close = text.rindex("]")
        body = text[:close].rstrip()
        add = "".join(",\n  " + json.dumps(p, ensure_ascii=False) for p in new.values())
        text = body + add + "\n " + text[close:]
    json.loads(text)
    spec.write_text(text, encoding="utf-8")
    print(f"wrote {len(made)} pieces to {spec}")


if __name__ == "__main__":
    write(Path(sys.argv[1]) if len(sys.argv) > 1 else SPEC)
