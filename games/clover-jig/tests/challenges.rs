//! The daily, weekly and monthly challenges.

use crate::common::*;
use clover_jig::challenge::{self, Plan, CHALLENGES_NAME};
use clover_jig::play::{Floor, Mode, END_PAUSE, PLAY_NAME};
use clover_jig::rules::{Luck, Run};
use clover_jig::tunes::Step;
use clover_jig::{ID, TITLE_NAME};
use hearken::{Headless, Key};
use parlor_core::{Challenge, Challenges, Period, Rng};

fn menu() -> Headless<parlor_core::Parlor> {
    let mut h = title();
    goto(&mut h, "Challenges");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    h
}

#[test]
fn the_menu_lists_three_challenges_and_back() {
    let mut h = menu();
    let r = rows(&h);
    assert_eq!(r.len(), 4);
    assert!(r[0].starts_with("Daily, Today's tune: "), "{}", r[0]);
    assert!(
        r[1].starts_with("Weekly, The weekly set: 3 tunes, "),
        "{}",
        r[1]
    );
    assert!(
        r[2].starts_with("Monthly, The long set: 5 tunes, "),
        "{}",
        r[2]
    );
    assert!(r[0].ends_with("Not played yet"));
    assert_eq!(r[3], "Back");
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn plans_come_from_the_seed_alone() {
    for period in Period::ALL {
        let a = Plan::new(period, Rng::seeded(5));
        let b = Plan::new(period, Rng::seeded(5));
        assert_eq!(a, b);
        assert_eq!(a.plans.len(), challenge::tune_count(period));
        let bpms: Vec<u32> = a.plans.iter().map(|p| p.tune.bpm).collect();
        let mut sorted = bpms.clone();
        sorted.sort();
        assert_eq!(bpms, sorted, "slowest first");
        for p in &a.plans {
            assert!(p.calls.iter().all(|c| c.luck == Luck::None));
            if period == Period::Daily {
                assert!(p
                    .calls
                    .iter()
                    .all(|c| !matches!(c.step, Step::Forward | Step::Back)));
            }
        }
    }
    let ch = Challenge::on(PLAIN_DAY, ID, Period::Weekly);
    assert_eq!(Plan::for_challenge(&ch), Plan::for_challenge(&ch));
}

#[test]
fn the_briefing_starts_the_challenge_and_escape_backs_out() {
    let mut h = menu();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "Today's tune:"));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    h.press(Key::Return);
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    assert!(spoke(&h, "Today's tune. 1 tune,"));
}

#[test]
fn a_finished_challenge_is_submitted_with_its_score() {
    let mut h = title();
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let mut twin = Run::new(Plan::for_challenge(&ch).plans, false);
    h.app.ctx.push(challenge::ChallengesMenu::new());
    h.frame(0.0);
    let f = Floor::challenge(&mut h.app.ctx, ch.clone());
    h.app.ctx.push(f);
    h.frame(0.0);
    start(&mut h, &mut twin);
    dance_all(&mut h, &mut twin);
    wait(&mut h, &mut twin, END_PAUSE + 0.5);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "Today's tune: 1 tune danced."));
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    assert_eq!(
        Challenges::best(&mut h.app.ctx, &ch),
        Some(twin.score() as u64)
    );
    let r = rows(&h);
    assert_eq!(r[0], "Play this daily challenge again");
    assert_eq!(r.last().map(String::as_str), Some("Back to the challenges"));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
}

#[test]
fn leaving_from_pause_does_not_count() {
    let mut h = menu();
    h.press(Key::Return);
    read_pages(&mut h);
    h.press(Key::Escape);
    assert_eq!(rows(&h)[3], "Leave the challenge");
    goto(&mut h, "Leave the challenge");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(!Challenges::done(&mut h.app.ctx, &ch));
}

#[test]
fn the_launcher_opens_a_challenge_through_its_host() {
    let mut h = Headless::new(data(PLAIN_DAY), Base);
    clover_jig::open_challenge(&mut h.app.ctx, Period::Weekly);
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "pages");
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    h.press(Key::Escape);
    goto(&mut h, "Leave the challenge");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "base");
}

#[test]
fn challenges_have_no_shamrocks_even_in_shamrock_week() {
    let mut h = title_on(SHAMROCK_DAY);
    let run = twin(&mut h.app.ctx, Mode::Challenge(Period::Daily));
    assert!(run.plans.iter().all(|p| p.shamrocks() == 0));
}
