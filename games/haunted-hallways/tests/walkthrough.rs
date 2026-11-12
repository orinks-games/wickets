//! Nevermore Manor driven the way a player drives it, by keys and frames.

use crate::common;

use common::{key_for, played, route, spoke, title, Twin};
use haunted_hallways::rules::{Mode, Puzzle, Side};
use haunted_hallways::{sound, Progress, ID};
use hearken::{Call, Headless, Key};
use parlor_core::{Parlor, Records};

#[test]
fn the_title_announces_itself() {
    let mut h = Headless::<Parlor>::new(Default::default(), common::Base);
    h.app.ctx.game.today = Some(common::TODAY);
    h.clear();
    h.app.ctx.push(haunted_hallways::title());
    h.frame(0.0);
    assert_eq!(
        h.spoken(),
        [
            "Nevermore Manor. New here? Start with the tutorial.",
            "Tutorial. 8 of 14."
        ]
    );
    let h = title();
    assert_eq!(
        h.spoken().last().map(String::as_str),
        Some("Play, floor 1: the ground floor. 1 of 14.")
    );
    assert_eq!(
        common::rows(&h),
        [
            "Play, floor 1: the ground floor",
            "Floor select",
            "Nightmare, locked",
            "Endless night, locked",
            "The house stirs, locked",
            "The Lantern Night, locked",
            "Challenges, daily not played yet",
            "Tutorial",
            "Help",
            "Sound check",
            "Records",
            "Unlocks, 0 of 69",
            "Settings",
            "Back to the parlor",
        ]
    );
}

#[test]
fn back_to_the_parlor_leaves_the_game() {
    let mut h = title();
    h.press(Key::End);
    assert!(spoke(&h, "Back to the parlor"));
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "base");
}

#[test]
fn the_title_plays_its_theme() {
    let h = title();
    assert_eq!(h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
}

#[test]
fn the_tutorial_walks_the_practice_house() {
    use haunted_hallways::haunt::Haunt;
    use haunted_hallways::tutorial;
    use parlor_core::Live;
    let mut h = title();
    common::goto(&mut h, "Tutorial");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "lesson");
    assert!(spoke(&h, "The doors have locked behind you, and each floor is a chain of rooms, each with one puzzle that unlocks its exit."));
    assert!(spoke(&h, "The Up arrow walks north"), "{:?}", h.spoken());
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "haunted_hallways");

    let house = tutorial::house();
    let lesson = tutorial::lesson_on(&mut h.app.ctx, house.clone());
    assert_eq!(lesson.len(), 13);
    h.app.ctx.push(lesson);
    h.frame(0.0);
    // The voice is reported quiet each frame, so the speech hold does not
    // stop the house; tests/pacing.rs covers the hold.
    let wait = |h: &mut Headless<Parlor>, secs: f64| {
        for _ in 0..(secs * 60.0) as usize {
            h.app.ctx.stop_speech();
            h.frame(1.0 / 60.0);
        }
    };
    let walk = |h: &mut Headless<Parlor>, house: &Live<Haunt>, to: (i32, i32)| {
        for _ in 0..40 {
            let run = house.with(|x| x.run.clone());
            if (run.x, run.y) == to {
                return;
            }
            let Some(&side) = route(&run, to).first() else {
                return;
            };
            h.press(key_for(side));
            wait(h, 0.2);
        }
    };
    let out = |h: &mut Headless<Parlor>, house: &Live<Haunt>| {
        let room = house.with(|x| x.run.current().clone());
        walk(h, house, room.inside(room.exit));
        h.press(key_for(room.exit.side));
        h.frame(1.0 / 60.0);
    };
    let solve = |h: &mut Headless<Parlor>, house: &Live<Haunt>| {
        for _ in 0..10 {
            if house.with(|x| x.run.current().solved) {
                return;
            }
            let goal = house.with(|x| x.run.goal());
            walk(h, house, goal);
            h.press(Key::Space);
            wait(h, 0.1);
        }
    };
    let said = |h: &Headless<Parlor>, line: &str| {
        assert!(spoke(h, line), "{line:?} not in {:?}", h.spoken());
    };

    // The foyer: walking, status in two presses, a named thing, the door.
    let run = house.with(|x| x.run.clone());
    walk(&mut h, &house, (run.x + 2, run.y));
    said(&h, "You are walking the house.");
    h.press(Key::S);
    h.press(Key::S);
    said(&h, "The second press gives the room");
    // The lesson names the thing, and so does the second S.
    said(&h, "This room asks for the ticking clock");
    said(&h, "Puzzle: find the ticking clock.");
    // R names the thing to find.
    h.clear();
    h.press(Key::R);
    said(
        &h,
        "Room 1 of 4: The foyer. Find the ticking clock. The exit is in the north wall, at column 3 of 5.",
    );
    solve(&mut h, &house);
    said(&h, "A solved room unlocks its exit");
    out(&mut h, &house);
    said(&h, "There is no going back.");

    // The library: a ghost to freeze for, a charm, a phonograph, a hint.
    assert!(spoke(&h, "One is coming now."));
    for _ in 0..30 * 4 {
        if spoke(&h, "It drifts away without finding you.") {
            break;
        }
        wait(&mut h, 0.25);
    }
    said(&h, "It drifts away without finding you.");
    assert_eq!(house.with(|x| x.run.courage), 3);
    walk(&mut h, &house, (4, 1));
    said(&h, "Salt: it wards off one catch.");
    assert!(!spoke(&h, "You carry 1 pinch"), "{:?}", h.spoken());
    assert!(!spoke(&h, "candle stub"), "{:?}", h.spoken());
    assert_eq!(house.with(|x| x.run.salt), 1);
    solve(&mut h, &house);
    said(&h, "A riddle room works the same way");
    h.press(Key::H);
    said(&h, "Hints cost more the deeper you go.");
    out(&mut h, &house);

    // The dining hall: around the rotten board to the spirit, and an
    // offering chosen with Tab.
    let door = house.with(|x| x.run.current().inside(x.run.current().exit));
    walk(&mut h, &house, door);
    said(&h, "You stand at the sealed door.");
    for _ in 0..3 {
        if house.with(|x| x.run.held()) == Some(haunted_hallways::rules::Item::Clock) {
            break;
        }
        h.press(Key::Tab);
    }
    h.press(Key::Space);
    said(&h, "The spirit takes it, and the door unlocks.");
    // Said once: the game leaves it to the lesson.
    assert!(!spoke(&h, "The spirit takes the"), "{:?}", h.spoken());
    assert!(!spoke(&h, "Through."), "{:?}", h.spoken());
    assert_eq!(house.with(|x| x.run.stats.falls), 0);
    out(&mut h, &house);

    // The parlour: the candle, with the shadow about. Finding it ends the
    // lesson; the hound is taught where it is first met.
    solve(&mut h, &house);
    said(&h, "The shadow never leaves its room");
    // The room gives its name, its thing and its door; the lesson does
    // the teaching.
    said(
        &h,
        "Room 4 of 4: The parlour. Find the crackling candle. The exit is in the east wall, at row 3 of 5.",
    );
    assert!(!spoke(&h, "follows your footsteps"), "{:?}", h.spoken());
    assert!(!spoke(&h, "hound"), "{:?}", h.spoken());
    said(&h, "Tutorial done.");
    said(&h, "Deeper floors bring new dangers");
    assert_eq!(h.app.top_name(), "haunted_hallways");
}

#[test]
fn the_sound_check_plays_every_cue_with_its_name() {
    let mut h = title();
    h.press(Key::S);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunted_hallways_sounds");
    let count = sound::catalog().len();
    let mut keys = Vec::new();
    for _ in 0..count {
        h.clear();
        h.press(Key::Return);
        keys.extend(h.played());
        assert!(h.spoken().is_empty());
        h.press(Key::Down);
    }
    for (key, _) in sound::catalog() {
        assert!(keys.iter().any(|k| k == key), "{key} was played");
    }
    h.clear();
    h.press(Key::Return); // Left and right
    assert!(h.spoken().is_empty());
    assert!(played(
        &h,
        sound::item(haunted_hallways::rules::Item::Clock)
    ));
    assert!(played(&h, sound::item(haunted_hallways::rules::Item::Bell)));
}

#[test]
fn entering_the_mansion_announces_the_floor_and_the_first_room() {
    let t = Twin::start(Mode::Climb(1));
    let h = &t.h;
    assert!(spoke(
        h,
        "Floor 1: the ground floor. 3 rooms, 3 minutes 50 seconds on the clock."
    ));
    let Puzzle::Find(item) = t.run.current().puzzle else {
        panic!("the first room is a find")
    };
    assert!(
        spoke(
            h,
            &format!("Room 1 of 3: The foyer. Find the {}", item.name())
        ),
        "{:?}",
        h.spoken()
    );
    assert!(spoke(h, "walk; you face north"));
    assert_eq!(h.app.ctx.audio.bed(), Some(sound::ambience(1)));
}

#[test]
fn walking_creaks_and_walls_thud() {
    let mut t = Twin::start(Mode::Climb(1));
    t.h.clear();
    t.walk(Side::North);
    assert!(played(&t.h, sound::STEP_WOOD_B) || played(&t.h, sound::STEP_WOOD_A));
    // A wall with no door in it at this row.
    let room = t.run.current().clone();
    let (side, x) = if room.exit.side == Side::West {
        (Side::East, room.w - 1)
    } else {
        (Side::West, 0)
    };
    let name = side.name();
    t.walk_to((x, t.run.y));
    t.h.clear();
    t.walk(side);
    assert_eq!(t.h.played(), [sound::BUMP]);
    assert!(spoke(
        &t.h,
        &format!("{} wall. ", hearken::text::sentence_start(name))
    ));
    t.h.clear();
    t.walk(side);
    let wall = format!("{}{} wall.", name[..1].to_uppercase(), &name[1..]);
    assert_eq!(t.h.spoken(), [wall]);
}

#[test]
fn the_door_behind_you_is_shut_fast() {
    let mut t = Twin::start(Mode::Climb(1));
    t.h.clear();
    t.walk(Side::South);
    assert!(played(&t.h, sound::DOOR_SHUT));
    assert!(spoke(&t.h, "That door is shut"));
}

#[test]
fn things_call_out_from_their_places() {
    let mut t = Twin::start(Mode::Climb(1));
    t.h.clear();
    for _ in 0..40 {
        t.frame(0.05);
    }
    let room = t.run.current().clone();
    for p in &room.items {
        let key = sound::item(p.item);
        let mix = t.h.sounds.calls().into_iter().find_map(|c| match c {
            Call::Play { key: k, mix, .. } if k == key => Some(mix),
            _ => None,
        });
        let mix = mix.unwrap_or_else(|| panic!("{key} called out"));
        let rel = mix.relative.expect("a placed sound");
        let want = sound::at((p.x - t.run.x) as f32, (p.y - t.run.y) as f32);
        assert!((rel - want).length() < 0.01, "{rel:?} vs {want:?}");
    }
}

#[test]
fn the_locked_exit_rattles() {
    let mut t = Twin::start(Mode::Climb(1));
    let room = t.run.current().clone();
    t.walk_to(room.inside(room.exit));
    t.h.clear();
    t.walk(room.exit.side);
    assert!(played(&t.h, sound::DOOR_LOCKED));
    assert!(spoke(&t.h, "Locked. Solve the room first."));
}

#[test]
fn the_wrong_thing_laughs_and_costs_ten_seconds() {
    let mut t = Twin::start(Mode::Climb(1));
    let room = t.run.current().clone();
    let decoy = *room
        .items
        .iter()
        .find(|p| p.item != room.puzzle.item())
        .unwrap();
    t.walk_to((decoy.x, decoy.y));
    assert!(spoke(&t.h, &format!("Here: the {}", decoy.item.name())));
    t.h.clear();
    t.interact();
    assert!(played(&t.h, sound::WRONG));
    assert!(spoke(&t.h, "Not the "));
    t.h.clear();
    t.h.press(Key::S);
    assert!(
        spoke(&t.h, "3 minutes 40 seconds left"),
        "{:?}",
        t.h.spoken()
    );
}

#[test]
fn finding_the_thing_solves_the_room_and_the_next_one_waits() {
    let mut t = Twin::start(Mode::Climb(1));
    let room = t.run.current().clone();
    let goal = t.run.goal();
    t.walk_to(goal);
    t.h.clear();
    t.interact();
    assert!(played(&t.h, sound::PICKUP) && played(&t.h, sound::DOOR_UNLOCK));
    assert!(spoke(
        &t.h,
        &format!("Took the {}", room.puzzle.item().name())
    ));
    assert!(spoke(&t.h, "Solved. Plus 100."));
    // R in a solved room gives the open door's place, as during the puzzle.
    let door = t.run.door_words(room.exit);
    t.h.clear();
    t.h.press(Key::R);
    assert!(
        spoke(&t.h, &format!("Exit open in {door}.")),
        "{:?}",
        t.h.spoken()
    );
    t.h.clear();
    for _ in 0..60 {
        t.frame(0.05);
    }
    assert!(played(&t.h, sound::DRAFT), "the open door breathes");
    t.walk_to(room.inside(room.exit));
    t.h.clear();
    t.walk(room.exit.side);
    assert!(played(&t.h, sound::DOOR_OPEN));
    assert!(spoke(&t.h, "Room 2 of 3"));
    assert!(spoke(&t.h, "Match the phonograph"));
    assert!(played(&t.h, sound::PHONOGRAPH));
    let Puzzle::Match(item) = t.run.current().puzzle else {
        panic!("room 2 is a match")
    };
    t.h.clear();
    t.frame(1.0);
    assert!(
        played(&t.h, sound::item(item)),
        "the phonograph plays the clue"
    );
}

#[test]
fn status_repeat_hint_and_help_all_answer() {
    let mut t = Twin::start(Mode::Climb(1));
    t.h.clear();
    t.h.press(Key::S);
    let s = t.h.spoken().join(" ");
    // The first press is short: no ghost goes unsaid, then the clock,
    // courage and the way out.
    assert!(s.starts_with("3 minutes"), "{s}");
    assert!(s.contains("courage 3 of 3."), "{s}");
    assert!(s.contains("Exit door locked, "), "{s}");
    // The way to the door ends with its wall.
    let wall = t.run.current().exit.side.name();
    assert!(
        s.contains(&format!(", then {wall}."))
            || s.contains(&format!("{wall} of you."))
            || s.contains(&format!("steps {wall}.")),
        "{s}"
    );
    assert!(!s.contains("ghost"), "{s}");
    assert!(!s.contains("column"), "{s}");
    // A second press straight away gives the rest.
    t.h.clear();
    t.h.press(Key::S);
    let s = t.h.spoken().join(" ");
    assert!(s.contains("Floor 1 of 100, the ground floor."), "{s}");
    assert!(s.contains("column 3 of 5 from the west, row 1 of 5 from the south"));
    assert!(s.contains("Score 0."), "{s}");

    t.h.clear();
    t.h.press(Key::R);
    assert!(spoke(&t.h, "Room 1 of 3"));

    t.h.clear();
    t.h.press(Key::H);
    t.run.hint();
    assert!(played(&t.h, sound::HINT));
    assert!(spoke(&t.h, "5 seconds spent"));

    t.h.clear();
    parlor_core::testkit::read_help(&mut t.h);
    assert!(spoke(&t.h, "Find the"), "{:?}", t.h.spoken());
    assert!(spoke(&t.h, "Escape pauses"));

    t.h.clear();
    t.h.press(Key::Tab);
    assert!(spoke(&t.h, "Your hands are empty"));
    t.h.clear();
    t.h.press(Key::Space);
    assert!(spoke(&t.h, "Nothing here."));
}

#[test]
fn escape_pauses_and_the_pause_menu_returns_to_the_title() {
    let mut t = Twin::start(Mode::Climb(1));
    t.h.press(Key::Escape);
    assert_eq!(t.h.app.top_name(), "pause");
    t.h.clear();
    t.h.press(Key::Return); // Resume
    assert_eq!(t.h.app.top_name(), "haunted_hallways_haunt");
    assert!(spoke(&t.h, "Back in the foyer"), "{:?}", t.h.spoken());
    t.h.press(Key::Escape);
    t.h.press(Key::End);
    t.h.press(Key::Up); // Return to the title
    t.h.clear();
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "haunted_hallways");
    assert!(spoke(&t.h, "Nevermore Manor."));
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::TITLE_MUSIC));
}

/// Into room 2 of the ground floor, where ghosts walk.
fn into_room_two() -> Twin {
    let mut t = Twin::start(Mode::Climb(1));
    t.solve_room();
    assert_eq!(t.run.room, 1);
    t.h.clear();
    t
}

fn wait_for(t: &mut Twin, line: &str) {
    for _ in 0..6000 {
        if spoke(&t.h, line) {
            return;
        }
        t.frame(0.05);
    }
    panic!("never heard {line:?}: {:?}", t.h.spoken());
}

#[test]
fn a_ghost_comes_and_moving_as_it_passes_gets_you_caught() {
    let mut t = into_room_two();
    wait_for(&mut t, "A ghost! ");
    assert!(spoke(&t.h, "At the stab, freeze until it breathes out."));
    assert!(t
        .h
        .app
        .ctx
        .audio
        .live()
        .iter()
        .any(|s| s.looping && s.key.contains("/ghost/")));
    wait_for(&mut t, "Freeze!");
    assert!(played(&t.h, sound::FREEZE));
    t.h.clear();
    t.h.press(Key::S);
    assert!(spoke(&t.h, "passing. Do not move!"));
    // Past the grace for a step already under the finger.
    for _ in 0..10 {
        t.frame(0.05);
    }
    t.h.clear();
    t.walk(Side::North);
    assert!(played(&t.h, sound::CAUGHT));
    assert!(spoke(
        &t.h,
        "saw you move. Back to the door. Courage 2 of 3."
    ));
    assert_eq!(t.run.courage, 2);
}

#[test]
fn a_step_right_on_the_stab_is_let_through() {
    let mut t = into_room_two();
    wait_for(&mut t, "Freeze!");
    t.h.clear();
    t.walk(Side::North);
    assert!(!played(&t.h, sound::CAUGHT), "{:?}", t.h.spoken());
    assert_eq!(t.run.courage, 3);
}

#[test]
fn freezing_lets_the_ghost_drift_away() {
    let mut t = into_room_two();
    wait_for(&mut t, "Freeze!");
    t.h.clear();
    wait_for(&mut t, "gone. Plus 25.");
    assert!(played(&t.h, sound::PASSED));
    assert_eq!(t.run.ghosts_evaded, 1);
    assert!(t
        .h
        .app
        .ctx
        .audio
        .live()
        .iter()
        .all(|s| !(s.looping && s.key.contains("/ghost/"))));
}

#[test]
fn the_clock_running_out_ends_the_haunt() {
    let mut t = Twin::start(Mode::Climb(1));
    wait_for(&mut t, "1 minute left.");
    assert!(played(&t.h, sound::TOLL));
    t.h.clear();
    for _ in 0..700 {
        t.frame(0.1);
        if t.h.app.top_name() == "pages" {
            break;
        }
    }
    assert!(spoke(
        &t.h,
        "The haunt clock strikes zero on floor 1, in the foyer."
    ));
    assert!(spoke(&t.h, "No score this time."));
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::LOSE));
    t.read_until("haunted_hallways");
}

#[test]
fn three_catches_chase_you_from_the_house() {
    let mut t = into_room_two();
    for _ in 0..3 {
        t.h.clear();
        wait_for(&mut t, "Freeze!");
        for _ in 0..10 {
            t.frame(0.05);
        }
        t.walk(Side::North);
    }
    assert!(
        spoke(&t.h, "Out of courage, you run screaming"),
        "{:?}",
        t.h.spoken()
    );
    // The solved room still scored, so the run is on the board.
    assert!(spoke(&t.h, "Score 100: your first on this board."));
    t.read_until("haunted_hallways");
    assert_eq!(Records::best(&mut t.h.app.ctx, ID, "climb"), Some(100));
}

#[test]
fn clearing_the_ground_floor_saves_its_time_and_opens_floor_two() {
    let mut t = Twin::start(Mode::Floor(1));
    for _ in 0..3 {
        t.frame(0.5);
        t.solve_room();
    }
    assert!(played(&t.h, sound::STAIRS));
    assert!(
        spoke(&t.h, "Floor 1 cleared in 1.5 seconds."),
        "{:?}",
        t.h.spoken()
    );
    assert!(spoke(&t.h, "Floor 2 is open."));
    assert_eq!(t.h.app.ctx.audio.music(), Some(sound::WIN));
    t.read_until("haunted_hallways");
    assert!(spoke(&t.h, "You cleared floor 1, the ground floor"));
    let ctx = &mut t.h.app.ctx;
    let progress = Progress::load(ctx);
    assert_eq!(progress.cleared, 1);
    assert_eq!(progress.floor_time(1), Some(1.5));
    let best = Records::best(ctx, ID, "floor 1").expect("a floor score");
    assert_eq!(best, t.run.score);

    // Floor select now offers floor 2, and shows floor 1's best.
    t.h.press(Key::F);
    t.h.clear();
    t.h.press(Key::Return);
    assert!(spoke(
        &t.h,
        "Floors 1 to 10, the ground floor, 1 of 10 cleared"
    ));
    t.h.press(Key::Down);
    assert!(spoke(&t.h, "Floors 11 to 20, locked"));
    t.h.press(Key::Up);
    t.h.clear();
    t.h.press(Key::Return);
    assert!(
        spoke(&t.h, "Floor 1, best 1.5 seconds"),
        "{:?}",
        t.h.spoken()
    );
    t.h.press(Key::Down);
    assert!(spoke(&t.h, "Floor 2, not cleared"));
    t.h.press(Key::Down);
    assert!(spoke(&t.h, "Floor 3, locked"));
    t.h.clear();
    t.h.press(Key::Return);
    assert!(spoke(&t.h, "Clear floor 2 to open floor 3."));
}

#[test]
fn a_slower_clear_keeps_the_better_time() {
    let mut t = Twin::start(Mode::Floor(1));
    for _ in 0..3 {
        t.solve_room();
    }
    t.read_until("haunted_hallways");
    let mut t = Twin::from_title(t.h, Mode::Floor(1));
    for _ in 0..3 {
        t.frame(2.0);
        t.solve_room();
    }
    assert!(
        spoke(&t.h, "Your best here is 0.0 seconds."),
        "{:?}",
        t.h.spoken()
    );
    t.read_until("haunted_hallways");
    assert_eq!(Progress::load(&mut t.h.app.ctx).floor_time(1), Some(0.0));
}

#[test]
fn the_climb_goes_on_and_floor_25_opens_nightmare() {
    let mut t = Twin::start(Mode::Climb(1));
    for floor in 1..=2 {
        let rooms = t.run.rooms.len();
        for _ in 0..rooms {
            t.frame(0.2);
            t.solve_room();
        }
        assert_eq!(t.h.app.top_name(), "pages");
        assert!(spoke(&t.h, &format!("Floor {floor} cleared in")));
        t.h.clear();
        t.read_until("haunted_hallways_haunt");
        t.run.next_floor();
        assert!(
            spoke(&t.h, &format!("Floor {}. ", floor + 1)),
            "{:?}",
            t.h.spoken()
        );
        assert_eq!(t.h.app.ctx.audio.bed(), Some(sound::ambience(floor + 1)));
    }
    assert_eq!(Progress::load(&mut t.h.app.ctx).cleared, 2);
    // Leave from the pause menu: the floors cleared stay open.
    t.h.press(Key::Escape);
    common::goto(&mut t.h, "Back to the title");
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "haunted_hallways");
    assert_eq!(common::rows(&t.h)[0], "Play, floor 3: the ground floor");

    // From floor 24 cleared, floor 25 opens Nightmare.
    let ctx = &mut t.h.app.ctx;
    let progress = Progress {
        cleared: 24,
        ..Default::default()
    };
    progress.store(ctx);
    let mut t = Twin::from_title(t.h, Mode::Climb(25));
    assert!(
        spoke(&t.h, "Floor 25: the bedchambers"),
        "{:?}",
        t.h.spoken()
    );
    for _ in 0..t.run.rooms.len() {
        t.frame(0.2);
        t.solve_room();
    }
    assert!(
        spoke(&t.h, "Nightmare is open on the title."),
        "{:?}",
        t.h.spoken()
    );
    assert!(spoke(&t.h, "Unlocked:"), "{:?}", t.h.spoken());
    t.read_until("haunted_hallways_haunt");
    t.h.press(Key::Escape);
    common::goto(&mut t.h, "Back to the title");
    t.h.press(Key::Return);
    let ctx = &mut t.h.app.ctx;
    assert_eq!(Progress::load(ctx).cleared, 25);
    assert!(parlor_core::Unlocks::has(ctx, ID, "mode-nightmare"));
    assert!(parlor_core::Unlocks::has(ctx, ID, "diary-5"));

    // Nightmare is now on the title, and starts from floor 1.
    t.h.clear();
    t.h.press(Key::N);
    assert!(spoke(&t.h, "Nightmare. 3 of 14."), "{:?}", t.h.spoken());
    let t = Twin::from_title(t.h, Mode::Nightmare);
    assert!(spoke(&t.h, "Floor 1: the ground floor. 3 rooms, "));
}

#[test]
fn records_read_back_what_was_saved() {
    let mut t = Twin::start(Mode::Floor(1));
    for _ in 0..3 {
        t.frame(1.0);
        t.solve_room();
    }
    t.read_until("haunted_hallways");
    common::goto(&mut t.h, "Records");
    t.h.clear();
    t.h.press(Key::Return);
    assert!(
        spoke(
            &t.h,
            "Records. Highest floor cleared: 1 of 100. Play, no score yet."
        ),
        "{:?}",
        t.h.spoken()
    );
    t.h.clear();
    t.h.press(Key::Return);
    assert!(
        spoke(
            &t.h,
            "Floors 1 to 10, the ground floor: floor 1, best 3.0 seconds; floor 2, not yet cleared."
        ),
        "{:?}",
        t.h.spoken()
    );
    t.h.clear();
    t.h.press(Key::Return);
    assert!(spoke(
        &t.h,
        "Floors 11 to 20, the portrait gallery: locked."
    ));
}

#[test]
fn an_offering_room_is_solved_from_the_inventory() {
    // Play floor 2 on its own, once it is open.
    let mut h = title();
    let ctx = &mut h.app.ctx;
    let mut p = Progress::load(ctx);
    p.cleared = 1;
    p.store(ctx);
    let mut t = Twin::from_title(h, Mode::Floor(2));
    assert!(spoke(&t.h, "Floor 2: the ground floor"));
    while !matches!(t.run.current().puzzle, Puzzle::Offering(_)) {
        t.solve_room();
    }
    let room = t.run.current().clone();
    let Puzzle::Offering(want) = room.puzzle else {
        unreachable!()
    };
    assert!(spoke(&t.h, "A spirit seals the exit"));
    assert!(played(&t.h, sound::SPIRIT));
    t.h.clear();
    t.frame(1.0);
    assert!(
        played(&t.h, sound::item(want)),
        "the spirit plays what it wants"
    );
    t.h.clear();
    t.interact();
    assert!(spoke(&t.h, "Offer at the sealed door"));
    t.walk_to(room.inside(room.exit));
    while t.run.held() != Some(want) {
        t.h.clear();
        t.cycle();
        assert!(played(&t.h, sound::SELECT));
    }
    t.h.clear();
    t.interact();
    assert!(played(&t.h, sound::OFFER_OK));
    assert!(spoke(
        &t.h,
        &format!("The spirit takes the {}.", want.name())
    ));
    assert!(spoke(&t.h, "Solved."));
}

#[test]
fn held_keys_do_not_walk() {
    let mut t = Twin::start(Mode::Climb(1));
    let y = t.run.y;
    t.h.clear();
    t.h.send(hearken::InputEvent::KeyDown {
        key: Key::Up,
        mods: hearken::Mods::NONE,
        text: None,
        repeat: true,
    });
    assert!(t.h.played().is_empty());
    assert_eq!(t.run.y, y);
}

#[test]
fn keys_with_no_job_stay_silent() {
    let mut t = Twin::start(Mode::Climb(1));
    for key in [Key::Return, Key::Home, Key::Q, Key::LShift] {
        t.h.clear();
        t.h.press(key);
        assert!(t.h.spoken().is_empty(), "{key:?}: {:?}", t.h.spoken());
    }
}

#[test]
fn thunder_and_screams_are_explained_once() {
    let mut t = into_room_two();
    let atmos = "thunder and distant screams are atmosphere and mean nothing";
    for _ in 0..1600 {
        t.frame(0.05);
        if played(&t.h, sound::THUNDER) || played(&t.h, sound::SCREAM) {
            break;
        }
    }
    assert!(played(&t.h, sound::THUNDER) || played(&t.h, sound::SCREAM));
    assert!(spoke(&t.h, atmos), "{:?}", t.h.spoken());
}

#[test]
fn the_breath_out_comes_as_soon_as_it_is_safe_to_walk() {
    let mut t = into_room_two();
    wait_for(&mut t, "Freeze!");
    t.h.clear();
    wait_for(&mut t, "gone. Plus");
    assert!(t.run.ghost.is_none());
    t.h.clear();
    // Walking straight after the breath out is safe.
    let side = if t.run.y + 1 < t.run.current().h {
        Side::North
    } else {
        Side::South
    };
    t.walk(side);
    assert!(!played(&t.h, sound::CAUGHT));
    assert_eq!(t.run.courage, 3);
}

fn score_stems(t: &Twin) -> usize {
    t.h.app
        .ctx
        .audio
        .live()
        .iter()
        .filter(|s| {
            s.looping
                && s.bus == hearken::Bus::MUSIC
                && s.key.starts_with("haunted-hallways/score/")
        })
        .count()
}

#[test]
fn each_floor_plays_its_score_and_a_ghost_darkens_it() {
    use haunted_hallways::haunt::Haunt;
    use haunted_hallways::rules::{Ghost, GhostKind, GhostPhase};
    use parlor_core::score::{DANGER, HURRY};
    let t = Twin::start(Mode::Climb(1));
    assert_eq!(
        score_stems(&t),
        sound::SCORE.len(),
        "every stem starts together"
    );
    assert_eq!(t.h.app.ctx.audio.music(), None);

    let mut haunt = Haunt::with_seed(Mode::Climb(1), 7);
    let calm = haunt.mood();
    assert_eq!(calm.flags, 0);
    haunt.run.room = haunt.run.rooms.len() - 1;
    assert!(
        haunt.mood().intensity > calm.intensity,
        "deeper in builds it"
    );
    haunt.run.haunt_left = 20.0;
    assert_ne!(
        haunt.mood().flags & (1 << HURRY),
        0,
        "the clock running low hurries it"
    );
    haunt.run.ghost = Some(Ghost {
        kind: GhostKind::Widow,
        bearing: 0.0,
        s: 3.0,
        speed: 1.0,
        phase: GhostPhase::Approaching,
    });
    assert_ne!(haunt.mood().flags & (1 << DANGER), 0, "a ghost darkens it");
}

/// A floor cleared on its own, read through to the results menu.
fn floor_one_results() -> Twin {
    let mut t = Twin::start(Mode::Floor(1));
    for _ in 0..3 {
        t.frame(0.5);
        t.solve_room();
    }
    for _ in 0..10 {
        if t.h.app.top_name() == "results" {
            break;
        }
        t.h.press(Key::Return);
    }
    assert_eq!(t.h.app.top_name(), "results");
    t
}

#[test]
fn a_cleared_floor_ends_on_the_shared_results() {
    let mut t = floor_one_results();
    assert_eq!(
        common::rows(&t.h),
        [
            "Next floor: floor 2, the ground floor",
            "Play again",
            "Hear the results again",
            "Back to the title",
        ]
    );
    t.h.clear();
    t.h.press(Key::Return);
    assert_eq!(t.h.app.top_name(), "haunted_hallways_haunt");
    assert!(spoke(&t.h, "Floor 2: the ground floor."));
}

#[test]
fn results_back_row_matches_escape() {
    let mut by_row = floor_one_results();
    by_row.h.press(Key::End);
    by_row.h.press(Key::Return);
    let mut by_escape = floor_one_results();
    by_escape.h.press(Key::Escape);
    let place = |t: &Twin| (t.h.app.top_name(), t.h.app.depth());
    assert_eq!(place(&by_row), place(&by_escape));
    assert_eq!(place(&by_row), ("haunted_hallways", 2));
}

#[test]
fn first_time_teaching_comes_once_per_save() {
    use haunted_hallways::haunt::Haunt;
    let mut h = title();
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunted_hallways_haunt");
    assert!(
        spoke(
            &h,
            "Arrows walk; you face north. S: clock and exit. H: hint."
        ),
        "{:?}",
        h.spoken()
    );
    assert!(spoke(&h, "Press Space on it."), "{:?}", h.spoken());
    // A second run on the same save is not taught again.
    h.app.ctx.pop();
    h.frame(0.0);
    h.clear();
    let haunt = Haunt::climb(&mut h.app.ctx);
    h.app.ctx.push(haunt);
    h.frame(0.0);
    assert!(spoke(&h, "Room 1 of"), "{:?}", h.spoken());
    assert!(!spoke(&h, "Arrows walk"), "{:?}", h.spoken());
    assert!(!spoke(&h, "Press Space on it."), "{:?}", h.spoken());
}
