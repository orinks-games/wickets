//! Pace and the speech hold: the street stands still while a line is
//! being spoken and no riding key is held, and runs at the ride's pace.
use std::cell::RefCell;
use std::rc::Rc;

use freewheel::course::{Course, District};
use freewheel::progress::{self, Choices};
use freewheel::world::World;
use freewheel::{Bike, Mode, Ride, ID};
use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};

/// A day in leaf season, so every ride can be made.
const DAY: Date = Date::new(2026, 10, 20);

struct Observed(Rc<RefCell<Ride>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "freewheel_observed"
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

type Screen = Rc<RefCell<Ride>>;

fn headless() -> Headless<Parlor> {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(DAY);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h
}

/// `ride` on screen at `pace`, its countdown run out and its lines said.
fn open_at(mut h: Headless<Parlor>, ride: Ride, pace: Pace) -> (Headless<Parlor>, Screen) {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(ride));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    for _ in 0..80 {
        quiet(&mut h);
        tick(&mut h, &s, 0.05);
    }
    assert!(s.borrow().rolling());
    quiet(&mut h);
    (h, s)
}

fn open(ride: Ride, pace: Pace) -> (Headless<Parlor>, Screen) {
    open_at(headless(), ride, pace)
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

/// One frame of `dt` straight to the screen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, s: &Screen, dt: f64) {
    s.borrow_mut().update(&mut h.app.ctx, dt);
}

fn hold(h: &mut Headless<Parlor>, key: Key) {
    h.send(InputEvent::KeyDown {
        key,
        mods: hearken::Mods::NONE,
        text: None,
        repeat: false,
    });
}

fn let_go(h: &mut Headless<Parlor>, key: Key) {
    h.send(InputEvent::key_up(key));
}

/// The street's own clock, game seconds.
fn time(s: &Screen) -> f64 {
    s.borrow().world.clock
}

/// Within one of the street's substeps.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= World::SUBSTEP + 1e-9
}

/// A short empty street, timed on the first course's boards.
fn short_trial(length: f64) -> Ride {
    Ride::custom(
        Course::empty("Short street", length),
        Bike::Road,
        Mode::TimeTrial(0),
    )
}

/// Rides to the end of the street holding Up, with the voice quiet, and
/// reads the result pages.
fn ride_out(h: &mut Headless<Parlor>, s: &Screen) -> String {
    hold(h, Key::Up);
    for _ in 0..40_000 {
        if h.app.top_name() == "pages" {
            break;
        }
        quiet(h);
        h.frame(0.05);
    }
    let _ = s;
    let_go(h, Key::Up);
    assert_eq!(h.app.top_name(), "pages", "{:?}", h.spoken());
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
fn normal_is_the_default_and_settings_ends_with_pace() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = Rc::new(RefCell::new(Ride::time_trial(0, Bike::Road)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().run_clock.pace(), Pace::Normal);
    let rows = <freewheel::Freewheel as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 3, "call-outs, units, then pace");
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    for pace in Pace::ALL {
        let h = headless();
        let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
        let rides = [
            Ride::time_trial(0, Bike::Road),
            Ride::free_ride(Bike::Road, 3),
            Ride::endless(Bike::Road, District::Downtown, 3),
            Ride::against_clock(Bike::Road, District::Downtown, 3),
            Ride::challenge(ch),
            Ride::leaf_season(Bike::Road, DAY),
            Ride::training(1, Bike::Road),
        ];
        for ride in rides {
            let (mut h, s) = open(ride, pace);
            let mode = s.borrow().mode;
            assert_eq!(s.borrow().run_clock.pace(), pace);
            let left = |s: &Screen| s.borrow().world.drop_clock.map(|c| c.left);
            let (before, clock) = (time(&s), left(&s));
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.2);
            assert_eq!(time(&s), before, "{mode:?} holds for speech");
            assert_eq!(left(&s), clock, "{mode:?} holds its clock");
            assert_eq!(s.borrow().run_clock.state(), ClockState::Held);
            quiet(&mut h);
            tick(&mut h, &s, 0.2);
            let ran = time(&s) - before;
            assert!(near(ran, 0.2 * pace.scale()), "{mode:?} at {pace:?}: {ran}");
            if let (Some(was), Some(now)) = (clock, left(&s)) {
                assert!(near(was - now, 0.2 * pace.scale()), "{now}");
            }
        }
    }
}

#[test]
fn a_held_riding_key_keeps_the_street_running_through_speech() {
    for key in [Key::Up, Key::Down, Key::Left, Key::Right] {
        let (mut h, s) = open(Ride::free_ride(Bike::Road, 3), Pace::Quick);
        hold(&mut h, key);
        let before = time(&s);
        h.app.ctx.say("A line still being spoken.");
        tick(&mut h, &s, 0.2);
        assert_eq!(s.borrow().run_clock.state(), ClockState::Bypassed);
        assert!(near(time(&s) - before, 0.2), "{key:?}");
        // Once it is let go, speech holds the street again.
        let_go(&mut h, key);
        h.app.ctx.say("Another line.");
        let before = time(&s);
        tick(&mut h, &s, 0.2);
        assert_eq!(time(&s), before, "{key:?}");
    }
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = open(Ride::free_ride(Bike::Road, 3), Pace::Quick);
    let before = time(&s);
    for _ in 0..300 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(time(&s) - before > 10.0, "{}", time(&s) - before);
}

#[test]
fn a_hop_pressed_during_a_hold_is_taken_once_the_street_runs() {
    let (mut h, s) = open(Ride::free_ride(Bike::Bmx, 3), Pace::Normal);
    hold(&mut h, Key::Up);
    for _ in 0..60 {
        quiet(&mut h);
        tick(&mut h, &s, 0.05);
    }
    let_go(&mut h, Key::Up);
    h.app.ctx.say("A line still being spoken.");
    tick(&mut h, &s, 0.05);
    assert!(s.borrow().run_clock.held());
    h.clear();
    h.press(Key::Space);
    tick(&mut h, &s, 0.05);
    assert!(!s.borrow().world.rider.airborne(), "held still");
    quiet(&mut h);
    tick(&mut h, &s, 0.05);
    assert!(s.borrow().world.rider.airborne(), "the hop was kept");
}

#[test]
fn pace_stays_fixed_for_the_ride_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = open(Ride::free_ride(Bike::Road, 3), Pace::Relaxed);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "freewheel_observed");
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "freewheel_observed");
    assert!(
        h.spoken().iter().any(|l| l.starts_with("Riding.")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().run_clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = time(&s);
    for _ in 0..30 {
        tick(&mut h, &s, 0.1);
        assert_eq!(time(&s), before, "the resume line has a fresh hold");
    }
    let (_h, s) = open(Ride::free_ride(Bike::Road, 3), Pace::Quick);
    assert_eq!(s.borrow().run_clock.pace(), Pace::Quick);
}

#[test]
fn a_relaxed_time_trial_keeps_its_own_time_and_points() {
    let (mut h, s) = open(short_trial(40.0), Pace::Relaxed);
    let pages = ride_out(&mut h, &s);
    let board = progress::board("station", Bike::Road);
    let ctx = &mut h.app.ctx;
    assert!(progress::best_time(ctx, &Pace::Relaxed.board(&board)).is_some());
    assert_eq!(progress::best_time(ctx, &board), None);
    assert!(Records::best(ctx, ID, &Pace::Relaxed.board(&board)).is_some());
    assert_eq!(Records::best(ctx, ID, &board), None);
    assert!(pages.contains("Your first relaxed time on"), "{pages}");
    assert!(pages.contains("Your first relaxed result here."), "{pages}");
}

#[test]
fn normal_and_quick_share_a_board() {
    let (mut h, s) = open(short_trial(40.0), Pace::Normal);
    ride_out(&mut h, &s);
    let board = progress::board("station", Bike::Road);
    assert!(progress::best_time(&mut h.app.ctx, &board).is_some());
    let (mut h, s) = open_at(h, short_trial(40.0), Pace::Quick);
    let pages = ride_out(&mut h, &s);
    // The quick ride is measured against the normal one's best.
    assert!(!pages.contains("Your first"), "{pages}");
    assert!(!pages.contains("relaxed"), "{pages}");
    let relaxed = Pace::Relaxed.board(&board);
    assert_eq!(progress::best_time(&mut h.app.ctx, &relaxed), None);
}

#[test]
fn a_relaxed_endless_ride_keeps_its_own_distance() {
    let world = World::new(Course::empty("Short street", 40.0), Bike::Road, 1);
    let ride = Ride::from_world(world, Mode::Endless(District::Downtown), 100.0);
    let (mut h, s) = open(ride, Pace::Relaxed);
    let pages = ride_out(&mut h, &s);
    let board = progress::endless_board(District::Downtown);
    assert!(Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(&board)).is_some());
    assert_eq!(Records::best(&mut h.app.ctx, ID, &board), None);
    assert!(
        pages.contains("Your first relaxed endless ride here."),
        "{pages}"
    );
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let h = headless();
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let (mut h, s) = open_at(h, Ride::challenge(ch.clone()), Pace::Relaxed);
    let pages = ride_out(&mut h, &s);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert!(Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed).is_some());
    assert!(pages.contains("Your first relaxed result here."), "{pages}");
}

/// Saves a best time and ride points on `board`.
fn best(ctx: &mut Ctx<Parlor>, board: &str, secs: f64, points: u64) {
    progress::save_time(ctx, board, secs, points, Pace::Normal);
}

#[test]
fn records_course_select_and_the_title_name_both_boards() {
    let mut h = headless();
    let ctx = &mut h.app.ctx;
    let station = progress::board("station", Bike::Road);
    let market = progress::board("market", Bike::Road);
    best(ctx, &station, 64.0, 98000);
    best(ctx, &Pace::Relaxed.board(&station), 110.0, 57000);
    best(ctx, &Pace::Relaxed.board(&market), 90.0, 61000);
    let endless = progress::endless_board(District::Downtown);
    Records::record(ctx, ID, &endless, 1200);
    Records::record(ctx, ID, &Pace::Relaxed.board(&endless), 900);
    let all = freewheel::records_pages(ctx).join(" ");
    assert!(
        all.contains("on the road bike, 1 minute 4.0 seconds, 98000 ride points, relaxed best 1 minute 50.0 seconds, 57000 ride points"),
        "{all}"
    );
    assert!(
        all.contains("on the road bike, relaxed best 1 minute 30.0 seconds, 61000 ride points"),
        "{all}"
    );
    assert!(
        all.contains("Downtown, 1.2 kilometres, relaxed best 900 metres"),
        "{all}"
    );
    assert!(!all.contains("best 0"), "{all}");

    Choices::default().store(&mut h.app.ctx);
    h.app.ctx.push(freewheel::title());
    h.frame(0.0);
    // F1 on a row reads its help.
    h.clear();
    h.press(Key::F1);
    let said = h.spoken().join(" ");
    assert!(
        said.contains("Best 1 minute 4.0 seconds, relaxed best 1 minute 50.0 seconds."),
        "{said}"
    );
    h.press(Key::Down);
    h.clear();
    h.press(Key::F1);
    let said = h.spoken().join(" ");
    assert!(
        said.contains("Best 1.2 kilometres, relaxed best 900 metres."),
        "{said}"
    );
}

/// The short street at quick: pedalled for 2 seconds, then coasted to the
/// line. With `spam`, the S key is pressed while coasting, and the street
/// held while its line is spoken. Gives the saved time.
fn coast_trial(spam: bool) -> f64 {
    let (mut h, s) = open(short_trial(25.0), Pace::Quick);
    hold(&mut h, Key::Up);
    for _ in 0..40 {
        quiet(&mut h);
        tick(&mut h, &s, 0.05);
    }
    let_go(&mut h, Key::Up);
    for i in 0..4000 {
        if h.app.top_name() == "pages" {
            break;
        }
        if spam && i < 40 {
            if !h.app.ctx.speech_busy() {
                h.press(Key::S);
            }
        } else {
            quiet(&mut h);
        }
        h.frame(0.05);
    }
    assert_eq!(h.app.top_name(), "pages");
    let board = progress::board("station", Bike::Road);
    progress::best_time(&mut h.app.ctx, &board).unwrap_or_default()
}

#[test]
fn info_keys_never_improve_a_ride_time() {
    let time = coast_trial(false);
    let spam = coast_trial(true);
    // The street stood still for the status lines, and their time counts.
    assert!(spam > time + 1.5, "{spam} {time}");
}
