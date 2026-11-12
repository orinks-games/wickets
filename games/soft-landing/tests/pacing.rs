//! Pace and the speech hold: the lander stands still while a line is being
//! spoken and falls at the run's pace, and a burn under the held engine
//! key keeps flying through speech.
use std::cell::RefCell;
use std::rc::Rc;

use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};
use soft_landing::rules::{Flight, STEP_HZ};
use soft_landing::{events, progress, Descent, Mode, Progress, ID};

const DT: f64 = 1.0 / 60.0;

struct Observed(Rc<RefCell<Descent>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "soft_landing_observed"
    }
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.0.borrow_mut().enter(ctx);
    }
    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.0.borrow_mut().resume(ctx);
    }
    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.0.borrow_mut().exit(ctx);
    }
    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        self.0.borrow_mut().handle_input(ctx, event);
    }
    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.0.borrow_mut().update(ctx, dt);
    }
    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        self.0.borrow().help(ctx)
    }
}
struct Base;
impl State<Parlor> for Base {
    fn name(&self) -> &'static str {
        "base"
    }
    fn help(&self, _: &Ctx<Parlor>) -> String {
        String::new()
    }
}

type Screen = Rc<RefCell<Descent>>;

fn headless() -> Headless<Parlor> {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
    h
}

/// `descent` on screen at `pace`, its opening said.
fn open(mut h: Headless<Parlor>, descent: Descent, pace: Pace) -> (Headless<Parlor>, Screen) {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(descent));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(&mut h);
    (h, s)
}

fn descent(h: &Headless<Parlor>, mode: Mode) -> Descent {
    match mode {
        Mode::Challenge(p) => Descent::challenge(Challenge::current(&h.app.ctx, ID, p)),
        Mode::Tranquility => Descent::event(mode, events::tranquility()),
        mode => Descent::new(mode),
    }
}

/// A flight of `mode` at `pace`, the descent begun and its key let go.
fn flying(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    let h = headless();
    let d = descent(&h, mode);
    let (mut h, s) = open(h, d, pace);
    lift_off(&mut h, &s);
    (h, s)
}

/// Space starts the descent, and is let go.
fn lift_off(h: &mut Headless<Parlor>, s: &Screen) {
    h.send(InputEvent::key(Key::Space));
    h.send(InputEvent::key_up(Key::Space));
    quiet(h);
    tick(h, s, 0.0);
    assert!(s.borrow().flight.started());
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

/// One frame of `dt` straight to the screen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, s: &Screen, dt: f64) {
    s.borrow_mut().update(&mut h.app.ctx, dt);
}

fn time(s: &Screen) -> f64 {
    s.borrow().flight.time
}

fn state(s: &Screen) -> ClockState {
    s.borrow().clock.state()
}

/// Within one of the flight's fixed steps.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1.0 / STEP_HZ + 1e-6
}

#[test]
fn normal_is_the_default_pace() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = Rc::new(RefCell::new(Descent::new(Mode::Moon(1))));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    let modes = [
        Mode::Moon(1),
        Mode::Moon(5),
        Mode::Practice,
        Mode::Tranquility,
        Mode::Challenge(Period::Daily),
        Mode::Challenge(Period::Weekly),
    ];
    for mode in modes {
        for pace in Pace::ALL {
            let (mut h, s) = flying(mode, pace);
            assert_eq!(s.borrow().clock.pace(), pace);
            let before = time(&s);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.2);
            assert_eq!(time(&s), before, "{mode:?} holds for speech");
            assert_eq!(state(&s), ClockState::Held);
            quiet(&mut h);
            tick(&mut h, &s, 0.2);
            assert!(
                close(time(&s) - before, 0.2 * pace.scale()),
                "{mode:?} at {pace:?}: {}",
                time(&s) - before
            );
        }
    }
}

#[test]
fn the_lander_holds_while_a_call_out_is_spoken() {
    let (mut h, s) = flying(Mode::Moon(1), Pace::Quick);
    for _ in 0..3000 {
        if h.spoken().iter().any(|l| l.contains(" metres, falling ")) {
            break;
        }
        quiet(&mut h);
        tick(&mut h, &s, DT);
    }
    assert!(
        h.spoken().iter().any(|l| l.contains(" metres, falling ")),
        "{:?}",
        h.spoken()
    );
    let altitude = s.borrow().flight.altitude;
    for _ in 0..10 {
        tick(&mut h, &s, DT);
        assert_eq!(state(&s), ClockState::Held, "the call-out is being said");
        assert_eq!(s.borrow().flight.altitude, altitude);
    }
    quiet(&mut h);
    tick(&mut h, &s, DT);
    assert!(s.borrow().flight.altitude < altitude);
}

#[test]
fn a_held_engine_burns_on_through_speech_and_coasting_holds() {
    let (mut h, s) = flying(Mode::Moon(1), Pace::Normal);
    h.send(InputEvent::key(Key::Space));
    let (fuel, before) = (s.borrow().flight.fuel, time(&s));
    for _ in 0..30 {
        h.app.ctx.say("A line spoken over the burn.");
        let at = time(&s);
        tick(&mut h, &s, DT);
        assert!(time(&s) > at, "the burn does not freeze");
    }
    assert!(s.borrow().flight.engine);
    assert!(s.borrow().flight.fuel < fuel, "fuel burns through speech");
    assert!(close(time(&s) - before, 30.0 * DT * Pace::Normal.scale()));
    // Let go: coasting, speech holds the lander again.
    h.send(InputEvent::key_up(Key::Space));
    h.app.ctx.say("Still talking.");
    tick(&mut h, &s, DT);
    let at = time(&s);
    tick(&mut h, &s, DT);
    assert_eq!(state(&s), ClockState::Held);
    assert_eq!(time(&s), at);
}

#[test]
fn the_key_that_starts_the_descent_does_not_skip_the_hold() {
    let h = headless();
    let d = descent(&h, Mode::Moon(1));
    let (mut h, s) = open(h, d, Pace::Quick);
    // Space starts the descent and stays held: it fires nothing until let
    // go, so "Descent begun." holds the lander.
    h.send(InputEvent::key(Key::Space));
    tick(&mut h, &s, DT);
    tick(&mut h, &s, DT);
    assert_eq!(state(&s), ClockState::Held);
    assert_eq!(time(&s), 0.0);
}

#[test]
fn with_the_tank_empty_a_held_key_no_longer_skips_the_hold() {
    let (mut h, s) = flying(Mode::Moon(1), Pace::Quick);
    s.borrow_mut().flight.fuel = 0.05;
    h.send(InputEvent::key(Key::Space));
    for _ in 0..30 {
        quiet(&mut h);
        tick(&mut h, &s, DT);
    }
    assert!(
        h.spoken()
            .iter()
            .any(|l| l == "Fuel out. The engine is dead."),
        "{:?}",
        h.spoken()
    );
    h.app.ctx.say("Still talking.");
    tick(&mut h, &s, DT);
    let at = time(&s);
    tick(&mut h, &s, DT);
    assert_eq!(state(&s), ClockState::Held);
    assert_eq!(time(&s), at);
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = flying(Mode::Moon(1), Pace::Quick);
    let before = time(&s);
    for _ in 0..600 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
        if s.borrow().flight.finished() {
            break;
        }
    }
    // The lander falls once each hold runs out; every call-out starts a
    // fresh one, so a voice that never stops slows the descent a lot.
    assert!(time(&s) - before > 3.0, "{}", time(&s) - before);
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = flying(Mode::Moon(1), Pace::Relaxed);
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "soft_landing_observed");
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "soft_landing_observed");
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.starts_with("Back in flight at ")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = time(&s);
    for _ in 0..30 {
        tick(&mut h, &s, 0.1);
        assert_eq!(time(&s), before, "the resume line has a fresh hold");
    }
    let (_h, s) = flying(Mode::Moon(1), Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

/// Holds the engine when falling faster than it can still brake from.
fn autopilot(f: &Flight) -> bool {
    let a = f.moon.net();
    let target = (2.0 * a * 0.85 * f.altitude).sqrt().max(f.moon.safe * 0.5);
    f.speed() > target
}

/// Flies every moon down by autopilot, quietly, and reads the result
/// pages that follow.
fn land(h: &mut Headless<Parlor>, s: &Screen) -> String {
    let mut held = false;
    for _ in 0..(1200.0 / DT) as usize {
        if h.app.top_name() == "pages" {
            break;
        }
        let f = s.borrow().flight.clone();
        if !f.started() && !f.finished() {
            lift_off(h, s);
            continue;
        }
        let want = !f.finished() && autopilot(&f);
        if want != held {
            held = want;
            h.send(if held {
                InputEvent::key(Key::Space)
            } else {
                InputEvent::key_up(Key::Space)
            });
        }
        quiet(h);
        h.frame(DT);
    }
    assert_eq!(h.app.top_name(), "pages", "{:?}", h.spoken());
    let mut said = Vec::new();
    for _ in 0..20 {
        said.extend(h.spoken().iter().cloned());
        if h.app.top_name() != "pages" {
            break;
        }
        h.clear();
        h.press(Key::Return);
    }
    said.join(" ")
}

#[test]
fn a_relaxed_landing_keeps_its_own_board_and_softest_touchdown() {
    let (mut h, s) = flying(Mode::Moon(1), Pace::Relaxed);
    let pages = land(&mut h, &s);
    let board = progress::board(1);
    assert!(Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(&board)).is_some());
    assert_eq!(Records::best(&mut h.app.ctx, ID, &board), None);
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.cleared, 1, "the moon opens the next at every pace");
    assert!(p.relaxed_softest.is_some());
    assert_eq!(p.softest, None);
    assert!(
        pages.contains("points, your first relaxed score on Thistle."),
        "{pages}"
    );
    assert!(
        pages.contains("Your softest relaxed touchdown yet."),
        "{pages}"
    );
}

#[test]
fn normal_and_quick_share_a_board() {
    let (mut h, s) = flying(Mode::Moon(1), Pace::Normal);
    land(&mut h, &s);
    assert!(Records::best(&mut h.app.ctx, ID, &progress::board(1)).is_some());
    let d = Descent::new(Mode::Moon(1));
    let (mut h, s) = open(h, d, Pace::Quick);
    lift_off(&mut h, &s);
    let pages = land(&mut h, &s);
    assert!(
        pages.contains("Your best is") || pages.contains("a new best, beating"),
        "{pages}"
    );
    assert_eq!(Progress::load(&mut h.app.ctx).relaxed_softest, None);
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, s) = flying(Mode::Challenge(Period::Daily), Pace::Relaxed);
    let pages = land(&mut h, &s);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert!(Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed).is_some());
    assert!(
        pages.contains("Your first relaxed result on this challenge."),
        "{pages}"
    );
}

#[test]
fn a_relaxed_event_landing_keeps_its_own_board() {
    let (mut h, s) = flying(Mode::Tranquility, Pace::Relaxed);
    h.app.ctx.game.today = Some(Date::new(2026, 7, 20));
    land(&mut h, &s);
    let relaxed = Pace::Relaxed.board(events::TRANQUILITY_BOARD);
    assert!(Records::best(&mut h.app.ctx, ID, &relaxed).is_some());
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, events::TRANQUILITY_BOARD),
        None
    );
}

#[test]
fn records_and_the_title_name_both_boards() {
    let mut h = headless();
    let board = progress::board(1);
    Records::record(&mut h.app.ctx, ID, &board, 900);
    Records::record(&mut h.app.ctx, ID, &Pace::Relaxed.board(&board), 1200);
    Records::record(
        &mut h.app.ctx,
        ID,
        &Pace::Relaxed.board(&progress::board(2)),
        700,
    );
    Progress {
        cleared: 0,
        softest: Some(1.2),
        relaxed_softest: Some(0.8),
        landings: 3,
        ..Progress::default()
    }
    .store(&mut h.app.ctx);
    let all = soft_landing::records_pages(&mut h.app.ctx).join(" ");
    assert!(
        all.contains(
            "softest touchdown 1.2 metres a second, relaxed softest touchdown 0.8 metres a second."
        ),
        "{all}"
    );
    assert!(
        all.contains("Moon 1, Thistle: best score 900, relaxed best score 1200."),
        "{all}"
    );
    assert!(all.contains(": relaxed best score 700."), "{all}");
    assert!(!all.contains("best score 0"), "{all}");

    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(soft_landing::title());
    h.frame(0.0);
    h.clear();
    h.press(Key::F1);
    let help = h.spoken().join(" ");
    assert!(
        help.contains(" Best score 900, relaxed best score 1200."),
        "{help}"
    );
}

#[test]
fn settings_has_units_then_pace() {
    let h = headless();
    let rows = <soft_landing::SoftLanding as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 2);
}
