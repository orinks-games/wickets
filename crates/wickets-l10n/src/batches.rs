//! Untranslated lines in batches, for translating many lines at once.
//!
//! `export` writes `batch-0001.json` and on: a JSON list of
//! `{"msgid", "notes", "refs", "before", "after"}` for each untranslated
//! line of one language, where `before` and `after` are the lines next to
//! it in the template (context, not to translate). A translator writes
//! `batch-0001.done.json` beside it: a list of `{"msgid", "msgstr"}`.
//! `import` puts every done batch's translations into the language file,
//! skipping any `msgid` the file no longer has, and leaves every other entry
//! as it was.

use std::path::Path;

use hearken_l10n_tools::po::{parse_po, write_po};
use hearken_l10n_tools::{util, Project};
use serde_json::{json, Value};

fn language_file(project: &Project, code: &str) -> Result<std::path::PathBuf, String> {
    let path = project.lang_dir.join(format!("{code}.po"));
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("{} is missing; run extract first", path.display()))
    }
}

pub fn export(project: &Project, code: &str, dir: &Path, size: usize) -> Result<(), String> {
    let path = language_file(project, code)?;
    let entries = parse_po(&util::read_text(&path)?)?;
    // A language file carries no references; the template does.
    let template = parse_po(&util::read_text(
        &project.lang_dir.join(&project.template_name),
    )?)?;
    let refs: std::collections::HashMap<&str, &Vec<String>> = template
        .iter()
        .map(|e| (e.msgid.as_str(), &e.refs))
        .collect();
    let live: Vec<_> = entries
        .iter()
        .filter(|e| !e.msgid.is_empty() && !e.obsolete)
        .collect();
    let mut todo = Vec::new();
    for (i, e) in live.iter().enumerate() {
        if !e.msgstr.is_empty() && !e.fuzzy() {
            continue;
        }
        let near = |j: Option<usize>| j.and_then(|j| live.get(j)).map(|e| e.msgid.clone());
        todo.push(json!({
            "msgid": e.msgid,
            "notes": e.comments,
            "refs": refs.get(e.msgid.as_str()).copied().cloned().unwrap_or_default(),
            "before": near(i.checked_sub(1)),
            "after": near(Some(i + 1)),
        }));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut n = 0;
    for chunk in todo.chunks(size.max(1)) {
        n += 1;
        let file = dir.join(format!("batch-{n:04}.json"));
        let text = serde_json::to_string_pretty(chunk).map_err(|e| e.to_string())?;
        util::write_text(&file, &format!("{text}\n"))?;
    }
    println!(
        "{} untranslated lines of {code} in {n} batches in {}",
        todo.len(),
        dir.display()
    );
    Ok(())
}

pub fn import(project: &Project, code: &str, dir: &Path) -> Result<i32, String> {
    let path = language_file(project, code)?;
    let mut entries = parse_po(&util::read_text(&path)?)?;
    let mut done: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".done.json"))
        .collect();
    done.sort();
    let (mut filled, mut unknown, mut bad) = (0, 0, 0);
    for file in &done {
        let value: Value = serde_json::from_str(&util::read_text(file)?)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let items = value
            .as_array()
            .ok_or_else(|| format!("{}: expected a JSON array", file.display()))?;
        for item in items {
            let (Some(msgid), Some(msgstr)) = (
                item.get("msgid").and_then(Value::as_str),
                item.get("msgstr").and_then(Value::as_str),
            ) else {
                bad += 1;
                continue;
            };
            match entries
                .iter_mut()
                .find(|e| !e.obsolete && e.msgctxt.is_none() && e.msgid == msgid)
            {
                Some(entry) => {
                    entry.msgstr = msgstr.to_string();
                    entry.flags.retain(|f| f != "fuzzy");
                    filled += 1;
                }
                None => unknown += 1,
            }
        }
    }
    util::write_text(&path, &write_po(&entries, false))?;
    println!(
        "{code}: {filled} lines filled from {} batches; {unknown} not in the file; {bad} unreadable",
        done.len()
    );
    Ok(i32::from(bad > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn fixture(batch: &str) -> (Project, std::path::PathBuf) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "wickets-l10n-import-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("es.po"), "msgid \"Hello\"\nmsgstr \"\"\n").unwrap();
        std::fs::write(dir.join("batch-0001.done.json"), batch).unwrap();
        let project = Project {
            lang_dir: dir.clone(),
            ..Project::default()
        };
        (project, dir)
    }

    #[test]
    fn imports_an_array() {
        let (project, dir) = fixture(r#"[{"msgid":"Hello","msgstr":"Hola"}]"#);
        assert_eq!(import(&project, "es", &dir), Ok(0));
        let entries = parse_po(&util::read_text(&dir.join("es.po")).unwrap()).unwrap();
        assert_eq!(
            entries.iter().find(|e| e.msgid == "Hello").unwrap().msgstr,
            "Hola"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn accepts_an_empty_array() {
        let (project, dir) = fixture("[]");
        assert_eq!(import(&project, "es", &dir), Ok(0));
        let entries = parse_po(&util::read_text(&dir.join("es.po")).unwrap()).unwrap();
        assert!(entries
            .iter()
            .find(|e| e.msgid == "Hello")
            .unwrap()
            .msgstr
            .is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_non_arrays_without_writing_the_language_file() {
        for batch in ["{}", "null", "true", "42", r#""text""#] {
            let (project, dir) = fixture(batch);
            let before = std::fs::read(dir.join("es.po")).unwrap();
            assert_eq!(
                import(&project, "es", &dir),
                Err(format!(
                    "{}: expected a JSON array",
                    dir.join("batch-0001.done.json").display()
                ))
            );
            assert_eq!(std::fs::read(dir.join("es.po")).unwrap(), before);
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
}
