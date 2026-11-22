"""Write the band's recorded notes into `sound-specs/band.json`, and the
bank the game reads them by into `crates/parlor-core/src/band/bank.json`.

Wicket's newer run music is written in the game for each run by Hearken's
band (`hearken::band`): a composer picks a key, scale, tempo, chords and
tune, and the band plays them note by note. Every note it plays is one of
these single notes from Oriol's genny, recorded every three semitones
across the range each instrument is used in and tuned a little up or down
in the game. Drum hits are recorded once each.

Each note gets the house sound: a light room (reverb mix 0.1) and no pan
(the band places each part). Notes are levelled alike, -22 LUFS, and the
band's style sheets balance the parts.

    python tools/band_notes.py
    python tools/render_sounds.py band
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "sound-specs" / "band.json"
BANK = ROOT / "crates" / "parlor-core" / "src" / "band" / "bank.json"

NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]

# How long each kind of note is recorded, seconds, and how fast it fades
# when the band cuts it short, milliseconds.
BASS = (2.2, 80)
KEYS = (2.5, 140)
PAD = (4.5, 320)
STRUCK = (2.0, 160)
LEAD = (2.5, 110)
BOWED = (4.5, 220)

# Instrument, lowest and highest MIDI note used, and its kind.
INSTRUMENTS = [
    # Basses.
    ("fm_bass", 28, 55, BASS), ("moog_bass", 28, 55, BASS), ("reese", 28, 48, BASS),
    ("bass808", 24, 43, BASS), ("upright_bass", 28, 52, BASS), ("finger_bass", 28, 52, BASS),
    ("tuba", 29, 53, BASS),
    # Keys.
    ("epiano", 48, 79, KEYS), ("wurli", 48, 79, KEYS), ("felt_piano", 45, 81, KEYS),
    ("organ", 45, 76, KEYS), ("clav", 48, 76, KEYS), ("harpsichord", 48, 79, KEYS),
    ("accordion", 50, 79, KEYS),
    # Pads and held parts.
    ("warm_pad", 40, 72, PAD), ("glass_pad", 52, 79, PAD), ("string_machine", 43, 76, PAD),
    ("strings", 43, 79, PAD), ("choir", 48, 76, PAD), ("cello", 38, 67, BOWED),
    # Struck and plucked.
    ("vibraphone", 53, 84, STRUCK), ("marimba", 48, 84, STRUCK), ("celesta", 60, 91, STRUCK),
    ("music_box", 72, 96, STRUCK), ("kalimba", 55, 86, STRUCK), ("glockenspiel", 79, 96, STRUCK),
    ("steel_drum", 52, 81, STRUCK), ("xylophone", 65, 93, STRUCK), ("harp", 48, 86, STRUCK),
    ("synth_pluck", 45, 81, STRUCK), ("guitar", 45, 76, STRUCK), ("mandolin", 55, 81, STRUCK),
    ("banjo", 50, 79, STRUCK), ("pizzicato", 45, 79, STRUCK), ("handpan", 48, 72, STRUCK),
    # Tunes.
    ("square_lead", 55, 84, LEAD), ("soft_lead", 55, 86, LEAD), ("sync_lead", 55, 84, LEAD),
    ("chip", 57, 88, LEAD), ("sax", 50, 77, LEAD), ("flute", 62, 91, LEAD),
    ("clarinet", 52, 81, LEAD), ("trumpet", 55, 81, LEAD), ("flugelhorn", 52, 79, LEAD),
    ("violin", 57, 88, LEAD), ("whistle", 72, 93, LEAD), ("pan_flute", 62, 88, LEAD),
    ("oboe", 58, 86, LEAD), ("theremin", 55, 84, LEAD), ("french_horn", 45, 72, LEAD),
    ("brass", 45, 76, LEAD), ("pwm", 48, 79, LEAD),
]

# Each kit: what plays each part of a groove (see hearken's band). The
# hits are genny drums, recorded once and shared.
KITS = {
    "acoustic": {"kick": "kick", "snare": "snare", "hat": "hihat", "open": "openhat", "crash": "crash", "perc": "tom", "rim": "rim"},
    "electro": {"kick": "kick909", "snare": "clap", "hat": "hihat", "open": "openhat", "crash": "splash", "perc": "rimshot", "rim": "rimshot"},
    "eight": {"kick": "kick808", "snare": "snare808", "hat": "hihat", "open": "openhat", "crash": "splash", "perc": "cowbell", "rim": "rim"},
    "brush": {"kick": "kick", "snare": "brush", "hat": "pedal_hat", "open": "hihat", "crash": "splash", "perc": "shaker", "rim": "rim"},
    "world": {"kick": "frame_drum", "snare": "conga", "hat": "shaker", "open": "tambourine", "crash": "finger_cymbals", "perc": "bongo", "rim": "clave"},
    "march": {"kick": "kick", "snare": "snare", "crash": "crash", "perc": "triangle", "rim": "rim"},
    "sleigh": {"kick": "kick", "snare": "brush", "hat": "sleigh", "open": "tambourine", "crash": "finger_cymbals", "perc": "triangle", "rim": "woodblock"},
    "chip": {"kick": "zap_kick", "snare": "snare808", "hat": "hihat", "open": "openhat", "crash": "splash", "perc": "clap", "rim": "rimshot"},
}

ROOM = [{"type": "reverb", "mix": 0.1, "size": 0.4}]
LOUDNESS = -22


def name(midi: int) -> str:
    return f"{NAMES[midi % 12]}{midi // 12 - 1}"


def samples(low: int, high: int) -> list[int]:
    notes = list(range(low + 1, high, 3))
    if not notes or notes[-1] < high - 1:
        notes.append(high - 1)
    return notes


def main() -> None:
    sounds = []
    bank = {"prefix": "band", "instruments": [], "kits": []}
    for inst, low, high, (length, release) in INSTRUMENTS:
        notes = samples(low, high)
        for midi in notes:
            sounds.append({
                "out": f"{inst}/{midi}.wav",
                "loudness": LOUDNESS,
                "layers": [{"type": "note", "inst": inst, "notes": name(midi), "dur": length, "vel": 0.75}],
                "fx": ROOM,
            })
        bank["instruments"].append({
            "name": inst, "low": low, "high": high, "samples": notes,
            "length_s": length, "release_ms": release,
        })
    hits = sorted({h for kit in KITS.values() for h in kit.values()})
    for hit in hits:
        sounds.append({
            "out": f"drums/{hit}.wav",
            "loudness": LOUDNESS,
            "layers": [{"type": "drum", "kind": hit, "vel": 0.9}],
            "fx": ROOM,
        })
    for kit, parts in KITS.items():
        bank["kits"].append({"name": kit, "hits": [[what, hit] for what, hit in parts.items()]})
    spec = {
        "_about": "The band's recorded notes and drum hits, written by tools/band_notes.py: one genny note every three semitones across each instrument's range, with a light room, levelled alike. Hearken's band plays them in the game as each run's own song.",
        "genny": "oriol",
        "sounds": sounds,
    }
    SPEC.write_text(json.dumps(spec, indent=1) + "\n", encoding="utf-8")
    BANK.write_text(json.dumps(bank, indent=1) + "\n", encoding="utf-8")
    print(f"{len(sounds)} sounds, {len(INSTRUMENTS)} instruments, {len(KITS)} kits")


if __name__ == "__main__":
    main()
