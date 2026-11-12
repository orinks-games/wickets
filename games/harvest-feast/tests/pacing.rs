//! Pace and the speech hold: every dish's clock stands still while a line
//! is being spoken, and runs at the meal's pace.
use harvest_feast::progress;
use harvest_feast::thanksgiving;
use harvest_feast::{ChallengeRun, Kitchen, Mode, Phase, Progress, ID};
use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::pace::{self, ClockState};
use parlor_core::{Challenge, Challenges, Date, Pace, Parlor, Period, Records};
use std::cell::RefCell;
use std::rc::Rc;

struct Observed(Rc<RefCell<Kitchen>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "harvest_feast_observed"
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

fn headless() -> Headless<Parlor> {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
    h
}

/// A meal of `kitchen` begun at `pace`, with the opening lines said.
fn begin(mut h: Headless<Parlor>, kitchen: Kitchen) -> (Headless<Parlor>, Rc<RefCell<Kitchen>>) {
    let k = Rc::new(RefCell::new(kitchen));
    h.app.ctx.push(Observed(k.clone()));
    h.frame(0.0);
    h.press(Key::Space);
    quiet(&mut h);
    (h, k)
}

fn start(mode: Mode, pace: Pace) -> (Headless<Parlor>, Rc<RefCell<Kitchen>>) {
    let mut h = headless();
    pace::set(&mut h.app.ctx, ID, pace);
    begin(h, Kitchen::new(mode))
}

/// Speech has finished: the voice is quiet.
fn quiet(h: &mut Headless<Parlor>) {
    h.app.ctx.stop_speech();
}

fn elapsed(k: &Rc<RefCell<Kitchen>>) -> f64 {
    k.borrow().meal.elapsed
}

/// One frame of `dt` straight to the kitchen, with nothing else moving.
fn tick(h: &mut Headless<Parlor>, k: &Rc<RefCell<Kitchen>>, dt: f64) {
    k.borrow_mut().update(&mut h.app.ctx, dt);
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn modes(h: &Headless<Parlor>) -> Vec<Mode> {
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    vec![
        Mode::Course(1),
        Mode::Helpings { round: 1, total: 0 },
        Mode::Thanksgiving { year: 2026 },
        Mode::Challenge(ChallengeRun::new(ch)),
    ]
}

#[test]
fn normal_is_the_default_and_the_old_relaxed_switch_carries_over() {
    let mut h = headless();
    Progress::load(&mut h.app.ctx);
    assert_eq!(progress::pace(&h.app.ctx), Pace::Normal);

    let mut h = headless();
    Progress {
        relaxed: true,
        ..Progress::default()
    }
    .store(&mut h.app.ctx);
    Records::record(&mut h.app.ctx, ID, "course 2 relaxed", 700);
    assert_eq!(progress::pace(&h.app.ctx), Pace::Relaxed);
    let (h, k) = begin(h, Kitchen::new(Mode::Course(1)));
    let mut h = h;
    assert_eq!(k.borrow().clock.pace(), Pace::Relaxed);
    assert_eq!(pace::stored(&h.app.ctx, ID), Some(Pace::Relaxed));
    assert_eq!(
        Records::best(
            &mut h.app.ctx,
            ID,
            &progress::paced_course_board(2, Pace::Relaxed)
        ),
        Some(700),
        "the old relaxed board moves to the relaxed board's name"
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, "course 2 relaxed"), None);
}

#[test]
fn every_meal_holds_for_speech_and_runs_at_its_pace() {
    let h = headless();
    for mode in modes(&h) {
        for pace in Pace::ALL {
            let (mut h, k) = start(mode.clone(), pace);
            assert_eq!(k.borrow().phase, Phase::Cooking, "{mode:?}");
            assert_eq!(k.borrow().clock.pace(), pace);
            let before = elapsed(&k);
            h.app.ctx.say("A line still being spoken.");
            tick(&mut h, &k, 1.0);
            assert_eq!(elapsed(&k), before, "{mode:?} holds for speech");
            assert_eq!(k.borrow().clock.state(), ClockState::Held);
            quiet(&mut h);
            tick(&mut h, &k, 1.0);
            assert!(
                close(elapsed(&k) - before, pace.scale()),
                "{mode:?} at {pace:?}: {}",
                elapsed(&k) - before
            );
        }
    }
}

#[test]
fn information_keys_hold_the_kitchen_only_while_they_are_spoken() {
    let (mut h, k) = start(Mode::Course(2), Pace::Quick);
    for key in [
        Key::S,
        Key::R,
        Key::Up,
        Key::Down,
        Key::H,
        Key::Right,
        Key::Num1,
    ] {
        let before = elapsed(&k);
        h.press(key);
        tick(&mut h, &k, 0.5);
        assert_eq!(elapsed(&k), before, "{key:?} is being spoken");
        quiet(&mut h);
        tick(&mut h, &k, 0.5);
        assert!(close(elapsed(&k), before + 0.5), "{key:?}");
    }
}

#[test]
fn the_kitchen_holds_until_the_recipe_call_has_been_said() {
    let mut h = headless();
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    let k = Rc::new(RefCell::new(Kitchen::new(Mode::Course(1))));
    h.app.ctx.push(Observed(k.clone()));
    h.frame(0.0);
    h.press(Key::Space);
    h.frame(0.05);
    assert!(k.borrow().clock.held(), "Cooking is being said");
    for _ in 0..2000 {
        if !k.borrow().clock.held() {
            break;
        }
        assert_eq!(elapsed(&k), 0.0, "nothing cooks while a line is said");
        h.frame(0.05);
    }
    h.frame(0.5);
    assert!(
        elapsed(&k) > 0.0,
        "the kitchen runs once the lines are said"
    );
    let said = |h: &Headless<Parlor>| h.spoken().iter().any(|l| l.starts_with("Recipe:"));
    for _ in 0..2000 {
        if said(&h) {
            break;
        }
        h.frame(0.05);
    }
    assert!(said(&h), "{:?}", h.spoken());
    let before = elapsed(&k);
    h.frame(0.05);
    assert!(k.borrow().clock.held(), "the recipe call is being said");
    assert_eq!(elapsed(&k), before);
}

#[test]
fn a_hold_never_lasts_forever() {
    let (mut h, k) = start(Mode::Course(1), Pace::Quick);
    for _ in 0..400 {
        h.app.ctx.say("Still talking.");
        tick(&mut h, &k, 0.1);
    }
    assert!(elapsed(&k) > 10.0, "{}", elapsed(&k));
}

#[test]
fn pace_stays_fixed_for_the_meal_and_carries_through_a_run() {
    let (mut h, k) = start(Mode::Course(1), Pace::Relaxed);
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    h.press(Key::Escape);
    h.press(Key::Escape);
    assert_eq!(k.borrow().clock.pace(), Pace::Relaxed);
    let next = Kitchen::new(Mode::Helpings {
        round: 2,
        total: 300,
    })
    .at_pace(Pace::Relaxed);
    let (_h, k) = begin(h, next);
    assert_eq!(
        k.borrow().clock.pace(),
        Pace::Relaxed,
        "the run keeps its pace"
    );
}

/// Cooks the rest of the meal by its rules, perfectly, then lets the
/// dinner bell ring out to the results.
fn finish(h: &mut Headless<Parlor>, k: &Rc<RefCell<Kitchen>>) {
    {
        let mut kitchen = k.borrow_mut();
        for _ in 0..4000 {
            if kitchen.meal.over {
                break;
            }
            kitchen.meal.update(0.05);
            if let Some(i) = kitchen.meal.most_urgent() {
                let station = kitchen.meal.dishes[i].station();
                kitchen.meal.act(station);
            }
        }
        assert!(kitchen.meal.over);
        kitchen.phase = Phase::Served(0.1);
    }
    for _ in 0..10 {
        h.frame(0.05);
    }
}

#[test]
fn a_relaxed_course_keeps_its_own_best_and_still_opens_the_next() {
    let (mut h, k) = start(Mode::Course(1), Pace::Relaxed);
    finish(&mut h, &k);
    assert_ne!(h.app.top_name(), "harvest_feast_observed");
    let relaxed = progress::paced_course_board(1, Pace::Relaxed);
    assert_eq!(relaxed, "course 1-relaxed");
    assert!(Records::best(&mut h.app.ctx, ID, &relaxed).is_some());
    assert!(Records::best(&mut h.app.ctx, ID, &progress::course_board(1)).is_none());
    assert!(Progress::load(&mut h.app.ctx).open(2));
    assert!(
        h.spoken().iter().any(|l| l.contains("relaxed score")),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn normal_and_quick_share_the_course_board() {
    let (mut h, k) = start(Mode::Course(1), Pace::Normal);
    finish(&mut h, &k);
    let normal = Records::best(&mut h.app.ctx, ID, &progress::course_board(1));
    assert!(normal.is_some());
    assert_eq!(
        progress::paced_course_board(1, Pace::Quick),
        progress::course_board(1)
    );
}

#[test]
fn a_relaxed_challenge_and_thanksgiving_keep_their_own_results() {
    let (mut h, k) = start(
        Mode::Challenge(ChallengeRun::new(Challenge::current(
            &headless().app.ctx,
            ID,
            Period::Daily,
        ))),
        Pace::Relaxed,
    );
    finish(&mut h, &k);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), None);
    assert!(Challenges::best_at(&mut h.app.ctx, &ch, Pace::Relaxed).is_some());

    let (mut h, k) = start(Mode::Thanksgiving { year: 2026 }, Pace::Relaxed);
    finish(&mut h, &k);
    assert_eq!(Records::best(&mut h.app.ctx, ID, thanksgiving::BOARD), None);
    assert!(Records::best(
        &mut h.app.ctx,
        ID,
        &Pace::Relaxed.board(thanksgiving::BOARD)
    )
    .is_some());
    let keep = thanksgiving::Keepsakes::load(&mut h.app.ctx);
    let year = keep.year(2026).unwrap();
    assert_eq!(year.best, 0);
    assert!(year.relaxed_best > 0);
}

/// A save from before the Pace setting: the old relaxed switch on, and
/// courses 1 and 2 with old relaxed boards.
fn old_save(h: &mut Headless<Parlor>) {
    Progress {
        stars: vec![3, 2, 1],
        relaxed: true,
        ..Progress::default()
    }
    .store(&mut h.app.ctx);
    Records::record(&mut h.app.ctx, ID, "course 1", 800);
    Records::record(&mut h.app.ctx, ID, "course 1 relaxed", 1500);
    Records::record(&mut h.app.ctx, ID, "course 2 relaxed", 900);
}

#[test]
fn old_relaxed_boards_survive_the_pace_row_being_touched_first() {
    let mut h = headless();
    old_save(&mut h);
    // The player changes the Pace row before cooking anything.
    pace::set(&mut h.app.ctx, ID, Pace::Quick);
    let pages = harvest_feast::records_pages(&mut h.app.ctx);
    assert_eq!(pace::stored(&h.app.ctx, ID), Some(Pace::Quick));
    for (board, best) in [("course 1-relaxed", 1500), ("course 2-relaxed", 900)] {
        assert_eq!(
            Records::best(&mut h.app.ctx, ID, board),
            Some(best),
            "{board}"
        );
    }
    for old in ["course 1 relaxed", "course 2 relaxed"] {
        assert_eq!(Records::best(&mut h.app.ctx, ID, old), None, "{old}");
    }
    let all = pages.join(" ");
    assert!(all.contains("best 800, relaxed best 1500"), "{all}");
    assert!(all.contains("relaxed best 900"), "{all}");
    assert!(!all.contains("best 0"), "{all}");

    // Running it again changes nothing.
    progress::migrate_pace(&mut h.app.ctx);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, "course 1-relaxed"),
        Some(1500)
    );
}

#[test]
fn moving_an_old_relaxed_board_keeps_the_higher_score() {
    let mut h = headless();
    old_save(&mut h);
    Records::record(&mut h.app.ctx, ID, "course 1-relaxed", 1200);
    Records::record(&mut h.app.ctx, ID, "course 2-relaxed", 1100);
    progress::migrate_pace(&mut h.app.ctx);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, "course 1-relaxed"),
        Some(1500)
    );
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, "course 2-relaxed"),
        Some(1100)
    );
    assert_eq!(Records::best(&mut h.app.ctx, ID, "course 1 relaxed"), None);
    assert_eq!(pace::stored(&h.app.ctx, ID), Some(Pace::Relaxed));
}

#[test]
fn course_select_names_both_boards() {
    let mut h = headless();
    old_save(&mut h);
    Records::record(&mut h.app.ctx, ID, "course 3", 400);
    h.app.ctx.push(harvest_feast::CourseSelect::new(1));
    h.frame(0.0);
    let first = h.spoken().join(" ");
    assert!(first.contains("best 800, relaxed best 1500"), "{first}");
    let mut lines = Vec::new();
    for _ in 0..2 {
        h.clear();
        h.press(Key::Down);
        lines.push(h.spoken().join(" "));
    }
    assert!(lines[0].contains("relaxed best 900"), "{lines:?}");
    assert!(lines[1].contains("best 400"), "{lines:?}");
    for line in lines.iter().chain([&first]) {
        assert!(!line.contains("best 0"), "{line}");
        assert!(!line.contains("no score yet"), "{line}");
    }
}

#[test]
fn records_leave_out_a_board_with_no_score() {
    let mut h = headless();
    Progress {
        stars: vec![1; 8],
        best_rounds: 4,
        ..Progress::default()
    }
    .store(&mut h.app.ctx);
    Records::record(
        &mut h.app.ctx,
        ID,
        &Pace::Relaxed.board(progress::HELPINGS_BOARD),
        2000,
    );
    Records::record(&mut h.app.ctx, ID, "course 1-relaxed", 700);
    let all = harvest_feast::records_pages(&mut h.app.ctx).join(" ");
    assert!(
        all.contains("Second helpings: relaxed best 2000, most rounds 4."),
        "{all}"
    );
    assert!(all.contains(": 1 star, relaxed best 700."), "{all}");
    assert!(!all.contains("best 0"), "{all}");
}

#[test]
fn status_from_the_first_frame_holds_only_for_the_budget() {
    // The status key every second from the first frame, the voice never
    // quiet: the danger moves once the information budget is spent.
    let (mut h, k) = start(Mode::Course(1), Pace::Normal);
    let before = elapsed(&k);
    let t = parlor_core::testkit::status_spam_until(
        &mut h,
        60,
        |h| tick(h, &k, 0.1),
        |_| elapsed(&k) > before,
    );
    let t = t.expect("the danger never moved");
    assert!(t <= parlor_core::testkit::SPAM_BOUND_S, "{t}");
}

#[test]
fn a_meal_that_scores_0_goes_on_no_board() {
    use harvest_feast::results::score_paced;
    use harvest_feast::rules::Outcome;
    let mut h = Headless::new(Default::default(), Base);
    let board = progress::paced_course_board(1, Pace::Normal);
    // An old save that kept a 0.
    Records::record(&mut h.app.ctx, ID, &board, 0);
    let zero = Outcome {
        qualities: Vec::new(),
        average: 0,
        stars: 0,
        fumbles: 0,
        ruined: 0,
        seconds: 60.0,
    };
    let served = score_paced(
        Mode::Course(1),
        zero.clone(),
        false,
        0,
        Pace::Normal,
        &mut h.app.ctx,
    );
    h.app.ctx.push(served);
    h.frame(0.0);
    h.press(Key::R);
    let said = h.spoken().join(" ");
    assert!(
        said.contains("0 points, so nothing goes on the board."),
        "{said}"
    );
    let all = harvest_feast::records_pages(&mut h.app.ctx).join(" ");
    assert!(!all.contains("best 0"), "{all}");
    for mode in [
        Mode::Helpings { round: 1, total: 0 },
        Mode::Thanksgiving { year: 2026 },
    ] {
        h.clear();
        let served = score_paced(mode, zero.clone(), false, 0, Pace::Normal, &mut h.app.ctx);
        h.app.ctx.push(served);
        h.frame(0.0);
        h.press(Key::R);
        let said = h.spoken().join(" ");
        assert!(
            said.contains("0 points, so nothing goes on the board."),
            "{said}"
        );
        assert!(!said.contains("keepsake"), "{said}");
    }
}
