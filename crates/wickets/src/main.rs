#![cfg_attr(windows, windows_subsystem = "windows")]
//! `wickets`: the launcher's entry point.

use std::path::{Path, PathBuf};

fn main() {
    move_old_name();
    // Check the computer's clock against a time server, so dated events
    // and challenges follow the real date. Never blocks start-up.
    hearken::clock::sync_in_background();
    // The internet date that lets a holiday special be found (see
    // discovery_clock). Never blocks either.
    wickets::discovery_clock::request();
    std::process::exit(hearken::run::<parlor_core::Parlor>(
        Default::default(),
        || Box::new(wickets::Lobby::new()),
    ));
}

/// Until 1.1.1 the game kept its files under the old project name: the
/// data folder (where saves live when the copy is not portable, as on a
/// Mac) and the seal witnesses. Each is renamed to the new name, unless
/// the new one is already there.
fn move_old_name() {
    let (old, new) = (parlor_core::OLD_DATA_DIR, parlor_core::DATA_DIR);
    let seal = |dir: &Path, name: &str| dir.join(format!("{name}.hearken-seal"));
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    if let Some(data) = dirs::data_local_dir() {
        moves.push((data.join(old), data.join(new)));
    }
    if let Some(config) = dirs::config_dir() {
        moves.push((seal(&config, old), seal(&config, new)));
    }
    if let Some(dir) = exe_dir {
        moves.push((seal(&dir, old), seal(&dir, new)));
    }
    for (from, to) in moves {
        if from.exists() && !to.exists() {
            // Best effort: the game still runs, with fresh files, if it fails.
            let _ = std::fs::rename(&from, &to);
        }
    }
}
