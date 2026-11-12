//! Pace and the speech hold: the rail stands still while a line is being
//! spoken, and runs at the run's pace.
use hearken::{Ctx, Headless, InputEvent, Key, State};
use neon_rail::profile::Profile;
use neon_rail::rules::{Loadout, Mode, Route, Run};
use neon_rail::screens::{Launch, RunScreen};
use neon_rail::ID;
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};
use std::cell::RefCell;
use std::rc::Rc;

struct Observed(Rc<RefCell<RunScreen>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "neon_rail_observed"
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

/// A Friday that is not the last of its month, far from any event.
const TODAY: Date = Date::new(2026, 10, 16);

fn headless() -> Headless<Parlor> {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(TODAY);
    Profile {
        tutorial_complete: true,
        runs_completed: 1,
        ..Profile::default()
    }
    .save(&mut h.app.ctx);
    h
}

/// A run of `mode` at `pace`, its opening line said.
fn start(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    let mut h = headless();
    pace::set(&mut h.app.ctx, ID, pace);
    let launch = Launch {
        mode,
        seed: Some(7),
        day: Some(TODAY),
    };
    let screen = RunScreen::start(&mut h.app.ctx, launch);
    let s = Rc::new(RefCell::new(screen));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(&mut h);
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

fn t(s: &Screen) -> f64 {
    s.borrow().run().map_or(-1.0, |r| r.t)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// Quiet frames until the run is over and its debrief is up.
fn run_out(h: &mut Headless<Parlor>, s: &Screen) {
    for _ in 0..4000 {
        if h.app.top_name() != "neon_rail_observed" {
            return;
        }
        quiet(h);
        tick(h, s, 0.25);
        h.frame(0.0);
    }
    panic!("the run never ended");
}

/// Every page of the debrief, read with Enter, as one string.
fn read_pages(h: &mut Headless<Parlor>) -> String {
    let mut said = h.spoken().to_vec();
    for _ in 0..30 {
        if h.app.top_name() != "pages" {
            break;
        }
        h.clear();
        h.press(Key::Return);
        said.extend(h.spoken().iter().cloned());
    }
    said.join(" ")
}

/// Quiet frames on whatever screen is on top until the debrief is up,
/// taking the first upgrade and route at each intermission. True when
/// there was one.
fn fly_out(h: &mut Headless<Parlor>) -> bool {
    let mut between = false;
    for _ in 0..20000 {
        match h.app.top_name() {
            "pages" => return between,
            "neon_rail_upgrade" | "neon_rail_route" => {
                between = true;
                h.press(Key::Return);
            }
            _ => {}
        }
        quiet(h);
        h.frame(0.25);
    }
    panic!("the run never ended: {}", h.app.top_name());
}

#[test]
fn normal_is_the_default_and_settings_has_the_pace_row() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    let launch = Launch {
        mode: Mode::BlackSky,
        seed: Some(7),
        day: None,
    };
    let screen = RunScreen::start(&mut h.app.ctx, launch);
    assert_eq!(screen.clock.pace(), Pace::Normal);
    let rows = <neon_rail::NeonRail as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 2, "call-outs, then pace");
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    let modes = [
        Mode::BlackSky,
        Mode::Challenge,
        Mode::Endless,
        Mode::ColdStart,
        Mode::Tutorial,
        Mode::FridayNight,
        Mode::Daily,
        Mode::Weekly,
        Mode::Monthly,
    ];
    for mode in modes {
        for pace in Pace::ALL {
            let (mut h, s) = start(mode, pace);
            assert_eq!(s.borrow().clock.pace(), pace);
            let before = t(&s);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.2);
            assert_eq!(t(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().clock.state(), ClockState::Held);
            quiet(&mut h);
            tick(&mut h, &s, 0.2);
            assert!(
                close(t(&s) - before, 0.2 * pace.scale()),
                "{mode:?} at {pace:?}: {}",
                t(&s) - before
            );
        }
    }
}

#[test]
fn each_arrival_gets_a_full_hold_however_long_the_lines_before_it() {
    let (mut h, s) = start(Mode::BlackSky, Pace::Quick);
    let announced = |s: &Screen| {
        s.borrow()
            .run()
            .map_or(0, |r| r.events.iter().filter(|e| e.announced).count())
    };
    // A voice that never stops: the hold expires and the rail runs on.
    for _ in 0..2000 {
        if announced(&s) > 0 {
            break;
        }
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(announced(&s) > 0, "an arrival came");
    let before = t(&s);
    h.app.ctx.say("Debris center.");
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held(), "the call-out has a fresh hold");
    assert_eq!(t(&s), before);
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = start(Mode::Challenge, Pace::Quick);
    // Each arrival starts a fresh hold, so a voice that never stops slows
    // the rail a great deal, but never stops it.
    for _ in 0..300 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(t(&s) > 1.0, "{}", t(&s));
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = start(Mode::BlackSky, Pace::Relaxed);
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "neon_rail_observed");
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    // The resume line has a full hold of its own.
    let before = t(&s);
    for _ in 0..10 {
        tick(&mut h, &s, 0.1);
    }
    assert_eq!(t(&s), before);
    // The sectors after an intermission keep the run's pace.
    let run = Run::challenge(Loadout::default(), 0);
    let launch = Launch {
        mode: Mode::Challenge,
        seed: Some(7),
        day: None,
    };
    let next = RunScreen::continuing(run, launch, Route::Standard, Pace::Relaxed);
    assert_eq!(next.clock.pace(), Pace::Relaxed);
    // The next run takes the new pace.
    let (_h, s) = start(Mode::BlackSky, Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

#[test]
fn a_relaxed_run_keeps_its_own_board_and_personal_best() {
    let (mut h, s) = start(Mode::Challenge, Pace::Relaxed);
    run_out(&mut h, &s);
    let relaxed = Records::best(
        &mut h.app.ctx,
        ID,
        &Pace::Relaxed.board(Mode::Challenge.board()),
    );
    assert!(relaxed.is_some());
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, Mode::Challenge.board()),
        None
    );
    let p = Profile::load(&mut h.app.ctx);
    assert_eq!(p.best_score, 0);
    assert_eq!(p.relaxed_best_score, relaxed.unwrap());
    let said = read_pages(&mut h);
    assert!(
        said.contains("on the relaxed Challenge corridor board"),
        "{said}"
    );
}

#[test]
fn a_relaxed_run_keeps_its_pace_through_its_intermissions() {
    let (mut h, _s) = start(Mode::BlackSky, Pace::Relaxed);
    assert!(fly_out(&mut h), "the run reached an intermission");
    assert!(Records::best(
        &mut h.app.ctx,
        ID,
        &Pace::Relaxed.board(Mode::BlackSky.board())
    )
    .is_some());
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, Mode::BlackSky.board()),
        None
    );
}

#[test]
fn normal_and_quick_share_the_board() {
    let (mut h, s) = start(Mode::Challenge, Pace::Normal);
    run_out(&mut h, &s);
    assert!(Records::best(&mut h.app.ctx, ID, Mode::Challenge.board()).is_some());
    assert_eq!(
        Pace::Quick.board(Mode::Challenge.board()),
        Mode::Challenge.board()
    );
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, s) = start(Mode::Daily, Pace::Relaxed);
    run_out(&mut h, &s);
    let ch = Challenge::on(TODAY, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert!(Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed).is_some());
    let said = read_pages(&mut h);
    assert!(
        said.contains("Your first recorded relaxed result."),
        "{said}"
    );
}

#[test]
fn records_and_the_title_name_both_boards() {
    let mut h = headless();
    Records::record(&mut h.app.ctx, ID, Mode::BlackSky.board(), 500);
    Records::record(
        &mut h.app.ctx,
        ID,
        &Pace::Relaxed.board(Mode::BlackSky.board()),
        300,
    );
    Records::record(
        &mut h.app.ctx,
        ID,
        &Pace::Relaxed.board(Mode::Challenge.board()),
        200,
    );
    let all = neon_rail::records_pages(&mut h.app.ctx).join(" ");
    assert!(
        all.contains("Black Sky Run: best 500, relaxed best 300."),
        "{all}"
    );
    assert!(
        all.contains("Challenge corridor: relaxed best 200."),
        "{all}"
    );
    assert!(!all.contains("best 0"), "{all}");
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(neon_rail::title());
    h.frame(0.0);
    let said = h.spoken().join(" ");
    assert!(
        said.contains("Play, Black Sky Run, best 500, relaxed best 300"),
        "{said}"
    );
}

#[test]
fn the_rail_gun_fired_while_the_rail_is_held_fires_once_it_has_cooled() {
    use neon_rail::cues;
    let (mut h, s) = start(Mode::BlackSky, Pace::Normal);
    let shots = |h: &Headless<Parlor>| {
        h.played()
            .iter()
            .filter(|k| *k == cues::RAILGUN || *k == cues::NO_LOCK)
            .count()
    };
    h.press(Key::F);
    assert_eq!(shots(&h), 1);
    let ready = |s: &Screen| s.borrow().run().unwrap().weapon_ready_in() <= 0.0;
    assert!(!ready(&s), "the rail gun is cooling");
    // A line holds the rail, so the cooldown stands still.
    h.app.ctx.say("Still talking.");
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held());
    h.clear();
    h.press(Key::F);
    assert!(!h.played().iter().any(|k| k == cues::DENIED), "not refused");
    assert!(
        !h.spoken()
            .iter()
            .any(|l| l.starts_with("Rail gun cooling.")),
        "{:?}",
        h.spoken()
    );
    quiet(&mut h);
    for _ in 0..40 {
        tick(&mut h, &s, 0.05);
    }
    assert_eq!(shots(&h), 1, "the press was kept and fired");
}

#[test]
fn status_from_the_first_frame_holds_only_for_the_budget() {
    // The status key every second from the first frame, the voice never
    // quiet: the danger moves once the information budget is spent.
    let (mut h, s) = start(Mode::Challenge, Pace::Normal);
    let before = t(&s);
    let t = parlor_core::testkit::status_spam_until(
        &mut h,
        60,
        |h| tick(h, &s, 0.1),
        |_| t(&s) > before,
    );
    let t = t.expect("the danger never moved");
    assert!(t <= parlor_core::testkit::SPAM_BOUND_S, "{t}");
}

#[test]
fn a_run_that_scores_0_goes_on_no_board() {
    use neon_rail::screens::no_board_text;
    let mut h = Headless::new(Default::default(), Base);
    let ctx = &mut h.app.ctx;
    // An old save that kept a 0.
    Records::record(ctx, ID, Mode::BlackSky.board(), 0);
    let all = neon_rail::records_pages(ctx).join(" ");
    assert!(all.contains("Black Sky Run: no score yet."), "{all}");
    assert_eq!(
        no_board_text(Mode::BlackSky, Pace::Normal),
        "Score 0, so nothing goes on the Black Sky Run board."
    );
    assert_eq!(
        pace::record(ctx, ID, Mode::BlackSky.board(), 0),
        None,
        "a 0 is not recorded"
    );
}
