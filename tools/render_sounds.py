"""Render Wicket's prototype sounds with genny, then encode them to Ogg.

Every game keeps its sound specs in `sound-specs/<game id>.json` (a genny
batch spec). Each sound's `out` path is
relative, such as `fireworks/boom.wav`; it renders to
`build/genny/<game id>/fireworks/boom.wav` and is encoded to
`sounds/<game id>/fireworks/boom.ogg`, which the game plays by the key
`<game id>/fireworks/boom`. A spec must not set `out_dir`.

Sounds render with Oriol Gomez's genny (github.com/ogomez92/genny), always
its newest commit on GitHub. It runs as
`uvx --refresh-package genny --from git+https://github.com/ogomez92/genny genny`
unless `GENNY_ORIOL` names another command.

Which build made a sound is the `"genny"` key on the sound, or else on its
spec:

- `"genny": "oriol"`: rendered by this tool on Oriol's newest genny. Every
  new spec starts with it.
- `"genny": "blal1"`: made with blal1's genny (github.com/blal1/genny), which
  Wicket's no longer uses. Kept as shipped and never re-rendered.
- no key, or `"genny": "ogomez"`: made with an early Oriol's genny. Kept as
  shipped and never re-rendered.

To change a kept sound, rewrite it for Oriol's genny (`genny list --json`
lists what it has; many blal1 sounds and params are not in it) and mark it
`"genny": "oriol"` on the sound (or on the whole spec), then re-audition it:
it will sound different.

A sound's `"loudness"` key sets how loud it is, in LUFS (integrated
loudness, measured over at least half a second). Oriol's genny has no such
key, so this tool turns each rendered sound to it as it is encoded, and an
early Oriol's genny sound by gaining its committed Ogg (never re-rendered).
blal1's genny mastered its sounds to the key as they rendered, so they are
left exactly as committed. A sharp sound
turned up has its peaks limited to -1 dBFS, by at most 6 dB, so a crash may
stay a little under its loudness.

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

RENDERED = "oriol"  # rendered here on Oriol's newest genny
BUILDS = (RENDERED, "blal1", "ogomez")  # blal1, ogomez: kept as shipped, never rendered
# Always the newest genny commit. A new commit may change older sounds when a
# spec is re-rendered; that is fine.
DEFAULT_GENNY = "uvx --refresh-package genny --from git+https://github.com/ogomez92/genny genny"


def genny_command() -> list[str]:
    return shlex.split(os.environ.get("GENNY_ORIOL") or DEFAULT_GENNY)


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


def gain_chain(audio: Path, loudness: float) -> str | None:
    """The ffmpeg filter that turns `audio` to `loudness` LUFS, or None when
    it is already near enough."""
    loud, peak = measure(audio)
    gain = min(float(loudness) - loud, PEAK_LIMIT - peak + SQUEEZE)
    if abs(gain) < LEVEL_SLACK:
        return None
    chain = f"volume={gain:.2f}dB"
    if peak + gain > PEAK_LIMIT:
        limit = 10 ** (PEAK_LIMIT / 20)
        chain += f",alimiter=limit={limit:.4f}:attack=1:release=40:level=0:latency=1"
    return chain


def encode(src: Path, ogg: Path, chain: str | None) -> None:
    filters = ["-af", chain] if chain else []
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-i", str(src), *filters,
         "-c:a", "libvorbis", "-q:a", QUALITY, "-map_metadata", "-1",
         # Byte-identical output for the same input, so an unchanged sound
         # never shows up as a changed file.
         "-fflags", "+bitexact", "-flags:a", "+bitexact", "-serial_offset", "0",
         str(ogg)],
        check=True,
    )


def level(game: str, kept: list[dict]) -> None:
    """Turn each kept sound with a `loudness` key up or down to it."""
    for s in kept:
        if s.get("loudness") is None:
            continue
        ogg = SOUNDS / game / Path(s["out"]).with_suffix(".ogg")
        chain = gain_chain(ogg, s["loudness"])
        if chain is None:
            continue
        tmp = ogg.with_suffix(".level.ogg")
        encode(ogg, tmp, chain)
        tmp.replace(ogg)
        print(f"{game}/{Path(s['out']).with_suffix('')}: {chain.split(',')[0]}")


def render(spec_path: Path) -> None:
    game = spec_path.stem
    build = BUILD / game
    build.mkdir(parents=True, exist_ok=True)
    sound_outs(spec_path)  # rejects out_dir and bad out paths
    spec = json.loads(spec_path.read_text(encoding="utf-8"))
    todo = [s for s, b in sound_builds(spec_path) if b == RENDERED]
    kept = [(s, b) for s, b in sound_builds(spec_path) if b != RENDERED]
    if kept:
        print(f"{game}: kept {len(kept)} sounds as shipped")
        # blal1's genny already mastered its sounds to their loudness; only
        # the early Oriol's genny sounds are turned to it here.
        level(game, [s for s, b in kept if b == "ogomez"])
    if not todo:
        return
    # Render only the Oriol's genny sounds, as one batch with the spec's
    # shared settings. Loudness is this tool's key, not genny's.
    batch = {k: v for k, v in spec.items() if k not in ("sounds", "genny", "out")}
    batch["sounds"] = [{k: v for k, v in s.items() if k not in ("genny", "loudness")} for s in todo]
    batch_path = build / "_batch.json"
    batch_path.write_text(json.dumps(batch), encoding="utf-8")
    subprocess.run(
        [*genny_command(), "render", str(batch_path), "--out-dir", str(build), "--quiet", "--strict"],
        check=True,
    )
    for s in todo:
        out = s["out"]
        wav = build / out
        ogg = SOUNDS / game / Path(out).with_suffix(".ogg")
        ogg.parent.mkdir(parents=True, exist_ok=True)
        chain = gain_chain(wav, s["loudness"]) if s.get("loudness") is not None else None
        encode(wav, ogg, chain)
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
