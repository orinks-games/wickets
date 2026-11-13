//! The menu standard, checked over every game in the launcher.
//!
//! Each test opens a game's title the way a player does, on top of the
//! lobby, the category list and the game list, and reads the rows with
//! `top_lines()`. Every game is on the shared menus from
//! `parlor_core::menus`, so every test runs over every registered game.

use hearken::{Headless, Key};
use parlor_core::menus::{labels, rows_of, NO_KEEP_PLAYING, QUIT_QUESTION, YES_QUIT};
use parlor_core::{Challenge, Challenges, Date, GameInfo, Parlor, ParlorData, Period};
use wickets::{CategoryList, ChallengeList, GameList, Lobby};

/// Saves that open every mode, for `rows_do_not_move`. A game with locked
/// modes adds an arm here when it joins the launcher.
fn unlock_everything(id: &str, h: &mut Headless<Parlor>) {
    if id == "sonar-dive" {
        let saved = sonar_dive::save::Saved {
            deepest: 5,
            expeditions: 1,
            ..Default::default()
        };
        saved.store(&mut h.app.ctx);
    }
    if id == "tightrope" {
        parlor_core::Records::record(
            &mut h.app.ctx,
            "tightrope",
            tightrope::progress::FURTHEST,
            10,
        );
        parlor_core::Records::record(&mut h.app.ctx, "tightrope", "training", 1);
    }
    if id == "lantern-lake" {
        let journal = lantern_lake::journal::Journal {
            nights: 3,
            outings: 4,
            pond_done: true,
            ..Default::default()
        };
        journal.store(&mut h.app.ctx);
    }
    if id == "tide-pools" {
        let mut tank = tide_pools::Aquarium::default();
        tank.add(&tide_pools::rules::Species::ALL, "at Sandy Cove");
        tank.outings = 1;
        tank.store(&mut h.app.ctx);
    }
    if id == "rooftop-garden" {
        let mut garden = rooftop_garden::rules::Garden::home(parlor_core::Rng::seeded(1));
        garden.harvests = 9;
        parlor_core::Records::store(
            &mut h.app.ctx,
            rooftop_garden::screens::GARDEN_KEY,
            &Some(garden),
        );
    }
    if id == "harvest-feast" {
        let progress = harvest_feast::Progress {
            stars: vec![3; 8],
            ..Default::default()
        };
        progress.store(&mut h.app.ctx);
    }
    if id == "winter-wonderland" {
        let progress = winter_wonderland::progress::Progress {
            shift: 8,
            night: 5,
            story: true,
            all_gifts: true,
            all_nights: true,
            stack: 5,
            all_stacks: true,
            ..Default::default()
        };
        progress.store(&mut h.app.ctx);
    }
    if id == "basket-run" {
        let progress = basket_run::progress::Progress {
            cleared: 8,
            tutorial_seen: true,
            ..Default::default()
        };
        progress.save(&mut h.app.ctx);
    }
    if id == "neon-rail" {
        let profile = neon_rail::profile::Profile {
            tutorial_complete: true,
            runs_completed: 1,
            extractions: 1,
            story_stage: 12,
            ..Default::default()
        };
        profile.save(&mut h.app.ctx);
    }
    if id == "haunted-hallways" {
        let progress = haunted_hallways::Progress {
            cleared: 5,
            escapes: 1,
            ..Default::default()
        };
        progress.store(&mut h.app.ctx);
    }
    if id == "liberty-lights" {
        let progress = liberty_lights::Progress {
            next_level: 20,
            cleared: (1..=20).collect(),
            ..Default::default()
        };
        progress.store(&mut h.app.ctx);
    }
    if id == "soft-landing" {
        let progress = soft_landing::Progress {
            cleared: 3,
            ..Default::default()
        };
        progress.store(&mut h.app.ctx);
    }
    if id == "safecracker" {
        let progress = safecracker::Progress {
            cracked: 10,
            opened: 10,
            ..Default::default()
        };
        progress.store(&mut h.app.ctx);
    }
}

const TODAY: Date = Date::new(2026, 10, 16);

fn data() -> ParlorData {
    ParlorData {
        today: Some(TODAY),
        ..Default::default()
    }
}

/// The launcher with the game list for `g` open, and nothing spoken yet.
fn game_list(g: &GameInfo) -> Headless<Parlor> {
    let mut h = Headless::new(data(), Lobby::new());
    h.app.ctx.push(CategoryList::new());
    h.app.ctx.push(GameList::new(g.category));
    h.frame(0.0);
    assert_eq!((h.app.top_name(), h.app.depth()), ("games", 3));
    h.clear();
    h
}

/// Opens `g`'s title on the game list.
fn open(h: &mut Headless<Parlor>, g: &GameInfo) {
    (g.open)(&mut h.app.ctx);
    h.frame(0.0);
}

/// A fresh save with `g`'s title open.
fn title(g: &GameInfo) -> Headless<Parlor> {
    let mut h = game_list(g);
    open(&mut h, g);
    h
}

/// The rows of the menu on top, without the title line or cursor marks.
fn rows(h: &Headless<Parlor>) -> Vec<String> {
    rows_of(&h.app.top_lines())
}

fn cursor(h: &Headless<Parlor>) -> usize {
    h.app
        .top_lines()
        .iter()
        .skip(1)
        .position(|l| l.starts_with("> "))
        .unwrap_or(0)
}

/// Moves the cursor to the first row starting with `label`.
fn goto(h: &mut Headless<Parlor>, label: &str) {
    let at = rows(h)
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row starts with {label:?}: {:?}", rows(h)));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
    assert_eq!(cursor(h), at);
}

/// Presses Enter through any pages on top, at most `n` of them.
fn read_pages(h: &mut Headless<Parlor>) {
    for _ in 0..30 {
        if h.app.top_name() != "pages" {
            return;
        }
        h.press(Key::Return);
    }
}

fn first_word(row: &str) -> &str {
    row.split([' ', ',', ':', '.']).next().unwrap_or("")
}

/// Checks a title's rows against the standard: Play first, the required
/// rows once each in order, and none of the retired labels.
fn standard_title(id: &str, rows: &[String]) {
    assert!(rows[0].starts_with(labels::PLAY), "{id}: {rows:?}");
    let required = [
        labels::CHALLENGES,
        labels::TUTORIAL,
        labels::HELP,
        labels::SOUND_CHECK,
        labels::RECORDS,
        labels::SETTINGS,
        labels::BACK_TO_PARLOR,
    ];
    let mut last = 0;
    for label in required {
        let at: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                if label == labels::CHALLENGES {
                    r.starts_with(label)
                } else {
                    r.as_str() == label
                }
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(at.len(), 1, "{id}: {label:?} once in {rows:?}");
        assert!(at[0] >= last, "{id}: {label:?} out of order in {rows:?}");
        last = at[0];
    }
    assert_eq!(
        rows.last().map(String::as_str),
        Some(labels::BACK_TO_PARLOR)
    );
    for retired in [
        "How to play",
        "Instructions",
        "Best scores",
        "Best times",
        "Sound guide",
    ] {
        assert!(
            !rows.iter().any(|r| r.starts_with(retired)),
            "{id}: {retired:?} in {rows:?}"
        );
    }
    // A game's own options go in Settings, not on the title.
    for row in rows {
        assert!(
            !row.ends_with(": on") && !row.ends_with(": off") && !row.contains("call-outs"),
            "{id}: {row:?} belongs in Settings, in {rows:?}"
        );
    }
}

// Every game.

#[test]
fn every_title_opens_under_its_title_name() {
    for g in wickets::games() {
        let h = title(&g);
        assert_eq!(h.app.top_name(), g.title_name(), "{}", g.id);
        assert_eq!(h.app.depth(), 4, "{}", g.id);
    }
}

#[test]
fn every_title_ends_with_back_to_the_parlor() {
    for g in wickets::games() {
        let mut h = title(&g);
        let rows = rows(&h);
        assert_eq!(
            rows.last().map(String::as_str),
            Some(labels::BACK_TO_PARLOR),
            "{}: {rows:?}",
            g.id
        );
        h.press(Key::End);
        h.press(Key::Return);
        assert_eq!((h.app.top_name(), h.app.depth()), ("games", 3), "{}", g.id);
    }
}

#[test]
fn escape_on_every_title_returns_to_the_game_list() {
    for g in wickets::games() {
        let mut h = title(&g);
        h.press(Key::Escape);
        assert_eq!((h.app.top_name(), h.app.depth()), ("games", 3), "{}", g.id);
        let music = h.app.ctx.audio.music().unwrap_or_default().to_string();
        assert!(
            wickets::music::LOBBY.contains(&music.as_str()),
            "{}: {music:?}",
            g.id
        );
    }
}

#[test]
fn every_row_has_f1_help() {
    for g in wickets::games() {
        let mut h = title(&g);
        let n = rows(&h).len();
        h.press(Key::Home);
        for i in 0..n {
            h.clear();
            h.press(Key::F1);
            let said = h.spoken().join(" ");
            assert!(
                !said.trim().is_empty() && said != hearken::menu::DEFAULT_INTRO_HELP,
                "{}: row {} {:?} has no help",
                g.id,
                i + 1,
                rows(&h)[i]
            );
            h.press(Key::Down);
        }
    }
}

// The shared menus.

#[test]
fn standard_title_order() {
    for g in wickets::games() {
        let h = title(&g);
        standard_title(g.id, &rows(&h));
    }
}

#[test]
fn rows_do_not_move() {
    for g in wickets::games() {
        let fresh = rows(&title(&g));
        let mut h = game_list(&g);
        unlock_everything(g.id, &mut h);
        open(&mut h, &g);
        let unlocked = rows(&h);
        assert_eq!(
            fresh.len(),
            unlocked.len(),
            "{}: {fresh:?} {unlocked:?}",
            g.id
        );
        for (a, b) in fresh.iter().zip(&unlocked) {
            assert_eq!(first_word(a), first_word(b), "{}: {a:?} became {b:?}", g.id);
        }
        standard_title(g.id, &unlocked);
    }
}

#[test]
fn first_visit_cursor_on_tutorial() {
    for g in wickets::games() {
        let mut h = title(&g);
        let spoken = h.spoken();
        assert!(spoken[0].contains("New here?"), "{}: {spoken:?}", g.id);
        assert!(
            spoken[1].starts_with(labels::TUTORIAL) || spoken[1].starts_with("Practice"),
            "{}: {spoken:?}",
            g.id
        );
        goto(&mut h, labels::TUTORIAL);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "lesson", "{}", g.id);
        // Leaving the lesson early keeps the tutorial first.
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), g.title_name(), "{}", g.id);
        h.press(Key::Escape);
        h.clear();
        open(&mut h, &g);
        assert!(h.spoken()[0].contains("New here?"), "{}", g.id);
        // A finished tutorial does not.
        parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, g.id);
        h.press(Key::Escape);
        h.clear();
        open(&mut h, &g);
        let spoken = h.spoken();
        assert!(spoken[1].starts_with(labels::PLAY), "{}: {spoken:?}", g.id);
    }
}

#[test]
fn p_jumps_to_play() {
    for g in wickets::games() {
        let mut h = title(&g);
        h.clear();
        h.press(Key::P);
        assert_eq!(cursor(&h), 0, "{}", g.id);
        assert!(h.spoken()[0].starts_with(labels::PLAY), "{}", g.id);
    }
}

#[test]
fn challenges_row_label() {
    for g in wickets::games() {
        let mut h = title(&g);
        let row = |h: &Headless<Parlor>| {
            rows(h)
                .into_iter()
                .find(|r| r.starts_with(labels::CHALLENGES))
                .unwrap_or_else(|| panic!("{}: no challenges row", g.id))
        };
        let expected = labels::challenges_row_label(&mut h.app.ctx, g.id);
        assert!(expected.ends_with("not played yet"), "{}", g.id);
        assert_eq!(row(&h), expected, "{}", g.id);

        h.press(Key::Escape);
        let daily = Challenge::current(&h.app.ctx, g.id, Period::Daily);
        Challenges::submit(&mut h.app.ctx, &daily, 1);
        open(&mut h, &g);
        let expected = labels::challenges_row_label(&mut h.app.ctx, g.id);
        assert!(expected.ends_with("daily done"), "{}", g.id);
        assert_eq!(row(&h), expected, "{}", g.id);
    }
}

#[test]
fn spoken_call_outs_row_flips_and_says_so() {
    let mut seen = 0;
    for g in wickets::games() {
        let mut h = title(&g);
        goto(&mut h, labels::SETTINGS);
        h.press(Key::Return);
        h.press(Key::Return); // the game's own category
        let Some(at) = rows(&h)
            .iter()
            .position(|r| r.starts_with("Spoken call-outs: "))
        else {
            continue;
        };
        seen += 1;
        goto(&mut h, "Spoken call-outs: ");
        let before = rows(&h)[at].clone();
        h.clear();
        h.press(Key::Return);
        let after = rows(&h)[at].clone();
        assert!(
            after.starts_with("Spoken call-outs: "),
            "{}: {after:?}",
            g.id
        );
        assert_ne!(before, after, "{}", g.id);
        let state = after.trim_start_matches("Spoken call-outs: ");
        let said = h.spoken().join(" ");
        assert!(
            said.contains(&format!("Spoken call-outs: {state}")),
            "{}: {said:?}",
            g.id
        );
    }
    assert!(seen >= 3, "only {seen} games have the toggle");
}

#[test]
fn a_games_settings_rows_leave_with_the_game() {
    for g in wickets::games() {
        let mut h = title(&g);
        goto(&mut h, labels::SETTINGS);
        h.press(Key::Return);
        let inside = rows(&h).len();
        h.press(Key::Escape);
        goto(&mut h, labels::BACK_TO_PARLOR);
        h.press(Key::Return);
        h.app
            .ctx
            .push(hearken::screens::SettingsScreen::<Parlor>::new());
        h.frame(0.0);
        // The launcher's Settings adds only Game preferences.
        let shared = rows(&h)
            .iter()
            .filter(|r| !r.starts_with(parlor_core::preferences::LABEL))
            .count();
        assert!(shared <= inside, "{}", g.id);
        assert!(
            !rows(&h)
                .iter()
                .any(|r| r.contains("call-outs") || r == g.name),
            "{}: {:?}",
            g.id,
            rows(&h)
        );
    }
}

/// The title's challenges menu, open.
fn challenges(g: &GameInfo) -> Headless<Parlor> {
    let mut h = title(g);
    goto(&mut h, labels::CHALLENGES);
    h.press(Key::Return);
    assert_eq!(
        h.app.top_name(),
        format!("{}_challenges", g.title_name()),
        "{}",
        g.id
    );
    h
}

#[test]
fn challenges_menu_rows() {
    for g in wickets::games() {
        let mut h = challenges(&g);
        let rows = rows(&h);
        assert_eq!(rows.len(), 4, "{}: {rows:?}", g.id);
        for (row, period) in rows.iter().zip(Period::ALL) {
            assert!(row.starts_with(period.label()), "{}: {row:?}", g.id);
        }
        assert_eq!(rows[3], labels::BACK, "{}", g.id);
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), g.title_name(), "{}", g.id);
        let mut h = challenges(&g);
        h.press(Key::End);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), g.title_name(), "{}", g.id);
    }
}

#[test]
fn briefing_escape_goes_back_without_starting() {
    for g in wickets::games() {
        for period in Period::ALL {
            let mut h = challenges(&g);
            let menu = h.app.top_name();
            goto(&mut h, period.label());
            let depth = h.app.depth();
            h.press(Key::Return);
            assert_eq!(h.app.top_name(), "pages", "{} {period:?}", g.id);
            h.press(Key::Escape);
            assert_eq!(
                (h.app.top_name(), h.app.depth()),
                (menu, depth),
                "{} {period:?}",
                g.id
            );
        }
    }
}

const PAUSE_ROWS: [&str; 5] = [
    labels::RESUME,
    labels::SETTINGS,
    labels::HELP,
    labels::BACK_TO_TITLE,
    labels::BACK_TO_LOBBY,
];

/// The main mode, started from the title and paused.
fn paused(g: &GameInfo) -> Headless<Parlor> {
    let mut h = title(g);
    h.press(Key::Home);
    h.press(Key::Return);
    // A game with a briefing before play starts on Enter.
    if h.app.top_name() == parlor_core::briefing::NAME {
        h.press(Key::Return);
    }
    read_pages(&mut h);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause", "{}", g.id);
    h
}

#[test]
fn pause_rows_and_leave_label() {
    for g in wickets::games() {
        let mut h = paused(&g);
        assert_eq!(rows(&h), PAUSE_ROWS, "{}", g.id);
        goto(&mut h, labels::BACK_TO_TITLE);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), g.title_name(), "{}", g.id);

        // A challenge opened from the lobby's board leaves back to it.
        let Some(_) = g.challenge else { continue };
        let mut h = Headless::new(data(), Lobby::new());
        h.app.ctx.push(ChallengeList::new(Period::Daily));
        h.frame(0.0);
        goto(&mut h, g.name);
        h.press(Key::Return);
        read_pages(&mut h);
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "pause", "{}", g.id);
        let rows = rows(&h);
        assert_eq!(rows[3], labels::LEAVE_CHALLENGE, "{}: {rows:?}", g.id);
        goto(&mut h, labels::LEAVE_CHALLENGE);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "challenge list", "{}", g.id);
    }
}

#[test]
fn back_to_the_lobby_leaves_the_game_without_quitting() {
    for g in wickets::games() {
        let mut h = paused(&g);
        goto(&mut h, labels::BACK_TO_LOBBY);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "lobby", "{}", g.id);
        assert!(h.app.ctx.running, "{}", g.id);
    }
}

#[test]
fn the_quit_row_asks_before_quitting() {
    let mut h = Headless::new(data(), Lobby::new());
    h.frame(0.0);
    h.press(Key::End);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "confirm");
    assert_eq!(h.spoken()[0], QUIT_QUESTION);
    assert!(h.app.ctx.running);
}

#[test]
fn escape_in_the_lobby_asks_before_quitting() {
    let mut h = Headless::new(data(), Lobby::new());
    h.frame(0.0);
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "confirm");
    assert_eq!(h.spoken()[0], QUIT_QUESTION);
    assert_eq!(rows(&h), [YES_QUIT, NO_KEEP_PLAYING]);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "lobby");
    assert!(h.app.ctx.running);
    h.press(Key::Escape);
    goto(&mut h, YES_QUIT);
    h.press(Key::Return);
    assert!(!h.app.ctx.running);
}
