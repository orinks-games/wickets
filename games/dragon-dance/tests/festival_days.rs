//! The real calendar through the screens: the run-up, the New Year
//! parade, the Lantern walk, and years past the calendar's table.

use crate::common::*;
use dragon_dance::dance::DANCE_NAME;
use dragon_dance::progress::Progress;
use dragon_dance::{sound, Mode, LANTERN_ROW, NEW_YEAR_ROW, TITLE_NAME};
use hearken::Key;
use parlor_core::Date;

/// Lunar New Year 2027, the Year of the Goat.
const NEW_YEAR: Date = Date::new(2027, 2, 6);
/// Five days before it.
const RUN_UP: Date = Date::new(2027, 2, 1);
/// The Lantern Festival of 2027.
const LANTERNS: Date = Date::new(2027, 2, 20);

#[test]
fn the_festival_rows_are_locked_on_an_ordinary_day_and_say_when() {
    let mut h = title();
    goto(&mut h, NEW_YEAR_ROW);
    assert_eq!(cursor_row(&h), "New Year parade, locked");
    h.clear();
    h.press(Key::F1);
    assert!(spoke(
        &h,
        "Opens for Lunar New Year, on February 6, in 107 days: the Year of the Goat."
    ));
    h.clear();
    h.press(Key::Return);
    assert!(played(&h, sound::NOPE));
    assert!(spoke(&h, "in 107 days"));
    assert_eq!(h.app.top_name(), TITLE_NAME);
    goto(&mut h, LANTERN_ROW);
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "on February 20, in 121 days."));
    assert_eq!(dragon_dance::event(ORDINARY), None);
}

#[test]
fn the_run_up_hangs_lanterns_and_brings_the_lion_drums() {
    let mut h = title_on(RUN_UP);
    assert!(played(&h, sound::FESTIVE_MUSIC));
    h.clear();
    h.press(Key::F1);
    assert!(spoke(
        &h,
        "Lanterns are up and lion-dance drums play along."
    ));
    assert!(spoke(&h, "Lunar New Year is in 5 days."));
    assert_eq!(rows(&h)[2], "New Year parade, locked");
    let event = (dragon_dance::INFO.event.unwrap())(RUN_UP).unwrap();
    assert_eq!(event.title, "Lanterns going up");
    h.press(Key::Home);
    h.clear();
    start(&mut h);
    assert!(spoke(&h, "Lanterns are up: red envelopes count double."));
    wait(&mut h, 8.0);
    assert!(played(&h, sound::LION));
}

#[test]
fn new_years_day_opens_the_parade_with_the_animals_greeting() {
    let mut h = title_on(NEW_YEAR);
    assert_eq!(rows(&h)[2], "New Year parade");
    let event = dragon_dance::event(NEW_YEAR).unwrap();
    assert_eq!(event.title, "New Year parade");
    assert!(event.line.contains("Year of the Goat"));
    goto(&mut h, NEW_YEAR_ROW);
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "The Year of the Goat parade:"));
    h.clear();
    start(&mut h);
    assert_eq!(h.app.top_name(), DANCE_NAME);
    assert!(spoke(&h, "The New Year parade, the Year of the Goat."));
    assert!(spoke(&h, "The Goat: Take your time."));
    assert!(played(&h, sound::GREETING));
}

#[test]
fn the_new_year_parade_ends_at_midnight_with_a_keepsake() {
    let mut h = dance_on(NEW_YEAR, Mode::NewYear(2027));
    let lines = dance_by_keys(&mut h, 400.0);
    let at = |line: &str| {
        lines
            .iter()
            .rposition(|l| l == line)
            .unwrap_or_else(|| panic!("{line}"))
    };
    assert!(at("5.") < at("4.") && at("4.") < at("3.") && at("2.") < at("1."));
    let heard = lines.join(" ");
    assert!(heard.contains("Midnight. Happy New Year, and welcome to the Year of the Goat."));
    assert!(heard.contains("for the new year."), "{heard}");
    assert_eq!(h.app.top_name(), "results");
    let progress = Progress::load(&mut h.app.ctx);
    assert_eq!(progress.keepsakes.get("Goat"), Some(&2027));
    assert!(progress.parades.contains(&2027));
    h.press(Key::Down);
    h.press(Key::Return);
    for _ in 0..2 {
        h.press(Key::Return);
    }
    assert!(spoke(
        &h,
        "Keepsake for the Year of the Goat: a woollen goat with curled horns. 1 of 12 animals collected."
    ));
}

#[test]
fn the_eve_and_the_day_after_count_as_new_years_day() {
    for date in [Date::new(2027, 2, 5), Date::new(2027, 2, 7)] {
        let h = title_on(date);
        assert_eq!(rows(&h)[2], "New Year parade", "{date:?}");
    }
    let h = title_on(Date::new(2027, 2, 8));
    assert_eq!(rows(&h)[2], "New Year parade, locked");
}

#[test]
fn the_lantern_walk_opens_on_the_fifteenth_day() {
    let mut h = title_on(LANTERNS);
    assert_eq!(rows(&h)[3], "Lantern walk");
    assert_eq!(rows(&h)[2], "New Year parade, locked");
    goto(&mut h, LANTERN_ROW);
    start(&mut h);
    assert_eq!(h.app.top_name(), DANCE_NAME);
    assert!(played(&h, sound::LANTERN_BED));
    assert!(played(&h, sound::NIGHT));
    let heard = dance_by_keys(&mut h, 200.0).join(" ");
    assert!(heard.contains("new for your collection"), "{heard}");
    assert!(!heard.contains("Firecrackers"));
    assert_eq!(h.app.top_name(), "results");
    let progress = Progress::load(&mut h.app.ctx);
    assert!(!progress.lanterns.is_empty());
    assert!(progress.walks.contains(&2027));
    let event = dragon_dance::event(LANTERNS).unwrap();
    assert_eq!(event.title, "Lantern walk");
}

#[test]
fn years_past_the_calendar_keep_the_festival_rows_locked_and_say_why() {
    let date = Date::new(2041, 2, 1);
    let mut h = title_on(date);
    assert_eq!(rows(&h)[2], "New Year parade, locked");
    assert_eq!(rows(&h)[3], "Lantern walk, locked");
    assert!(played(&h, sound::TITLE_MUSIC));
    goto(&mut h, NEW_YEAR_ROW);
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "Lunar New Year dates are known only up to 2040"));
    assert_eq!(dragon_dance::event(date), None);
}

#[test]
fn keepsakes_list_the_animals_collected_and_still_to_come() {
    let mut h = title();
    let mut progress = Progress::default();
    progress.add_keepsake("Horse", 2026);
    progress.add_keepsake("Goat", 2027);
    assert!(!progress.add_keepsake("Goat", 2039));
    progress.lanterns.insert(2);
    progress.store(&mut h.app.ctx);
    goto(&mut h, "Keepsakes");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "Keepsakes, 2 of 12. The Year of the Horse, 2026: a paper-cut galloping horse. The Year of the Goat, 2027: a woollen goat with curled horns."
    ));
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "Still to come: Rat, Ox, Tiger, Rabbit, Dragon, Snake, Monkey"
    ));
    h.press(Key::Return);
    assert!(spoke(&h, "Lanterns, 1 of 12: carp lantern."));
}
