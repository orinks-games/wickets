//! The calendar: winter, the Frost Fair, the lantern skate's evenings and
//! the midnight skate.

use hearken::LocalTime;
use parlor_core::Date;
use snowfall_skate::events::{self, dusk, midnight_open, new_year, secs_to_midnight, tune};
use snowfall_skate::in_season;
use snowfall_skate::ponds::LANTERN_PAIRS;

fn at(text: &str) -> LocalTime {
    LocalTime::parse(text).unwrap()
}

#[test]
fn listed_from_december_to_february() {
    assert!(!in_season(Date::new(2026, 11, 30)));
    assert!(in_season(Date::new(2026, 12, 1)));
    assert!(in_season(Date::new(2027, 2, 28)));
    assert!(!in_season(Date::new(2027, 3, 1)));
}

#[test]
fn the_frost_fair_is_named_on_the_solstice_only() {
    assert!(events::event(Date::new(2026, 12, 21)).is_some());
    assert!(events::event(Date::new(2026, 12, 18)).is_none());
    assert!(events::event(Date::new(2026, 6, 21)).is_none());
}

#[test]
fn the_lantern_skate_is_lit_in_the_evening() {
    assert!(!dusk(15));
    assert!(dusk(16));
    assert!(dusk(20));
    assert!(!dusk(21));
}

#[test]
fn the_days_tune_is_the_same_for_everyone_and_ends_at_home() {
    let d = Date::new(2026, 12, 5);
    assert_eq!(tune(d), tune(d));
    assert_eq!(tune(d).len(), LANTERN_PAIRS as usize);
    assert_eq!(*tune(d).last().unwrap(), 0);
    assert!(tune(d).iter().all(|&n| n <= 7));
    assert_ne!(tune(d), tune(Date::new(2026, 12, 6)));
}

#[test]
fn the_midnight_skate_opens_at_eleven_on_new_years_eve() {
    assert!(!midnight_open(at("2026-12-31T22:59")));
    assert!(midnight_open(at("2026-12-31T23:00")));
    assert!(midnight_open(at("2027-01-01T00:30")));
    assert!(!midnight_open(at("2027-01-01T01:00")));
    assert_eq!(secs_to_midnight(at("2026-12-31T23:59")), 60);
    assert_eq!(secs_to_midnight(at("2026-12-31T23:00")), 3600);
    assert_eq!(secs_to_midnight(at("2027-01-01T00:10")), 0);
    assert!(new_year(Date::new(2026, 12, 31)));
    assert!(new_year(Date::new(2027, 1, 1)));
    assert!(!new_year(Date::new(2027, 1, 2)));
}
