#!/usr/bin/env python3
"""Check that every game screen's name says which game it belongs to.

Each Hearken `State` is addressed by the string its `name()` returns: the
agent server's `screen` tool prints it, tests compare against it and
`pop_to` jumps to it. A screen inside games/<id>/ must return a name that
starts with the game id (hyphens as underscores), so the name alone says
whose screen it is, and no two screens may share a name. A game may wrap
one of the shared screens (results, pages, briefing) and keep that screen's name.

Run with --list to print every screen name and where it lives. The private
overlay under secrets/ is checked too when it is present.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TREES = ["games", "crates", "secrets/games", "secrets/crates"]
SHARED_WRAPPED = {"results", "pages", "briefing"}
IMPL = re.compile(
    r"\bimpl(?:<[^{]*?>)?\s+(?:\w+::)*(?:State|Menu)(?:<[^{]*?>)?\s+for\s+([\w:]+)"
)
NAME_FN = re.compile(r"fn name\(&self\) -> &'static str\s*\{")
LITERAL = re.compile(r'"([^"\\]*)"')
CONST = re.compile(r'const (\w+): &(?:\'static )?str = "([^"]*)"')


def block(text: str, start: int) -> tuple[int, int]:
    """Return the span of the brace block that opens at text[start] == '{'."""
    depth = 0
    i = start
    while i < len(text):
        c = text[i]
        if c == '"':
            end = text.find('"', i + 1)
            while end != -1 and text[end - 1] == "\\":
                end = text.find('"', end + 1)
            i = end + 1 if end != -1 else len(text)
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return start, i
        i += 1
    return start, len(text)


def strip_comments(text: str) -> str:
    """Drop line comments and `#[cfg(test)]` modules, which hold test-only screens."""
    text = re.sub(r"//[^\n]*", "", text)
    while True:
        m = re.search(r"#\[cfg\(test\)\]\s*(?:pub\s+)?mod\s+\w+\s*\{", text)
        if m is None:
            return text
        start, end = block(text, m.end() - 1)
        text = text[: m.start()] + text[end + 1 :]


def consts_in(path: Path, cache: dict[Path, dict[str, str]]) -> dict[str, str]:
    crate = path
    while crate.parent != crate and crate.name != "src":
        crate = crate.parent
    crate = crate.parent
    if crate not in cache:
        found: dict[str, str] = {}
        for p in crate.rglob("*.rs"):
            if "tests" in p.parts:
                continue
            for m in CONST.finditer(strip_comments(p.read_text(encoding="utf-8"))):
                found[m.group(1)] = m.group(2)
        cache[crate] = found
    return cache[crate]


def naming_macros(text: str) -> list[str]:
    """Macros defined in this file whose body writes the `name` method."""
    out = []
    for m in re.finditer(r"macro_rules!\s*(\w+)\s*\{", text):
        start, end = block(text, m.end() - 1)
        if NAME_FN.search(text[start:end]):
            out.append(m.group(1))
    return out


def macro_literal(body: str, macros: list[str]) -> str | None:
    """The name an impl body passes to one of the naming macros."""
    for name in macros:
        call = re.search(r"\b%s!\s*\(([^)]*)\)" % re.escape(name), body)
        if call:
            lit = LITERAL.findall(call.group(1))
            if lit:
                return lit[-1]
    return None


def resolve(expr: str, path: Path, text: str, cache) -> list[str] | None:
    expr = expr.strip()
    lits = LITERAL.findall(expr)
    if lits and not re.search(r"\w\s*\(", expr.replace("if", "").replace("else", "")):
        return lits
    if re.fullmatch(r"(?:[\w]+::)*[A-Z][A-Z0-9_]*", expr):
        ident = expr.split("::")[-1]
        own = {m.group(1): m.group(2) for m in CONST.finditer(text)}
        found = own.get(ident) if "::" not in expr else None
        found = found or consts_in(path, cache).get(ident)
        return [found] if found else None
    return None


def screens(root: Path = ROOT) -> list[tuple[str | None, Path, str, str]]:
    """Return (name, path, type, tree) for every screen under `root`; the
    name is None when only the running game knows it."""
    out = []
    cache: dict[Path, dict[str, str]] = {}
    for tree in TREES:
        base = root / tree
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            if "tests" in path.parts or "target" in path.parts:
                continue
            text = strip_comments(path.read_text(encoding="utf-8"))
            macros = naming_macros(text)
            for m in IMPL.finditer(text):
                open_brace = text.find("{", m.end())
                if open_brace == -1:
                    continue
                start, end = block(text, open_brace)
                body = text[start : end + 1]
                rel = path.relative_to(root)
                fn = NAME_FN.search(body)
                if fn is None:
                    given = macro_literal(body, macros)
                    if given is not None:
                        out.append((given, rel, m.group(1), tree))
                    continue
                fs, fe = block(body, fn.end() - 1)
                expr = body[fs + 1 : fe]
                names = resolve(expr, path, text, cache)
                if names is None:
                    out.append((None, rel, m.group(1), tree))
                for n in names or []:
                    out.append((n, rel, m.group(1), tree))
    return out


def game_of(rel: Path) -> str | None:
    parts = rel.parts
    if parts[0] == "secrets":
        parts = parts[1:]
    if len(parts) > 2 and parts[0] == "games":
        return parts[1].replace("-", "_")
    return None


def same_screen(a: Path, b: Path) -> bool:
    def key(p: Path) -> tuple[str, ...]:
        parts = [x for x in p.parts]
        if parts[0] == "secrets":
            parts = parts[1:]
        if parts[-1] == "public.rs":
            parts[-1] = "lib.rs"
        return tuple(parts)

    return key(a) == key(b)


def problems(found: list[tuple[str | None, Path, str, str]]) -> list[str]:
    """Every rule the screens break: a game screen without its game's
    prefix, or one name on two screens."""
    out: list[str] = []
    for name, rel, typ, _ in found:
        game = game_of(rel)
        if name is None or game is None:
            continue
        if name == game or name.startswith(game + "_") or name in SHARED_WRAPPED:
            continue
        out.append(
            f"{rel}: {typ} is named {name!r}; a {game.replace('_', '-')} screen "
            f"must start with {game + '_'!r}"
        )

    by_name: dict[str, list[tuple[Path, str]]] = {}
    for name, rel, typ, _ in found:
        if name is not None:
            by_name.setdefault(name, []).append((rel, typ))
    for name, places in sorted(by_name.items()):
        if name in SHARED_WRAPPED:
            places = [(rel, typ) for rel, typ in places if game_of(rel) is None]
        distinct: list[tuple[Path, str]] = []
        for rel, typ in places:
            if not any(same_screen(rel, other) for other, _ in distinct):
                distinct.append((rel, typ))
        if len(distinct) > 1:
            where = ", ".join(f"{rel} ({typ})" for rel, typ in distinct)
            out.append(f"{name!r} names more than one screen: {where}")
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--list", action="store_true", help="print every screen")
    args = ap.parse_args()
    found = screens()
    if args.list:
        width = max((len(n or "") for n, *_ in found), default=0)
        for name, rel, typ, _ in found:
            label = name if name is not None else "(decided at run time)"
            print(f"{label:<{width}}  {rel}  {typ}")
    found_problems = problems(found)
    for p in found_problems:
        print(p)
    count = len({(n, r) for n, r, *_ in found if n is not None})
    print(f"{count} screens checked, {len(found_problems)} problems.")
    return 1 if found_problems else 0


if __name__ == "__main__":
    sys.exit(main())
