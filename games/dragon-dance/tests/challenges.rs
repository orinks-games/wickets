//! The daily, weekly and monthly challenges, with the date pinned.

use crate::common::*;
use dragon_dance::challenge::{self, CHALLENGES_NAME};
use dragon_dance::dance::DANCE_NAME;
use dragon_dance::rules::plan;
use dragon_dance::{DanceScreen, ID, TITLE_NAME};
use hearken::{Headless, Key};
use parlor_core::{Challenge, Challenges, Date, Parlor, Period};

const DAY: Date = Date::new(2026, 10, 21);

/// The challenges menu, from the title.
fn menu() -> Headless<Parlor> {
    let mut h = title_on(DAY);
    goto(&mut h, "Challenges");
    h.press(Key::Return);
    h
}

/// Opens `period`'s briefing from the menu and reads it to the start.
fn start(h: &mut Headless<Parlor>, period: Period) {
    goto(h, period.label());
    h.press(Key::Return);
    while h.app.top_name() == "pages" {
        h.press(Key::Return);
    }
    assert_eq!(h.app.top_name(), DANCE_NAME);
}

#[test]
fn the_challenges_menu_lists_three_then_back() {
    let h = menu();
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    assert_eq!(
        rows(&h),
        [
            "Daily, Today's street: 4 lanes, 24 bars. Not played yet",
            "Weekly, The long parade: 4 lanes, 48 bars, a gong every other bar. Not played yet",
            "Monthly, The grand festival: 5 lanes, quick drums, firecrackers close behind. Not played yet",
            "Back",
        ]
    );
}

#[test]
fn a_challenge_is_the_same_street_all_day_and_new_the_next() {
    let ch = Challenge::on(DAY, ID, Period::Daily);
    let a = DanceScreen::challenge(ch.clone()).dance.plan;
    let b = DanceScreen::challenge(ch.clone()).dance.plan;
    assert_eq!(a, b);
    assert_eq!(a, plan(&challenge::street(Period::Daily), ch.seed));
    let next = Challenge::on(Date::new(2026, 10, 22), ID, Period::Daily);
    assert_ne!(a, DanceScreen::challenge(next).dance.plan);
}

#[test]
fn a_finished_challenge_is_submitted_and_shown() {
    let mut h = menu();
    start(&mut h, Period::Daily);
    assert!(spoke(&h, "Today's street. 4 lanes, 24 bars"));
    dance_by_keys(&mut h, 200.0);
    assert_eq!(h.app.top_name(), "results");
    goto(&mut h, "Hear the results again");
    h.press(Key::Return);
    h.press(Key::Return);
    h.press(Key::Return);
    assert!(spoke(&h, "Daily challenge result:"));
    assert!(spoke(
        &h,
        "Your first result on this challenge. A 1 day streak."
    ));
    h.press(Key::Return);
    let ch = Challenge::on(DAY, ID, Period::Daily);
    let best = Challenges::best(&mut h.app.ctx, &ch).expect("submitted");
    assert!(best > 0);
    assert_eq!(rows(&h)[0], "Play this daily challenge again");
    assert_eq!(rows(&h).last().unwrap(), "Back to the challenges");
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    assert!(rows(&h)[0].ends_with(&format!("Done, best {best}, a 1 day streak")));
}

#[test]
fn leaving_a_challenge_from_pause_does_not_count_it() {
    let mut h = menu();
    start(&mut h, Period::Monthly);
    wait(&mut h, 3.0);
    h.press(Key::Escape);
    goto(&mut h, "Leave the challenge");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    let ch = Challenge::on(DAY, ID, Period::Monthly);
    assert!(!Challenges::done(&mut h.app.ctx, &ch));
}

#[test]
fn briefing_escape_goes_back_without_starting() {
    let mut h = menu();
    goto(&mut h, "Weekly");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "The long parade:"));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
}

#[test]
fn the_launcher_opens_a_challenge_and_gets_it_back() {
    let mut h = base_on(DAY);
    dragon_dance::open_challenge(&mut h.app.ctx, Period::Weekly);
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "pages");
    while h.app.top_name() == "pages" {
        h.press(Key::Return);
    }
    assert_eq!(h.app.top_name(), DANCE_NAME);
    h.press(Key::Escape);
    goto(&mut h, "Leave the challenge");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "base");
}

#[test]
fn back_on_the_menu_returns_to_the_title() {
    let mut h = menu();
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}
