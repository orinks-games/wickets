//! Pace and the speech hold: the pools and the tide stand still while a
//! line is being spoken and run at the trip's pace.
use std::cell::RefCell;
use std::rc::Rc;

use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Pace, Parlor, Period, Records, ScoreResult};
use tide_pools::events::{self, Event};
use tide_pools::outing::{challenge_outing, outing_for, OutingScreen, OUTING_NAME};
use tide_pools::rules::{self, Mode, Place, Species};
use tide_pools::{sound, ID};

use crate::common::{self, SEED};

struct Observed(Rc<RefCell<OutingScreen>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        OUTING_NAME
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

type Screen = Rc<RefCell<OutingScreen>>;

/// A trip of `mode` on top at `pace`, its opening said.
fn open_at(h: &mut Headless<Parlor>, screen: OutingScreen, pace: Pace) -> Screen {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(screen));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(h);
    s
}

fn open(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    let mut h = common::title();
    rules::fix_seed(Some(SEED));
    let screen = match mode {
        Mode::Challenge(period) => {
            let ch = Challenge::current(&h.app.ctx, ID, period);
            OutingScreen::new(challenge_outing(&ch), Some(ch))
        }
        _ => OutingScreen::new(outing_for(mode), None),
    };
    let s = open_at(&mut h, screen, pace);
    (h, s)
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

/// One frame of `dt` straight to the screen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, s: &Screen, dt: f64) {
    s.borrow_mut().update(&mut h.app.ctx, dt);
}

fn elapsed(s: &Screen) -> f64 {
    s.borrow().outing.elapsed
}

/// Brings the tide in at once on a trip that scored `points`, and gives
/// the result page.
fn tide_in(h: &mut Headless<Parlor>, s: &Screen, points: u64) -> String {
    {
        let o = &mut s.borrow_mut().outing;
        o.points = points;
        o.tide_left = 0.05;
    }
    h.clear();
    for _ in 0..10 {
        if h.app.top_name() != OUTING_NAME {
            break;
        }
        quiet(h);
        h.frame(0.1);
    }
    assert_eq!(h.app.top_name(), "pages");
    let said = h.spoken().join(" ");
    common::read_pages(h);
    said
}

#[test]
fn normal_is_the_default_and_settings_has_the_pace_row() {
    let mut h = common::base();
    Records::store(&mut h.app.ctx, &pace::key(ID), &None::<Pace>);
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = open_at(
        &mut h,
        OutingScreen::new(outing_for(Mode::Beach(Place::Cove)), None),
        Pace::Normal,
    );
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
    let rows = <tide_pools::TidePools as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 1);
}

#[test]
fn every_trip_holds_for_speech_and_runs_at_its_pace() {
    for pace in Pace::ALL {
        for mode in [
            Mode::Beach(Place::Cove),
            Mode::Beach(Place::Reef),
            Mode::Practice,
            Mode::Event(Event::KingTide, Place::Cove),
            Mode::Event(Event::Moonlit, Place::Point),
            Mode::Challenge(Period::Daily),
            Mode::Challenge(Period::Weekly),
            Mode::Challenge(Period::Monthly),
        ] {
            let (mut h, s) = open(mode, pace);
            assert_eq!(s.borrow().clock.pace(), pace);
            let (before, tide) = (elapsed(&s), s.borrow().outing.tide_left);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.4);
            assert_eq!(elapsed(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().outing.tide_left, tide, "the tide holds");
            assert_eq!(s.borrow().clock.state(), ClockState::Held);
            quiet(&mut h);
            for _ in 0..4 {
                tick(&mut h, &s, 0.1);
            }
            let ran = elapsed(&s) - before;
            assert!(
                (ran - 0.4 * pace.scale()).abs() < 1e-6,
                "{mode:?} at {pace:?}: {ran}"
            );
        }
    }
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = open(Mode::Beach(Place::Cove), Pace::Quick);
    let before = elapsed(&s);
    for _ in 0..300 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(elapsed(&s) > before);
}

#[test]
fn the_time_to_reach_in_waits_for_a_line_and_space_gathers_at_once() {
    // The practice pools: the anemone in front, never shy.
    let (mut h, s) = open(Mode::Practice, Pace::Normal);
    h.press(Key::Up);
    quiet(&mut h);
    let anemone = sound::creature(Species::Anemone);
    h.clear();
    for _ in 0..200 {
        if h.played().iter().any(|k| k == anemone) {
            break;
        }
        quiet(&mut h);
        tick(&mut h, &s, 0.05);
    }
    assert!(
        h.played().iter().any(|k| k == anemone),
        "the anemone called"
    );
    // Longer than the time to reach in, all of it spoken over.
    for _ in 0..40 {
        h.app.ctx.say("A line still being spoken.");
        tick(&mut h, &s, 0.1);
    }
    assert!(s.borrow().clock.held());
    h.press(Key::Space);
    assert_eq!(s.borrow().outing.gathered, vec![Species::Anemone]);
}

#[test]
fn info_keys_hold_the_shore_and_change_nothing_else() {
    let (mut h, s) = open(Mode::Beach(Place::Cove), Pace::Quick);
    let before = elapsed(&s);
    for key in [Key::S, Key::S, Key::S, Key::H, Key::R, Key::Tab] {
        h.press(key);
        tick(&mut h, &s, 0.1);
        assert!(s.borrow().clock.held(), "{key:?}");
    }
    assert_eq!(elapsed(&s), before);
    assert_eq!(s.borrow().outing.score(), 0);
}

#[test]
fn pace_stays_fixed_for_the_trip_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = open(Mode::Beach(Place::Cove), Pace::Relaxed);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), OUTING_NAME);
    assert!(
        h.spoken().iter().any(|l| l.starts_with("Back at pool 1")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = elapsed(&s);
    for _ in 0..10 {
        tick(&mut h, &s, 0.1);
        assert_eq!(elapsed(&s), before, "the resume line has a fresh hold");
    }
    let (_h, s) = open(Mode::Beach(Place::Cove), Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

#[test]
fn a_relaxed_trip_keeps_its_own_board_and_normal_and_quick_share_one() {
    let (mut h, s) = open(Mode::Beach(Place::Cove), Pace::Relaxed);
    let said = tide_in(&mut h, &s, 30);
    assert!(
        said.contains("Your first relaxed score at Sandy Cove."),
        "{said}"
    );
    let board = Place::Cove.board();
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(board)),
        Some(30)
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, board), None);
    let mut kept = Vec::new();
    for (pace, points) in [(Pace::Normal, 20), (Pace::Quick, 10)] {
        h.press(Key::Escape);
        rules::fix_seed(Some(SEED));
        let s = open_at(
            &mut h,
            OutingScreen::new(outing_for(Mode::Beach(Place::Cove)), None),
            pace,
        );
        kept.push(tide_in(&mut h, &s, points));
    }
    assert!(
        kept[1].contains("Your best at Sandy Cove is 20."),
        "{kept:?}"
    );
    assert!(!kept[1].contains("relaxed"), "{kept:?}");
    assert_eq!(Records::best(&mut h.app.ctx, ID, board), Some(20));
}

#[test]
fn a_relaxed_event_trip_keeps_its_own_board() {
    let (mut h, s) = open(Mode::Event(Event::Moonlit, Place::Cove), Pace::Relaxed);
    let said = tide_in(&mut h, &s, 40);
    assert!(
        said.contains("Your first relaxed score on the moonlit pools."),
        "{said}"
    );
    let board = Event::Moonlit.board();
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(board)),
        Some(40)
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, board), None);
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, s) = open(Mode::Challenge(Period::Daily), Pace::Relaxed);
    let said = tide_in(&mut h, &s, 50);
    assert!(
        said.contains("Your first relaxed score on this one."),
        "{said}"
    );
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert!(Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed).is_some());
    // A better relaxed run says so in relaxed words.
    let r = Challenges::submit_at(&mut h.app.ctx, &ch, Pace::Relaxed, 10_000);
    assert!(matches!(r, ScoreResult::NewBest { .. }));
}

#[test]
fn records_the_title_and_event_rows_name_both_boards() {
    let mut h = common::title();
    let ctx = &mut h.app.ctx;
    let board = Place::Cove.board();
    Records::record(ctx, ID, board, 120);
    Records::record(ctx, ID, &Pace::Relaxed.board(board), 140);
    Records::record(ctx, ID, &Pace::Relaxed.board(Event::KingTide.board()), 90);
    let all = tide_pools::records_pages(ctx).join(" ");
    assert!(
        all.contains("Sandy Cove: best 120, relaxed best 140."),
        "{all}"
    );
    assert!(
        all.contains(&format!("{}: relaxed best 90.", Event::KingTide.name())),
        "{all}"
    );
    assert!(!all.contains("best 0"), "{all}");
    let today = parlor_core::Date::new(2026, 10, 16);
    let help = events::row_help(Event::KingTide, today, "Sandy Cove", (None, Some(90)));
    assert!(help.contains(" Your relaxed best is 90."), "{help}");
    assert!(!help.contains("Your best"), "{help}");

    h.app.ctx.pop();
    h.app.ctx.push(tide_pools::title());
    h.clear();
    h.frame(0.0);
    let rows = parlor_core::menus::rows_of(&h.app.top_lines());
    assert!(
        rows.iter()
            .any(|r| r.contains("Sandy Cove, best 120, relaxed best 140")),
        "{rows:?}"
    );
}

#[test]
fn status_from_the_first_frame_holds_only_for_the_budget() {
    // The status key every second from the first frame, the voice never
    // quiet: the danger moves once the information budget is spent.
    let (mut h, s) = open(Mode::Beach(Place::Cove), Pace::Normal);
    let before = elapsed(&s);
    let t = parlor_core::testkit::status_spam_until(
        &mut h,
        60,
        |h| tick(h, &s, 0.1),
        |_| elapsed(&s) > before,
    );
    let t = t.expect("the danger never moved");
    assert!(t <= parlor_core::testkit::SPAM_BOUND_S, "{t}");
}
