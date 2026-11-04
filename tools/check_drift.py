#!/usr/bin/env python3
"""Keep the public copies and their full originals in step.

Some files exist twice: the public copy here and the full original in the
private overlay (secrets/), which is what a full build compiles. The overlay
lists them in secrets/DUPLICATED-FILES.txt and records, in secrets/drift.lock,
the approved difference between each pair (none, for most). A change made
to one copy and not the other changes that difference, and this check fails
with the path. After mirroring a change on purpose, run with --update and
commit the new lock in the overlay. Prints paths only, never content.

Without the overlay it prints a notice and passes, unless --require.
"""
import argparse, difflib, hashlib, json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SECRETS = ROOT / "secrets"
LIST = SECRETS / "DUPLICATED-FILES.txt"
LOCK = SECRETS / "drift.lock"


def pairs():
    for line in LIST.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            yield line


def fingerprint(rel):
    public = (ROOT / rel).read_bytes()
    private = (SECRETS / rel).read_bytes()
    if public == private:
        return "same"
    diff = difflib.unified_diff(
        private.decode("utf-8", "replace").splitlines(),
        public.decode("utf-8", "replace").splitlines(),
        lineterm="", n=0,
    )
    body = "\n".join(l for l in diff if not l.startswith("@@"))
    return hashlib.sha256(body.encode()).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--update", action="store_true")
    ap.add_argument("--require", action="store_true")
    args = ap.parse_args()
    if not LIST.is_file():
        print("Overlay unavailable; drift check skipped.")
        return 1 if args.require else 0
    bad, now = [], {}
    for rel in pairs():
        if not (ROOT / rel).is_file() or not (SECRETS / rel).is_file():
            bad.append(f"{rel}: one copy is missing")
            continue
        now[rel] = fingerprint(rel)
    if args.update:
        LOCK.write_text(json.dumps(now, indent=0, sort_keys=True) + "\n")
        print(f"Recorded {len(now)} pairs.")
        return 1 if bad else 0
    lock = json.loads(LOCK.read_text()) if LOCK.is_file() else {}
    for rel, fp in now.items():
        if lock.get(rel) != fp:
            bad.append(f"{rel}: public copy and original have drifted")
    for line in bad:
        print(line)
    print(f"{len(now)} pairs checked, {len(bad)} problems.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
