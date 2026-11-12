//! Pace and the speech hold: the haunt clock and the ghosts stand still
//! while a line is being spoken, and run at the run's pace.
use std::cell::RefCell;
use std::rc::Rc;

use haunted_hallways::haunt::Haunt;
use haunted_hallways::rules::{self, GhostKind, Mode, Side, GHOST_NEAR};
use haunted_hallways::{Progress, ID};
use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Pace, Parlor, Period, Records};

use crate::common::{self, key_for, spoke, Base, Twin, SEED, TODAY};

struct Observed(Rc<RefCell<Haunt>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "haunted_hallways_observed"
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

type Screen = Rc<RefCell<Haunt>>;

fn headless() -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(TODAY);
    h
}

/// `haunt` on screen at `pace`, its opening lines said.
fn open(mut h: Headless<Parlor>, haunt: Haunt, pace: Pace) -> (Headless<Parlor>, Screen) {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(haunt));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(&mut h);
    (h, s)
}

fn start(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    let haunt = match mode {
        Mode::Challenge(e) => {
            let h = headless();
            Haunt::challenge(Challenge::current(&h.app.ctx, ID, e.period))
        }
        mode => Haunt::with_seed(mode, SEED),
    };
    open(headless(), haunt, pace)
}

fn challenge_mode(period: Period) -> Mode {
    let h = headless();
    let ch = Challenge::current(&h.app.ctx, ID, period);
    Mode::Challenge(rules::Event::from_challenge(&ch).0)
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

/// One frame of `dt` straight to the screen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, s: &Screen, dt: f64) {
    s.borrow_mut().update(&mut h.app.ctx, dt);
}

fn left(s: &Screen) -> f64 {
    s.borrow().run.haunt_left
}

/// A ghost of `kind` just short of you, with no other ghost to come.
fn ghost_close(s: &Screen, kind: GhostKind) {
    let mut haunt = s.borrow_mut();
    haunt.run.spec.ghost_gap = (1e9, 1e9);
    let mut g = haunt.run.summon(Some(kind), Some(0.0));
    g.s = GHOST_NEAR + 0.01;
    haunt.run.ghost = Some(g);
}

#[test]
fn normal_is_the_default_pace() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = Rc::new(RefCell::new(Haunt::with_seed(Mode::Climb(1), SEED)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    let modes = [
        Mode::Climb(1),
        Mode::Floor(3),
        Mode::Nightmare,
        challenge_mode(Period::Daily),
        challenge_mode(Period::Monthly),
    ];
    for mode in modes {
        for pace in Pace::ALL {
            let (mut h, s) = start(mode, pace);
            assert_eq!(s.borrow().clock.pace(), pace);
            let before = left(&s);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.2);
            assert_eq!(left(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().clock.state(), ClockState::Held);
            quiet(&mut h);
            tick(&mut h, &s, 0.2);
            let ran = before - left(&s);
            assert!(
                (ran - 0.2 * pace.scale()).abs() < 1e-9,
                "{mode:?} at {pace:?}: {ran}"
            );
        }
    }
}

#[test]
fn the_ghost_holds_while_freeze_is_spoken() {
    let (mut h, s) = start(Mode::Floor(1), Pace::Quick);
    ghost_close(&s, GhostKind::Widow);
    tick(&mut h, &s, 0.05);
    assert!(spoke(&h, "Freeze!"), "{:?}", h.spoken());
    // The line is still being said: the ghost and the stab stand still.
    let at = s.borrow().run.ghost.unwrap().s;
    for _ in 0..10 {
        tick(&mut h, &s, 0.05);
        assert!(s.borrow().clock.held(), "Freeze! is being said");
        assert_eq!(s.borrow().run.ghost.unwrap().s, at);
        assert_eq!(s.borrow().run.since_stab, 0.0);
    }
    // A step on the held stab is inside its grace.
    h.press(key_for(Side::North));
    assert_eq!(s.borrow().run.courage, rules::COURAGE);
    // Once it is said, the ghost moves on.
    quiet(&mut h);
    tick(&mut h, &s, 0.05);
    assert!(s.borrow().run.ghost.unwrap().s < at);
}

#[test]
fn the_hound_holds_while_keep_walking_is_spoken() {
    let (mut h, s) = start(Mode::Floor(1), Pace::Quick);
    ghost_close(&s, GhostKind::Hound);
    tick(&mut h, &s, 0.05);
    assert!(spoke(&h, "Keep walking!"), "{:?}", h.spoken());
    // Standing still through the line does not count against you.
    for _ in 0..60 {
        tick(&mut h, &s, 0.05);
        assert!(s.borrow().clock.held(), "Keep walking! is being said");
    }
    assert_eq!(s.borrow().run.still, 0.0);
    assert_eq!(s.borrow().run.courage, rules::COURAGE);
}

#[test]
fn a_ghost_coming_gets_a_fresh_hold_however_long_the_lines_before_it() {
    // Room 2 of the ground floor, where ghosts walk.
    let mut haunt = Haunt::with_seed(Mode::Floor(1), SEED);
    common::solve_run_room(&mut haunt.run);
    assert_eq!(haunt.run.room, 1);
    let (mut h, s) = open(headless(), haunt, Pace::Quick);
    // A voice that never stops: the clock runs once the hold expires.
    for _ in 0..300 {
        h.app.ctx.say("A long line still being spoken.");
        tick(&mut h, &s, 0.1);
    }
    assert_eq!(s.borrow().clock.state(), ClockState::Running);
    {
        let mut haunt = s.borrow_mut();
        haunt.run.spec.ghosts = &[GhostKind::Widow];
        haunt.run.ghost_in = 0.01;
    }
    h.clear();
    tick(&mut h, &s, 0.1);
    assert!(spoke(&h, "A ghost! "), "{:?}", h.spoken());
    let at = s.borrow().run.ghost.unwrap().s;
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held(), "the ghost's line has a fresh hold");
    assert_eq!(s.borrow().run.ghost.unwrap().s, at);
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = start(Mode::Floor(1), Pace::Quick);
    let before = left(&s);
    for _ in 0..600 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(before - left(&s) > 10.0, "{}", before - left(&s));
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = start(Mode::Floor(1), Pace::Relaxed);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "haunted_hallways_observed");
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "haunted_hallways_observed");
    assert!(spoke(&h, "Back in the foyer."), "{:?}", h.spoken());
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    // The resume line has a full hold of its own.
    let before = left(&s);
    for _ in 0..30 {
        tick(&mut h, &s, 0.1);
        assert_eq!(left(&s), before);
    }
    // The next run takes the new pace.
    let (_h, s) = start(Mode::Floor(1), Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

/// Ends the run with `score` by running the haunt clock out, quietly.
fn run_out(h: &mut Headless<Parlor>, s: &Screen, score: u64) {
    {
        let mut haunt = s.borrow_mut();
        haunt.run.score = score;
        haunt.run.haunt_left = 0.05;
    }
    for _ in 0..20 {
        quiet(h);
        h.frame(0.1);
        if h.app.top_name() == "pages" {
            break;
        }
    }
    assert_eq!(h.app.top_name(), "pages", "{:?}", h.spoken());
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
fn a_relaxed_run_keeps_its_own_board() {
    let (mut h, s) = start(Mode::Climb(1), Pace::Relaxed);
    run_out(&mut h, &s, 500);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, "climb-relaxed"),
        Some(500)
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, "climb"), None);

    let (mut h2, s) = open(h, Haunt::with_seed(Mode::Climb(1), SEED), Pace::Relaxed);
    run_out(&mut h2, &s, 300);
    let pages = read_pages(&mut h2);
    assert!(pages.contains("Your relaxed best is 500."), "{pages}");
}

#[test]
fn normal_and_quick_share_a_board() {
    let (mut h, s) = start(Mode::Nightmare, Pace::Normal);
    run_out(&mut h, &s, 200);
    assert_eq!(Records::best(&mut h.app.ctx, ID, "nightmare"), Some(200));
    let (mut h, s) = open(h, Haunt::with_seed(Mode::Nightmare, SEED), Pace::Quick);
    run_out(&mut h, &s, 300);
    let pages = read_pages(&mut h);
    assert!(pages.contains("a new best, beating 200"), "{pages}");
    assert_eq!(Records::best(&mut h.app.ctx, ID, "nightmare-relaxed"), None);
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, s) = start(challenge_mode(Period::Daily), Pace::Relaxed);
    run_out(&mut h, &s, 400);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert_eq!(
        Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed),
        Some(400)
    );
    let pages = read_pages(&mut h);
    assert!(pages.contains("first relaxed result"), "{pages}");
}

#[test]
fn a_relaxed_floor_time_goes_on_its_own_list_and_still_opens_the_next() {
    let mut h = common::title();
    pace::set(&mut h.app.ctx, ID, Pace::Relaxed);
    let mut t = Twin::from_title(h, Mode::Floor(1));
    for _ in 0..3 {
        t.solve_room();
    }
    assert!(
        spoke(&t.h, "Your first relaxed time on this floor."),
        "{:?}",
        t.h.spoken()
    );
    t.read_until("haunted_hallways");
    let p = Progress::load(&mut t.h.app.ctx);
    assert_eq!(p.cleared, 1, "the floor opens the next at every pace");
    assert_eq!(p.floor_time(1), None);
    assert_eq!(p.floor_time_at(Pace::Relaxed, 1), Some(0.0));
    assert!(Records::best(&mut t.h.app.ctx, ID, "floor 1-relaxed").is_some());
    assert_eq!(Records::best(&mut t.h.app.ctx, ID, "floor 1"), None);
}

#[test]
fn records_and_the_title_name_both_boards() {
    let mut h = headless();
    Records::record(&mut h.app.ctx, ID, "climb", 30);
    Records::record(&mut h.app.ctx, ID, "climb-relaxed", 41);
    let mut p = Progress {
        cleared: 2,
        ..Progress::default()
    };
    p.floor_best.insert(1, 620);
    p.relaxed_floor_best.insert(1, 500);
    p.relaxed_floor_best.insert(2, 450);
    p.store(&mut h.app.ctx);
    let all = haunted_hallways::records_pages(&mut h.app.ctx).join(" ");
    assert!(
        all.contains("Play, best score 30, relaxed best score 41."),
        "{all}"
    );
    assert!(
        all.contains("floor 1, 1 minute 2.0 seconds, relaxed best 50.0 seconds; floor 2, relaxed best 45.0 seconds;"),
        "{all}"
    );
    assert!(!all.contains("relaxed best score 0"), "{all}");

    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(haunted_hallways::title());
    h.frame(0.0);
    h.clear();
    h.press(Key::F1);
    let help = h.spoken().join(" ");
    assert!(
        help.contains("Best score 30, relaxed best score 41."),
        "{help}"
    );
    // Floor select: the first wing's floors.
    h.press(Key::F);
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(
            &h,
            "Floor 1, best 1 minute 2.0 seconds, relaxed best 50.0 seconds"
        ),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::Down);
    assert!(
        spoke(&h, "Floor 2, relaxed best 45.0 seconds"),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn settings_has_the_pace_row() {
    let h = headless();
    let rows =
        <haunted_hallways::HauntedHallways as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 1);
}
