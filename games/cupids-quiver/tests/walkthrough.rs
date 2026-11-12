//! The game through its screens, as a player meets it.

use crate::common::*;
use cupids_quiver::field::FIELD_NAME;
use cupids_quiver::progress::Progress;
use cupids_quiver::rules::Mode;
use cupids_quiver::{sound, ID, TITLE_NAME};
use hearken::Key;
use parlor_core::menus::rows_of;
use parlor_core::testkit::every_action_answers;
use parlor_core::testkit::press_action;
use parlor_core::ParlorAction;
use parlor_core::Records;

#[test]
fn the_title_follows_the_standard_order() {
    let h = title();
    assert_eq!(
        rows_of(&h.app.top_lines()),
        [
            "Play, the rooftops",
            "Moonlight, locked",
            "Valentine's night, locked",
            "For two, locked",
            "Challenges, daily not played yet",
            "Practice field",
            "Tutorial",
            "Help",
            "Sound check",
            "Records",
            "Love notes, 0 of 24",
            "Keepsake box",
            "Settings",
            "Back to the parlor",
        ]
    );
    assert!(played(&h, sound::TITLE_MUSIC));
}

#[test]
fn a_first_visit_starts_on_the_tutorial() {
    let mut h = base(AUTUMN);
    h.app.ctx.push(cupids_quiver::title());
    h.frame(0.0);
    assert_eq!(
        h.spoken()[1..],
        [
            "Cupid's Quiver. New here? Start with the tutorial.",
            "Tutorial. 7 of 14."
        ]
    );
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "You stand on a rooftop at dusk with a bow, and paper hearts drift across the sky."
    ));
    assert!(spoke(&h, "The Left arrow and the Right arrow turn the bow"));
    press_action(&mut h, ParlorAction::Right);
    h.clear();
    press_action(&mut h, ParlorAction::Up);
    assert!(spoke(&h, "Hold Space to draw the string."));
    shoot(&mut h);
    assert!(spoke(&h, "That is a shot."));
    for _ in 0..30 {
        if h.app.top_name() != "lesson" {
            break;
        }
        h.clear();
        press_action(&mut h, ParlorAction::Hint);
        let hint = h.spoken().join(" ");
        let turn = if hint.contains("hold your aim") {
            Some(0)
        } else {
            hint.find("turn ").and_then(|at| {
                let words: Vec<&str> = hint[at + 5..].split(' ').collect();
                let n = words[0].parse::<i32>().ok()?;
                Some(if words[1].starts_with("left") { -n } else { n })
            })
        };
        if let Some(target) = turn {
            let action = if target < 0 {
                ParlorAction::Left
            } else {
                ParlorAction::Right
            };
            for _ in 0..(target.abs() / 5) {
                press_action(&mut h, action);
            }
            shoot(&mut h);
        } else {
            wait(&mut h, 1.0);
        }
    }
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert!(spoke(&h, "A hit. Tutorial done."));
    // The hit that ends the lesson brings no new heart over the closing line.
    let said = h.spoken();
    let at = said
        .iter()
        .position(|s| s.starts_with("A hit. Tutorial done."))
        .unwrap();
    assert!(!said[at - 1].contains("second"), "{said:?}");
}

#[test]
fn valentine_rows_stay_locked_and_count_down_the_rest_of_the_year() {
    let mut h = title();
    goto(&mut h, "Valentine's night");
    h.clear();
    h.press(Key::F1);
    let said = h.spoken().join(" ");
    assert!(
        said.contains(
            "Opens on Valentine's Day, the 14th of February. Valentine's Day is in 121 days."
        ),
        "{said}"
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert!(spoke(&h, "Valentine's Day is in 121 days."));
    assert!(played(&h, sound::NOPE));
    goto(&mut h, "Moonlight");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert!(spoke(&h, "Clear all six rooftops"));
}

#[test]
fn the_season_brings_roses_and_new_music() {
    let h = title_on(SEASON);
    assert!(played(&h, sound::ROSES_MUSIC));
    let mut h = field_on(Mode::Rooftops, SEASON);
    assert!(spoke(&h, "Roses drift among the hearts, worth 25 more."));
    assert!(played(&h, sound::ROSE_GARDEN));
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    // Valentine's night is still locked in the run-up.
    let h = title_on(SEASON);
    let rows = rows_of(&h.app.top_lines());
    assert_eq!(rows[2], "Valentine's night, locked");
}

#[test]
fn valentines_day_opens_the_night_and_the_game_for_two() {
    let mut h = title_on(DAY);
    assert!(played(&h, sound::VALENTINE_MUSIC));
    let rows = rows_of(&h.app.top_lines());
    assert_eq!(rows[2], "Valentine's night");
    assert_eq!(rows[3], "For two");
    goto(&mut h, "Valentine's night");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(&h, "Valentine's night, round 1 of 4: The bridge."));
    assert!(spoke(&h, "Golden hearts drift tonight, worth 100."));
    assert!(played(&h, sound::NIGHT));

    let mut h = title_on(DAY);
    goto(&mut h, "For two");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(&h, "Player 1's turn."));
    h.clear();
    shoot(&mut h);
    assert!(played(&h, sound::TURN));
    assert!(status_line(&h).starts_with("Player 2's turn"));
}

#[test]
fn a_night_on_the_day_leaves_a_keepsake() {
    let mut h = field_on(Mode::Night, DAY);
    let rounds = play_out(&mut h);
    assert_eq!(rounds, 4, "every round of the night is played");
    assert!(spoke(&h, "Valentine's night done."));
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    let progress = Progress::load(&mut h.app.ctx);
    let k = progress.keepsakes.get(&2027).expect("a keepsake for 2027");
    assert_eq!(k.plays, 1);
    assert!(k.night_best.is_some());
    let pages = cupids_quiver::keepsake_pages(&mut h.app.ctx);
    assert!(
        pages[1].starts_with("2027: two paper hearts stitched together"),
        "{pages:?}"
    );
}

#[test]
fn every_key_answers_in_play() {
    let mut h = field(Mode::Rooftops);
    every_action_answers(&mut h);
    let mut h = field(Mode::Practice);
    every_action_answers(&mut h);
}

#[test]
fn play_opens_the_sky_and_says_where_you_are() {
    let mut h = title();
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(
        &h,
        "The rooftops, round 1 of 6: The bakery roof. Warm air rises from the ovens, and pigeons doze in the gutter. 10 arrows. Hit 4 hearts to go on."
    ));
    assert!(spoke(
        &h,
        "Hold Space past the full draw cue, then let go at the sights cue."
    ));
    assert!(played(&h, sound::ROOFTOP));
    assert!(played(&h, sound::ROUND_START));
}

#[test]
fn turning_ticks_and_says_the_aim() {
    let mut h = field(Mode::Rooftops);
    wait(&mut h, 3.0);
    h.clear();
    h.press(Key::Right);
    assert!(played(&h, sound::TICK));
    h.press(Key::Right);
    assert!(status_line(&h).contains("Aim 10 degrees right"));
    h.clear();
    h.press(Key::Up);
    assert_eq!(h.spoken(), ["Straight ahead."]);
    for _ in 0..20 {
        h.press(Key::Left);
    }
    h.clear();
    h.press(Key::Left);
    assert!(played(&h, sound::EDGE));
    assert!(spoke(&h, "As far as the bow turns, 70 degrees left."));
}

#[test]
fn the_status_reads_in_a_fixed_order() {
    let mut h = field(Mode::Rooftops);
    h.clear();
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        ["Aim straight ahead, 10 arrows left, 0 of 4 hearts."]
    );
    h.clear();
    h.press(Key::S);
    assert_eq!(h.spoken(), ["Round 1 of 6, 0 points, 0 in the sky."]);
}

#[test]
fn a_hearts_hum_starts_when_it_drifts_in() {
    let mut h = field(Mode::Rooftops);
    h.clear();
    wait(&mut h, 3.0);
    assert!(played(&h, sound::HUM));
    assert!(spoke(&h, "heart, from the"), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Tab);
    assert!(spoke(&h, "in the sky."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::H);
    assert!(played(&h, sound::HINT));
    assert!(
        spoke(&h, " heart: turn ") || spoke(&h, "hold your aim"),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn a_full_draw_shoots_and_an_early_release_does_not() {
    let mut h = field(Mode::Rooftops);
    h.clear();
    hold(&mut h, 0.3);
    assert!(played(&h, sound::CREAK));
    assert!(played(&h, sound::SLACK));
    assert!(spoke(&h, "Too soon"), "{:?}", h.spoken());
    assert!(status_line(&h).contains("10 arrows left"));
    h.clear();
    hold(&mut h, 1.0);
    assert!(played(&h, sound::FULL));
    // The tutorial taught the click, so play does not explain it again.
    assert!(!spoke(&h, "Full draw."), "{:?}", h.spoken());
    assert!(played(&h, sound::LOOSE));
    assert!(status_line(&h).contains("9 arrows left"));
    // Down eases a draw without shooting.
    h.send(hearken::InputEvent::key(Key::Space));
    wait(&mut h, 0.3);
    h.clear();
    h.press(Key::Down);
    assert!(spoke(&h, "String eased."));
    h.send(hearken::InputEvent::key_up(Key::Space));
    wait(&mut h, 0.1);
    assert!(status_line(&h).contains("9 arrows left"));
}

#[test]
fn a_run_ends_in_results_and_records_its_score() {
    let mut h = field(Mode::Rooftops);
    play_out(&mut h);
    assert_eq!(h.app.top_name(), "pages");
    assert!(played(&h, sound::EVENING_ENDS) || played(&h, sound::WIN));
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    let rows = rows_of(&h.app.top_lines());
    assert!(
        rows[0] == "Try again" || rows[0] == "Play again",
        "{rows:?}"
    );
    assert_eq!(rows.last().unwrap(), "Back to the title");
    assert!(Records::best(&mut h.app.ctx, ID, "rooftops").is_some());
    assert_eq!(Progress::load(&mut h.app.ctx).runs, 1);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    h.press(Key::Escape);
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn moonlight_opens_once_the_rooftops_are_cleared() {
    let mut h = title();
    let progress = Progress {
        best_roofs: 6,
        ..Default::default()
    };
    progress.store(&mut h.app.ctx);
    h.press(Key::Escape);
    h.app.ctx.push(cupids_quiver::title());
    h.frame(0.0);
    goto(&mut h, "Moonlight");
    assert_eq!(cursor_row(&h), "Moonlight");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(&h, "Moonlight, round 1: The chimney tops."));
}

#[test]
fn practice_never_ends_and_escape_leaves() {
    let mut h = title();
    goto(&mut h, "Practice field");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(&h, "The practice field."));
    for _ in 0..15 {
        shoot(&mut h);
    }
    assert_eq!(h.app.top_name(), FIELD_NAME);
    h.clear();
    h.press(Key::Down);
    assert!(spoke(&h, "Endless arrows"));
    h.press(Key::Escape);
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

/// Settings, from whichever menu is on top, with `row` flipped, then back.
fn flip_in_settings(h: &mut hearken::Headless<parlor_core::Parlor>, row: &str) {
    goto(h, "Settings");
    h.press(Key::Return);
    h.press(Key::Return); // the game's own category
    goto(h, row);
    h.press(Key::Return);
    h.press(Key::Escape);
    h.press(Key::Escape);
}

#[test]
fn call_outs_switch_off_in_settings_and_stay_off() {
    let mut h = title();
    goto(&mut h, "Settings");
    h.press(Key::Return);
    h.press(Key::Return); // the game's own category
    goto(&mut h, "Spoken targets");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Spoken targets: off"));
    assert!(Progress::load(&mut h.app.ctx).quiet);
    assert_eq!(cursor_row(&h), "Spoken targets: off");
    goto(&mut h, "Spoken aim");
    h.press(Key::Right);
    assert!(Progress::load(&mut h.app.ctx).quiet_aim);
    assert_eq!(cursor_row(&h), "Spoken aim: off");
}

#[test]
fn the_parlor_settings_hold_no_quiver_rows_once_the_game_is_left() {
    let mut h = title();
    goto(&mut h, "Back to the parlor");
    h.press(Key::Return);
    h.app
        .ctx
        .push(hearken::screens::SettingsScreen::<parlor_core::Parlor>::new());
    h.frame(0.0);
    assert!(!h
        .app
        .top_lines()
        .iter()
        .any(|l| l.contains(cupids_quiver::AIM_CALLS)
            || l.contains(cupids_quiver::TARGET_CALLS)
            || l.contains("Cupid")));
}

#[test]
fn the_pause_menu_settings_change_the_run_at_once() {
    let mut h = field(Mode::Rooftops);
    h.press(Key::Escape);
    flip_in_settings(&mut h, "Spoken aim");
    goto(&mut h, "Resume");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Right);
    assert!(played(&h, sound::TICK));
    assert!(!spoke(&h, "degrees"), "{:?}", h.spoken());
}

#[test]
fn the_first_heart_is_named_after_the_rounds_opening() {
    use hearken::speech::SpeechChannel;
    let mut h = field(Mode::Rooftops);
    wait(&mut h, 3.0);
    let calls = h.speech.borrow().calls(SpeechChannel::Main);
    let named = calls
        .iter()
        .find(|(l, _)| {
            l.ends_with("heart, from the left.") || l.ends_with("heart, from the right.")
        })
        .expect("the first heart is named");
    assert!(!named.1, "it waits for the opening: {calls:?}");
}

#[test]
fn turning_says_the_aim_even_over_a_longer_line() {
    // The round's opening is still being read.
    let mut h = field(Mode::Rooftops);
    h.clear();
    h.press(Key::Right);
    assert_eq!(h.spoken(), ["5 degrees right."]);
}

#[test]
fn aim_call_outs_off_turn_with_only_the_tick() {
    let mut h = title();
    flip_in_settings(&mut h, "Spoken aim");
    goto(&mut h, "Play");
    h.press(Key::Return);
    wait(&mut h, 3.0);
    h.clear();
    h.press(Key::Right);
    assert!(played(&h, sound::TICK));
    assert!(!spoke(&h, "degrees"), "{:?}", h.spoken());
    assert!(status_line(&h)
        .to_lowercase()
        .contains("aim 5 degrees right"));
}

#[test]
fn target_call_outs_off_name_no_target() {
    let mut h = title();
    // A player who has met a sweetheart pair before, so nothing is new.
    let mut progress = Progress::load(&mut h.app.ctx);
    progress.taught.insert("pair".to_string());
    progress.store(&mut h.app.ctx);
    flip_in_settings(&mut h, "Spoken targets");
    goto(&mut h, "Play");
    h.press(Key::Return);
    h.clear();
    wait(&mut h, 3.0);
    assert!(played(&h, sound::HUM));
    assert!(!spoke(&h, "heart, from the"), "{:?}", h.spoken());
}

#[test]
fn a_new_player_is_taught_each_cue_once() {
    let mut h = base(AUTUMN);
    h.app.ctx.push(cupids_quiver::Field::new(
        cupids_quiver::field::Launch::seeded(Mode::Rooftops, SEED),
    ));
    h.frame(0.0);
    h.clear();
    hold(&mut h, 1.0);
    assert!(spoke(&h, "Full draw. Let go to shoot."));
    // A new run remembers it.
    h.app.ctx.pop();
    h.app.ctx.push(cupids_quiver::Field::new(
        cupids_quiver::field::Launch::seeded(Mode::Rooftops, SEED),
    ));
    h.frame(0.0);
    h.clear();
    hold(&mut h, 1.0);
    assert!(played(&h, sound::FULL));
    assert!(!spoke(&h, "Full draw."), "{:?}", h.spoken());
}

#[test]
fn the_tutorial_never_talks_over_its_own_prompts() {
    use hearken::speech::SpeechChannel;
    let mut h = base(AUTUMN);
    h.app.ctx.push(cupids_quiver::title());
    h.frame(0.0);
    h.press(Key::Return);
    wait(&mut h, 3.0);
    press_action(&mut h, ParlorAction::Right);
    press_action(&mut h, ParlorAction::Up);
    shoot(&mut h);
    wait(&mut h, 3.0);
    let calls = h.speech.borrow().calls(SpeechChannel::Main);
    for (line, interrupts) in &calls {
        for field_line in ["heart, from the", "Full draw", "Miss", "In your sights:"] {
            assert!(
                !(*interrupts && line.contains(field_line)),
                "{line:?} cut off the lesson: {calls:?}"
            );
        }
    }
    // The field's opening is the lesson's to give.
    assert!(!calls.iter().any(|(l, _)| l.contains("aim. Hold Space")));
}

#[test]
fn records_love_notes_and_the_keepsake_box_read_as_pages() {
    let mut h = title();
    goto(&mut h, "Records");
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken()[0].starts_with("Records. The rooftops: not played yet."));
    read_pages(&mut h);
    goto(&mut h, "Love notes");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Love notes: none kept yet, of 24."));
    read_pages(&mut h);
    goto(&mut h, "Keepsake box");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "The keepsake box is empty."));
    read_pages(&mut h);
    let mut p = Progress::default();
    p.find_note();
    p.find_note();
    p.store(&mut h.app.ctx);
    let pages = cupids_quiver::notes_pages(&mut h.app.ctx);
    assert_eq!(pages.len(), 3);
    assert!(pages[1].starts_with("Note 1. In neat pencil"));
}

#[test]
fn the_sound_check_plays_each_sound_by_name() {
    let mut h = title();
    goto(&mut h, "Sound check");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "cupids_quiver_sounds");
    let rows = rows_of(&h.app.top_lines());
    assert_eq!(rows[0], "Red heart");
    assert!(rows.contains(&"Sweetheart chord".to_string()));
    assert!(!rows.contains(&"Enter between rooftops".to_string()));
    h.press(Key::Home);
    h.clear();
    h.press(Key::Return);
    assert!(played(&h, sound::HUM));
    assert!(h.spoken().is_empty());
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn pause_offers_the_standard_rows() {
    let mut h = field(Mode::Rooftops);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    assert_eq!(
        rows_of(&h.app.top_lines()),
        [
            "Resume",
            "Settings",
            "Help",
            "Back to the title",
            "Back to the lobby"
        ]
    );
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), FIELD_NAME);
    assert!(spoke(&h, "Back on the roof."));
}

#[test]
fn the_help_document_ends_with_controls() {
    let help = cupids_quiver::HELP;
    assert!(help.contains("## About the game"));
    assert!(help.trim_end().ends_with(
        "These are the default keys. If you change them in Settings, the game's spoken instructions follow your bindings."
    ));
    let last = help.rsplit("\n## ").next().unwrap();
    assert!(last.starts_with("Controls"));
}

#[test]
fn every_title_row_has_help_and_rows_never_move() {
    let mut h = title();
    let locked = rows_of(&h.app.top_lines());
    h.press(Key::Home);
    for row in &locked {
        h.clear();
        h.press(Key::F1);
        let said = h.spoken().join(" ");
        assert!(
            !said.is_empty() && said != hearken::menu::DEFAULT_INTRO_HELP,
            "{row}: {said}"
        );
        h.press(Key::Down);
    }
    // Everything open, on the day itself.
    let mut h = title_on(DAY);
    Progress {
        best_roofs: 6,
        ..Default::default()
    }
    .store(&mut h.app.ctx);
    h.press(Key::Escape);
    h.app.ctx.push(cupids_quiver::title());
    h.frame(0.0);
    let open = rows_of(&h.app.top_lines());
    assert_eq!(locked.len(), open.len());
    let first = |r: &String| r.split([' ', ',', ':']).next().unwrap_or("").to_string();
    for (a, b) in locked.iter().zip(&open) {
        assert_eq!(first(a), first(b), "{a:?} became {b:?}");
    }
    assert!(!open.iter().any(|r| r.ends_with(", locked")), "{open:?}");
}
