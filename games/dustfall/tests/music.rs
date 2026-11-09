//! The score's choices: the parts follow the pressure, no phrase twice
//! running, the colony bed thins when hungry, every key is rendered, and
//! the soundtrack performance ends on its own.

use std::path::Path;

use dustfall::music::{
    colony_key, every_key, score_key, stab_key, Conductor, Part, Performance, Pressure, Stab,
    CHORDS, CYCLE, PAD_ROW, PLUCK_ROW,
};
use hearken::{Headless, Performer, Variety};
use parlor_core::{Parlor, ParlorData};

fn audio_host() -> Headless<Parlor> {
    Headless::new(ParlorData::default(), dustfall::title())
}

fn part_of(key: &str) -> &str {
    key.rsplit('/').next().unwrap().split('-').next().unwrap()
}

fn chord_of(key: &str) -> &str {
    key.rsplit('/')
        .next()
        .unwrap()
        .split('-')
        .nth(1)
        .unwrap_or("")
}

#[test]
fn the_parts_follow_the_pressure() {
    assert_eq!(Pressure::default().parts(), vec![Part::Bass]);
    let far = Pressure {
        near: Some(PAD_ROW + 1),
        ..Pressure::default()
    };
    assert_eq!(far.parts(), vec![Part::Bass]);
    let close = Pressure {
        near: Some(PAD_ROW),
        ..Pressure::default()
    };
    assert_eq!(close.parts(), vec![Part::Bass, Part::Pad]);
    let at_the_dome = Pressure {
        near: Some(PLUCK_ROW),
        charge: true,
        ..Pressure::default()
    };
    assert_eq!(
        at_the_dome.parts(),
        vec![Part::Bass, Part::Pad, Part::Pluck, Part::Danger]
    );
    let shield_down = Pressure {
        shield_down: true,
        near: Some(PAD_ROW),
        ..Pressure::default()
    };
    assert_eq!(
        shield_down.parts(),
        vec![Part::Bass, Part::Pad],
        "a bare shield alone is not danger"
    );
    let exposed = Pressure {
        shield_down: true,
        near: Some(PLUCK_ROW),
        ..Pressure::default()
    };
    assert_eq!(
        exposed.parts(),
        vec![Part::Bass, Part::Pad, Part::Pluck, Part::Danger]
    );
}

#[test]
fn a_night_walks_the_cycle_and_never_repeats_a_phrase_twice_running() {
    let mut h = audio_host();
    let audio = &mut h.app.ctx.audio;
    let mut c = Conductor::night(7);
    c.set_pressure(Pressure {
        near: Some(1),
        charge: true,
        shield_down: false,
    });
    c.start(audio);
    while c.bars() < 24 {
        c.update(audio, 0.25);
    }
    c.stop(audio, 0);
    assert_eq!(c.played.len(), 24);
    for (i, bar) in c.played.iter().enumerate() {
        assert_eq!(bar.len(), 4, "{bar:?}");
        let chord = CYCLE[i % CYCLE.len()];
        for key in bar.iter().take(3) {
            assert_eq!(chord_of(key), chord, "{key}");
        }
        assert_eq!(bar[3], score_key(Part::Danger, "", 0));
    }
    for pair in c.played.windows(2) {
        for part in 0..3 {
            assert_ne!(pair[0][part], pair[1][part], "{pair:?}");
        }
    }
    let bass: Vec<&str> = c.played.iter().map(|b| part_of(&b[0])).collect();
    assert!(bass.iter().all(|p| *p == "bass"), "{bass:?}");
}

#[test]
fn pressure_changes_take_effect_at_the_next_bar() {
    let mut h = audio_host();
    let audio = &mut h.app.ctx.audio;
    let mut c = Conductor::night(3);
    c.start(audio);
    assert_eq!(c.played[0].len(), 1, "the bass alone at first");
    c.set_pressure(Pressure {
        near: Some(PAD_ROW),
        ..Pressure::default()
    });
    c.update(audio, 0.1);
    assert_eq!(c.bars(), 1, "the pad waits for the bar");
    while c.bars() < 2 {
        c.update(audio, 0.25);
    }
    assert_eq!(c.played[1].len(), 2);
    assert_eq!(part_of(&c.played[1][1]), "pad");
    c.stop(audio, 0);
}

#[test]
fn the_colony_bed_is_bass_and_pad_and_bass_alone_when_hungry() {
    let mut h = audio_host();
    let audio = &mut h.app.ctx.audio;
    let mut c = Conductor::colony(5);
    assert!(c.bar_secs() > Conductor::night(5).bar_secs() * 1.9);
    c.start(audio);
    assert_eq!(
        c.played[0],
        vec![colony_key(Part::Bass, "Dm"), colony_key(Part::Pad, "Dm")]
    );
    c.set_hungry(true);
    while c.bars() < 2 {
        c.update(audio, 0.5);
    }
    assert_eq!(c.played[1], vec![colony_key(Part::Bass, "Bb")]);
    c.stop(audio, 0);
}

#[test]
fn a_stab_takes_the_bars_chord() {
    for chord in CHORDS {
        assert_eq!(
            stab_key(Stab::Heavy, chord),
            format!("dustfall/music/score/heavy-{chord}")
        );
    }
    let mut h = audio_host();
    let audio = &mut h.app.ctx.audio;
    let mut c = Conductor::night(1);
    c.start(audio);
    assert_eq!(c.chord(), "Dm");
    c.stab(audio, Stab::Run);
    c.stop(audio, 0);
}

#[test]
fn every_key_is_rendered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    let keys = every_key();
    assert_eq!(keys.len(), 73, "{}", keys.len());
    for key in keys {
        let ogg = root.join(format!("{key}.ogg"));
        assert!(ogg.exists(), "missing {}", ogg.display());
    }
}

#[test]
fn the_performance_ends_on_its_own() {
    let mut h = audio_host();
    let audio = &mut h.app.ctx.audio;
    let mut p = Performance::new(Variety::new(11));
    p.start(audio);
    let mut secs = 0.0;
    while p.update(audio, 0.25) {
        secs += 0.25;
        assert!(secs < 240.0, "the performance never ended");
    }
    assert!(secs > 60.0, "{secs}");
    p.stop(audio);
}
