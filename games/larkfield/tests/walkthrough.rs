//! Larkfield through the headless app: the title, a day in the hamlet,
//! supper, and the next morning kept.

use hearken::{Headless, Key};
use larkfield::rules::{Phase, Plot};
use larkfield::save::Progress;
use larkfield::title_with_seed;
use parlor_core::{Date, Parlor, ParlorData};

const SEED: u64 = 0x1a4c;
/// A day with no event on (not the third Tuesday nor the fourth Sunday).
const TODAY: Date = Date::new(2026, 11, 4);

fn data() -> ParlorData {
    ParlorData {
        today: Some(TODAY),
        ..Default::default()
    }
}

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

/// Reads pages until the hamlet is on top.
fn read_pages(h: &mut Headless<Parlor>) {
    for _ in 0..20 {
        if h.app.top_name() != "pages" {
            return;
        }
        h.press(Key::Return);
    }
}

#[test]
fn a_first_day_sows_a_plot_and_eats_supper() {
    let mut h = start();
    select(&mut h, "Play");
    h.press(Key::Return);
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "larkfield_hamlet");
    assert!(spoke(&h, "Work starts"), "{:?}", h.spoken());
    // To the fields, and into the first row, which is dug.
    h.clear();
    h.press(Key::Right);
    h.press(Key::Right);
    assert!(spoke(&h, "The fields"), "{:?}", h.spoken());
    h.press(Key::Up);
    assert!(spoke(&h, "Row 1, plot 1: dug"), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.app.top_name(), "larkfield_seeds", "{:?}", h.spoken());
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "larkfield_hamlet");
    assert!(spoke(&h, "sown"), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Space);
    assert!(spoke(&h, "Watered"), "{:?}", h.spoken());
    // S and H answer.
    h.clear();
    h.press(Key::S);
    assert!(!h.spoken().is_empty());
    h.clear();
    h.press(Key::H);
    assert!(!h.spoken().is_empty());
    // The day runs on to the supper bell.
    for _ in 0..20_000 {
        if h.app.top_name() != "larkfield_hamlet" {
            break;
        }
        h.frame(0.1);
    }
    assert_eq!(h.app.top_name(), "larkfield_supper");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "larkfield_hamlet");
    let p = Progress::peek(&h.app.ctx);
    let saved = p.hamlet.expect("the hamlet is kept");
    assert_eq!(saved.day, 2);
    assert_eq!(saved.phase, Phase::Morning, "kept at bedtime, before the next morning");
    assert!(saved
        .plots
        .iter()
        .any(|p| matches!(p, Plot::Sown { growth: 1.., .. })), "{:?}", saved.plots);
}

#[test]
fn every_key_answers_in_the_hamlet() {
    let mut h = start();
    select(&mut h, "Play");
    h.press(Key::Return);
    read_pages(&mut h);
    for key in [Key::Left, Key::Right, Key::Up, Key::Down, Key::Space, Key::Tab, Key::S, Key::H, Key::R, Key::Return, Key::L] {
        h.clear();
        h.press(key);
        if h.app.top_name() != "larkfield_hamlet" {
            h.press(Key::Escape);
            continue;
        }
        assert!(!h.spoken().is_empty() || !h.played().is_empty(), "{key:?} said nothing");
    }
    h.clear();
    h.press(Key::J);
    assert_eq!(h.app.top_name(), "larkfield_jobs");
    h.press(Key::Right);
    assert!(spoke(&h, "first job") || spoke(&h, ":"), "{:?}", h.spoken());
    h.press(Key::Escape);
    h.press(Key::P);
    assert_eq!(h.app.top_name(), "pages");
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "larkfield_hamlet");
}
