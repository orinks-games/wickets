//! Every sound the game asks for has an Ogg.

use std::path::Path;

use larkfield::music::{bed_key, part_key, Chord, VARIANTS};
use larkfield::people::INSTRUMENTS;
use larkfield::sound;

fn has(key: &str) -> bool {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    root.join(format!("{key}.ogg")).exists()
}

#[test]
fn every_phrase_is_rendered() {
    for chord in Chord::ALL {
        for n in 0..VARIANTS {
            assert!(has(&bed_key(chord, n)), "no Ogg for {}", bed_key(chord, n));
            for i in 0..INSTRUMENTS.len() {
                let key = part_key(i, chord, n);
                assert!(has(&key), "no Ogg for {key}");
            }
        }
    }
}

#[test]
fn every_cue_and_loop_is_rendered() {
    for id in sound::WORK {
        assert!(has(&sound::work(id)), "no Ogg for work cue {id}");
    }
    for key in [
        sound::TITLE_MUSIC,
        sound::SUPPER_MUSIC,
        sound::PAUSE_MUSIC,
        sound::DAY,
        sound::RAIN,
        sound::WINTER,
        sound::RIVER,
        sound::STEP_LANE,
        sound::STEP_IN,
        sound::STEP_FIELD,
        sound::BITE,
        sound::MORNING,
        sound::SUPPER_BELL,
        sound::BUILT,
        sound::ARRIVED,
        sound::KEEPSAKE,
        sound::GOAL,
        "larkfield/menu/move",
        "larkfield/menu/select",
        "larkfield/menu/back",
    ] {
        assert!(has(key), "no Ogg for {key}");
    }
}
