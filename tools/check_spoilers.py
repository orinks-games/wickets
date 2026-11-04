#!/usr/bin/env python3
"""Reject private-overlay terms and accidentally restored sound assets."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PRIVATE = ROOT / "secrets"
WORDS = PRIVATE / "spoiler-words.txt"
ALLOW = PRIVATE / "spoiler-allow.txt"


def tracked_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    return [ROOT / p.decode("utf-8") for p in result.stdout.split(b"\0") if p]


def load_words() -> list[str]:
    return [
        line.strip().casefold()
        for line in WORDS.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def load_allow() -> tuple[list[str], list[tuple[str, re.Pattern[str]]]]:
    paths: list[str] = []
    lines: list[tuple[str, re.Pattern[str]]] = []
    if not ALLOW.exists():
        return paths, lines
    for raw in ALLOW.read_text(encoding="utf-8").splitlines():
        raw = raw.strip()
        if not raw or raw.startswith("#"):
            continue
        if ":" in raw:
            pattern, expression = raw.split(":", 1)
            lines.append((pattern, re.compile(expression, re.IGNORECASE)))
        else:
            paths.append(raw)
    return paths, lines


def permitted(
    relative: str,
    text: str,
    paths: list[str],
    lines: list[tuple[str, re.Pattern[str]]],
) -> bool:
    if any(fnmatch.fnmatch(relative, pattern) for pattern in paths):
        return True
    return any(
        fnmatch.fnmatch(relative, pattern) and expression.search(text)
        for pattern, expression in lines
    )


def digest(path: Path) -> bytes:
    return hashlib.sha256(path.read_bytes()).digest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--require", action="store_true")
    args = parser.parse_args()
    if not WORDS.exists():
        print("private word list is not present")
        return 1 if args.require else 0

    words = load_words()
    allow_paths, allow_lines = load_allow()
    files = [p for p in tracked_files() if p.exists()]
    hits: set[tuple[str, int]] = set()
    for path in files:
        relative = path.relative_to(ROOT).as_posix()
        if relative == "secrets" or relative.startswith("secrets/"):
            continue
        folded_path = relative.casefold()
        if any(word in folded_path for word in words) and not permitted(
            relative, relative, allow_paths, allow_lines
        ):
            hits.add((relative, 0))
        data = path.read_bytes()
        if b"\0" in data:
            continue
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError:
            continue
        for number, line in enumerate(text.splitlines(), 1):
            folded = line.casefold()
            if any(word in folded for word in words) and not permitted(
                relative, line, allow_paths, allow_lines
            ):
                hits.add((relative, number))

    private_sounds = PRIVATE / "sounds"
    if private_sounds.exists():
        private_hashes = {
            digest(path)
            for path in private_sounds.rglob("*")
            if path.is_file()
        }
        for path in files:
            relative = path.relative_to(ROOT).as_posix()
            if relative.startswith("secrets/") or not path.is_file():
                continue
            if digest(path) in private_hashes:
                hits.add((relative, 0))

    for relative, number in sorted(hits):
        print(f"{relative}:{number}")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
