//! Starting play keeps the ordinary game; seasonal finales keep their dates.
//! Doctor Volt's laboratory still waits for Halloween season.

use crate::common::*;
use graveyard_shuffle::play::{Mode, END_PAUSE};
use graveyard_shuffle::progress::{dated, in_season, Progress, SEASON_NIGHT};
use graveyard_shuffle::songs::NIGHTS;
use hearken::Key;
use parlor_core::Date;

/// A day in July, out of Halloween season.
const SUMMER: Date = Date::new(2027, 7, 10);

fn cleared(nights: &[u32]) -> Progress {
    let mut p = Progress::default();
    for &n in nights {
        for t in NIGHTS[n as usize - 1].songs() {
            p.stars.insert(t.id.to_string(), 1);
        }
    }
    p
}

#[test]
fn the_laboratory_opens_only_in_halloween_season() {
    assert!(in_season(Date::new(2026, 10, 17)));
    assert!(in_season(Date::new(2026, 11, 1)));
    assert!(!in_season(SUMMER));
    assert!(dated(3, SUMMER));
    assert!(!dated(SEASON_NIGHT, SUMMER));
    assert!(dated(SEASON_NIGHT, PLAIN_DAY));
    let p = cleared(&[1, 2, 3]);
    assert!(p.set_open(4), "earned");
    assert!(!p.night_open(4, SUMMER));
    assert!(p.night_open(4, PLAIN_DAY));
    assert_eq!(p.waiting(SUMMER), Some(4));
    assert_eq!(p.waiting(PLAIN_DAY), None);
    assert_eq!(p.current_set(SUMMER), 3);
    assert_eq!(p.current_set(PLAIN_DAY), 4);
}

#[test]
fn three_stars_on_night_1_keep_the_game() {
    assert!(!Progress::default().kept());
    assert!(cleared(&[1]).kept());
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    dance_all(&mut h, &mut twin);
    wait(&mut h, &mut twin, END_PAUSE + 0.5);
    assert!(
        spoke(&h, "Graveyard Shuffle is yours to keep"),
        "{:?}",
        h.spoken()
    );
    // In season the laboratory is open, so the line does not say it waits.
    assert!(!spoke(&h, "waits for Halloween"), "{:?}", h.spoken());
    assert!(graveyard_shuffle::kept(&mut h.app.ctx));
}

#[test]
fn out_of_season_the_kept_line_says_the_laboratory_waits() {
    use graveyard_shuffle::play::set_pages;
    use graveyard_shuffle::progress::SetSaved;
    let (h, run) = floor(Mode::Set(1));
    drop(h);
    let mut saved = SetSaved {
        score: parlor_core::ScoreResult::First,
        better: Vec::new(),
        opened: None,
        best_streak: 0,
        streak_record: false,
        next_open: None,
        waiting: None,
        kept: true,
        in_season: true,
    };
    assert!(!set_pages(1, &run, &saved)[0].contains("laboratory"));
    saved.in_season = false;
    assert!(set_pages(1, &run, &saved)[0].contains("Doctor Volt's laboratory waits for Halloween."));
}

#[test]
fn out_of_season_the_laboratory_and_its_songs_wait() {
    let mut h = title_on(SUMMER);
    cleared(&[1, 2, 3]).store(&mut h.app.ctx);
    h.app.ctx.pop();
    h.app.ctx.push(graveyard_shuffle::title());
    h.frame(0.0);
    assert_eq!(rows(&h)[0], "Play, night 3, the old church");
    goto(&mut h, "Night select");
    h.press(Key::Return);
    let r = rows(&h);
    assert!(
        r[3].starts_with("Night 4") && r[3].contains("locked"),
        "{r:?}"
    );
    goto(&mut h, "Night 4");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "opens only around Halloween"), "{:?}", h.spoken());
    h.press(Key::Escape);
    goto(&mut h, "Practice room");
    h.press(Key::Return);
    let r = rows(&h);
    assert!(r[8].starts_with("The Bell Tower Bop,"), "{r:?}");
    assert!(r[9].contains("locked"), "{r:?}");
}

#[test]
fn starting_play_or_practice_keeps_the_game_without_a_score() {
    for mode in [Mode::Set(1), Mode::Practice(0)] {
        let (mut h, mut twin) = floor(mode);
        assert!(
            !Progress::load(&mut h.app.ctx).kept(),
            "ready is not playing"
        );
        start(&mut h, &mut twin);
        let saved = Progress::load(&mut h.app.ctx);
        assert!(saved.played);
        assert!(saved.kept());
        assert!(saved.stars.is_empty(), "retention must not award stars");
        h.app.ctx.pop();
        h.frame(0.0);
        assert!(
            graveyard_shuffle::kept(&mut h.app.ctx),
            "abandoning still keeps the game"
        );
    }
}

#[test]
fn old_practice_progress_keeps_the_game_without_a_new_flag() {
    let old = Progress {
        practice_steps: 1,
        ..Default::default()
    };
    assert!(!old.played);
    assert!(old.kept());
}
