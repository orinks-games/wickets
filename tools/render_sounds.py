"""Render Wicket's prototype sounds with genny, then encode them to Ogg.

Every game keeps its sound specs in `sound-specs/<game id>.json` (a genny
batch spec). Each sound's `out` path is
relative, such as `fireworks/boom.wav`; it renders to
`build/genny/<game id>/fireworks/boom.wav` and is encoded to
`sounds/<game id>/fireworks/boom.ogg`, which the game plays by the key
`<game id>/fireworks/boom`. A spec must not set `out_dir`.

Sounds render with blal1's genny (github.com/blal1/genny, on PyPI as
`genny`), always the latest release, run as `uvx --from genny@latest genny` unless `GENNY_BLAL1` names
another command, such as `genny-blal1`.

The first games were made with Oriol Gomez's genny
(github.com/ogomez92/genny). This tool no longer runs it: those sounds stay
as they were rendered, and their committed Oggs are kept. Which build made a
sound is the `"genny"` key on the sound, or else on its spec:

- `"genny": "blal1"`: rendered by this tool. Every new spec starts with it.
- no key, or `"genny": "ogomez"`: an Oriol's genny sound, kept as shipped
  and never re-rendered. To change one, rewrite it for blal1's genny and
  mark it `"genny": "blal1"` on the sound (or on the whole spec), then
  re-audition it: the same spec renders differently, often several dB
  louder or quieter.

A sound's `"loudness"` key sets how loud it is, in LUFS (integrated
loudness, measured over at least half a second, as genny does). genny masters
blal1 sounds to it as they render. A kept Oriol's genny sound with the key is
turned up or down to it instead: its committed Ogg is gained and re-encoded,
never re-rendered. A sharp sound turned up has its peaks limited to -1 dBFS,
by at most 6 dB, so a crash may stay a little under its loudness.

Also needs `ffmpeg`. Deterministic: the same specs give the same sounds.

Usage:
    python tools/render_sounds.py                 # every spec
    python tools/render_sounds.py liberty-lights  # one game's specs
    python tools/render_sounds.py --check         # every spec's sounds exist, none extra

"""

from __future__ import annotations

import json
import os
import shlex
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPECS = ROOT / "sound-specs"
BUILD = ROOT / "build" / "genny"
SOUNDS = ROOT / "sounds"
QUALITY = "5"
PEAK_LIMIT = -1.0  # dBFS: a kept sound turned up never peaks past this
SQUEEZE = 6.0  # dB: how far a kept sound's peaks may be limited to reach its loudness
LEVEL_SLACK = 0.3  # LU: a kept sound this near its loudness is left alone

# Recorded lines a game plays when present and speaks when not: the game,
# the folder under sounds/<game id>/, and the script whose ids name the
# clips (`id|...` a line, `#` comments skipped).
OPTIONAL_RECORDINGS = {}

BUILDS = ("blal1", "ogomez")  # ogomez: Oriol's genny, kept as shipped, never rendered
# Always the latest genny. A new release may change older sounds when a spec is
# re-rendered; that is fine.
DEFAULT_GENNY = "uvx --from genny@latest genny"


def genny_command() -> list[str]:
    return shlex.split(os.environ.get("GENNY_BLAL1") or DEFAULT_GENNY)


def sound_builds(spec_path: Path) -> list[tuple[dict, str]]:
    """Each sound in the spec with the genny build that made it."""
    spec = json.loads(spec_path.read_text(encoding="utf-8"))
    sounds = spec["sounds"] if "sounds" in spec else [spec]
    out = []
    for s in sounds:
        build = s.get("genny", spec.get("genny", "ogomez"))
        if build not in BUILDS:
            sys.exit(f"{spec_path.name}: genny must be one of {', '.join(BUILDS)}, not {build!r}")
        out.append((s, build))
    return out


def sound_outs(spec_path: Path) -> list[str]:
    spec = json.loads(spec_path.read_text(encoding="utf-8"))
    if "out_dir" in spec:
        sys.exit(f"{spec_path.name}: remove out_dir; outs are relative to the game's folder")
    sounds = spec["sounds"] if "sounds" in spec else [spec]
    outs = []
    for s in sounds:
        out = s["out"]
        if Path(out).is_absolute() or ".." in Path(out).parts or not out.endswith(".wav"):
            sys.exit(f"{spec_path.name}: bad out path {out!r}")
        outs.append(out)
    return outs


def measure(ogg: Path) -> tuple[float, float]:
    """Integrated loudness (LUFS, padded to half a second) and sample peak (dBFS)."""
    out = subprocess.run(
        ["ffmpeg", "-hide_banner", "-nostats", "-i", str(ogg),
         "-af", "apad=whole_dur=0.5,ebur128,volumedetect", "-f", "null", "-"],
        capture_output=True, text=True, check=True,
    ).stderr
    loud = float(out.split("Summary:")[1].split("I:")[1].split("LUFS")[0])
    peak = float(out.split("max_volume:")[1].split("dB")[0])
    return loud, peak


def level(game: str, kept: list[dict]) -> None:
    """Turn each kept sound with a `loudness` key up or down to it."""
    for s in kept:
        if s.get("loudness") is None:
            continue
        ogg = SOUNDS / game / Path(s["out"]).with_suffix(".ogg")
        loud, peak = measure(ogg)
        gain = min(float(s["loudness"]) - loud, PEAK_LIMIT - peak + SQUEEZE)
        if abs(gain) < LEVEL_SLACK:
            continue
        chain = f"volume={gain:.2f}dB"
        if peak + gain > PEAK_LIMIT:
            limit = 10 ** (PEAK_LIMIT / 20)
            chain += f",alimiter=limit={limit:.4f}:attack=1:release=40:level=0:latency=1"
        tmp = ogg.with_suffix(".level.ogg")
        subprocess.run(
            ["ffmpeg", "-y", "-loglevel", "error", "-i", str(ogg), "-af", chain,
             "-c:a", "libvorbis", "-q:a", QUALITY, "-map_metadata", "-1",
             "-fflags", "+bitexact", "-flags:a", "+bitexact", "-serial_offset", "0",
             str(tmp)],
            check=True,
        )
        tmp.replace(ogg)
        print(f"{game}/{Path(s['out']).with_suffix('')}: {gain:+.1f} dB")


def render(spec_path: Path) -> None:
    game = spec_path.stem
    build = BUILD / game
    build.mkdir(parents=True, exist_ok=True)
    sound_outs(spec_path)  # rejects out_dir and bad out paths
    spec = json.loads(spec_path.read_text(encoding="utf-8"))
    todo = [s for s, b in sound_builds(spec_path) if b == "blal1"]
    kept = [s for s, b in sound_builds(spec_path) if b != "blal1"]
    if kept:
        print(f"{game}: kept {len(kept)} sounds from Oriol's genny as shipped")
        level(game, kept)
    if not todo:
        return
    # Render only the blal1 sounds, as one batch with the spec's shared settings.
    batch = {k: v for k, v in spec.items() if k not in ("sounds", "genny", "out")}
    batch["sounds"] = [{k: v for k, v in s.items() if k != "genny"} for s in todo]
    batch_path = build / "_batch.json"
    batch_path.write_text(json.dumps(batch), encoding="utf-8")
    subprocess.run(
        [*genny_command(), "render", str(batch_path), "--out-dir", str(build), "--quiet", "--strict"],
        check=True,
    )
    for out in (s["out"] for s in todo):
        wav = build / out
        ogg = SOUNDS / game / Path(out).with_suffix(".ogg")
        ogg.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            ["ffmpeg", "-y", "-loglevel", "error", "-i", str(wav),
             "-c:a", "libvorbis", "-q:a", QUALITY, "-map_metadata", "-1",
             # Byte-identical output for the same WAV, so an unchanged sound
             # never shows up as a changed file.
             "-fflags", "+bitexact", "-flags:a", "+bitexact", "-serial_offset", "0",
             str(ogg)],
            check=True,
        )
        print(f"{game}/{Path(out).with_suffix('')}")


def recorded(game: str) -> set[Path]:
    """The recorded voice clips of a game, which genny does not render: the
    names in `sound-specs/<game id>.voice.txt`, one `name|words` a line,
    each played from `sounds/<game id>/voice/<name>.ogg`."""
    listing = SPECS / f"{game}.voice.txt"
    if not listing.exists():
        return set()
    names = [line.split("|", 1)[0].strip() for line in listing.read_text(encoding="utf-8").splitlines()
             if line.strip() and not line.startswith("#")]
    return {(SOUNDS / game / "voice" / f"{n}.ogg").resolve() for n in names}


def optional(game: str) -> set[Path]:
    """The clips a game may have recorded but need not: see
    `OPTIONAL_RECORDINGS`."""
    if game not in OPTIONAL_RECORDINGS:
        return set()
    folder, script = OPTIONAL_RECORDINGS[game]
    ids = [line.split("|", 1)[0].strip() for line in script.read_text(encoding="utf-8").splitlines()
           if line.strip() and not line.startswith("#")]
    return {(SOUNDS / game / folder / f"{i}.ogg").resolve() for i in ids}


def check() -> int:
    bad = 0
    for spec_path in sorted(SPECS.glob("*.json")):
        game = spec_path.stem
        sound_builds(spec_path)  # rejects an unknown build name
        want = {(SOUNDS / game / Path(o).with_suffix(".ogg")).resolve() for o in sound_outs(spec_path)}
        have = {p.resolve() for p in (SOUNDS / game).rglob("*.ogg")} if (SOUNDS / game).exists() else set()
        have -= recorded(game)
        have -= optional(game)
        for p in sorted(recorded(game) - {p.resolve() for p in (SOUNDS / game).rglob("*.ogg")}):
            print(f"missing recording: {p.relative_to(ROOT)}")
            bad += 1
        for p in sorted(want - have):
            print(f"missing: {p.relative_to(ROOT)}")
            bad += 1
        for p in sorted(have - want):
            print(f"not in any spec: {p.relative_to(ROOT)}")
            bad += 1
    return 1 if bad else 0


def main(argv: list[str]) -> int:
    if argv == ["--check"]:
        return check()
    specs = [SPECS / f"{name}.json" for name in argv] if argv else sorted(SPECS.glob("*.json"))
    for spec_path in specs:
        if not spec_path.exists():
            sys.exit(f"no spec {spec_path.relative_to(ROOT)}")
        render(spec_path)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
