#!/usr/bin/env python3
"""Check player text against the mechanical rules of the in-game text skill.

The skill is .claude/skills/in-game-text/SKILL.md. This script checks only
the rules a program can check; a person still reads the text.

It reads two kinds of text:

- every help.md under crates/ and games/: no em-dashes, en-dashes,
  parentheses, ellipses, "!!", "(s)" plurals, ALL CAPS words, "%" or "deg",
  clock-style times such as 1:30, or tables; the last section is
  "## Controls"; the default-keys line appears exactly once.
- the story files, story/**/*.md: every page and opening line, by the
  same symbol, number and capital rules. Notes for writers (lines starting
  with ">"), headings and `key: value` lines are skipped. README.md,
  TEMPLATE.md and BIBLE.md are for writers, not player text, and are
  skipped.
- Rust string literals in crates/*/src and games/*/src: the same symbol,
  number and capital rules. Strings that are not player text are skipped:
  ones inside log, panic, assert, expect and print macros, attributes,
  #[cfg(test)] modules, sound and file keys, and short ids with no space.
  Format placeholders such as {name} or {:.1} are removed before checking.

Justified exceptions go in tools/check_text_allow.txt, one per line as
`path | rule | text`, under a `#` comment that says why. `text` is a part of
the flagged line, or `*` for every finding of that rule in the file. An
entry that no longer matches anything is reported, so stale ones get
removed.

Exit status 0 when clean, 1 when anything is flagged.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOW = ROOT / "tools" / "check_text_allow.txt"

DEFAULT_KEYS_LINE = (
    "These are the default keys. If you change them in Settings, "
    "the game's spoken instructions follow your bindings."
)

# Words in capitals that are fine: key names, and names that are said letter
# by letter anyway, such as BMX, Freewheel's bike. Words of one or two letters
# are never flagged.
CAPS_OK = {"ESC", "TAB", "USB", "NVDA", "JAWS", "TTS", "UTC", "BMX"}
CAPS_OK_RE = re.compile(r"^F\d{1,2}$")

# Calls and macros whose string arguments never reach a player.
SKIP_CALLS = {
    "panic", "assert", "assert_eq", "assert_ne", "debug_assert",
    "debug_assert_eq", "debug_assert_ne", "unreachable", "todo",
    "unimplemented", "expect", "expect_err", "println", "eprintln", "print",
    "eprint", "dbg", "trace", "debug", "info", "warn", "error", "log",
    "include_str", "include_bytes", "env", "concat", "compile_error",
    "file", "path", "join", "with_extension", "open", "create", "read",
    "write_all", "Path", "PathBuf", "var", "matches", "starts_with",
    "ends_with", "strip_prefix", "strip_suffix", "split", "trim_matches",
    "contains_key", "get", "insert", "remove", "entry", "parse",
}

RULES = [
    ("em-dash", re.compile("\u2014")),
    ("en-dash", re.compile("\u2013")),
    ("plural-s", re.compile(r"\(s\)")),
    ("parentheses", re.compile(r"[()]")),
    ("ellipsis", re.compile(r"\.\.\.|\u2026")),
    ("double-bang", re.compile(r"!!|\?!|!\?")),
    ("percent-sign", re.compile(r"%")),
    ("deg-unit", re.compile(r"\d\s*deg\b|\bdeg\b")),
    ("clock-time", re.compile(r"\b\d{1,2}:\d{2}\b")),
]
CAPS_WORD = re.compile(r"\b[A-Z][A-Z]{2,}\b")
# Functions whose strings no player hears: `State::lines` is the screen as
# text for a sighted helper and for tests.
SKIP_FNS = {"lines"}

# A sound key, file name or id: "safecracker/title-music", "records.json".
KEY_LIKE = re.compile(r"^[A-Za-z0-9_.\-]+(/[A-Za-z0-9_.\-{}]+)*$")
PLACEHOLDER = re.compile(r"\{\{|\}\}|\{[^{}]*\}")


def line_findings(text):
    """(rule, excerpt) for each rule a line of player text breaks."""
    out = []
    for rule, pattern in RULES:
        if rule == "parentheses" and any(r == "plural-s" for r, _ in out):
            continue
        if pattern.search(text):
            out.append((rule, text.strip()))
    for word in CAPS_WORD.findall(text):
        if word not in CAPS_OK and not CAPS_OK_RE.match(word):
            out.append(("all-caps", text.strip()))
            break
    return out


# -- help.md --------------------------------------------------------------


def check_help(path):
    findings = []
    lines = path.read_text(encoding="utf-8").splitlines()
    headings = []
    for n, line in enumerate(lines, 1):
        if line.startswith("## "):
            headings.append(line[3:].strip())
        if line.lstrip().startswith("|"):
            findings.append((n, "table", line.strip()))
        # Markdown link targets and inline code are not read as prose.
        prose = re.sub(r"`[^`]*`", "", re.sub(r"\]\([^)]*\)", "]", line))
        for rule, excerpt in line_findings(prose):
            findings.append((n, rule, excerpt))
    if not headings or headings[-1] != "Controls":
        last = headings[-1] if headings else "none"
        findings.append((len(lines), "controls-last", f"last section is {last}"))
    flat = " ".join(" ".join(lines).split())
    count = flat.count(DEFAULT_KEYS_LINE)
    if count != 1:
        findings.append((len(lines), "default-keys-line", f"found {count} times"))
    return findings


# -- Rust -----------------------------------------------------------------

TOKEN = re.compile(
    r"""
    (?P<line_comment>//[^\n]*)
  | (?P<block_comment>/\*)
  | (?P<raw>(?:b|c)?r(?P<hashes>\#*)")
  | (?P<string>(?:b|c)?")
  | (?P<char>'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'\n])')
  | (?P<lifetime>'[A-Za-z_]\w*)
  | (?P<ident>[A-Za-z_]\w*)
  | (?P<open>[(\[{])
  | (?P<close>[)\]}])
  | (?P<newline>\n)
  | (?P<other>.)
    """,
    re.X | re.S,
)


def unescape(body):
    """Turns the escapes that matter for checking into their characters."""

    def sub(m):
        e = m.group(1)
        if e.startswith("u{"):
            return chr(int(e[2:-1], 16))
        return {"n": "\n", "t": " ", "\\": "\\", '"': '"', "'": "'", "0": ""}.get(e, "")

    body = re.sub(r"\\\n\s*", "", body)  # line continuation
    return re.sub(r"\\(u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)", sub, body)


def rust_strings(src):
    """(line, text, skipped) for each string literal in a Rust file."""
    out = []
    pos = 0
    line = 1
    stack = []  # (kind, call name) per open bracket: call, attr, test, group
    last_ident = None
    prev = [None, None]  # the last two significant tokens
    test_mark = None  # None, "cfg" after #[cfg(test)], "mod" after its `mod`
    while pos < len(src):
        m = TOKEN.match(src, pos)
        kind = m.lastgroup
        if kind == "hashes":
            kind = "raw"
        text = m.group(0)
        if kind == "block_comment":
            depth, i = 1, m.end()
            while depth and i < len(src):
                if src.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif src.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
            line += src.count("\n", pos, i)
            pos = i
            continue
        if kind in ("raw", "string"):
            if kind == "raw":
                close = '"' + m.group("hashes")
                end = src.index(close, m.end())
                body = src[m.end():end]
                pos = end + len(close)
            else:
                i = m.end()
                while src[i] != '"':
                    i += 2 if src[i] == "\\" else 1
                body = unescape(src[m.end():i])
                pos = i + 1
            skipped = any(k in ("attr", "test") or name in SKIP_CALLS for k, name in stack)
            out.append((line, body, skipped))
            line += src.count("\n", m.start(), pos)
            prev = [prev[1], "string"]
            continue
        pos = m.end()
        if kind == "newline":
            line += 1
            continue
        if kind == "line_comment" or (kind == "other" and text.isspace()):
            continue
        if kind == "ident":
            last_ident = text
            if prev[1] == "fn_kw" and text in SKIP_FNS:
                test_mark = "mod"  # skip the body like a test module
            if test_mark == "cfg" and not any(k == "attr" for k, _ in stack):
                if text == "mod":
                    test_mark = "mod"
                elif text not in ("pub", "crate", "super", "in", "self"):
                    test_mark = None  # #[cfg(test)] on a fn or use: no block to skip
        elif kind == "open":
            if text == "[" and (prev[1] == "#" or prev == ["#", "!"]):
                stack.append(("attr", None))
                close = src.find("]", pos)
                if re.fullmatch(r"\s*cfg\s*\(\s*test\s*\)\s*", src[pos:close]):
                    test_mark = "cfg"
            elif text == "{" and test_mark == "mod":
                stack.append(("test", None))
                test_mark = None
            elif text == "(" and prev[1] in ("ident", "fn_kw", "!"):
                stack.append(("call", last_ident))
            else:
                stack.append(("group", None))
        elif kind == "close":
            if stack:
                stack.pop()
        sig = ("fn_kw" if text == "fn" else "ident") if kind == "ident" else text if kind == "other" and text in "#!" else kind
        prev = [prev[1], sig]
    return out


def check_rust(path):
    findings = []
    src = path.read_text(encoding="utf-8")
    for line, body, skipped in rust_strings(src):
        if skipped:
            continue
        text = PLACEHOLDER.sub("", body)
        stripped = text.strip()
        if not stripped:
            continue
        if KEY_LIKE.match(stripped):
            continue
        # Short ids and keys have no space; check those only for the marks
        # that cannot be part of one.
        if " " not in stripped:
            for rule, excerpt in line_findings(stripped):
                if rule in ("em-dash", "en-dash", "double-bang", "ellipsis"):
                    findings.append((line, rule, excerpt))
            continue
        for part in stripped.splitlines():
            for rule, excerpt in line_findings(part):
                findings.append((line, rule, excerpt))
    return findings


# -- story files ----------------------------------------------------------

META_LINE = re.compile(r"^[a-z][a-z ]*:")


def check_story(path):
    findings = []
    for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        text = line.strip()
        if not text or text.startswith((">", "#")) or META_LINE.match(text):
            continue
        for rule, excerpt in line_findings(text):
            findings.append((n, rule, excerpt))
    return findings


# -- allowlist ------------------------------------------------------------


def load_allow():
    entries = []
    if not ALLOW.exists():
        return entries
    reason = None
    for n, raw in enumerate(ALLOW.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line:
            reason = None
            continue
        if line.startswith("#"):
            reason = line
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) != 3 or not reason:
            print(f"{ALLOW.relative_to(ROOT)}:{n}: expected `path | rule | text` under a # reason")
            entries.append(None)
            continue
        entries.append({"path": parts[0], "rule": parts[1], "text": parts[2], "line": n, "used": False})
    return entries


def allowed(entries, path, rule, excerpt):
    for e in entries:
        if e and e["path"] == path and e["rule"] == rule and (e["text"] == "*" or e["text"] in excerpt):
            e["used"] = True
            return True
    return False


def main() -> int:
    entries = load_allow()
    problems = [None for e in entries if e is None]
    files = []
    for pattern in ("crates/*/help.md", "games/*/help.md"):
        files += [(p, check_help) for p in sorted(ROOT.glob(pattern))]
    for p in sorted(ROOT.glob("story/**/*.md")):
        if p.name not in ("README.md", "TEMPLATE.md", "BIBLE.md"):
            files.append((p, check_story))
    for pattern in ("crates/*/src/**/*.rs", "games/*/src/**/*.rs"):
        files += [(p, check_rust) for p in sorted(ROOT.glob(pattern))]
    count = 0
    for path, check in files:
        rel = path.relative_to(ROOT).as_posix()
        for line, rule, excerpt in check(path):
            if allowed(entries, rel, rule, excerpt):
                continue
            count += 1
            print(f"{rel}:{line}: {rule}: {excerpt[:120]}")
    for e in entries:
        if e and not e["used"]:
            problems.append(e)
            print(f"{ALLOW.relative_to(ROOT)}:{e['line']}: allowlist entry matches nothing, remove it")
    if count or problems:
        print(f"{count} text problems; rules in .claude/skills/in-game-text/SKILL.md")
        return 1
    print(f"text OK ({len(files)} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
