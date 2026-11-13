//! The game through its screens, as a player meets it.

use crate::common::*;
use dragon_dance::dance::DANCE_NAME;
use dragon_dance::progress::Progress;
use dragon_dance::select::SELECT_NAME;
use dragon_dance::{sound, Mode, ID, TITLE_NAME};
use hearken::{Headless, Key};
use parlor_core::testkit::press_action;
use parlor_core::ParlorAction;
use parlor_core::{testkit, Records};

#[test]
fn the_title_follows_the_standard_order() {
    let h = title();
    assert_eq!(
        rows(&h),
        [
            "Play, street 1: Market Row",
            "Street select",
            "New Year parade, locked",
            "Lantern walk, locked",
            "Challenges, daily not played yet",
            "Practice street",
            "Tutorial",
            "Help",
            "Sound check",
            "Records",
            "Keepsakes",
            "Settings",
            "Back to the parlor",
        ]
    );
    assert!(played(&h, sound::TITLE_MUSIC));
}

#[test]
fn a_first_visit_starts_on_the_tutorial() {
    let mut h = base_on(ORDINARY);
    h.app.ctx.push(dragon_dance::title());
    h.frame(0.0);
    assert_eq!(
        h.spoken()[1..],
        [
            "Dragon Dance. New here? Start with the tutorial.",
            "Tutorial. 7 of 13."
        ]
    );
    h.press(Key::Return);
    assert!(spoke(&h, "You are the head of a long paper dragon in a street parade, and the dragon follows where you lead."));
    assert_eq!(h.app.top_name(), "lesson");
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert!(spoke(&h, "Tutorial left."));
}

#[test]
fn the_tutorial_teaches_on_the_practice_street() {
    use dragon_dance::rules::Mark;
    use dragon_dance::tutorial;
    let mut h = title();
    let street = tutorial::street();
    let lesson = tutorial::lesson_on(&mut h.app.ctx, street.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    assert!(spoke(
        &h,
        "The Left arrow and the Right arrow move your head one lane."
    ));
    let dt = 1.0 / 60.0;
    for _ in 0..(60 * 200) {
        if h.app.top_name() != "lesson" {
            break;
        }
        let (lane, pearl, clock, gong) = street.with(|s| {
            let d = &s.dance;
            let gong = d
                .plan
                .marks
                .iter()
                .filter(|m| matches!(m.mark, Mark::Gong { .. }))
                .map(|m| d.time_of(m.beat))
                .find(|t| *t >= d.clock - 0.02);
            (d.lane, d.pearl, d.clock, gong)
        });
        if lane < pearl {
            press_action(&mut h, ParlorAction::Right);
        } else if lane > pearl {
            press_action(&mut h, ParlorAction::Left);
        } else if gong.is_some_and(|t| (t - clock).abs() < dt) {
            press_action(&mut h, ParlorAction::Primary);
        } else {
            h.frame(dt);
        }
    }
    assert!(spoke(&h, "beats in a row."));
    assert!(spoke(&h, "A rear on the gong. Tutorial done."));
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn a_tutorial_that_never_rears_starts_the_street_over_instead_of_scoring() {
    use dragon_dance::tutorial;
    let mut h = title();
    let street = tutorial::street();
    let lesson = tutorial::lesson_on(&mut h.app.ctx, street.clone());
    h.app.ctx.push(lesson);
    let beats = street.with(|s| s.dance.street.beats());
    // Long enough for the whole street and more, never pressing a key.
    let seconds = (beats as f64 + 40.0) * street.with(|s| s.dance.street.beat_len());
    for _ in 0..(seconds as usize * 10) {
        h.frame(0.1);
    }
    assert!(spoke(&h, "Back to the start of the street."));
    assert_eq!(h.app.top_name(), "lesson");
    // Standing still never passes the follow step.
    assert!(!spoke(&h, "4 beats in a row. Each beat"));
}

#[test]
fn the_tutorial_head_starts_a_lane_away_from_the_pearl() {
    let street = dragon_dance::tutorial::street();
    street.with(|s| assert_ne!(s.dance.lane, s.dance.pearl));
}

#[test]
fn every_title_row_has_its_own_help() {
    let mut h = title();
    let n = rows(&h).len();
    h.press(Key::Home);
    for i in 0..n {
        h.clear();
        h.press(Key::F1);
        let said = h.spoken().join(" ");
        assert!(
            !said.is_empty() && said != hearken::menu::DEFAULT_INTRO_HELP,
            "row {i}"
        );
        h.press(Key::Down);
    }
}

#[test]
fn play_starts_the_first_street_with_its_intro() {
    let mut h = title();
    h.clear();
    start(&mut h);
    assert_eq!(h.app.top_name(), DANCE_NAME);
    assert!(spoke(
        &h,
        "Street 1: Market Row. 3 lanes, 24 bars, no firecrackers. Follow the pearl. The drums count in 8 beats."
    ));
    assert!(spoke(
        &h,
        "The Left arrow and the Right arrow change lanes."
    ));
    assert!(played(&h, sound::PEARL));
    assert!(played(&h, sound::CROWD));
    assert!(played(&h, sound::STREET_BED));
}

#[test]
fn the_count_in_then_the_first_beat_teaches_the_pearl() {
    let mut h = dance(Mode::Street(1));
    h.clear();
    wait(&mut h, 3.0);
    assert!(played(&h, sound::COUNT));
    assert!(h.spoken().is_empty());
    wait(&mut h, 3.5);
    assert!(spoke(
        &h,
        "Dance. Keep your head in the shimmering pearl's lane."
    ));
    assert!(played(&h, sound::DRUM_DOWN));
    assert!(played(&h, sound::DRUM));
}

#[test]
fn every_action_answers_in_play() {
    let mut h = dance(Mode::Street(2));
    wait(&mut h, 1.0);
    testkit::every_action_answers(&mut h);
    wait(&mut h, 8.0);
    testkit::every_action_answers(&mut h);
}

#[test]
fn left_and_right_name_the_lane_and_the_edge() {
    let mut h = dance(Mode::Street(1));
    h.clear();
    h.press(Key::Left);
    assert_eq!(h.spoken(), ["Lane 1."]);
    assert!(played(&h, sound::STEP));
    h.clear();
    h.press(Key::Left);
    assert_eq!(h.spoken(), ["The left edge of the street. Lane 1."]);
    assert!(played(&h, sound::EDGE));
    h.clear();
    h.press(Key::Right);
    assert_eq!(h.spoken(), ["Lane 2, the pearl."]);
    assert!(played(&h, sound::PEARL_HERE));
}

#[test]
fn tab_finds_the_pearl_and_status_reads_the_street() {
    let mut h = dance(Mode::Street(1));
    h.press(Key::Right);
    h.clear();
    h.press(Key::Tab);
    assert_eq!(h.spoken(), ["Pearl, lane 2, 1 lane to your left."]);
    assert!(played(&h, sound::PEARL_MOVE));
    h.clear();
    h.press(Key::S);
    let status = h.spoken().join(" ");
    assert!(
        status.starts_with("Lane 3 of 3, pearl in lane 2, 1 lane to your left"),
        "{status}"
    );
    h.clear();
    h.press(Key::H);
    assert!(spoke(&h, "Counting in."));
    h.clear();
    h.press(Key::Down);
    assert!(spoke(&h, "Your dragon is 6 segments long"));
    h.clear();
    h.press(Key::Up);
    assert!(spoke(&h, "Nothing coming. Follow the pearl."));
}

#[test]
fn a_stray_rear_says_there_is_no_gong() {
    let mut h = dance(Mode::Street(1));
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.spoken(), ["No gong to rear on."]);
    assert!(played(&h, sound::REAR_STRAY));
    h.clear();
    h.press(Key::R);
    assert_eq!(h.spoken(), ["No gong to rear on."]);
}

#[test]
fn a_key_with_no_job_stays_silent() {
    let mut h = dance(Mode::Street(1));
    h.clear();
    h.press(Key::Q);
    assert!(h.spoken().is_empty(), "{:?}", h.spoken());
}

#[test]
fn escape_pauses_and_back_to_the_title_leaves() {
    let mut h = dance(Mode::Street(1));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), DANCE_NAME);
    assert!(spoke(&h, "Back in the parade."));
    h.press(Key::Escape);
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn a_whole_street_danced_by_the_keys_earns_stars_and_opens_the_next() {
    let mut h = title();
    start(&mut h);
    let heard = dance_by_keys(&mut h, 200.0);
    assert_eq!(h.app.top_name(), "results");
    let all = heard.join(" ");
    assert!(all.contains("Perfect rear, 100 points."), "{all}");
    assert!(all.contains("Drum roll: a gong in 2 beats."));
    let results = rows(&h);
    assert_eq!(results[0], "Next street: street 2, Tea House Lane");
    assert_eq!(results[1], "Play again");
    assert_eq!(results[2], "Hear the results again");
    assert_eq!(results.last().unwrap(), "Back to the title");
    let title_line = &h.app.top_lines()[0];
    assert!(title_line.starts_with("Street 1 danced: "), "{title_line}");
    let progress = Progress::load(&mut h.app.ctx);
    assert!(progress.stars_on(1) >= 2);
    assert!(progress.opened(2));
    assert_eq!(progress.danced, 1);
    assert!(Records::best(&mut h.app.ctx, ID, "street 1").is_some());
    // Next street opens its own briefing in place of the results, so
    // Escape there goes back to the title.
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), parlor_core::briefing::NAME);
    assert!(spoke(&h, "Street 2: Tea House Lane."), "{:?}", h.spoken());
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    goto(&mut h, "Play, street 2: Tea House Lane");
    start(&mut h);
    assert_eq!(h.app.top_name(), DANCE_NAME);
    h.press(Key::Escape);
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(rows(&h)[0], "Play, street 2: Tea House Lane");
}

#[test]
fn standing_still_earns_no_star_and_says_what_one_needs() {
    let mut h = dance(Mode::Street(1));
    h.press(Key::Left);
    wait(&mut h, 120.0);
    assert_eq!(h.app.top_name(), "results");
    assert_eq!(rows(&h)[0], "Try again");
    h.press(Key::Down);
    h.press(Key::Return);
    assert!(spoke(&h, "Street 1 danced: no stars"));
    h.press(Key::Return);
    assert!(spoke(&h, "On the pearl for"));
    h.press(Key::Return);
    assert!(spoke(&h, "A star needs"));
    assert!(!Progress::load(&mut h.app.ctx).opened(2));
}

#[test]
fn firecrackers_are_called_and_a_stumble_costs_points() {
    // Street 2 has firecrackers; stand in one lane and wait for one.
    let mut h = dance(Mode::Street(2));
    let mut heard = Vec::new();
    for _ in 0..(90 * 60) {
        h.frame(1.0 / 60.0);
        heard.extend(h.spoken());
        h.clear();
        if h.app.top_name() != DANCE_NAME {
            break;
        }
    }
    let all = heard.join(" ");
    assert!(all.contains("Firecrackers, lane"), "{all}");
    let progress = Progress::load(&mut h.app.ctx);
    assert!(progress.taught.contains("firecracker"));
}

#[test]
fn the_practice_street_saves_nothing() {
    let mut h = title();
    goto(&mut h, "Practice street");
    start(&mut h);
    assert!(spoke(
        &h,
        "Practice: nothing is saved, and stumbles cost nothing."
    ));
    dance_by_keys(&mut h, 120.0);
    assert_eq!(h.app.top_name(), "results");
    assert!(h.app.top_lines()[0].starts_with("Practice street danced"));
    let progress = Progress::load(&mut h.app.ctx);
    assert_eq!(progress.danced, 0);
    assert!(progress.stars.is_empty());
}

#[test]
fn street_select_lists_open_and_locked_streets() {
    let mut h = title();
    goto(&mut h, "Street select");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), SELECT_NAME);
    let list = rows(&h);
    assert_eq!(list.len(), 9);
    assert_eq!(list[0], "Street 1: Market Row, new");
    assert_eq!(list[1], "Street 2: Tea House Lane, locked");
    assert_eq!(list[8], "Back");
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Earn a star on street 1 to open it."));
    assert!(played(&h, sound::NOPE));
    h.press(Key::Up);
    start(&mut h);
    assert_eq!(h.app.top_name(), DANCE_NAME);
}

#[test]
fn street_select_shows_stars_once_earned() {
    let mut h = title();
    let mut progress = Progress::default();
    progress.set_stars(1, 2);
    progress.store(&mut h.app.ctx);
    goto(&mut h, "Street select");
    h.press(Key::Return);
    let list = rows(&h);
    assert_eq!(list[0], "Street 1: Market Row, 2 stars");
    assert_eq!(list[1], "Street 2: Tea House Lane, new");
    assert_eq!(list[2], "Street 3: Silk Street, locked");
}

#[test]
fn the_call_outs_row_flips_and_says_so() {
    let mut h = title();
    goto(&mut h, "Settings");
    h.press(Key::Return);
    h.press(Key::Return); // the game's own category
    goto(&mut h, "Spoken call-outs");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Spoken call-outs: off."));
    assert!(rows(&h).contains(&"Spoken call-outs: off".to_string()));
    assert!(Progress::load(&mut h.app.ctx).quiet);
}

#[test]
fn records_and_keepsakes_read_as_pages() {
    let mut h = title();
    goto(&mut h, "Records");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Records. No streets danced yet."));
    h.press(Key::Escape);
    goto(&mut h, "Keepsakes");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Keepsakes, none yet."));
    h.press(Key::Return);
    assert!(spoke(&h, "Lanterns, none yet."));
}

#[test]
fn the_sound_check_plays_and_names_every_sound() {
    let mut h = title();
    goto(&mut h, "Sound check");
    h.press(Key::Return);
    let n = rows(&h).len();
    assert!(n >= 28);
    h.press(Key::Home);
    for _ in 0..n - 1 {
        h.clear();
        h.press(Key::Return);
        assert!(h.spoken().is_empty(), "{}", cursor_row(&h));
        assert!(!h.played().is_empty(), "{}", cursor_row(&h));
        h.press(Key::Down);
    }
    assert_eq!(cursor_row(&h), "Back");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn every_sound_key_has_a_file() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    for (key, _) in sound::catalog() {
        let path = root.join(format!("{key}.ogg"));
        assert!(path.exists(), "{key}");
    }
}

#[test]
fn help_ends_with_the_controls() {
    // A Windows checkout may turn the line ends into CRLF.
    let help = dragon_dance::HELP.replace("\r\n", "\n");
    assert!(help.starts_with("# Dragon Dance\n\n## About the game"));
    let last = help.rsplit("\n## ").next().unwrap();
    assert!(last.starts_with("Controls"));
    let _ = Headless::<parlor_core::Parlor>::new(Default::default(), Base);
}

#[test]
fn a_streets_briefing_plays_the_cues_it_uses() {
    let labels = |mode| {
        dragon_dance::briefing::cues(mode)
            .iter()
            .map(|c| c.label().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        labels(Mode::Street(1)),
        [
            "Drum",
            "Drum roll",
            "Gong",
            "Pearl",
            "Pearl moves",
            "Off the pearl",
            "Red envelope coming"
        ]
    );
    assert_eq!(
        labels(Mode::Practice),
        [
            "Drum",
            "Drum roll",
            "Gong",
            "Pearl",
            "Pearl moves",
            "Off the pearl",
            "Red envelope coming",
            "Firecrackers coming",
        ]
    );
    assert_eq!(
        labels(Mode::NewYear(2027)),
        [
            "Drum",
            "Drum roll",
            "Gong",
            "Midnight gong",
            "Pearl",
            "Pearl moves",
            "Off the pearl",
            "Red envelope coming",
            "Firecrackers coming",
            "Countdown",
        ]
    );
    assert_eq!(
        labels(Mode::Lantern(2027)),
        [
            "Drum",
            "Pearl",
            "Pearl moves",
            "Off the pearl",
            "Lantern coming"
        ]
    );

    let mut h = title();
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), parlor_core::briefing::NAME);
    h.frame(1.0);
    assert_eq!(h.app.ctx.audio.music(), None, "no music under the briefing");
    assert_eq!(h.app.ctx.audio.bed(), None, "no bed under the briefing");
    assert_eq!(
        h.spoken(),
        ["Street 1: Market Row. Keep your head in the pearl's lane on every beat. Space rears up as the gong strikes. Catch red envelopes. 1 to 7 play the sounds. Enter starts."]
    );
    h.clear();
    h.press(Key::Num3);
    wait(&mut h, 1.0);
    assert_eq!(h.spoken(), ["Gong"]);
    assert!(played(&h, sound::GONG), "{:?}", h.played());
    h.press(Key::Space);
    assert_eq!(h.app.top_name(), DANCE_NAME);
}
