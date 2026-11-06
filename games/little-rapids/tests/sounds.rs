//! Every sound the game asks for has an Ogg.

use std::path::Path;

use little_rapids::music::{note_key, Chord, Part, NOTES};
use little_rapids::rules::Kind;
use little_rapids::sound;

fn has(key: &str) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    root.join(format!("{key}.ogg")).exists()
}

#[test]
fn every_phrase_and_note_is_rendered() {
    for part in Part::ALL {
        for chord in Chord::ALL {
            for n in 0..part.variants() {
                let key = part.key(chord, n);
                assert!(has(&key), "no Ogg for {key}");
            }
        }
    }
    for chord in Chord::ALL {
        for combo in 1..=NOTES {
            let key = note_key(chord, combo);
            assert!(has(&key), "no Ogg for {key}");
        }
    }
}

#[test]
fn every_cue_and_loop_is_rendered() {
    for kind in Kind::ALL {
        assert!(has(sound::floating(kind)), "no float loop for {kind:?}");
    }
    for key in [
        sound::TITLE_MUSIC,
        sound::MOUTH_MUSIC,
        sound::SANK_MUSIC,
        sound::WATER,
        sound::BANK,
        sound::CURRENT,
        sound::LAUNCH,
        sound::BUMP_BANK,
        sound::BUMP_STONE,
        sound::BUMP_MILL,
        sound::BUMP_DUCK,
        sound::BUMP_FALLS,
        sound::CLEAN,
        sound::SCRAP,
        sound::TRINKET,
        sound::GATE,
        sound::FALLS,
        sound::MULTIPLIER,
        sound::NEW_BEST,
        sound::RIBBON,
        sound::MOUTH,
        sound::SANK,
        sound::MUSIC_BOX,
        sound::FOLD,
        sound::CURRENT_IN,
        sound::CURRENT_OUT,
        "little-rapids/music/pause",
    ] {
        assert!(has(key), "no Ogg for {key}");
    }
}
