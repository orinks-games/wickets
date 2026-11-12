//! Tide Pools driven the way a player drives it, by keys and frames.

use crate::common;

use common::{base, choose, played, read_pages, spoke, title, Twin, SEED};
use hearken::{Headless, Key};
use parlor_core::menus;
use parlor_core::{Challenge, Challenges, Parlor, Period, Records};
use tide_pools::outing::{challenge_outing, outing_for};
use tide_pools::rules::{self, Ev, Lean, Mode, Modifier, Place, Species};
use tide_pools::{sound, Aquarium, ID};

#[test]
fn the_title_announces_itself() {
    // On a quiet date, so no event row changes the count.
    let mut h = base();
    h.clear();
    h.app.ctx.push(tide_pools::title());
    h.frame(0.0);
    assert_eq!(
        h.spoken()[0],
        "Tide Pools. New here? Start with the tutorial."
    );
    assert_eq!(h.spoken()[1], "Tutorial. 6 of 12.");
}

#[test]
fn the_title_plays_its_theme_and_follows_the_standard() {
    let h = title();
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
    assert_eq!(
        menus::rows_of(&h.app.top_lines()),
        [
            "Play, Sandy Cove, not visited yet",
            "King tide, locked",
            "Moonlit pools, locked",
            "Challenges, daily not played yet",
            "Practice pools",
            "Tutorial",
            "Help",
            "Sound check",
            "Records",
            "Your aquarium, 0 of 14 kinds",
            "Settings",
            "Back to the parlor",
        ]
    );
}

#[test]
fn left_and_right_choose_the_shore_on_the_play_row() {
    let mut h = title();
    h.press(Key::Home);
    h.clear();
    h.press(Key::Right);
    assert_eq!(h.spoken(), ["Rocky Point, locked."]);
    h.clear();
    h.press(Key::Right);
    assert_eq!(h.spoken(), ["The Far Reef, locked."]);
    h.clear();
    h.press(Key::Right);
    assert_eq!(
        h.spoken(),
        ["The Far Reef, locked."],
        "it stops at the last shore"
    );
    h.clear();
    h.press(Key::F1);
    assert!(spoke(
        &h,
        "Gather 7 different kinds of creature to open the Far Reef."
    ));
    for _ in 0..3 {
        h.press(Key::Left);
    }
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "6 pools, about 3 minutes"), "{:?}", h.spoken());
    assert_eq!(
        menus::rows_of(&h.app.top_lines())[0],
        "Play, Sandy Cove, not visited yet"
    );
}

#[test]
fn back_to_the_parlor_leaves_the_game() {
    let mut h = title();
    h.press(Key::End);
    assert!(spoke(&h, "Back to the parlor"));
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "base");
    assert_eq!(h.app.ctx.audio.music(), None);
}

#[test]
fn locked_shores_say_how_to_open_them() {
    let mut h = title();
    h.press(Key::Home);
    h.press(Key::Right);
    h.clear();
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "Rocky Point opens once your aquarium holds 4 kinds"
    ));
    assert_eq!(h.app.top_name(), "tide_pools");
}

#[test]
fn the_tutorial_gathers_an_anemone_and_a_shy_crab() {
    let mut h = title();
    choose(&mut h, "Tutorial");
    assert_eq!(h.app.top_name(), "lesson");
    assert!(spoke(
        &h,
        "The tide is out, and rock pools line the shore with the sea ahead of you."
    ));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "tide_pools");

    rules::fix_seed(Some(SEED));
    let pools = tide_pools::tutorial::pools();
    let lesson = tide_pools::tutorial::lesson_on(&mut h.app.ctx, pools.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    rules::fix_seed(Some(SEED));
    let mut t = Twin::new(h, outing_for(Mode::Practice));
    t.lean_in();
    assert!(
        !spoke(&t.h, "Leaning in, over the first pool."),
        "{:?}",
        t.h.spoken()
    );
    t.gather(Species::Anemone);
    assert!(spoke(&t.h, "The anemone is yours."), "{:?}", t.h.spoken());
    let said = t.h.spoken().join(" ");
    assert!(!said.contains("Plus"), "{said}");
    assert!(!said.contains("New for your aquarium"), "{said}");
    t.press(Key::Right);
    t.lean_in();
    loop {
        t.frame(0.05);
        if t.evs.iter().any(|e| matches!(e, Ev::Emerge { .. })) {
            break;
        }
    }
    t.gather(Species::Crab);
    assert!(
        spoke(&t.h, "The crab is yours. Tutorial done."),
        "{:?}",
        t.h.spoken()
    );
    assert!(
        spoke(&t.h, "These practice creatures go back in the water."),
        "{:?}",
        t.h.spoken()
    );
    assert_eq!(t.h.app.top_name(), "tide_pools");
    assert_eq!(pools.with(|s| s.outing.gathered.len()), 2);
    assert_eq!(Aquarium::load(&mut t.h.app.ctx).kinds(), 0);
    // The step taught the neighbouring pools, so the side-sound line is
    // kept for the first real trip.
    let key = format!("{ID}/told_neighbours");
    assert!(!Records::load::<bool>(&mut t.h.app.ctx, &key));
}

#[test]
fn the_full_guide_opens_by_section() {
    let mut h = title();
    choose(&mut h, "Help");
    assert_ne!(h.app.top_name(), "tide_pools");
    assert!(spoke(&h, "Tide Pools"));
}

#[test]
fn the_sound_check_plays_every_sound_with_its_name() {
    let mut h = title();
    choose(&mut h, "Sound check");
    assert_eq!(h.app.top_name(), "tide_pools_sounds");
    let catalog = sound::catalog();
    let mut keys = Vec::new();
    for _ in 0..catalog.len() {
        h.clear();
        h.press(Key::Return);
        keys.extend(h.played());
        if let Some(m) = h.app.ctx.audio.music() {
            keys.push(m.to_string());
        }
        assert!(h.spoken().is_empty());
        h.press(Key::Down);
    }
    for (key, _, _) in &catalog {
        assert!(keys.iter().any(|k| k == key), "{key} was played");
    }
    for s in Species::ALL {
        assert!(keys.iter().any(|k| k == sound::creature(s)));
    }
    h.clear();
    h.press(Key::Return); // Left and right
    assert!(h.spoken().is_empty());
    assert!(played(&h, sound::creature(Species::Crab)));
    assert!(played(&h, sound::creature(Species::Periwinkle)));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "tide_pools");
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
}

#[test]
fn every_sound_the_game_plays_is_in_the_specs() {
    let spec = include_str!("../../../sound-specs/tide-pools.json");
    let mut keys: Vec<String> = sound::catalog()
        .into_iter()
        .map(|(k, _, _)| k.to_string())
        .collect();
    keys.push(sound::STEP_B.to_string());
    for key in keys {
        let path = key.strip_prefix("tide-pools/").unwrap();
        assert!(
            spec.contains(&format!("\"{path}.wav\"")),
            "{path} has a spec"
        );
    }
}

/// Opens the practice pools from the title and walks down to the water.
fn practice() -> Twin {
    let mut h = title();
    choose(&mut h, "Practice pools");
    assert!(spoke(&h, "The practice pools"));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "tide_pools_outing");
    rules::fix_seed(Some(SEED));
    Twin::new(h, outing_for(Mode::Practice))
}

#[test]
fn arriving_at_the_shore_explains_the_controls() {
    let t = practice();
    assert!(spoke(&t.h, "Pool 1 of 3, at the water's edge"));
    assert!(spoke(&t.h, "Up arrow leans in"), "{:?}", t.h.spoken());
    assert_eq!(t.h.app.ctx.audio.bed(), Some(sound::SHORE));
    assert_eq!(t.h.app.ctx.audio.music(), None);
}

#[test]
fn after_the_tutorial_a_trip_leaves_the_controls_to_f1() {
    let mut h = title();
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, tide_pools::ID);
    choose(&mut h, "Practice pools");
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "F1 lists the controls."), "{:?}", h.spoken());
    assert!(!spoke(&h, "Up arrow leans in"), "{:?}", h.spoken());
}

#[test]
fn walking_splashes_and_the_rocks_end() {
    let mut t = practice();
    t.h.clear();
    t.press(Key::Right);
    assert!(played(&t.h, sound::STEP_B) || played(&t.h, sound::STEP_A));
    assert_eq!(t.h.spoken(), ["Pool 2."]);
    t.press(Key::Right);
    assert_eq!(t.h.spoken().last().unwrap(), "Pool 3, the last.");
    t.h.clear();
    t.press(Key::Right);
    assert!(played(&t.h, sound::EDGE));
    assert!(spoke(&t.h, "No more pools to the right"));
}

#[test]
fn leaning_in_and_stepping_back_are_spoken_and_heard() {
    let mut t = practice();
    t.h.clear();
    t.press(Key::Up);
    assert!(played(&t.h, sound::LEAN_IN));
    assert!(spoke(&t.h, "Leaning in close. Only this pool now"));
    t.h.clear();
    t.press(Key::Up);
    assert!(spoke(&t.h, "Already leaning in"));
    t.press(Key::Down);
    t.h.clear();
    t.press(Key::Down);
    assert!(played(&t.h, sound::LEAN_BACK));
    assert!(spoke(
        &t.h,
        "Standing back on the rocks. Two pools either side"
    ));
    t.h.clear();
    t.press(Key::Down);
    assert!(spoke(&t.h, "Already standing back"));
}

#[test]
fn reaching_from_too_far_back_pats_the_rock() {
    let mut t = practice();
    t.h.clear();
    t.press(Key::Space);
    assert!(played(&t.h, sound::REACH));
    assert!(spoke(&t.h, "Out of reach. Lean in with the Up arrow."));
    t.h.clear();
    t.press(Key::R);
    assert!(spoke(&t.h, "Out of reach. Lean in with the Up arrow."));
}

#[test]
fn reaching_in_too_early_comes_up_with_water() {
    let mut t = practice();
    t.press(Key::Up);
    t.h.clear();
    t.press(Key::Space);
    assert!(played(&t.h, sound::EMPTY));
    assert!(spoke(&t.h, "Only water"));
}

#[test]
fn the_first_sound_of_each_creature_is_named() {
    let mut t = practice();
    t.press(Key::Up);
    t.h.clear();
    loop {
        t.frame(0.05);
        if t.evs
            .iter()
            .any(|e| matches!(e, Ev::Call { offset: 0, .. }))
        {
            break;
        }
    }
    assert!(played(&t.h, sound::creature(Species::Anemone)));
    assert!(
        spoke(&t.h, "A soft, low, wet pop: a sea anemone."),
        "{:?}",
        t.h.spoken()
    );
    t.h.clear();
    t.press(Key::Tab);
    assert!(spoke(&t.h, "That last sound was the sea anemone"));
}

#[test]
fn gathering_a_creature_by_ear() {
    let mut t = practice();
    t.press(Key::Up);
    t.h.clear();
    let evs = t.gather(Species::Anemone);
    assert!(matches!(evs[0], Ev::Gathered { .. }), "{evs:?}");
    assert!(played(&t.h, sound::GATHER));
    assert!(played(&t.h, sound::NEW));
    assert!(spoke(
        &t.h,
        "A sea anemone. 5 points. New for your aquarium."
    ));
    t.h.clear();
    t.press(Key::S);
    assert!(spoke(&t.h, "Pool 1 of 3, leaning in close."));
    t.press(Key::S);
    assert!(spoke(&t.h, "1 creature gathered. Score 5."));
}

#[test]
fn a_shy_crab_hides_and_creeps_back_out() {
    let mut t = practice();
    t.press(Key::Right);
    t.h.clear();
    t.press(Key::Up);
    assert!(played(&t.h, sound::HIDE));
    assert!(spoke(&t.h, "Something shy hid"));
    t.h.clear();
    t.press(Key::H);
    assert!(spoke(&t.h, "Something shy is hiding"));
    t.h.clear();
    t.press(Key::Space);
    assert!(spoke(&t.h, "Too soon"));
    t.h.clear();
    loop {
        t.frame(0.05);
        if t.evs.iter().any(|e| matches!(e, Ev::Emerge { .. })) {
            break;
        }
    }
    assert!(played(&t.h, sound::EMERGE));
    assert!(spoke(&t.h, "Two rising notes"));
    t.gather(Species::Crab);
    assert!(spoke(&t.h, "A shore crab. 15 points."));
}

#[test]
fn the_hint_points_the_way() {
    let mut t = practice();
    t.h.clear();
    t.press(Key::H);
    assert!(spoke(
        &t.h,
        "Something lives here. Lean in with the Up arrow."
    ));
    t.press(Key::Up);
    t.gather(Species::Anemone);
    t.h.clear();
    t.press(Key::H);
    assert!(spoke(&t.h, "1 pool to your right"), "{:?}", t.h.spoken());
}

#[test]
fn f1_help_follows_where_you_stand() {
    let mut t = practice();
    t.h.clear();
    parlor_core::testkit::read_help(&mut t.h);
    assert!(spoke(&t.h, "You are at the water's edge"));
    t.press(Key::Up);
    t.h.clear();
    parlor_core::testkit::read_help(&mut t.h);
    assert!(spoke(&t.h, "You are leaning in"));
}

#[test]
fn escape_pauses_and_resume_comes_back() {
    let mut t = practice();
    t.press(Key::Escape);
    assert_eq!(t.h.app.top_name(), "pause");
    t.h.clear();
    t.h.press(Key::Return); // Resume
    assert_eq!(t.h.app.top_name(), "tide_pools_outing");
    assert!(spoke(&t.h, "Back at pool 1, at the water's edge."));
}

#[test]
fn pause_returns_to_the_title() {
    let mut t = practice();
    t.press(Key::Escape);
    for _ in 0..3 {
        t.h.press(Key::Down);
    }
    assert!(spoke(&t.h, "Back to the title"));
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "tide_pools");
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
}

#[test]
fn leaving_from_pause_keeps_what_was_gathered() {
    let mut t = practice();
    t.press(Key::Up);
    t.gather(Species::Anemone);
    t.press(Key::Escape);
    for _ in 0..3 {
        t.h.press(Key::Down);
    }
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "tide_pools");
    let aquarium = Aquarium::load(&mut t.h.app.ctx);
    assert_eq!(aquarium.kinds(), 1);
}

#[test]
fn finishing_the_practice_pools_fills_the_aquarium_and_opens_rocky_point() {
    let mut t = practice();
    t.press(Key::Up);
    t.gather(Species::Anemone);
    t.press(Key::Right);
    t.lean_in();
    t.gather(Species::Crab);
    t.press(Key::Right);
    t.lean_in();
    t.h.clear();
    // The third pool holds two: gather whichever sounds first, then the other.
    let first = loop {
        t.frame(0.05);
        if let Some(Ev::Call { species, .. }) = t
            .evs
            .iter()
            .rev()
            .find(|e| matches!(e, Ev::Call { offset: 0, .. }))
        {
            break *species;
        }
    };
    t.press(Key::Space);
    let other = if first == Species::Starfish {
        Species::Periwinkle
    } else {
        Species::Starfish
    };
    t.gather(other);
    assert!(t.o.finished);
    assert_eq!(t.h.app.top_name(), "pages");
    // The last find is said once, and the page waits behind it.
    let page = calls(&t.h)
        .into_iter()
        .find(|(l, _)| l.contains("Practice pools done."))
        .expect("the results page");
    assert!(page.0.starts_with("Practice pools done."), "{page:?}");
    assert!(!page.1, "the page queues behind the last find");
    assert!(spoke(&t.h, "You gathered 1 sea anemone, 1 shore crab"));
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::WIN));
    assert!(spoke(&t.h, "Score 35."));
    assert!(spoke(&t.h, ", 4 of 14."), "{:?}", t.h.spoken());
    assert!(spoke(&t.h, "Rocky Point is now open"));
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "results");
    t.h.press(Key::Escape);
    assert_eq!(t.h.app.top_name(), "tide_pools");
    t.h.press(Key::Home);
    t.h.clear();
    t.h.press(Key::Right);
    assert!(
        spoke(&t.h, "Rocky Point, not visited yet."),
        "{:?}",
        t.h.spoken()
    );
    let lines = t.h.app.top_lines().join("\n");
    assert!(lines.contains("Your aquarium, 4 of 14 kinds"));
    let tank = Aquarium::load(&mut t.h.app.ctx);
    assert_eq!(tank.count(Species::Crab), 1);
    assert_eq!(tank.outings, 1);
    assert!(Records::best(&mut t.h.app.ctx, ID, "cove").is_none());
}

/// Opens Sandy Cove from the title and walks down to the water.
fn cove() -> Twin {
    let mut h = title();
    h.press(Key::Home);
    h.press(Key::Return);
    assert!(spoke(&h, "Sandy Cove. A "), "{:?}", h.spoken());
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "tide_pools_outing");
    rules::fix_seed(Some(SEED));
    Twin::new(h, outing_for(Mode::Beach(Place::Cove)))
}

#[test]
fn a_trip_to_the_cove_ends_gently_with_the_tide_and_keeps_the_best() {
    let mut t = cove();
    assert_eq!(t.o.plan.place, Place::Cove);
    // Gather one creature that is not shy, wherever it is.
    let target =
        t.o.pools
            .iter()
            .flat_map(|p| p.finds.iter())
            .map(|f| f.species)
            .find(|s| s.shy().is_none())
            .expect("something calm on the shore");
    let evs = t.fetch(target);
    assert!(matches!(evs[0], Ev::Gathered { .. }), "{evs:?}");
    let score = t.o.score();
    assert!(score > 0);
    t.h.clear();
    t.wait_for_tide();
    assert!(spoke(&t.h, "The tide has turned"));
    assert!(spoke(&t.h, "Tide in about a minute"));
    assert!(played(&t.h, sound::TIDE_TURN));
    assert!(played(&t.h, sound::WAVE_CRASH));
    assert!(spoke(&t.h, "Tide's in."));
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::WIN), "a new kind");
    assert!(spoke(
        &t.h,
        &format!("Score {score}. Your first score at Sandy Cove.")
    ));
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "results");
    t.h.press(Key::End);
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "tide_pools");
    assert_eq!(Records::best(&mut t.h.app.ctx, ID, "cove"), Some(score));
    assert!(t
        .h
        .app
        .top_lines()
        .iter()
        .any(|l| l.contains(&format!("Play, Sandy Cove, best {score}"))));
}

#[test]
fn a_trip_with_nothing_new_walks_home_quietly() {
    let mut t = cove();
    t.wait_for_tide();
    assert!(spoke(&t.h, "Nothing gathered this time"));
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::HOME));
    t.h.press(Key::Return);
    assert!(spoke(&t.h, "Score 0, so nothing goes on the board."));
    read_pages(&mut t.h);
    assert_eq!(Records::best(&mut t.h.app.ctx, ID, "cove"), None);
}

#[test]
fn status_tells_the_tide_and_the_day() {
    let mut t = cove();
    t.h.clear();
    t.press(Key::S);
    t.press(Key::S);
    t.press(Key::S);
    let said = t.h.spoken().join(" ");
    assert!(said.contains("Pool 1 of 6, at the water's edge."), "{said}");
    assert!(said.contains("The tide returns in about "), "{said}");
    assert!(said.contains("Nothing gathered yet."));
    assert!(said.contains("Sandy Cove. A "), "{said}");
}

#[test]
fn the_waves_come_nearer_as_the_tide_returns() {
    let mut t = cove();
    let start = t.o.tide_in();
    t.h.clear();
    for _ in 0..100 {
        t.frame(1.0);
    }
    assert!(t.o.tide_in() > start);
    assert!(played(&t.h, sound::GULL) || played(&t.h, sound::GUST));
}

#[test]
fn the_aquarium_starts_empty() {
    let mut h = title();
    choose(&mut h, "Your aquarium");
    assert_eq!(h.app.top_name(), "tide_pools_aquarium");
    assert!(spoke(&h, "Your aquarium is empty"));
    h.clear();
    h.press(Key::Right);
    assert!(spoke(&h, "Your aquarium is empty"));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "tide_pools");
}

#[test]
fn the_aquarium_plays_what_you_have_gathered() {
    let mut h = title();
    let mut tank = Aquarium::default();
    tank.add(
        &[
            Species::Crab,
            Species::Crab,
            Species::Starfish,
            Species::Cowrie,
        ],
        "at the Far Reef on a sunny morning",
    );
    tank.outings = 1;
    tank.store(&mut h.app.ctx);
    choose(&mut h, "Your aquarium");
    assert!(spoke(
        &h,
        "Your aquarium holds 3 of 14 kinds, 4 creatures in all, from 1 trip."
    ));
    assert_eq!(h.app.ctx.audio.music(), None);
    h.clear();
    h.press(Key::Right);
    assert!(played(&h, sound::creature(Species::Starfish)));
    assert!(spoke(&h, "Starfish, 1 in the tank."), "{:?}", h.spoken());
    h.clear();
    h.press(Key::Left);
    assert!(spoke(&h, "Shore crab, 2 in the tank. A quick skitter of clicking claws. First found at the Far Reef on a sunny morning. 1 of 3."));
    h.clear();
    h.press(Key::Tab);
    assert!(spoke(
        &h,
        "The whole tank, from left to right: shore crab, starfish and cowrie shell."
    ));
    for _ in 0..60 {
        h.frame(0.1);
    }
    for s in [Species::Crab, Species::Starfish, Species::Cowrie] {
        assert!(played(&h, sound::creature(s)), "{s:?}");
    }
    h.clear();
    h.press(Key::H);
    assert!(spoke(
        &h,
        "You have not found a periwinkle shell yet. It lives at Sandy Cove"
    ));
    // It lives on: creatures call by themselves.
    h.clear();
    for _ in 0..200 {
        h.frame(0.1);
    }
    assert!(!h.played().is_empty());
}

#[test]
fn the_challenge_menu_lists_three_periods() {
    let mut h = title();
    choose(&mut h, "Challenges");
    assert_eq!(h.app.top_name(), "tide_pools_challenges");
    let lines = h.app.top_lines().join("\n");
    assert!(
        lines.contains("Daily, Tonight's three species: "),
        "{lines}"
    );
    assert!(lines.contains("Weekly, This week's rare find: the "));
    assert!(lines
        .contains("Monthly, The Great Low Tide: gather 8 kinds on the Far Reef. Not played yet"));
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "A new one tomorrow."));
}

/// Opens today's challenge of `period` from the stand-in launcher.
fn challenge(period: Period) -> Twin {
    let mut h = base();
    tide_pools::open_challenge(&mut h.app.ctx, period);
    h.frame(0.0);
    let c = Challenge::current(&h.app.ctx, ID, period);
    let o = challenge_outing(&c);
    Twin::new(h, o)
}

#[test]
fn the_daily_states_its_goal_and_twist_on_one_page() {
    let mut t = challenge(Period::Daily);
    assert_eq!(t.h.app.top_name(), "pages");
    let targets: Vec<&str> = t.o.plan.targets.iter().map(|s| s.name()).collect();
    assert!(spoke(&t.h, "Tonight's three species: find a "));
    assert!(spoke(&t.h, targets[0]));
    assert!(spoke(&t.h, &t.o.plan.modifier.describe()));
    assert!(spoke(&t.h, "Enter to start."));
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "tide_pools_outing");
}

#[test]
fn a_finished_daily_counts_its_targets_and_starts_a_streak() {
    let mut t = challenge(Period::Daily);
    read_pages(&mut t.h);
    let target = t.o.plan.targets[0];
    let evs = t.fetch(target);
    assert!(
        matches!(evs[0], Ev::Gathered { target: true, .. }),
        "{evs:?}"
    );
    assert!(spoke(&t.h, "One of tonight's three."));
    t.h.clear();
    t.press(Key::S);
    t.press(Key::S);
    assert!(spoke(&t.h, "Tonight's three:"));
    t.wait_for_tide();
    let score = t.o.score();
    assert!(score >= 100);
    t.h.press(Key::Return);
    assert!(
        spoke(&t.h, "Daily challenge: You found"),
        "{:?}",
        t.h.spoken()
    );
    assert!(spoke(&t.h, &format!("Score {score}")));
    assert!(spoke(&t.h, "Your first score on this one."));
    assert!(spoke(&t.h, "The first day of a streak."));
    assert!(spoke(&t.h, "A new one tomorrow."));
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "results");
    assert_eq!(
        menus::rows_of(&t.h.app.top_lines()),
        [
            "Play this daily challenge again",
            "Hear the results again",
            "Your aquarium",
            "Back to the challenges",
        ]
    );
    t.h.press(Key::Escape);
    assert_eq!(t.h.app.top_name(), "base");
    let c = Challenge::current(&t.h.app.ctx, ID, Period::Daily);
    assert_eq!(Challenges::best(&mut t.h.app.ctx, &c), Some(score));
    assert_eq!(Challenges::streak(&mut t.h.app.ctx, ID, Period::Daily), 1);
    assert!(Challenges::summary(&mut t.h.app.ctx, &c).contains("done, best"));
}

#[test]
fn the_weekly_rare_find_is_worth_300() {
    let mut t = challenge(Period::Weekly);
    assert!(spoke(&t.h, "This week's rare find, on the Far Reef:"));
    read_pages(&mut t.h);
    let Modifier::RareHunt(rare) = t.o.plan.modifier else {
        panic!("a rare hunt")
    };
    t.fetch(rare);
    assert!(spoke(&t.h, "A rare find."));
    assert!(spoke(&t.h, "This week's rare find."));
    assert!(played(&t.h, sound::RARE));
    t.wait_for_tide();
    assert!(t.o.score() >= 350);
    t.h.press(Key::Return);
    assert!(spoke(
        &t.h,
        &format!("You found this week's {}.", rare.name())
    ));
    assert!(spoke(&t.h, "A new one in 3 days."));
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::WIN));
}

#[test]
fn the_monthly_is_the_great_low_tide() {
    let t = challenge(Period::Monthly);
    assert!(spoke(&t.h, "15 pools, about 7 minutes."));
    assert_eq!(t.o.plan.modifier, Modifier::GreatLowTide);
    assert_eq!(t.o.pools.len(), 15);
}

#[test]
fn leaving_a_challenge_from_pause_does_not_count() {
    let mut t = challenge(Period::Daily);
    read_pages(&mut t.h);
    t.press(Key::Escape);
    for _ in 0..3 {
        t.h.press(Key::Down);
    }
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "base");
    let c = Challenge::current(&t.h.app.ctx, ID, Period::Daily);
    assert_eq!(Challenges::best(&mut t.h.app.ctx, &c), None);
    assert_eq!(Challenges::streak(&mut t.h.app.ctx, ID, Period::Daily), 0);
}

#[test]
fn challenges_from_the_title_return_to_the_list() {
    let mut h = title();
    choose(&mut h, "Challenges");
    h.press(Key::Return); // Daily
    assert_eq!(h.app.top_name(), "pages");
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "tide_pools_outing");
    let c = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let mut t = Twin::new(h, challenge_outing(&c));
    t.wait_for_tide();
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "results");
    t.h.press(Key::End);
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "tide_pools_challenges");
    assert!(t
        .h
        .app
        .top_lines()
        .iter()
        .any(|l| l.starts_with("> Daily, ") && l.contains("Done, best")));
}

#[test]
fn the_neighbouring_pools_are_heard_from_the_side() {
    let mut t = cove();
    t.press(Key::Down);
    t.h.clear();
    let mut side = false;
    for _ in 0..600 {
        t.frame(0.05);
        if t.evs
            .iter()
            .any(|e| matches!(e, Ev::Call { offset, lean: Lean::Back, .. } if *offset != 0))
        {
            side = true;
            break;
        }
    }
    assert!(side, "something called from a neighbouring pool");
    assert!(spoke(&t.h, "Side sounds are the next pools."));
}

/// Main-channel speech since the last clear, with whether each interrupts.
fn calls(h: &Headless<Parlor>) -> Vec<(String, bool)> {
    h.speech
        .borrow()
        .calls(hearken::speech::SpeechChannel::Main)
}

#[test]
fn creeping_out_never_cuts_off_the_hiding_lesson() {
    let mut t = practice();
    t.press(Key::Right);
    t.lean_in();
    t.h.clear();
    loop {
        t.frame(0.05);
        if t.evs.iter().any(|e| matches!(e, Ev::Emerge { .. })) {
            break;
        }
    }
    let line = calls(&t.h)
        .into_iter()
        .find(|(l, _)| l.contains("Two rising notes"))
        .expect("the emerge lesson");
    assert!(!line.1, "the emerge lesson must queue, not interrupt");
}

#[test]
fn repeat_says_where_you_walked_and_leaned() {
    let mut t = practice();
    t.press(Key::Right);
    t.h.clear();
    t.press(Key::R);
    assert!(spoke(&t.h, "Pool 2."), "{:?}", t.h.spoken());
    t.press(Key::Down);
    t.h.clear();
    t.press(Key::R);
    assert!(spoke(&t.h, "Standing back on the rocks."));
    t.press(Key::Left);
    t.press(Key::Left);
    t.h.clear();
    t.press(Key::R);
    assert!(spoke(&t.h, "No more pools to the"));
}

#[test]
fn the_last_find_is_kept_when_the_pools_are_cleared() {
    let mut t = practice();
    t.press(Key::Up);
    t.gather(Species::Anemone);
    t.press(Key::Right);
    t.lean_in();
    t.gather(Species::Crab);
    t.press(Key::Right);
    t.lean_in();
    let first = loop {
        t.frame(0.05);
        if let Some(Ev::Call { species, .. }) = t
            .evs
            .iter()
            .rev()
            .find(|e| matches!(e, Ev::Call { offset: 0, .. }))
        {
            break *species;
        }
    };
    t.press(Key::Space);
    let other = if first == Species::Starfish {
        Species::Periwinkle
    } else {
        Species::Starfish
    };
    t.h.clear();
    t.gather(other);
    assert_eq!(t.h.app.top_name(), "pages");
    // The last find is said once, and the page queues behind it rather
    // than cutting it off or saying it again.
    let calls = calls(&t.h);
    let find = format!("A {}.", other.name());
    assert_eq!(
        calls.iter().filter(|(l, _)| l.contains(&find)).count(),
        1,
        "{calls:?}"
    );
    let (first_page, interrupts) = calls
        .iter()
        .find(|(l, _)| l.contains("Practice pools done") || l.contains("Everything gathered"))
        .expect("the end of the trip");
    assert!(!first_page.contains(&find), "{first_page}");
    assert!(!interrupts, "{first_page}");
}

#[test]
fn the_monthly_states_its_eight_kind_goal() {
    let mut t = challenge(Period::Monthly);
    t.h.press(Key::Return);
    assert!(spoke(&t.h, "Gather 8 different kinds to meet the goal."));
    read_pages(&mut t.h);
    t.h.clear();
    t.press(Key::S);
    t.press(Key::S);
    assert!(spoke(&t.h, "0 of 8 different kinds so far."));
}

#[test]
fn the_shore_has_its_score_and_it_follows_the_trip() {
    use parlor_core::score::{DANGER, HURRY};
    use parlor_core::Rng;
    use tide_pools::outing::mood;
    use tide_pools::rules::{Outing, Plan};
    let t = cove();
    let stems =
        t.h.app
            .ctx
            .audio
            .live()
            .iter()
            .filter(|s| s.looping && s.bus == hearken::Bus::MUSIC && s.key.contains("/score/"))
            .count();
    assert_eq!(stems, sound::SCORE.len(), "every stem starts together");

    let mut rng = Rng::seeded(7);
    let plan = Plan::beach(Place::Cove, &mut rng);
    let mut o = Outing::new(plan, rng);
    let start = mood(&o);
    assert_eq!(start.flags, 0, "a fresh trip is calm");
    o.gathered.push(Species::Crab);
    o.gathered.push(Species::Crab);
    let found = mood(&o);
    assert!(found.intensity > start.intensity, "finds build it");
    o.lean = Lean::In;
    assert!(
        mood(&o).intensity < found.intensity,
        "it thins while you lean in to listen"
    );
    o.tide_left = 50.0;
    assert_ne!(
        mood(&o).flags & (1 << HURRY),
        0,
        "the last minute hurries it"
    );
    assert_eq!(mood(&o).flags & (1 << DANGER), 0);
    o.tide_left = 15.0;
    assert_ne!(mood(&o).flags & (1 << DANGER), 0, "the waves at the rocks");
}

/// A trip to the cove, waited out, with its results menu open.
fn cove_results() -> Twin {
    let mut t = cove();
    t.wait_for_tide();
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "results");
    t
}

#[test]
fn results_back_row_matches_escape() {
    let mut t = cove_results();
    assert_eq!(
        menus::rows_of(&t.h.app.top_lines()),
        [
            "Play again",
            "Hear the results again",
            "Your aquarium",
            "Back to the title",
        ]
    );
    t.h.press(Key::Escape);
    let by_escape = (t.h.app.top_name(), t.h.app.depth());
    let mut t = cove_results();
    t.h.press(Key::End);
    t.h.press(Key::Return);
    assert_eq!((t.h.app.top_name(), t.h.app.depth()), by_escape);
    assert_eq!(by_escape.0, "tide_pools");
}

#[test]
fn play_again_reads_the_next_trip_then_sets_off() {
    let mut t = cove_results();
    t.h.press(Key::Home);
    t.h.clear();
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "pages");
    assert!(spoke(&t.h, "Sandy Cove. A "), "{:?}", t.h.spoken());
    t.h.press(Key::Escape);
    assert_eq!(t.h.app.top_name(), "tide_pools", "Escape backs out");
    t.h.press(Key::Home);
    t.h.press(Key::Return);
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "tide_pools_outing");
}

#[test]
fn hearing_the_results_again_and_visiting_the_aquarium() {
    let mut t = cove_results();
    t.h.press(Key::Down);
    t.h.clear();
    t.h.press(Key::Return);
    assert!(spoke(&t.h, "Tide's in."), "{:?}", t.h.spoken());
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "results");
    t.h.press(Key::Down);
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "tide_pools_aquarium");
    t.h.press(Key::Escape);
    assert_eq!(t.h.app.top_name(), "results");
}

#[test]
fn a_trip_briefing_escape_goes_back_without_starting() {
    let mut h = title();
    h.press(Key::Home);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    let depth = h.app.depth();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "tide_pools");
    assert_eq!(h.app.depth(), depth - 1);
}

#[test]
fn every_key_answers_on_the_shore() {
    let mut t = cove();
    parlor_core::testkit::every_action_answers(&mut t.h);
}

#[test]
fn records_read_the_aquarium_the_shores_and_the_challenges() {
    let mut h = title();
    choose(&mut h, "Records");
    assert_eq!(h.app.top_name(), "pages");
    assert!(spoke(
        &h,
        "Records. Your aquarium holds 0 of 14 kinds, 0 creatures in all, from 0 trips."
    ));
    h.clear();
    h.press(Key::Return);
    assert!(spoke(
        &h,
        "Shore scores. Sandy Cove: not visited yet. Rocky Point: opens at 4 kinds."
    ));
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Challenges. Daily challenge, not played yet."));
}
