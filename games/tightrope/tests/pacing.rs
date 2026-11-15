//! Pace and the speech hold: the wire stands still while a line is being
//! spoken and runs at the run's pace, and a gust's whoosh still peaks as
//! the gust hits.
use std::cell::RefCell;
use std::rc::Rc;

use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records, Rng};
use tightrope::rules::{Gusts, Level, Mode, Modifier, Pigeons, Plan, Run, RISER};
use tightrope::screens::{Launch, RunScreen};
use tightrope::{cues, progress, ID};

const SEED: u64 = 7;

struct Observed(Rc<RefCell<RunScreen>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "tightrope_observed"
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

type Screen = Rc<RefCell<RunScreen>>;

fn headless() -> Headless<Parlor> {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
    h
}

/// A still wire of `steps` steps: no lean grows by itself, and the wind
/// and pigeons come only when asked for.
fn still_level(steps: u32, gusts: Option<Gusts>, pigeons: Option<Pigeons>) -> Level {
    Level {
        number: 1,
        from: "here",
        to: "there",
        floors: 3,
        steps,
        instability: 0.0,
        drift: 0.0,
        sway: 0.0,
        sway_period: 4.0,
        gusts,
        pigeons,
    }
}

/// One gust soon after the calm, warned `warning` seconds ahead.
fn one_gust(warning: f64) -> Gusts {
    Gusts {
        first: 0.2,
        gap: (1e9, 1e9),
        strength: 0.1,
        warning,
    }
}

/// `screen` on screen at `pace`, its opening said.
fn open(mut h: Headless<Parlor>, screen: RunScreen, pace: Pace) -> (Headless<Parlor>, Screen) {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(screen));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(&mut h);
    (h, s)
}

fn launch(mode: Mode) -> Launch {
    Launch {
        mode,
        seed: Some(SEED),
    }
}

/// A run of `mode` on `level`.
fn on_level(mode: Mode, level: Level, pace: Pace) -> (Headless<Parlor>, Screen) {
    let h = headless();
    let challenge = match mode {
        Mode::Challenge(p) => Some(Challenge::current(&h.app.ctx, ID, p)),
        _ => None,
    };
    let mut screen = RunScreen::new(launch(mode), challenge);
    let plan = Plan {
        mode,
        levels: vec![level],
        lives: 3,
        modifier: Modifier::None,
    };
    *screen.run_mut() = Run::new(plan, Rng::seeded(SEED));
    open(h, screen, pace)
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
    s.borrow().run().total_t
}

fn warn_left(s: &Screen) -> Option<f64> {
    s.borrow().run().gust.as_ref().map(|g| g.warn_left)
}

fn played_count(h: &Headless<Parlor>, key: &str) -> usize {
    h.played().iter().filter(|k| k.as_str() == key).count()
}

/// Quiet frames until `line` is said.
fn until_said(h: &mut Headless<Parlor>, s: &Screen, line: &str) {
    for _ in 0..2000 {
        if h.spoken().iter().any(|l| l.contains(line)) {
            return;
        }
        quiet(h);
        tick(h, s, 0.05);
    }
    panic!("never said {line:?}: {:?}", h.spoken());
}

#[test]
fn normal_is_the_default_pace() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = Rc::new(RefCell::new(RunScreen::new(launch(Mode::Skyline), None)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().run_clock.pace(), Pace::Normal);
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    let modes = [
        Mode::Skyline,
        Mode::Night,
        Mode::Single(3),
        Mode::Training,
        Mode::Challenge(Period::Daily),
        Mode::Challenge(Period::Monthly),
    ];
    for mode in modes {
        for pace in Pace::ALL {
            let h = headless();
            let challenge = match mode {
                Mode::Challenge(p) => Some(Challenge::current(&h.app.ctx, ID, p)),
                _ => None,
            };
            let launch = Launch {
                mode,
                seed: challenge.as_ref().map(|c| c.seed).or(Some(SEED)),
            };
            let (mut h, s) = open(h, RunScreen::new(launch, challenge), pace);
            assert_eq!(s.borrow().run_clock.pace(), pace);
            let before = time(&s);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.2);
            assert_eq!(time(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().run_clock.state(), ClockState::Held);
            quiet(&mut h);
            tick(&mut h, &s, 0.2);
            let ran = time(&s) - before;
            assert!(
                (ran - 0.2 * pace.scale()).abs() < 1e-9,
                "{mode:?} at {pace:?}: {ran}"
            );
        }
    }
}

#[test]
fn the_wire_holds_while_a_gust_is_called() {
    let (mut h, s) = on_level(
        Mode::Single(2),
        still_level(10, Some(one_gust(3.0)), None),
        Pace::Quick,
    );
    until_said(&mut h, &s, "Gust from the");
    let left = warn_left(&s).expect("a gust on its way");
    for _ in 0..10 {
        tick(&mut h, &s, 0.05);
        assert!(s.borrow().run_clock.held(), "the call is being said");
        assert_eq!(warn_left(&s), Some(left));
    }
    quiet(&mut h);
    tick(&mut h, &s, 0.05);
    assert!(warn_left(&s).unwrap() < left);
}

#[test]
fn a_pigeon_landing_holds_the_wire() {
    let pigeons = Pigeons {
        first: 0.2,
        gap: (1e9, 1e9),
    };
    let (mut h, s) = on_level(
        Mode::Single(2),
        still_level(10, None, Some(pigeons)),
        Pace::Quick,
    );
    until_said(&mut h, &s, "A pigeon lands");
    let before = time(&s);
    for _ in 0..10 {
        tick(&mut h, &s, 0.05);
        assert!(s.borrow().run_clock.held(), "the landing is being said");
    }
    assert_eq!(time(&s), before);
}

/// Real seconds from the whoosh starting to the gust hitting, at `pace`,
/// with a voice that keeps talking once the whoosh has started.
fn whoosh_to_blast(pace: Pace) -> f64 {
    let (mut h, s) = on_level(
        Mode::Single(2),
        still_level(10, Some(one_gust(3.0)), None),
        pace,
    );
    until_said(&mut h, &s, "Gust from the");
    h.clear();
    let dt = 0.01;
    for _ in 0..2000 {
        if played_count(&h, cues::GUST_WARNING) > 0 {
            break;
        }
        quiet(&mut h);
        tick(&mut h, &s, dt);
    }
    assert_eq!(played_count(&h, cues::GUST_WARNING), 1, "{pace:?}");
    let left = warn_left(&s).unwrap();
    assert!(
        (left - RISER * pace.scale()).abs() < 0.02,
        "{pace:?}: whoosh with {left} game seconds to go"
    );
    let mut real = 0.0;
    for _ in 0..2000 {
        if played_count(&h, cues::GUST_BLAST) > 0 {
            break;
        }
        // Speech does not stop the wire while the whoosh climbs.
        h.app.ctx.say("A line spoken over the whoosh.");
        tick(&mut h, &s, dt);
        assert_ne!(s.borrow().run_clock.state(), ClockState::Held);
        real += dt;
    }
    assert!(played_count(&h, cues::GUST_BLAST) > 0, "{pace:?}");
    assert_eq!(played_count(&h, cues::GUST_WARNING), 1, "one whoosh a gust");
    real
}

#[test]
fn the_whoosh_peaks_as_the_gust_hits_at_every_pace() {
    for pace in Pace::ALL {
        let real = whoosh_to_blast(pace);
        assert!((real - RISER).abs() < 0.05, "{pace:?}: {real}");
    }
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = on_level(Mode::Single(2), still_level(10, None, None), Pace::Quick);
    let before = time(&s);
    for _ in 0..600 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(time(&s) - before > 10.0, "{}", time(&s) - before);
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = on_level(Mode::Single(2), still_level(10, None, None), Pace::Relaxed);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "tightrope_observed");
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "tightrope_observed");
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.starts_with("Back on the wire.")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().run_clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = time(&s);
    for _ in 0..30 {
        tick(&mut h, &s, 0.1);
        assert_eq!(time(&s), before, "the resume line has a fresh hold");
    }
    // The next run takes the new pace.
    let (_h, s) = on_level(Mode::Single(2), still_level(10, None, None), Pace::Quick);
    assert_eq!(s.borrow().run_clock.pace(), Pace::Quick);
}

/// Walks the still wire to its end and reads the result pages.
fn cross(h: &mut Headless<Parlor>) -> String {
    for _ in 0..200 {
        if h.app.top_name() == "pages" {
            break;
        }
        h.press(Key::Space);
        quiet(h);
        h.frame(0.25);
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
fn a_relaxed_crossing_keeps_its_own_score_and_time() {
    let (mut h, _s) = on_level(Mode::Single(1), still_level(3, None, None), Pace::Relaxed);
    let pages = cross(&mut h);
    let ctx = &mut h.app.ctx;
    assert!(Records::best(ctx, ID, "single 1-relaxed").is_some());
    assert_eq!(Records::best(ctx, ID, "single 1"), None);
    assert!(progress::best_time(ctx, "crossing 1-relaxed").is_some());
    assert_eq!(progress::best_time(ctx, "crossing 1"), None);
    assert_eq!(
        progress::furthest(ctx),
        1,
        "the furthest counts at every pace"
    );
    assert!(
        pages.contains("First relaxed Crossing 1 score saved"),
        "{pages}"
    );
    assert!(
        pages.contains("First relaxed time on crossing 1"),
        "{pages}"
    );
}

#[test]
fn normal_and_quick_share_a_board() {
    let (mut h, _s) = on_level(Mode::Single(1), still_level(3, None, None), Pace::Normal);
    cross(&mut h);
    assert!(Records::best(&mut h.app.ctx, ID, "single 1").is_some());
    let mut screen = RunScreen::new(launch(Mode::Single(1)), None);
    let plan = Plan {
        mode: Mode::Single(1),
        levels: vec![still_level(3, None, None)],
        lives: 3,
        modifier: Modifier::None,
    };
    *screen.run_mut() = Run::new(plan, Rng::seeded(SEED));
    let (mut h, _s) = open(h, screen, Pace::Quick);
    let pages = cross(&mut h);
    // Quick is faster, so it beats the normal score on the same board.
    assert!(pages.contains("New best Crossing 1 score"), "{pages}");
    assert_eq!(Records::best(&mut h.app.ctx, ID, "single 1-relaxed"), None);
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, _s) = on_level(
        Mode::Challenge(Period::Daily),
        still_level(3, None, None),
        Pace::Relaxed,
    );
    let pages = cross(&mut h);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert!(Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed).is_some());
    assert!(
        pages.contains("Your first relaxed score on this one."),
        "{pages}"
    );
}

#[test]
fn records_and_the_title_name_both_boards() {
    let mut h = headless();
    Records::record(&mut h.app.ctx, ID, "skyline", 30);
    Records::record(&mut h.app.ctx, ID, "skyline-relaxed", 41);
    Records::record(&mut h.app.ctx, ID, progress::FURTHEST, 2);
    progress::record_time(&mut h.app.ctx, "crossing 1", 20.0);
    progress::record_time(&mut h.app.ctx, "crossing 1-relaxed", 35.0);
    progress::record_time(&mut h.app.ctx, "crossing 2-relaxed", 44.0);
    let all = tightrope::records_pages(&mut h.app.ctx).join(" ");
    assert!(
        all.contains("Skyline run: best 30, relaxed best 41, furthest crossing 2"),
        "{all}"
    );
    assert!(
        all.contains("Crossing 1, best 20.0 seconds, relaxed best 35.0 seconds. Crossing 2, relaxed best 44.0 seconds. Crossing 3, no time yet."),
        "{all}"
    );
    assert!(!all.contains("best 0"), "{all}");

    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(tightrope::title_with_seed(SEED));
    h.frame(0.0);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.contains("Play, skyline run, best 30, relaxed best 41, furthest crossing 2")),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn settings_has_the_pace_row() {
    let h = headless();
    let rows = <tightrope::Tightrope as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 1);
}

#[test]
fn a_step_pressed_while_the_wire_is_held_is_taken_once_it_runs() {
    let (mut h, s) = on_level(Mode::Single(2), still_level(10, None, None), Pace::Normal);
    let pos = |s: &Screen| s.borrow().run().pos;
    h.press(Key::Space);
    assert_eq!(pos(&s), 1);
    // The step's cooldown has not run out, and a line holds the wire.
    h.app.ctx.say("Still talking.");
    tick(&mut h, &s, 0.05);
    assert!(s.borrow().run_clock.held());
    let refused = played_count(&h, cues::REFUSED);
    h.press(Key::Space);
    assert_eq!(pos(&s), 1);
    assert_eq!(played_count(&h, cues::REFUSED), refused, "not refused");
    for _ in 0..20 {
        tick(&mut h, &s, 0.05);
    }
    assert_eq!(pos(&s), 1, "still held");
    quiet(&mut h);
    for _ in 0..20 {
        tick(&mut h, &s, 0.05);
    }
    assert_eq!(pos(&s), 2, "the press was kept");
}

/// Crossing a still wire of 4 steps at quick, half a second between
/// steps. With `spam`, a status line is spoken before every step, and the
/// wire is held while it is. Gives the crossing's saved time, the score,
/// the arrival line and the time held for speech.
fn cross_still(spam: bool) -> (f64, u64, String, f64) {
    let (mut h, s) = on_level(Mode::Single(1), still_level(4, None, None), Pace::Quick);
    let mut held = 0.0;
    for _ in 0..4 {
        if spam {
            for _ in 0..3 {
                h.app.ctx.say("Step 1 of 4.");
                tick(&mut h, &s, 0.1);
                held += 0.1;
            }
        }
        h.press(Key::Space);
        quiet(&mut h);
        tick(&mut h, &s, 0.5);
    }
    let run = s.borrow();
    let &(_, time) = run.run().times.last().expect("crossed");
    let line = h
        .spoken()
        .iter()
        .find(|l| l.starts_with("Across."))
        .cloned()
        .unwrap_or_default();
    (time, run.run().score, line, held)
}

#[test]
fn info_keys_never_improve_a_crossing_time_or_its_par_bonus() {
    let (time, score, line, _) = cross_still(false);
    assert!(
        (time - 1.5).abs() < 1e-6,
        "with nothing said, as before: {time}"
    );
    assert!(
        line.contains(&format!(" {time:.1} seconds, par ")),
        "{line}"
    );
    let (spam_time, spam_score, spam_line, held) = cross_still(true);
    assert!((spam_time - (time + held)).abs() < 1e-6, "{spam_time}");
    // The par bonus is worked out from the time with the hold counted.
    let par = still_level(4, None, None).par();
    let bonus = |t: f64| ((par - t).max(0.0) * 5.0) as u64;
    assert_eq!(spam_score, score - (bonus(time) - bonus(spam_time)));
    assert!(spam_score < score);
    // What is said is what is saved.
    assert!(
        spam_line.contains(&format!(" {spam_time:.1} seconds, par ")),
        "{spam_line}"
    );
    assert!(
        spam_line.ends_with(&format!("Score {spam_score}.")),
        "{spam_line}"
    );
}

#[test]
fn status_from_the_first_frame_holds_only_for_the_budget() {
    // The status key every second from the first frame, the voice never
    // quiet: the danger moves once the information budget is spent.
    let (mut h, s) = on_level(Mode::Single(2), still_level(10, None, None), Pace::Normal);
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

#[test]
fn a_run_that_scores_0_goes_on_no_board() {
    let mut h = headless();
    let ctx = &mut h.app.ctx;
    // An old save that kept a 0.
    Records::record(ctx, ID, "skyline", 0);
    let all = tightrope::records_pages(ctx).join(" ");
    assert!(all.contains("Skyline run: no score yet"), "{all}");
    let mut run = Run::of(Mode::Skyline, Rng::seeded(1));
    let lines = progress::save_run(ctx, &run, Pace::Normal);
    assert!(
        lines.contains(&"Score 0, so nothing goes on the board.".to_string()),
        "{lines:?}"
    );
    run.score = 40;
    let lines = progress::save_run(ctx, &run, Pace::Normal);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("First Skyline run score saved")),
        "{lines:?}"
    );
}
