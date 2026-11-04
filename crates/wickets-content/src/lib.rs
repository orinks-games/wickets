use std::path::{Path, PathBuf};

pub fn detect() {
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
