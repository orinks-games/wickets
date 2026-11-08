//! The game through its screens, as a player would meet it.

use crate::common::*;
use graveyard_shuffle::play::{Mode, END_PAUSE, HINT_GAP, PLAY_NAME};
use graveyard_shuffle::progress::{self, Progress};
use graveyard_shuffle::rules::{Ev, Judgment, RunEv};
use graveyard_shuffle::songs::{Monster, NIGHTS, SONGS};
use graveyard_shuffle::soundcheck::SOUNDS_NAME;
use graveyard_shuffle::{sound, ID, TITLE_NAME};
use hearken::{Headless, Key};
use parlor_core::testkit::every_action_answers;
use parlor_core::Records;

#[test]
fn the_title_announces_itself_and_plays_its_theme() {
    let h = title();
    assert_eq!(
        h.spoken()[1..],
        [
            "Graveyard Shuffle.",
            "Play, night 1, the graveyard gate. 1 of 10."
        ]
    );
    assert!(played(&h, sound::TITLE_MUSIC));
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(graveyard_shuffle::INFO.title_name(), TITLE_NAME);
}

#[test]
fn the_title_follows_the_standard_order() {
    let h = title();
    assert_eq!(
        rows(&h),
        [
            "Play, night 1, the graveyard gate",
            "Night select",
            "Challenges, daily not played yet",
            "Practice room",
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
fn a_first_visit_starts_on_the_tutorial() {
    let mut h = Headless::new(data(PLAIN_DAY), Base);
    h.app.ctx.push(graveyard_shuffle::title());
    h.frame(0.0);
    assert_eq!(
        h.spoken()[1..],
        [
            "Graveyard Shuffle. New here? Start with the tutorial.",
            "Tutorial. 5 of 10."
        ]
    );
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "Doctor Volt's monster has started a dance craze, and you lead it."
    ));
    assert_eq!(h.app.top_name(), "lesson");
}

#[test]
fn the_tutorial_teaches_in_the_practice_room() {
    use graveyard_shuffle::tutorial;
    let mut h = title();
    let room = tutorial::room(&mut h.app.ctx);
    let lesson = tutorial::lesson_on(&mut h.app.ctx, room.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    assert!(spoke(&h, "Space starts the dance"));
    h.press(Key::Space);
    let mut waited = Vec::new();
    for _ in 0..40_000 {
        if h.app.top_name() != "lesson" {
            break;
        }
        // Wait-dance the first moves, then dance the next one on its beat.
        let (waiting, on_beat) = room.with(|f| {
            let d = &f.run.dance;
            let waiting = d.waiting().map(|i| d.plan.calls[i].step);
            let on_beat = d
                .next_open()
                .filter(|&i| d.now() >= d.time_of(d.plan.calls[i].beat as i64) - 30)
                .map(|i| d.plan.calls[i].step);
            (waiting, on_beat)
        });
        match (waiting, on_beat) {
            (Some(step), _) => {
                waited.push(step);
                h.press(key_for(step));
            }
            (None, Some(step)) if waited.len() >= tutorial::STEPS as usize => {
                h.press(key_for(step))
            }
            _ => h.frame(DT),
        }
    }
    assert_eq!(
        waited[..3],
        [Monster::Zombie, Monster::Skeleton, Monster::Mummy],
        "the lesson dances all three monsters"
    );
    assert!(spoke(&h, "Here comes the band."));
    assert!(spoke(&h, "spot on, with a spooky chime, for 3 points."));
    assert!(spoke(&h, "Now one in time"));
    assert!(spoke(&h, "In time. Tutorial done."));
    assert!(
        !spoke(&h, "Zombie, the Down arrow."),
        "the wait names each move, so the call does not"
    );
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn every_title_row_has_its_own_help() {
    for date in [PLAIN_DAY] {
        let mut h = title_on(date);
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
}

#[test]
fn play_opens_the_floor_with_the_intro_and_bed() {
    let mut h = title();
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    assert!(spoke(
        &h,
        "Night 1, the graveyard gate. 3 songs: The Graveyard Warm-up, Boneyard Boogie, The Shallow Grave Shuffle. Space starts the dance. H plays every monster and its move."
    ));
    assert!(spoke(
        &h,
        "Move on the beat after each call. Zombie, the Down arrow; skeleton, the Up arrow; mummy, Space."
    ));
    assert!(played(&h, sound::BED_GATE));
}

#[test]
fn before_the_dance_each_step_key_plays_its_call() {
    let (mut h, _) = floor(Mode::Set(1));
    for step in [Monster::Vampire, Monster::Werewolf, Monster::Ghost] {
        h.clear();
        h.press(key_for(step));
        assert!(played(&h, sound::call_key(step)), "{step:?}");
        assert!(played(&h, sound::step_key(step)), "{step:?}");
    }
    h.clear();
    h.press(Key::Down);
    assert_eq!(h.spoken(), ["Zombie, the drag: the Down arrow."]);
    h.clear();
    h.press(Key::Tab);
    assert_eq!(
        h.spoken(),
        ["Ghost, the float: Tab. It does not come up in these songs."]
    );
}

#[test]
fn hint_before_the_dance_plays_every_step_with_its_key() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    h.clear();
    h.press(Key::H);
    assert_eq!(
        h.spoken(),
        ["Zombie, the drag: the Down arrow. Skeleton, the rattle: the Up arrow. Mummy, the unwrap: Space."]
    );
    wait(&mut h, &mut twin, HINT_GAP * 3.0);
    for s in [Monster::Zombie, Monster::Skeleton, Monster::Mummy] {
        assert!(played(&h, sound::call_key(s)));
    }
    assert!(!twin.started());
}

#[test]
fn every_action_answers_before_and_during_the_dance() {
    let (mut h, _) = floor(Mode::Set(1));
    h.clear();
    every_action_answers(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    let (mut h, mut twin) = floor(Mode::Set(2));
    start(&mut h, &mut twin);
    wait(&mut h, &mut twin, 6.0);
    every_action_answers(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    let (mut h, mut twin) = floor(Mode::Practice(0));
    start(&mut h, &mut twin);
    wait(&mut h, &mut twin, 8.0);
    every_action_answers(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
}

#[test]
fn the_dance_counts_in_plays_the_tune_and_calls_each_step() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    h.clear();
    start(&mut h, &mut twin);
    assert!(spoke(&h, "Here we go. Four knocks in."));
    let evs = wait(&mut h, &mut twin, 4.0);
    assert_eq!(played_count(&h, sound::COUNT), 4);
    let first = SONGS[0].part_key(false);
    assert!(played(&h, &first));
    let call = evs
        .iter()
        .find_map(|e| match e {
            RunEv::Dance(Ev::Call { step, .. }) => Some(*step),
            _ => None,
        })
        .expect("a call by now");
    assert!(played(&h, sound::call_key(call)));
}

#[test]
fn a_step_on_the_beat_is_spot_on_and_says_so_once() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    let i = twin.dance.next_open().unwrap();
    let c = twin.dance.plan.calls[i];
    while twin.dance.now() < twin.dance.time_of(c.beat as i64) {
        frame(&mut h, &mut twin);
    }
    h.clear();
    let evs = press(&mut h, &mut twin, c.step);
    assert!(matches!(
        evs[0],
        Ev::Judged {
            judgment: Judgment::SpotOn,
            ..
        }
    ));
    assert!(played(&h, sound::SPOT_ON));
    assert!(played(&h, sound::step_key(c.step)));
    assert_eq!(h.spoken(), ["Spot on."]);
    h.clear();
    dance_next(&mut h, &mut twin);
    assert!(played(&h, sound::SPOT_ON));
    assert!(!spoke(&h, "Spot on."), "said the first time only");
}

#[test]
fn a_missed_step_stumbles_and_names_the_step() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    let c = twin.dance.plan.calls[0];
    let end = twin.dance.time_of(c.beat as i64) + 300;
    h.clear();
    while twin.dance.now() < end {
        frame(&mut h, &mut twin);
    }
    assert!(played(&h, sound::STUMBLE));
    assert!(spoke(
        &h,
        &format!("Missed. That was {}, {}.", c.step.lower(), c.step.dance())
    ));
}

#[test]
fn status_reads_the_tune_bar_score_and_streak() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    h.clear();
    h.press(Key::S);
    assert_eq!(h.spoken(), ["Night 1, 3 songs, not started."]);
    h.clear();
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        ["Space starts the dance. H plays every monster and its move."]
    );
    start(&mut h, &mut twin);
    dance_next(&mut h, &mut twin);
    dance_next(&mut h, &mut twin);
    h.clear();
    h.press(Key::S);
    let bar = twin.dance.bar();
    assert_eq!(
        h.spoken(),
        [format!(
            "The Graveyard Warm-up, song 1 of 3, bar {bar} of 32, score 6, streak 2."
        )]
    );
}

#[test]
fn repeat_and_hint_name_the_calls() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    h.clear();
    h.press(Key::R);
    assert_eq!(h.spoken(), ["No monster yet in this song."]);
    let c = twin.dance.plan.calls[0];
    h.clear();
    h.press(Key::H);
    let key = h.app.ctx.control_name(c.step.action());
    assert!(spoke(
        &h,
        &format!("Next: {}, {}, {key}, in", c.step.lower(), c.step.dance())
    ));
    while twin.dance.last_call.is_none() {
        frame(&mut h, &mut twin);
    }
    h.clear();
    h.press(Key::R);
    assert_eq!(
        h.spoken(),
        [format!(
            "Last up: {}, {}, {key}.",
            c.step.lower(),
            c.step.dance()
        )]
    );
    assert!(played(&h, sound::call_key(c.step)));
}

#[test]
fn an_unknown_key_says_so() {
    let (mut h, _) = floor(Mode::Set(1));
    h.clear();
    h.press(Key::Q);
    assert_eq!(h.spoken(), ["Not a dance key. F1 lists the controls."]);
}

#[test]
fn pausing_counts_back_in_on_three_beats() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    wait(&mut h, &mut twin, 5.0);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    twin.count_in();
    assert!(spoke(&h, "Back on the floor. Three knocks in."));
    let before = twin.dance.now();
    wait(&mut h, &mut twin, 2.5);
    assert_eq!(played_count(&h, sound::COUNT), 3);
    assert!(twin.dance.now() > before);
    // Still in step with the twin: the next call dances spot on.
    h.clear();
    dance_next(&mut h, &mut twin);
    assert!(played(&h, sound::SPOT_ON));
}

#[test]
fn pause_leaves_to_the_title() {
    let (mut h, _) = floor(Mode::Set(1));
    h.press(Key::Escape);
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn a_whole_set_danced_well_earns_stars_and_opens_the_next() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    dance_all(&mut h, &mut twin);
    assert!(twin.results.iter().all(|r| r.stars == 3));
    wait(&mut h, &mut twin, END_PAUSE + 0.5);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "Night 1, the graveyard gate, danced."));
    assert!(spoke(&h, "Night 2, the crypt is open."));
    read_pages(&mut h);
    assert!(spoke(&h, "Longest streak: "));
    assert!(
        !spoke(&h, "More stars than ever"),
        "a first score already read every song's stars"
    );
    assert_eq!(h.app.top_name(), "results");
    let r = rows(&h);
    assert_eq!(r[0], "Next night: night 2, the crypt");
    assert_eq!(r[1], "Play again");
    assert_eq!(r.last().map(String::as_str), Some("Back to the title"));
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.stars_for("warm-up"), 3);
    assert!(p.set_open(2));
    assert!(!p.set_open(3));
    assert_eq!(p.tunes_danced, 3);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, &progress::set_board(1)),
        Some(twin.score() as u64)
    );
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(rows(&h)[0], "Play, night 2, the crypt");
}

#[test]
fn a_set_danced_badly_still_ends_with_results() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    for _ in 0..(200.0 / DT) as usize {
        frame(&mut h, &mut twin);
        if twin.finished() {
            break;
        }
    }
    wait_off_floor(&mut h);
    read_pages(&mut h);
    assert!(spoke(&h, "A song earns its first star at half the points"));
    assert_eq!(rows(&h)[0], "Play again");
    assert!(!Progress::load(&mut h.app.ctx).set_open(2));
}

#[test]
fn set_select_names_why_a_set_is_locked() {
    let mut h = title();
    goto(&mut h, "Night select");
    h.press(Key::Return);
    assert_eq!(
        rows(&h),
        [
            "Night 1, the graveyard gate, 0 of 9 stars",
            "Night 2, the crypt, locked",
            "Night 3, the old church, locked",
            "Night 4, Doctor Volt's laboratory, locked",
            "Back"
        ]
    );
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken(),
        ["Earn a star on every song of night 1 to open the crypt."]
    );
    h.press(Key::Up);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), PLAY_NAME);
}

#[test]
fn the_practice_room_waits_and_names_the_key() {
    let mut h = title();
    goto(&mut h, "Practice room");
    h.press(Key::Return);
    assert_eq!(rows(&h)[0], "The Graveyard Warm-up, no stars");
    assert_eq!(rows(&h)[3], "The Cobweb Crawl, locked");
    let (mut h, mut twin) = floor(Mode::Practice(1));
    assert!(spoke(
        &h,
        "The song waits for each monster until you dance its move."
    ));
    start(&mut h, &mut twin);
    while twin.dance.waiting().is_none() {
        frame(&mut h, &mut twin);
    }
    let i = twin.dance.waiting().unwrap();
    let step = twin.dance.plan.calls[i].step;
    let key = h.app.ctx.control_name(step.action());
    assert!(spoke(
        &h,
        &format!("{}, {}: {key}.", step.word(), step.dance())
    ));
    h.clear();
    press(&mut h, &mut twin, wrong_for(step));
    assert!(spoke(
        &h,
        &format!(
            "Not {}. {}, {}: {key}.",
            wrong_for(step).dance(),
            step.word(),
            step.dance()
        )
    ));
    h.clear();
    press(&mut h, &mut twin, step);
    assert!(played(&h, sound::step_key(step)));
    dance_all(&mut h, &mut twin);
    wait_off_floor(&mut h);
    assert!(spoke(&h, "Practice done: Boneyard Boogie."));
    read_pages(&mut h);
    assert_eq!(rows(&h)[0], "Practise again");
    assert!(Progress::load(&mut h.app.ctx).practice_steps > 0);
}

#[test]
fn spoken_call_outs_speak_each_call() {
    let mut h = title();
    goto(&mut h, "Settings");
    h.press(Key::Return);
    h.press(Key::Return); // the game's own category
    goto(&mut h, "Spoken call-outs");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Spoken call-outs: on."));
    assert!(rows(&h).contains(&"Spoken call-outs: on".to_string()));
    let mut run = twin(&mut h.app.ctx, Mode::Set(1));
    let f = graveyard_shuffle::Floor::with_seed(&mut h.app.ctx, Mode::Set(1), SEED);
    h.app.ctx.push(f);
    h.frame(0.0);
    start(&mut h, &mut run);
    h.clear();
    while run.dance.last_call.is_none() {
        frame(&mut h, &mut run);
    }
    let c = run.dance.plan.calls[0];
    assert!(h.spoken().contains(&c.step.word().to_string()));
}

#[test]
fn the_sound_check_plays_every_row() {
    let mut h = title();
    goto(&mut h, "Sound check");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), SOUNDS_NAME);
    let n = rows(&h).len();
    assert_eq!(rows(&h)[0], "Zombie");
    assert!(!rows(&h).iter().any(|row| row == "A song"));
    assert_eq!(rows(&h).last().map(String::as_str), Some("Back"));
    h.press(Key::Home);
    for _ in 0..n - 1 {
        h.clear();
        h.press(Key::Return);
        assert!(!h.played().is_empty(), "{}", cursor_row(&h));
        assert!(h.spoken().is_empty(), "{}", cursor_row(&h));
        h.press(Key::Down);
    }
    h.press(Key::Home);
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken().is_empty());
    assert!(!h.played().is_empty());
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn every_tune_has_its_parts_rendered() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    for t in SONGS.iter() {
        for b in [false, true] {
            let key = t.part_key(b);
            assert!(root.join(format!("{key}.ogg")).exists(), "{key}");
        }
    }
    for s in Monster::ALL {
        assert!(root.join(format!("{}.ogg", sound::call_key(s))).exists());
        assert!(root.join(format!("{}.ogg", sound::step_key(s))).exists());
    }
    for (key, _) in sound::catalog() {
        assert!(root.join(format!("{key}.ogg")).exists(), "{key}");
    }
}

#[test]
fn records_read_stars_and_tallies() {
    let mut h = title();
    goto(&mut h, "Records");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Records. Stars: 0 of 36. Nights open: 1 of 4."));
    assert_eq!(NIGHTS.len(), 4);
}

#[test]
fn each_monster_is_named_once_per_save() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    let first = twin.plans[0].calls[0].step;
    let line = format!(
        "{}, {}.",
        first.word(),
        h.app.ctx.control_name(first.action())
    );
    while twin.dance.last_call.is_none() {
        frame(&mut h, &mut twin);
    }
    assert!(spoke(&h, &line));
    // The next night on the same save leaves it unsaid.
    let mut again = crate::common::twin(&mut h.app.ctx, Mode::Set(1));
    let f = graveyard_shuffle::Floor::with_seed(&mut h.app.ctx, Mode::Set(1), SEED);
    h.app.ctx.push(f);
    h.frame(0.0);
    h.clear();
    start(&mut h, &mut again);
    while again.dance.last_call.is_none() {
        frame(&mut h, &mut again);
    }
    assert!(!spoke(&h, &line), "{:?}", h.spoken());
}

#[test]
fn a_new_monster_is_named_before_the_dance_and_not_again() {
    let (mut h, mut twin) = floor(Mode::Set(2));
    let key = h.app.ctx.control_name(Monster::Vampire.action());
    assert!(
        spoke(&h, &format!("New tonight: the vampire, {key}.")),
        "{:?}",
        h.spoken()
    );
    start(&mut h, &mut twin);
    let vampire_called = |twin: &graveyard_shuffle::rules::Run| {
        twin.dance
            .last_call
            .is_some_and(|i| twin.dance.plan.calls[i].step == Monster::Vampire)
    };
    while !vampire_called(&twin) {
        assert!(
            dance_next(&mut h, &mut twin),
            "no vampire in the first song"
        );
    }
    assert!(
        !spoke(&h, &format!("Vampire, {key}.")),
        "named twice: {:?}",
        h.spoken()
    );
    assert!(progress::met(&mut h.app.ctx).contains("Vampire"));
    // Once met, a later visit has no news.
    let f = graveyard_shuffle::Floor::with_seed(&mut h.app.ctx, Mode::Set(2), SEED);
    h.clear();
    h.app.ctx.push(f);
    h.frame(0.0);
    assert!(!spoke(&h, "New tonight"), "{:?}", h.spoken());
}

#[test]
fn practice_names_each_move_once_and_does_not_count_as_meeting() {
    let (mut h, mut twin) = floor(Mode::Practice(0));
    start(&mut h, &mut twin);
    while twin.dance.waiting().is_none() {
        frame(&mut h, &mut twin);
    }
    let step = twin.dance.plan.calls[twin.dance.waiting().unwrap()].step;
    let key = h.app.ctx.control_name(step.action());
    assert!(spoke(
        &h,
        &format!("{}, {}: {key}.", step.word(), step.dance())
    ));
    assert!(!spoke(&h, &format!("{}, {key}.", step.word())));
    assert!(progress::met(&mut h.app.ctx).is_empty());
}

#[test]
fn help_mid_dance_pauses_the_song_and_names_only_this_nights_monsters() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    wait(&mut h, &mut twin, 5.0);
    h.clear();
    h.press(Key::F1);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(
        &h,
        "Move on the beat after each call. Zombie, the Down arrow; skeleton, the Up arrow; mummy, Space."
    ));
    assert!(!spoke(&h, "vampire"));
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    assert!(spoke(&h, "Back on the floor. Three knocks in."));
}
