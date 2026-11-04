#!/usr/bin/env python3
"""Check the test-target settings in every crate's Cargo.toml.

To keep CI fast, each crate builds its integration tests as one binary
(tests/all.rs, `autotests = false`), and a library with no unit tests
says `test = false`. This script fails when that would hide a test:

- a test file in tests/ that tests/all.rs does not list as a module;
- `#[cfg(test)]` in a library that says `test = false`.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def harness_files(all_rs: Path, tests: Path) -> set[Path]:
    """Return local Rust files reached by include! from the test harness."""
    pending = [all_rs]
    reached = set()
    while pending:
        path = pending.pop()
        if path in reached or not path.is_file():
            continue
        reached.add(path)
        text = path.read_text(encoding="utf-8")
        for name in re.findall(r'include!\("([^"]+)"\)', text):
            included = (path.parent / name).resolve()
            if tests.resolve() in included.parents:
                pending.append(included)
    return reached


def main() -> int:
    problems = []
    for toml in sorted(ROOT.glob("*/*/Cargo.toml")):
        crate = toml.parent
        text = toml.read_text(encoding="utf-8")
        tests = crate / "tests"
        files = sorted(p.stem for p in tests.glob("*.rs") if p.stem != "all")
        if files:
            all_rs = tests / "all.rs"
            harness = harness_files(all_rs, tests)
            listed = {
                name
                for path in harness
                for name in re.findall(
                    r"^mod (\w+);", path.read_text(encoding="utf-8"), re.M
                )
            }
            listed.update(path.stem for path in harness if path != all_rs)
            for name in files:
                if name not in listed:
                    problems.append(f"{crate.relative_to(ROOT)}/tests/{name}.rs is not a module of tests/all.rs")
        lib = re.search(r"^\[lib\]\n((?:[^\[].*\n|\n)*)", text, re.M)
        if lib and re.search(r"^test = false", lib.group(1), re.M):
            for src in (crate / "src").rglob("*.rs"):
                if "cfg(test)" in src.read_text(encoding="utf-8"):
                    problems.append(
                        f"{src.relative_to(ROOT)} has unit tests, but {toml.relative_to(ROOT)} says test = false under [lib]"
                    )
    for p in problems:
        print(p)
    if problems:
        return 1
    print("test targets OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
