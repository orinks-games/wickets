//! Saint Patrick's Day on the real calendar, across several years.

use clover_jig::season::*;
use parlor_core::Date;

fn d(y: i32, m: u32, day: u32) -> Date {
    Date::new(y, m, day)
}

#[test]
fn the_festival_follows_the_calendar_every_year() {
    for y in 2026..=2032 {
        assert_eq!(festival(d(y, 3, 9)), Festival::Off, "{y}");
        assert_eq!(
            festival(d(y, 3, 10)),
            Festival::Shamrocks { days: 7 },
            "{y}"
        );
        assert_eq!(
            festival(d(y, 3, 15)),
            Festival::Shamrocks { days: 2 },
            "{y}"
        );
        for day in 16..=18 {
            assert_eq!(festival(d(y, 3, day)), Festival::Ceili, "{y}-03-{day}");
            assert!(ceili_open(d(y, 3, day)));
            assert!(shamrocks_on(d(y, 3, day)));
        }
        assert_eq!(festival(d(y, 3, 19)), Festival::Off, "{y}");
        assert!(!ceili_open(d(y, 3, 15)));
        assert!(shamrocks_on(d(y, 3, 12)));
        assert!(!shamrocks_on(d(y, 6, 1)));
        assert!(!shamrocks_on(d(y, 12, 31)));
        assert!(!shamrocks_on(d(y, 1, 1)));
    }
}

#[test]
fn the_launcher_hears_the_event() {
    assert_eq!(event(d(2026, 10, 22)), None);
    let week = event(d(2027, 3, 12)).expect("shamrock week");
    assert_eq!(week.title, "Shamrock week");
    let night = event(d(2027, 3, 17)).expect("ceili night");
    assert_eq!(night.title, "Ceili night");
    assert!(night.line.contains("four-leaf clover"));
    assert_eq!(
        event(d(2027, 3, 18)).map(|h| h.title),
        Some("Ceili night".to_string())
    );
    assert_eq!(event(d(2027, 3, 19)), None);
    let info = clover_jig::INFO.event.expect("the game holds events");
    assert_eq!(info(d(2028, 3, 16)), event(d(2028, 3, 16)));
}

#[test]
fn the_locked_row_counts_down_across_the_new_year() {
    assert_eq!(
        countdown(d(2026, 10, 22)),
        "Saint Patrick's Day is in 146 days."
    );
    assert_eq!(
        countdown(d(2026, 12, 31)),
        "Saint Patrick's Day is in 76 days."
    );
    assert_eq!(
        countdown(d(2027, 1, 1)),
        "Saint Patrick's Day is in 75 days."
    );
    assert_eq!(
        countdown(d(2027, 3, 19)),
        "Saint Patrick's Day is in 364 days."
    );
    // 2028 is a leap year: February has 29 days.
    assert_eq!(
        countdown(d(2028, 2, 28)),
        "Saint Patrick's Day is in 18 days."
    );
    assert_eq!(
        countdown(d(2028, 3, 1)),
        "Saint Patrick's Day is in 16 days."
    );
    assert_eq!(
        countdown(d(2027, 3, 16)),
        "Saint Patrick's Day is tomorrow."
    );
    assert!(ceili_help(d(2026, 10, 22)).ends_with("Saint Patrick's Day is in 146 days."));
    assert!(ceili_help(d(2027, 3, 17)).starts_with("The village ceili, open tonight"));
}

#[test]
fn the_play_row_names_shamrock_week() {
    assert_eq!(shamrock_line(d(2026, 10, 22)), None);
    assert_eq!(
        shamrock_line(d(2027, 3, 10)).unwrap(),
        "Shamrock week: shamrocks hide in the tunes for 9 more days."
    );
    assert!(shamrock_line(d(2027, 3, 17))
        .unwrap()
        .starts_with("Saint Patrick's Day"));
}

#[test]
fn nights_and_keepsake_years() {
    assert_eq!(night_key(d(2027, 3, 17)), "2027-03-17");
    assert_ne!(night_key(d(2027, 3, 17)), night_key(d(2027, 3, 18)));
    assert_eq!(keepsake_year(d(2027, 3, 16)), 2027);
    assert_eq!(keepsake_year(d(2031, 3, 18)), 2031);
}
