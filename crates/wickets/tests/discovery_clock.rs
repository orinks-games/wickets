use parlor_core::challenges::Date;
use std::time::Duration;
use wickets::discovery_clock::{Anchor, MAX_AGE, MAX_DRIFT};

#[test]
fn server_date_advances_over_a_season_boundary_only_with_elapsed_time() {
    let anchor = Anchor::from_http_date("Sun, 15 Nov 2026 23:59:59 GMT").unwrap();
    assert_eq!(
        anchor.date_after(Duration::ZERO),
        Some(Date::new(2026, 11, 15))
    );
    assert_eq!(
        anchor.date_after(Duration::from_secs(1)),
        Some(Date::new(2026, 11, 16))
    );
    // Repeating the same elapsed value is deterministic, regardless of the
    // PC's date, timezone or any intervening wall-clock adjustment.
    assert_eq!(
        anchor.date_after(Duration::ZERO),
        Some(Date::new(2026, 11, 15))
    );
}

#[test]
fn anchors_expire_without_a_wall_clock_fallback() {
    let anchor = Anchor::from_http_date("Sun, 08 Nov 2026 12:00:00 GMT").unwrap();
    assert!(anchor
        .date_after(MAX_AGE - Duration::from_nanos(1))
        .is_some());
    assert_eq!(anchor.date_after(MAX_AGE), None);
    assert_eq!(anchor.date_after(Duration::MAX), None);
}

#[test]
fn accepts_leap_day_and_year_rollover() {
    let leap = Anchor::from_http_date("Thu, 29 Feb 2024 23:59:59 GMT").unwrap();
    assert_eq!(
        leap.date_after(Duration::from_secs(1)),
        Some(Date::new(2024, 3, 1))
    );
    let year = Anchor::from_http_date("Thu, 31 Dec 2026 23:59:59 GMT").unwrap();
    assert_eq!(
        year.date_after(Duration::from_secs(1)),
        Some(Date::new(2027, 1, 1))
    );
}

#[test]
fn rejects_invalid_or_non_http_dates() {
    for bad in [
        "",
        "2026-11-08",
        "Sun, 08 Nov 2026 12:00:00 UTC",
        "Mon, 08 Nov 2026 12:00:00 GMT",
        "Sun, 08 Nov 2026 24:00:00 GMT",
        "Sun, 08 Nov 2026 12:60:00 GMT",
        "Sun, 08 Nov 2026 12:00:60 GMT",
        "Sun, 31 Nov 2026 12:00:00 GMT",
        "Thu, 29 Feb 2026 12:00:00 GMT",
        "Sun, 08 Xxx 2026 12:00:00 GMT",
        "Sun, 08 Nov 2026 12:00:00 GMT trailing",
        "Sun, 08 Nov 1969 12:00:00 GMT",
    ] {
        assert!(Anchor::from_http_date(bad).is_none(), "accepted {bad}");
    }
}

#[test]
fn changing_the_computer_clock_after_the_check_needs_a_new_check() {
    let anchor = Anchor::from_http_date("Sun, 08 Nov 2026 12:00:00 GMT").unwrap();
    let hour = Duration::from_secs(3600);
    // The clocks agree: the internet date moves on with elapsed time only.
    assert_eq!(
        anchor.date_after_checked(hour, Some(hour)),
        Some(Date::new(2026, 11, 8))
    );
    // The computer's clock jumped a week ahead, or back: no date.
    let week = Duration::from_secs(7 * 24 * 3600);
    assert_eq!(anchor.date_after_checked(hour, Some(hour + week)), None);
    assert_eq!(anchor.date_after_checked(hour, None), None);
    // A small difference is allowed.
    assert!(anchor
        .date_after_checked(hour, Some(hour + MAX_DRIFT))
        .is_some());
    assert_eq!(
        anchor.date_after_checked(hour, Some(hour + MAX_DRIFT + Duration::from_secs(1))),
        None
    );
}

#[test]
fn a_simulated_check_moves_from_offline_to_checking() {
    use wickets::discovery_clock::{request, simulate, status, Status};
    simulate(Some(Status::Unavailable));
    assert_eq!(status(), Status::Unavailable);
    request();
    assert_eq!(status(), Status::Checking);
    simulate(Some(Status::Ready(Date::new(2026, 11, 8))));
    request();
    assert_eq!(status(), Status::Ready(Date::new(2026, 11, 8)));
    simulate(None);
}
