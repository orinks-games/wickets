//! Names that stay as they are: `names.txt` in a language folder lists the
//! app's, the games' and the people's names. Each becomes an entry of the
//! template and a translation of itself in every language file, so the
//! game finds the name whole and never translates it word by word inside a
//! longer line ("Lantern Lake" stays "Lantern Lake", not half Spanish).

use std::path::PathBuf;

use hearken_l10n_tools::po::{parse_po, write_po};
use hearken_l10n_tools::{extract, util, Entry, Project};

/// The names listed in the project's `names.txt`.
pub fn listed(project: &Project) -> Result<Vec<String>, String> {
    let path = project.lang_dir.join("names.txt");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    Ok(util::read_text(&path)?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect())
}

/// The names as template entries, for [`Project::entries`].
pub fn entries(project: &Project) -> Result<Vec<Entry>, String> {
    let reference = util::rel(&project.lang_dir.join("names.txt"), &project.root);
    Ok(listed(project)?
        .into_iter()
        .map(|name| {
            let mut entry = Entry::new(&name);
            entry.refs = vec![reference.clone()];
            entry.comments = vec!["a name: keep it as it is".to_string()];
            entry
        })
        .collect())
}

/// Every file the extractor writes, with each listed name filled in as
/// itself where a language file has it empty.
fn render(project: &Project) -> Result<Vec<(PathBuf, String)>, String> {
    let names = listed(project)?;
    let mut files = extract::render_all(project)?;
    for (_, text) in files.iter_mut().skip(1) {
        let mut entries = parse_po(text)?;
        let mut changed = false;
        for entry in entries.iter_mut() {
            if entry.msgstr.is_empty() && !entry.obsolete && names.contains(&entry.msgid) {
                entry.msgstr = entry.msgid.clone();
                changed = true;
            }
        }
        if changed {
            *text = write_po(&entries, false);
        }
    }
    Ok(files)
}

/// `extract::run` with the names filled in.
pub fn extract(project: &Project, check: bool, command: &str) -> Result<i32, String> {
    let stale: Vec<(PathBuf, String)> = render(project)?
        .into_iter()
        .filter(|(path, text)| util::read_text_lf(path).as_ref() != Some(text))
        .collect();
    if check {
        for (path, _) in &stale {
            println!("stale: {}", util::rel(path, &project.root));
        }
        if !stale.is_empty() {
            println!("Run: {command}");
            return Ok(1);
        }
        return Ok(0);
    }
    for (path, text) in &stale {
        util::write_text(path, text)?;
        println!("wrote {}", util::rel(path, &project.root));
    }
    Ok(0)
}
