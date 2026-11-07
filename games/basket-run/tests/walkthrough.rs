//! Basket Run through the headless app: the title, a run, its results.

use basket_run::rules::{Kind, ROUNDS};
use basket_run::run::MAX_LEAD_IN;
use basket_run::{sound, title_with_seed};
use hearken::{Ctx, Headless, Key, State};
use parlor_core::{Challenge, Challenges, Date, Parlor, ParlorData, Period, Records};

const SEED: u64 = 0xE66;
const DT: f64 = 0.05;

/// The date every test plays on, so challenges are fixed.
const TODAY: Date = Date::new(2026, 10, 16);

fn data() -> ParlorData {
    ParlorData {
        today: Some(TODAY),
        ..Default::default()
    }
}

/// A fresh save's title, with the cursor moved from Tutorial to Play.
fn start() -> Headless<Parlor> {
    let mut h = fresh();
    h.press(Key::Home);
    h.clear();
    h
}

/// A fresh save's title, as it first opens.
fn fresh() -> Headless<Parlor> {
    Headless::new(data(), title_with_seed(SEED))
}

/// The run screen on top, nothing spoken yet.
fn into_meadow() -> Headless<Parlor> {
    let mut h = start();
    h.press(Key::Return); // Play the meadow
    assert_eq!(h.app.top_name(), "basket_run_run");
    h.clear();
    h
}

/// Moves the cursor to the first row starting with `label`.
fn select(h: &mut Headless<Parlor>, label: &str) {
    let rows = parlor_core::menus::rows_of(&h.app.top_lines());
    let at = rows
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row {label}: {rows:?}"));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

/// Reads a challenge briefing through to the run, with Enter.
fn brief(h: &mut Headless<Parlor>) {
    for _ in 0..6 {
        if h.app.top_name() != "pages" {
            return;
        }
        h.press(Key::Return);
    }
}

fn spoke(h: &Headless<Parlor>, needle: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(needle))
}

fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

/// The run screen as numbers, read from its text lines.
#[derive(Debug, Default)]
struct View {
    lane: usize,
    lanes: usize,
    cart: usize,
    basket: usize,
    cap: usize,
    score: u64,
    time: f64,
    phase: String,
    falling: Vec<(String, usize, f64)>,
}

fn nums(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|t| !t.is_empty() && *t != ".")
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn view(h: &Headless<Parlor>) -> View {
    let lines = h.app.top_lines();
    let mut v = View {
        phase: lines[0].rsplit(", ").next().unwrap_or("").to_string(),
        ..View::default()
    };
    let l = nums(&lines[1]);
    (v.lane, v.lanes, v.cart) = (l[0] as usize, l[1] as usize, l[2] as usize);
    let b = nums(&lines[2]);
    (v.basket, v.cap, v.score) = (b[0] as usize, b[1] as usize, b[2] as u64);
    v.time = *nums(&lines[3]).last().unwrap();
    for line in &lines[4..] {
        let rest = line.trim_start_matches("falling: ");
        let mut parts = rest.split(' ');
        let kind = parts.next().unwrap().to_string();
        let lane: usize = parts.next().unwrap().parse().unwrap();
        let left: f64 = parts.next().unwrap().parse().unwrap();
        v.falling.push((kind, lane, left));
    }
    v
}

fn is_egg(kind: &str) -> bool {
    kind == "egg" || kind == "golden"
}

/// One frame of a decent player: catch the next egg, dodge hazards, bank
/// when full, when nothing is coming, and before the bell.
fn play_frame(h: &mut Headless<Parlor>) {
    let v = view(h);
    if v.phase == "playing" || v.phase == "ready" {
        let mut falling = v.falling.clone();
        falling.sort_by(|a, b| a.2.total_cmp(&b.2));
        let egg = falling.iter().find(|f| is_egg(&f.0));
        let to_bank = v.basket > 0
            && (v.basket >= v.cap
                || v.time < 6.0
                || egg.is_none_or(|e| e.2 > 1.2 && v.lane.abs_diff(v.cart) <= 1));
        let mut target = if to_bank {
            v.cart
        } else {
            egg.map_or(v.lane, |e| e.1)
        };
        // Never be under a hazard as it lands.
        let danger = |lane: usize| {
            falling
                .iter()
                .any(|f| !is_egg(&f.0) && f.1 == lane && f.2 < 0.4)
        };
        if danger(target) {
            target = if target > 1 { target - 1 } else { target + 1 };
        }
        if target < v.lane {
            h.press(Key::Left);
        } else if target > v.lane {
            h.press(Key::Right);
        } else if to_bank && v.lane == v.cart {
            h.press(Key::Space);
        }
    }
    h.frame(DT);
}

/// Plays until `done` says so, reading every page that opens on the way.
fn autoplay(h: &mut Headless<Parlor>, mut done: impl FnMut(&Headless<Parlor>) -> bool) {
    for _ in 0..40_000 {
        if done(h) {
            return;
        }
        match h.app.top_name() {
            "pages" => h.press(Key::Return),
            "basket_run_run" => play_frame(h),
            other => panic!("unexpected screen {other}"),
        }
    }
    panic!("autoplay never finished");
}

/// Runs frames, reading pages, without moving.
fn idle_until(h: &mut Headless<Parlor>, mut done: impl FnMut(&Headless<Parlor>) -> bool) {
    for _ in 0..40_000 {
        if done(h) {
            return;
        }
        if h.app.top_name() == "pages" {
            h.press(Key::Return);
        } else {
            h.frame(DT);
        }
    }
    panic!("never finished");
}

fn falling_loops(h: &Headless<Parlor>) -> Vec<hearken::LiveSound> {
    h.app
        .ctx
        .audio
        .live()
        .into_iter()
        .filter(|s| s.looping && s.key.starts_with("basket-run/fall/"))
        .collect()
}

#[test]
fn the_title_announces_itself_and_plays_its_music() {
    let mut h = fresh();
    assert_eq!(
        h.spoken(),
        [
            "Basket Run. New here? Start with the tutorial.",
            "Tutorial. 6 of 11."
        ]
    );
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
    h.clear();
    h.press(Key::Home);
    assert_eq!(h.spoken(), ["Play, the meadow, no score yet. 1 of 11."]);
}

#[test]
fn the_first_run_opens_round_one() {
    let mut h = start();
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(
        spoke(&h, "Round 1 of 8. 3 lanes, the cart in lane 1 on the left."),
        "{:?}",
        h.spoken()
    );
    assert_eq!(h.app.ctx.audio.bed(), Some(sound::MEADOW));
    assert_eq!(h.app.ctx.audio.music(), None);
}

#[test]
fn the_tutorial_catches_and_banks_an_egg() {
    let mut h = fresh();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "lesson");
    assert!(spoke(
        &h,
        "It is Easter morning, and eggs tumble from the trees over the meadow."
    ));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "basket_run");

    let lanes = basket_run::tutorial::lanes();
    let lesson = basket_run::tutorial::lesson_on(&mut h.app.ctx, lanes.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    // Follow each falling egg's lane, and read through each new kind.
    for _ in 0..20_000 {
        if h.app.top_name() == "pages" {
            h.press(Key::Return);
            continue;
        }
        if h.app.top_name() != "lesson" {
            break;
        }
        let (lane, want) = lanes.with(|s| {
            let r = &s.run;
            let want = if r.caught_total == 0 {
                r.falling
                    .iter()
                    .find(|f| f.kind == Kind::Egg)
                    .map(|f| f.lane)
            } else {
                Some(r.cart_lane())
            };
            (r.lane, want)
        });
        match want {
            Some(w) if w < lane => h.press(Key::Left),
            Some(w) if w > lane => h.press(Key::Right),
            Some(_) if lanes.with(|s| s.run.caught_total > 0) => h.press(Key::Space),
            _ => {}
        }
        h.frame(DT);
    }
    assert!(spoke(&h, "Caught, with a woody thump"), "{:?}", h.spoken());
    assert!(
        spoke(&h, "Banked as points. Tutorial done."),
        "{:?}",
        h.spoken()
    );
    assert_eq!(h.app.top_name(), "basket_run");
}

#[test]
fn the_first_egg_is_explained_before_it_falls_and_the_meadow_waits() {
    let mut h = into_meadow();
    for _ in 0..((MAX_LEAD_IN + 2.0) / DT) as usize {
        h.frame(DT);
        if h.app.top_name() == "pages" {
            break;
        }
    }
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "Your first egg:"), "{:?}", h.spoken());
    let loops = falling_loops(&h);
    assert_eq!(loops.len(), 1);
    assert!(loops[0].paused, "the egg waits while the page is read");
    assert_eq!(loops[0].key, sound::falling(Kind::Egg));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(spoke(&h, "Egg in lane"), "{:?}", h.spoken());
    assert!(!falling_loops(&h)[0].paused);
}

#[test]
fn stepping_speaks_the_lane_and_the_fence() {
    let mut h = into_meadow();
    h.press(Key::Left);
    assert_eq!(h.spoken(), ["The fence on the left. Lane 1."]);
    assert!(played(&h, sound::EDGE));
    h.clear();
    h.press(Key::Right);
    assert_eq!(h.spoken(), ["Lane 2."]);
    assert!(played(&h, sound::STEP));
    h.clear();
    h.press(Key::Left);
    assert_eq!(h.spoken(), ["Lane 1, the cart."]);
    assert!(played(&h, sound::AT_CART));
    h.clear();
    h.press(Key::Up);
    assert!(spoke(&h, "Sideways only"), "{:?}", h.spoken());
}

#[test]
fn status_repeat_hint_cart_and_help_all_answer() {
    let mut h = into_meadow();
    h.press(Key::S);
    assert_eq!(h.spoken(), ["Lane 1 of 3, at the cart, basket 0 of 4."]);
    h.clear();
    h.press(Key::S);
    let status = h.spoken().join(" ");
    assert!(status.contains("3 baskets left."), "{status}");
    h.clear();
    h.press(Key::R);
    assert!(spoke(&h, "Round 1 of 8."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::H);
    assert_eq!(h.spoken().len(), 1);
    h.clear();
    h.press(Key::Tab);
    assert_eq!(h.spoken(), ["You are at the cart, lane 1."]);
    assert!(played(&h, sound::BEACON));
    h.clear();
    h.press(Key::Space); // during the lead-in, Space starts the round
    assert_eq!(h.spoken(), ["Go."]);
    assert!(played(&h, sound::READY));
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.spoken(), ["Your basket is empty."]);
    assert!(played(&h, sound::EMPTY));
    h.press(Key::Right);
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.spoken(), ["Cart 1 lane left, lane 1."]);
    h.clear();
    h.press(Key::F1);
    let help = h.spoken().join(" ");
    assert!(help.contains("step one lane"), "{help}");
    assert!(help.contains("Space empties your basket"), "{help}");
}

#[test]
fn drops_sound_from_their_lane_falling_lower_and_louder() {
    let mut h = into_meadow();
    idle_until(&mut h, |h| {
        h.app.top_name() == "basket_run_run" && !falling_loops(h).is_empty()
    });
    let lane = view(&h).falling[0].1;
    let first = falling_loops(&h)[0].pos.unwrap();
    for _ in 0..30 {
        h.frame(DT);
    }
    let later = falling_loops(&h)[0].pos.unwrap();
    assert!(later.z < first.z, "it falls: {first:?} then {later:?}");
    assert_eq!(first.x, sound::lane_x(lane - 1, 3));
    // The pitch of the loop comes down as it falls.
    let pitches: Vec<f32> = h
        .sounds
        .calls()
        .iter()
        .filter_map(|c| match c {
            hearken::Call::SetMix { mix, .. } => Some(mix.pitch),
            _ => None,
        })
        .filter(|p| *p < 1.36 && *p > 0.74 && *p != 1.0)
        .collect();
    assert!(pitches.len() > 5);
    assert!(pitches.last() < pitches.first(), "{pitches:?}");
}

#[test]
fn a_good_player_clears_round_one_and_hears_every_catch_and_bank() {
    let mut h = into_meadow();
    autoplay(&mut h, |h| {
        h.app.top_name() == "pages" && h.spoken().iter().any(|l| l.contains("Round 1 cleared."))
    });
    assert!(played(&h, sound::CATCH_EGG));
    assert!(played(&h, sound::BANK));
    assert!(played(&h, sound::READY));
    assert!(played(&h, sound::BELL));
    assert!(played(&h, sound::ROUND_CLEAR));
    assert!(spoke(&h, "1 in the basket."));
    assert!(spoke(&h, "Banked "));
    assert!(spoke(&h, "Next, round 2: Mud clods start falling"));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(
        spoke(
            &h,
            "Round 2 of 8. 3 lanes, the cart in lane 3 on the right."
        ),
        "{:?}",
        h.spoken()
    );
    let v = view(&h);
    assert!(v.score > 0);
    assert_eq!(v.lane, 3, "you start at the cart");
}

#[test]
fn standing_still_fails_round_one_and_the_score_is_saved() {
    let mut h = into_meadow();
    idle_until(&mut h, |h| h.app.top_name() == "results");
    assert!(
        spoke(&h, "banked, run over. Score 0, saved."),
        "{:?}",
        h.spoken()
    );
    assert!(played(&h, sound::CRACK_EGG));
    assert_eq!(h.app.ctx.audio.music(), Some(sound::LOSE));
    assert!(falling_loops(&h).is_empty());
    assert_eq!(h.app.ctx.audio.bed(), None);
    assert_eq!(
        Records::best(&mut h.app.ctx, basket_run::ID, "meadow"),
        Some(0)
    );
    h.press(Key::End);
    h.clear();
    h.press(Key::Return); // Back to the title
    assert_eq!(h.app.top_name(), "basket_run");
    assert!(spoke(&h, "Play, the meadow, best 0."), "{:?}", h.spoken());
}

#[test]
fn a_better_run_sets_a_new_best_and_a_worse_one_does_not() {
    let mut h = into_meadow();
    // Clear round 1, then stand still through round 2.
    autoplay(&mut h, |h| {
        h.app.top_name() == "pages" && h.spoken().iter().any(|l| l.contains("Round 1 cleared."))
    });
    idle_until(&mut h, |h| h.app.top_name() == "results");
    assert!(spoke(&h, ", saved."), "{:?}", h.spoken());
    let best = Records::best(&mut h.app.ctx, basket_run::ID, "meadow").unwrap();
    assert!(best > 0);
    assert_eq!(h.app.ctx.audio.music(), Some(sound::LOSE));
    // Play again, standing still.
    h.press(Key::Home);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    h.clear();
    idle_until(&mut h, |h| h.app.top_name() == "results");
    assert!(spoke(&h, &format!(", best {best}.")), "{:?}", h.spoken());
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "basket_run");
    assert!(spoke(&h, &format!("Play, the meadow, best {best}.")));
}

#[test]
fn pause_holds_the_meadow_and_returns_to_the_title() {
    let mut h = into_meadow();
    idle_until(&mut h, |h| {
        h.app.top_name() == "basket_run_run" && !falling_loops(h).is_empty()
    });
    let before = view(&h).falling[0].2;
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    assert!(falling_loops(&h).iter().all(|s| s.paused));
    for _ in 0..20 {
        h.frame(DT);
    }
    h.clear();
    h.press(Key::Return); // Resume
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(
        spoke(&h, "Back in the meadow. Lane 1 of 3, basket 0 of 4."),
        "{:?}",
        h.spoken()
    );
    assert_eq!(view(&h).falling[0].2, before, "nothing fell while paused");
    assert!(falling_loops(&h).iter().all(|s| !s.paused));
    h.press(Key::Escape);
    for _ in 0..3 {
        h.press(Key::Down);
    }
    h.press(Key::Return); // Return to the title
    assert_eq!((h.app.top_name(), h.app.depth()), ("basket_run", 1));
    assert!(falling_loops(&h).is_empty(), "the drops stop with the run");
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
    assert_eq!(h.app.ctx.audio.bed(), None);
    assert_eq!(
        Records::best(&mut h.app.ctx, basket_run::ID, "meadow"),
        None
    );
}

#[test]
fn drop_calls_can_be_turned_off() {
    let mut h = into_meadow();
    // The first egg is explained; later ones are called by lane.
    idle_until(&mut h, |h| spoke(h, "Egg, lane"));
    h.press(Key::Escape);
    for _ in 0..3 {
        h.press(Key::Down);
    }
    h.press(Key::Return); // Back to the title
    select(&mut h, "Settings");
    h.press(Key::Return);
    h.press(Key::Return); // the game's own category
    select(&mut h, "Spoken call-outs");
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "Spoken call-outs: off. From your next run, play by ear"),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::Down);
    h.press(Key::Up);
    assert!(spoke(&h, "Spoken call-outs: off. 1 of"), "{:?}", h.spoken());
    h.press(Key::Escape);
    h.press(Key::Escape);
    h.press(Key::Home);
    h.press(Key::Return);
    h.clear();
    for _ in 0..400 {
        h.frame(DT);
        if h.app.top_name() == "pages" {
            h.press(Key::Return);
        }
    }
    assert!(!spoke(&h, "Egg, lane"), "{:?}", h.spoken());
    assert!(played(&h, sound::CRACK_EGG), "the eggs still fall");
}

#[test]
fn the_sound_check_drops_each_kind_and_tours_the_lanes() {
    let mut h = start();
    select(&mut h, "Sound check");
    h.clear();
    h.press(Key::Return); // Sound check
    assert_eq!(h.app.top_name(), "basket_run_sounds");
    assert_eq!(h.spoken(), ["Sound check.", "An egg falling. 1 of 31."]);
    assert_eq!(h.app.ctx.audio.music(), None);
    h.press(Key::Return);
    assert!(played(&h, sound::falling(Kind::Egg)));
    for _ in 0..60 {
        h.frame(DT);
    }
    assert!(played(&h, sound::CATCH_EGG), "it lands");
    assert!(falling_loops(&h).is_empty());
    h.press(Key::Down);
    h.press(Key::Down);
    h.press(Key::Return);
    assert!(played(&h, sound::falling(Kind::Mud)));
    for _ in 0..60 {
        h.frame(DT);
    }
    assert!(played(&h, sound::SPLAT));
    h.press(Key::Down);
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return); // Lanes, left to right
    for _ in 0..140 {
        h.frame(DT);
    }
    assert_eq!(
        h.spoken(),
        ["Lane 1.", "Lane 2.", "Lane 3.", "Lane 4.", "Lane 5."]
    );
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "from lane 1 on the far left"));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "basket_run");
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
}

#[test]
fn the_endless_meadow_is_locked_at_first() {
    let mut h = start();
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken(),
        ["The endless meadow opens once you clear all 8 rounds of the meadow."]
    );
    assert_eq!(h.app.top_name(), "basket_run");
}

#[test]
fn practice_explains_every_kind_and_costs_nothing() {
    let mut h = start();
    for _ in 0..4 {
        h.press(Key::Down);
    }
    h.press(Key::Return); // Practice lanes
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(spoke(&h, "Practice, round 1."));
    let mut intros = 0;
    for _ in 0..2400 {
        if h.app.top_name() == "pages" {
            if h.spoken().iter().any(|l| {
                ["Your first egg:", "A golden egg:", "A mud clod:", "A bee:"]
                    .iter()
                    .any(|m| l.starts_with(m))
            }) {
                intros += 1;
            }
            h.clear();
            h.press(Key::Return);
        } else {
            h.frame(DT);
        }
    }
    assert!(intros >= 3, "{intros} kinds explained");
    let v = view(&h);
    assert_eq!(v.lanes, 3);
    let lines = h.app.top_lines();
    assert!(lines[3].contains("baskets 3"), "{lines:?}");
}

#[test]
fn the_daily_meadow_opens_with_its_rule_and_counts_toward_the_streak() {
    let mut h = start();
    select(&mut h, "Challenges");
    h.press(Key::Return); // Challenges
    assert_eq!(h.app.top_name(), "basket_run_challenges");
    h.clear();
    h.press(Key::Return); // the daily
    assert_eq!(h.app.top_name(), "pages");
    assert!(
        spoke(&h, "Twist of the day: One 60-second round"),
        "{:?}",
        h.spoken()
    );
    assert!(played(&h, sound::TWIST), "the twist is sounded");
    brief(&mut h);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert_eq!(view(&h).lanes, 5);
    autoplay(&mut h, |h| h.app.top_name() == "results");
    assert!(spoke(&h, "Daily challenge result:"), "{:?}", h.spoken());
    assert!(spoke(&h, "Daily streak: 1 day."), "{:?}", h.spoken());
    assert!(spoke(&h, "A new one tomorrow."), "{:?}", h.spoken());
    let music = h.app.ctx.audio.music();
    assert!(
        music == Some(sound::WIN) || music == Some(sound::LOSE),
        "{music:?}"
    );
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "basket_run_challenges");
    assert!(spoke(&h, "Daily, Twist of the day"), "{:?}", h.spoken());
    assert!(spoke(&h, "Done, best"), "{:?}", h.spoken());
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
}

#[test]
fn a_good_player_clears_the_whole_meadow_and_opens_endless() {
    let mut h = into_meadow();
    autoplay(&mut h, |h| h.app.top_name() == "results");
    assert!(
        spoke(&h, &format!("Meadow cleared: all {ROUNDS} rounds.")),
        "{:?}",
        &h.spoken()[h.spoken().len().saturating_sub(6)..]
    );
    assert_eq!(h.app.ctx.audio.music(), Some(sound::WIN));
    assert!(played(&h, sound::falling(Kind::Mud)));
    assert!(played(&h, sound::falling(Kind::Bee)));
    assert!(played(&h, sound::falling(Kind::Golden)));
    h.press(Key::Escape);
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(spoke(&h, "Endless, round 1."), "{:?}", h.spoken());
}

/// A stand-in for the launcher beneath the title.
struct Root;

impl State<Parlor> for Root {
    fn name(&self) -> &'static str {
        "root"
    }
    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        String::new()
    }
}

#[test]
fn back_to_the_parlor_leaves_and_stops_the_music() {
    let mut h = Headless::<Parlor>::new(Default::default(), Root);
    (basket_run::INFO.open)(&mut h.app.ctx);
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "basket_run");
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "root");
    assert_eq!(h.app.ctx.audio.music(), None);
}

// ---- Review fixes ----

use basket_run::run::{lead_in_for, speech_secs};
use hearken::speech::SpeechChannel;

/// Main-channel speech calls, with whether each interrupted.
fn calls(h: &Headless<Parlor>) -> Vec<(String, bool)> {
    h.speech.borrow().calls(SpeechChannel::Main)
}

#[test]
fn nothing_falls_until_the_round_announcement_has_been_heard() {
    let mut h = into_meadow();
    let intro = h.spoken().join(" ");
    // into_meadow cleared before the run spoke, so read it from the repeat key.
    h.press(Key::R);
    let intro = if intro.is_empty() {
        h.spoken().join(" ")
    } else {
        intro
    };
    assert!(intro.contains("Starting. Space skips the wait."), "{intro}");
    let wait = lead_in_for(&intro);
    assert!(wait >= speech_secs(&intro).min(basket_run::run::MAX_LEAD_IN));
    let mut t = 0.0;
    while view(&h).phase == "ready" {
        h.frame(DT);
        t += DT;
    }
    assert!(t + DT >= wait, "the lead-in lasted {t}s of {wait}s");
    assert!(
        t > basket_run::rules::LEAD_IN,
        "longer than the bare 4 seconds"
    );
}

#[test]
fn space_starts_the_round_early_and_hint_says_so() {
    let mut h = into_meadow();
    h.press(Key::H);
    assert!(spoke(&h, "Space starts the round."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Space);
    assert_eq!(view(&h).phase, "playing");
    assert_eq!(calls(&h), [("Go.".to_string(), true)]);
    // A second press is a bank again, not another start.
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.spoken(), ["Your basket is empty."]);
}

#[test]
fn hints_use_the_right_article() {
    let mut h = into_meadow();
    h.press(Key::Space);
    idle_until(&mut h, |h| {
        h.app.top_name() == "basket_run_run" && !view(h).falling.is_empty()
    });
    h.clear();
    h.press(Key::H);
    let hint = h.spoken().join(" ");
    assert!(
        hint.contains("an egg lands") || hint.contains("a golden egg lands"),
        "{hint}"
    );
    assert!(!hint.contains("a egg"), "{hint}");
}

#[test]
fn a_page_in_the_middle_of_play_is_not_skipped_by_space_or_the_arrows() {
    let mut h = into_meadow();
    h.press(Key::Space);
    while h.app.top_name() != "pages" {
        h.frame(DT);
    }
    assert!(spoke(&h, "Your first egg:"));
    for key in [Key::Space, Key::Right, Key::Tab, Key::S] {
        h.clear();
        h.press(key);
        assert_eq!(h.app.top_name(), "pages", "{key:?} kept the page");
        assert!(spoke(&h, "The meadow waits while you read."), "{key:?}");
    }
    h.clear();
    h.press(Key::R);
    assert!(spoke(&h, "Your first egg:"), "R repeats it");
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "the clock is stopped"), "{:?}", h.spoken());
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
}

#[test]
fn the_round_summary_survives_a_late_bank_press() {
    let mut h = into_meadow();
    autoplay(&mut h, |h| {
        h.app.top_name() == "pages" && h.spoken().iter().any(|l| l.contains("Round 1 cleared."))
    });
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.app.top_name(), "pages");
    h.press(Key::R);
    assert!(spoke(&h, "Round 1 cleared."), "{:?}", h.spoken());
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
}

#[test]
fn drop_calls_never_cut_off_an_important_line() {
    let mut h = into_meadow();
    let important = [
        "Round ",
        "Ten seconds",
        "Time.",
        "Mud!",
        "Stung!",
        "Banked ",
    ];
    let mut busy_until = 0.0;
    let mut t = 0.0;
    let mut seen = 0;
    let mut checked = 0;
    while h.app.top_name() != "results" {
        if h.app.top_name() == "pages" {
            h.press(Key::Return);
            busy_until = 0.0;
        } else {
            play_frame(&mut h);
            t += DT;
        }
        let all = calls(&h);
        for (text, interrupt) in &all[seen..] {
            let is_call = text.contains(", lane ") || text.starts_with("Basket ");
            if is_call && *interrupt {
                assert!(t >= busy_until, "{text:?} cut off a line at {t}");
                checked += 1;
            }
            if important.iter().any(|p| text.starts_with(p)) {
                busy_until = t + speech_secs(text) - DT;
            }
        }
        seen = all.len();
        if t > 400.0 {
            break;
        }
    }
    assert!(checked > 10, "only {checked} calls checked");
}

/// Catches eggs but never banks.
fn hoard_frame(h: &mut Headless<Parlor>) {
    let v = view(h);
    let mut falling = v.falling.clone();
    falling.sort_by(|a, b| a.2.total_cmp(&b.2));
    if let Some(egg) = falling.iter().find(|f| is_egg(&f.0)) {
        if egg.1 < v.lane {
            h.press(Key::Left);
        } else if egg.1 > v.lane {
            h.press(Key::Right);
        }
    }
    h.frame(DT);
}

#[test]
fn eggs_lost_to_the_bell_are_said_once() {
    let mut h = into_meadow();
    while h.app.top_name() != "results" {
        if h.app.top_name() == "pages" {
            h.press(Key::Return);
        } else {
            hoard_frame(&mut h);
        }
    }
    let all = h.spoken().join(" ");
    assert!(spoke(&h, "Time. 4 eggs lost."), "{all}");
    assert_eq!(all.matches("4 eggs lost").count(), 1, "{all}");
    assert!(all.contains("0 of 8 banked, run over."), "{all}");
}

// ---- Challenges ----

use basket_run::challenge::{challenge_tag, CHALLENGES_NAME};
use basket_run::rules::{Mode, Run, Twist, PATTERN_BONUS};

/// The title's Challenges row, opened.
fn into_challenges() -> Headless<Parlor> {
    let mut h = start();
    select(&mut h, "Challenges");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    h
}

fn last_lines(h: &Headless<Parlor>) -> Vec<String> {
    h.spoken()[h.spoken().len().saturating_sub(8)..].to_vec()
}

#[test]
fn the_title_offers_the_challenges_with_todays_state() {
    let mut h = start();
    h.press(Key::Down);
    h.press(Key::Down);
    h.clear();
    h.press(Key::Down);
    assert_eq!(h.spoken(), ["Challenges, daily not played yet. 4 of 11."]);
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "the weekly marathon"), "{:?}", h.spoken());
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "basket_run");
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
}

#[test]
fn each_challenge_row_says_its_best_and_what_it_is() {
    let mut h = into_challenges();
    let daily = Challenge::on(TODAY, basket_run::ID, Period::Daily);
    let twist = Twist::for_seed(daily.seed);
    assert!(
        spoke(
            &h,
            &format!(
                "Daily, Twist of the day: one 60-second round, today's twist: {}. Not played yet. 1 of 4.",
                twist.name()
            )
        ),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, twist.describe()), "{:?}", h.spoken());
    assert!(spoke(&h, "A new one tomorrow."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Down);
    let weekly = Challenge::on(TODAY, basket_run::ID, Period::Weekly);
    assert!(
        spoke(
            &h,
            &format!(
                "Weekly, The long meadow: {}. Not played yet. 2 of 4.",
                challenge_tag(&weekly)
            )
        ),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "A new one in 3 days."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Down);
    assert!(
        spoke(
            &h,
            "Monthly, The Easter festival: 3 rounds of egg sweeps, bee fences and nests. Not played yet. 3 of 4."
        ),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "A new one in 16 days."), "{:?}", h.spoken());
    h.press(Key::Down);
    h.press(Key::Return); // Back
    assert_eq!(h.app.top_name(), "basket_run");
}

#[test]
fn the_daily_briefing_states_the_twist_on_one_page_then_the_run_names_it() {
    let mut h = into_challenges();
    let daily = Challenge::on(TODAY, basket_run::ID, Period::Daily);
    let twist = Twist::for_seed(daily.seed);
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, &format!("Twist, {}:", twist.name())),
        "{:?}",
        h.spoken()
    );
    assert!(spoke(&h, "Enter to start."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(
        spoke(
            &h,
            &format!("Today's meadow, with the {} twist.", twist.name())
        ),
        "{:?}",
        h.spoken()
    );
    h.clear();
    parlor_core::testkit::read_help(&mut h);
    assert!(spoke(&h, "Today's twist,"), "{:?}", h.spoken());
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    h.press(Key::S);
    assert!(
        spoke(&h, &format!("Twist: {}.", twist.name())),
        "{:?}",
        h.spoken()
    );
    // The run's drops are the challenge's, the same for everyone.
    let lines = h.app.top_lines();
    assert!(
        lines[0].starts_with("Daily challenge, round 1"),
        "{lines:?}"
    );
    let expected = Run::new(Mode::Daily, daily.rng());
    assert_eq!(view(&h).cap, expected.capacity());
}

#[test]
fn leaving_a_challenge_from_pause_does_not_count() {
    let mut h = into_challenges();
    h.press(Key::Down);
    h.press(Key::Return); // the weekly
    brief(&mut h);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(
        spoke(&h, "Marathon, round 1 of 5. 4 lanes"),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    for _ in 0..3 {
        h.press(Key::Down);
    }
    h.press(Key::Return); // Leave the challenge
    assert_eq!(h.app.top_name(), "basket_run_challenges");
    let weekly = Challenge::current(&h.app.ctx, basket_run::ID, Period::Weekly);
    assert!(!Challenges::done(&mut h.app.ctx, &weekly));
}

#[test]
fn the_weekly_marathon_records_a_failed_run_and_its_streak() {
    let mut h = into_challenges();
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "The long meadow: Rounds 4 to 8"),
        "{:?}",
        h.spoken()
    );
    brief(&mut h);
    // Standing at the cart banks nothing: round 1 misses its target.
    idle_until(&mut h, |h| h.app.top_name() == "results");
    let lines = last_lines(&h);
    assert!(
        spoke(
            &h,
            "Weekly challenge result: 0. Your first score here is saved."
        ),
        "{lines:?}"
    );
    assert!(
        spoke(&h, "Weekly streak: 1 week. A new one in 3 days."),
        "{lines:?}"
    );
    assert_eq!(h.app.ctx.audio.music(), Some(sound::LOSE));
    let weekly = Challenge::current(&h.app.ctx, basket_run::ID, Period::Weekly);
    assert_eq!(Challenges::best(&mut h.app.ctx, &weekly), Some(0));
    assert_eq!(
        Challenges::streak(&mut h.app.ctx, basket_run::ID, Period::Weekly),
        1
    );
    // Play it again: the briefing again, and the same drops.
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "The long meadow:"), "{:?}", h.spoken());
    brief(&mut h);
    idle_until(&mut h, |h| h.app.top_name() == "results");
    assert!(spoke(&h, "Your best is 0."), "{:?}", last_lines(&h));
    // Escape does what the last row, Back to the challenges, does.
    h.press(Key::End);
    assert_eq!(
        h.app.top_lines().last().map(String::as_str),
        Some("> Back to the challenges")
    );
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    assert!(
        spoke(
            &h,
            "Weekly, The long meadow: 5 rounds back to back from round 4, each with an egg target. Done, best 0, a 1 week streak."
        ),
        "the row under the cursor reads back the result: {:?}",
        h.spoken()
    );
    // Records read all three, after the meadow's page and the keepsakes.
    h.press(Key::Escape);
    select(&mut h, "Records");
    h.press(Key::Return);
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "Weekly challenge, done, best 0, a 1 week streak."),
        "{:?}",
        h.spoken()
    );
    assert!(spoke(&h, "Monthly challenge, not played yet."));
}

#[test]
fn the_festival_calls_its_patterns_and_scores_them() {
    let mut h = into_challenges();
    h.press(Key::Down);
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "The Easter festival:"), "{:?}", h.spoken());
    brief(&mut h);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(
        spoke(&h, "Festival, round 1 of 3. 5 lanes"),
        "{:?}",
        h.spoken()
    );
    autoplay(&mut h, |h| h.app.top_name() == "results");
    assert!(played(&h, sound::PATTERN_CALL));
    assert!(
        spoke(&h, "Egg sweep from the") || spoke(&h, "Bee fence!") || spoke(&h, "A nest in lane"),
        "a pattern is called"
    );
    assert!(
        spoke(&h, "Monthly challenge result:"),
        "{:?}",
        last_lines(&h)
    );
    assert!(spoke(&h, "Monthly streak: 1 month. A new one in 16 days."));
    let monthly = Challenge::current(&h.app.ctx, basket_run::ID, Period::Monthly);
    assert!(Challenges::done(&mut h.app.ctx, &monthly));
    if played(&h, sound::PATTERN_DONE) {
        assert!(spoke(&h, &format!("caught. {PATTERN_BONUS} bonus points.")));
    }
}

#[test]
fn a_festival_pattern_call_silences_its_own_drop_calls() {
    let mut h = into_challenges();
    h.press(Key::End);
    h.press(Key::Up);
    h.press(Key::Return);
    brief(&mut h);
    h.press(Key::Space); // start at once
                         // Wait for the first pattern call, and listen a moment after it.
    idle_until(&mut h, |h| {
        spoke(h, "Egg sweep from the") || spoke(h, "Bee fence!") || spoke(h, "A nest in lane")
    });
    let at = h.spoken().len();
    // The pattern's own drops start within its span; none is called.
    for _ in 0..40 {
        h.frame(DT);
    }
    let after: Vec<String> = h.spoken()[at..].to_vec();
    assert!(
        !after.iter().any(|l| [
            "Egg, lane",
            "Bee, lane",
            "Golden egg, lane",
            "Mud clod, lane"
        ]
        .iter()
        .any(|c| l.starts_with(c))),
        "{after:?}"
    );
}

/// A stand-in for the launcher's challenge list.
struct Lobby;

impl State<Parlor> for Lobby {
    fn name(&self) -> &'static str {
        "lobby"
    }
    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        String::new()
    }
}

#[test]
fn the_launcher_opens_a_challenge_and_gets_it_back() {
    let mut h = Headless::<Parlor>::new(data(), Lobby);
    assert!(basket_run::INFO.challenge.is_some());
    (basket_run::INFO.challenge.unwrap())(&mut h.app.ctx, Period::Monthly);
    h.clear();
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "The Easter festival:"), "{:?}", h.spoken());
    brief(&mut h);
    assert_eq!(h.app.top_name(), "basket_run_run");
    h.press(Key::Escape);
    for _ in 0..3 {
        h.press(Key::Down);
    }
    h.press(Key::Return); // Leave the challenge: the parlor here
    h.frame(0.0);
    assert_eq!((h.app.top_name(), h.app.depth()), ("lobby", 1));
    assert_eq!(h.app.ctx.audio.music(), None);
    let monthly = Challenge::current(&h.app.ctx, basket_run::ID, Period::Monthly);
    assert!(
        !Challenges::done(&mut h.app.ctx, &monthly),
        "leaving does not count"
    );
}

#[test]
fn a_challenge_from_the_launcher_returns_there_when_finished() {
    let mut h = Headless::<Parlor>::new(data(), Lobby);
    basket_run::open_challenge(&mut h.app.ctx, Period::Weekly);
    h.frame(0.0);
    brief(&mut h);
    idle_until(&mut h, |h| h.app.top_name() == "results");
    assert!(
        spoke(&h, "Weekly challenge result: 0."),
        "{:?}",
        last_lines(&h)
    );
    h.clear();
    h.press(Key::End);
    assert_eq!(
        h.spoken().last().map(String::as_str),
        Some("Back to the challenges. 3 of 3."),
        "the row names where it goes"
    );
    h.press(Key::Return); // Back to the challenges, the launcher's here
    h.frame(0.0);
    assert_eq!((h.app.top_name(), h.app.depth()), ("lobby", 1));
    let weekly = Challenge::current(&h.app.ctx, basket_run::ID, Period::Weekly);
    assert!(Challenges::done(&mut h.app.ctx, &weekly));
}

fn score_stems(h: &Headless<Parlor>) -> Vec<String> {
    h.app
        .ctx
        .audio
        .live()
        .into_iter()
        .filter(|s| {
            s.looping && s.bus == hearken::Bus::MUSIC && s.key.starts_with("basket-run/score/")
        })
        .map(|s| s.key)
        .collect()
}

#[test]
fn the_meadow_plays_its_score_while_a_round_runs_and_stops_it_between() {
    let mut h = into_meadow();
    assert_eq!(
        score_stems(&h).len(),
        sound::SCORE.len(),
        "every stem starts together"
    );
    assert_eq!(h.app.ctx.audio.music(), None);
    autoplay(&mut h, |h| {
        h.app.top_name() == "pages" && h.spoken().iter().any(|l| l.contains("Round 1 cleared."))
    });
    for _ in 0..20 {
        h.frame(0.1);
    }
    assert!(score_stems(&h).is_empty(), "the score stops between rounds");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert_eq!(
        score_stems(&h).len(),
        sound::SCORE.len(),
        "and starts again with the next"
    );
}

#[test]
fn the_score_builds_with_rounds_and_streaks_and_hurries_and_darkens() {
    use basket_run::rules::{Mode, Phase};
    use basket_run::run::{Launch, RunScreen};
    use parlor_core::score::{DANGER, HURRY};
    let mut s = RunScreen::new(
        Launch {
            mode: Mode::Meadow,
            seed: Some(SEED),
        },
        None,
    );
    s.run.phase = Phase::Playing;
    let calm = s.mood();
    s.run.round = ROUNDS;
    s.run.streak = 10;
    let hot = s.mood();
    assert!(hot.intensity > calm.intensity + 0.5, "{calm:?} {hot:?}");
    assert_eq!(calm.flags & (1 << HURRY), 0);
    s.run.clock = s.run.spec.duration - 5.0;
    assert_ne!(
        s.mood().flags & (1 << HURRY),
        0,
        "the last ten seconds hurry it"
    );
    s.run.lives = 1;
    assert_ne!(
        s.mood().flags & (1 << DANGER),
        0,
        "one basket left darkens it"
    );
}

#[test]
fn results_back_row_matches_escape() {
    let mut h = into_meadow();
    idle_until(&mut h, |h| h.app.top_name() == "results");
    assert_eq!(
        parlor_core::menus::rows_of(&h.app.top_lines()),
        ["Try again", "Hear the results again", "Back to the title"]
    );
    h.press(Key::Escape);
    let by_escape = (h.app.top_name(), h.app.depth());
    let mut h = into_meadow();
    idle_until(&mut h, |h| h.app.top_name() == "results");
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!((h.app.top_name(), h.app.depth()), by_escape);
    assert_eq!(by_escape, ("basket_run", 1));
}

#[test]
fn the_call_outs_row_switches_and_says_so() {
    let mut h = start();
    select(&mut h, "Settings");
    h.press(Key::Return);
    h.press(Key::Return); // the game's own category
    select(&mut h, "Spoken call-outs");
    assert!(h
        .app
        .top_lines()
        .contains(&"> Spoken call-outs: on".to_string()));
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Spoken call-outs: off."), "{:?}", h.spoken());
    assert!(h
        .app
        .top_lines()
        .contains(&"> Spoken call-outs: off".to_string()));
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Spoken call-outs: on."), "{:?}", h.spoken());
}

#[test]
fn records_start_with_records_and_name_the_challenges() {
    let mut h = start();
    select(&mut h, "Records");
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(
            &h,
            "Records. The meadow: no score yet, 0 of 8 rounds cleared."
        ),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(
            &h,
            "Keepsakes. None yet: Easter morning opens on Easter Sunday."
        ),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "Challenges. Daily challenge, not played yet."),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn every_play_key_answers_in_the_meadow() {
    let mut h = into_meadow();
    h.press(Key::Space);
    parlor_core::testkit::every_action_answers(&mut h);
}
