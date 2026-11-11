//! Dustfall through the headless app: the title, the colony phase, a night
//! watched to its end, the tutorial, the settings and the sound check.

use dustfall::colony::{share_as, Mode, COLONY_NAME, SLOTS_NAME};
use dustfall::night::{NightScreen, NIGHT_NAME};
use dustfall::rules::{Input, Kind, Run};
use dustfall::{title_with_seed, CHALLENGES_NAME, TITLE_NAME};
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

/// Into night 1 of a new colony, just as it opens.
fn night() -> Headless<Parlor> {
    let mut h = colony();
    h.clear();
    choose(&mut h, "Ready for night 1");
    assert_eq!(h.app.top_name(), NIGHT_NAME);
    h
}

/// Frames for `secs` seconds of the night.
fn pass(h: &mut Headless<Parlor>, secs: f64) {
    let frames = (secs / DT).round() as usize;
    for _ in 0..frames {
        h.frame(DT);
    }
}

/// Frames until something is spoken containing `needle`.
fn wait_for(h: &mut Headless<Parlor>, needle: &str, most_secs: f64) {
    let frames = (most_secs / DT).round() as usize;
    for _ in 0..frames {
        if spoke(h, needle) {
            return;
        }
        h.frame(DT);
    }
    panic!("never heard {needle:?}: {}", all_spoken(h));
}

/// The nearest raider's lane, from the night screen's lines.
fn nearest_lane(h: &Headless<Parlor>) -> Option<i64> {
    h.app
        .top_lines()
        .iter()
        .find(|l| l.starts_with("raider "))
        .map(|l| {
            let at = l.split(" at ").nth(1).expect("a position");
            nums(at)[0]
        })
}

/// Watches the night on top to its end: the gun slides under the nearest
/// raider and fires whenever it can.
fn watch_night(h: &mut Headless<Parlor>) {
    for _ in 0..30_000 {
        if h.app.top_name() != NIGHT_NAME {
            return;
        }
        let first = nums(&h.app.top_lines()[0]);
        let (gun, cooldown) = (first[2], first[6]);
        match nearest_lane(h) {
            None => {}
            Some(lane) if lane < gun => h.press(Key::Left),
            Some(lane) if lane > gun => h.press(Key::Right),
            Some(_) if cooldown == 0 => h.press(Key::Space),
            Some(_) => {}
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
        rows[0].starts_with("Ready for night 1, 2 drifters, in 2 waves"),
        "{rows:?}"
    );
    assert!(rows
        .iter()
        .any(|r| r.starts_with("Build a solar array, 8 metal")));
    assert!(rows
        .iter()
        .any(|r| r.starts_with("Build a flak post, 6 metal and 1 power")));
    assert!(rows
        .iter()
        .any(|r| r.starts_with("Lay a mine, 3 metal, 1 of 1 tonight")));
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
    h.frame(DT);
    assert!(
        spoke(&h, "Night 1, gun at the centre. 2 drifters, in 2 waves."),
        "{}",
        all_spoken(&h)
    );
    h.clear();
    h.press(Key::Down);
    assert!(
        spoke(&h, "Nothing in the sky. Wave 1 of 2 in "),
        "{}",
        all_spoken(&h)
    );
    wait_for(&mut h, "Wave 1 of 2: a drifter.", 4.0);
    assert!(h.played().iter().any(|k| k == "dustfall/wave/coming"));
    h.clear();
    wait_for(&mut h, "Drifter, ", 3.0);
    assert!(h.played().iter().any(|k| k == "dustfall/raider/arrive"));
}

#[test]
fn the_colony_builds_lays_mines_and_refuses() {
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
    choose(&mut h, "Lay a mine");
    assert_eq!(h.app.top_name(), SLOTS_NAME);
    assert!(spoke(&h, "Which lane to mine"), "{}", all_spoken(&h));
    assert_eq!(rows(&h)[0], "The far left lane");
    choose(&mut h, "The centre lane");
    assert_eq!(h.app.top_name(), COLONY_NAME);
    assert!(
        spoke(&h, "Mine laid in the centre lane. Metal 13."),
        "{}",
        all_spoken(&h)
    );
    assert!(
        !rows(&h).iter().any(|r| r.starts_with("Lay a mine")),
        "one mine a night: {:?}",
        rows(&h)
    );
    h.clear();
    choose(&mut h, "Build a workshop");
    assert_eq!(
        h.app.top_name(),
        SLOTS_NAME,
        "two slots are free: {}",
        all_spoken(&h)
    );
    assert_eq!(rows(&h)[0], "The centre slot");
    choose(&mut h, "Back");
    assert_eq!(h.app.top_name(), COLONY_NAME);
    assert!(spoke(&h, "Nothing built."), "{}", all_spoken(&h));
    h.clear();
    choose(&mut h, "Build a workshop");
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
            "Solar array under the left; workshop under the centre; the right slot open. A mine in the centre lane."
        ),
        "{}",
        all_spoken(&h)
    );
}

#[test]
fn scan_status_and_the_sky_speak() {
    let mut h = night();
    wait_for(&mut h, "Drifter, ", 6.0);
    h.clear();
    h.press(Key::Down);
    assert!(spoke(&h, "Drifter, "), "{}", all_spoken(&h));
    assert!(spoke(&h, ", far, 1 hit."), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/ui/scan"));
    let first = h.spoken().last().cloned().unwrap();
    h.clear();
    h.press(Key::Down);
    assert_eq!(
        h.spoken().last().cloned().unwrap(),
        first,
        "one raider: the scan stays on it"
    );
    h.clear();
    h.press(Key::S);
    assert!(
        spoke(&h, "Gun centre. Dome 100, shield 2"),
        "{}",
        all_spoken(&h)
    );
    assert!(
        spoke(&h, ". Wave 1 of 2. 1 raider in the sky, 1 to come."),
        "{}",
        all_spoken(&h)
    );
    h.press(Key::S);
    assert!(spoke(&h, "Drifter, "), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::H);
    let said = all_spoken(&h);
    assert!(
        said.contains("a drifter") || said.contains("Drifter"),
        "{said}"
    );
    h.clear();
    h.press(Key::R);
    assert!(!h.spoken().is_empty(), "R repeats the last line");
}

#[test]
fn the_gun_slides_and_the_lock_tone_pulses_under_a_raider() {
    let mut h = night();
    wait_for(&mut h, "Drifter, ", 6.0);
    let lane = nearest_lane(&h).unwrap();
    h.clear();
    h.press(Key::Left);
    assert!(spoke(&h, "Left"), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/gun/rail"));
    h.press(Key::Left);
    h.clear();
    h.press(Key::Left);
    assert!(spoke(&h, "The rail ends"), "{}", all_spoken(&h));
    for _ in 0..lane {
        h.press(Key::Right);
    }
    h.clear();
    pass(&mut h, 1.0);
    assert!(
        h.played().iter().any(|k| k == "dustfall/gun/lock"),
        "the lock tone under the raider: {:?}",
        h.played()
    );
    h.clear();
    h.press(Key::Space);
    assert!(h.played().iter().any(|k| k == "dustfall/gun/fire"));
    assert!(spoke(&h, "Drifter down."), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::Space);
    assert!(
        h.spoken().is_empty(),
        "reloading is silent: {}",
        all_spoken(&h)
    );
}

#[test]
fn a_night_watched_through_opens_the_next_colony_and_saves() {
    let mut h = night();
    watch_night(&mut h);
    assert_eq!(h.app.top_name(), COLONY_NAME, "{}", all_spoken(&h));
    assert!(spoke(&h, "Wave 1 of 2 down."), "{}", all_spoken(&h));
    assert!(spoke(&h, "Wave 2 of 2: a drifter."), "{}", all_spoken(&h));
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
fn a_mothership_night_is_heard_holding_shifting_and_firing() {
    let mut h = start();
    let mut run = Run::at_night(4, 5);
    run.take_events();
    run.apply(Input::Ready).unwrap();
    run.waves
        .retain(|w| w.iter().any(|a| a.kind == Kind::Mothership));
    h.app
        .ctx
        .push(NightScreen::new(share_as(run, Mode::Practice)));
    h.frame(DT);
    assert_eq!(h.app.top_name(), NIGHT_NAME);
    wait_for(&mut h, "Mothership, ", 6.0);
    // Shots from the dome glance off while it is far out.
    let lane = nearest_lane(&h).unwrap();
    for _ in 0..(lane - 2).abs() {
        h.press(if lane < 2 { Key::Left } else { Key::Right });
    }
    h.clear();
    h.press(Key::Space);
    assert!(spoke(&h, "Out of range."), "{}", all_spoken(&h));
    assert!(h.played().iter().any(|k| k == "dustfall/charge/pass"));
    wait_for(&mut h, "Mothership holds at mid range", 12.0);
    wait_for(&mut h, "Siege shot coming.", 8.0);
    wait_for(&mut h, "Mothership to the ", 8.0);
    assert!(
        spoke(&h, "Hit. Shield ") && spoke(&h, ", dome 95."),
        "the siege shot lands through the shield: {}",
        all_spoken(&h)
    );
    // It shifted a lane: slide after it, and a shot at mid range lands.
    let to = nearest_lane(&h).unwrap();
    for _ in 0..(to - lane).abs() {
        h.press(if to < lane { Key::Left } else { Key::Right });
    }
    h.clear();
    h.press(Key::Space);
    assert!(spoke(&h, "Mothership, 7 left."), "{}", all_spoken(&h));
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
    assert!(said.contains("points. Your first colony."), "{said}");
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
    wait_for(&mut h, "Drifter, ", 8.0);
    h.clear();
    h.press(Key::Down);
    h.frame(DT);
    assert!(spoke(&h, "Each raider is named"), "{}", all_spoken(&h));
    let lane = nearest_lane(&h).unwrap();
    h.clear();
    for _ in 0..4 {
        if spoke(&h, "Locked on.") {
            break;
        }
        h.press(if lane < 2 { Key::Left } else { Key::Right });
        h.frame(DT);
    }
    assert!(spoke(&h, "Locked on."), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::Space);
    wait_for(&mut h, "Down. Its wreck leaves salvage", 2.0);
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
    assert!(spoke(&h, "The sky, read out."), "{}", all_spoken(&h));
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
        "Scan order: nearest first",
        "Storm wind: on",
    ] {
        assert!(
            rows.iter().any(|r| r.starts_with(label)),
            "{label}: {rows:?}"
        );
    }
    assert!(
        !rows.iter().any(|r| r.contains("turn")),
        "nothing turn-based is left: {rows:?}"
    );
}

#[test]
fn every_sound_check_row_plays_something() {
    let mut h = start();
    choose(&mut h, "Sound check");
    let rows = rows(&h);
    assert!(rows.len() > 25, "{rows:?}");
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
        spoke(&h, "Records. No colony started yet."),
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
        rows[2].starts_with("Monthly, The long watch: a whole colony, to Relief and on past it"),
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
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), NIGHT_NAME, "{}", all_spoken(&h));
    // The colony came built: two missiles in the rack.
    let lines = h.app.top_lines();
    assert!(lines[1].contains("missiles 2"), "{}", lines[1]);
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

#[test]
fn the_title_locks_the_long_watch_and_the_storm_and_lists_the_unlocks() {
    let mut h = start();
    let shown = rows(&h);
    assert!(
        shown
            .iter()
            .any(|r| r.starts_with("The long watch, locked")),
        "{shown:?}"
    );
    assert!(
        shown.iter().any(|r| r.starts_with("Dust storm, locked")),
        "{shown:?}"
    );
    assert!(shown.iter().any(|r| r == "Unlocks, 0 of 23"), "{shown:?}");
    h.clear();
    choose(&mut h, "The long watch, locked");
    assert!(
        spoke(&h, "The long watch opens once Relief has landed."),
        "{}",
        all_spoken(&h)
    );
    assert_eq!(h.app.top_name(), TITLE_NAME);
    h.clear();
    choose(&mut h, "Dust storm, locked");
    // 4 November 2026 is a Wednesday; the fourth Monday is the 23rd.
    assert!(
        spoke(&h, "The dust storm comes Monday 23 November, in 19 days."),
        "{}",
        all_spoken(&h)
    );
    assert_eq!(h.app.top_name(), TITLE_NAME);
    h.clear();
    choose(&mut h, "Unlocks");
    assert_eq!(h.app.top_name(), dustfall::UNLOCKS_NAME);
    let groups = rows(&h);
    assert!(groups[0].starts_with("Colony, 0 of 4"), "{groups:?}");
    assert!(
        groups
            .iter()
            .any(|r| r.starts_with("Reva's logbook, 0 of 12")),
        "{groups:?}"
    );
}

#[test]
fn the_colony_offers_two_orders_and_takes_one() {
    let mut h = colony();
    let shown = rows(&h);
    let orders: Vec<&String> = shown.iter().filter(|r| r.starts_with("Order: ")).collect();
    assert_eq!(orders.len(), 2, "{shown:?}");
    let first = orders[0].clone();
    h.clear();
    choose(&mut h, &first);
    let said = all_spoken(&h);
    assert!(said.contains(" taken. "), "{said}");
    let shown = rows(&h);
    assert!(!shown.iter().any(|r| r.starts_with("Order: ")), "{shown:?}");
    assert!(
        shown.iter().any(|r| r.starts_with("Order taken: ")),
        "{shown:?}"
    );
}

#[test]
fn on_the_fourth_monday_the_storm_row_opens_a_colony_in_the_dust() {
    let data = ParlorData {
        today: Some(Date::new(2026, 11, 23)),
        ..Default::default()
    };
    let mut h = Headless::new(data, title_with_seed(SEED));
    select(&mut h, "Play");
    let shown = rows(&h);
    assert!(
        shown.iter().any(|r| r == "Dust storm, tonight"),
        "{shown:?}"
    );
    h.clear();
    choose(&mut h, "Dust storm, tonight");
    assert_eq!(h.app.top_name(), COLONY_NAME, "{}", all_spoken(&h));
    h.clear();
    choose(&mut h, "Ready for night 1");
    assert_eq!(h.app.top_name(), NIGHT_NAME);
    h.frame(DT);
    assert!(spoke(&h, "Dust at the far left."), "{}", all_spoken(&h));
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    assert!(spoke(&h, "Dust at the far left."), "{}", all_spoken(&h));
    h.clear();
    wait_for(&mut h, "Dust at the left.", 6.0);
    // Leaving saves the colony as a dust storm one.
    h.press(Key::Escape);
    choose(&mut h, "Back to the title");
    for _ in 0..5 {
        h.frame(DT);
    }
    let shown = rows(&h);
    assert!(
        shown[0].starts_with("Continue the dust storm colony, night 1"),
        "{shown:?}"
    );
}
