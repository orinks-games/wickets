//! The game through its screens, as a player would meet it.

use crate::common::*;
use hearken::{Headless, Key};
use parlor_core::testkit::every_action_answers;
use parlor_core::{Challenge, Period, Records};
use the_echo::play::PLAY_NAME;
use the_echo::progress::RANK_BOARD;
use the_echo::soundcheck::SOUNDS_NAME;
use the_echo::{sound, Mode, Progress, Tuning, ID, TITLE_NAME};

#[test]
fn the_title_follows_the_standard_order() {
    let h = title();
    assert_eq!(
        h.spoken()[1..],
        ["The Echo.", "Play, evening tuning. 1 of 9."]
    );
    assert!(played(&h, sound::TITLE_MUSIC));
    assert_eq!(
        rows(&h),
        [
            "Play, evening tuning",
            "Challenges, today's daily not played yet",
            "Practice rank",
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
fn a_rank_opens_with_both_tones_and_the_first_pipe() {
    let (h, twin) = play(Mode::Rank);
    assert!(spoke(&h, "Evening tuning. A rank of 6 pipes, low to high."));
    assert!(spoke(
        &h,
        &format!(
            "Pipe 1 of 6, {}.",
            the_echo::rules::note_name(twin.pipes[0].note)
        )
    ));
    assert_eq!(h.played().iter().filter(|k| *k == sound::PIPE).count(), 2);
}

#[test]
fn every_key_answers_while_tuning() {
    let (mut h, _) = play(Mode::Rank);
    every_action_answers(&mut h);
}

#[test]
fn tuning_a_pipe_by_keys() {
    let (mut h, mut twin) = play(Mode::Rank);
    h.clear();
    h.press(Key::Space);
    twin.set();
    assert!(played(&h, sound::NOT_YET));
    h.clear();
    h.press(Key::H);
    twin.hint();
    let sharp = twin.pipe().off > 0;
    assert!(
        spoke(&h, if sharp { "Sharp, " } else { "Flat, " }),
        "{:?}",
        h.spoken()
    );
    assert!(spoke(&h, "That hint cost 3 points."));
    h.clear();
    h.press(Key::Up);
    twin.nudge(true);
    assert!(played(&h, sound::SLIDE));
    h.clear();
    tune_pipe(&mut h, &mut twin);
    assert!(played(&h, sound::IN_TUNE));
    assert!(spoke(&h, "In tune. 5 points."), "{:?}", h.spoken());
    assert!(spoke(&h, "Pipe 2 of 6"));
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        [
            "Pipe 2 of 6, wobbling, 1 of 6 in tune.",
            "Evening tuning, score 5."
        ]
    );
    h.clear();
    h.press(Key::Left);
    h.press(Key::Up);
    assert!(spoke(&h, "This pipe is in tune."));
}

#[test]
fn a_tuned_rank_plays_back_and_records_the_score() {
    let (mut h, mut twin) = play(Mode::Rank);
    tune_all(&mut h, &mut twin);
    assert!(spoke(&h, "Every pipe is in tune."));
    h.clear();
    hear_tune(&mut h);
    let notes = h.played().iter().filter(|k| *k == sound::NOTE).count();
    assert_eq!(notes, the_echo::rules::TUNE_LEN);
    assert!(played(&h, sound::RANK_TUNED));
    assert_eq!(h.app.top_name(), "pages");
    assert!(
        spoke(
            &h,
            "Rank tuned: 6 pipes, all 6 at the first try without a hint."
        ),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Return);
    assert!(
        spoke(&h, "60 points, your first score."),
        "{:?}",
        h.spoken()
    );
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!((p.ranks, p.pipes, p.perfect), (1, 6, 6));
    assert_eq!(Records::best(&mut h.app.ctx, ID, RANK_BOARD), Some(60));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn the_practice_rank_has_free_hints_and_no_score() {
    let mut h = title();
    goto(&mut h, "Practice rank");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    assert!(spoke(&h, "Hints are free here."));
    let (mut h, mut twin) = play(Mode::Practice);
    h.clear();
    h.press(Key::H);
    assert!(!spoke(&h, "That hint cost"));
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        [
            "Pipe 1 of 4, wobbling, 0 of 4 in tune.",
            "Practice rank, no score kept."
        ]
    );
    tune_all(&mut h, &mut twin);
    hear_tune(&mut h);
    assert!(spoke(&h, "Practice rank tuned. 4 pipes in tune."));
    read_pages(&mut h);
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!((p.practice_ranks, p.pipes, p.ranks), (1, 4, 0));
}

#[test]
fn back_to_the_title_from_pause_does_not_count_the_rank() {
    let (mut h, mut twin) = play(Mode::Rank);
    tune_pipe(&mut h, &mut twin);
    h.press(Key::Escape);
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(Progress::load(&mut h.app.ctx).ranks, 0);
}

#[test]
fn a_challenge_rank_is_briefed_tuned_and_submitted() {
    let mut h = title();
    goto(&mut h, "Challenges");
    h.press(Key::Return);
    h.press(Key::Home);
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "Tonight's rank: A rank of 6 pipes"),
        "{:?}",
        h.spoken()
    );
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let mut twin = the_echo::challenge::Plan::for_challenge(&ch).rank();
    tune_all(&mut h, &mut twin);
    hear_tune(&mut h);
    assert!(spoke(&h, "Rank tuned: 6 pipes"), "{:?}", h.spoken());
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    assert!(parlor_core::Challenges::best(&mut h.app.ctx, &ch).is_some());
}

#[test]
fn the_tutorial_tunes_two_pipes() {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(QUIET_DAY);
    let rank = the_echo::tutorial::rank();
    let lesson = the_echo::tutorial::lesson_on(&mut h.app.ctx, rank.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    assert!(
        spoke(&h, "Theo's notes are open on the cellar bench"),
        "{:?}",
        h.spoken()
    );
    let mut second = None;
    for _ in 0..4 {
        if h.app.top_name() != "lesson" || rank.with(|t| t.rank.finished()) {
            break;
        }
        let off = rank.with(|t| t.rank.pipe().off);
        let key = if off > 0 { Key::Down } else { Key::Up };
        for _ in 0..off.abs() {
            h.press(key);
            h.frame(DT);
        }
        h.press(Key::Space);
        h.frame(DT);
        if second.is_none() {
            // The game's words for the first pipe are heard before the
            // next step is said.
            assert_eq!(
                h.spoken().last().map(String::as_str),
                Some("Pipe 2 of 2, D 5.")
            );
            for _ in 0..240 {
                h.frame(DT);
            }
            second = h
                .spoken()
                .iter()
                .position(|l| l.contains("The next pipe is higher."));
            assert!(second.is_some(), "{:?}", h.spoken());
        }
    }
    assert!(rank.with(|t| t.rank.finished()));
    assert_eq!(h.app.top_name(), "lesson");
    h.clear();
    for _ in 0..600 {
        if h.app.top_name() == "base" {
            break;
        }
        h.frame(DT);
    }
    assert_eq!(h.app.top_name(), "base", "{:?}", h.spoken());
    // Both notes of the playback are heard before the lesson ends.
    assert_eq!(h.played().iter().filter(|k| *k == sound::NOTE).count(), 2);
    let done = h
        .spoken()
        .iter()
        .position(|l| l.starts_with("Tutorial done."))
        .unwrap();
    assert_eq!(h.spoken().len(), done + 2, "{:?}", h.spoken());
    assert!(!h.spoken()[done].contains("plays them back"));
    assert!(!h.spoken().iter().any(|l| l.contains("Both pipes")));
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
    for (key, _) in sound::catalog() {
        let path = root.join(format!("{key}.ogg"));
        assert!(path.exists(), "{}", path.display());
    }
}

#[test]
fn records_name_the_rank_and_the_tallies() {
    let (mut h, mut twin) = play(Mode::Rank);
    tune_all(&mut h, &mut twin);
    hear_tune(&mut h);
    read_pages(&mut h);
    h.press(Key::Escape);
    goto(&mut h, "Records");
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "Records. Evening tuning: best score 60, 1 rank tuned."),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Return);
    assert!(
        spoke(
            &h,
            "Tuned so far: 6 pipes in all, 6 at the first try without a hint, and 0 practice ranks."
        ),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn keys_during_the_playback_are_answered() {
    let (mut h, mut twin) = play(Mode::Rank);
    tune_all(&mut h, &mut twin);
    h.clear();
    h.press(Key::Escape);
    assert!(
        spoke(&h, "The Echo is playing the rank back."),
        "{:?}",
        h.spoken()
    );
    assert_eq!(h.app.top_name(), PLAY_NAME);
    hear_tune(&mut h);
    assert_eq!(h.app.top_name(), "pages");
}

#[test]
fn a_not_yet_says_what_it_cost() {
    let (mut h, twin) = play(Mode::Rank);
    assert_ne!(twin.pipe().off, 0);
    h.clear();
    h.press(Key::Space);
    assert!(spoke(&h, "That try cost 2 points."), "{:?}", h.spoken());
}

#[test]
fn practice_reads_the_controls_only_to_a_newcomer() {
    let (mut h, mut twin) = play(Mode::Practice);
    assert!(spoke(&h, "Space sets it"), "{:?}", h.spoken());
    tune_all(&mut h, &mut twin);
    hear_tune(&mut h);
    read_pages(&mut h);
    h.app.ctx.push(Tuning::with_seed(Mode::Practice, SEED));
    h.clear();
    h.frame(0.0);
    assert!(spoke(&h, "F1 lists the controls."), "{:?}", h.spoken());
    assert!(!spoke(&h, "Space sets it"), "{:?}", h.spoken());
}

#[test]
fn a_challenge_replay_adds_no_pipes_to_the_totals() {
    let mut h = title();
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let mut twin = the_echo::challenge::Plan::for_challenge(&ch).rank();
    while !twin.finished() {
        let off = twin.pipe().off;
        for _ in 0..off.abs() {
            twin.nudge(off < 0);
        }
        twin.set();
    }
    the_echo::challenge::finish(&mut h.app.ctx, &ch, &twin);
    let first = Progress::load(&mut h.app.ctx);
    the_echo::challenge::finish(&mut h.app.ctx, &ch, &twin);
    let again = Progress::load(&mut h.app.ctx);
    assert_eq!(first.pipes, 6);
    assert_eq!((again.pipes, again.perfect), (first.pipes, first.perfect));
}

#[test]
fn the_tutorial_ends_promptly_when_both_pipes_are_tuned_at_once() {
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(QUIET_DAY);
    let rank = the_echo::tutorial::rank();
    let lesson = the_echo::tutorial::lesson_on(&mut h.app.ctx, rank.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    // Both pipes, straight away, before step 1's words are heard.
    for _ in 0..2 {
        let off = rank.with(|t| t.rank.pipe().off);
        let key = if off > 0 { Key::Down } else { Key::Up };
        for _ in 0..off.abs() {
            h.press(key);
        }
        h.press(Key::Space);
    }
    assert!(rank.with(|t| t.rank.finished()));
    wait(&mut h, 30.0);
    assert_eq!(h.app.top_name(), "base", "{:?}", h.spoken());
    assert!(spoke(&h, "Tutorial done."), "{:?}", h.spoken());
}
