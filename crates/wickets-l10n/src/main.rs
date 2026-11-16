//! Wicket's translation tools.
//!
//! ```text
//! cargo run -p wickets-l10n -- extract           # rewrite lang/ (and secrets/lang/)
//! cargo run -p wickets-l10n -- extract --check   # fail if any file is stale
//! cargo run -p wickets-l10n -- check [--stats]   # check every language file
//! cargo run -p wickets-l10n -- export es DIR [--private] [--size 200]
//! cargo run -p wickets-l10n -- import es DIR [--private]
//! ```
//!
//! `export` writes the untranslated lines of one language as numbered JSON
//! batches for a translator (see [`batches`]); `import` puts the finished
//! `*.done.json` batches back into the language file.
//!
//! Each language folder has a `sources.txt` naming where its spoken lines
//! come from (see [`sources`]). The public folder is `lang/`. When the
//! private overlay is checked out at `secrets/`, its `secrets/lang/` is a
//! second template holding only the lines the public one does not have, and
//! the full build merges the two for each language.

mod batches;
mod names;
mod rules;
mod sources;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hearken_l10n_tools::{check, extract, Project};

const COMMAND: &str = "cargo run -p wickets-l10n -- extract";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/wickets-l10n sits two folders under the workspace")
        .to_path_buf()
}

fn project(root: &Path, name: &str, template: &str) -> Result<Project, String> {
    let lang_dir = root.join("lang");
    let listed = sources::read(root, &lang_dir.join("sources.txt"))?;
    let mut project = Project {
        root: root.to_path_buf(),
        lang_dir,
        template_name: template.to_string(),
        header: Project::header_for(name, "https://github.com/orinks-games/wickets/issues"),
        sources: listed.rust,
        skip_paths: listed.skip,
        entries: listed.entries,
        rules: rules::ALL.to_vec(),
        ..Project::default()
    };
    let names = names::entries(&project)?;
    project.entries.extend(names);
    Ok(project)
}

/// The public project, then the private one when the overlay is present.
fn projects() -> Result<Vec<Project>, String> {
    let root = workspace_root();
    let public = project(&root, "Wicket's", "wickets.pot")?;
    let mut out = vec![public];
    let secrets = root.join("secrets");
    if secrets.join("lang").join("sources.txt").is_file() {
        let mut private = project(&secrets, "Wicket's", "private.pot")?;
        private.exclude = extract::template(&out[0])?
            .into_iter()
            .map(|e| e.msgid)
            .collect();
        out.push(private);
    }
    Ok(out)
}

fn run(args: &[String]) -> Result<i32, String> {
    let flag = |name: &str| args.iter().any(|a| a == name);
    let mut code = 0;
    match args.first().map(String::as_str) {
        Some("extract") => {
            for project in projects()? {
                code |= names::extract(&project, flag("--check"), COMMAND)?;
            }
        }
        Some("check") => {
            for project in projects()? {
                code |= check::run(&project, flag("--stats"))?;
            }
        }
        Some(command @ ("export" | "import")) => {
            let lang = args.get(1).ok_or("which language?")?;
            let dir = PathBuf::from(args.get(2).ok_or("which folder?")?);
            let mut all = projects()?;
            let project = if flag("--private") {
                all.pop()
                    .filter(|_| all.len() == 1)
                    .ok_or("no private overlay at secrets/")?
            } else {
                all.swap_remove(0)
            };
            let size = args
                .iter()
                .position(|a| a == "--size")
                .and_then(|i| args.get(i + 1))
                .and_then(|n| n.parse().ok())
                .unwrap_or(200);
            if command == "export" {
                batches::export(&project, lang, &dir, size)?;
            } else {
                code = batches::import(&project, lang, &dir)?;
            }
        }
        _ => {
            return Err(
                "usage: wickets-l10n extract [--check] | check [--stats] | export|import CODE DIR [--private]"
                    .into(),
            );
        }
    }
    Ok(code)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(2)
        }
    }
}
