//! Regression tests for the review fixes: progress that never goes back,
//! run totals that a replay cannot pile up, combos that only pay on a
//! cleared show, a quiet count-in, and a saved show's best that belongs to
//! that show.

use hearken::{Ctx, Headless, InputEvent, Key, State};
use liberty_lights::progress::{self, Progress};
use liberty_lights::rules::{self, generate, judge, Kind, Launch, Pos};
use liberty_lights::{title, ID};
use parlor_core::menus;
use parlor_core::{Parlor, Records, Rng};

const SEED: u64 = 4;

/// Stands in for the launcher beneath the title.
struct Base;

impl State<Parlor> for Base {
    fn name(&self) -> &'static str {
        "base"
    }
    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        String::new()
    }
}

/// The title, tutorial read, so the cursor starts on Play.
fn start() -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(title());
    h.frame(0.0);
    h
}

fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

fn last(h: &Headless<Parlor>) -> String {
    h.spoken().last().cloned().unwrap_or_default()
}

/// Moves the cursor to the first row starting with `label`.
fn goto(h: &mut Headless<Parlor>, label: &str) {
    let rows = menus::rows_of(&h.app.top_lines());
    let at = rows
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row starts with {label:?}: {rows:?}"));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

fn listen(h: &mut Headless<Parlor>) {
    h.press(Key::Space);
    for _ in 0..1000 {
        if spoke(h, "Your turn.") {
            return;
        }
        h.frame(0.05);
    }
    panic!("the show never handed over: {:?}", h.spoken());
}

fn digit(kind: Kind) -> Key {
    match kind {
        Kind::Boom => Key::Num1,
        Kind::Crackle => Key::Num2,
        Kind::Whistle => Key::Num3,
        Kind::Sparkle => Key::Num4,
    }
}

fn perform(h: &mut Headless<Parlor>, launches: &[Launch]) {
    h.press(Key::Space);
    let mut ms = -1500;
    for l in launches {
        while ms < l.ms {
            h.frame(0.01);
            ms += 10;
        }
        let hold = match l.pos {
            Pos::Left => Some(Key::Left),
            Pos::Right => Some(Key::Right),
            Pos::Center => None,
        };
        if let Some(k) = hold {
            h.send(InputEvent::key(k));
        }
        h.press(digit(l.kind));
        if let Some(k) = hold {
            h.send(InputEvent::key_up(k));
        }
    }
}

/// From a show's Ready phase, plays it perfectly and submits.
fn play_perfect(h: &mut Headless<Parlor>, level: u32) {
    listen(h);
    perform(h, &generate(level, &mut Rng::seeded(SEED)));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "results");
}

#[test]
fn replaying_an_earlier_level_keeps_start_level_where_it_was() {
    let mut h = start();
    let mut p = Progress::load(&mut h.app.ctx);
    p.next_level = 10;
    p.store(&mut h.app.ctx);
    // Title row 4, Level select, then level 2.
    for _ in 0..3 {
        h.press(Key::Down);
    }
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "level_select");
    h.press(Key::Home);
    h.press(Key::Down);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "show");
    play_perfect(&mut h, 2);
    assert!(h.spoken()[0].contains("Level 2 cleared, 100 percent,"));
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.next_level(), 10);
    assert!(p.is_cleared(2));
}

#[test]
fn replaying_a_level_does_not_pile_up_the_run_total() {
    let mut h = start();
    h.press(Key::Return);
    play_perfect(&mut h, 1);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, progress::TOTAL_BOARD),
        Some(1000)
    );
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return); // Play again
    assert_eq!(h.app.top_name(), "show");
    h.press(Key::S);
    assert!(
        !spoke(&h, "show points so far"),
        "a replay should start from the total before the level: {:?}",
        h.spoken()
    );
    play_perfect(&mut h, 1);
    assert!(h.spoken()[0].starts_with("Level 1 cleared, "));
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, progress::TOTAL_BOARD),
        Some(1000)
    );
}

#[test]
fn combos_pay_only_when_the_show_clears() {
    let show: Vec<Launch> = (0..4)
        .map(|i| Launch::new(Kind::from_index(i), Pos::Center, i as i32 * 2000))
        .collect();
    // Every target hit perfectly, buried in ten extra fireworks: 50 percent.
    let mut spray = show.clone();
    for i in 0..10 {
        spray.push(Launch::new(Kind::Sparkle, Pos::Left, 9000 + i * 100));
    }
    let j = judge(&show, &spray, false);
    assert_eq!(j.accuracy, 50);
    assert!(!j.passed());
    assert_eq!(j.best_combo, 4);
    assert_eq!(j.combo_bonus, 0);
    assert_eq!(j.points, 500);
    // Clean, the same hits pass and the combo pays.
    let j = judge(&show, &show, false);
    assert_eq!(j.combo_bonus, 300);
}

#[test]
fn a_failed_show_with_a_combo_says_it_did_not_pay() {
    let mut h = start();
    // Level 16 has four fireworks; hit them all and launch eleven extras.
    for _ in 0..3 {
        h.press(Key::Down);
    }
    h.press(Key::Return);
    h.press(Key::End);
    h.press(Key::Up);
    for _ in 0..4 {
        h.press(Key::Up);
    }
    h.press(Key::Return);
    assert!(h.spoken().iter().any(|l| l.starts_with("Level 16.")));
    listen(&mut h);
    let mut launches = generate(16, &mut Rng::seeded(SEED));
    let end = launches.last().unwrap().ms;
    for i in 0..11 {
        launches.push(Launch::new(Kind::Boom, Pos::Center, end + 300 + i * 50));
    }
    perform(&mut h, &launches);
    h.clear();
    h.press(Key::Return);
    let summary = h.spoken()[0].clone();
    assert!(summary.contains(" percent, 60 needed."), "{summary}");
    goto(&mut h, "Hear the results again");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(
        last(&h).contains("but combos only pay on a cleared show."),
        "{}",
        last(&h)
    );
    assert!(!h.played().iter().any(|k| k == "liberty-lights/cue/combo"));
}

#[test]
fn starting_your_count_in_silences_the_turn_instructions() {
    let mut h = start();
    h.press(Key::Return);
    listen(&mut h);
    let before = h.speech.borrow().stop_calls();
    h.press(Key::Space);
    assert!(h.speech.borrow().stop_calls() > before);
    assert!(h.played().iter().any(|k| k == "liberty-lights/count/tick"));
}

#[test]
fn saving_a_new_freestyle_show_clears_the_old_shows_best() {
    let mut h = start();
    Records::record(&mut h.app.ctx, ID, progress::MY_SHOW_BOARD, 900);
    Records::record(&mut h.app.ctx, ID, "level 1", 700);
    // Title row 3: Freestyle show, then Compose a new show.
    h.press(Key::Down);
    h.press(Key::Down);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "freestyle_menu");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "freestyle");
    h.press(Key::Num1);
    h.frame(0.5);
    h.press(Key::Num4);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "freestyle_done");
    h.press(Key::Down); // Save this show
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Show saved."));
    let ctx = &mut h.app.ctx;
    assert_eq!(Records::best(ctx, ID, progress::MY_SHOW_BOARD), None);
    assert_eq!(Records::best(ctx, ID, "level 1"), Some(700));
    assert_eq!(Progress::load(ctx).saved_show.len(), 2);
}
