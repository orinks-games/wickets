//! Little Rapids through the headless app: the title, a stream, its results.

use hearken::{Headless, InputEvent, Key};
use little_rapids::progress::Progress;
use little_rapids::{sound, title_with_seed};
use parlor_core::{Date, Parlor, ParlorData, Records};

const SEED: u64 = 0x5a11;
const DT: f64 = 0.02;
/// A day with no event on (not the second Monday nor the third Friday).
const TODAY: Date = Date::new(2026, 11, 4);

fn data() -> ParlorData {
    ParlorData {
        today: Some(TODAY),
        ..Default::default()
    }
}

/// A fresh save's title, with the cursor moved from Tutorial to Play.
fn start() -> Headless<Parlor> {
    let mut h = Headless::new(data(), title_with_seed(SEED));
    h.press(Key::Home);
    h.clear();
    h
}

fn select(h: &mut Headless<Parlor>, label: &str) {
    let rows = parlor_core::menus::rows_of(&h.app.top_lines());
    let at = rows
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row {label}: {rows:?}"));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

fn spoke(h: &Headless<Parlor>, needle: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(needle))
}

fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

fn nums(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|t| t.parse().ok())
        .collect()
}

/// Where the bot wants the boat, from the run screen's lines.
fn want(lines: &[String]) -> (f64, f64) {
    let boat = nums(&lines[1]);
    let (x, current) = (boat[0], boat[1]);
    let mut next: Option<(f64, String, f64)> = None;
    for line in &lines[3..] {
        let rest = line.trim_start_matches("coming: ");
        let kind = rest.split(' ').next().unwrap().to_string();
        let n = nums(rest);
        let (at, left) = (n[0], n[1]);
        if left > -0.1 && left < 3.0 && next.as_ref().is_none_or(|b| left < b.2) {
            next = Some((at, kind, left));
        }
    }
    let target = match next {
        None => current,
        Some((at, kind, _)) => match kind.as_str() {
            "stone" | "duck" => {
                if at > 0.0 {
                    at - 1.6
                } else {
                    at + 1.6
                }
            }
            "mill" => -at.signum() * 1.5,
            _ => at,
        },
    };
    (x, target)
}

/// Sails until the results open, steering with held arrow keys and
/// reading any page that opens on the way.
fn sail(h: &mut Headless<Parlor>) {
    let mut held: Option<Key> = None;
    for _ in 0..200_000 {
        match h.app.top_name() {
            "pages" => {
                if let Some(k) = held.take() {
                    h.send(InputEvent::key_up(k));
                }
                h.press(Key::Return);
                continue;
            }
            "little_rapids_run" => {}
            _ => {
                if let Some(k) = held.take() {
                    h.send(InputEvent::key_up(k));
                }
                return;
            }
        }
        let lines = h.app.top_lines();
        if lines[0].contains("ready") {
            h.press(Key::Space);
        }
        let (x, target) = want(&lines);
        let key = if target > x + 0.15 {
            Some(Key::Right)
        } else if target < x - 0.15 {
            Some(Key::Left)
        } else {
            None
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
    panic!("the run never ended");
}

#[test]
fn the_title_opens_on_the_tutorial_the_first_time() {
    let h = Headless::new(data(), title_with_seed(SEED));
    assert!(spoke(&h, "Tutorial"), "{:?}", h.spoken());
    assert!(played(&h, sound::TITLE_MUSIC), "{:?}", h.played());
}

#[test]
fn garden_brook_runs_from_the_launch_to_the_results() {
    let mut h = start();
    h.press(Key::Return); // Play
    assert_eq!(h.app.top_name(), "little_rapids_streams");
    assert!(spoke(&h, "Garden brook"), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Return); // Garden brook
    assert_eq!(h.app.top_name(), "little_rapids_run");
    assert!(spoke(&h, "starts"), "{:?}", h.spoken());
    sail(&mut h);
    assert_eq!(h.app.top_name(), "results");
    assert!(spoke(&h, "River mouth"), "{:?}", h.spoken());
    let p = Progress::load(&mut h.app.ctx);
    assert!(p.stars_for(little_rapids::rules::Stream::GardenBrook) >= 2, "{:?}", h.spoken());
    assert!(p.endless_open());
    assert!(Records::best(&mut h.app.ctx, little_rapids::ID, "garden").is_some());
    assert!(spoke(&h, "endless river is open"), "{:?}", h.spoken());
}

#[test]
fn status_and_hint_answer_on_the_water() {
    let mut h = start();
    h.press(Key::Return);
    h.press(Key::Return);
    h.press(Key::Space);
    for _ in 0..100 {
        h.frame(DT);
    }
    while h.app.top_name() == "pages" {
        h.press(Key::Return);
    }
    h.clear();
    h.press(Key::S);
    assert!(spoke(&h, "Combo"), "{:?}", h.spoken());
    h.clear();
    h.press(Key::H);
    assert!(!h.spoken().is_empty());
    h.clear();
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "little_rapids_run");
}

#[test]
fn the_endless_river_is_locked_until_a_garden_star() {
    let mut h = start();
    select(&mut h, "The endless river");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "first star on Garden brook"), "{:?}", h.spoken());
    assert_eq!(h.app.top_name(), "little_rapids");
}

#[test]
fn drift_sails_to_the_mouth_with_no_score() {
    let mut h = start();
    select(&mut h, "Drift");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "little_rapids_run");
    sail(&mut h);
    assert_eq!(h.app.top_name(), "results");
    assert!(spoke(&h, "Nothing is scored"), "{:?}", h.spoken());
}
