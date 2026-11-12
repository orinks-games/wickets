//! Pace and the speech hold: the cats and every timer stand still while
//! a line is being spoken and run at the run's pace, while an arrow moves
//! the mouse at once.
use std::cell::RefCell;
use std::rc::Rc;

use hearken::{Ctx, Headless, InputEvent, Key, State};
use pantry_mouse::play::END_PAUSE;
use pantry_mouse::rules::{self, Dir};
use pantry_mouse::{events, progress, Mode, Play, ID};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};

const SEED: u64 = 11;
/// A day with Bin night and no other special night open.
const DAY: Date = Date::new(2026, 10, 6);

struct Observed(Rc<RefCell<Play>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "pantry_mouse_observed"
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

type Screen = Rc<RefCell<Play>>;

fn headless() -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(DAY);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h
}

/// `play` on screen at `pace`, its opening said.
fn open_at(mut h: Headless<Parlor>, play: Play, pace: Pace) -> (Headless<Parlor>, Screen) {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(play));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(&mut h);
    (h, s)
}

fn open(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    open_at(headless(), Play::with_seed(mode, SEED), pace)
}

fn challenge(period: Period, pace: Pace) -> (Headless<Parlor>, Screen) {
    let h = headless();
    let ch = Challenge::current(&h.app.ctx, ID, period);
    open_at(h, Play::challenge(ch), pace)
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

/// One frame of `dt` straight to the screen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, s: &Screen, dt: f64) {
    s.borrow_mut().update(&mut h.app.ctx, dt);
}

fn frozen(s: &Screen) -> u32 {
    s.borrow().run.frozen_ms()
}

fn cats(s: &Screen) -> Vec<rules::Cell> {
    s.borrow().run.cats.iter().map(|c| c.pos).collect()
}

fn key_for(dir: Dir) -> Key {
    match dir {
        Dir::Up => Key::Up,
        Dir::Left => Key::Left,
        Dir::Down => Key::Down,
        Dir::Right => Key::Right,
    }
}

/// One step the mouse can take, tapped.
fn first_step(h: &mut Headless<Parlor>, s: &Screen) {
    let way = {
        let run = &s.borrow().run;
        run.maze.open_dirs(run.mouse)[0]
    };
    h.press(key_for(way));
    h.send(InputEvent::key_up(key_for(way)));
    quiet(h);
}

/// Waits, voice quiet, until the run is over, then reads the pages.
fn play_out(h: &mut Headless<Parlor>, s: &Screen) -> String {
    first_step(h, s);
    for _ in 0..20_000 {
        if s.borrow().run.finished() {
            break;
        }
        quiet(h);
        h.frame(0.05);
    }
    assert!(s.borrow().run.finished());
    for _ in 0..((END_PAUSE / 0.05) as usize + 5) {
        h.frame(0.05);
    }
    let mut said = h.spoken();
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
    let s = Rc::new(RefCell::new(Play::with_seed(Mode::Pantry, SEED)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
    let rows = <pantry_mouse::PantryMouse as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 1);
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    for pace in Pace::ALL {
        let runs = [
            open(Mode::Pantry, pace),
            open(Mode::Practice, pace),
            open(Mode::Leftovers, pace),
            open(Mode::BinNight, pace),
            challenge(Period::Daily, pace),
            challenge(Period::Weekly, pace),
            challenge(Period::Monthly, pace),
        ];
        for (mut h, s) in runs {
            let mode = s.borrow().mode;
            assert_eq!(s.borrow().clock.pace(), pace);
            // A freeze runs down on the clock.
            h.press(Key::Tab);
            quiet(&mut h);
            let before = frozen(&s);
            assert!(before > 0, "{mode:?}");
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.4);
            assert_eq!(frozen(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().clock.state(), ClockState::Held);
            quiet(&mut h);
            for _ in 0..4 {
                tick(&mut h, &s, 0.1);
            }
            let ran = (before - frozen(&s)) as f64 / 1000.0;
            assert!(
                (ran - 0.4 * pace.scale()).abs() <= 0.011,
                "{mode:?} at {pace:?}: {ran}"
            );
        }
    }
}

#[test]
fn the_cats_stand_still_while_a_line_is_spoken() {
    let (mut h, s) = open(Mode::Pantry, Pace::Quick);
    first_step(&mut h, &s);
    let still = cats(&s);
    for _ in 0..40 {
        h.app.ctx.say("Ginger cat close, right.");
        tick(&mut h, &s, 0.1);
    }
    assert_eq!(cats(&s), still);
    for _ in 0..80 {
        quiet(&mut h);
        tick(&mut h, &s, 0.1);
    }
    assert_ne!(cats(&s), still, "the cats move once the line ends");
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = open(Mode::Pantry, Pace::Quick);
    first_step(&mut h, &s);
    let still = cats(&s);
    for _ in 0..300 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert_ne!(cats(&s), still);
}

#[test]
fn an_arrow_during_a_hold_moves_the_mouse_at_once_and_a_held_one_waits() {
    let (mut h, s) = open(Mode::Pantry, Pace::Normal);
    first_step(&mut h, &s);
    h.app.ctx.say("A line still being spoken.");
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held());
    let (from, way) = {
        let run = &s.borrow().run;
        (run.mouse, run.maze.open_dirs(run.mouse)[0])
    };
    // Pressed and held during the hold: one step at once.
    h.send(InputEvent::KeyDown {
        key: key_for(way),
        mods: hearken::Mods::NONE,
        text: None,
        repeat: false,
    });
    let moved = s.borrow().run.mouse;
    assert_eq!(moved, from.step(way));
    // The held arrow steps on only once the pantry runs.
    for _ in 0..10 {
        h.app.ctx.say("A line still being spoken.");
        tick(&mut h, &s, 0.1);
    }
    assert_eq!(s.borrow().run.mouse, moved);
    h.send(InputEvent::key_up(key_for(way)));
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = open(Mode::Pantry, Pace::Relaxed);
    h.press(Key::Tab);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "pantry_mouse_observed");
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pantry_mouse_observed");
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.starts_with("Back in the pantry.")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = frozen(&s);
    assert!(before > 0);
    for _ in 0..10 {
        tick(&mut h, &s, 0.1);
        assert_eq!(frozen(&s), before, "the resume line has a fresh hold");
    }
    let (_h, s) = open(Mode::Pantry, Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

#[test]
fn a_relaxed_run_keeps_its_own_board() {
    let (mut h, s) = open(Mode::Pantry, Pace::Relaxed);
    let pages = play_out(&mut h, &s);
    let ctx = &mut h.app.ctx;
    assert!(Records::best(ctx, ID, "pantry-relaxed").is_some());
    assert_eq!(Records::best(ctx, ID, progress::PANTRY_BOARD), None);
    assert!(
        pantry_mouse::Progress::load(ctx).deepest >= 1,
        "the deepest pantry counts at every pace"
    );
    assert!(pages.contains("your first relaxed score."), "{pages}");
}

#[test]
fn a_relaxed_special_night_keeps_its_own_board() {
    let (mut h, s) = open(Mode::BinNight, Pace::Relaxed);
    play_out(&mut h, &s);
    let board = events::BIN_NIGHT_BOARD;
    assert!(Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(board)).is_some());
    assert_eq!(Records::best(&mut h.app.ctx, ID, board), None);
}

#[test]
fn normal_and_quick_share_a_board() {
    let (mut h, s) = open(Mode::Pantry, Pace::Normal);
    play_out(&mut h, &s);
    assert!(Records::best(&mut h.app.ctx, ID, progress::PANTRY_BOARD).is_some());
    h.clear();
    let (mut h, s) = open_at(h, Play::with_seed(Mode::Pantry, SEED), Pace::Quick);
    let pages = play_out(&mut h, &s);
    assert!(!pages.contains("your first score"), "{pages}");
    assert!(pages.contains("Your best is 10."), "{pages}");
    assert!(!pages.contains("relaxed"), "{pages}");
    assert_eq!(Records::best(&mut h.app.ctx, ID, "pantry-relaxed"), None);
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, s) = challenge(Period::Daily, Pace::Relaxed);
    let pages = play_out(&mut h, &s);
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
fn records_the_title_and_special_nights_name_both_boards() {
    let mut h = headless();
    let ctx = &mut h.app.ctx;
    Records::record(ctx, ID, progress::PANTRY_BOARD, 300);
    Records::record(ctx, ID, "pantry-relaxed", 410);
    Records::record(ctx, ID, &Pace::Relaxed.board(events::BIN_NIGHT_BOARD), 520);
    let all = pantry_mouse::records_pages(ctx).join(" ");
    assert!(
        all.contains("The pantry: best score 300, relaxed best score 410"),
        "{all}"
    );
    assert!(
        all.contains(&format!("{}, relaxed best 520.", events::BIN_NIGHT)),
        "{all}"
    );
    assert!(!all.contains("best 0"), "{all}");
    let both = events::bin_night_help((Some(900), Some(1200)));
    assert!(
        both.ends_with(" Your best is 900. Your relaxed best is 1200."),
        "{both}"
    );
    let relaxed = events::bin_night_help((None, Some(1200)));
    assert!(
        relaxed.ends_with(" Your relaxed best is 1200."),
        "{relaxed}"
    );
    assert!(!relaxed.contains("Your best"), "{relaxed}");
    assert!(!events::bin_night_help((None, None)).contains("best"));

    h.app.ctx.push(pantry_mouse::title());
    h.frame(0.0);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.contains("Play, the pantry, best 300, relaxed best 410")),
        "{:?}",
        h.spoken()
    );
}
