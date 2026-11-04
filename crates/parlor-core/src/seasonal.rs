//! Text that only an installed content overlay may show.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use hearken::Ctx;

use crate::{challenges, Date, Parlor};

type Filtered = HashMap<(usize, String), &'static str>;
type Open = fn(Date) -> bool;

const SEASONS: &[(&str, Open)] = &[];

pub fn help(ctx: &Ctx<Parlor>, doc: &'static str) -> &'static str {
    help_on(doc, challenges::today(ctx))
}

pub fn help_on(doc: &'static str, date: Date) -> &'static str {
    if !doc.contains('{') {
        return doc;
    }
    let open: Vec<&str> = SEASONS
        .iter()
        .filter(|(_, on)| on(date))
        .map(|(name, _)| *name)
        .collect();
    static CACHE: OnceLock<Mutex<Filtered>> = OnceLock::new();
    let key = (doc.as_ptr() as usize, open.join(","));
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    cache
        .entry(key)
        .or_insert_with(|| Box::leak(filter(doc, &open).into_boxed_str()))
}

pub fn filter(doc: &str, open: &[&str]) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in doc.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let (heading, mark) = trailing_mark(heading);
            skipping = mark.is_some_and(|name| !open.contains(&name));
            if !skipping {
                out.push_str("## ");
                out.push_str(heading);
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
        let rest = match leading_mark(rest) {
            Some((name, _)) if !open.contains(&name) => continue,
            Some((_, words)) => words,
            None => rest,
        };
        out.push_str(bullet);
        out.push_str(&inline(rest, open));
        out.push('\n');
    }
    out
}

fn season(name: &str) -> Option<&'static str> {
    SEASONS.iter().map(|(value, _)| *value).find(|value| *value == name)
}

fn trailing_mark(heading: &str) -> (&str, Option<&'static str>) {
    if let Some(open) = heading.rfind(" {") {
        if let Some(name) = heading[open + 2..].strip_suffix('}') {
            if let Some(mark) = season(name) {
                return (&heading[..open], Some(mark));
            }
        }
    }
    (heading, None)
}

fn leading_mark(line: &str) -> Option<(&'static str, &str)> {
    let rest = line.strip_prefix('{')?;
    let (name, words) = rest.split_once("} ")?;
    Some((season(name)?, words))
}

fn inline(line: &str, open: &[&str]) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let mark = after.split_once(':').and_then(|(name, tail)| {
            let name = season(name)?;
            let (words, tail) = tail.split_once('}')?;
            Some((name, words, tail))
        });
        match mark {
            Some((name, words, tail)) => {
                out.push_str(&rest[..start]);
                if open.contains(&name) {
                    out.push_str(words);
                }
                rest = tail;
            }
            None => {
                out.push_str(&rest[..start + 1]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_text_is_unchanged() {
        assert_eq!(filter("## Play\n\nRun.\n", &[]), "## Play\n\nRun.\n");
    }

    #[test]
    fn unknown_marks_are_ordinary_braces() {
        assert_eq!(filter("A {b} c {d:e}.\n", &[]), "A {b} c {d:e}.\n");
    }
}
