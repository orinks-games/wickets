//! Dustfall through the headless app: the title, the colony phase, a night
//! watched to its end, the tutorial, the settings and the sound check.

use dustfall::colony::{COLONY_NAME, PLACER_NAME, SLOTS_NAME};
use dustfall::night::NIGHT_NAME;
use dustfall::rules::Run;
use dustfall::{title_with_seed, tutorial, CHALLENGES_NAME, TITLE_NAME};
use hearken::{Headless, Key};
use parlor_core::{Date, Parlor, ParlorData};

const SEED: u64 = 0xd05;
const DT: f64 = 0.1;
const TODAY: Date = Date::new(2026, 11, 4);

fn data() -> ParlorData {
    ParlorData {
        today: Some(TODAY),
        ..Default::default()
    }
}

/// A fresh save's title, with the cursor on Play.
fn start() -> Headless<Parlor> {
    let mut h = Headless::new(data(), title_with_seed(SEED));
    select(&mut h, "Play");
    h.clear();
    h
}

fn rows(h: &Headless<Parlor>) -> Vec<String> {
    parlor_core::menus::rows_of(&h.app.top_lines())
}

fn select(h: &mut Headless<Parlor>, label: &str) {
    let rows = rows(h);
    let at = rows
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row {label}: {rows:?}"));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

fn choose(h: &mut Headless<Parlor>, label: &str) {
    select(h, label);
    h.press(Key::Return);
}

fn spoke(h: &Headless<Parlor>, needle: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(needle))
}

fn all_spoken(h: &Headless<Parlor>) -> String {
    h.spoken().join(" | ")
}

fn nums(s: &str) -> Vec<i64> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter_map(|t| t.parse().ok())
        .collect()
}

/// Into the colony phase of a new colony.
fn colony() -> Headless<Parlor> {
    let mut h = start();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), COLONY_NAME, "{}", all_spoken(&h));
    h
}

/// Into night 1 of a new colony, with the opening beats played.
fn night() -> Headless<Parlor> {
    let mut h = colony();
    h.clear();
    choose(&mut h, "Ready for night 1");
    assert_eq!(h.app.top_name(), NIGHT_NAME);
    settle(&mut h);
    h
}

/// Frames until the raiders' beats are done and it is the player's turn.
fn settle(h: &mut Headless<Parlor>) {
    for _ in 0..600 {
        if h.app.top_name() != NIGHT_NAME {
            return;
        }
        let first = h.app.top_lines()[0].clone();
        let n = nums(&first);
        if first.contains(", turn,") && n[4] == 0 {
            return;
        }
        h.frame(DT);
    }
    panic!("the raiders never settled: {:?}", h.app.top_lines());
}

/// The nearest raider's column, from the night screen's lines.
fn nearest_col(h: &Headless<Parlor>) -> Option<i64> {
    h.app
        .top_lines()
        .iter()
        .find(|l| l.starts_with("raider "))
        .map(|l| {
            let at = l.split(" at ").nth(1).expect("a position");
            nums(at)[0]
        })
}

/// Watches the night on top to its end: the gun goes under the nearest
/// raider and fires, and the turn ends when there is nothing to shoot.
fn watch_night(h: &mut Headless<Parlor>) {
    for _ in 0..3000 {
        if h.app.top_name() != NIGHT_NAME {
            return;
        }
        settle(h);
        if h.app.top_name() != NIGHT_NAME {
            return;
        }
        let gun = nums(&h.app.top_lines()[0])[2];
        match nearest_col(h) {
            None => h.frame(DT),
            Some(col) if col == gun => h.press(Key::Space),
            Some(col) if col < gun => h.press(Key::Left),
            Some(_) => h.press(Key::Right),
        }
        h.frame(DT);
    }
    panic!("the night never ended: {:?}", h.app.top_lines());
}

#[test]
fn the_title_has_the_shared_rows_and_play_first() {
    let h = start();
    let rows = rows(&h);
    for label in [
        "Play",
        "Tutorial",
        "Help",
        "Sound check",
        "Records",
        "Settings",
        "Challenges",
    ] {
        assert!(
            rows.iter().any(|r| r.starts_with(label)),
            "{label}: {rows:?}"
        );
    }
    assert_eq!(rows[0], "Play");
}

#[test]
fn play_opens_the_colony_and_ready_starts_the_night() {
    let mut h = colony();
    assert!(spoke(&h, "Before night 1"), "{}", all_spoken(&h));
    assert!(
        spoke(&h, "Power 12, metal 24, food 6"),
        "{}",
        all_spoken(&h)
    );
    let rows = rows(&h);
    assert!(
        rows[0].starts_with("Ready for night 1, 2 drifters"),
        "{rows:?}"
    );
    assert!(rows
        .iter()
        .any(|r| r.starts_with("Build a solar array, 8 metal")));
    assert!(rows.iter().any(|r| r.starts_with("Place a wall, 3 metal")));
    assert!(rows
        .iter()
        .any(|r| r.starts_with("Make a missile, 4 metal and 6 power")));
    assert!(
        !rows.iter().any(|r| r.starts_with("Repair the dome")),
        "the dome is whole: {rows:?}"
    );
    h.clear();
    choose(&mut h, "Ready for night 1");
    assert_eq!(h.app.top_name(), NIGHT_NAME);
    settle(&mut h);
    assert!(spoke(&h, "Night 1. 2 drifters."), "{}", all_spoken(&h));
    assert!(spoke(&h, "Your turn, 2 actions."), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/raider/arrive"));
}

#[test]
fn the_colony_builds_places_and_refuses() {
    let mut h = colony();
    h.clear();
    choose(&mut h, "Build a solar array");
    assert_eq!(
        h.app.top_name(),
        SLOTS_NAME,
        "three slots are free: {}",
        all_spoken(&h)
    );
    assert_eq!(rows(&h)[0], "The left slot");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), COLONY_NAME);
    assert!(
        spoke(&h, "Solar array built under the left. Metal 16."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    choose(&mut h, "Place a wall");
    assert_eq!(h.app.top_name(), PLACER_NAME);
    assert!(spoke(&h, "Centre, row 4, open."), "{}", all_spoken(&h));
    h.press(Key::Up);
    assert!(spoke(&h, "Centre, row 5, open."), "{}", all_spoken(&h));
    h.press(Key::Left);
    assert!(spoke(&h, "Left, row 5, open."), "{}", all_spoken(&h));
    h.press(Key::Up);
    assert!(spoke(&h, "The grid ends there."), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), COLONY_NAME);
    assert!(
        spoke(&h, "Wall placed at left, row 5. Metal 13."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    choose(&mut h, "Place a mine");
    h.press(Key::Up);
    h.press(Key::Left);
    h.press(Key::Return);
    assert!(spoke(&h, "That sector is taken."), "{}", all_spoken(&h));
    assert_eq!(h.app.top_name(), PLACER_NAME);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), COLONY_NAME);
    assert!(spoke(&h, "Nothing placed."), "{}", all_spoken(&h));
    h.clear();
    choose(&mut h, "The grid");
    assert!(spoke(&h, "a wall at left, row 5"), "{}", all_spoken(&h));
    h.clear();
    choose(&mut h, "Build a workshop");
    assert_eq!(
        h.app.top_name(),
        SLOTS_NAME,
        "two slots are free: {}",
        all_spoken(&h)
    );
    assert_eq!(rows(&h)[0], "The centre slot");
    h.press(Key::Return);
    assert!(
        spoke(&h, "Workshop built under the centre. Metal 3."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    choose(&mut h, "Build a scanner");
    assert!(
        spoke(&h, "Too dear. It needs 8 metal and 2 power."),
        "{}",
        all_spoken(&h)
    );
    assert_eq!(h.app.top_name(), COLONY_NAME);
    h.clear();
    choose(&mut h, "Make a missile");
    assert!(
        spoke(&h, "Missile made, 1 of 3. Power 9, metal 1."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    choose(&mut h, "The colony");
    assert!(
        spoke(
            &h,
            "Solar array under the left; workshop under the centre; the right slot open."
        ),
        "{}",
        all_spoken(&h)
    );
}

#[test]
fn scan_status_and_the_route_finder_speak() {
    let mut h = night();
    h.clear();
    h.press(Key::Down);
    assert!(spoke(&h, "Drifter, "), "{}", all_spoken(&h));
    assert!(spoke(&h, "row 6, 1 hit."), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/raider/drifter"));
    let first = h.spoken().last().cloned().unwrap();
    h.clear();
    h.press(Key::Down);
    let second = h.spoken().last().cloned().unwrap();
    assert_ne!(first, second, "two drifters to scan");
    h.press(Key::Down);
    assert_eq!(h.spoken().last().cloned().unwrap(), first, "the scan wraps");
    h.clear();
    h.press(Key::S);
    assert!(
        spoke(
            &h,
            "Gun centre. Dome 100, shield 20. 2 actions left. 2 raiders."
        ),
        "{}",
        all_spoken(&h)
    );
    h.press(Key::S);
    assert!(
        spoke(&h, "Night 1, turn 1. Missiles 0."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    h.press(Key::H);
    assert!(spoke(&h, "Drifter heads to "), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::R);
    assert!(spoke(&h, "Drifter heads to "), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::Tab);
    assert!(spoke(&h, "No missiles."), "{}", all_spoken(&h));
}

#[test]
fn the_gun_slides_fires_and_the_turn_ends() {
    let mut h = night();
    h.clear();
    h.press(Key::Left);
    assert!(spoke(&h, "Left"), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/gun/rail"));
    h.clear();
    h.press(Key::Left);
    // The second action ends the turn: the raiders move.
    settle(&mut h);
    assert!(spoke(&h, "Your turn."), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/raider/drifter"));
    h.clear();
    h.press(Key::Left);
    assert!(
        spoke(&h, "The rail ends at the far left."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    h.press(Key::Return);
    settle(&mut h);
    assert!(
        spoke(&h, "Turn over. Shield up 10, to 30."),
        "{}",
        all_spoken(&h)
    );
}

#[test]
fn keys_wait_while_the_raiders_move() {
    let mut h = night();
    h.clear();
    h.press(Key::Return);
    // Beats are queued: the first plays on the next frame.
    h.frame(0.01);
    h.press(Key::Space);
    assert!(spoke(&h, "The raiders are moving."), "{}", all_spoken(&h));
    settle(&mut h);
}

#[test]
fn a_night_watched_through_opens_the_next_colony_and_saves() {
    let mut h = night();
    watch_night(&mut h);
    assert_eq!(h.app.top_name(), COLONY_NAME, "{}", all_spoken(&h));
    assert!(spoke(&h, "Night 1 held."), "{}", all_spoken(&h));
    assert!(spoke(&h, "Before night 2."), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/music/held"));
    // Back to the title through the pause menu: the colony is saved.
    h.press(Key::Escape);
    choose(&mut h, "Back to the title");
    assert_eq!(h.app.top_name(), TITLE_NAME);
    let rows = rows(&h);
    assert!(rows[0].starts_with("Continue, night 2"), "{rows:?}");
    assert_eq!(rows[1], "New colony");
    assert!(
        rows.iter().any(|r| r == "Abandon the saved colony"),
        "{rows:?}"
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), COLONY_NAME);
    assert!(spoke(&h, "Before night 2."), "{}", all_spoken(&h));
}

#[test]
fn a_whole_colony_reaches_the_results() {
    let mut h = night();
    for _ in 0..12 {
        watch_night(&mut h);
        if h.app.top_name() != COLONY_NAME {
            break;
        }
        h.clear();
        choose(&mut h, "Ready for night");
    }
    assert_eq!(h.app.top_name(), "results", "{}", all_spoken(&h));
    let said = all_spoken(&h);
    assert!(
        said.contains("Relief has landed.") || said.contains("The dome fell on night"),
        "{said}"
    );
    assert!(said.contains("Your first colony."), "{said}");
    let results = rows(&h);
    assert!(
        results
            .iter()
            .any(|r| r == "Play again" || r == "Try again"),
        "{results:?}"
    );
    choose(&mut h, "Back to the title");
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(rows(&h)[0], "Play", "the finished colony is not saved");
}

#[test]
fn the_tutorial_runs_through() {
    let mut h = Headless::new(data(), title_with_seed(SEED));
    assert!(rows(&h)[0].starts_with("Play"));
    select(&mut h, "Tutorial");
    h.press(Key::Return);
    h.frame(DT);
    assert!(
        spoke(&h, "Learn the gun on a quiet night first"),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    h.press(Key::Down);
    h.frame(DT);
    assert!(spoke(&h, "Each raider is named"), "{}", all_spoken(&h));
    // Slide toward a drifter's column, over the arrivals still playing.
    let run = Run::new(tutorial::SEED);
    let target = run.forecast[0].col as i64;
    h.clear();
    for _ in 0..4 {
        if spoke(&h, "Under it.") {
            break;
        }
        h.press(if target < 2 { Key::Left } else { Key::Right });
        h.frame(DT);
    }
    assert!(spoke(&h, "Under it."), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::Space);
    for _ in 0..100 {
        if spoke(&h, "Down. Its wreck leaves salvage") {
            break;
        }
        h.frame(DT);
    }
    assert!(
        spoke(&h, "Down. Its wreck leaves salvage"),
        "{}",
        all_spoken(&h)
    );
    // That was the second action: the raiders move on their own first.
    for _ in 0..200 {
        if spoke(&h, "Your turn.") {
            break;
        }
        h.frame(DT);
    }
    assert!(spoke(&h, "Your turn."), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::Return);
    for _ in 0..200 {
        if spoke(&h, "Your turn again.") {
            break;
        }
        h.frame(DT);
    }
    assert!(
        spoke(&h, "The raiders have moved. Your turn again."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    h.press(Key::S);
    h.frame(DT);
    assert!(
        spoke(&h, "That is the watch at a glance."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    h.press(Key::H);
    h.frame(DT);
    assert!(spoke(&h, "The route finder."), "{}", all_spoken(&h));
    assert!(spoke(&h, "Good luck on the watch."), "{}", all_spoken(&h));
    assert_eq!(h.app.top_name(), TITLE_NAME, "{}", all_spoken(&h));
}

#[test]
fn the_settings_have_the_games_rows() {
    let mut h = start();
    choose(&mut h, "Settings");
    choose(&mut h, "Dustfall");
    let rows = rows(&h);
    for label in [
        "Spoken call-outs: on",
        "Confirm end of turn: off",
        "Scan order: nearest first",
    ] {
        assert!(
            rows.iter().any(|r| r.starts_with(label)),
            "{label}: {rows:?}"
        );
    }
}

#[test]
fn every_sound_check_row_plays_something() {
    let mut h = start();
    choose(&mut h, "Sound check");
    let rows = rows(&h);
    assert!(rows.len() > 20, "{rows:?}");
    h.press(Key::Home);
    for row in rows.iter().take(rows.len() - 1) {
        h.clear();
        h.press(Key::Return);
        assert!(!h.played().is_empty(), "{row} played nothing");
        h.press(Key::Down);
    }
    assert_eq!(rows.last().map(String::as_str), Some("Back"));
}

#[test]
fn records_read_before_and_after_a_colony() {
    let mut h = start();
    choose(&mut h, "Records");
    assert!(
        spoke(&h, "No colony finished yet. No colony started yet."),
        "{}",
        all_spoken(&h)
    );
}

// --- challenges ---------------------------------------------------------------

/// The challenges menu, open from the title.
fn challenges() -> Headless<Parlor> {
    let mut h = start();
    h.clear();
    choose(&mut h, "Challenges");
    assert_eq!(h.app.top_name(), CHALLENGES_NAME, "{}", all_spoken(&h));
    h
}

#[test]
fn the_challenges_menu_names_the_three_skies() {
    let h = challenges();
    let rows = rows(&h);
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert!(
        rows[0].starts_with("Daily, Tonight's sky: one night, the raiders of night"),
        "{rows:?}"
    );
    assert!(rows[0].ends_with("Not played yet"), "{rows:?}");
    assert!(
        rows[1].starts_with("Weekly, Siege week: five nights, 4 to 8"),
        "{rows:?}"
    );
    assert!(
        rows[2].starts_with("Monthly, The long watch: a whole colony, twelve nights to Relief"),
        "{rows:?}"
    );
    assert_eq!(rows[3], "Back");
}

#[test]
fn tonights_sky_opens_on_a_built_colony_and_scores_its_board() {
    let mut h = challenges();
    h.clear();
    choose(&mut h, "Daily");
    assert_eq!(h.app.top_name(), "pages");
    let said = all_spoken(&h);
    assert!(
        said.contains("Tonight's sky: One night against the raiders of night"),
        "{said}"
    );
    assert!(!said.contains("not played yet"), "{said}");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), NIGHT_NAME, "{}", all_spoken(&h));
    // The colony came built: works on the grid, and two missiles in the rack.
    let lines = h.app.top_lines();
    let second = &lines[1];
    assert!(second.contains("missiles 2"), "{second}");
    settle(&mut h);
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    let said = all_spoken(&h);
    assert!(said.contains("Missiles 2."), "{said}");
    // The night to its end: the results lead back to the challenges.
    watch_night(&mut h);
    assert_eq!(h.app.top_name(), "results", "{}", all_spoken(&h));
    let said = all_spoken(&h);
    assert!(
        said.contains("Tonight's sky held.") || said.contains("The dome fell on night"),
        "{said}"
    );
    assert!(
        said.contains("Your first result on this challenge."),
        "{said}"
    );
    let results = rows(&h);
    assert!(
        results
            .iter()
            .any(|r| r == "Play this daily challenge again"),
        "{results:?}"
    );
    choose(&mut h, "Back to the challenges");
    assert_eq!(h.app.top_name(), CHALLENGES_NAME);
    assert!(rows(&h)[0].contains("Done, best"), "{:?}", rows(&h));
    // Nothing of it was saved as a colony.
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(rows(&h)[0], "Play", "{:?}", rows(&h));
}

#[test]
fn siege_week_opens_on_a_stocked_colony_and_leaving_does_not_count() {
    let mut h = challenges();
    choose(&mut h, "Weekly");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), COLONY_NAME, "{}", all_spoken(&h));
    let said = all_spoken(&h);
    assert!(said.contains("Before night 4."), "{said}");
    assert!(said.contains("Power 18, metal 39, food 6"), "{said}");
    // Escape pauses; leaving goes back to the challenges, unscored.
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    choose(&mut h, "Leave the challenge");
    assert_eq!(h.app.top_name(), CHALLENGES_NAME, "{}", all_spoken(&h));
    assert!(rows(&h)[1].ends_with("Not played yet"), "{:?}", rows(&h));
    h.press(Key::Escape);
    assert_eq!(rows(&h)[0], "Play", "{:?}", rows(&h));
}

#[test]
fn the_long_watch_is_a_whole_colony_from_night_one() {
    let mut h = challenges();
    choose(&mut h, "Monthly");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), COLONY_NAME, "{}", all_spoken(&h));
    assert!(spoke(&h, "Before night 1."), "{}", all_spoken(&h));
    assert!(
        spoke(&h, "Power 12, metal 24, food 6"),
        "{}",
        all_spoken(&h)
    );
}

#[test]
fn leaving_the_tutorial_early_keeps_the_first_visit() {
    let mut h = Headless::new(data(), title_with_seed(SEED));
    assert!(spoke(&h, "New here?"), "{}", all_spoken(&h));
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "lesson");
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    h.press(Key::Escape);
    h.clear();
    h.app.ctx.push(title_with_seed(SEED));
    h.frame(0.0);
    assert!(spoke(&h, "New here?"), "{}", all_spoken(&h));
}
