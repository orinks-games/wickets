//! Pace and the speech hold: the safe's clock, the guard and the building
//! stand still while a line is being spoken and run at the run's pace,
//! while the dial answers every key at once.
use std::cell::RefCell;
use std::rc::Rc;

use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};
use safecracker::rules::{self, GuardPhase, Mode, Turn};
use safecracker::{Crack, Progress, ID};

const SEED: u64 = 42;

struct Observed(Rc<RefCell<Crack>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "safecracker_observed"
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

type Screen = Rc<RefCell<Crack>>;

fn headless() -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h
}

/// `crack` on screen at `pace`, its opening said.
fn open_at(mut h: Headless<Parlor>, crack: Crack, pace: Pace) -> (Headless<Parlor>, Screen) {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(crack));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(&mut h);
    (h, s)
}

fn open(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    open_at(headless(), Crack::with_seed(mode, SEED), pace)
}

fn challenge(period: Period, pace: Pace) -> (Headless<Parlor>, Screen) {
    let h = headless();
    let ch = Challenge::current(&h.app.ctx, ID, period);
    open_at(h, Crack::challenge(ch), pace)
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

/// One frame of `dt` straight to the screen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, s: &Screen, dt: f64) {
    s.borrow_mut().update(&mut h.app.ctx, dt);
}

/// The time on the safe, as the clock counts it.
fn time(s: &Screen) -> f64 {
    s.borrow().run.time
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// One turn of the dial, which starts the safe's clock.
fn first_turn(h: &mut Headless<Parlor>) {
    h.press(Key::Right);
    quiet(h);
}

/// Turns to each wheel's number the way it wants and sets it, until the
/// safe is open.
fn crack_open(h: &mut Headless<Parlor>, s: &Screen) {
    for _ in 0..10 {
        let (way, steps) = {
            let c = s.borrow();
            if c.run.finished() {
                return;
            }
            let way = c.run.required().unwrap_or(Turn::Right);
            (way, way.distance(c.run.dial, c.run.target()))
        };
        let key = match way {
            Turn::Right => Key::Right,
            Turn::Left => Key::Left,
        };
        for _ in 0..steps {
            h.press(key);
        }
        h.press(Key::Space);
        quiet(h);
    }
}

/// Every page said, until the pages close.
fn read_pages(h: &mut Headless<Parlor>) -> String {
    let mut said: Vec<String> = h.spoken();
    for _ in 0..20 {
        if h.app.top_name() != "pages" {
            break;
        }
        h.clear();
        h.press(Key::Return);
        said.extend(h.spoken());
    }
    said.join(" ")
}

#[test]
fn normal_is_the_default_and_settings_has_the_pace_row() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = Rc::new(RefCell::new(Crack::with_seed(Mode::Safe(1), SEED)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
    let rows = <safecracker::Safecracker as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 1);
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    for pace in Pace::ALL {
        let runs = [
            open(Mode::Safe(1), pace),
            open(Mode::Safe(5), pace),
            open(Mode::TimeAttack(2), pace),
            open(Mode::Unlucky, pace),
            open(Mode::Blackout, pace),
            challenge(Period::Daily, pace),
            challenge(Period::Weekly, pace),
            challenge(Period::Monthly, pace),
        ];
        for (mut h, s) in runs {
            let mode = s.borrow().run.mode;
            assert_eq!(s.borrow().clock.pace(), pace);
            first_turn(&mut h);
            let guard = |s: &Screen| s.borrow().run.guard.map(|g| (g.phase, g.left));
            let (before, left, watch) = (time(&s), s.borrow().run.time_left, guard(&s));
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.2);
            assert_eq!(s.borrow().clock.state(), ClockState::Held, "{mode:?}");
            assert_eq!(s.borrow().run.time_left, left, "{mode:?} holds its clock");
            assert_eq!(guard(&s), watch, "{mode:?} holds the guard");
            // The time on the safe still counts the hold, at the pace.
            let held = time(&s) - before;
            assert!(close(held, 0.2 * pace.scale()), "{mode:?}: {held}");
            quiet(&mut h);
            tick(&mut h, &s, 0.2);
            let ran = time(&s) - before - held;
            assert!(
                close(ran, 0.2 * pace.scale()),
                "{mode:?} at {pace:?}: {ran}"
            );
            if mode.timed() {
                let used = left - s.borrow().run.time_left;
                assert!(close(used, 0.2 * pace.scale()), "{mode:?}: {used}");
            }
            if let Some((_, was)) = watch {
                let now = guard(&s).map(|g| g.1).unwrap_or_default();
                assert!(close(was - now, 0.2 * pace.scale()), "{mode:?}: {now}");
            }
        }
    }
}

#[test]
fn the_guard_stands_still_while_a_line_is_spoken() {
    let (mut h, s) = open(Mode::Safe(5), Pace::Quick);
    first_turn(&mut h);
    for _ in 0..4000 {
        if s.borrow().run.guard_phase() == Some(GuardPhase::Coming) {
            break;
        }
        quiet(&mut h);
        tick(&mut h, &s, 0.05);
    }
    assert_eq!(s.borrow().run.guard_phase(), Some(GuardPhase::Coming));
    let left = s.borrow().run.guard.map(|g| g.left);
    h.app.ctx.say("Dial at 40, wheel 1 of 4.");
    tick(&mut h, &s, 0.5);
    assert_eq!(s.borrow().run.guard.map(|g| g.left), left);
    quiet(&mut h);
    tick(&mut h, &s, 0.5);
    let now = s.borrow().run.guard.map(|g| g.left).unwrap_or_default();
    assert!(close(left.unwrap_or_default() - now, 0.5), "{now}");
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = open(Mode::Safe(1), Pace::Quick);
    first_turn(&mut h);
    let before = time(&s);
    for _ in 0..300 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(time(&s) - before > 10.0, "{}", time(&s) - before);
}

#[test]
fn a_key_during_a_hold_turns_the_dial_at_once() {
    let (mut h, s) = open(Mode::Safe(1), Pace::Normal);
    first_turn(&mut h);
    h.app.ctx.say("A line still being spoken.");
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held());
    let dial = s.borrow().run.dial;
    h.press(Key::Right);
    assert_eq!(s.borrow().run.dial, Turn::Right.step(dial));
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = open(Mode::Safe(1), Pace::Relaxed);
    first_turn(&mut h);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "safecracker_observed");
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "safecracker_observed");
    assert!(
        h.spoken().iter().any(|l| l.starts_with("Back at the dial")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = time(&s);
    for _ in 0..30 {
        tick(&mut h, &s, 0.1);
        assert_eq!(s.borrow().clock.state(), ClockState::Held);
    }
    assert!(
        close(time(&s) - before, 30.0 * 0.1 * Pace::Relaxed.scale()),
        "held time still counts at the run's pace: {}",
        time(&s) - before
    );
    // The next safe takes the new pace.
    let (_h, s) = open(Mode::Safe(1), Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

#[test]
fn a_relaxed_safe_keeps_its_own_score_and_time() {
    let (mut h, s) = open(Mode::Safe(1), Pace::Relaxed);
    first_turn(&mut h);
    tick(&mut h, &s, 4.0);
    crack_open(&mut h, &s);
    let pages = read_pages(&mut h);
    let ctx = &mut h.app.ctx;
    assert!(Records::best(ctx, ID, "safe 1-relaxed").is_some());
    assert_eq!(Records::best(ctx, ID, "safe 1"), None);
    let p = Progress::load(ctx);
    assert_eq!(p.relaxed_time(1), Some(2.0));
    assert_eq!(p.best_time(1), None);
    assert_eq!(p.cracked, 1, "safes cracked count at every pace");
    assert!(
        pages.contains("your first relaxed score on this safe"),
        "{pages}"
    );
}

#[test]
fn normal_and_quick_share_a_board() {
    let (mut h, s) = open(Mode::Safe(1), Pace::Normal);
    first_turn(&mut h);
    tick(&mut h, &s, 4.0);
    crack_open(&mut h, &s);
    read_pages(&mut h);
    assert!(Records::best(&mut h.app.ctx, ID, "safe 1").is_some());
    let normal = Progress::load(&mut h.app.ctx).best_time(1);
    assert_eq!(normal, Some(3.4));
    h.clear();
    let (mut h, s) = open_at(h, Crack::with_seed(Mode::Safe(1), SEED), Pace::Quick);
    first_turn(&mut h);
    tick(&mut h, &s, 4.0);
    crack_open(&mut h, &s);
    let pages = read_pages(&mut h);
    assert!(pages.contains("Your best is"), "{pages}");
    assert!(pages.contains("Your best time is"), "{pages}");
    assert_eq!(Records::best(&mut h.app.ctx, ID, "safe 1-relaxed"), None);
    assert_eq!(Progress::load(&mut h.app.ctx).relaxed_time(1), None);
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, s) = challenge(Period::Daily, Pace::Relaxed);
    first_turn(&mut h);
    crack_open(&mut h, &s);
    let pages = read_pages(&mut h);
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
fn records_safe_select_and_the_title_name_both_boards() {
    let mut h = headless();
    let ctx = &mut h.app.ctx;
    let mut p = Progress {
        cracked: 3,
        ..Default::default()
    };
    p.best.insert(1, 120);
    p.relaxed_best.insert(1, 200);
    p.relaxed_best.insert(2, 300);
    p.relaxed_attack_best.insert(1, 450);
    p.store(ctx);
    Records::record(ctx, ID, "safe 1", 300);
    Records::record(ctx, ID, "safe 1-relaxed", 250);
    Records::record(ctx, ID, "safe 2-relaxed", 240);
    Records::record(ctx, ID, "time attack 1-relaxed", 260);
    Records::record(ctx, ID, "unlucky-relaxed", 410);
    let all = safecracker::records_pages(ctx).join(" ");
    assert!(
        all.contains("Safe 1, the office lockbox: best time 12 seconds, relaxed best time 20 seconds, best score 300, relaxed best score 250."),
        "{all}"
    );
    assert!(
        all.contains(
            "Safe 2, the hotel strongbox: relaxed best time 30 seconds, relaxed best score 240."
        ),
        "{all}"
    );
    assert!(
        all.contains("Time attack: safe 1, relaxed best 45 seconds, relaxed best score 260"),
        "{all}"
    );
    assert!(all.contains("relaxed best score 410."), "{all}");
    assert!(!all.contains("best 0"), "{all}");
    assert!(!all.contains("best score 0"), "{all}");

    h.app.ctx.push(safecracker::SafeSelect::new(false));
    h.frame(0.0);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l
                .contains("Safe 1, the office lockbox, best 12 seconds, relaxed best 20 seconds")),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::Down);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.contains("Safe 2, the hotel strongbox, relaxed best 30 seconds")),
        "{:?}",
        h.spoken()
    );
}

/// Safe 1 cracked at quick, 2 seconds of play after the first turn. With
/// `spam`, the S key is pressed and its line spoken for 1 second of play
/// before the rest. Gives the time saved, the score and the time held.
fn crack_still(mode: Mode, spam: bool) -> (f64, u64, f64) {
    let (mut h, s) = open(mode, Pace::Quick);
    first_turn(&mut h);
    let mut held = 0.0;
    if spam {
        h.press(Key::S);
        for _ in 0..10 {
            if !h.app.ctx.speech_busy() {
                h.press(Key::S);
            }
            tick(&mut h, &s, 0.1);
            held += 0.1;
        }
    }
    quiet(&mut h);
    tick(&mut h, &s, 2.0);
    crack_open(&mut h, &s);
    let run = &s.borrow().run;
    (run.time, run.score, held)
}

#[test]
fn info_keys_never_improve_a_time_or_its_bonus() {
    let (time, score, _) = crack_still(Mode::Safe(1), false);
    assert!(close(time, 2.0), "with nothing said, as before: {time}");
    let (spam_time, spam_score, held) = crack_still(Mode::Safe(1), true);
    assert!(close(spam_time, time + held), "{spam_time}");
    assert!(spam_score <= score, "{spam_score} {score}");

    // Time attack: the clock holds, and its bonus counts the hold.
    let (_, score, _) = crack_still(Mode::TimeAttack(1), false);
    let (spam_time, spam_score, held) = crack_still(Mode::TimeAttack(1), true);
    assert!(close(spam_time, 2.0 + held), "{spam_time}");
    assert_eq!(spam_score, score - (held * 3.0).round() as u64);
}

#[test]
fn status_from_the_first_frame_holds_only_for_the_budget() {
    // The status key every second from the first frame, the voice never
    // quiet: the danger moves once the information budget is spent.
    let (mut h, s) = open(Mode::Safe(1), Pace::Normal);
    // The clock starts with the first turn, by design.
    first_turn(&mut h);
    let before = time(&s);
    let t = parlor_core::testkit::status_spam_until(
        &mut h,
        60,
        |h| tick(h, &s, 0.1),
        |_| time(&s) > before,
    );
    let t = t.expect("the danger never moved");
    assert!(t <= parlor_core::testkit::SPAM_BOUND_S, "{t}");
}
