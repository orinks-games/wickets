//! Pace and the speech hold: the drops and the round's clock stand still
//! while a line is being spoken and run at the run's pace, while a step or
//! a bank answers at once.
use std::cell::RefCell;
use std::rc::Rc;

use basket_run::easter::{Keepsakes, Kept};
use basket_run::progress::Progress;
use basket_run::rules::{Kind, Mode, Phase};
use basket_run::run::{score_line, short_score_line, Launch, RunScreen, RUN_NAME};
use basket_run::ID;
use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{
    Challenge, Challenges, Date, Pace, Parlor, ParlorData, Period, Records, ScoreResult,
};

const SEED: u64 = 0xE66;
/// Easter Sunday 2026, so Easter morning is open.
const EASTER: Date = Date::new(2026, 4, 5);

struct Observed(Rc<RefCell<RunScreen>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        RUN_NAME
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

type Screen = Rc<RefCell<RunScreen>>;

/// The title, with every drop already explained so no page stops play.
fn headless() -> Headless<Parlor> {
    let data = ParlorData {
        today: Some(EASTER),
        ..Default::default()
    };
    let mut h = Headless::new(data, basket_run::title_with_seed(SEED));
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    let mut progress = Progress::load(&mut h.app.ctx);
    for kind in Kind::ALL {
        progress.meet(kind);
    }
    progress.save(&mut h.app.ctx);
    h.frame(0.0);
    h
}

/// A run of `mode` on top at `pace`, its announcement said.
fn open_at(h: &mut Headless<Parlor>, mode: Mode, pace: Pace) -> Screen {
    pace::set(&mut h.app.ctx, ID, pace);
    let (seed, challenge) = match mode.period() {
        Some(period) => {
            let ch = Challenge::current(&h.app.ctx, ID, period);
            (ch.seed, Some(ch))
        }
        None => (SEED, None),
    };
    let launch = Launch {
        mode,
        seed: Some(seed),
    };
    let s = Rc::new(RefCell::new(RunScreen::new(launch, challenge)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(h);
    s
}

/// A run of `mode` at `pace`, under way: Space has skipped the lead-in.
fn playing(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    let mut h = headless();
    let s = open_at(&mut h, mode, pace);
    h.press(Key::Space);
    quiet(&mut h);
    assert_eq!(s.borrow().run.phase, Phase::Playing);
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

fn time_left(s: &Screen) -> f64 {
    s.borrow().run.time_left()
}

/// Plays on with the voice quiet until the results, reading any page in
/// the way; gives everything said.
fn play_out(h: &mut Headless<Parlor>) -> String {
    for _ in 0..40_000 {
        match h.app.top_name() {
            "results" => break,
            "pages" => h.press(Key::Return),
            RUN_NAME => {
                quiet(h);
                h.frame(0.05);
            }
            other => panic!("unexpected screen {other}"),
        }
    }
    assert_eq!(h.app.top_name(), "results");
    h.spoken().join(" ")
}

#[test]
fn normal_is_the_default_and_pace_follows_call_outs() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    let mut fresh = Headless::new(ParlorData::default(), basket_run::title_with_seed(SEED));
    assert_eq!(pace::get(&fresh.app.ctx, ID), Pace::Normal);
    fresh.frame(0.0);
    let s = open_at(&mut fresh, Mode::Meadow, Pace::Normal);
    assert_eq!(s.borrow().run_clock.pace(), Pace::Normal);
    let rows = <basket_run::BasketRun as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 2, "call-outs, then pace");
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    for pace in Pace::ALL {
        for mode in [
            Mode::Meadow,
            Mode::Endless,
            Mode::Practice,
            Mode::Easter,
            Mode::Daily,
            Mode::Weekly,
            Mode::Festival,
        ] {
            let (mut h, s) = playing(mode, pace);
            assert_eq!(s.borrow().run_clock.pace(), pace);
            let before = time_left(&s);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.4);
            assert_eq!(time_left(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().run_clock.state(), ClockState::Held);
            quiet(&mut h);
            for _ in 0..4 {
                tick(&mut h, &s, 0.1);
            }
            let ran = before - time_left(&s);
            assert!(
                (ran - 0.4 * pace.scale()).abs() < 1e-6,
                "{mode:?} at {pace:?}: {ran}"
            );
        }
    }
}

#[test]
fn the_drops_stand_still_while_a_line_is_spoken() {
    let (mut h, s) = playing(Mode::Meadow, Pace::Quick);
    for _ in 0..40 {
        quiet(&mut h);
        tick(&mut h, &s, 0.1);
        if !s.borrow().run.falling.is_empty() {
            break;
        }
    }
    let still: Vec<f64> = s.borrow().run.falling.iter().map(|f| f.left).collect();
    assert!(!still.is_empty(), "something is falling");
    for _ in 0..10 {
        h.app.ctx.say("Egg, lane 2.");
        tick(&mut h, &s, 0.1);
    }
    let now: Vec<f64> = s.borrow().run.falling.iter().map(|f| f.left).collect();
    assert_eq!(now, still);
    quiet(&mut h);
    tick(&mut h, &s, 0.1);
    let on: Vec<f64> = s.borrow().run.falling.iter().map(|f| f.left).collect();
    assert_ne!(on, still, "the drops fall once the line ends");
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = playing(Mode::Meadow, Pace::Quick);
    let before = time_left(&s);
    for _ in 0..300 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(time_left(&s) < before);
}

#[test]
fn the_wait_before_go_keeps_real_time_at_every_pace() {
    for pace in Pace::ALL {
        let mut h = headless();
        let s = open_at(&mut h, Mode::Meadow, pace);
        let Phase::LeadIn(before) = s.borrow().run.phase else {
            panic!("not in the lead-in");
        };
        h.app.ctx.say("The round's announcement, still being read.");
        tick(&mut h, &s, 0.5);
        let Phase::LeadIn(after) = s.borrow().run.phase else {
            panic!("not in the lead-in");
        };
        assert!((before - after - 0.5).abs() < 1e-9, "{pace:?}");
        assert_eq!(s.borrow().run_clock.state(), ClockState::Bypassed);
    }
}

#[test]
fn a_step_or_a_bank_during_a_hold_applies_at_once() {
    let (mut h, s) = playing(Mode::Meadow, Pace::Normal);
    h.app.ctx.say("A line still being spoken.");
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().run_clock.held());
    let cart = s.borrow().run.cart_lane();
    s.borrow_mut().run.lane = cart;
    let (away, back) = if cart == 0 {
        (Key::Right, Key::Left)
    } else {
        (Key::Left, Key::Right)
    };
    h.press(away);
    assert_ne!(s.borrow().run.lane, cart, "one step, at once");
    h.press(back);
    assert_eq!(s.borrow().run.lane, cart);
    // A bank at the cart, mid-hold, banks at once.
    s.borrow_mut().run.basket = vec![10, 10];
    let score = s.borrow().run.score;
    h.press(Key::Space);
    assert!(s.borrow().run.basket.is_empty());
    assert!(s.borrow().run.score > score);
    assert!(s.borrow().run_clock.held(), "still holding for the line");
}

#[test]
fn info_keys_hold_the_meadow_and_change_nothing_else() {
    let (mut h, s) = playing(Mode::Meadow, Pace::Quick);
    let before = time_left(&s);
    for key in [Key::S, Key::S, Key::S, Key::H, Key::R, Key::Tab] {
        h.press(key);
        tick(&mut h, &s, 0.1);
        assert!(s.borrow().run_clock.held(), "{key:?}");
    }
    assert_eq!(time_left(&s), before);
    assert_eq!(s.borrow().run.score, 0);
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = playing(Mode::Meadow, Pace::Relaxed);
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
    assert_eq!(h.app.top_name(), RUN_NAME);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.starts_with("Back in the meadow.")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().run_clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = time_left(&s);
    for _ in 0..10 {
        tick(&mut h, &s, 0.1);
        assert_eq!(time_left(&s), before, "the resume line has a fresh hold");
    }
    let (_h, s) = playing(Mode::Meadow, Pace::Quick);
    assert_eq!(s.borrow().run_clock.pace(), Pace::Quick);
}

#[test]
fn the_call_outs_off_bonus_holds_at_every_pace() {
    for pace in Pace::ALL {
        let mut h = headless();
        let mut progress = Progress::load(&mut h.app.ctx);
        progress.quiet_calls = true;
        progress.save(&mut h.app.ctx);
        let s = open_at(&mut h, Mode::Meadow, pace);
        h.press(Key::Space);
        assert!(s.borrow().run.by_ear, "{pace:?}");
        let cart = s.borrow().run.cart_lane();
        s.borrow_mut().run.lane = cart;
        s.borrow_mut().run.basket = vec![10];
        h.press(Key::Space);
        // 10 points and a tenth more.
        assert_eq!(s.borrow().run.score, 11, "{pace:?}");
    }
}

#[test]
fn a_relaxed_run_keeps_its_own_board_and_normal_and_quick_share_one() {
    let mut h = headless();
    let s = open_at(&mut h, Mode::Meadow, Pace::Relaxed);
    h.press(Key::Space);
    s.borrow_mut().run.score = 120;
    play_out(&mut h);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, "meadow-relaxed"),
        Some(120)
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, "meadow"), None);
    for (pace, score) in [(Pace::Normal, 90), (Pace::Quick, 70)] {
        h.press(Key::Escape);
        let s = open_at(&mut h, Mode::Meadow, pace);
        h.press(Key::Space);
        s.borrow_mut().run.score = score;
        h.clear();
        let said = play_out(&mut h);
        if pace == Pace::Quick {
            assert!(said.contains("Score 70, best 90."), "{said}");
        }
    }
    assert_eq!(Records::best(&mut h.app.ctx, ID, "meadow"), Some(90));
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, "meadow-relaxed"),
        Some(120)
    );
    let p = Progress::load(&mut h.app.ctx);
    assert!(p.cleared == 0, "no round was cleared");
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let mut h = headless();
    let s = open_at(&mut h, Mode::Daily, Pace::Relaxed);
    h.press(Key::Space);
    s.borrow_mut().run.score = 140;
    play_out(&mut h);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert_eq!(
        Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed),
        Some(140)
    );
}

#[test]
fn a_relaxed_easter_hunt_keeps_its_own_keepsake() {
    let mut h = headless();
    let s = open_at(&mut h, Mode::Easter, Pace::Relaxed);
    h.press(Key::Space);
    s.borrow_mut().run.score = 300;
    let said = play_out(&mut h);
    assert!(
        said.contains("Your relaxed keepsake for Easter 2026 is saved."),
        "{said}"
    );
    let ctx = &mut h.app.ctx;
    assert!(Keepsakes::load(ctx).years.is_empty());
    let relaxed = Keepsakes::load_at(ctx, Pace::Relaxed);
    assert!(relaxed.year(2026).is_some_and(|k| k.best >= 300));
    assert!(Records::best(ctx, ID, "easter morning-relaxed").is_some());
    assert_eq!(Records::best(ctx, ID, "easter morning"), None);
}

#[test]
fn result_lines_name_the_relaxed_best() {
    let first = ScoreResult::First;
    let new = ScoreResult::NewBest { previous: 40 };
    let short = ScoreResult::Short { best: 90 };
    assert_eq!(
        score_line(first, Pace::Relaxed),
        "Your first relaxed score here is saved."
    );
    assert_eq!(
        score_line(new, Pace::Relaxed),
        "A new relaxed best. The old relaxed best was 40."
    );
    assert_eq!(score_line(short, Pace::Relaxed), "Your relaxed best is 90.");
    assert_eq!(score_line(short, Pace::Quick), "Your best is 90.");
    assert_eq!(
        short_score_line(70, short, Pace::Relaxed),
        "Score 70, relaxed best 90."
    );
    assert_eq!(
        short_score_line(70, new, Pace::Normal),
        "Score 70, a new best."
    );
    assert_eq!(
        Kept::Short { best: 90 }.line(2026, Pace::Relaxed),
        "Your relaxed best for Easter 2026 is 90."
    );
}

#[test]
fn records_and_the_title_name_both_boards() {
    let mut h = headless();
    let ctx = &mut h.app.ctx;
    Records::record(ctx, ID, "meadow", 300);
    Records::record(ctx, ID, "meadow-relaxed", 410);
    Records::record(ctx, ID, "easter morning-relaxed", 520);
    let mut relaxed = Keepsakes::default();
    relaxed.keep(2026, 520, 6);
    relaxed.store_at(ctx, Pace::Relaxed);
    let all = basket_run::records_pages(ctx).join(" ");
    assert!(
        all.contains("The meadow: best 300, relaxed best 410,"),
        "{all}"
    );
    assert!(
        all.contains(
            "Relaxed keepsakes. Easter 2026: relaxed best 520, 6 golden eggs in one hunt."
        ),
        "{all}"
    );
    assert!(!all.contains("best 0"), "{all}");
    let help = basket_run::easter::row_help(EASTER, (None, Some(520)));
    assert!(help.ends_with(" Your relaxed best is 520."), "{help}");
    assert!(!help.contains("Your best"), "{help}");

    h.app.ctx.pop();
    h.app.ctx.push(basket_run::title_with_seed(SEED));
    h.clear();
    h.frame(0.0);
    let rows = parlor_core::menus::rows_of(&h.app.top_lines());
    assert!(
        rows.iter()
            .any(|r| r.contains("the meadow, best 300, relaxed best 410")),
        "{rows:?}"
    );
}

#[test]
fn an_idle_relaxed_run_scores_0_and_saves_no_relaxed_best() {
    let (mut h, _s) = playing(Mode::Meadow, Pace::Relaxed);
    let said = play_out(&mut h);
    assert!(
        said.contains("Score 0, so nothing goes on the board."),
        "{said}"
    );
    assert!(!said.contains("saved"), "{said}");
    let relaxed = Pace::Relaxed.board("meadow");
    assert_eq!(Records::best(&mut h.app.ctx, ID, &relaxed), None);
    assert_eq!(
        basket_run::progress::best_line(&mut h.app.ctx, "meadow"),
        "no score yet"
    );
    // A relaxed 0 saved before this counts as no score: the next score is
    // the first, not "a new best, the old best was 0".
    Records::record(&mut h.app.ctx, ID, &relaxed, 0);
    assert_eq!(
        basket_run::progress::best_line(&mut h.app.ctx, "meadow"),
        "no score yet"
    );
    let r = pace::record(&mut h.app.ctx, ID, &relaxed, 33).unwrap();
    assert_eq!(
        score_line(r, Pace::Relaxed),
        "Your first relaxed score here is saved."
    );
}
