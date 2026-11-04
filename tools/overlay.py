#!/usr/bin/env python3
"""Install or remove private sound assets in the workspace sound tree."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "secrets" / "sounds"
DESTINATION = ROOT / "sounds"
EXCLUDE = ROOT / ".git" / "info" / "exclude"
BEGIN = "# wickets overlay begin"
END = "# wickets overlay end"


def paths() -> list[Path]:
    if not SOURCE.is_dir():
        return []
    return sorted(path for path in SOURCE.rglob("*") if path.is_file())


def exclude_lines(relative: list[Path]) -> list[str]:
    return [BEGIN, *(f"/sounds/{path.as_posix()}" for path in relative), END]


def read_exclude() -> list[str]:
    return EXCLUDE.read_text(encoding="utf-8").splitlines() if EXCLUDE.exists() else []


def without_block(lines: list[str]) -> list[str]:
    out: list[str] = []
    inside = False
    for line in lines:
        if line == BEGIN:
            inside = True
            continue
        if line == END and inside:
            inside = False
            continue
        if not inside:
            out.append(line)
    return out


def write_exclude(relative: list[Path]) -> None:
    lines = without_block(read_exclude())
    if relative:
        if lines and lines[-1]:
            lines.append("")
        lines.extend(exclude_lines(relative))
    EXCLUDE.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join(lines)
    EXCLUDE.write_text(f"{text}\n" if text else "", encoding="utf-8")


def install() -> None:
    files = paths()
    if not files:
        print("Private sound overlay is unavailable; nothing copied.")
        return
    relative = [path.relative_to(SOURCE) for path in files]
    for source, rel in zip(files, relative, strict=True):
        destination = DESTINATION / rel
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
    write_exclude(relative)
    # A fresh mtime makes every build script look again (see wickets-content).
    marker = ROOT / "secrets" / "WICKETS_SECRETS"
    if marker.is_file():
        marker.touch()
    print(f"Copied {len(files)} sound files.")


def clean() -> None:
    relative = [path.relative_to(SOURCE) for path in paths()]
    for rel in relative:
        destination = DESTINATION / rel
        if destination.is_file():
            destination.unlink()
    for directory in sorted(
        {path.parent for rel in relative for path in [(DESTINATION / rel)]},
        key=lambda path: len(path.parts),
        reverse=True,
    ):
        if directory != DESTINATION:
            try:
                directory.rmdir()
            except OSError:
                pass
    write_exclude([])
    print(f"Removed {len(relative)} overlaid sound files.")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--clean", action="store_true")
    args = parser.parse_args()
    clean() if args.clean else install()


if __name__ == "__main__":
    main()
