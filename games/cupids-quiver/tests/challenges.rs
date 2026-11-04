//! The daily, weekly and monthly challenges.

use crate::common::*;
use cupids_quiver::challenge::{self, CHALLENGES_NAME};
use cupids_quiver::field::FIELD_NAME;
use cupids_quiver::{ChallengeHost, ID, TITLE_NAME};
use hearken::Key;
use parlor_core::menus::rows_of;
use parlor_core::{Challenge, Challenges, Date, Period};

fn menu() -> hearken::Headless<parlor_core::Parlor> {
    let mut h = title();
    goto(&mut h, "Challenges");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    h
}

#[test]
fn the_menu_lists_three_challenges_then_back() {
    let h = menu();
    let rows = rows_of(&h.app.top_lines());
    assert_eq!(rows.len(), 4);
    assert!(
        rows[0].starts_with("Daily, Today's sky: one long round, with "),
        "{rows:?}"
    );
    assert!(rows[1].starts_with("Weekly, The week's rooftops: three rooftops"));
    assert!(rows[2].starts_with("Monthly, The full moon: five moonlit rounds"));
    assert_eq!(rows[3], "Back");
}

#[test]
fn every_player_gets_the_same_sky() {
    let d = Date::new(2026, 10, 16);
    for period in Period::ALL {
        let a = Challenge::on(d, ID, period);
        let b = Challenge::on(d, ID, period);
        assert_eq!(challenge::launch(&a), challenge::launch(&b));
        assert_eq!(challenge::launch(&a).mode, challenge::mode(period));
    }
    assert!(challenge::twist(&Challenge::on(d, ID, Period::Daily)).is_some());
    assert!(challenge::twist(&Challenge::on(d, ID, Period::Weekly)).is_none());
    // The twist changes from day to day.
    let twists: std::collections::HashSet<_> = (1..=20)
        .map(|day| challenge::twist(&Challenge::on(Date::new(2026, 11, day), ID, Period::Daily)))
        .collect();
    assert!(twists.len() > 1);
}

#[test]
fn the_daily_plays_one_round_and_counts() {
    let mut h = menu();
    h.press(Key::Home);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(&h, "Today's twist,"), "{:?}", h.spoken());
    assert!(status_line(&h).contains("15 arrows left"));
    let rounds = play_out(&mut h);
    assert_eq!(rounds, 1);
    assert!(spoke(&h, "Today's sky done."));
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    let rows = rows_of(&h.app.top_lines());
    assert_eq!(rows[0], "Play this daily challenge again");
    assert_eq!(rows.last().unwrap(), "Back to the challenges");
    let daily = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &daily));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    assert!(rows_of(&h.app.top_lines())[0].contains("Done, best"));
}

#[test]
fn the_weekly_climbs_three_roofs_whatever_the_score() {
    let mut h = menu();
    goto(&mut h, "Weekly");
    h.press(Key::Return);
    read_pages(&mut h);
    assert!(spoke(
        &h,
        "The week's rooftops, round 1 of 3: The post office."
    ));
    let rounds = play_out(&mut h);
    assert_eq!(rounds, 3);
}

#[test]
fn leaving_a_challenge_does_not_count_it() {
    let mut h = menu();
    h.press(Key::Home);
    h.press(Key::Return);
    read_pages(&mut h);
    h.press(Key::Escape);
    let rows = rows_of(&h.app.top_lines());
    assert_eq!(rows[3], "Leave the challenge");
    goto(&mut h, "Leave the challenge");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    let daily = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(!Challenges::done(&mut h.app.ctx, &daily));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn the_launcher_opens_a_challenge_through_the_host() {
    let mut h = base(AUTUMN);
    h.app.ctx.push(ChallengeHost::new(Period::Monthly));
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "pages");
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(&h, "The full moon, round 1 of 5"));
    assert!(spoke(&h, "Roses drift among the hearts"));
    h.press(Key::Escape);
    goto(&mut h, "Leave the challenge");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "base");
}

#[test]
fn challenge_runs_have_no_roses_in_season_except_the_monthly() {
    let mut h = base(SEASON);
    h.app.ctx.push(ChallengeHost::new(Period::Daily));
    h.frame(0.0);
    read_pages(&mut h);
    assert!(!spoke(&h, "Roses drift"));
    assert!(spoke(&h, "Today's sky, round 1 of 1"));
}
