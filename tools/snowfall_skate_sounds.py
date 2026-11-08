"""Write Snowfall Skate's sounds into `sound-specs/snowfall-skate.json`.

Every sound the skate plays, its menu sounds, and its music, on blal1's
genny. Then render with:

    python tools/snowfall_skate_sounds.py
    python tools/render_sounds.py snowfall-skate

The skate's music is played live (`games/snowfall-skate/src/music.rs`): a
medley of an original waltz and public-domain tunes, strung together a
section at a time, all in G. Every section is rendered as:

- a lead line on each lead instrument, `music/lead/<section>-<lead>`;
- an accompaniment in each of three styles, `music/bed/<section>-<style>`:
  "waltz", upright bass, pizzicato and harp; "hush", held strings and a
  soft bass, for deep snow; "band", tuba and trombones, for the Frost Fair.

The game picks the tune, the lead and the style as it goes, and starts a
lead and an accompaniment together on the same sample. Each file is its
phrase plus a ring-out, so the next section starts over the last one's
tail. The title and pause music are fixed renders.

Light room reverb. Leads a little right, the plucked or bowed
accompaniment a little left, the bass in the middle. The game never names
the tunes.

If you change a section, an instrument or a style, change the lists in
`games/snowfall-skate/src/music.rs` to match; a test checks every key the
medley can ask for has an Ogg.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "snowfall-skate.json"

ROOM = [{"type": "reverb", "mix": 0.1, "size": 0.45}, {"type": "compressor", "threshold": -18, "ratio": 2}]
TAIL = 2.0
LEAD_LUFS = -29
BED_LUFS = -30.5


def sfx(what: str, at: float = 0.0, gain: float = 1.0, fx: list[dict] | None = None, **params) -> dict:
    layer = {"type": "sfx", "kind": what, "params": params, "gain": gain, "at": at}
    if fx:
        layer["fx"] = fx
    return layer


def notes(inst: str, steps: str, gain: float = 0.5, at: float = 0.0, **params) -> dict:
    layer = {"type": "seq", "inst": inst, "steps": steps, "gain": gain, "at": at}
    if params:
        layer["params"] = params
    return layer


def sound(out: str, about: str, layers: list[dict], lufs: float, *,
          duration: float | None = None, fade: float | None = None, loop: float | None = None) -> dict:
    fx = []
    if fade:
        fx.append({"type": "fade", "out": fade})
    fx.append({"type": "mono"})
    if loop:
        fx.append({"type": "loop", "crossfade": loop})
    s = {"out": out, "_about": about, "layers": layers, "loudness": lufs, "fx": fx}
    if duration is not None:
        s["duration"] = duration
    return s


def low(cutoff: float) -> list[dict]:
    return [{"type": "lowpass", "cutoff": cutoff}]


def high(cutoff: float) -> list[dict]:
    return [{"type": "highpass", "cutoff": cutoff}]


# --- the pond ---------------------------------------------------------------

# The lantern notes: G major from G4 up, one for each of light0 to light7.
SCALE = ["G4", "A4", "B4", "C5", "D5", "E5", "F#5", "G5"]


def up_octave(note: str) -> str:
    return note[:-1] + str(int(note[-1]) + 1)


def pond_sounds() -> list[dict]:
    loop = 10.0
    long = loop + 1.0
    out = [
        sound("bed/pond.wav",
              "The pond at night, looped: a still hush, a breath of wind over the ice, the village far off.",
              [sfx("gust", 0.0, 0.4, through="leaves", dur=long, strength=0.12, seed=1),
               sfx("wind", 0.0, 0.3, low(900), dur=long, scale=0.6, seed=2),
               sfx("distant_bell", 6.5, 0.05, seed=3)],
              -36, duration=loop, loop=1.0),
        sound("bed/snowfall.wav",
              "Snow coming down, looped: a soft fine hiss. The game turns it up as the snow lies deeper.",
              [sfx("shake", 0.0, 0.25, high(1500), preset="sandpaper", energy=0.15, rate=1.5, dur=long, seed=4),
               sfx("noise_bed", 0.0, 0.3, low(3000), kind="hiss", dur=long, level=-26, bright=0.3, seed=5)],
              -37, duration=loop, loop=1.0),
        sound("bed/river.wav",
              "The frozen river at the fair, looped: a crowd along the banks, laughter, boots on the ice.",
              [sfx("crowd_murmur", 0.0, 0.5, people=40, mood=0.7, distance=0.7, dur=long, seed=6),
               sfx("laugh", 3.0, 0.08, seed=7),
               sfx("footsteps", 1.0, 0.1, low(1500), ground="snow", gait="walk", steps=8, weight=0.8, seed=8)],
              -34, duration=loop, loop=1.0),
        sound("skate/blades.wav",
              "Your blades on clear ice, looped: long gliding strokes, left and right.",
              [sfx("scrape", 0.0, 0.8, dur=1.0, speed=0.7, force=0.6, roughness=0.3, material="metal",
                   body="rect_plate", size=0.3, bright=0.7, seed=10),
               sfx("scrape", 1.0, 0.7, dur=1.0, speed=0.7, force=0.55, roughness=0.3, material="metal",
                   body="rect_plate", size=0.3, bright=0.7, seed=11)],
              -31, duration=2.0, loop=0.3),
        sound("skate/blades_soft.wav",
              "Your blades in settled snow, looped: the same strokes, muffled and crunching.",
              [sfx("footsteps", 0.0, 0.6, low(2500), ground="snow", gait="walk", steps=2, weight=0.7, seed=12),
               sfx("scrape", 0.0, 0.4, low(1800), dur=2.0, speed=0.5, force=0.4, roughness=0.6, material="metal",
                   body="rect_plate", size=0.3, seed=13)],
              -32, duration=2.0, loop=0.3),
        sound("skate/partner.wav",
              "Your partner's blades, looped: a lighter pair of strokes a little ahead.",
              [sfx("scrape", 0.0, 0.7, high(600), dur=0.9, speed=0.8, force=0.4, roughness=0.25, material="metal",
                   body="rect_plate", size=0.25, bright=0.8, seed=14),
               sfx("scrape", 0.9, 0.6, high(600), dur=0.9, speed=0.8, force=0.35, roughness=0.25,
                   material="metal", body="rect_plate", size=0.25, bright=0.8, seed=15)],
              -32, duration=1.8, loop=0.3),
        sound("skate/carve.wav", "A lean taken up: one carving stroke of the blade.",
              [sfx("scrape", 0.0, 0.9, dur=0.35, speed=1.0, force=0.9, roughness=0.3, material="metal",
                   body="rect_plate", size=0.3, bright=0.8, seed=16)],
              -27, duration=0.45, fade=0.1),
        sound("skate/push.wav", "The push-off: a dig of the blade, a long first glide and a harp lift.",
              [sfx("scrape", 0.0, 1.0, dur=0.2, speed=0.4, force=1.4, roughness=0.6, material="metal",
                   body="rect_plate", size=0.3, seed=17),
               sfx("scrape", 0.2, 0.8, dur=1.0, speed=0.9, force=0.6, roughness=0.3, material="metal",
                   body="rect_plate", size=0.3, bright=0.7, seed=18),
               notes("harp", "G4:0.12 B4:0.12 D5:0.12 G5:0.6", 0.4, 0.3)],
              -22, duration=1.6, fade=0.3),
        sound("skate/stride.wav", "One stride: a quick push of the blade.",
              [sfx("scrape", 0.0, 0.9, dur=0.25, speed=0.6, force=1.0, roughness=0.4, material="metal",
                   body="rect_plate", size=0.3, seed=19)],
              -27, duration=0.35, fade=0.08),
        sound("lantern/flame.wav",
              "A lantern burning ahead, looped: a small flame crackling in its glass.",
              [sfx("fire", 0.0, 0.9, dur=3.0, size=0.15, crackle=10, wet=0.05, wind=0.1, seed=20),
               sfx("flame", 0.0, 0.3, kind="candle", dur=3.0, seed=21)],
              -28, duration=2.5, loop=0.4),
        sound("lantern/wrong.wav",
              "The wrong lantern taken: a flat glassy clink and a low note.",
              [sfx("clink", 0.0, 0.6, seed=22),
               notes("celesta", "[C5,C#5]:0.5", 0.4, 0.02)],
              -24, duration=0.9, fade=0.2),
        sound("lantern/passed.wav", "A lantern gone by: its flame slipping past behind you, and a low falling note.",
              [sfx("whoosh", 0.0, 0.4, dur=0.5, low=200, high=1500, direction="down"),
               notes("celesta", "D5:0.2 G4:0.5", 0.4, 0.05)],
              -28, duration=0.9, fade=0.2),
        sound("patch/drift.wav",
              "A drift ahead, looped: blown snow hissing over a soft heap.",
              [sfx("gust", 0.0, 0.7, through="leaves", dur=3.0, strength=0.3, tone=1.5, seed=30),
               sfx("shake", 0.0, 0.4, preset="sandpaper", energy=0.3, rate=3.0, dur=3.0, seed=31)],
              -29, duration=2.5, loop=0.4),
        sound("patch/thin.wav",
              "Thin ice ahead, looped: the ice ticking, creaking and pinging under strain.",
              [sfx("freeze", 0.0, 0.9, dur=3.0, rate=6.0, creak=0.6, pings=3, seed=32)],
              -28, duration=2.5, loop=0.4),
        sound("patch/drifted.wav", "Into a drift: a soft crunch of snow and a slow slide.",
              [sfx("footsteps", 0.0, 0.8, ground="snow", gait="walk", steps=1, weight=1.3, seed=33),
               sfx("scrape", 0.1, 0.5, low(1500), dur=0.7, speed=0.3, force=0.5, roughness=0.8, material="metal",
                   body="rect_plate", size=0.3, seed=34)],
              -23, duration=1.0, fade=0.25),
        sound("patch/crack.wav", "The ice cracks under you: a sharp crack running away through the ice.",
              [sfx("ice_crack", 0.0, 1.0, dur=1.4, thickness=0.05, distance=40.0, cracks=3, brittle=0.7, seed=35),
               sfx("freeze", 0.05, 0.5, dur=1.2, rate=14.0, creak=0.7, pings=2, seed=36),
               sfx("thud", 0.0, 0.4, low(400), body="box", contact_ms=8, size=1.5, force=0.9, seed=37)],
              -20, duration=1.4, fade=0.3),
        sound("spin/wind.wav",
              "A spin winding, looped while the key is held: blades turning on the spot, faster.",
              [sfx("scrape", 0.0, 0.8, dur=1.5, speed=1.2, force=0.5, roughness=0.2, material="metal",
                   body="rect_plate", size=0.25, bright=0.8, seed=40),
               sfx("whoosh", 0.0, 0.3, dur=1.5, low=500, high=2500, direction="up")],
              -29, duration=1.4, loop=0.3),
        sound("spin/ready.wav", "A spin ready: a rising glassy shimmer. Let go now.",
              [notes("glockenspiel", "D6:0.06 G6:0.06 B6:0.06 D7:0.4", 0.35),
               sfx("magic", 0.0, 0.2, dur=0.6)],
              -24, duration=0.8, fade=0.2),
        sound("spin/land.wav", "A spin landed: a clean scrape out of the turn and a bright ring.",
              [sfx("scrape", 0.0, 0.9, dur=0.3, speed=1.0, force=1.2, roughness=0.2, material="metal",
                   body="rect_plate", size=0.3, bright=0.9, seed=42),
               notes("celesta", "[G5,B5,D6]:0.8", 0.4, 0.1)],
              -21, duration=1.2, fade=0.3),
        sound("spin/wobble.wav", "A wobble: the blades scuffing as you stagger out of the spin.",
              [sfx("scrape", 0.0, 0.8, dur=0.25, speed=0.3, force=1.2, roughness=0.9, material="metal",
                   body="rect_plate", size=0.3, seed=43),
               sfx("scrape", 0.3, 0.7, dur=0.2, speed=0.3, force=1.0, roughness=0.9, material="metal",
                   body="rect_plate", size=0.3, seed=44),
               notes("pizzicato", "D3:0.2 C#3:0.4", 0.4, 0.1)],
              -23, duration=0.9, fade=0.2),
        sound("end/snowed.wav", "The snow has covered the ice: a hush, the wind dropping, and a low harp settling.",
              [sfx("gust", 0.0, 0.6, through="leaves", dur=2.5, strength=0.25, seed=50),
               notes("harp", "D5:0.3 B4:0.3 G4:0.3 D4:1.2", 0.4, 0.3)],
              -22, duration=2.8, fade=0.8),
        sound("end/done.wav", "A course skated to its end: a long glide to a stop and a harp run up.",
              [sfx("scrape", 0.0, 0.7, dur=1.2, speed=0.4, force=0.5, roughness=0.3, material="metal",
                   body="rect_plate", size=0.3, seed=51),
               notes("harp", "G4:0.1 B4:0.1 D5:0.1 G5:0.1 B5:0.8", 0.4, 0.6)],
              -22, duration=2.0, fade=0.4),
        sound("end/won.wav", "A night with a star: flute and celesta over the waltz bass.",
              [notes("flute", "D5:0.25 B4:0.25 G4:0.25 B4:0.25 D5:0.25 G5:0.9", 0.45, breath=0.3),
               notes("celesta", "[G4,B4,D5]:1.4", 0.3, 0.75),
               notes("upright_bass", "G2:0.75 D2:0.75 G2:0.9", 0.5)],
              -20, duration=2.6, fade=0.4),
        sound("end/lost.wav", "No star this night: the clarinet drifts down.",
              [notes("clarinet", "D5:0.4 C5:0.4 B4:0.4 A4:1.0", 0.45),
               notes("upright_bass", "D2:1.2 G1:1.0", 0.5)],
              -21, duration=2.4, fade=0.4),
        sound("stall/chestnuts.wav", "A chestnut stall, looped: roasting chestnuts popping and hissing on a brazier.",
              [sfx("fire", 0.0, 0.6, dur=3.0, size=0.3, crackle=18, wet=0.3, wind=0.1, seed=60),
               sfx("sizzle", 0.0, 0.3, dur=3.0, intensity=0.3, spit=3.0, hiss=0.3, seed=61)],
              -30, duration=2.5, loop=0.4),
        sound("stall/cider.wav", "A cider stall, looped: a kettle on the boil and mugs clinking.",
              [sfx("boil", 0.0, 0.5, dur=3.0, intensity=0.4, size=0.6, seed=62),
               sfx("clink", 0.8, 0.3, seed=63),
               sfx("clink", 2.1, 0.25, seed=64)],
              -30, duration=2.5, loop=0.4),
        sound("stall/band.wav", "A little band on the bank, looped: a hurdy accordion and a bass drum.",
              [notes("accordion", "G4:0.5 B4:0.5 D5:0.5 B4:0.5 C5:0.5 A4:0.5 B4:1.0", 0.4),
               notes("tuba", "G2:1 D2:1 C2:1 G2:1", 0.45)],
              -30, duration=4.0, loop=0.3),
        sound("stall/gingerbread.wav", "A gingerbread stall, looped: paper bags rustling and a crowd chatting close by.",
              [sfx("crumple", 0.0, 0.3, kind="paper", energy=0.2, density=0.3, size=0.3, dur=0.5, bright=0.3, seed=65),
               sfx("crumple", 1.5, 0.25, kind="paper", energy=0.2, density=0.3, size=0.3, dur=0.4, bright=0.3, seed=66),
               sfx("crowd_murmur", 0.0, 0.5, people=6, mood=0.7, distance=0.2, dur=3.0, seed=67)],
              -30, duration=2.5, loop=0.4),
        sound("music/sweep.wav", "A landed spin's harp sweep up.",
              [notes("harp", "G3:0.06 B3:0.06 D4:0.06 G4:0.06 B4:0.06 D5:0.06 G5:0.5", 0.45)],
              -24, duration=1.2, fade=0.3),
        {"out": "menu/move.wav", "_about": "Menu move: a light tick of a blade.",
         "layers": [sfx("scrape", 0.0, 1.0, dur=0.08, speed=1.0, force=0.6, roughness=0.3, material="metal",
                        body="rect_plate", size=0.25, bright=0.8, seed=70)],
         "fx": [{"type": "fade", "out": 0.03}, {"type": "mono"}], "duration": 0.12,
         "normalize": -9, "max_loudness": -28},
        {"out": "menu/select.wav", "_about": "Menu select: a short glide and a celesta note.",
         "layers": [sfx("scrape", 0.0, 0.6, dur=0.2, speed=0.9, force=0.5, roughness=0.3, material="metal",
                        body="rect_plate", size=0.25, seed=71),
                    notes("celesta", "D5:0.3", 0.4, 0.02)],
         "fx": [{"type": "mono"}], "normalize": -6, "max_loudness": -22},
        {"out": "menu/back.wav", "_about": "Menu back: a soft pat of snow.",
         "layers": [sfx("thud", 0.0, 1.0, body="sack", contact_ms=30, size=0.4, force=0.4, seed=72)],
         "fx": [{"type": "mono"}], "normalize": -9, "max_loudness": -26},
    ]
    for i, note in enumerate(SCALE):
        out.append(sound(f"lantern/light{i}.wav",
                         f"A lantern lights: a soft bell on {note}, the lantern's own note.",
                         [notes("celesta", f"{note}:1.0", 0.5),
                          notes("glass", f"{up_octave(note)}:0.8", 0.15, 0.02)],
                         -25, duration=1.4, fade=0.3))
        out.append(sound(f"lantern/take{i}.wav",
                         f"A lantern taken: a bright chime on {note} as you skate through its glow.",
                         [notes("glockenspiel", f"{up_octave(note) if i < 5 else note}:0.8", 0.45),
                          notes("celesta", f"{note}:0.8", 0.35, 0.01),
                          sfx("whoosh", 0.0, 0.2, dur=0.3, low=800, high=4000, direction="up")],
                         -22, duration=1.2, fade=0.3))
    return out


# --- music ------------------------------------------------------------------


def tokens(*bars: str) -> list[tuple[str, float]]:
    """Every note as (pitch, beats); a bare note is one beat."""
    out = []
    for bar in bars:
        for t in bar.split():
            pitch, _, length = t.partition(":")
            out.append((pitch, float(length) if length else 1.0))
    return out


def steps(toks: list[tuple[str, float]], transpose: int = 0) -> str:
    return " ".join(f"{p}:{n:g}" for p, n in toks)


# Chord: (bass root, bass fifth, chord voicing, arpeggio)
CHORD = {
    "G": ("G2", "D2", "[D3,G3,B3]", "G3 B3 D4"),
    "C": ("C2", "G2", "[C3,E3,G3]", "C3 E3 G3"),
    "D": ("D2", "A1", "[D3,F#3,A3]", "D3 F#3 A3"),
    "A": ("A1", "E2", "[C#3,E3,A3]", "A2 C#3 E3"),
    "Em": ("E2", "B1", "[E3,G3,B3]", "E3 G3 B3"),
}

# Each section: its meter (beats a bar), its tune bar by bar, and the chord
# in each bar ("C-D" splits a bar). Every tune is in G. The other tunes' pickups
# are moved to the end of the phrase before, so every section starts on its
# first downbeat.
SECTIONS: dict[str, tuple[int, list[tuple[str, float]], list[str]]] = {
    # The game's own waltz.
    "waltz-a": (3, tokens("D5:2 B4", "G4:2 B4", "A4 B4 C5", "B4:3",
                          "E5:2 C5", "D5:2 B4", "A4:2 F#4", "A4:3"),
                ["G", "G", "D", "G", "C", "G", "D", "D"]),
    "waltz-b": (3, tokens("D5:2 B4", "C5:2 A4", "B4:2 G4", "E4:3",
                          "F#4 G4 A4", "B4:2 D5", "C5 A4 F#4", "G4:3"),
                ["G", "C", "G", "C", "D", "G", "D", "G"]),
    # The tree song.
    "tree-a": (3, tokens("G4:0.75 G4:0.25 G4:1.5 A4:0.5", "B4:0.75 B4:0.25 B4:1.5 B4:0.5",
                         "A4:0.5 B4:0.5 C5 F#4", "A4 G4 D4",
                         "G4:0.75 G4:0.25 G4:1.5 A4:0.5", "B4:0.75 B4:0.25 B4:1.5 B4:0.5",
                         "A4:0.5 B4:0.5 C5 F#4", "A4 G4 D5"),
               ["G", "G", "C-D", "G", "G", "G", "C-D", "G"]),
    "tree-b": (3, tokens("D5:0.5 B4:0.5 E5:1.5 D5:0.5", "D5:0.75 C5:0.25 C5:1.5 C5:0.5",
                         "C5:0.5 A4:0.5 D5:1.5 C5:0.5", "C5:0.75 B4:0.25 B4:1.5 D4:0.5",
                         "G4:0.75 G4:0.25 G4:1.5 A4:0.5", "B4:0.75 B4:0.25 B4:1.5 B4:0.5",
                         "A4:0.5 B4:0.5 C5 F#4", "A4 G4:2"),
               ["G-C", "D", "D", "G", "G", "G", "C-D", "G"]),
    # The wishing song.
    "wish": (3, tokens("G4 G4:0.5 A4:0.5 G4:0.5 F#4:0.5", "E4 E4 E4", "A4 A4:0.5 B4:0.5 A4:0.5 G4:0.5",
                       "F#4 D4 D4", "B4 B4:0.5 C5:0.5 B4:0.5 A4:0.5", "G4 E4 D4:0.5 D4:0.5",
                       "E4 A4 F#4", "G4:2 D4"),
             ["G", "C", "A", "D", "Em", "C", "C-D", "G"]),
    # The king song.
    "king-a": (4, tokens("G4 G4 G4 A4", "G4 G4 D4:2", "E4 D4 E4 F#4", "G4:2 G4:2",
                         "G4 G4 G4 A4", "G4 G4 D4:2", "E4 D4 E4 F#4", "G4:2 G4:2"),
               ["G", "G-D", "Em-D", "G", "G", "G-D", "Em-D", "G"]),
    "king-b": (4, tokens("D5 C5 B4 A4", "B4 A4 G4:2", "E4 D4 E4 F#4", "G4:2 G4:2",
                         "D4 D4 E4 F#4", "G4 G4 A4:2", "D5 C5 B4 A4", "G4:2 C5:2", "G4:4"),
               ["G-D", "G", "Em-D", "G", "G-D", "G-D", "G-D", "G-C", "G"]),
    # The quiet song.
    "still-a": (3, tokens("D5:1.5 E5:0.5 D5", "B4:3", "D5:1.5 E5:0.5 D5", "B4:3",
                          "A5:2 A5", "F#5:3", "G5:2 G5", "D5:3",
                          "E5:2 E5", "G5:1.5 F#5:0.5 E5", "D5:1.5 E5:0.5 D5", "B4:3"),
                ["G", "G", "G", "G", "D", "D", "G", "G", "C", "C", "G", "G"]),
    "still-b": (3, tokens("E5:2 E5", "G5:1.5 F#5:0.5 E5", "D5:1.5 E5:0.5 D5", "B4:3",
                          "A5:2 A5", "C6:1.5 A5:0.5 F#5", "G5:3", "B5:3",
                          "G5:1.5 D5:0.5 B4", "D5:1.5 C5:0.5 A4", "G4:3", "G4:3"),
                ["C", "C", "G", "G", "D", "D", "G", "G", "G", "D", "G", "G"]),
    # Auld Lang Syne, for the new year.
    "year": (4, tokens("G4:1.5 G4:0.5 G4 B4", "A4:1.5 G4:0.5 A4 B4", "G4:1.5 G4:0.5 B4 D5", "E5:3 E5",
                       "D5:1.5 B4:0.5 B4 G4", "A4:1.5 G4:0.5 A4 B4", "G4:1.5 E4:0.5 E4 D4", "G4:3 D4"),
             ["G", "D", "G", "C", "G", "D", "C-D", "G"]),
}

# The tempo of each section, beats a minute. Must match music.rs.
BPM = {"waltz": 144, "tree": 120, "wish": 150, "king": 112, "still": 96, "year": 100}

# Lead: (instrument, gain, velocity, params, transpose)
LEADS = {
    "flute": ("flute", 0.42, 0.55, {"breath": 0.3}, 0),
    "clarinet": ("clarinet", 0.4, 0.55, None, 0),
    "celesta": ("celesta", 0.45, 0.55, None, 0),
    "music_box": ("music_box", 0.45, 0.55, None, 12),
    "trumpet": ("trumpet", 0.36, 0.6, None, 0),
    "horn": ("french_horn", 0.42, 0.6, None, -12),
}
# The accompaniments, and the leads each carries. Must match music.rs.
STYLES = {
    "waltz": ["flute", "clarinet", "celesta"],
    "hush": ["music_box", "celesta"],
    "band": ["trumpet", "horn"],
}


def bpm_of(section: str) -> int:
    return BPM[section.split("-")[0]]


def beats_of(section: str) -> float:
    return sum(n for _, n in SECTIONS[section][1])


def seq(inst: str, steps_: str, gain: float, pan: float = 0.0, vel: float = 0.6, **extra) -> dict:
    layer = {"type": "seq", "inst": inst, "step": 1, "steps": steps_, "gain": gain, "vel": vel}
    layer.update(extra)
    if pan:
        layer["fx"] = [{"type": "pan", "pos": pan}]
    return layer


def lead_layer(name: str, tune: list[tuple[str, float]], pan: float = 0.3) -> dict:
    inst, gain, vel, params, transpose = LEADS[name]
    extra = {}
    if params:
        extra["params"] = params
    if transpose:
        extra["transpose"] = transpose
    return seq(inst, steps(tune), gain, pan, vel, **extra)


def split(chord: str, meter: int) -> list[tuple[str, int]]:
    """A bar's chords as (chord, beats): "C-D" in 3 is 2 beats then 1."""
    a, _, b = chord.partition("-")
    if not b:
        return [(a, meter)]
    first = 2 if meter == 3 else meter // 2
    return [(a, first), (b, meter - first)]


def bed_layers(style: str, meter: int, chords: list[str]) -> list[dict]:
    bass, comp, arp = [], [], []
    for c in chords:
        for name, beats in split(c, meter):
            root, fifth, voicing, arpeggio = CHORD[name]
            if style == "waltz":
                # Bass on the first beat, the chord plucked on the others,
                # the harp running up the chord in quavers.
                bass.append(f"{root}:1 -:{beats - 1}" if beats > 1 else f"{root}:1")
                comp.extend(["-:1"] + [f"{voicing}:1"] * (beats - 1) if beats > 1 else [f"{voicing}:1"])
                run = arpeggio.split()
                arp.extend(f"{run[i % len(run)]}:0.5" for i in range(beats * 2))
            elif style == "hush":
                bass.append(f"{root}:{beats}")
                comp.append(f"{voicing}:{beats}")
            elif style == "band":
                # Oom-pah: tuba on the strong beats, trombones on the rest.
                for i in range(beats):
                    if i == 0:
                        bass.append(f"{root}:1")
                        comp.append("-:1")
                    elif meter == 4 and i == 2:
                        bass.append(f"{fifth}:1")
                        comp.append("-:1")
                    else:
                        bass.append("-:1")
                        comp.append(f"{voicing}:1")
            else:
                raise ValueError(style)
    if style == "waltz":
        return [
            seq("upright_bass", " ".join(bass), 0.5),
            seq("pizzicato", " ".join(comp), 0.28, -0.3, 0.5),
            seq("harp", " ".join(arp), 0.22, -0.45, 0.45),
        ]
    if style == "hush":
        return [
            seq("upright_bass", " ".join(bass), 0.35),
            seq("strings", " ".join(comp), 0.24, -0.25, 0.4),
        ]
    return [
        seq("tuba", " ".join(bass), 0.45),
        seq("trombone", " ".join(comp), 0.22, -0.3, 0.5),
    ]


def phrase(out: str, about: str, section: str, layers: list[dict], loudness: float) -> dict:
    return {
        "out": out,
        "_about": about,
        "bpm": bpm_of(section),
        "beats": beats_of(section),
        "tail": TAIL,
        "layers": layers,
        "fx": ROOM,
        "loudness": loudness,
    }


def medley() -> list[dict]:
    out = []
    leads_done = set()
    for name, (meter, tune, chords) in SECTIONS.items():
        bars = len(chords)
        assert beats_of(name) == meter * bars, name
        for style, leads in STYLES.items():
            for lead in leads:
                if (name, lead) in leads_done:
                    continue
                leads_done.add((name, lead))
                out.append(phrase(
                    f"music/lead/{name}-{lead}.wav",
                    f"Skate medley: the {name} tune on {lead}, {bars} bars at {bpm_of(name)} beats a minute "
                    "in G, a little right, plus its ring-out. Played with a bed of the same section.",
                    name, [lead_layer(lead, tune)], LEAD_LUFS))
            out.append(phrase(
                f"music/bed/{name}-{style}.wav",
                f"Skate medley: the {name} accompaniment, {style}, {bars} bars at {bpm_of(name)} beats a minute "
                "in G, plus its ring-out.",
                name, bed_layers(style, meter, chords), BED_LUFS))
    return out


def group(name: str, at: float, layers: list[dict]) -> dict:
    return {"type": "group", "_section": name, "at": at, "layers": layers}


def title() -> dict:
    """The waltz, twice through: first on celesta over held strings, then
    on flute over the waltz band. 96 beats at 144, 40 seconds."""
    layers = []
    at = 0.0
    for i, (name, style, lead) in enumerate([
        ("waltz-a", "hush", "celesta"), ("waltz-b", "hush", "celesta"),
        ("waltz-a", "waltz", "flute"), ("waltz-b", "waltz", "flute"),
    ]):
        meter, tune, chords = SECTIONS[name]
        layers.append(group(f"{i}-{name}", at, bed_layers(style, meter, chords) + [lead_layer(lead, tune)]))
        at += beats_of(name)
    return {
        "out": "music/title.wav",
        "bpm": 144,
        "beats": at,
        "loop": "fold",
        "loudness": -25,
        "_about": "Title theme, 96 beats at 144 a minute in G: the game's waltz twice, first on celesta over held "
                  "strings, then on flute over bass, pizzicato and harp. Leads a little right, accompaniment a "
                  "little left.",
        "layers": layers,
        "fx": ROOM,
    }


def pause() -> dict:
    """The waltz slowed on music box over strings: 48 beats at 120, 24 seconds."""
    layers = []
    at = 0.0
    for name in ("waltz-a", "waltz-b"):
        meter, tune, chords = SECTIONS[name]
        layers.append(group(name, at, bed_layers("hush", meter, chords) + [lead_layer("music_box", tune)]))
        at += beats_of(name)
    return {
        "out": "music/pause.wav",
        "bpm": 120,
        "beats": at,
        "loop": "fold",
        "loudness": -27,
        "_about": "Pause music, 48 beats at 120 a minute in G: the game's waltz slowed, on music box over held "
                  "strings and a soft bass.",
        "layers": layers,
        "fx": ROOM,
    }


def main() -> None:
    spec = {
        "genny": "blal1",
        "_about": "Snowfall Skate sounds, on blal1's genny, written by tools/snowfall_skate_sounds.py. A frozen "
                  "pond at night: your blades and your partner's, lanterns burning and chiming, drifts and thin "
                  "ice, spins, the snow coming down, and the Frost Fair's stalls. Music is a live medley of an "
                  "original waltz and public-domain tunes, with fixed renders for the title and pause.",
        "sounds": pond_sounds() + medley() + [title(), pause()],
    }
    SPEC.write_text(json.dumps(spec, indent=2) + "\n")
    print(f"wrote {SPEC.relative_to(ROOT)}: {len(spec['sounds'])} sounds")


if __name__ == "__main__":
    main()
