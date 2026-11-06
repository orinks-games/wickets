//! The daily, weekly and monthly challenges: their rules, and playing them
//! from the title and from the launcher.

use crate::common;

use common::{played, solve_run_room, spoke, title, Base, Twin, SEED, TODAY};
use haunted_hallways::challenge::{challenge_name, event};
use haunted_hallways::rules::{
    ball_spec, floor_spec, Ev, Event, GhostKind, Mode, Outcome, Puzzle, Run, Side, Twist,
    BALL_FLOOR, COURAGE, FLOORS, STAB_GRACE, WRONG_PENALTY,
};
use haunted_hallways::{open_challenge, sound, Haunt, Progress, ID};
use hearken::{Call, Headless, Key};
use parlor_core::{Challenge, Challenges, Date, Parlor, Period, Records};

fn daily(twist: Twist, floor: u32) -> Mode {
    Mode::Challenge(Event {
        period: Period::Daily,
        first: floor,
        last: floor,
        twist: Some(twist),
    })
}

fn monthly() -> Mode {
    Mode::Challenge(Event {
        period: Period::Monthly,
        first: BALL_FLOOR,
        last: BALL_FLOOR,
        twist: None,
    })
}

fn weekly(first: u32, twist: Twist) -> Mode {
    Mode::Challenge(Event {
        period: Period::Weekly,
        first,
        last: first + 2,
        twist: Some(twist),
    })
}

/// The launcher stand-in with the date pinned.
fn base() -> Headless<Parlor> {
    haunted_hallways::rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(TODAY);
    h
}

// ---- The rules of each challenge ----

#[test]
fn every_period_builds_its_own_event_the_same_for_everyone() {
    for day in 0..120 {
        let date = Date::from_days(TODAY.days() + day);
        for period in Period::ALL {
            let ch = Challenge::on(date, ID, period);
            let (a, seed_a) = Event::from_challenge(&ch);
            let (b, seed_b) = Event::from_challenge(&ch);
            assert_eq!((a, seed_a), (b, seed_b), "the same for every player");
            assert_eq!(a.period, period);
            match period {
                Period::Daily => {
                    assert!((3..=30).contains(&a.first));
                    assert_eq!(a.floors(), 1);
                    assert!(a.twist.is_some());
                }
                Period::Weekly => {
                    assert!((10..=40).contains(&a.first));
                    assert_eq!(a.floors(), 3);
                    assert!(a.last <= FLOORS);
                    assert!(a.twist.is_some());
                    assert!(a.carries_courage());
                }
                Period::Monthly => {
                    assert_eq!(a.first, BALL_FLOOR);
                    assert_eq!(a.twist, None);
                    assert!(a.spec(BALL_FLOOR).seance);
                }
            }
        }
    }
    // Different days give different houses.
    let a = Event::from_challenge(&Challenge::on(TODAY, ID, Period::Daily)).1;
    let b = Event::from_challenge(&Challenge::on(Date::new(2026, 10, 17), ID, Period::Daily)).1;
    assert_ne!(a, b);
}

#[test]
fn the_dailies_cycle_through_every_twist() {
    let mut seen = Vec::new();
    for day in 0..200 {
        let ch = Challenge::on(Date::from_days(TODAY.days() + day), ID, Period::Daily);
        let t = event(&ch).twist.unwrap();
        if !seen.contains(&t) {
            seen.push(t);
        }
    }
    assert_eq!(seen.len(), Twist::CHALLENGE.len());
}

#[test]
fn each_twist_changes_the_floor_it_names() {
    let plain = floor_spec(3, false);
    let spec = |t: Twist| match daily(t, 3) {
        Mode::Challenge(e) => e.spec(3),
        _ => unreachable!(),
    };
    let r = spec(Twist::Restless);
    assert_eq!(
        r.ghost_gap,
        (plain.ghost_gap.0 * 0.5, plain.ghost_gap.1 * 0.5)
    );
    assert_eq!(
        r.ghosts,
        &[GhostKind::Widow, GhostKind::Butler, GhostKind::Child]
    );
    assert!(spec(Twist::LightsOut).lights_out && !plain.lights_out);
    assert_eq!(spec(Twist::WitchingHour).haunt, (plain.haunt * 0.6).round());
    assert_eq!(spec(Twist::CursedHands).penalty, 2.0 * plain.penalty);
    assert!(!spec(Twist::SilentSpirits).hints && plain.hints);
    // The layout is the floor's own: same rooms count and size.
    assert_eq!(spec(Twist::LightsOut).rooms, plain.rooms);
}

#[test]
fn cursed_hands_costs_twenty_seconds_for_a_wrong_touch() {
    let mut run = Run::new(daily(Twist::CursedHands, 2), SEED);
    let room = run.current().clone();
    let decoy = *room
        .items
        .iter()
        .find(|p| !room.puzzle.wanted().contains(&p.item))
        .unwrap();
    for side in common::route(&run, (decoy.x, decoy.y)) {
        run.walk(side);
    }
    let before = run.haunt_left;
    assert_eq!(run.interact(), vec![Ev::WrongItem(decoy.item)]);
    assert_eq!(run.haunt_left, before - 20.0);
}

#[test]
fn silent_spirits_give_no_hints_and_take_no_time() {
    let mut run = Run::new(daily(Twist::SilentSpirits, 2), SEED);
    let before = run.haunt_left;
    assert_eq!(run.hint(), vec![Ev::NoHints]);
    assert_eq!(run.haunt_left, before);
    assert_eq!(run.hints, 0);
}

#[test]
fn lights_out_things_call_less_often() {
    let count = |mode: Mode| {
        let mut run = Run::new(mode, SEED);
        let mut n = 0;
        for _ in 0..200 {
            n += run
                .update(0.05)
                .iter()
                .filter(|e| matches!(e, Ev::Emit { .. }))
                .count();
        }
        n
    };
    let dark = count(daily(Twist::LightsOut, 2));
    let lit = count(Mode::Floor(2));
    assert!(dark < lit, "{dark} calls in the dark vs {lit} lit");
    assert!(dark > 0, "they still call");
}

#[test]
fn witching_hour_shortens_the_clock() {
    let run = Run::new(daily(Twist::WitchingHour, 4), SEED);
    assert_eq!(run.haunt_left, (floor_spec(4, false).haunt * 0.6).round());
}

#[test]
fn the_weekly_carries_courage_from_floor_to_floor() {
    let mut run = Run::new(weekly(2, Twist::LightsOut), SEED);
    assert_eq!(run.spec.number, 2);
    solve_run_room(&mut run);
    run.summon(Some(GhostKind::Widow), Some(0.0));
    for _ in 0..400 {
        if run
            .update(0.05)
            .iter()
            .any(|e| matches!(e, Ev::GhostNear(_)))
        {
            break;
        }
    }
    run.update(STAB_GRACE);
    let side = if run.y > 0 { Side::South } else { Side::North };
    assert!(matches!(run.walk(side)[0], Ev::Caught { courage: 2, .. }));
    while !run.next_floor_waiting {
        solve_run_room(&mut run);
    }
    run.next_floor();
    assert_eq!(run.spec.number, 3);
    assert_eq!(run.courage, COURAGE - 1, "courage does not refill");
    while !run.next_floor_waiting {
        solve_run_room(&mut run);
    }
    run.next_floor();
    assert_eq!(run.spec.number, 4);
    while !run.finished() {
        solve_run_room(&mut run);
    }
    assert_eq!(run.outcome, Some(Outcome::Escaped));
    assert_eq!(run.floor_times.len(), 3);

    // The mansion refills it.
    let mut run = Run::new(Mode::Climb(1), SEED);
    run.courage = 1;
    while !run.next_floor_waiting {
        solve_run_room(&mut run);
    }
    run.next_floor();
    assert_eq!(run.courage, COURAGE);
}

#[test]
fn the_ball_lays_out_seance_rooms() {
    for seed in 0..40 {
        let rooms = Run::plan(monthly(), seed, BALL_FLOOR);
        let spec = ball_spec();
        assert_eq!(rooms.len(), spec.rooms);
        assert!(matches!(rooms[0].puzzle, Puzzle::Match(_)));
        assert!(matches!(rooms[2].puzzle, Puzzle::Offering(_)));
        let seances = rooms
            .iter()
            .filter(|r| matches!(r.puzzle, Puzzle::Seance(..)))
            .count();
        assert_eq!(seances, 3);
        for room in &rooms {
            if let Puzzle::Seance(seq) = room.puzzle {
                let (a, b) = (seq.items()[0], seq.items()[1]);
                assert_eq!(seq.len(), 2);
                assert_ne!(a, b);
                assert_eq!(room.items.len(), 2 + spec.decoys);
                for want in [a, b] {
                    assert_eq!(room.items.iter().filter(|p| p.item == want).count(), 1);
                }
                let start = room.inside(room.entry);
                assert!(room.items.iter().all(|p| (p.x, p.y) != start));
            }
        }
    }
}

#[test]
fn a_seance_wants_its_two_things_in_order() {
    let mut run = Run::new(monthly(), SEED);
    solve_run_room(&mut run);
    let room = run.current().clone();
    let Puzzle::Seance(seq) = room.puzzle else {
        panic!("room 2 of the ball is a séance")
    };
    let (a, b) = (seq.items()[0], seq.items()[1]);
    // The second thing first is out of order.
    let second = room.items.iter().find(|p| p.item == b).unwrap();
    for side in common::route(&run, (second.x, second.y)) {
        run.walk(side);
    }
    let before = run.haunt_left;
    assert_eq!(run.interact(), vec![Ev::WrongItem(b)]);
    assert_eq!(run.haunt_left, before - WRONG_PENALTY);
    assert!(!run.current().solved);
    // In order: the first, then the second.
    let goal = run.goal();
    let first = room.items.iter().find(|p| p.item == a).unwrap();
    assert_eq!(goal, (first.x, first.y));
    for side in common::route(&run, goal) {
        run.walk(side);
    }
    assert_eq!(
        run.interact(),
        vec![
            Ev::PickedUp(a),
            Ev::SeanceStep {
                next: b,
                done: 1,
                of: 2
            }
        ]
    );
    assert!(!run.current().solved);
    assert_eq!(run.goal(), (second.x, second.y));
    let score = run.score;
    for side in common::route(&run, run.goal()) {
        run.walk(side);
    }
    let evs = run.interact();
    assert_eq!(evs[0], Ev::PickedUp(b));
    assert!(matches!(evs[1], Ev::Solved { .. }));
    assert_eq!(run.score, score + 100 * ball_spec().level as u64);
    assert!(run.inventory.contains(&a) && run.inventory.contains(&b));
}

#[test]
fn the_whole_ball_can_be_escaped() {
    let mut run = Run::new(monthly(), SEED);
    while !run.finished() {
        solve_run_room(&mut run);
    }
    assert_eq!(run.outcome, Some(Outcome::Escaped));
}

// ---- Playing them ----

#[test]
fn the_title_lists_the_three_challenges_with_what_each_is() {
    let mut h = title();
    h.press(Key::C);
    assert!(
        spoke(&h, "Challenges, daily not played yet. 7 of 14."),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunted_hallways_challenges");
    let daily = event(&Challenge::on(TODAY, ID, Period::Daily));
    assert!(
        spoke(
            &h,
            &format!(
                "Daily, Tonight's haunting: floor {}, twist {}. Not played yet. 1 of 4.",
                daily.first,
                daily.twist.unwrap().name()
            )
        ),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Down);
    let weekly = event(&Challenge::on(TODAY, ID, Period::Weekly));
    assert!(spoke(
        &h,
        &format!(
            "Weekly, The long night: floors {} to {}, courage carried",
            weekly.first, weekly.last
        )
    ));
    h.press(Key::Down);
    assert!(spoke(
        &h,
        "Monthly, The midnight masquerade: the hidden ballroom wing, séance rooms. Not played yet. 3 of 4."
    ));
    // F1 on a row reads its rule and when it resets.
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "A new one in 16 days."), "{:?}", h.spoken());
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunted_hallways");
}

#[test]
fn the_daily_briefing_states_its_floor_and_twist_on_one_page() {
    let mut h = title();
    h.press(Key::C);
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    let e = event(&Challenge::on(TODAY, ID, Period::Daily));
    let t = e.twist.unwrap();
    assert!(spoke(
        &h,
        &format!("Tonight's haunting: Floor {}, in", e.first)
    ));
    assert!(spoke(
        &h,
        &format!("Twist, {}: {}. Enter to start.", t.name(), t.describe())
    ));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunt");
    assert!(
        spoke(&h, &format!("Floor {}: ", e.first)),
        "{:?}",
        h.spoken()
    );
    // The briefing said the twist; play does not say it again.
    assert!(
        !spoke(&h, &format!("{}: {}", t.name(), t.describe())),
        "{:?}",
        h.spoken()
    );
    // Status and help name the challenge.
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    assert!(spoke(
        &h,
        &format!("Tonight's haunting, twist {}, floor 1 of 1", t.name())
    ));
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "Tonight's haunting, a daily challenge."));
}

#[test]
fn winning_the_daily_records_its_result_best_and_streak() {
    let mut t = Twin::challenge(title(), Period::Daily);
    while !t.run.finished() {
        t.solve_room();
    }
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::WIN));
    let score = t.run.score;
    assert!(score > 0);
    for _ in 0..6 {
        if spoke(&t.h, "Daily challenge result") {
            break;
        }
        t.h.press(Key::Return);
    }
    assert!(
        // One floor: its page gave the time, so the results do not.
        spoke(&t.h, "Tonight's haunting survived."),
        "{:?}",
        t.h.spoken()
    );
    assert!(
        spoke(
            &t.h,
            &format!(
                "Daily challenge result: score {score}. Your first result on this challenge. A 1 day streak. A new one tomorrow."
            )
        ),
        "{:?}",
        t.h.spoken()
    );
    while t.h.app.top_name() == "pages" {
        t.h.press(Key::Return);
    }
    assert_eq!(
        common::rows(&t.h),
        [
            "Play this daily challenge again",
            "Hear the results again",
            "Back to the challenges"
        ]
    );
    t.read_until("haunted_hallways_challenges");
    assert!(spoke(&t.h, &format!("Done, best {score}, a 1 day streak.")));
    let ctx = &mut t.h.app.ctx;
    let ch = Challenge::current(ctx, ID, Period::Daily);
    assert_eq!(Challenges::best(ctx, &ch), Some(score));
    // A challenge leaves the ordinary records alone.
    let floor = event(&ch).first;
    assert_eq!(Records::best(ctx, ID, &format!("floor {floor}")), None);
    let progress = Progress::load(ctx);
    assert_eq!(progress.cleared, 0);
    assert_eq!(progress.floor_time(floor), None);
    assert_eq!(progress.escapes, 0);
}

#[test]
fn a_second_try_keeps_the_best_and_tomorrow_extends_the_streak() {
    let mut t = Twin::challenge(title(), Period::Daily);
    while !t.run.finished() {
        t.solve_room();
    }
    let best = t.run.score;
    t.read_until("haunted_hallways_challenges");
    // A worse try: lose on the clock with nothing solved.
    t.h.press(Key::Home);
    t.h.press(Key::Return);
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "haunt");
    for _ in 0..2000 {
        t.h.frame(0.5);
        if t.h.app.top_name() == "pages" {
            break;
        }
    }
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::LOSE));
    for _ in 0..6 {
        if spoke(&t.h, "Daily challenge result") {
            break;
        }
        t.h.press(Key::Return);
    }
    assert!(spoke(&t.h, "The haunt clock strikes zero on floor"));
    assert!(
        spoke(&t.h, &format!("Your best is {best}. A 1 day streak.")),
        "{:?}",
        t.h.spoken()
    );
    t.read_until("haunted_hallways_challenges");
    let ctx = &mut t.h.app.ctx;
    let ch = Challenge::current(ctx, ID, Period::Daily);
    assert_eq!(Challenges::best(ctx, &ch), Some(best));

    // The next day it is a new challenge, and the streak grows.
    t.h.app.ctx.game.today = Some(Date::new(2026, 10, 17));
    let ch = Challenge::current(&t.h.app.ctx, ID, Period::Daily);
    assert!(!Challenges::done(&mut t.h.app.ctx, &ch));
    Challenges::submit(&mut t.h.app.ctx, &ch, 10);
    assert_eq!(Challenges::streak(&mut t.h.app.ctx, ID, Period::Daily), 2);
}

#[test]
fn quitting_a_challenge_from_pause_does_not_count() {
    let mut t = Twin::challenge(title(), Period::Daily);
    t.solve_room();
    t.h.press(Key::Escape);
    assert_eq!(t.h.app.top_name(), "pause");
    assert_eq!(common::rows(&t.h)[3], "Leave the challenge");
    common::goto(&mut t.h, "Leave the challenge");
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "haunted_hallways_challenges");
    let ch = Challenge::current(&t.h.app.ctx, ID, Period::Daily);
    assert!(!Challenges::done(&mut t.h.app.ctx, &ch));
}

#[test]
fn the_weekly_climbs_three_floors_with_courage_carried() {
    let mut t = Twin::challenge(title(), Period::Weekly);
    let e = event(&Challenge::on(TODAY, ID, Period::Weekly));
    assert!(spoke(&t.h, &format!("Floor {}: ", e.first)));
    for floor in e.first..=e.last {
        while t.run.spec.number == floor && !t.run.finished() && !t.run.next_floor_waiting {
            t.solve_room();
        }
        if floor < e.last {
            assert!(spoke(&t.h, &format!("Floor {floor} cleared in")));
            t.h.press(Key::Return);
            assert!(spoke(&t.h, "Your courage does not refill: 3 of 3."));
            t.read_until("haunt");
            t.run.next_floor();
            assert!(spoke(&t.h, &format!("Floor {}", floor + 1)));
        }
    }
    assert!(t.run.finished());
    for _ in 0..8 {
        if spoke(&t.h, "Weekly challenge result") {
            break;
        }
        t.h.press(Key::Return);
    }
    assert!(spoke(&t.h, "The long night survived in"));
    assert!(spoke(&t.h, "A 1 week streak. A new one in 3 days."));
    t.h.press(Key::Return);
    assert!(spoke(
        &t.h,
        &format!(
            "Floor times: floor {}, 0.0 seconds; floor {}, 0.0 seconds; floor {}, 0.0 seconds.",
            e.first,
            e.first + 1,
            e.last
        )
    ));
    t.read_until("haunted_hallways_challenges");
}

#[test]
fn the_monthly_masquerade_plays_its_seances_by_ear() {
    let mut t = Twin::challenge(title(), Period::Monthly);
    assert!(spoke(&t.h, "The hidden wing: The midnight masquerade."));
    assert_eq!(t.h.app.ctx.audio.bed(), Some(sound::ambience(BALL_FLOOR)));
    t.solve_room();
    let room = t.run.current().clone();
    let Puzzle::Seance(seq) = room.puzzle else {
        panic!("a séance")
    };
    let (a, b) = (seq.items()[0], seq.items()[1]);
    assert!(spoke(&t.h, "A séance, "), "{:?}", t.h.spoken());
    assert!(played(&t.h, sound::SEANCE));
    // The two sounds follow, in order.
    // The séance's sounds are not placed in the room: they come from
    // the medium, unplaced, while the things themselves call from where
    // they lie.
    let clue = |h: &Headless<Parlor>, key: &str| {
        h.sounds.calls().into_iter().any(|c| match c {
            Call::Play { key: k, mix, .. } => k == key && mix.relative.is_none(),
            _ => false,
        })
    };
    t.h.clear();
    t.frame(1.0);
    assert!(clue(&t.h, sound::item(a)));
    assert!(!clue(&t.h, sound::item(b)));
    t.frame(1.6);
    assert!(clue(&t.h, sound::item(b)));
    // Status names the wing, and R plays it all again.
    t.h.clear();
    t.h.press(hearken::Key::S);
    t.h.press(hearken::Key::S);
    assert!(spoke(&t.h, "The midnight masquerade, the hidden wing"));
    t.h.clear();
    t.h.press(Key::R);
    assert!(played(&t.h, sound::SEANCE));
    // Out of order laughs; in order hums and then solves.
    let second = *room.items.iter().find(|p| p.item == b).unwrap();
    t.walk_to((second.x, second.y));
    t.h.clear();
    t.interact();
    assert!(played(&t.h, sound::WRONG));
    assert!(spoke(&t.h, "Wrong order."));
    let goal = t.run.goal();
    t.walk_to(goal);
    t.h.clear();
    t.interact();
    assert!(played(&t.h, sound::SEANCE_STEP));
    assert!(spoke(&t.h, "1 of 2. Last one."));
    let goal = t.run.goal();
    t.walk_to(goal);
    t.h.clear();
    t.interact();
    assert!(spoke(&t.h, "Solved."));
    // On through the rest of the ball.
    let door = room.inside(room.exit);
    t.walk_to(door);
    t.walk(room.exit.side);
    while !t.run.finished() {
        t.solve_room();
    }
    for _ in 0..6 {
        if spoke(&t.h, "Monthly challenge result") {
            break;
        }
        t.h.press(Key::Return);
    }
    assert!(spoke(&t.h, "The midnight masquerade survived."));
    assert!(spoke(&t.h, "cleared in"), "the floor page gives the time");
    assert!(spoke(&t.h, "A 1 month streak. A new one in 16 days."));
}

#[test]
fn the_launcher_opens_a_challenge_and_gets_it_back_when_finished() {
    let mut h = base();
    open_challenge(&mut h.app.ctx, Period::Daily);
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "Tonight's haunting:"));
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunt");
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let (e, seed) = Event::from_challenge(&ch);
    let mut t = Twin {
        h,
        run: Run::new(Mode::Challenge(e), seed),
    };
    while !t.run.finished() {
        t.solve_room();
    }
    t.read_until("base");
    assert!(Challenges::done(&mut t.h.app.ctx, &ch));
}

#[test]
fn leaving_a_launcher_challenge_from_pause_returns_to_the_launcher() {
    let mut h = base();
    open_challenge(&mut h.app.ctx, Period::Monthly);
    h.frame(0.0);
    assert!(spoke(&h, "The midnight masquerade:"));
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunt");
    h.press(Key::Escape);
    h.press(Key::End);
    h.press(Key::Up); // Return to the title
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "base");
    let ch = Challenge::current(&h.app.ctx, ID, Period::Monthly);
    assert!(!Challenges::done(&mut h.app.ctx, &ch));
}

#[test]
fn a_haunt_built_for_a_challenge_says_its_twist_on_screen() {
    // Silent spirits refuse a hint out loud.
    let mut h = base();
    h.app
        .ctx
        .push(Haunt::with_seed(daily(Twist::SilentSpirits, 2), SEED));
    h.frame(0.0);
    h.clear();
    h.press(Key::H);
    assert!(spoke(&h, "No hints on this floor."));
    h.clear();
    parlor_core::testkit::read_help(&mut h);
    assert!(spoke(&h, "No hints on this floor."));
    assert_eq!(challenge_name(Period::Daily), "Tonight's haunting");
}
