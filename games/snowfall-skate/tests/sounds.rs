//! Every sound the game asks for has an Ogg.

use std::path::Path;

use snowfall_skate::music::{every_key, SWEEP};
use snowfall_skate::rules::{PatchKind, StallKind};
use snowfall_skate::sound;

fn has(key: &str) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    root.join(format!("{key}.ogg")).exists()
}

/// Every whole key written in the game's source, such as
/// "snowfall-skate/skate/blades" or "liberty-lights/crowd/ooh".
fn keys_in_source() -> Vec<String> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    let mut dirs = vec![src];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for part in text.split('"').skip(1).step_by(2) {
                let whole =
                    part.starts_with("snowfall-skate/") || part.starts_with("liberty-lights/");
                if whole && !part.contains('{') && !part.ends_with('/') {
                    out.push(part.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn every_key_in_the_source_is_rendered() {
    let keys = keys_in_source();
    assert!(keys.len() > 30, "{keys:?}");
    // The soundtrack's medley is a track id, played live, not a file.
    for key in keys
        .into_iter()
        .filter(|k| k != snowfall_skate::soundtrack::MEDLEY)
    {
        assert!(has(&key), "no Ogg for {key}");
    }
}

#[test]
fn every_section_note_patch_and_stall_is_rendered() {
    for key in every_key() {
        assert!(has(&key), "no Ogg for {key}");
    }
    assert!(has(SWEEP));
    for n in 0..8 {
        assert!(has(&sound::light_key(n)), "light {n}");
        assert!(has(&sound::take_key(n)), "take {n}");
    }
    for kind in [PatchKind::Drift, PatchKind::Thin] {
        assert!(has(sound::patch_key(kind)));
    }
    for kind in [
        StallKind::Chestnuts,
        StallKind::Cider,
        StallKind::Band,
        StallKind::Gingerbread,
    ] {
        assert!(has(sound::stall_key(kind)));
    }
}
