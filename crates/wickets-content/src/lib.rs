use std::path::{Path, PathBuf};

/// Set the `wickets_secrets` cfg when the overlay is in use, and say
/// whether it is.
pub fn detect() -> bool {
    println!("cargo:rustc-check-cfg=cfg(wickets_secrets)");
    println!("cargo:rerun-if-env-changed=WICKETS_STUB");
    // WICKETS_MODE pins the mode: "full" requires the overlay and fails the
    // build without it, "stub" ignores it. Release builds set "full", so a
    // stale cache or a missing checkout can never ship the stub build.
    println!("cargo:rerun-if-env-changed=WICKETS_MODE");
    let root = workspace_root();
    let marker = root.join("secrets").join("WICKETS_SECRETS");
    // A checkout moved into place keeps old mtimes, so watching the marker
    // alone can miss it; tools/overlay.py touches the marker, and the
    // directory itself is watched too.
    println!("cargo:rerun-if-changed={}", marker.display());
    println!("cargo:rerun-if-changed={}", root.join("secrets").display());
    let present = marker.is_file();
    if !present {
        // A watched path that never exists makes cargo rerun this script on
        // every build without the overlay, so an overlay checkout moved in
        // later (old mtimes and all) switches to the full build on the next
        // build, with no overlay.py step.
        println!(
            "cargo:rerun-if-changed={}",
            root.join("secrets").join(".absent").display()
        );
    }
    let mode = std::env::var("WICKETS_MODE").unwrap_or_default();
    let full = match mode.as_str() {
        "full" => {
            assert!(
                present,
                "WICKETS_MODE=full but the overlay checkout is missing"
            );
            true
        }
        "stub" => false,
        "" => present && std::env::var_os("WICKETS_STUB").is_none(),
        other => panic!("WICKETS_MODE must be full or stub, not {other}"),
    };
    if full {
        println!("cargo:rustc-cfg=wickets_secrets");
    }
    full
}

/// Write `$OUT_DIR/catalogs.rs`: `CATALOGS`, each language's PO text for
/// `Game::translation_catalogs`. The public `lang/<code>.po` comes first;
/// with the overlay (`full`), `secrets/lang/<code>.po` follows it without
/// its header, so each language is one catalog.
pub fn write_catalogs(full: bool) {
    let root = workspace_root();
    let public = root.join("lang");
    let private = root.join("secrets").join("lang");
    println!("cargo:rerun-if-changed={}", public.display());
    if full {
        println!("cargo:rerun-if-changed={}", private.display());
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    let mut codes: Vec<String> = std::fs::read_dir(&public)
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "po"))
                .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();
    codes.sort();
    let mut rows = String::new();
    for code in &codes {
        let path = public.join(format!("{code}.po"));
        println!("cargo:rerun-if-changed={}", path.display());
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        let extra = private.join(format!("{code}.po"));
        if full {
            println!("cargo:rerun-if-changed={}", extra.display());
            if let Ok(more) = std::fs::read_to_string(&extra) {
                text.push_str("\n");
                text.push_str(without_header(&more));
            }
        }
        let merged = out.join(format!("{code}.po"));
        std::fs::write(&merged, text).expect("OUT_DIR is writable");
        rows.push_str(&format!(
            "    ({code:?}, include_str!({:?})),\n",
            merged.display().to_string()
        ));
    }
    let source = format!(
        "/// Each language's catalog, written by `wickets_content::write_catalogs`.\n\
         pub const CATALOGS: &[(&str, &str)] = &[\n{rows}];\n"
    );
    std::fs::write(out.join("catalogs.rs"), source).expect("OUT_DIR is writable");
}

/// A PO file's text after its header entry (the first `msgid ""` and its
/// `msgstr` lines, up to the first blank line).
fn without_header(text: &str) -> &str {
    if !text.trim_start().starts_with("msgid \"\"") {
        return text;
    }
    match text.find("\n\n") {
        Some(end) => &text[end + 2..],
        None => "",
    }
}

fn workspace_root() -> PathBuf {
    let start = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"),
    );
    start
        .ancestors()
        .find(|dir| is_workspace(dir))
        .unwrap_or(&start)
        .to_path_buf()
}

fn is_workspace(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml"))
        .is_ok_and(|text| text.lines().any(|line| line.trim() == "[workspace]"))
}
