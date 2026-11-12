//! Pace and the speech hold in Gift Sort: the belt, each gift's ride and
//! the shift clock stand still while a line is being spoken, and run at
//! the run's pace.
use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};
use std::cell::RefCell;
use std::rc::Rc;
use winter_wonderland::gifts::Spot;
use winter_wonderland::progress::{GIFT_BOARD, STORY_BOARD};
use winter_wonderland::{set_seed, Mode, Plan, SortRun, ID};

struct Observed(Rc<RefCell<SortRun>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "winter_wonderland_observed"
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

type Screen = Rc<RefCell<SortRun>>;

fn headless() -> Headless<Parlor> {
    set_seed(Some(31));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
    h
}

/// The daily challenge on the test date: one Gift Sort shift.
fn daily(h: &Headless<Parlor>) -> Mode {
    Mode::Challenge {
        plan: Box::new(Plan::current(&h.app.ctx, Period::Daily)),
        gifts: 0,
    }
}

/// A run of `mode` from shift `level` at `pace`, the belt started and its
/// line said.
fn start_at(mode: Mode, level: u32, pace: Pace) -> (Headless<Parlor>, Screen) {
    let mut h = headless();
    pace::set(&mut h.app.ctx, ID, pace);
    let s = Rc::new(RefCell::new(SortRun::new(mode, level)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    quiet(&mut h);
    h.press(Key::Space);
    assert!(s.borrow().playing());
    quiet(&mut h);
    (h, s)
}

fn start(mode: Mode, pace: Pace) -> (Headless<Parlor>, Screen) {
    start_at(mode, 1, pace)
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

/// One frame of `dt` straight to the screen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, s: &Screen, dt: f64) {
    s.borrow_mut().update(&mut h.app.ctx, dt);
}

fn clock(s: &Screen) -> f64 {
    s.borrow().shift().clock
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// The key that sends the gift on the belt home, if there is one.
fn home_key(s: &Screen) -> Option<Key> {
    let run = s.borrow();
    let shift = run.shift();
    let gift = shift.gift.as_ref()?;
    Some(match shift.home_of(gift.kind) {
        Spot::Left => Key::Left,
        Spot::Ahead => Key::Up,
        Spot::Right => Key::Right,
        Spot::Sack => Key::Down,
    })
}

/// Sorts the first gift, so the run scores, then quiet frames until the
/// shift clock runs out and the results are up.
fn run_out(h: &mut Headless<Parlor>, s: &Screen) {
    for _ in 0..2000 {
        if let Some(key) = home_key(s) {
            h.press(key);
            break;
        }
        quiet(h);
        tick(h, s, 0.1);
    }
    for _ in 0..2000 {
        if h.app.top_name() != "winter_wonderland_observed" {
            return;
        }
        quiet(h);
        tick(h, s, 0.5);
        h.frame(0.0);
    }
    panic!("the clock never ran out");
}

#[test]
fn normal_is_the_default_and_settings_has_the_pace_row() {
    let mut h = headless();
    Records::load::<Option<Pace>>(&mut h.app.ctx, &pace::key(ID));
    let s = Rc::new(RefCell::new(SortRun::new(Mode::Free, 1)));
    h.app.ctx.push(Observed(s.clone()));
    h.frame(0.0);
    assert_eq!(s.borrow().clock.pace(), Pace::Normal);
    let rows = <winter_wonderland::Winter as parlor_core::menus::TitleGame>::settings(&h.app.ctx);
    assert_eq!(rows.len(), 1);
}

#[test]
fn every_gift_sort_run_holds_for_speech_and_runs_at_its_pace() {
    let h = headless();
    let modes = vec![
        Mode::Free,
        Mode::Story {
            blizzard: false,
            gifts: 0,
        },
        daily(&h),
    ];
    for mode in modes {
        for pace in Pace::ALL {
            let (mut h, s) = start(mode.clone(), pace);
            assert_eq!(s.borrow().clock.pace(), pace);
            let before = clock(&s);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &s, 0.5);
            assert_eq!(clock(&s), before, "{mode:?} holds for speech");
            assert_eq!(s.borrow().clock.state(), ClockState::Held);
            quiet(&mut h);
            tick(&mut h, &s, 0.5);
            assert!(
                close(before - clock(&s), 0.5 * pace.scale()),
                "{mode:?} at {pace:?}: {}",
                before - clock(&s)
            );
        }
    }
}

#[test]
fn a_gift_rides_only_while_nothing_is_spoken() {
    let (mut h, s) = start(Mode::Free, Pace::Quick);
    for _ in 0..100 {
        if s.borrow().shift().gift.is_some() {
            break;
        }
        quiet(&mut h);
        tick(&mut h, &s, 0.1);
    }
    let ridden = |s: &Screen| s.borrow().shift().gift.as_ref().unwrap().ridden;
    let before = ridden(&s);
    h.app.ctx.say("A gift on the belt. Shake it with Space.");
    tick(&mut h, &s, 1.0);
    assert_eq!(ridden(&s), before);
    quiet(&mut h);
    tick(&mut h, &s, 1.0);
    assert!(close(ridden(&s) - before, 1.0));
}

#[test]
fn a_change_of_plans_gets_a_full_hold_however_long_the_lines_before_it() {
    // Shift 5: someone changes their mind half way through.
    let (mut h, s) = start_at(Mode::Free, 5, Pace::Quick);
    let changed =
        |h: &Headless<Parlor>| h.spoken().iter().any(|l| l.starts_with("Change of plans."));
    for _ in 0..4000 {
        if changed(&h) {
            break;
        }
        // A voice that never stops: the hold expires and play runs on.
        h.app.ctx.say("Still talking.");
        if let Some(key) = home_key(&s) {
            h.press(key);
        }
        if changed(&h) {
            break;
        }
        tick(&mut h, &s, 0.1);
    }
    assert!(changed(&h), "{:?}", h.spoken());
    let before = clock(&s);
    tick(&mut h, &s, 0.1);
    assert!(s.borrow().clock.held(), "the change has a fresh hold");
    assert_eq!(clock(&s), before);
}

#[test]
fn a_hold_never_lasts_forever() {
    // The first gift's line and the first edge warning get holds of
    // their own, each bounded.
    let (mut h, s) = start(Mode::Free, Pace::Quick);
    let before = clock(&s);
    for _ in 0..900 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    assert!(before - clock(&s) > 5.0, "{}", before - clock(&s));
}

#[test]
fn pace_stays_fixed_for_the_run_and_resume_gives_a_fresh_hold() {
    let (mut h, s) = start(Mode::Free, Pace::Relaxed);
    // A long hold, nearly at its limit.
    for _ in 0..190 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &s, 0.1);
    }
    h.press(Key::Escape);
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "winter_wonderland_observed");
    assert_eq!(s.borrow().clock.pace(), Pace::Relaxed, "pace is fixed");
    // The resume line has a full hold of its own.
    let before = clock(&s);
    for _ in 0..20 {
        tick(&mut h, &s, 0.1);
    }
    assert_eq!(clock(&s), before);
    // The next run takes the new pace.
    let (_h, s) = start(Mode::Free, Pace::Quick);
    assert_eq!(s.borrow().clock.pace(), Pace::Quick);
}

#[test]
fn a_relaxed_run_keeps_its_own_best() {
    let (mut h, s) = start(Mode::Free, Pace::Relaxed);
    run_out(&mut h, &s);
    assert!(Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(GIFT_BOARD)).is_some());
    assert_eq!(Records::best(&mut h.app.ctx, ID, GIFT_BOARD), None);
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.contains("Your first relaxed score here is saved.")),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn normal_and_quick_share_the_gift_sort_board() {
    let (mut h, s) = start(Mode::Free, Pace::Normal);
    run_out(&mut h, &s);
    assert!(Records::best(&mut h.app.ctx, ID, GIFT_BOARD).is_some());
    assert_eq!(Pace::Quick.board(GIFT_BOARD), GIFT_BOARD);
}

#[test]
fn a_relaxed_challenge_and_story_keep_their_own_results() {
    let h = headless();
    let (mut h, s) = start_at(daily(&h), 5, Pace::Relaxed);
    run_out(&mut h, &s);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert!(Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed).is_some());
    assert!(
        h.spoken()
            .iter()
            .any(|l| l.contains("Your first relaxed result on this challenge")),
        "{:?}",
        h.spoken()
    );

    let story = Mode::Story {
        blizzard: false,
        gifts: 0,
    };
    let (mut h, s) = start(story, Pace::Relaxed);
    run_out(&mut h, &s);
    assert_eq!(Records::best(&mut h.app.ctx, ID, STORY_BOARD), None);
    assert!(Records::best(&mut h.app.ctx, ID, &Pace::Relaxed.board(STORY_BOARD)).is_some());
}

#[test]
fn the_title_and_records_name_both_boards() {
    let mut h = headless();
    Records::record(&mut h.app.ctx, ID, GIFT_BOARD, 400);
    Records::record(&mut h.app.ctx, ID, &Pace::Relaxed.board(GIFT_BOARD), 300);
    Records::record(&mut h.app.ctx, ID, &Pace::Relaxed.board(STORY_BOARD), 900);
    let all = winter_wonderland::title::records_pages(&mut h.app.ctx).join(" ");
    assert!(all.contains("Story mode: relaxed best 900."), "{all}");
    assert!(
        all.contains("Gift Sort: best 400, relaxed best 300,"),
        "{all}"
    );
    assert!(!all.contains("best 0"), "{all}");
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(winter_wonderland::title());
    h.frame(0.0);
    h.clear();
    for _ in 0..12 {
        if h.spoken().iter().any(|l| l.starts_with("Gift Sort, shift")) {
            break;
        }
        h.press(Key::Down);
    }
    let said = h.spoken().join(" ");
    assert!(
        said.contains("Gift Sort, shift 1, best 400, relaxed best 300"),
        "{said}"
    );
}

#[test]
fn a_change_of_plans_waits_behind_the_line_before_it() {
    use hearken::speech::SpeechChannel;
    // Shift 5: someone changes their mind half way through, often in the
    // frame a gift is delivered.
    let (mut h, s) = start_at(Mode::Free, 5, Pace::Quick);
    let changed =
        |h: &Headless<Parlor>| h.spoken().iter().any(|l| l.starts_with("Change of plans."));
    for _ in 0..4000 {
        if changed(&h) {
            break;
        }
        quiet(&mut h);
        if let Some(key) = home_key(&s) {
            h.press(key);
        }
        if changed(&h) {
            break;
        }
        tick(&mut h, &s, 0.1);
    }
    let calls = h.speech.borrow().calls(SpeechChannel::Main);
    let change = calls
        .iter()
        .find(|(l, _)| l.starts_with("Change of plans."))
        .expect("someone changes their mind");
    assert!(!change.1, "it is queued, not said over: {calls:?}");
}

/// The tutorial on the first shift, its belt watched, its opening said.
fn tutorial() -> (Headless<Parlor>, parlor_core::Live<SortRun>) {
    use winter_wonderland::tutorial;
    let mut h = headless();
    let (belt, desk) = (tutorial::belt(), tutorial::desk());
    let lesson = tutorial::lesson_on(&mut h.app.ctx, belt.clone(), desk);
    h.app.ctx.push(lesson);
    h.frame(0.0);
    (h, belt)
}

#[test]
fn the_tutorial_says_shake_it_now_with_a_gift_on_the_belt() {
    let (mut h, belt) = tutorial();
    quiet(&mut h);
    h.frame(0.01);
    h.press(Key::Space);
    assert!(belt.with(|s| s.playing()));
    for _ in 0..2000 {
        if h.spoken().iter().any(|l| l.contains("Shake it now.")) {
            break;
        }
        quiet(&mut h);
        h.frame(0.01);
    }
    assert!(
        h.spoken().iter().any(|l| l.contains("Shake it now.")),
        "{:?}",
        h.spoken()
    );
    assert!(belt.with(|s| s.shift().gift.is_some()), "a gift to shake");
}

#[test]
fn space_while_the_tutorial_says_the_wishes_starts_the_belt_after_them() {
    let (mut h, belt) = tutorial();
    // The wishes are still being said: the press is kept, not dropped.
    h.press(Key::Space);
    h.frame(0.01);
    assert!(!belt.with(|s| s.playing()));
    assert!(belt.with(|s| s.clock.held()));
    quiet(&mut h);
    h.frame(0.01);
    assert!(belt.with(|s| s.playing()), "the belt starts once they end");
}

#[test]
fn the_pace_rows_help_says_it_applies_to_gift_sort() {
    let mut h = headless();
    h.app.ctx.game.open = Some(parlor_core::OpenGame::new(
        ID,
        "Winter Wonderland",
        <winter_wonderland::Winter as parlor_core::menus::TitleGame>::settings,
    ));
    h.app
        .ctx
        .push(hearken::screens::SettingsScreen::<Parlor>::new());
    h.frame(0.0);
    h.press(Key::Return);
    h.frame(0.0);
    h.clear();
    h.press(Key::F1);
    let help = h.spoken().last().cloned().unwrap_or_default();
    assert!(help.ends_with("Pace applies to Gift Sort only."), "{help}");
}

/// Shift 1 at quick, every gift sent home as soon as it lands. With
/// `spam`, a status line is spoken before every gift is sent, and the belt
/// is held while it is. Gives the clock bonus said, the score and the time
/// held for speech.
fn sort_shift_one(spam: bool) -> (u64, u64, f64) {
    let (mut h, s) = start_at(Mode::Free, 1, Pace::Quick);
    let mut held = 0.0;
    let done = |h: &Headless<Parlor>| {
        h.spoken()
            .iter()
            .find(|l| l.starts_with("Shift 1 complete."))
            .cloned()
    };
    for _ in 0..5000 {
        if done(&h).is_some() {
            break;
        }
        let Some(key) = home_key(&s) else {
            quiet(&mut h);
            tick(&mut h, &s, 0.1);
            continue;
        };
        if spam {
            for _ in 0..3 {
                h.app.ctx.say("3 to go.");
                tick(&mut h, &s, 0.5);
                held += 0.5;
            }
        }
        h.press(key);
    }
    let line = done(&h).unwrap_or_else(|| panic!("{:?}", h.spoken()));
    let bonus = line
        .split(". ")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{line}"));
    let score = s.borrow().score();
    (bonus, score, held)
}

#[test]
fn info_keys_never_raise_the_clock_bonus() {
    let (bonus, score, _) = sort_shift_one(false);
    let (spam_bonus, spam_score, held) = sort_shift_one(true);
    assert!(held >= 3.0, "{held}");
    // Every second held comes off the time left, ten points a second.
    let lost = bonus - spam_bonus;
    assert!(
        (lost as f64 - held * 10.0).abs() <= 10.0,
        "{bonus} > {spam_bonus} for {held} held"
    );
    assert_eq!(spam_score, score - lost);
}

#[test]
fn status_from_the_first_frame_holds_only_for_the_budget() {
    // The status key every second from the first frame, the voice never
    // quiet: the danger moves once the information budget is spent.
    let (mut h, s) = start(Mode::Free, Pace::Normal);
    let before = clock(&s);
    let t = parlor_core::testkit::status_spam_until(
        &mut h,
        60,
        |h| tick(h, &s, 0.1),
        |_| clock(&s) < before,
    );
    let t = t.expect("the danger never moved");
    assert!(t <= parlor_core::testkit::SPAM_BOUND_S, "{t}");
}

#[test]
fn a_run_that_scores_0_goes_on_no_board() {
    use winter_wonderland::progress::{best_text, record_at};
    let mut h = Headless::new(Default::default(), Base);
    let ctx = &mut h.app.ctx;
    // An old save that kept a 0.
    Records::record(ctx, ID, GIFT_BOARD, 0);
    assert_eq!(best_text(ctx, GIFT_BOARD), "Not played yet");
    assert_eq!(
        record_at(ctx, GIFT_BOARD, Pace::Normal, 0),
        "Nothing goes on the board."
    );
    assert_eq!(
        record_at(ctx, GIFT_BOARD, Pace::Normal, 120),
        "Your first score here is saved."
    );
    assert_eq!(best_text(ctx, GIFT_BOARD), "Best 120");
}
