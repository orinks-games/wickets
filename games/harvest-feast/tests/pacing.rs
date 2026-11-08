//! Listening protection belongs to Harvest Feast's discrete station actions.
use harvest_feast::progress;
use harvest_feast::rules::{self, Stage};
use harvest_feast::{Kitchen, Mode, Phase, Progress, ID};
use hearken::{ControllerButton, Ctx, Headless, InputEvent, Key, State};
use parlor_core::{Challenge, Date, Parlor, Period, Records};
use std::cell::RefCell;
use std::rc::Rc;

struct Observed(Rc<RefCell<Kitchen>>);
impl State<Parlor> for Observed {
    fn name(&self) -> &'static str {
        "observed_kitchen"
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
fn start(mode: Mode, relaxed: bool) -> (Headless<Parlor>, Rc<RefCell<Kitchen>>) {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
    Progress {
        relaxed,
        ..Progress::default()
    }
    .store(&mut h.app.ctx);
    let k = Rc::new(RefCell::new(Kitchen::new(mode)));
    h.app.ctx.push(Observed(k.clone()));
    h.frame(0.0);
    h.press(Key::Space);
    (h, k)
}
fn frames(h: &mut Headless<Parlor>, seconds: f64) {
    for _ in 0..(seconds / 0.05).round() as usize {
        h.frame(0.05);
    }
}
fn elapsed(k: &Rc<RefCell<Kitchen>>) -> f64 {
    k.borrow().meal.elapsed
}
fn assert_frozen(h: &mut Headless<Parlor>, k: &Rc<RefCell<Kitchen>>) {
    let before = elapsed(k);
    frames(h, 30.0);
    assert_eq!(elapsed(k), before);
}
#[test]
fn relaxed_call_waits_for_listening_and_the_same_press_starts_the_dish() {
    let (mut h, k) = start(Mode::Course(1), true);
    frames(&mut h, 3.0);
    assert_eq!(k.borrow().meal.dishes[0].stage, Stage::Due);
    assert_frozen(&mut h, &k);
    h.press(Key::Space);
    assert!(k.borrow().meal.dishes[0].started());
    assert_eq!(k.borrow().meal.dishes[0].scores, vec![100]);
    assert_frozen(&mut h, &k); // hear action feedback before the next wait
    h.press(Key::Space); // resumes, but must not stir early
    assert_eq!(k.borrow().meal.dishes[0].fumbles, 0);
    let before = elapsed(&k);
    frames(&mut h, 2.0);
    assert!((elapsed(&k) - before - 1.0).abs() < 0.001);
}
#[test]
fn status_repeat_recipe_hint_listen_and_navigation_wait_without_changing_quality() {
    let (mut h, k) = start(Mode::Course(2), true);
    for key in [
        Key::S,
        Key::R,
        Key::Up,
        Key::Down,
        Key::H,
        Key::T,
        Key::Right,
        Key::Num1,
    ] {
        h.press(key);
        assert_frozen(&mut h, &k);
        h.press(Key::Space);
    }
    assert!(k
        .borrow()
        .meal
        .dishes
        .iter()
        .all(|d| d.fumbles == 0 && d.ruined == 0));
}
#[test]
fn controller_status_and_action_use_the_same_listening_rules() {
    let (mut h, k) = start(Mode::Course(1), true);
    h.connect_pad();
    h.send(InputEvent::button(ControllerButton::Y));
    h.send(InputEvent::button_up(ControllerButton::Y));
    assert_frozen(&mut h, &k);
    h.send(InputEvent::button(ControllerButton::A));
    h.send(InputEvent::button_up(ControllerButton::A));
    frames(&mut h, 3.0);
    assert_frozen(&mut h, &k);
    h.send(InputEvent::button(ControllerButton::A));
    h.send(InputEvent::button_up(ControllerButton::A));
    assert!(k.borrow().meal.dishes[0].started());
    assert_eq!(k.borrow().meal.dishes[0].fumbles, 0);
}
#[test]
fn pace_stays_fixed_during_a_course_and_resume_waits_for_the_status() {
    let (mut h, k) = start(Mode::Course(1), true);
    let mut p = Progress::load(&mut h.app.ctx);
    p.relaxed = false;
    p.store(&mut h.app.ctx);
    h.press(Key::Escape);
    h.press(Key::Escape);
    assert_frozen(&mut h, &k);
    h.press(Key::Space);
    let before = elapsed(&k);
    frames(&mut h, 1.0);
    assert!((elapsed(&k) - before - 0.5).abs() < 0.001);
}
#[test]
fn standard_courses_and_other_modes_do_not_freeze_or_slow() {
    let (mut h, k) = start(Mode::Course(1), false);
    h.press(Key::S);
    frames(&mut h, 1.0);
    assert!((elapsed(&k) - 1.0).abs() < 0.001);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    for mode in [
        Mode::Helpings { round: 1, total: 0 },
        Mode::Thanksgiving { year: 2026 },
        Mode::Challenge(harvest_feast::ChallengeRun::new(ch)),
    ] {
        let (mut h, k) = start(mode, true);
        h.press(Key::S);
        frames(&mut h, 1.0);
        assert!((elapsed(&k) - 1.0).abs() < 0.001);
    }
}
#[test]
fn relaxed_meal_can_finish_with_slow_listening_and_multistroke_actions() {
    let (mut h, k) = start(Mode::Course(3), true);
    for _ in 0..3000 {
        if !matches!(k.borrow().phase, Phase::Cooking) {
            break;
        }
        let urgent = k
            .borrow()
            .meal
            .most_urgent()
            .map(|i| k.borrow().meal.dishes[i].station());
        if let Some(station) = urgent {
            let key = match station.number() {
                1 => Key::Num1,
                2 => Key::Num2,
                3 => Key::Num3,
                4 => Key::Num4,
                _ => Key::Num5,
            };
            h.press(key);
            assert_frozen(&mut h, &k);
            h.press(Key::Space);
        } else {
            h.press(Key::S); // safely inspect before resuming the next wait
            h.press(Key::Space);
        }
        h.frame(0.05);
    }
    assert!(k.borrow().meal.over);
    assert_eq!(k.borrow().meal.outcome().fumbles, 0);
    assert_eq!(k.borrow().meal.outcome().ruined, 0);
    frames(&mut h, 30.0);
    assert_eq!(h.app.top_name(), "observed_kitchen");
    h.press(Key::S);
    assert_eq!(h.app.top_name(), "observed_kitchen");
    h.press(Key::Space);
    assert!(Records::best(&mut h.app.ctx, ID, &progress::paced_course_board(3, true)).is_some());
    assert!(Records::best(&mut h.app.ctx, ID, &progress::course_board(3)).is_none());
    assert!(Progress::load(&mut h.app.ctx).open(4));
}
#[test]
fn original_rules_and_standard_board_identifiers_are_unchanged() {
    assert!(!Progress::default().relaxed);
    assert_eq!(
        progress::paced_course_board(1, false),
        progress::course_board(1)
    );
    assert_ne!(
        progress::paced_course_board(1, true),
        progress::course_board(1)
    );
    assert_eq!(rules::START_WINDOW, 4.0);
    assert_eq!(rules::MIN_WINDOW, 2.0);
}

#[test]
fn a_multistroke_task_waits_between_primary_presses_without_navigation() {
    let (mut h, k) = start(Mode::Course(3), true);
    // Celery starts on the board, then needs several chopping strokes.
    h.press(Key::Num2);
    h.press(Key::Space);
    frames(&mut h, 3.0);
    h.press(Key::Space); // start celery
    h.press(Key::Space); // resume until chopping is called
    frames(&mut h, 1.0);
    let needed = k.borrow().meal.dishes[0].current().unwrap().strokes;
    assert!(needed > 1);
    for _ in 0..needed {
        assert_frozen(&mut h, &k);
        h.press(Key::Space);
    }
    assert_eq!(k.borrow().meal.dishes[0].scores, vec![100, 100]);
    assert_eq!(k.borrow().meal.dishes[0].fumbles, 0);
}

#[test]
fn two_due_dishes_keep_their_deadlines_while_the_player_listens() {
    let (mut h, k) = start(Mode::Course(2), true);
    {
        let mut kitchen = k.borrow_mut();
        for dish in &mut kitchen.meal.dishes {
            dish.clock = dish.current().unwrap().wait;
            dish.stage = Stage::Due;
        }
    }
    h.press(Key::S);
    assert_frozen(&mut h, &k);
    h.press(Key::Space);
    h.press(Key::Num3);
    assert_frozen(&mut h, &k);
    h.press(Key::Space);
    assert!(k.borrow().meal.dishes.iter().all(|d| d.scores == vec![100]));
}

#[test]
fn rebound_primary_resumes_and_instructional_keys_hold_the_clock() {
    let (mut h, k) = start(Mode::Course(1), true);
    let _ = h.app.ctx.bindings.set_chord(
        parlor_core::ParlorAction::Primary,
        hearken::Chord::plain(Key::P),
    );
    h.press(Key::Return); // explains the rebound action
    assert_frozen(&mut h, &k);
    assert!(h.spoken().iter().any(|line| line.contains("P acts")));
    h.press(Key::P);
    h.press(Key::Num5); // unused station also gives time to hear its explanation
    assert_frozen(&mut h, &k);
    h.press(Key::P);
    frames(&mut h, 3.0);
    h.press(Key::P);
    assert!(k.borrow().meal.dishes[0].started());
    assert_eq!(k.borrow().meal.dishes[0].fumbles, 0);
}

#[test]
fn escape_from_a_completed_relaxed_meal_saves_before_leaving() {
    let (mut h, k) = start(Mode::Course(1), true);
    // Finish the rules normally, then exercise the screen's terminal Escape.
    {
        let mut kitchen = k.borrow_mut();
        for _ in 0..1000 {
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
        kitchen.phase = Phase::Served(2.5);
    }
    h.press(Key::Escape);
    assert_ne!(h.app.top_name(), "pause");
    assert_ne!(h.app.top_name(), "observed_kitchen");
    assert!(Records::best(&mut h.app.ctx, ID, &progress::paced_course_board(1, true)).is_some());
    assert!(Progress::load(&mut h.app.ctx).open(2));
}
