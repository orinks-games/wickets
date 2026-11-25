#!/usr/bin/env python3
"""Says which channels a release builds a copy for.

A copy shows what is ready for the channel it was built for
(parlor_core::channel), so a release reaches a less steady channel with a
copy built for that channel, not with its own package:

- a stable release builds stable, beta and alpha copies;
- a beta release builds beta and alpha copies;
- an alpha release builds an alpha copy.

A player's copy takes the newest GitHub release that carries its channel's
manifest and installs it only if it is newer than itself. So a release
leaves out a channel that already has a release as new as this one or
newer (say a 1.8.3 patch while 1.9.0 alphas are out): publishing there
would hide that channel's newer release from every copy that looks. Those
players get the fixes in their channel's next release.

    python3 tools/release_channels.py --tag=v1.9.0 --channel=stable RELEASES.json

RELEASES.json is GitHub's release list (`gh api repos/OWNER/REPO/releases`).
Prints the channels to build as a JSON list, own channel first, and why
any were left out.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

# Each channel and the less steady ones a release on it also reaches.
REACH = {
    "stable": ["stable", "beta", "alpha"],
    "beta": ["beta", "alpha"],
    "alpha": ["alpha"],
}

VERSION = re.compile(
    r"^v?(?P<core>\d+\.\d+\.\d+)(?:-(?P<pre>[0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$"
)
MANIFEST = re.compile(r"^update-(?P<channel>[a-z0-9-]+?)-[a-z0-9]+(?:-[a-z0-9]+)?\.json$")


def version_key(text: str):
    """A sort key with semver precedence, or None for a tag that is not one."""
    m = VERSION.match(text)
    if not m:
        return None
    core = tuple(int(n) for n in m["core"].split("."))
    pre = m["pre"]
    if pre is None:
        # A release sorts after every prerelease of it.
        return (core, 1, ())
    parts = tuple((0, int(p), "") if p.isdigit() else (1, 0, p) for p in pre.split("."))
    return (core, 0, parts)


def channels_published(release: dict) -> set[str]:
    """The channels a release carries a manifest for."""
    found = set()
    for asset in release.get("assets") or []:
        m = MANIFEST.match(asset.get("name", ""))
        if m:
            found.add(m["channel"])
    return found


def plan(tag: str, channel: str, releases: list[dict]) -> tuple[list[str], list[str]]:
    """The channels to build, and why any were left out."""
    own = version_key(tag)
    if own is None:
        raise ValueError(f"{tag} is not a version")
    build, skipped = [channel], []
    for extra in REACH[channel][1:]:
        newer = None
        for release in releases:
            if release.get("draft") or release.get("tag_name") == tag:
                continue
            if extra not in channels_published(release):
                continue
            key = version_key(release.get("tag_name", ""))
            if key is not None and key >= own and (newer is None or key > newer[0]):
                newer = (key, release["tag_name"])
        if newer:
            skipped.append(f"{extra}: {newer[1]} is already out there, as new as {tag} or newer")
        else:
            build.append(extra)
    return build, skipped


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tag", required=True)
    parser.add_argument("--channel", required=True, choices=sorted(REACH))
    parser.add_argument("releases", type=Path)
    args = parser.parse_args(argv)
    releases = json.loads(args.releases.read_text(encoding="utf-8"))
    build, skipped = plan(args.tag, args.channel, releases)
    for why in skipped:
        print(f"Leaving out {why}.", file=sys.stderr)
    print(json.dumps(build))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
