//! The dangers of the deeper floors, one by one, with a bare run, and the
//! Unlocks menu on the title.

use crate::common::{self, route, solve_run_room, spoke, SEED};
use haunted_hallways::rules::*;
use haunted_hallways::ID;
use hearken::{Headless, Key};
use parlor_core::Unlocks;

/// A run on `floor`, with the ghosts held off so a test meets one danger
/// at a time.
fn quiet(floor: u32) -> Run {
    let mut run = Run::new(Mode::Floor(floor), SEED);
    run.ghost_in = 1e9;
    run
}

/// An open cell in the middle of the room with an open cell to its east.
fn middle(run: &Run) -> (i32, i32) {
    let room = run.current();
    let open = |x: i32, y: i32| {
        x >= 1
            && y >= 1
            && x + 1 < room.w
            && y + 1 < room.h
            && room.item_at(x, y).is_none()
            && room.item_at(x + 1, y).is_none()
    };
    (1..room.w)
        .flat_map(|x| (1..room.h).map(move |y| (x, y)))
        .find(|&(x, y)| open(x, y))
        .expect("an open middle")
}

/// The first floor and seed whose layout has a room passing `test`, and
/// that room's index.
fn find(floors: std::ops::RangeInclusive<u32>, test: impl Fn(&Room) -> bool) -> (u32, u64, usize) {
    for floor in floors {
        for seed in 0..40 {
            let rooms = Run::plan(Mode::Floor(floor), seed, floor);
            if let Some(i) = rooms.iter().position(&test) {
                return (floor, seed, i);
            }
        }
    }
    panic!("no room found");
}

/// A quiet run standing in the found room.
fn into_room(floor: u32, seed: u64, index: usize) -> Run {
    let mut run = Run::new(Mode::Floor(floor), seed);
    while run.room < index {
        solve_run_room(&mut run);
        assert!(!run.finished(), "the run ended before room {index}");
    }
    run.ghost = None;
    run.ghost_in = 1e9;
    run
}

#[test]
fn a_rotten_board_groans_then_gives_way_and_leaves_a_hole() {
    let mut run = quiet(1);
    let (x, y) = middle(&run);
    run.rooms[0].rotten = vec![(x + 1, y)];
    run.x = x - 1;
    run.y = y;
    let evs = run.walk(Side::East);
    assert!(
        evs.contains(&Ev::Groan { dx: 1.0, dy: 0.0 }),
        "a board next to you groans: {evs:?}"
    );
    let evs = run.walk(Side::East);
    assert_eq!(
        evs[0],
        Ev::Caught {
            by: Threat::Board,
            courage: COURAGE - 1
        }
    );
    assert_eq!(run.stats.falls, 1);
    let room = run.current();
    assert!(room.rotten.is_empty() && room.holes == vec![(x + 1, y)]);
    assert_eq!((run.x, run.y), room.inside(room.entry), "back at the door");
    // The hole stays, and stops you without a fall.
    run.x = x;
    run.y = y;
    assert_eq!(run.walk(Side::East), vec![Ev::Hole(Side::East)]);
    assert_eq!((run.x, run.y, run.courage), (x, y, COURAGE - 1));
}

#[test]
fn the_shadow_follows_footsteps_and_catches() {
    let mut run = quiet(1);
    let (x, y) = middle(&run);
    run.x = x;
    run.y = y;
    run.rooms[0].shadow = Some(Shadow {
        x: x + 1,
        y,
        home: (0, 0),
        interval: 1.0,
        step_in: 1.0,
    });
    // You just stepped, so it hears you and steps your way.
    run.since_step = 0.0;
    let evs = run.update(1.01);
    assert!(
        evs.contains(&Ev::Caught {
            by: Threat::Shadow,
            courage: COURAGE - 1
        }),
        "{evs:?}"
    );
    let sh = run.current().shadow.unwrap();
    assert_eq!((sh.x, sh.y), (0, 0), "it goes home after a catch");
    assert_eq!(run.stats.shadow_catches, 1);
    assert!(run.shadow_words().is_some());
}

#[test]
fn walking_into_the_shadow_is_a_catch() {
    let mut run = quiet(1);
    let (x, y) = middle(&run);
    run.x = x;
    run.y = y;
    run.rooms[0].shadow = Some(Shadow {
        x: x + 1,
        y,
        home: (0, 0),
        interval: 9.0,
        step_in: 9.0,
    });
    let evs = run.walk(Side::East);
    assert!(evs.contains(&Ev::Caught {
        by: Threat::Shadow,
        courage: COURAGE - 1
    }));
}

/// Runs the clock until the ghost is right beside you.
fn until_near(run: &mut Run) {
    for _ in 0..400 {
        if run.ghost.is_some_and(|g| g.phase == GhostPhase::Near) {
            return;
        }
        run.update(0.02);
    }
    panic!("the ghost never came near");
}

#[test]
fn the_hound_catches_you_if_you_stand_still() {
    let mut run = quiet(30);
    run.summon(Some(GhostKind::Hound), Some(0.0));
    until_near(&mut run);
    let mut caught = false;
    for _ in 0..100 {
        let evs = run.update(0.02);
        if evs.iter().any(|e| {
            matches!(
                e,
                Ev::Caught {
                    by: Threat::Ghost(GhostKind::Hound),
                    ..
                }
            )
        }) {
            caught = true;
            break;
        }
    }
    assert!(caught, "standing still is a catch");
}

#[test]
fn walking_while_the_hound_passes_is_safe() {
    let mut run = quiet(30);
    run.summon(Some(GhostKind::Hound), Some(0.0));
    until_near(&mut run);
    let room = run.current().clone();
    let steps = route(&run, room.inside(room.exit));
    let mut back = false;
    let mut i = 0;
    while run.ghost.is_some() {
        // Pace back and forth, never standing still for long.
        let side = steps.get(i).copied().unwrap_or(Side::North);
        let evs = run.walk(if back { side.opposite() } else { side });
        assert!(
            !evs.iter().any(|e| matches!(e, Ev::Caught { .. })),
            "{evs:?}"
        );
        back = !back;
        i = (i + 1) % steps.len().max(1);
        run.update(0.3);
    }
    assert_eq!(run.courage, COURAGE);
    assert_eq!(run.ghosts_evaded, 1);
}

#[test]
fn other_ghosts_still_catch_a_moving_player() {
    let mut run = quiet(30);
    run.summon(Some(GhostKind::Widow), Some(0.0));
    until_near(&mut run);
    run.update(STAB_GRACE);
    let evs = run.walk(Side::North);
    assert!(matches!(
        evs[0],
        Ev::Caught {
            by: Threat::Ghost(GhostKind::Widow),
            ..
        }
    ));
}

#[test]
fn a_mimic_costs_time_and_hops_away() {
    let (floor, seed, index) = find(MIMICS_FROM..=FLOORS, |r| r.mimics() > 0);
    let mut run = into_room(floor, seed, index);
    let room = run.current().clone();
    let at = room.items.iter().position(|p| p.mimic).unwrap();
    let mimic = room.items[at];
    let real = room.target().copied().unwrap();
    assert_eq!(mimic.item, real.item, "it copies the answer");
    run.x = mimic.x;
    run.y = mimic.y;
    let before = run.haunt_left;
    assert_eq!(run.interact(), vec![Ev::Mimic(mimic.item)]);
    assert_eq!(run.haunt_left, before - run.spec.penalty);
    let moved = run.current().items[at];
    assert_ne!((moved.x, moved.y), (mimic.x, mimic.y), "it hops");
    // The real one solves the room and the mimics fall silent.
    run.x = real.x;
    run.y = real.y;
    let evs = run.interact();
    let solved = evs.iter().any(|e| matches!(e, Ev::Solved { .. }));
    if solved {
        assert!(run
            .current()
            .items
            .iter()
            .filter(|p| p.mimic)
            .all(|p| p.taken));
    }
}

#[test]
fn mimics_hop_on_their_own_and_the_real_thing_keeps_still() {
    let (floor, seed, index) = find(MIMICS_FROM..=FLOORS, |r| r.mimics() > 0);
    let mut run = into_room(floor, seed, index);
    let real = run.current().target().copied().unwrap();
    let before: Vec<_> = run.current().items.clone();
    for _ in 0..90 {
        run.update(0.1);
        run.ghost = None;
        run.ghost_in = 1e9;
    }
    let after = &run.current().items;
    let real_now = after
        .iter()
        .find(|p| !p.mimic && p.item == real.item)
        .unwrap();
    assert_eq!((real_now.x, real_now.y), (real.x, real.y));
    assert!(
        before
            .iter()
            .zip(after)
            .any(|(a, b)| a.mimic && (a.x, a.y) != (b.x, b.y)),
        "a mimic moved within nine seconds"
    );
}

#[test]
fn a_dark_room_only_calls_out_close_by() {
    let mut run = quiet(1);
    run.rooms[0].dark = true;
    run.x = 0;
    run.y = 0;
    let mut emits = 0;
    for _ in 0..200 {
        for e in run.update(0.05) {
            if let Ev::Emit { dx, dy, .. } = e {
                emits += 1;
                assert!(dx.hypot(dy) <= DARK_REACH, "{dx}, {dy}");
            }
        }
    }
    let far = run
        .current()
        .items
        .iter()
        .any(|p| ((p.x as f32).hypot(p.y as f32)) > DARK_REACH);
    assert!(far, "the test room has something out of reach");
    let near = run
        .current()
        .items
        .iter()
        .any(|p| ((p.x as f32).hypot(p.y as f32)) <= DARK_REACH);
    assert_eq!(emits > 0, near);
}

#[test]
fn a_missing_room_wants_the_thing_the_phonograph_leaves_out() {
    let (floor, seed, index) = find(MISSING_FROM..=40, |r| {
        matches!(r.puzzle, Puzzle::Missing(_))
    });
    let mut run = into_room(floor, seed, index);
    let Puzzle::Missing(want) = run.current().puzzle else {
        unreachable!()
    };
    assert!(run.current().items.iter().any(|p| p.item == want));
    let evs = solve_run_room(&mut run);
    assert!(evs.contains(&Ev::PickedUp(want)));
    assert!(evs.iter().any(|e| matches!(e, Ev::Solved { .. })));
}

#[test]
fn a_long_seance_wants_four_things_in_order() {
    let (floor, seed, index) = find(
        75..=FLOORS,
        |r| matches!(r.puzzle, Puzzle::Seance(s) if s.len() == 4),
    );
    let mut run = into_room(floor, seed, index);
    let Puzzle::Seance(seq) = run.current().puzzle else {
        unreachable!()
    };
    let items = seq.items().to_vec();
    // The second thing first is wrong.
    let second = *run
        .current()
        .items
        .iter()
        .find(|p| p.item == items[1] && !p.mimic)
        .unwrap();
    run.x = second.x;
    run.y = second.y;
    assert_eq!(run.interact(), vec![Ev::WrongItem(items[1])]);
    // In order, each step says how far along you are.
    for (n, item) in items.iter().enumerate() {
        let goal = run.goal();
        for side in route(&run, goal) {
            run.walk(side);
        }
        let evs = run.interact();
        assert_eq!(evs[0], Ev::PickedUp(*item));
        if n + 1 < items.len() {
            assert_eq!(
                evs[1],
                Ev::SeanceStep {
                    next: items[n + 1],
                    done: n + 1,
                    of: 4
                }
            );
        } else {
            assert!(matches!(evs[1], Ev::Solved { .. }));
        }
    }
}

#[test]
fn nightmare_carries_courage_and_a_landing_gives_one_back() {
    let mut run = Run::new(Mode::Nightmare, SEED);
    assert_eq!(Mode::Nightmare.landing_every(), Some(10));
    run.courage = 1;
    // Floors 1 to 10 cleared: courage carries, then floor 11's landing.
    for floor in 1..=10 {
        while run.spec.number == floor && !run.next_floor_waiting {
            solve_run_room(&mut run);
            assert!(!run.finished(), "floor {floor}");
            run.ghost = None;
            run.ghost_in = 1e9;
        }
        let evs = run.next_floor();
        let landing = evs.iter().any(|e| matches!(e, Ev::Landing { .. }));
        assert_eq!(landing, floor == 10, "floor {floor}");
        run.ghost_in = 1e9;
    }
    assert_eq!(run.courage, 2);
}

#[test]
fn curses_change_the_floor_and_raise_the_score() {
    let curses = Curses::default()
        .with(Twist::CursedHands)
        .with(Twist::SilentSpirits);
    assert_eq!(curses.count(), 2);
    assert_eq!(curses.bonus(100), 150);
    let plain = Run::new(Mode::Floor(1), SEED);
    let cursed = Run::cursed(Mode::Floor(1), SEED, curses);
    assert_eq!(cursed.spec.penalty, plain.spec.penalty * 2.0);
    assert!(!cursed.spec.hints);
}

#[test]
fn the_unlocks_menu_lists_groups_and_switches_a_curse() {
    fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), common::Base);
    h.app.ctx.game.today = Some(common::TODAY);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    Unlocks::earn(&mut h.app.ctx, ID, "curse-cursed-hands");
    h.app.ctx.push(haunted_hallways::title());
    h.frame(0.0);
    common::goto(&mut h, "Unlocks, 1 of 69");
    h.press(Key::Return);
    let rows = common::rows(&h);
    assert!(rows.contains(&"Curses, 1 of 7".to_string()), "{rows:?}");
    assert!(
        rows.contains(&"Diary pages, 0 of 20".to_string()),
        "{rows:?}"
    );
    common::goto(&mut h, "Diary pages");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(spoke(&h, "Clear floor 5."), "{:?}", h.spoken());
    h.press(Key::Escape);
    common::goto(&mut h, "Curses");
    h.press(Key::Return);
    assert!(
        common::rows(&h).contains(&"Cursed hands: off".to_string()),
        "{:?}",
        common::rows(&h)
    );
    common::goto(&mut h, "Cursed hands");
    h.press(Key::Return);
    assert!(spoke(&h, "Cursed hands: on"), "{:?}", h.spoken());
    let curses = haunted_hallways::Progress::load(&mut h.app.ctx).curses();
    assert!(curses.has(Twist::CursedHands));
}
