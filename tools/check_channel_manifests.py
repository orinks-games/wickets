#!/usr/bin/env python3
"""Checks that a release publishes an update manifest for its own channel only.

Alpha players get alpha releases, beta players beta releases and stable
players stable ones. A package for one channel may leave out games from
another, so publishing a manifest for the wrong channel can hide content
from players who install it.

    python3 tools/check_channel_manifests.py --channel=CHANNEL DIR...
    python3 tools/check_channel_manifests.py --workflow [FILE]

The first checks a release's files (the release workflow's dist/, or a
published release's downloaded assets): every `update-<channel>-*.json`
is for CHANNEL, and its signed channel and platform match its name. The
second checks the release workflow itself: it signs manifests only with
`hearken release --channel="$CHANNEL"` and uploads only that manifest.

The signature is not checked here (the game checks that).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github" / "workflows" / "release.yml"

MANIFEST = re.compile(
    r"^update-(?P<channel>[a-z0-9-]+?)-"
    r"(?P<platform>windows-x64|linux-x64|linux-arm64|macos-arm64|macos)\.json$"
)


def check_dir(folder: Path, channel: str) -> list[str]:
    """Every manifest in `folder` that is not for `channel`."""
    problems: list[str] = []
    found = 0
    for path in sorted(folder.glob("update-*.json")):
        name = MANIFEST.match(path.name)
        if not name:
            problems.append(f"{path}: not named update-<channel>-<platform>.json")
            continue
        found += 1
        if name["channel"] != channel:
            problems.append(
                f"{path}: manifest for {name['channel']} on a {channel} release; "
                "each release publishes its own channel's only"
            )
        try:
            signed = json.loads(json.loads(path.read_text(encoding="utf-8"))["signed"])
        except (ValueError, KeyError) as e:
            problems.append(f"{path}: not a manifest ({e})")
            continue
        if signed.get("channel") != name["channel"]:
            problems.append(f"{path}: signed for channel {signed.get('channel')!r}")
        if signed.get("platform") != name["platform"]:
            problems.append(f"{path}: signed for platform {signed.get('platform')!r}")
        built = signed.get("built")
        if built is not None and built != channel:
            problems.append(f"{path}: its package was built for {built}, not {channel}")
    if found == 0:
        problems.append(f"{folder}: no update manifest for {channel}")
    return problems


def check_workflow(text: str) -> list[str]:
    """Every way the release workflow could publish another channel's manifest."""
    problems: list[str] = []
    for line in text.splitlines():
        code = line.split("#", 1)[0]
        if "channel-manifest" in code:
            problems.append(f"signs a manifest again for other channels: {line.strip()}")
        for flag in re.findall(r"hearken release\s[^\n]*?--channel=(\S+)", code):
            if flag.strip("\"'") != "$CHANNEL":
                problems.append(f"packages for a channel not the release's: {line.strip()}")
        upload = re.search(r"gh release upload\s+\S+\s+(\S+)", code)
        if upload and "update-" in upload.group(1):
            problems.append(f"uploads a manifest by name: {line.strip()}")
    uploads = re.findall(r"gh release upload\s+\S+\s+(\S+)", text)
    if uploads.count('"$MANIFEST"') != 1:
        problems.append("does not upload exactly one manifest, $MANIFEST")
    allowed = {'"$FILE"', '"$INDEX"', '"$MANIFEST"'}
    for arg in uploads:
        if arg not in allowed:
            problems.append(f"uploads {arg}, not the package, its index or its manifest")
    return problems


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--channel")
    parser.add_argument("--workflow", action="store_true")
    parser.add_argument("paths", nargs="*", type=Path)
    args = parser.parse_args(argv)
    if args.workflow:
        files = args.paths or [WORKFLOW]
        problems = [f"{f}: {p}" for f in files for p in check_workflow(f.read_text(encoding="utf-8"))]
    elif args.channel and args.paths:
        problems = [p for d in args.paths for p in check_dir(d, args.channel)]
    else:
        parser.error("give --channel and folders, or --workflow")
    for p in problems:
        print(f"::error::{p}" if "GITHUB_ACTIONS" in os.environ else p)
    if not problems:
        print("Only the release's own channel gets a manifest.")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
