//! Valentine's Day on the calendar: what opens when, across several years.

use cupids_quiver::valentine::{
    self, countdown, days_until, event, gate, keepsake_item, keepsake_year, locked_help,
    night_open, roses_on, Gate,
};
use parlor_core::Date;

fn d(y: i32, m: u32, day: u32) -> Date {
    Date::new(y, m, day)
}

#[test]
fn the_day_holds_one_day_either_side() {
    for year in [2026, 2027, 2028, 2031, 2040] {
        assert_eq!(gate(d(year, 2, 13)), Gate::Day, "{year}");
        assert_eq!(gate(d(year, 2, 14)), Gate::Day, "{year}");
        assert_eq!(gate(d(year, 2, 15)), Gate::Day, "{year}");
        assert!(night_open(d(year, 2, 14)));
        assert!(!night_open(d(year, 2, 16)), "{year}");
        assert!(!night_open(d(year, 2, 12)), "{year}");
    }
}

#[test]
fn the_season_is_the_week_before() {
    for year in [2026, 2027, 2028, 2035] {
        assert_eq!(gate(d(year, 2, 7)), Gate::Season { days: 7 }, "{year}");
        assert_eq!(gate(d(year, 2, 12)), Gate::Season { days: 2 }, "{year}");
        assert_eq!(gate(d(year, 2, 6)), Gate::Off { days: 8 }, "{year}");
        assert!(roses_on(d(year, 2, 7)));
        assert!(roses_on(d(year, 2, 14)), "roses on the day too");
        assert!(!roses_on(d(year, 2, 6)));
        assert!(!roses_on(d(year, 2, 16)));
    }
}

#[test]
fn the_rest_of_the_year_counts_down_across_new_year() {
    assert_eq!(gate(d(2026, 2, 16)), Gate::Off { days: 363 });
    assert_eq!(gate(d(2026, 10, 22)), Gate::Off { days: 115 });
    assert_eq!(gate(d(2026, 12, 31)), Gate::Off { days: 45 });
    assert_eq!(gate(d(2027, 1, 1)), Gate::Off { days: 44 });
    // 2028 is a leap year: 29 February counts.
    assert_eq!(gate(d(2028, 2, 29)), Gate::Off { days: 351 });
    assert_eq!(days_until(d(2027, 2, 14)), 0);
    assert_eq!(days_until(d(2027, 2, 15)), -1);
}

#[test]
fn locked_rows_say_how_long_to_wait() {
    assert_eq!(
        countdown(d(2026, 10, 22)),
        "Valentine's Day is in 115 days."
    );
    assert_eq!(countdown(d(2027, 2, 13)), "Valentine's Day is tomorrow.");
    assert_eq!(
        locked_help(d(2027, 1, 1)),
        "Opens on Valentine's Day, the 14th of February. Valentine's Day is in 44 days."
    );
}

#[test]
fn the_launcher_hears_of_each_event() {
    let night = event(d(2027, 2, 14)).expect("an event on the day");
    assert_eq!(night.title, "Valentine's night");
    assert!(night.line.ends_with('.'));
    let roses = event(d(2027, 2, 10)).expect("an event in the season");
    assert_eq!(roses.title, "Roses in the sky");
    assert!(roses.line.contains("in 4 days"), "{}", roses.line);
    assert_eq!(event(d(2027, 3, 1)), None);
    assert_eq!(event(d(2026, 12, 31)), None);
    assert_eq!(
        cupids_quiver::INFO.event.map(|e| e(d(2027, 2, 13))),
        Some(Some(night))
    );
}

#[test]
fn keepsakes_file_under_the_day_they_count_for() {
    assert_eq!(keepsake_year(d(2027, 2, 13)), 2027);
    assert_eq!(keepsake_year(d(2027, 2, 15)), 2027);
    assert_eq!(keepsake_year(d(2028, 2, 14)), 2028);
    assert_ne!(keepsake_item(2027), keepsake_item(2028));
    assert_eq!(keepsake_item(2024), keepsake_item(2032));
    assert_eq!(valentine::HOLIDAY.name(), "Valentine's Day");
}
