//! Pace and the speech hold: the sea stands still while a line is being
//! spoken and runs at the expedition's pace, while a held turn and a
//! ping's echoes run on through speech.
use std::cell::RefCell;
use std::rc::Rc;

use hearken::{Ctx, Headless, InputEvent, Key, Mods, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records, Rng};
use sonar_dive::cues;
use sonar_dive::rules::{Config, Dive, Expedition, ExpeditionEnd, Kind, Layout, V2};
use sonar_dive::save::Saved;
use sonar_dive::screens::{challenge_dive, conclude, DiveScreen, Mode};
use sonar_dive::{events, ID};

/// A new moon night, so the glow dive is open.
const DAY: Date = Date::new(2026, 11, 10);

struct Observed(Rc<RefCell<DiveScreen>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "sonar_dive_dive"
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

type Screen = Rc<RefCell<DiveScreen>>;

fn headless() -> Headless<Parlor> {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(DAY);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(sonar_dive::title());
    h.frame(0.0);
    h
}

/// An expedition on an empty sea with only what the test lays.
fn sea(config: Config, lay: impl FnOnce(&mut Dive)) -> Expedition {
    let mut exp = Expedition::new(config.clone(), Rng::seeded(1));
    exp.dive = Dive::with_objects(
        Layout::for_dive(1, &config),
        &config,
        config.hull,
        Vec::new(),
    );
    lay(&mut exp.dive);
    exp
}

/// A dive that starts just past the bell, heading home at full ahead.
fn homecoming(config: Config, carried: u32) -> Expedition {
    sea(config, |d| {
        d.carried = carried;
        d.pos = V2::new(0.0, 13.0);
        d.heading = 180.0;
        d.throttle = 3;
    })
}

/// `screen` on top at `pace`, its opening said.
fn open_at(h: &mut Headless<Parlor>, screen: DiveScreen, pace: Pace) -> Screen {
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(screen));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(h);
    s
}

fn open(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    let mut h = headless();
    let exp = Expedition::new(sonar_dive::config_for(mode), Rng::seeded(1));
    let s = open_at(&mut h, DiveScreen::new(exp, mode, None), pace);
    (h, s)
}

fn challenge(period: Period, pace: Pace) -> (Headless<Parlor>, Screen) {
    let mut h = headless();
    let ch = Challenge::current(&h.app.ctx, ID, period);
    let s = open_at(&mut h, challenge_dive(&ch), pace);
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

fn time(s: &Screen) -> f64 {
    s.borrow().expedition().unwrap().dive.time
}

fn air(s: &Screen) -> f64 {
    s.borrow().expedition().unwrap().dive.oxygen
}

fn key_down(h: &mut Headless<Parlor>, key: Key) {
    h.send(InputEvent::KeyDown {
        key,
        mods: Mods::NONE,
        text: None,
        repeat: false,
    });
}

/// Frames with the voice quiet until the dive screen gives way.
fn until_the_dive_ends(h: &mut Headless<Parlor>) {
    for _ in 0..20_000 {
        if h.app.top_name() != "sonar_dive_dive" {
            return;
        }
        quiet(h);
        h.frame(0.05);
    }
    panic!("the dive never ended");
}

/// Reads the pages on top, Enter by Enter, and gives all they said.
fn read_pages(h: &mut Headless<Parlor>) -> String {
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

/// Surfaces from `exp` at `pace` and stops at the boat: the result pages.
fn bank(h: &mut Headless<Parlor>, exp: Expedition, pace: Pace) -> String {
    pace::set(&mut h.app.ctx, ID, pace);
    h.app.ctx.push(DiveScreen::new(exp, Mode::Expedition, None));
    h.frame(0.0);
    until_the_dive_ends(h);
    assert_eq!(h.app.top_name(), "sonar_dive_surface", "{:?}", h.spoken());
    h.press(Key::End);
    h.clear();
    h.press(Key::Return);
    let said = read_pages(h);
    h.press(Key::Escape);
    said
}

#[test]
fn normal_is_the_default_and_pace_follows_call_outs_and_units() {
    let mut h = Headless::new(Default::default(), Base);
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    assert_eq!(pace::get(&h.app.ctx, ID), Pace::Normal);
    let s = Rc::new(RefCell::new(DiveScreen::new(
        sea(Config::expedition(), |_| {}),
        Mode::Expedition,
        None,
    )));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
    let rows = <sonar_dive::SonarDive as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 3, "call-outs, units, then pace");
}

#[test]
fn every_mode_holds_for_speech_and_runs_at_its_pace() {
    for pace in Pace::ALL {
        let runs = [
            open(Mode::Expedition, pace),
            open(Mode::Deep, pace),
            open(Mode::Practice, pace),
            open(Mode::Glow, pace),
            challenge(Period::Daily, pace),
            challenge(Period::Weekly, pace),
            challenge(Period::Monthly, pace),
        ];
        for (mut h, s) in runs {
            assert_eq!(s.borrow().clock.pace(), pace);
            let (before, breath) = (time(&s), air(&s));
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.4);
            assert_eq!(time(&s), before, "holds for speech");
            assert_eq!(air(&s), breath, "the air holds too");
            assert_eq!(s.borrow().clock.state(), ClockState::Held);
            quiet(&mut h);
            for _ in 0..4 {
                tick(&mut h, &s, 0.1);
            }
            let ran = time(&s) - before;
            assert!((ran - 0.4 * pace.scale()).abs() < 1e-6, "{pace:?}: {ran}");
            assert_eq!(s.borrow().clock.state(), ClockState::Running);
        }
    }
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, s) = open(Mode::Expedition, Pace::Quick);
    let before = time(&s);
    for _ in 0..300 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(time(&s) > before);
}

#[test]
fn a_held_turn_runs_through_speech() {
    let (mut h, s) = open(Mode::Expedition, Pace::Normal);
    let heading = s.borrow().expedition().unwrap().dive.heading;
    h.app.ctx.say("A line still being spoken.");
    key_down(&mut h, Key::Right);
    tick(&mut h, &s, 0.1);
    assert_eq!(s.borrow().clock.state(), ClockState::Bypassed);
    assert_ne!(s.borrow().expedition().unwrap().dive.heading, heading);
    h.send(InputEvent::key_up(Key::Right));
    h.app.ctx.say("A line still being spoken.");
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held(), "hands off, the sea holds again");
}

#[test]
fn keys_pressed_during_a_hold_apply_at_once_and_the_echoes_keep_coming() {
    for pace in Pace::ALL {
        let mut h = headless();
        let exp = sea(Config::expedition(), |d| {
            d.place(Kind::Chest, V2::new(0.0, 30.0), 40);
        });
        let s = open_at(&mut h, DiveScreen::new(exp, Mode::Expedition, None), pace);
        h.app.ctx.say("A line still being spoken.");
        tick(&mut h, &s, 0.1);
        assert!(s.borrow().clock.held());
        // The throttle answers at once, once.
        h.press(Key::Up);
        assert_eq!(s.borrow().expedition().unwrap().dive.throttle, 1);
        h.press(Key::Down);
        assert_eq!(s.borrow().expedition().unwrap().dive.throttle, 0);
        // So does a ping, and its echoes come back through the line.
        h.press(Key::Space);
        assert!(h.played().iter().any(|k| k == cues::PING));
        assert_eq!(s.borrow().expedition().unwrap().dive.pings, 1);
        let echo = cues::echo(Kind::Chest, false);
        for _ in 0..40 {
            if h.played().iter().any(|k| k == echo) {
                break;
            }
            h.app.ctx.say("Still talking.");
            tick(&mut h, &s, 0.05);
            assert_eq!(s.borrow().clock.state(), ClockState::Bypassed);
        }
        assert!(h.played().iter().any(|k| k == echo), "{pace:?}");
        for _ in 0..4 {
            h.app.ctx.say("Still talking.");
            tick(&mut h, &s, 0.05);
        }
        assert!(
            !s.borrow().expedition().unwrap().dive.echoes_pending(),
            "{pace:?}"
        );
        assert!(s.borrow().clock.held(), "{pace:?}: the echoes are back");
    }
}

#[test]
fn info_keys_hold_the_air_and_change_nothing_else() {
    let (mut h, s) = open(Mode::Expedition, Pace::Quick);
    let (before, breath) = (time(&s), air(&s));
    for key in [Key::S, Key::S, Key::H, Key::R] {
        h.press(key);
        tick(&mut h, &s, 0.1);
        assert!(s.borrow().clock.held());
    }
    assert_eq!(time(&s), before);
    assert_eq!(air(&s), breath);
    let screen = s.borrow();
    let exp = screen.expedition().unwrap();
    assert_eq!((exp.dive.carried, exp.banked, exp.score()), (0, 0, 0));
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = open(Mode::Expedition, Pace::Relaxed);
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
    assert_eq!(h.app.top_name(), "sonar_dive_dive");
    assert!(
        h.spoken().iter().any(|l| l.starts_with("Back to dive 1.")),
        "{:?}",
        h.spoken()
    );
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    let before = time(&s);
    for _ in 0..10 {
        tick(&mut h, &s, 0.1);
        assert_eq!(time(&s), before, "the resume line has a fresh hold");
    }
    let (_h, s) = open(Mode::Expedition, Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

#[test]
fn the_next_dive_keeps_the_expeditions_pace() {
    let mut h = headless();
    pace::set(&mut h.app.ctx, ID, Pace::Relaxed);
    h.app.ctx.push(DiveScreen::new(
        homecoming(Config::expedition(), 70),
        Mode::Expedition,
        None,
    ));
    h.frame(0.0);
    until_the_dive_ends(&mut h);
    assert_eq!(h.app.top_name(), "sonar_dive_surface");
    // Changed at the boat: the expedition keeps its pace all the same.
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "sonar_dive_dive");
    until_the_dive_ends(&mut h);
    let said = read_pages(&mut h);
    assert!(
        said.contains("Haul: 70 coins. Your first relaxed haul."),
        "{said}"
    );
    let ctx = &mut h.app.ctx;
    assert_eq!(Records::best(ctx, ID, "best-relaxed"), Some(70));
    assert_eq!(Records::best(ctx, ID, "best"), None);
    assert_eq!(Saved::load(ctx).deepest, 2, "depth counts at every pace");
}

#[test]
fn a_relaxed_haul_keeps_its_own_board_and_normal_and_quick_share_one() {
    let mut h = headless();
    let said = bank(&mut h, homecoming(Config::expedition(), 40), Pace::Relaxed);
    assert!(
        said.contains("Haul: 40 coins. Your first relaxed haul."),
        "{said}"
    );
    let said = bank(&mut h, homecoming(Config::expedition(), 30), Pace::Relaxed);
    assert!(said.contains("Your relaxed best haul is 40."), "{said}");
    let said = bank(&mut h, homecoming(Config::expedition(), 20), Pace::Normal);
    assert!(said.contains("Your first recorded haul."), "{said}");
    let said = bank(&mut h, homecoming(Config::expedition(), 50), Pace::Quick);
    assert!(
        said.contains("A new best haul. The old best was 20."),
        "{said}"
    );
    let ctx = &mut h.app.ctx;
    assert_eq!(Records::best(ctx, ID, "best-relaxed"), Some(40));
    assert_eq!(Records::best(ctx, ID, "best"), Some(50));
    assert_eq!(Saved::load(ctx).expeditions, 4);
}

#[test]
fn a_relaxed_glow_dive_keeps_its_own_board() {
    let mut h = headless();
    let mut exp = sea(Config::glow(), |_| {});
    exp.banked = 420;
    exp.end = Some(ExpeditionEnd::Banked);
    conclude(&mut h.app.ctx, exp, Mode::Glow, None, Pace::Relaxed);
    let board = events::GLOW_BOARD;
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(board)),
        Some(420)
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, board), None);
}

#[test]
fn a_relaxed_challenge_keeps_its_own_result() {
    let (mut h, s) = challenge(Period::Daily, Pace::Relaxed);
    {
        let mut screen = s.borrow_mut();
        let d = &mut screen.expedition_mut().unwrap().dive;
        d.carried = 40;
        d.pos = V2::new(0.0, 13.0);
        d.heading = 180.0;
        d.throttle = 3;
    }
    until_the_dive_ends(&mut h);
    let said = read_pages(&mut h);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert_eq!(
        Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed),
        Some(40)
    );
    assert!(
        said.contains("result: 40 coins. Your first relaxed result."),
        "{said}"
    );
}

#[test]
fn records_and_the_title_name_both_boards() {
    let mut h = headless();
    let ctx = &mut h.app.ctx;
    Records::record(ctx, ID, "best", 300);
    Records::record(ctx, ID, "best-relaxed", 180);
    Records::record(ctx, ID, "deep-relaxed", 90);
    let mut saved = Saved::load(ctx);
    saved.deepest = 4;
    saved.store(ctx);
    let all = sonar_dive::records_pages(ctx).join(" ");
    assert!(
        all.contains("Expedition: best haul 300 coins, relaxed best haul 180 coins."),
        "{all}"
    );
    assert!(
        all.contains("Deep start: relaxed best haul 90 coins."),
        "{all}"
    );
    assert!(!all.contains("best haul 0"), "{all}");
    Records::record(ctx, ID, &Pace::Relaxed.board(events::GLOW_BOARD), 260);

    h.app.ctx.pop();
    h.app.ctx.push(sonar_dive::title());
    h.clear();
    h.frame(0.0);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l
                .contains("Play, expedition, best haul 300 coins, relaxed best haul 180 coins")),
        "{:?}",
        h.spoken()
    );
    for _ in 0..12 {
        if h.app
            .top_lines()
            .iter()
            .any(|l| l == &format!("> {}", events::GLOW_DIVE))
        {
            break;
        }
        h.press(Key::Down);
    }
    h.clear();
    h.press(Key::F1);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.ends_with(" Your relaxed best haul is 260 coins.")),
        "{:?}",
        h.spoken()
    );
    assert!(!h.spoken().iter().any(|l| l.contains("Your best haul")));
}

#[test]
fn status_from_the_first_frame_holds_only_for_the_budget() {
    // The status key every second from the first frame, the voice never
    // quiet: the danger moves once the information budget is spent.
    let (mut h, s) = open(Mode::Expedition, Pace::Normal);
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
