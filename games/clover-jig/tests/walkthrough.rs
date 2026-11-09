//! The game through its screens, as a player would meet it.

use crate::common::*;
use clover_jig::play::{Mode, END_PAUSE, HINT_GAP, PLAY_NAME};
use clover_jig::progress::{self, Progress};
use clover_jig::rules::{Ev, Judgment, Luck, Phase, RunEv};
use clover_jig::soundcheck::SOUNDS_NAME;
use clover_jig::tunes::{Step, CEILI_TUNES, SETS, TUNES};
use clover_jig::{sound, ID, TITLE_NAME};
use hearken::{Headless, Key};
use parlor_core::testkit::every_action_answers;
use parlor_core::Records;

#[test]
fn the_title_announces_itself_and_plays_its_theme() {
    let h = title();
    assert_eq!(
        h.spoken()[1..],
        ["Clover Jig.", "Play, set 1, the kitchen. 1 of 12."]
    );
    assert!(played(&h, sound::TITLE_MUSIC));
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(clover_jig::INFO.title_name(), TITLE_NAME);
}

#[test]
fn the_title_follows_the_standard_order() {
    let h = title();
    assert_eq!(
        rows(&h),
        [
            "Play, set 1, the kitchen",
            "Set select",
            "Ceili night, locked",
            "Challenges, daily not played yet",
            "Practice room",
            "Tutorial",
            "Help",
            "Sound check",
            "Records",
            "Keepsakes",
            "Settings",
            "Back to the parlor",
        ]
    );
}

#[test]
fn a_first_visit_starts_on_the_tutorial() {
    let mut h = Headless::new(data(PLAIN_DAY), Base);
    h.app.ctx.push(clover_jig::title());
    h.frame(0.0);
    assert_eq!(
        h.spoken()[1..],
        [
            "Clover Jig. New here? Start with the tutorial.",
            "Tutorial. 6 of 12."
        ]
    );
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "You dance at a village ceili, and a caller calls your steps."
    ));
    assert_eq!(h.app.top_name(), "lesson");
}

#[test]
fn the_tutorial_teaches_in_the_practice_room() {
    use clover_jig::tutorial;
    let mut h = title();
    let room = tutorial::room(&mut h.app.ctx);
    let lesson = tutorial::lesson_on(&mut h.app.ctx, room.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    assert!(spoke(&h, "Space starts the dance"));
    h.press(Key::Space);
    let mut counted_in = false;
    for _ in 0..20_000 {
        if h.app.top_name() != "lesson" {
            break;
        }
        // The live step holds the tune while its line is said.
        if !counted_in && spoke(&h, "4 steps danced.") {
            counted_in = true;
            assert!(room.with(|f| f.run.dance.counting_in()));
        }
        // Waiting steps at once; on the moving tune, each step on its beat.
        let due = room.with(|f| {
            let d = &f.run.dance;
            if let Some(i) = d.waiting() {
                return Some(d.plan.calls[i].step);
            }
            (0..d.plan.calls.len())
                .find(|&i| {
                    let off = d.time_of(d.plan.calls[i].beat as i64) - d.now();
                    d.judgment(i).is_none() && (0..=(DT * 1000.0) as i64).contains(&off)
                })
                .map(|i| d.plan.calls[i].step)
        });
        h.frame(DT);
        if let Some(step) = due {
            h.press(key_for(step));
        }
    }
    assert!(spoke(&h, "Here comes the tune."));
    assert!(spoke(&h, "4 steps danced."));
    assert!(spoke(
        &h,
        "Now the tune goes on, as in a real dance, after three drum beats."
    ));
    assert!(spoke(&h, "That bright chime is 3 points."));
    assert!(!spoke(&h, "Missed."), "no call is lost under the line");
    assert!(
        spoke(&h, "Tutorial done. Before a dance, H plays every step with its call. Play starts the first set."),
        "{:?}",
        &h.spoken()[h.spoken().len().saturating_sub(8)..]
    );
    assert_eq!(h.app.top_name(), TITLE_NAME);
}

#[test]
fn every_title_row_has_its_own_help() {
    for date in [PLAIN_DAY, PATRICKS_DAY] {
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
fn ceili_night_is_locked_with_a_countdown_the_rest_of_the_year() {
    let mut h = title_on(parlor_core::Date::new(2026, 10, 22));
    goto(&mut h, "Ceili night");
    h.clear();
    h.press(Key::F1);
    assert_eq!(
        h.spoken(),
        ["The village ceili opens on Saint Patrick's Day, March 17. Saint Patrick's Day is in 146 days."]
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert!(spoke(&h, "is in 146 days."));
}

#[test]
fn shamrock_week_names_itself_on_the_play_row() {
    let mut h = title_on(SHAMROCK_DAY);
    assert_eq!(rows(&h)[2], "Ceili night, locked");
    h.press(Key::Home);
    h.clear();
    h.press(Key::F1);
    assert!(spoke(
        &h,
        "Shamrock week: shamrocks hide in the tunes for 7 more days."
    ));
}

#[test]
fn play_opens_the_floor_with_the_intro_and_bed() {
    let mut h = title();
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), PLAY_NAME);
    assert!(spoke(
        &h,
        "Set 1, the kitchen. 3 tunes: The Kettle on the Hob, The Turf Fire, Granny's Rocking Chair. Space starts the dance. H plays every step."
    ));
    assert!(spoke(
        &h,
        "The Left arrow and the Right arrow dance left and right"
    ));
    assert!(played(&h, sound::BED_KITCHEN));
}

#[test]
fn before_the_dance_each_step_key_plays_its_call() {
    let (mut h, _) = floor(Mode::Set(1));
    for step in [Step::Left, Step::Right, Step::Turn] {
        h.clear();
        h.press(key_for(step));
        assert!(played(&h, sound::call_key(step)), "{step:?}");
        assert!(played(&h, sound::step_key(step)), "{step:?}");
    }
    h.clear();
    h.press(Key::Left);
    assert_eq!(h.spoken(), ["Left: the Left arrow."]);
    h.clear();
    h.press(Key::Tab);
    assert_eq!(h.spoken(), ["Turn: Tab. Not called in these tunes."]);
}

#[test]
fn hint_before_the_dance_plays_every_step_with_its_key() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    h.clear();
    h.press(Key::H);
    assert_eq!(
        h.spoken(),
        ["Left, the Left arrow. Right, the Right arrow. Hop, Space."]
    );
    wait(&mut h, &mut twin, HINT_GAP * 3.0);
    for s in [Step::Left, Step::Right, Step::Hop] {
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
    assert!(spoke(&h, "Here we go. Four beats in."));
    let evs = wait(&mut h, &mut twin, 4.0);
    assert_eq!(played_count(&h, sound::COUNT), 4);
    let first = TUNES[0].part_key(false);
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
    assert!(h.spoken().is_empty(), "said the first time only");
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
    assert!(spoke(&h, &format!("Missed. That was {}.", c.step.lower())));
}

#[test]
fn status_reads_the_tune_bar_score_and_streak() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    h.clear();
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        ["Set 1, the kitchen, 3 tunes, not started. Space starts the dance. H plays every step."]
    );
    start(&mut h, &mut twin);
    dance_next(&mut h, &mut twin);
    dance_next(&mut h, &mut twin);
    h.clear();
    h.press(Key::S);
    let bar = twin.dance.bar();
    assert_eq!(h.spoken(), [format!("Bar {bar} of 32, score 6, streak 2.")]);
    // A second S says something new: the tune and how the steps went.
    h.clear();
    h.press(Key::S);
    let said = h.spoken().join(" ");
    assert!(
        said.starts_with("The Kettle on the Hob, tune 1 of 3, ") && said.contains("missed."),
        "{said}"
    );
}

#[test]
fn f1_in_a_dance_stops_the_tune_and_counts_back_in() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    dance_next(&mut h, &mut twin);
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "The tune is stopped."), "{:?}", h.spoken());
    assert_eq!(h.app.top_name(), "pages");
    // Nothing is missed while the help is open.
    for _ in 0..(10.0 / DT) as usize {
        h.frame(DT);
    }
    assert!(!spoke(&h, "Missed."));
    h.press(Key::Escape);
    assert!(spoke(&h, "Three beats in."), "{:?}", h.spoken());
}

#[test]
fn repeat_and_hint_name_the_calls() {
    let (mut h, mut twin) = floor(Mode::Set(1));
    start(&mut h, &mut twin);
    h.clear();
    h.press(Key::R);
    assert_eq!(h.spoken(), ["No call yet in this tune."]);
    let c = twin.dance.plan.calls[0];
    h.clear();
    h.press(Key::H);
    let key = h.app.ctx.control_name(c.step.action());
    assert!(spoke(&h, &format!("Next: {}, {key}, in", c.step.lower())));
    while twin.dance.last_call.is_none() {
        frame(&mut h, &mut twin);
    }
    h.clear();
    h.press(Key::R);
    assert_eq!(
        h.spoken(),
        [format!("Last call: {}, {key}.", c.step.lower())]
    );
    assert!(played(&h, sound::call_key(c.step)));
}

#[test]
fn an_unknown_key_stays_silent() {
    let (mut h, _) = floor(Mode::Set(1));
    h.clear();
    h.press(Key::Q);
    assert!(h.spoken().is_empty(), "{:?}", h.spoken());
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
    assert!(spoke(&h, "Back on the floor. Three beats in."));
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
    assert!(spoke(&h, "Set 1, the kitchen danced."));
    assert!(spoke(&h, "Set 2, the crossroads is open."));
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "results");
    let r = rows(&h);
    assert_eq!(r[0], "Next set: set 2, the crossroads");
    assert_eq!(r[1], "Play again");
    assert_eq!(r.last().map(String::as_str), Some("Back to the title"));
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.stars_for("kettle"), 3);
    assert!(p.set_open(2));
    assert!(!p.set_open(3));
    assert_eq!(p.tunes_danced, 3);
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, &progress::set_board(1)),
        Some(twin.score() as u64)
    );
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME);
    assert_eq!(rows(&h)[0], "Play, set 2, the crossroads");
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
    assert!(spoke(&h, "A tune earns its first star at half the points"));
    assert_eq!(rows(&h)[0], "Play again");
    assert!(!Progress::load(&mut h.app.ctx).set_open(2));
}

#[test]
fn set_select_names_why_a_set_is_locked() {
    let mut h = title();
    goto(&mut h, "Set select");
    h.press(Key::Return);
    assert_eq!(
        rows(&h),
        [
            "Set 1, the kitchen, 0 of 9 stars",
            "Set 2, the crossroads, locked",
            "Set 3, the market square, locked",
            "Set 4, the harbour hall, locked",
            "Back"
        ]
    );
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken(),
        ["Earn a star on every tune of set 1 to open the crossroads."]
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
    assert_eq!(rows(&h)[0], "The Kettle on the Hob, a jig, no stars");
    assert_eq!(rows(&h)[3], "The Crossroads Jig, a jig, locked");
    let (mut h, mut twin) = floor(Mode::Practice(1));
    assert!(spoke(&h, "The tune waits at each step until you dance it."));
    start(&mut h, &mut twin);
    while twin.dance.waiting().is_none() {
        frame(&mut h, &mut twin);
    }
    let i = twin.dance.waiting().unwrap();
    let step = twin.dance.plan.calls[i].step;
    let key = h.app.ctx.control_name(step.action());
    assert!(spoke(&h, &format!("{}: {key}.", step.word())));
    h.clear();
    press(&mut h, &mut twin, wrong_for(step));
    assert!(spoke(
        &h,
        &format!("Not {}. {}: {key}.", wrong_for(step).lower(), step.word())
    ));
    h.clear();
    press(&mut h, &mut twin, step);
    assert!(played(&h, sound::step_key(step)));
    dance_all(&mut h, &mut twin);
    wait_off_floor(&mut h);
    assert!(spoke(&h, "Practice done: The Turf Fire."));
    read_pages(&mut h);
    assert_eq!(rows(&h)[0], "Practise again");
    assert!(Progress::load(&mut h.app.ctx).practice_steps > 0);
}

#[test]
fn shamrock_week_hides_shamrocks_that_give_luck() {
    let (mut h, mut twin) = floor_on(SHAMROCK_DAY, Mode::Set(1));
    assert!(spoke(&h, "Shamrocks hide in the tunes:"));
    assert!(twin.plans.iter().all(|p| p.shamrocks() >= 2));
    start(&mut h, &mut twin);
    let first = twin
        .dance
        .plan
        .calls
        .iter()
        .position(|c| c.luck == Luck::Shamrock)
        .unwrap();
    let mut heard_chime = false;
    for _ in 0..=first {
        h.clear();
        dance_next(&mut h, &mut twin);
        heard_chime |= played(&h, sound::LUCKY);
    }
    assert!(heard_chime);
    assert!(spoke(&h, "Shamrock. 1 luck point."));
    dance_all(&mut h, &mut twin);
    wait_off_floor(&mut h);
    read_pages(&mut h);
    assert!(spoke(&h, "Shamrocks:"));
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.luck, twin.luck());
    assert!(p.luck >= 6);
}

#[test]
fn no_shamrocks_the_rest_of_the_year() {
    let (_, twin) = floor(Mode::Set(1));
    assert!(twin.plans.iter().all(|p| p.shamrocks() == 0));
}

#[test]
fn ceili_night_opens_on_the_day_with_the_band_and_the_clover() {
    let mut h = title_on(PATRICKS_DAY);
    assert_eq!(rows(&h)[2], "Ceili night, open tonight");
    goto(&mut h, "Ceili night");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(&h, "Ceili night. The village hall is full"));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), TITLE_NAME, "the briefing backs out");

    let (mut h, mut twin) = floor_on(PATRICKS_DAY, Mode::Ceili);
    assert!(spoke(
        &h,
        "Ceili night. The full band plays the set dance, 4 tunes"
    ));
    assert!(spoke(&h, "A four-leaf clover hides tonight"));
    assert!(played(&h, sound::BED_HALL));
    assert!(twin.plans.iter().any(|p| p.has_clover()));
    start(&mut h, &mut twin);
    wait(&mut h, &mut twin, 4.0);
    assert!(played(&h, &CEILI_TUNES[0].part_key(false)));
    dance_all(&mut h, &mut twin);
    assert!(twin.clover());
    wait_off_floor(&mut h);
    assert!(spoke(&h, "You found the four-leaf clover."));
    read_pages(&mut h);
    assert!(spoke(&h, "Your keepsake for Saint Patrick's Day 2027"));
    assert_eq!(h.app.top_name(), "results");
    let p = Progress::load(&mut h.app.ctx);
    let k = p.keepsakes[&2027];
    assert_eq!(k.nights, 1);
    assert!(k.clover);
    assert_eq!(k.best, twin.score());
    assert_eq!(p.clovers, 1);

    // Once a night: the next dance tonight has no clover.
    let mut h2 = h;
    let again = twin_for_night(&mut h2);
    assert!(again.plans.iter().all(|p| !p.has_clover()));
}

fn twin_for_night(h: &mut Headless<parlor_core::Parlor>) -> clover_jig::rules::Run {
    twin(&mut h.app.ctx, Mode::Ceili)
}

#[test]
fn the_clover_comes_back_the_next_night() {
    let mut h = title_on(PATRICKS_DAY);
    progress::clover_called(&mut h.app.ctx, "2027-03-17");
    assert!(!progress::clover_due(&mut h.app.ctx, "2027-03-17"));
    assert!(progress::clover_due(&mut h.app.ctx, "2027-03-18"));
}

#[test]
fn the_figures_are_named_between_the_ceili_tunes() {
    let (mut h, mut twin) = floor_on(PATRICKS_DAY, Mode::Ceili);
    start(&mut h, &mut twin);
    for _ in 0..(60.0 / DT) as usize {
        let evs = frame(&mut h, &mut twin);
        if evs.contains(&RunEv::TuneDone(0)) {
            break;
        }
    }
    assert!(spoke(
        &h,
        "Next figure, the swing: The Lucky Penny, a reel."
    ));
    assert!(matches!(twin.phase, Phase::Between(_)));
    h.clear();
    h.press(Key::S);
    assert!(spoke(&h, "Ceili night, tune 1 of 4 done"));
}

#[test]
fn keepsakes_count_down_when_empty() {
    let mut h = title_on(parlor_core::Date::new(2026, 10, 22));
    goto(&mut h, "Keepsakes");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "Keepsakes. 0 luck points, from 0 shamrocks heard."
    ));
    h.press(Key::Return);
    assert!(spoke(&h, "No ceili keepsakes yet."));
    assert!(spoke(&h, "Saint Patrick's Day is in 146 days."));
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
    let f = clover_jig::Floor::with_seed(&mut h.app.ctx, Mode::Set(1), SEED);
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
    assert_eq!(rows(&h)[0], "Left");
    assert!(!rows(&h).iter().any(|row| row == "A tune"));
    assert!(!rows(&h).iter().any(|row| row == "The ceili band"));
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
    for t in TUNES.iter().chain(CEILI_TUNES.iter()) {
        for b in [false, true] {
            let key = t.part_key(b);
            assert!(root.join(format!("{key}.ogg")).exists(), "{key}");
        }
    }
    for s in Step::ALL {
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
    assert!(spoke(&h, "Records. Stars: 0 of 36. Sets open: 1 of 4."));
    assert_eq!(SETS.len(), 4);
}

#[test]
fn help_names_only_the_steps_the_tunes_call() {
    let (mut h, _) = floor(Mode::Set(1));
    h.clear();
    h.press(Key::F1);
    let said = h.spoken().join(" ");
    assert!(
        said.contains("dance left and right, and Space hops."),
        "{said}"
    );
    assert!(
        !said.contains("turns") && !said.contains("forward"),
        "{said}"
    );

    let (mut h, _) = floor(Mode::Practice(0));
    h.press(Key::Space);
    for _ in 0..400 {
        h.frame(DT);
    }
    h.clear();
    h.press(Key::F1);
    let said = h.spoken().join(" ");
    assert!(
        !said.contains("in a row add"),
        "nothing scores in practice: {said}"
    );
}
