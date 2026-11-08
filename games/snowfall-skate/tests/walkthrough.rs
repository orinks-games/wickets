//! Snowfall Skate through the headless app: the title, a pond, its results,
//! and the event rows.

use hearken::{Headless, InputEvent, Key, LocalTime};
use parlor_core::{Date, Parlor, ParlorData, Records};
use snowfall_skate::progress::{pond_board, Progress};
use snowfall_skate::rules::lane_words;
use snowfall_skate::{sound, title_with_seed, ID};

const SEED: u64 = 0x5e1f;
const DT: f64 = 0.02;
/// A winter day with no event on.
const TODAY: Date = Date::new(2026, 12, 9);

fn data_at(today: Date, now: &str) -> ParlorData {
    ParlorData {
        today: Some(today),
        now: LocalTime::parse(now),
        ..Default::default()
    }
}

fn data() -> ParlorData {
    data_at(TODAY, "2026-12-09T11:00")
}

/// A fresh save's title, with the cursor moved from Tutorial to Play.
fn start_with(data: ParlorData) -> Headless<Parlor> {
    let mut h = Headless::new(data, title_with_seed(SEED));
    tap(&mut h, Key::Home);
    h.clear();
    h
}

fn rows(h: &Headless<Parlor>) -> Vec<String> {
    parlor_core::menus::rows_of(&h.app.top_lines())
}

/// A key pressed and let go.
fn tap(h: &mut Headless<Parlor>, key: Key) {
    h.press(key);
    h.send(InputEvent::key_up(key));
}

fn select(h: &mut Headless<Parlor>, label: &str) {
    let rows = rows(h);
    let at = rows
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row {label}: {rows:?}"));
    tap(h, Key::Home);
    for _ in 0..at {
        tap(h, Key::Down);
    }
}

fn spoke(h: &Headless<Parlor>, needle: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(needle))
}

fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

const LANES: [i32; 5] = [-2, -1, 0, 1, 2];

fn lane_of(words: &str) -> Option<i32> {
    LANES.into_iter().find(|&l| words.ends_with(lane_words(l)))
}

/// Skates until the screen closes, leaning toward the next lantern's lane
/// from the screen's lines and reading any page on the way.
fn skate(h: &mut Headless<Parlor>) {
    let mut held: Option<Key> = None;
    let mut pushed = false;
    for _ in 0..400_000 {
        match h.app.top_name() {
            "pages" => {
                if let Some(k) = held.take() {
                    h.send(InputEvent::key_up(k));
                }
                tap(h, Key::Return);
                continue;
            }
            "snowfall_skate_play" => {}
            _ => {
                if let Some(k) = held.take() {
                    h.send(InputEvent::key_up(k));
                }
                return;
            }
        }
        if !pushed {
            tap(h, Key::Space);
            pushed = true;
        }
        let lines = h.app.top_lines();
        let you = lines
            .iter()
            .find_map(|l| l.strip_prefix("You: "))
            .and_then(lane_of);
        let want = lines
            .iter()
            .find_map(|l| l.strip_prefix("Lantern "))
            .and_then(lane_of);
        let key = match (you, want) {
            (Some(y), Some(w)) if w < y => Some(Key::Left),
            (Some(y), Some(w)) if w > y => Some(Key::Right),
            _ => None,
        };
        if key != held {
            if let Some(k) = held {
                h.send(InputEvent::key_up(k));
            }
            if let Some(k) = key {
                h.send(InputEvent::key(k));
            }
            held = key;
        }
        h.frame(DT);
    }
    panic!("the skate never ended");
}

#[test]
fn the_title_opens_on_the_tutorial_the_first_time() {
    let h = Headless::new(data(), title_with_seed(SEED));
    assert!(spoke(&h, "Tutorial"), "{:?}", h.spoken());
    assert!(played(&h, sound::TITLE_MUSIC), "{:?}", h.played());
}

#[test]
fn the_village_pond_skates_to_the_results() {
    let mut h = start_with(data());
    select(&mut h, "Play, Pond 1");
    h.clear();
    tap(&mut h, Key::Return);
    assert_eq!(h.app.top_name(), "snowfall_skate_play");
    assert!(spoke(&h, "pushes off"), "{:?}", h.spoken());
    skate(&mut h);
    assert_eq!(h.app.top_name(), "results", "{:?}", h.spoken());
    assert!(Records::best(&mut h.app.ctx, ID, &pond_board(1)).is_some());
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.skates, 1);
    assert!(p.lanterns > 10, "{p:?}");
}

#[test]
fn status_and_hint_answer_on_the_ice() {
    let mut h = start_with(data());
    tap(&mut h, Key::Return);
    assert_eq!(h.app.top_name(), "snowfall_skate_play");
    h.clear();
    tap(&mut h, Key::H);
    assert!(spoke(&h, "A lantern"), "{:?}", h.spoken());
    tap(&mut h, Key::Space);
    for _ in 0..200 {
        h.frame(DT);
    }
    h.clear();
    tap(&mut h, Key::S);
    assert!(spoke(&h, "You,"), "{:?}", h.spoken());
    h.clear();
    tap(&mut h, Key::S);
    assert!(spoke(&h, "line"), "{:?}", h.spoken());
    h.clear();
    tap(&mut h, Key::Escape);
    assert_ne!(h.app.top_name(), "snowfall_skate_play");
}

#[test]
fn event_rows_are_locked_until_their_time() {
    let mut h = start_with(data());
    let r = rows(&h);
    assert!(
        r.iter().any(|r| r.starts_with("Lantern skate, locked")),
        "{r:?}"
    );
    assert!(
        r.iter().any(|r| r.starts_with("The Frost Fair, locked")),
        "{r:?}"
    );
    assert!(
        r.iter().any(|r| r.starts_with("Midnight skate, locked")),
        "{r:?}"
    );
    select(&mut h, "Lantern skate");
    h.clear();
    tap(&mut h, Key::Return);
    assert!(spoke(&h, "every evening"), "{:?}", h.spoken());
    assert_eq!(h.app.top_name(), "snowfall_skate");
}

#[test]
fn the_lantern_skate_opens_at_dusk_and_ends_after_its_pairs() {
    let mut h = start_with(data_at(TODAY, "2026-12-09T17:30"));
    select(&mut h, "Lantern skate, tonight");
    tap(&mut h, Key::Return);
    assert_eq!(h.app.top_name(), "snowfall_skate_play");
    skate(&mut h);
    assert_eq!(h.app.top_name(), "results", "{:?}", h.spoken());
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.lantern_skates, 1);
    assert!(p.lantern_today(TODAY).is_some());
}

#[test]
fn the_frost_fair_opens_on_the_solstice() {
    let day = Date::new(2026, 12, 21);
    let mut h = start_with(data_at(day, "2026-12-21T12:00"));
    select(&mut h, "The Frost Fair");
    tap(&mut h, Key::Return);
    assert_eq!(h.app.top_name(), "snowfall_skate_play");
    skate(&mut h);
    assert_eq!(h.app.top_name(), "results", "{:?}", h.spoken());
    let p = Progress::load(&mut h.app.ctx);
    assert!(p.fair_open(Date::new(2026, 12, 28)));
    assert!(!p.fair_open(Date::new(2027, 12, 28)));
}

#[test]
fn the_midnight_skate_ends_in_fireworks() {
    let mut h = start_with(data_at(Date::new(2026, 12, 31), "2026-12-31T23:59"));
    select(&mut h, "Midnight skate");
    tap(&mut h, Key::Return);
    assert_eq!(h.app.top_name(), "snowfall_skate_play");
    skate(&mut h);
    assert!(
        played(&h, "liberty-lights/fireworks/boom"),
        "{:?}",
        h.played()
    );
    assert_eq!(h.app.top_name(), "results", "{:?}", h.spoken());
}

#[test]
fn control_help_uses_alternative_lean_keys() {
    let mut h = start_with(data());
    tap(&mut h, Key::Return);
    h.clear();
    tap(&mut h, Key::F1);
    let spoken = h.spoken().join(" ");
    let left = h.app.ctx.control_name(parlor_core::ParlorAction::Left);
    let right = h.app.ctx.control_name(parlor_core::ParlorAction::Right);
    assert!(
        spoken.contains(&format!("Hold {left} or {right}")),
        "{spoken}"
    );
    assert!(
        !spoken.contains(&format!("Hold {left} and {right}")),
        "{spoken}"
    );
}
