//! `lang/sources.txt`: where a template's spoken lines come from.
//!
//! One source a line, a kind and a path relative to the repository root.
//! `#` starts a comment.
//!
//! ```text
//! rust games              # every .rs file under games/ (tests skipped)
//! help games              # every help.md under games/
//! story story             # every .md under story/, read as scene text
//! lines games/x/lines.txt 3   # field 3 of each `a|b|c` line
//! fields days.md brief,special # the values of these `key: value` lines
//! skip /some/path/part    # pass over .rs files whose path contains this
//! engine crates/hearken-app/src  # Hearken's own lines, from the pinned engine
//! ```
//!
//! Rust literals go through Hearken's extractor. The text kinds are read
//! here: each line or paragraph a player can hear becomes an entry, and so
//! does each of its sentences, since the game translates a long line one
//! sentence at a time when it has no entry for the whole.

use std::path::{Path, PathBuf};

use hearken_l10n_tools::{util, Entry};

#[derive(Default)]
pub struct Sources {
    pub rust: Vec<PathBuf>,
    pub skip: Vec<String>,
    pub entries: Vec<Entry>,
}

pub fn read(root: &Path, file: &Path) -> Result<Sources, String> {
    let text = util::read_text(file)?;
    let mut out = Sources::default();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let at = || format!("{}:{}", file.display(), n + 1);
        let path = parts
            .get(1)
            .ok_or_else(|| format!("{}: needs a path", at()))?;
        let full = root.join(path);
        match parts[0] {
            "rust" => out.rust.push(PathBuf::from(path)),
            "skip" => out.skip.push(path.to_string()),
            "engine" => {
                let engine = engine_root()?;
                let found = hearken_l10n_tools::rust_strings::extract(
                    &engine,
                    &[PathBuf::from(path)],
                    &[],
                )?;
                for template in found {
                    let mut entry = Entry::new(&template.msgid);
                    // A path in Cargo's checkout differs by machine, so
                    // the reference names the engine's own path.
                    entry.refs = template
                        .refs
                        .iter()
                        .take(1)
                        .map(|r| format!("hearken/{r}"))
                        .collect();
                    if !template.placeholders.is_empty() {
                        let holes: Vec<String> = template
                            .placeholders
                            .iter()
                            .map(|p| format!("{{{p}}}"))
                            .collect();
                        entry.comments = vec![format!("placeholders: {}", holes.join(", "))];
                    }
                    if !out.entries.iter().any(|e| e.msgid == entry.msgid) {
                        out.entries.push(entry);
                    }
                }
            }
            "help" => {
                for file in files(&full, |p| p.file_name().is_some_and(|n| n == "help.md")) {
                    let rel = util::rel(&file, root);
                    let doc = util::read_text(&file)?;
                    // A seasonal mark shows or hides text by the date: take
                    // the help both ways.
                    for open in [true, false] {
                        add_all(&mut out.entries, &rel, help_lines(&seasons(&doc, open)));
                    }
                }
            }
            "story" => {
                for file in files(&full, |p| p.extension().is_some_and(|e| e == "md")) {
                    let rel = util::rel(&file, root);
                    add_all(
                        &mut out.entries,
                        &rel,
                        story_lines(&util::read_text(&file)?),
                    );
                }
            }
            "lines" => {
                let column: usize = parts
                    .get(2)
                    .and_then(|c| c.parse().ok())
                    .filter(|c| *c >= 1)
                    .ok_or_else(|| format!("{}: lines needs a field number", at()))?;
                let rel = util::rel(&full, root);
                add_all(
                    &mut out.entries,
                    &rel,
                    field_lines(&util::read_text(&full)?, column),
                );
            }
            "fields" => {
                let keys: Vec<&str> = parts
                    .get(2)
                    .ok_or_else(|| format!("{}: fields needs keys", at()))?
                    .split(',')
                    .collect();
                for file in files(&full, |p| p.extension().is_some_and(|e| e == "md")) {
                    let rel = util::rel(&file, root);
                    add_all(
                        &mut out.entries,
                        &rel,
                        field_values(&util::read_text(&file)?, &keys),
                    );
                }
            }
            other => return Err(format!("{}: unknown kind {other:?}", at())),
        }
    }
    Ok(out)
}

/// The checkout of the Hearken this workspace builds against: Cargo's git
/// checkout of the pinned commit, or a local path patch.
fn engine_root() -> Result<PathBuf, String> {
    let output =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(["metadata", "--format-version", "1"])
            .output()
            .map_err(|e| format!("cargo metadata: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let key = "\"manifest_path\":\"";
    let manifest = text
        .split(key)
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(|raw| PathBuf::from(raw.replace("\\\\", "\\")))
        .find(|path| {
            path.parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == "hearken-app")
        })
        .ok_or("cargo metadata does not list hearken-app; build once first")?;
    // .../crates/hearken-app/Cargo.toml -> the engine's checkout
    manifest
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .ok_or_else(|| "hearken-app's checkout has no parent".to_string())
}

/// Files under `path` (or `path` itself) that `keep` accepts, sorted.
fn files(path: &Path, keep: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    util::walk_files(path)
        .into_iter()
        .filter(|p| !p.components().any(|c| c.as_os_str() == "target"))
        .filter(|p| keep(p))
        .collect()
}

fn add_all(entries: &mut Vec<Entry>, reference: &str, lines: Vec<String>) {
    for line in lines {
        let mut pieces = vec![line.clone()];
        let sentences = sentences(&line);
        if sentences.len() > 1 {
            pieces.extend(sentences.into_iter().map(str::to_string));
        }
        for piece in pieces {
            let piece = piece.trim().to_string();
            if piece.chars().filter(|c| c.is_alphabetic()).count() < 2 {
                continue;
            }
            if entries.iter().any(|e| e.msgid == piece) {
                continue;
            }
            let mut entry = Entry::new(&piece);
            entry.refs = vec![reference.to_string()];
            entries.push(entry);
        }
    }
}

/// Sentences the way the game's translator cuts a line: after `.`, `!` or
/// `?`, at a space before a capital, a digit or an opening quote.
pub fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 1;
    while i < chars.len() {
        if matches!(chars[i - 1].1, '.' | '!' | '?') && chars[i].1.is_whitespace() {
            let mut j = i;
            while j < chars.len() && chars[j].1.is_whitespace() {
                j += 1;
            }
            if j < chars.len()
                && (chars[j].1.is_uppercase() || chars[j].1.is_ascii_digit() || chars[j].1 == '"')
            {
                out.push(text[start..chars[i].0].trim());
                start = chars[j].0;
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    let rest = text[start..].trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

/// A list item's text (`- `, `* `, `1. `), or `None`.
fn list_item(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return Some(rest.trim_start());
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        if let Some(rest) = line[digits..].strip_prefix(". ") {
            return Some(rest.trim_start());
        }
    }
    None
}

/// A help document as the help screen pages it: each heading, and each
/// paragraph or list item joined into one line. Text before the first
/// `## ` heading (the document's title) is not read.
pub fn help_lines(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut page = String::new();
    let mut started = false;
    let flush = |page: &mut String, out: &mut Vec<String>| {
        if !page.trim().is_empty() {
            out.push(std::mem::take(page).trim().to_string());
        }
        page.clear();
    };
    for raw in doc.lines() {
        let line = raw.trim();
        if let Some(title) = line.strip_prefix("## ") {
            started = true;
            flush(&mut page, &mut out);
            out.push(title.trim().to_string());
            continue;
        }
        if !started {
            continue;
        }
        if line.is_empty() {
            flush(&mut page, &mut out);
            continue;
        }
        if line.starts_with('#') {
            flush(&mut page, &mut out);
            let words = line.trim_start_matches('#').trim();
            out.push(words.to_string());
            if !words.ends_with(['.', ':', '?', '!']) {
                out.push(format!("{words}."));
            }
            continue;
        }
        match list_item(line) {
            Some(text) => {
                flush(&mut page, &mut out);
                page.push_str(text);
            }
            None => {
                if !page.is_empty() {
                    page.push(' ');
                }
                page.push_str(line);
            }
        }
    }
    flush(&mut page, &mut out);
    out
}

fn is_mark(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

/// A help document with every season shown (`open`) or none, as
/// `parlor_core::seasonal::filter` shows it: a `## Heading {season}`
/// section, a `{season} ` line and an inline `{season:words}` appear only
/// in their season.
pub fn seasons(doc: &str, open: bool) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in doc.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let marked = heading.rfind(" {").filter(|&at| {
                heading.ends_with('}') && is_mark(&heading[at + 2..heading.len() - 1])
            });
            skipping = marked.is_some() && !open;
            if !skipping {
                out.push_str("## ");
                out.push_str(marked.map_or(heading, |at| &heading[..at]));
                out.push('\n');
            }
            continue;
        }
        if skipping {
            continue;
        }
        let (bullet, rest) = match line.strip_prefix("- ") {
            Some(rest) => ("- ", rest),
            None => ("", line),
        };
        let lead = rest
            .strip_prefix('{')
            .and_then(|r| r.split_once("} "))
            .filter(|(name, _)| is_mark(name));
        let rest = match lead {
            Some(_) if !open => continue,
            Some((_, words)) => words,
            None => rest,
        };
        out.push_str(bullet);
        let mut tail = rest;
        while let Some(start) = tail.find('{') {
            let after = &tail[start + 1..];
            let mark = after
                .split_once(':')
                .filter(|(name, _)| is_mark(name))
                .and_then(|(_, t)| t.split_once('}'));
            match mark {
                Some((words, t)) => {
                    out.push_str(&tail[..start]);
                    if open {
                        out.push_str(words);
                    }
                    tail = t;
                }
                None => {
                    out.push_str(&tail[..=start]);
                    tail = after;
                }
            }
        }
        out.push_str(tail);
        out.push('\n');
    }
    out
}

/// Whether a line is a `key: value` setting (`id: act1/keeper`,
/// `lamps: 0`), not text.
fn is_setting(line: &str) -> bool {
    match line.split_once(':') {
        Some((key, _)) => {
            !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        }
        None => false,
    }
}

/// A speaker's name before a line (`Mabel: Hello.`): one or two capitalised
/// words.
fn strip_speaker(line: &str) -> &str {
    if let Some((who, said)) = line.split_once(": ") {
        let words: Vec<&str> = who.split(' ').collect();
        let named = words.len() <= 2
            && words.iter().all(|w| {
                w.chars().next().is_some_and(char::is_uppercase)
                    && w.chars()
                        .all(|c| c.is_alphabetic() || c == '\'' || c == '.')
            });
        if named && !said.trim().is_empty() {
            return said.trim();
        }
    }
    line
}

/// Scene text: every line that is not a note (`>`), a setting or a code
/// block. Headings are kept (scene names are heard); a speaker's name is
/// left off its line.
pub fn story_lines(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for raw in doc.lines() {
        let line = raw.trim();
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.is_empty() || line.starts_with('>') || line.starts_with("<!--") {
            continue;
        }
        if is_setting(line) {
            continue;
        }
        let line = line.trim_start_matches('#').trim();
        let line = list_item(line).unwrap_or(line);
        out.push(strip_speaker(line).to_string());
    }
    out
}

/// Field `column` (from 1) of each `a|b|c` line, skipping `#` comments.
pub fn field_lines(text: &str, column: usize) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('|').nth(column - 1))
        .map(|f| f.trim().to_string())
        .collect()
}

/// The values of `key: value` lines whose key is one of `keys`.
pub fn field_values(text: &str, keys: &[&str]) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().split_once(": "))
        .filter(|(k, _)| keys.contains(k))
        .map(|(_, v)| v.trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_pages_join_paragraphs_and_split_items() {
        let doc = "# Title\n\nSkipped.\n\n## How to play\n\nPress Enter to start.\nThe run ends at dawn.\n\n- Up moves.\n- Down waits.\n";
        assert_eq!(
            help_lines(doc),
            [
                "How to play",
                "Press Enter to start. The run ends at dawn.",
                "Up moves.",
                "Down waits."
            ]
        );
    }

    #[test]
    fn seasonal_marks_show_both_ways() {
        let doc =
            "## Play\n\nRun.{winter: Or skate.}\n\n{winter} Snow.\n\n## Ice {winter}\n\nCold.\n";
        assert_eq!(
            seasons(doc, true),
            "## Play\n\nRun. Or skate.\n\nSnow.\n\n## Ice\n\nCold.\n"
        );
        assert_eq!(seasons(doc, false), "## Play\n\nRun.\n\n\n");
    }

    #[test]
    fn story_drops_notes_settings_and_speakers() {
        let doc = "> A note.\n\n# The new keeper\nid: act1/keeper\nlamps: 0\n\nMabel: So you're the new keeper.\nHello, keeper.\n";
        assert_eq!(
            story_lines(doc),
            [
                "The new keeper",
                "So you're the new keeper.",
                "Hello, keeper."
            ]
        );
    }

    #[test]
    fn sentences_cut_like_the_translator() {
        assert_eq!(
            sentences("Walls say \"Open: up.\" Then you move. 1.5 seconds pass."),
            [
                "Walls say \"Open: up.\" Then you move.",
                "1.5 seconds pass."
            ]
        );
    }

    #[test]
    fn fields_come_from_pipe_lines() {
        assert_eq!(
            field_lines("# id|who|words\na|Mabel|Hello there.|warm\n", 3),
            ["Hello there."]
        );
    }
}
