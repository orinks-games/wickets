//! The rules every game's challenges share, checked across the whole
//! collection from the launcher's Challenges board: each one opens for every
//! period, the board says when the next one comes, briefings leave the
//! shared rules to Help, a run pauses on Escape (and
//! on a controller's B and Start), answers F1 and the pad's face buttons in
//! play, and leaving from the pause menu returns to the board without
//! counting the run.

use hearken::{ControllerButton, Headless, InputEvent, Key};
use parlor_core::{Challenge, Challenges, Date, GameInfo, Parlor, Period};
use wickets::{ChallengeBoard, ChallengeList, Lobby};

const TODAY: Date = Date::new(2026, 10, 16);

/// The launcher with `period`'s challenge list open.
fn board(period: Period) -> Headless<Parlor> {
    let mut h = Headless::new(Default::default(), Lobby::new());
    h.app.ctx.game.today = Some(TODAY);
    h.app.ctx.push(ChallengeList::new(period));
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "challenge list");
    h
}

/// Screens that come before play: briefings, story pages, menus that only
/// lead on into the run.
fn before_play(name: &str) -> bool {
    name == "pages" || name.ends_with("briefing") || name.ends_with("host")
}

/// Opens `g`'s `period` challenge from the board and reads every page up to
/// play, returning what was said on the way.
fn open_to_play(h: &mut Headless<Parlor>, g: &GameInfo, period: Period) -> String {
    let open = g.challenge.expect("every game has challenges");
    h.clear();
    open(&mut h.app.ctx, period);
    h.frame(0.0);
    let mut said = h.spoken().join(" ");
    for _ in 0..12 {
        if !before_play(h.app.top_name()) {
            break;
        }
        h.clear();
        h.press(Key::Return);
        h.frame(0.0);
        said.push(' ');
        said.push_str(&h.spoken().join(" "));
    }
    assert!(
        !before_play(h.app.top_name()),
        "{} {period:?}: still on {} after the briefing",
        g.name,
        h.app.top_name()
    );
    said
}

fn games() -> Vec<GameInfo> {
    wickets::games()
}

#[test]
fn every_game_has_challenges() {
    for g in games() {
        assert!(g.challenge.is_some(), "{} has no challenge", g.name);
    }
}

#[test]
fn the_board_says_when_each_period_resets() {
    // Briefings leave the reset to the board, which says it once a period.
    for period in Period::ALL {
        let mut h = Headless::new(Default::default(), Lobby::new());
        h.app.ctx.game.today = Some(TODAY);
        h.app.ctx.push(ChallengeBoard::new());
        h.frame(0.0);
        let resets = Challenge::on(TODAY, "wickets", period).resets();
        let label = period_row(&mut h, period);
        assert!(label.contains(&resets), "{period:?}: {label}");
    }
}

/// The board's row for `period`, as it is spoken when focused.
fn period_row(h: &mut Headless<Parlor>, period: Period) -> String {
    h.press(Key::Home);
    for _ in 0..Period::ALL.iter().position(|p| *p == period).unwrap() {
        h.press(Key::Down);
    }
    h.clear();
    h.press(Key::Down);
    h.press(Key::Up);
    h.spoken().join(" ")
}

#[test]
fn briefings_leave_out_the_shared_rules() {
    // The counting rule is in the launcher's Help, and Pages says Enter
    // starts, so no briefing repeats them.
    let mut games = games();
    games.extend(wickets::specials().into_iter().map(|(g, _)| g));
    games.extend(wickets::story_games().into_iter().map(|(g, _)| g));
    for g in games {
        for period in Period::ALL {
            let mut h = board(period);
            let said = open_to_play(&mut h, &g, period);
            for boilerplate in [
                "Escape on this briefing",
                "does not count",
                "Your result counts",
                "not played yet",
            ] {
                assert!(
                    !said.contains(boilerplate),
                    "{} {period:?} still says {boilerplate:?}: {said}",
                    g.name
                );
            }
            assert_eq!(
                said.matches("Enter to start.").count(),
                1,
                "{} {period:?}: {said}",
                g.name
            );
        }
    }
}

#[test]
fn leaving_a_challenge_from_pause_returns_to_the_board_and_does_not_count() {
    for g in games() {
        for period in Period::ALL {
            let mut h = board(period);
            open_to_play(&mut h, &g, period);
            h.frame(0.5);
            h.press(Key::Escape);
            assert_eq!(
                h.app.top_name(),
                "pause",
                "{} {period:?}: Escape did not pause",
                g.name
            );
            // Resume, Settings, Help, Return to the title.
            for _ in 0..3 {
                h.press(Key::Down);
            }
            h.press(Key::Return);
            h.frame(0.0);
            assert_eq!(
                h.app.top_name(),
                "challenge list",
                "{} {period:?}: leaving did not return to the board",
                g.name
            );
            let ch = Challenge::on(TODAY, g.id, period);
            assert!(
                !Challenges::done(&mut h.app.ctx, &ch),
                "{} {period:?}: a run left from pause was counted",
                g.name
            );
        }
    }
}

#[test]
fn pausing_a_challenge_stops_everything_under_it() {
    for g in games() {
        let mut h = board(Period::Daily);
        open_to_play(&mut h, &g, Period::Daily);
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "pause", "{}", g.name);
        // A long wait on the pause menu: the run must still be there, and
        // nothing may finish it.
        for _ in 0..600 {
            h.frame(1.0);
        }
        assert_eq!(h.app.top_name(), "pause", "{}", g.name);
        let ch = Challenge::on(TODAY, g.id, Period::Daily);
        assert!(!Challenges::done(&mut h.app.ctx, &ch), "{}", g.name);
        h.press(Key::Escape);
        assert!(
            !before_play(h.app.top_name()) && h.app.top_name() != "pause",
            "{}: Escape on pause did not resume, on {}",
            g.name,
            h.app.top_name()
        );
    }
}

#[test]
fn f1_in_a_challenge_run_speaks_help() {
    for g in games() {
        let mut h = board(Period::Daily);
        open_to_play(&mut h, &g, Period::Daily);
        h.clear();
        h.press(Key::F1);
        let said = h.spoken().join(" ");
        assert!(said.len() > 20, "{}: F1 said {said:?}", g.name);
    }
}

#[test]
fn a_controller_works_in_a_challenge_run() {
    let mut failures = Vec::new();
    for g in games() {
        let mut h = board(Period::Daily);
        h.connect_pad();
        open_to_play(&mut h, &g, Period::Daily);
        let top = h.app.top_name();
        let tap = |h: &mut Headless<Parlor>, b: ControllerButton| {
            h.send(InputEvent::button(b));
            h.send(InputEvent::button_up(b));
        };
        // Y is Status and the left bumper the hint:
        // every run answers them.
        for (b, what) in [
            (ControllerButton::Y, "status"),
            (ControllerButton::LeftShoulder, "hint"),
        ] {
            h.clear();
            tap(&mut h, b);
            if h.spoken().is_empty() && h.played().is_empty() {
                failures.push(format!("{}: {b:?} ({what}) did nothing on {top}", g.name));
            }
        }
        // B pauses, B again resumes; Start pauses too.
        tap(&mut h, ControllerButton::B);
        if h.app.top_name() != "pause" {
            failures.push(format!("{}: B did not pause", g.name));
            continue;
        }
        tap(&mut h, ControllerButton::B);
        if h.app.top_name() != top {
            failures.push(format!("{}: B did not resume", g.name));
            continue;
        }
        tap(&mut h, ControllerButton::Start);
        if h.app.top_name() != "pause" {
            failures.push(format!("{}: Start did not pause", g.name));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn the_pads_a_and_x_act_in_a_challenge_run() {
    let mut failures = Vec::new();
    for g in games() {
        for (b, what) in [
            (ControllerButton::A, "main action"),
            (ControllerButton::X, "second action"),
        ] {
            let mut h = board(Period::Daily);
            h.connect_pad();
            open_to_play(&mut h, &g, Period::Daily);
            let (top, depth) = (h.app.top_name(), h.app.depth());
            h.clear();
            h.send(InputEvent::button(b));
            h.send(InputEvent::button_up(b));
            let acted = !h.spoken().is_empty()
                || !h.played().is_empty()
                || (h.app.top_name(), h.app.depth()) != (top, depth);
            if !acted {
                failures.push(format!("{}: {b:?} ({what}) did nothing on {top}", g.name));
            }
            if h.app.top_name() == "pause" {
                failures.push(format!("{}: {b:?} ({what}) paused", g.name));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
