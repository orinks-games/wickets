#!/usr/bin/env python3
"""Reject private-overlay terms in pull request text.

Reads the text to check (a PR title, description and commit messages) from
the files given as arguments, or from stdin when there are none or when an
argument is "-". Each input is named by its file name in the output.

The terms come from secrets/spoiler-words.txt. Terms listed in
secrets/spoiler-allow-pr.txt (one per line, "#" comments) are fine in PR
text and are dropped. Matching is case-insensitive, like check_spoilers.py.

CI logs are public, so this prints the input's name and the line number
only, never the term.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PRIVATE = ROOT / "secrets"
WORDS = PRIVATE / "spoiler-words.txt"
ALLOW_PR = PRIVATE / "spoiler-allow-pr.txt"


def read_terms(path: Path) -> list[str]:
    if not path.exists():
        return []
    return [
        line.strip().casefold()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]


def load_terms(words: Path | None = None, allow: Path | None = None) -> list[str]:
    allowed = set(read_terms(allow or ALLOW_PR))
    return [term for term in read_terms(words or WORDS) if term not in allowed]


def find_hits(text: str, terms: list[str]) -> list[int]:
    """Line numbers (from 1) of the lines that contain any term."""
    return [
        number
        for number, line in enumerate(text.splitlines(), 1)
        if any(term in line.casefold() for term in terms)
    ]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--require", action="store_true")
    parser.add_argument("inputs", nargs="*", help="files to check; - is stdin")
    args = parser.parse_args(argv)
    if not WORDS.exists():
        print("private word list is not present")
        return 1 if args.require else 0

    terms = load_terms()
    inputs = args.inputs or ["-"]
    failed = False
    for item in inputs:
        if item == "-":
            label, text = "stdin", sys.stdin.read()
        else:
            path = Path(item)
            label = path.name
            text = path.read_text(encoding="utf-8", errors="replace")
        for number in find_hits(text, terms):
            print(f"{label}:{number}")
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
