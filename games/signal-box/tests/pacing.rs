//! Pace and the speech hold: the answer window stands still while a line
//! is being spoken, and runs at the run's pace.
use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};
use signal_box::progress::SHIFT_BOARD;
use signal_box::rules::{self, Call, Phase};
use signal_box::{events, Mode, Progress, Signal, ID};
use std::cell::RefCell;
use std::rc::Rc;

struct Observed(Rc<RefCell<Signal>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "signal_box_observed"
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

type Screen = Rc<RefCell<Signal>>;

fn headless() -> Headless<Parlor> {
    rules::fix_seed(Some(42));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
    h
}

/// `signal` on screen at `pace`, the line open and its line said.
fn open(mut h: Headless<Parlor>, signal: Signal, pace: Pace) -> (Headless<Parlor>, Screen) {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(signal));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    h.press(Key::Space);
    quiet(&mut h);
    (h, s)
}

fn start(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    let signal = match mode {
        Mode::Challenge(period) => {
            let h = headless();
            Signal::challenge(Challenge::current(&h.app.ctx, ID, period))
        }
        mode => Signal::with_seed(mode, 42),
    };
    open(headless(), signal, pace)
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
    s.borrow().shift.time
}

/// Within one of the shift's fixed steps.
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1.0 / rules::STEP_HZ + 1e-6
}

/// Quiet frames until a call is out, then that call.
fn next_call(h: &mut Headless<Parlor>, s: &Screen) -> Call {
    for _ in 0..2000 {
        if let Some(call) = s.borrow().shift.current() {
            return call;
        }
        quiet(h);
        tick(h, s, 0.05);
    }
    panic!("no call came");
}

fn key_for(call: Call) -> Key {
    match call {
        Call::PointsLeft => Key::Left,
        Call::PointsRight => Key::Right,
        Call::SignalUp => Key::Up,
        Call::SignalDown => Key::Down,
        Call::RingBack => Key::Space,
    }
}

fn wrong_for(call: Call) -> Key {
    if call == Call::RingBack {
        Key::Left
    } else {
        Key::Space
    }
}

fn modes() -> Vec<Mode> {
    vec![
        Mode::Shift,
        Mode::SteamGala,
        Mode::Practice,
        Mode::Challenge(Period::Daily),
    ]
}

#[test]
fn normal_is_the_default_pace() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = Rc::new(RefCell::new(Signal::with_seed(Mode::Shift, 42)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    for mode in modes() {
        for pace in Pace::ALL {
            let (mut h, s) = start(mode, pace);
            assert_eq!(s.borrow().clock.pace(), pace);
            let before = time(&s);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.2);
            assert_eq!(time(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().clock.state(), ClockState::Held);
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
fn the_window_holds_while_the_call_is_spoken() {
    let mut h = headless();
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    let s = Rc::new(RefCell::new(Signal::with_seed(Mode::Shift, 42)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    h.press(Key::Space);
    // Real frames: the lines are said at the voice's own speed.
    let calling = |s: &Screen| matches!(s.borrow().shift.phase, Phase::Calling { .. });
    for _ in 0..2000 {
        if calling(&s) {
            break;
        }
        h.frame(0.05);
    }
    assert!(calling(&s));
    let call = s.borrow().shift.current().unwrap();
    assert!(h.spoken().iter().any(|l| l == &format!("{}.", call.word())));
    // The call is being said, so its window has not started.
    for _ in 0..4 {
        h.frame(0.05);
        assert!(s.borrow().clock.held(), "the call is being said");
        assert_eq!(
            s.borrow().shift.phase,
            Phase::Calling { call, elapsed: 0.0 }
        );
    }
    for _ in 0..200 {
        if !s.borrow().clock.held() {
            break;
        }
        h.frame(0.05);
    }
    h.frame(0.05);
    let Phase::Calling { elapsed, .. } = s.borrow().shift.phase else {
        panic!("still calling");
    };
    assert!(elapsed > 0.0, "the window runs once the call is said");
}

#[test]
fn each_call_gets_a_full_hold_however_long_the_lines_before_it() {
    let (mut h, s) = start(Mode::Shift, Pace::Quick);
    // A voice that never stops: the lead-in runs once the hold expires.
    for _ in 0..1000 {
        if s.borrow().shift.current().is_some() {
            break;
        }
        h.app.ctx.say("A long opening still being spoken.");
        tick(&mut h, &s, 0.1);
    }
    let call = s.borrow().shift.current().expect("a call came");
    h.app.ctx.say(format!("{}.", call.word()));
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held(), "the call has a fresh hold");
    assert_eq!(
        s.borrow().shift.phase,
        Phase::Calling { call, elapsed: 0.0 }
    );
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = start(Mode::Practice, Pace::Quick);
    // Two holds: the opening lines, then the call's own.
    for _ in 0..600 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(time(&s) > 10.0, "{}", time(&s));
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = start(Mode::Shift, Pace::Relaxed);
    let call = next_call(&mut h, &s);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "signal_box_observed");
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    // The resume line and the call again: a full hold of their own.
    for _ in 0..30 {
        tick(&mut h, &s, 0.1);
        assert_eq!(
            s.borrow().shift.phase,
            Phase::Calling { call, elapsed: 0.0 }
        );
    }
    // The next run takes the new pace.
    let (_h, s) = start(Mode::Shift, Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

/// Passes `trains` trains, then derails on the next call and reads the
/// results to the end.
fn derail_after(h: &mut Headless<Parlor>, s: &Screen, trains: u32) {
    for _ in 0..trains {
        let call = next_call(h, s);
        h.press(key_for(call));
    }
    let call = next_call(h, s);
    h.press(wrong_for(call));
    for _ in 0..100 {
        quiet(h);
        h.frame(0.05);
        if h.app.top_name() == "pages" {
            break;
        }
    }
}

/// Everything the result pages say, pressing Enter through them.
fn read_pages(h: &mut Headless<Parlor>) -> String {
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
fn a_relaxed_shift_keeps_its_own_records() {
    let (mut h, s) = start(Mode::Shift, Pace::Relaxed);
    derail_after(&mut h, &s, 2);
    assert_eq!(h.app.top_name(), "pages");
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(SHIFT_BOARD)),
        Some(2)
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, SHIFT_BOARD), None);
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!((p.best_run, p.relaxed_best_run, p.shifts), (0, 2, 1));
    assert!(p.shortest_window.is_none());
    assert!(p.relaxed_shortest_window.is_some());
    let pages = read_pages(&mut h);
    assert!(pages.contains("your first relaxed score"), "{pages}");
}

#[test]
fn normal_and_quick_share_the_shift_board() {
    let (mut h, s) = start(Mode::Shift, Pace::Normal);
    derail_after(&mut h, &s, 1);
    assert_eq!(Records::best(&mut h.app.ctx, ID, SHIFT_BOARD), Some(1));
    assert_eq!(Pace::Quick.board(SHIFT_BOARD), SHIFT_BOARD);
    assert_eq!(Progress::load(&mut h.app.ctx).best_run, 1);
}

#[test]
fn a_relaxed_challenge_and_gala_keep_their_own_results() {
    let (mut h, s) = start(Mode::Challenge(Period::Daily), Pace::Relaxed);
    derail_after(&mut h, &s, 1);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert_eq!(
        Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed),
        Some(1)
    );
    let pages = read_pages(&mut h);
    assert!(pages.contains("first relaxed result"), "{pages}");

    let (mut h, s) = start(Mode::SteamGala, Pace::Relaxed);
    derail_after(&mut h, &s, 1);
    assert_eq!(Records::best(&mut h.app.ctx, ID, events::GALA_BOARD), None);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(events::GALA_BOARD)),
        Some(1)
    );
}

#[test]
fn records_and_the_title_name_both_boards() {
    let mut h = headless();
    Records::record(&mut h.app.ctx, ID, SHIFT_BOARD, 30);
    Records::record(&mut h.app.ctx, ID, &Pace::Relaxed.board(SHIFT_BOARD), 41);
    Progress {
        best_run: 25,
        relaxed_best_run: 33,
        shortest_window: Some(1400),
        shifts: 2,
        ..Progress::default()
    }
    .store(&mut h.app.ctx);
    let all = signal_box::records_pages(&mut h.app.ctx).join(" ");
    assert!(
        all.contains("best score 30, relaxed best score 41"),
        "{all}"
    );
    assert!(
        all.contains("longest run 25 trains, relaxed longest run 33 trains"),
        "{all}"
    );
    assert!(all.contains("shortest window about 1.4 seconds."), "{all}");
    assert!(!all.contains("relaxed shortest window"), "{all}");

    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(signal_box::title());
    h.frame(0.0);
    let said = h.spoken().join(" ");
    assert!(
        said.contains("Play, night shift, best run 25 trains, relaxed best run 33 trains"),
        "{said}"
    );
    h.clear();
    h.press(Key::F1);
    let help = h.spoken().join(" ");
    assert!(
        help.contains("Best score 30, relaxed best score 41."),
        "{help}"
    );
}

#[test]
fn records_leave_out_a_board_with_no_score() {
    let mut h = headless();
    Records::record(&mut h.app.ctx, ID, &Pace::Relaxed.board(SHIFT_BOARD), 12);
    Progress {
        relaxed_best_run: 9,
        shifts: 1,
        ..Progress::default()
    }
    .store(&mut h.app.ctx);
    let all = signal_box::records_pages(&mut h.app.ctx).join(" ");
    assert!(
        all.contains("Night shift: relaxed best score 12, relaxed longest run 9 trains"),
        "{all}"
    );
    assert!(!all.contains("best score 0"), "{all}");
    assert!(!all.contains("relaxed best score 0"), "{all}");
}

#[test]
fn settings_has_the_pace_row() {
    let h = headless();
    let rows = <signal_box::SignalBox as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 1);
}
