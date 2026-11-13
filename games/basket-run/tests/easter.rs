//! Easter morning on the title: locked with a countdown most of the year,
//! the bunny's news in the run-up, a golden-egg hunt on the day, and a
//! keepsake for each Easter played.

use basket_run::easter::{self, Keepsakes};
use basket_run::rules::{Kind, Mode, Run, EASTER_ROUNDS};
use basket_run::{sound, title_with_seed, ID, INFO};
use hearken::{Headless, Key};
use parlor_core::{menus, testkit, Date, Parlor, ParlorData, Records, Rng};

const SEED: u64 = 0xE66;
const DT: f64 = 0.05;

/// The row's place on the title: after Play and Endless.
const ROW: usize = 2;

/// Easter Sunday in the years the tests visit.
const EASTERS: [(i32, u32, u32); 6] = [
    (2025, 4, 20),
    (2026, 4, 5),
    (2027, 3, 28),
    (2028, 4, 16),
    (2029, 4, 1),
    (2030, 4, 21),
];

/// The title on `date`, tutorial read, nothing spoken yet.
fn title_on(date: Date) -> Headless<Parlor> {
    let data = ParlorData {
        today: Some(date),
        ..Default::default()
    };
    let mut h = Headless::new(data, title_with_seed(SEED));
    menus::mark_tutorial_read(&mut h.app.ctx, ID);
    parlor_core::pace::set(&mut h.app.ctx, ID, parlor_core::Pace::Quick);
    h.frame(0.0);
    h.clear();
    h
}

/// One frame with the voice quiet, so the meadow never holds for speech.
fn step(h: &mut Headless<Parlor>, dt: f64) {
    h.app.ctx.stop_speech();
    h.frame(dt);
}

fn rows(h: &Headless<Parlor>) -> Vec<String> {
    menus::rows_of(&h.app.top_lines())
}

fn goto(h: &mut Headless<Parlor>, label: &str) {
    let at = rows(h)
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row {label:?}: {:?}", rows(h)));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

fn nums(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|t| !t.is_empty() && *t != ".")
        .filter_map(|t| t.parse().ok())
        .collect()
}

/// One frame of a decent player, as in the walkthrough: catch the next
/// egg, dodge hazards, bank when full or when nothing is coming.
fn play_frame(h: &mut Headless<Parlor>) {
    let lines = h.app.top_lines();
    let phase = lines[0].rsplit(", ").next().unwrap_or("").to_string();
    if phase == "playing" || phase == "ready" {
        let l = nums(&lines[1]);
        let (lane, cart) = (l[0] as usize, l[2] as usize);
        let b = nums(&lines[2]);
        let (basket, cap) = (b[0] as usize, b[1] as usize);
        let time = *nums(&lines[3]).last().unwrap();
        let mut falling: Vec<(String, usize, f64)> = lines[4..]
            .iter()
            .map(|line| {
                let mut parts = line.trim_start_matches("falling: ").split(' ');
                let kind = parts.next().unwrap().to_string();
                let lane = parts.next().unwrap().parse().unwrap();
                let left = parts.next().unwrap().parse().unwrap();
                (kind, lane, left)
            })
            .collect();
        falling.sort_by(|a, b| a.2.total_cmp(&b.2));
        let is_egg = |k: &str| k == "egg" || k == "golden";
        let egg = falling.iter().find(|f| is_egg(&f.0));
        let to_bank = basket > 0
            && (basket >= cap
                || time < 6.0
                || egg.is_none_or(|e| e.2 > 1.2 && lane.abs_diff(cart) <= 1));
        let mut target = if to_bank {
            cart
        } else {
            egg.map_or(lane, |e| e.1)
        };
        let danger = |at: usize| {
            falling
                .iter()
                .any(|f| !is_egg(&f.0) && f.1 == at && f.2 < 0.4)
        };
        if danger(target) {
            target = if target > 1 { target - 1 } else { target + 1 };
        }
        if target < lane {
            h.press(Key::Left);
        } else if target > lane {
            h.press(Key::Right);
        } else if to_bank && lane == cart {
            h.press(Key::Space);
        }
    }
    step(h, DT);
}

/// Plays to the results, reading every page on the way.
fn hunt(h: &mut Headless<Parlor>) {
    for _ in 0..40_000 {
        match h.app.top_name() {
            "results" => return,
            "pages" => h.press(Key::Return),
            "basket_run_run" => play_frame(h),
            other => panic!("unexpected screen {other}"),
        }
    }
    panic!("the hunt never ended");
}

#[test]
fn the_launcher_names_the_hunt_only_around_easter_sunday() {
    let event = INFO.event.expect("Basket Run holds an event");
    for (year, month, day) in EASTERS {
        let sunday = Date::new(year, month, day);
        let on = event(sunday).expect("on Easter");
        assert_eq!(on.title, "Easter morning");
        for off in [-1, 1] {
            assert!(event(Date::from_days(sunday.days() + off)).is_some());
        }
        for off in [-2, 2, -10, 30] {
            assert!(
                event(Date::from_days(sunday.days() + off)).is_none(),
                "{year} {off}"
            );
        }
    }
    // Easter moves: the 5th of April is Easter in 2026 but not in 2027.
    assert!(event(Date::new(2026, 4, 5)).is_some());
    assert!(event(Date::new(2027, 4, 5)).is_none());
}

#[test]
fn the_row_is_locked_with_a_countdown_the_rest_of_the_year() {
    let mut h = title_on(Date::new(2026, 10, 16));
    assert_eq!(rows(&h)[ROW], "Easter morning, locked");
    goto(&mut h, "Easter morning");
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "Easter is in 163 days."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run");
    assert_eq!(
        h.spoken(),
        ["Easter morning opens on Easter Sunday. Easter is in 163 days."]
    );
    assert!(!h.played().contains(&sound::CATCH_GOLDEN.to_string()));
}

#[test]
fn the_run_up_tells_how_the_bunny_is_getting_on() {
    let mut h = title_on(Date::new(2027, 3, 22));
    goto(&mut h, "Easter morning");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run");
    assert_eq!(
        h.spoken(),
        ["Easter is in 6 days. The Easter bunny is polishing the golden eggs until they ring."]
    );
    assert!(h.played().contains(&sound::CATCH_GOLDEN.to_string()));
}

#[test]
fn rows_keep_their_places_before_on_and_after_the_day() {
    let first = rows(&title_on(Date::new(2026, 10, 16)));
    for (year, month, day) in EASTERS {
        let sunday = Date::new(year, month, day).days();
        for off in -12..=3 {
            let d = Date::from_days(sunday + off);
            let list = rows(&title_on(d));
            assert_eq!(list.len(), first.len(), "{d:?}");
            assert_eq!(list[0].split(',').next(), first[0].split(',').next());
            assert!(list[ROW].starts_with("Easter morning"), "{d:?}");
            assert_eq!(
                !list[ROW].ends_with(", locked"),
                easter::is_open(d),
                "{d:?}"
            );
        }
    }
}

#[test]
fn on_the_day_the_hunt_opens_and_every_key_answers() {
    let mut h = title_on(Date::new(2026, 4, 6));
    assert_eq!(rows(&h)[ROW], "Easter morning");
    goto(&mut h, "Easter morning");
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "golden eggs worth double"), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "basket_run_run");
    assert!(
        spoke(&h, "Easter morning, round 1 of 3: sunrise in the garden."),
        "{:?}",
        h.spoken()
    );
    testkit::every_action_answers(&mut h);
}

#[test]
fn the_hunt_has_no_bees_and_golden_eggs_count_double() {
    for n in 1..=EASTER_ROUNDS {
        let spec = basket_run::rules::round_spec(Mode::Easter, n);
        assert_eq!(spec.quota, 0.0, "round {n}");
        assert_eq!((spec.bee, spec.chasers), (0.0, 0.0), "round {n}");
        assert!(spec.golden > 0.0, "round {n}");
    }
    let easter = Run::new(Mode::Easter, Rng::seeded(1));
    let meadow = Run::new(Mode::Meadow, Rng::seeded(1));
    assert_eq!(
        easter.value_of(Kind::Golden),
        2 * meadow.value_of(Kind::Golden)
    );
    assert_eq!(easter.value_of(Kind::Egg), meadow.value_of(Kind::Egg));
}

#[test]
fn a_hunt_played_through_is_kept_for_its_year() {
    let mut h = title_on(Date::new(2027, 3, 28));
    goto(&mut h, "Easter morning");
    h.press(Key::Return);
    hunt(&mut h);
    assert!(spoke(&h, "Happy Easter."), "{:?}", h.spoken());
    assert!(
        spoke(&h, "Your keepsake for Easter 2027 is saved."),
        "{:?}",
        h.spoken()
    );
    let keep = Keepsakes::load(&mut h.app.ctx);
    let k = keep.year(2027).expect("kept");
    assert_eq!(k.hunts, 1);
    assert!(keep.year(2026).is_none());
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, "easter morning"),
        Some(k.best)
    );
    let page = keep.page(Date::new(2027, 3, 28));
    assert!(page.starts_with("Keepsakes. Easter 2027: best"), "{page}");
}
