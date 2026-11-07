"""Write Freewheel's pedal-bike drivetrain loops into sound-specs/freewheel.json.

A pedal bike makes no engine sound. Pedalling is the chain running over
the teeth, a soft run of small ticks that swells with each push on a pedal.
Coasting is the freewheel: the pawls in the rear hub ticking over the
ratchet, fast and even, slowing as the bike slows (the game turns the loop's
pitch with speed, which also turns its tick rate).

Each bike gets its own pair:

- road: a fine, light chain and a fast, light freewheel tick.
- mtb: a gritty, uneven chain and a deeper, louder freewheel buzz.
- bmx: a chunky single-speed chain and a loud, slower clatter.
- ebike: its own motor stays as it is; only its freewheel tick is here.

The ticks are genny's pinball spinner flap, one flap each, every one with
its own seed so no two ticks are alike. The drivetrain sits a little to
the right, where a bike's chain is, with a light outdoor reverb. Each is a
two-second "fold" loop, so the reverb tail wraps into the start.

Run it after changing anything here, then render:

    python tools/freewheel_bike_sounds.py
    python tools/render_sounds.py freewheel
"""

from __future__ import annotations

import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = ROOT / "sound-specs" / "freewheel.json"

LOOP = 2.0
# Five pedal strokes in a loop: an easy cadence at the loop's own speed.
STROKE = LOOP / 5
PAN = 0.25
ROOM = {"type": "reverb", "mix": 0.08, "size": 0.3, "damp": 0.5}


def fx(cutoff: float | None = None):
    tone = [{"type": "lowpass", "cutoff": cutoff}] if cutoff else []
    return [{"type": "mono"}, *tone, ROOM, {"type": "pan", "pos": PAN}]


def tick(at: float, gain: float, tone: float, seed: int) -> dict:
    return {
        "type": "sfx",
        "kind": "spinner",
        "at": round(at, 4),
        "gain": round(gain, 3),
        "params": {"spins": 1, "tone": round(tone, 3), "seed": seed},
    }


def freehub(out: str, rate: float, tone: float, gain: float, jitter: float,
            seed: int, loudness: float) -> dict:
    """Pawls over the ratchet: `rate` ticks a second at the loop's speed."""
    rng = random.Random(seed)
    n = round(LOOP * rate)
    step = LOOP / n
    layers = []
    for i in range(n):
        at = i * step + rng.uniform(-jitter, jitter) * step
        # Two pawls a little apart in strength, as real hubs have.
        g = gain * (1.0 if i % 2 == 0 else 0.82) * rng.uniform(0.88, 1.0)
        layers.append(tick(max(at, 0.0), g, tone * rng.uniform(0.97, 1.03), seed * 1000 + i))
    return {"genny": "blal1", "out": out, "layers": layers, "fx": fx(),
            "duration": LOOP, "loop": "fold", "loudness": loudness}


def chain(out: str, rate: float, tone: float, gain: float, swell: float,
          grit: float, push: float, cutoff: float, seed: int, loudness: float) -> dict:
    """The chain running over the teeth while you pedal.

    `rate` ticks a second, louder at each pedal push by `swell`; `grit` is
    how uneven they are; `push` is a soft scuff of the drivetrain taking
    each stroke. The ticks are lower and softer than the freewheel's and
    fall at random, so they blur into a purr: an even train this fast
    would be heard as a pitched buzz, like a motor.
    """
    rng = random.Random(seed)
    n = round(LOOP * rate)
    layers = []
    for i in range(n):
        at = rng.uniform(0.0, LOOP)
        phase = (at % STROKE) / STROKE
        # Strongest just after the top of each stroke, easing off before the next.
        force = 1.0 - swell + swell * max(0.0, 1.0 - abs(phase - 0.25) / 0.45)
        g = gain * force * rng.uniform(1.0 - 0.5 * grit, 1.0)
        layers.append(tick(at, g, tone * rng.uniform(0.85, 1.15), seed * 1000 + i))
    if push:
        for k in range(5):
            layers.append({
                "type": "sfx", "kind": "noise", "at": round(k * STROKE + 0.04, 4), "gain": push,
                "params": {"dur": 0.22, "color": "pink", "attack": 0.06},
                "fx": [{"type": "bandpass", "center": 700, "q": 1.2}],
            })
    return {"genny": "blal1", "out": out, "layers": layers, "fx": fx(cutoff),
            "duration": LOOP, "loop": "fold", "loudness": loudness}


# Loudness (LUFS): the freewheel sits level with the tyres so coasting is
# easy to hear; the chain sits under them, as on a real bike.
SOUNDS = [
    freehub("bike/road/freehub.wav", rate=34, tone=1.15, gain=0.7, jitter=0.05, seed=11, loudness=-28),
    freehub("bike/mtb/freehub.wav", rate=24, tone=0.8, gain=0.9, jitter=0.06, seed=12, loudness=-27),
    freehub("bike/bmx/freehub.wav", rate=15, tone=0.65, gain=1.0, jitter=0.08, seed=13, loudness=-27),
    freehub("bike/ebike/freehub.wav", rate=28, tone=1.0, gain=0.6, jitter=0.05, seed=14, loudness=-30),
    chain("bike/road/pedal.wav", rate=60, tone=0.7, gain=0.4, swell=0.45, grit=0.2,
          push=0.0, cutoff=4500, seed=21, loudness=-31),
    chain("bike/mtb/pedal.wav", rate=48, tone=0.55, gain=0.5, swell=0.55, grit=0.7,
          push=0.12, cutoff=3500, seed=22, loudness=-30),
    chain("bike/bmx/pedal.wav", rate=36, tone=0.45, gain=0.6, swell=0.6, grit=0.5,
          push=0.18, cutoff=3000, seed=23, loudness=-29),
]


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
    text = SPEC.read_text(encoding="utf-8")
    new = {s["out"]: s for s in SOUNDS}
    spans = sound_spans(text)
    missing = set(new) - {s["out"] for _, _, s in spans}
    if missing:
        raise SystemExit(f"not in the spec: {sorted(missing)}")
    for start, end, s in reversed(spans):
        if s["out"] in new:
            text = text[:start] + json.dumps(new[s["out"]], ensure_ascii=False) + text[end:]
    json.loads(text)
    SPEC.write_text(text, encoding="utf-8")
    print(f"wrote {len(new)} drivetrain loops to {SPEC.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
