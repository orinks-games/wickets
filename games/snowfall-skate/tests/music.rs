//! The medley's choices: style by mood, never the same tune twice running,
//! the quiet carol in deep snow, and the new year's song only at new year.

use hearken::Headless;
use parlor_core::{Parlor, ParlorData};
use snowfall_skate::music::{bed_key, every_key, lead_key, sections, Medley, Mood, YEAR};

fn audio_host() -> Headless<Parlor> {
    Headless::new(ParlorData::default(), snowfall_skate::title())
}

/// Plays `n` sections in `mood`, giving each cue's bed.
fn play(seed: u64, mood: Mood, n: usize) -> Vec<String> {
    let mut h = audio_host();
    let audio = &mut h.app.ctx.audio;
    let mut m = Medley::new(seed);
    m.set_mood(mood);
    m.start(audio);
    while m.played.len() < n {
        m.update(audio, 0.5);
    }
    m.stop(audio, 0);
    m.played.iter().map(|c| c.bed.clone()).collect()
}

fn tune_of(bed: &str) -> &str {
    let name = bed.rsplit('/').next().unwrap();
    name.split(['-']).next().unwrap()
}

#[test]
fn every_key_names_each_section_in_each_style() {
    let keys = every_key();
    for s in sections() {
        assert!(keys.contains(&bed_key(s.name, "waltz")));
        assert!(keys.contains(&bed_key(s.name, "hush")));
        assert!(keys.contains(&bed_key(s.name, "band")));
        assert!(keys.contains(&lead_key(s.name, "music_box")));
    }
}

#[test]
fn clear_ice_plays_the_waltz_band_and_deep_snow_the_hush() {
    for bed in play(1, Mood::default(), 12) {
        assert!(bed.ends_with("-waltz"), "{bed}");
    }
    let deep = play(
        2,
        Mood {
            deep: true,
            ..Mood::default()
        },
        24,
    );
    assert!(deep.iter().all(|b| b.ends_with("-hush")), "{deep:?}");
    let still = deep.iter().filter(|b| tune_of(b) == "still").count();
    assert!(still * 3 >= deep.len(), "{deep:?}");
    for bed in play(
        3,
        Mood {
            fair: true,
            ..Mood::default()
        },
        12,
    ) {
        assert!(bed.ends_with("-band"), "{bed}");
    }
}

#[test]
fn a_tune_never_follows_itself() {
    let beds = play(4, Mood::default(), 60);
    // Each tune is at most two sections in a row; a tune right after
    // itself would show as more.
    let mut last = "";
    let mut count = 0;
    for b in &beds {
        let t = tune_of(b);
        if t == last {
            count += 1;
        } else {
            last = t;
            count = 1;
        }
        assert!(count <= 2, "{beds:?}");
    }
}

#[test]
fn the_new_years_song_plays_only_at_new_year() {
    let plain = play(5, Mood::default(), 80);
    assert!(!plain.iter().any(|b| tune_of(b) == YEAR.name));
    let year = play(
        5,
        Mood {
            new_year: true,
            ..Mood::default()
        },
        80,
    );
    assert!(year.iter().any(|b| tune_of(b) == YEAR.name), "{year:?}");
}
