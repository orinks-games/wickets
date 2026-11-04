//! The game through its screens, as a player would meet it.

use crate::common::*;
use hearken::{Headless, Key};
use lamplighter::play::{END_PAUSE, PLAY_NAME, PULSE};
use lamplighter::progress::ROUND_BOARD;
use lamplighter::soundcheck::SOUNDS_NAME;
use lamplighter::{sound, Mode, Progress, ID, TITLE_NAME};
use parlor_core::testkit::every_action_answers;
use parlor_core::{Challenge, Period, Records};

#[test]
fn the_title_follows_the_standard_order() {
    let h = title();
    assert_eq!(
        h.spoken()[1..],
        ["Lamplighter.", "Play, evening round. 1 of 9."]
    );
    assert!(played(&h, sound::TITLE_MUSIC));
    assert_eq!(
        rows(&h),
        [
            "Play, evening round",
            "Challenges, today's daily not played yet",
            "Practice hall",
            "Tutorial",
            "Help",
            "Sound check",
            "Records",
            "Settings",
            "Back to the parlor",
        ]
    );
}

#[test]
fn every_title_row_has_its_own_help() {
    let mut h = title();
    let n = rows(&h).len();
    h.press(Key::Home);
    for _ in 0..n {
        h.clear();
        h.press(Key::F1);
        let said = h.spoken().join(" ");
        assert!(
            !said.is_empty() && said != hearken::menu::DEFAULT_INTRO_HELP,
            "{}: {said}",
            cursor_row(&h)
        );
        h.press(Key::Down);
    }
}

#[test]
fn a_round_waits_for_the_taper() {
    let (mut h, twin) = play(Mode::Round);
    assert!(spoke(&h, "Evening round. 8 lamps out along 16 steps."));
    assert!(spoke(&h, "Space lights your taper and starts the clock."));
    h.clear();
    h.press(Key::Up);
    assert!(spoke(&h, "Your taper is not lit."));
    wait(&mut h, 5.0);
    assert!(!played(&h, sound::LAMP_OUT), "no pulses before the taper");
    h.press(Key::Space);
    assert!(spoke(&h, "Taper lit."));
    h.clear();
    wait(&mut h, PULSE + 0.5);
    let near = twin
        .out_lamps()
        .iter()
        .any(|(step, _)| *step as i32 <= lamplighter::play::HEAR);
    assert_eq!(played(&h, sound::LAMP_OUT), near);
}

#[test]
fn every_key_answers_in_the_hall() {
    let (mut h, mut twin) = play(Mode::Round);
    light_taper(&mut h, &mut twin);
    every_action_answers(&mut h);
}

#[test]
fn walking_steps_and_reaching_lights_with_sound_and_words() {
    let (mut h, mut twin) = play(Mode::Round);
    light_taper(&mut h, &mut twin);
    h.clear();
    h.press(Key::Down);
    assert!(played(&h, sound::LOBBY_DOOR));
    assert!(spoke(&h, "The lobby door."));
    h.clear();
    h.press(Key::H);
    let (ahead, side) = twin.nearest_out().unwrap();
    assert!(played(&h, sound::LAMP_OUT));
    assert!(spoke(&h, "Nearest lamp out: "), "{:?}", h.spoken());
    h.clear();
    light_nearest(&mut h, &mut twin);
    assert!(played(&h, sound::LIGHT));
    if ahead != 0 {
        assert!(played(&h, sound::STEP_A));
    }
    assert!(spoke(&h, "Lit. 7 to go."), "{:?}", h.spoken());
    h.clear();
    h.press(match side {
        lamplighter::rules::Side::Left => Key::Left,
        lamplighter::rules::Side::Right => Key::Right,
    });
    assert!(played(&h, sound::ALREADY_BURNING));
    h.clear();
    h.press(Key::R);
    assert!(spoke(&h, &format!("Step {} of 16. Left: ", twin.pos + 1)));
    h.clear();
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        [format!(
            "7 lamps out, at step {}, taper 2 minutes left.",
            twin.pos + 1
        )]
    );
    h.clear();
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        [format!("Step {} of 16, score 10.", twin.pos + 1)]
    );
}

#[test]
fn a_draught_as_you_light_gives_the_count_after_it() {
    let (mut h, mut twin) = play(Mode::Round);
    light_taper(&mut h, &mut twin);
    // The draught comes with the fourth lamp lit.
    for _ in 0..3 {
        light_nearest(&mut h, &mut twin);
    }
    h.clear();
    light_nearest(&mut h, &mut twin);
    let said = h.spoken();
    assert!(said.iter().any(|l| l == "Lit."), "{said:?}");
    let draught = said
        .iter()
        .find(|l| l.starts_with("A draught."))
        .expect("a draught");
    // 8 out, 4 lit, 1 blown out again.
    assert!(draught.ends_with(" 5 to go."), "{draught}");
    assert!(!said.iter().any(|l| l.starts_with("Lit. ")), "{said:?}");
}

#[test]
fn the_unlit_lamps_nearby_pulse_from_their_walls() {
    let (mut h, mut twin) = play(Mode::Practice);
    let near = twin
        .out_lamps()
        .iter()
        .any(|(step, _)| *step as i32 <= lamplighter::play::HEAR);
    h.clear();
    wait(&mut h, PULSE * 2.0);
    assert_eq!(played(&h, sound::LAMP_OUT), near);
    light_all(&mut h, &mut twin);
}

#[test]
fn a_lit_hall_scores_and_records_the_round() {
    let (mut h, mut twin) = play(Mode::Round);
    light_taper(&mut h, &mut twin);
    wait(&mut h, 10.0);
    light_all(&mut h, &mut twin);
    assert!(played(&h, sound::HALL_LIT));
    assert!(spoke(&h, "Every lamp is burning."));
    wait(&mut h, END_PAUSE + 0.1);
    assert_eq!(h.app.top_name(), "pages");
    assert!(
        spoke(&h, "Hall lit. 8 lamps and 1 relit in 10 seconds."),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Return);
    assert!(
        spoke(&h, "90 points for the lamps and a taper bonus of 110. 200 points, your first score. Fastest hall: 10 seconds, your first."),
        "{:?}",
        h.spoken()
    );
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!((p.rounds, p.lamps, p.fastest), (1, 9, Some(10)));
    assert_eq!(Records::best(&mut h.app.ctx, ID, ROUND_BOARD), Some(200));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn a_burnt_out_taper_is_replaced() {
    let (mut h, mut twin) = play(Mode::Round);
    light_taper(&mut h, &mut twin);
    h.clear();
    wait(&mut h, 101.0);
    assert!(played(&h, sound::TAPER_LOW));
    assert!(spoke(&h, "Taper low. 20 seconds left."));
    wait(&mut h, 20.0);
    assert!(played(&h, sound::TAPER_OUT));
    assert!(spoke(&h, "Taper out. Mabel lights a fresh one."));
    assert_eq!(h.app.top_name(), PLAY_NAME);
    light_all(&mut h, &mut twin);
    wait(&mut h, END_PAUSE + 0.1);
    assert!(
        spoke(&h, "Mabel lit you 1 fresh taper."),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Return);
    assert!(spoke(&h, "No taper bonus: it went with the first taper."));
}

#[test]
fn the_practice_hall_has_no_taper_and_counts_its_hall() {
    let mut h = title();
    goto(&mut h, "Practice hall");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    assert!(spoke(
        &h,
        "Practice hall. 4 lamps out along 8 steps. No taper, so take your time."
    ));
    let (mut h, mut twin) = play(Mode::Practice);
    h.clear();
    h.press(Key::S);
    assert_eq!(h.spoken(), ["4 lamps out, at step 1, no taper."]);
    light_all(&mut h, &mut twin);
    wait(&mut h, END_PAUSE + 0.1);
    assert!(spoke(&h, "Practice hall lit. 4 lamps lit."));
    read_pages(&mut h);
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!((p.practice_halls, p.lamps, p.rounds), (1, 4, 0));
}

#[test]
fn back_to_the_title_from_pause_does_not_count_the_round() {
    let (mut h, mut twin) = play(Mode::Round);
    light_taper(&mut h, &mut twin);
    light_nearest(&mut h, &mut twin);
    h.press(Key::Escape);
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(Progress::load(&mut h.app.ctx).rounds, 0);
}

#[test]
fn a_challenge_hall_is_briefed_played_and_submitted() {
    let mut h = title();
    goto(&mut h, "Challenges");
    h.press(Key::Return);
    h.press(Key::Home);
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "Tonight's hall: A hall of 16 steps with 8 lamps out"),
        "{:?}",
        h.spoken()
    );
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let mut twin = lamplighter::challenge::Plan::for_challenge(&ch).hall();
    light_taper(&mut h, &mut twin);
    light_all(&mut h, &mut twin);
    wait(&mut h, END_PAUSE + 0.1);
    assert!(
        spoke(&h, "Hall lit. 8 lamps and 1 relit in"),
        "{:?}",
        h.spoken()
    );
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    assert!(parlor_core::Challenges::best(&mut h.app.ctx, &ch).is_some());
}

#[test]
fn the_tutorials_walk_step_waits_for_a_walk() {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(QUIET_DAY);
    let hall = lamplighter::tutorial::hall();
    assert_eq!(hall.with(|r| r.hall.nearest_out().map(|(d, _)| d)), Some(2));
    let lesson = lamplighter::tutorial::lesson_on(&mut h.app.ctx, hall.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    wait(&mut h, 2.0);
    let beside = "A lamp that is out is right beside you.";
    assert!(!spoke(&h, beside), "{:?}", h.spoken());
    h.press(Key::Up);
    h.frame(DT);
    assert!(!spoke(&h, beside), "{:?}", h.spoken());
    h.press(Key::Up);
    h.frame(DT);
    assert!(spoke(&h, beside), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Left);
    h.frame(DT);
    let all = h.spoken().join(" ");
    assert!(all.starts_with("Lit. 2 to go."), "{all}");
    assert_eq!(all.matches("Lit.").count(), 1, "{all}");
}

#[test]
fn the_tutorial_walks_and_lights_a_short_hall() {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(QUIET_DAY);
    let hall = lamplighter::tutorial::hall();
    let lesson = lamplighter::tutorial::lesson_on(&mut h.app.ctx, hall.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    assert!(spoke(&h, "The lamps along the Long Hall have gone out, and Mabel has handed you her old taper to light them again"), "{:?}", h.spoken());
    for _ in 0..40 {
        if h.app.top_name() != "lesson" {
            break;
        }
        let next = hall.with(|r| r.hall.nearest_out());
        let Some((ahead, side)) = next else { break };
        let key = if ahead > 0 { Key::Up } else { Key::Down };
        for _ in 0..ahead.abs() {
            h.press(key);
            h.frame(DT);
        }
        h.press(match side {
            lamplighter::rules::Side::Left => Key::Left,
            lamplighter::rules::Side::Right => Key::Right,
        });
        h.frame(DT);
    }
    assert!(spoke(&h, "Every lamp is burning."), "{:?}", h.spoken());
    wait(&mut h, END_PAUSE + 1.0);
    for _ in 0..5 {
        if h.app.top_name() == "base" {
            break;
        }
        h.press(Key::Return);
    }
    assert_eq!(h.app.top_name(), "base", "{:?}", h.spoken());
}

#[test]
fn the_sound_check_plays_every_sound_by_name() {
    let mut h = title();
    goto(&mut h, "Sound check");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), SOUNDS_NAME);
    assert_eq!(rows(&h).len(), sound::catalog().len() + 1);
    for (key, meaning) in sound::catalog() {
        let label = meaning.split(':').next().unwrap();
        goto(&mut h, label);
        h.clear();
        h.press(Key::Return);
        assert!(played(&h, key), "{key}");
        assert_eq!(h.spoken(), [meaning]);
    }
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn every_sound_key_has_a_file() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    let mut keys: Vec<&str> = sound::catalog().into_iter().map(|(k, _)| k).collect();
    keys.extend([sound::STEP_B]);
    for key in keys {
        let path = root.join(format!("{key}.ogg"));
        assert!(path.exists(), "{}", path.display());
    }
}

#[test]
fn records_name_the_round_and_the_tallies() {
    let (mut h, mut twin) = play(Mode::Round);
    light_taper(&mut h, &mut twin);
    light_all(&mut h, &mut twin);
    wait(&mut h, END_PAUSE + 0.1);
    read_pages(&mut h);
    h.press(Key::Escape);
    goto(&mut h, "Records");
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "Records. Evening round: best score "),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Return);
    assert!(
        spoke(
            &h,
            "Lit so far: 1 evening round, 0 practice halls, and 9 lamps in all."
        ),
        "{:?}",
        h.spoken()
    );
}
