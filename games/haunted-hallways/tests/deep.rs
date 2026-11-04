//! The deep floors' dangers and the charms, one by one, with bare runs.

use crate::common::{bare_walk, solve_run_room, SEED};
use haunted_hallways::rules::*;

/// A run standing in the second room of `floor`, with the ghosts held off.
fn second_room(floor: u32) -> Run {
    let mut run = Run::new(Mode::Floor(floor), SEED);
    solve_run_room(&mut run);
    assert_eq!(run.room, 1);
    run.ghost = None;
    run.ghost_in = 1e9;
    run
}

/// Runs the clock in small steps, gathering what happens.
fn tick(run: &mut Run, secs: f64) -> Vec<Ev> {
    let mut evs = Vec::new();
    for _ in 0..(secs * 20.0) as usize {
        evs.extend(run.update(0.05));
    }
    evs
}

#[test]
fn each_new_danger_arrives_in_the_second_room_of_its_first_floor() {
    for seed in 0..10 {
        let plan = |n: u32| Run::plan(Mode::Floor(n), seed, n);
        assert!(plan(PENDULUM_FROM)[1].pendulum.is_some());
        assert!(!plan(WATER_FROM)[1].water.is_empty());
        assert!(plan(STONES_FROM)[1].stones);
        assert!(plan(SHIFTS_FROM)[1].shifts);
        for n in [1, 20, 50] {
            assert!(plan(n)
                .iter()
                .all(|r| r.pendulum.is_none() && r.water.is_empty()));
            assert!(plan(n).iter().all(|r| !r.stones && !r.shifts));
        }
        assert_eq!(floor_spec(FOUNDER_FROM - 1, false).founder, 0);
        assert_eq!(floor_spec(FOUNDER_FROM, false).founder, 3);
        assert_eq!(floor_spec(FLOORS, false).founder, 4);
        assert_eq!(floor_spec(FLOORS + 1, false).founder, 5);
    }
}

#[test]
fn charms_lie_only_from_floor_3_and_never_in_a_first_room() {
    let mut seen = 0;
    for seed in 0..20 {
        assert!(Run::plan(Mode::Floor(2), seed, 2)
            .iter()
            .all(|r| r.charm.is_none()));
        let rooms = Run::plan(Mode::Floor(10), seed, 10);
        assert!(rooms[0].charm.is_none());
        for r in &rooms {
            if let Some(c) = r.charm {
                seen += 1;
                assert!(r.item_at(c.x, c.y).is_none());
                assert_ne!((c.x, c.y), r.inside(r.exit));
            }
        }
    }
    assert!(seen > 0);
}

#[test]
fn salt_wards_off_a_ghost_and_a_stub_adds_time() {
    let mut run = second_room(5);
    run.salt = 1;
    run.summon(Some(GhostKind::Widow), Some(0.0));
    while run.ghost.is_some_and(|g| g.phase != GhostPhase::Near) {
        run.update(0.05);
    }
    // Past the grace a step already under the finger gets.
    run.since_stab = haunted_hallways::rules::STAB_GRACE;
    let (x, y) = (run.x, run.y);
    let side = if x > 0 { Side::West } else { Side::East };
    assert_eq!(
        run.walk(side),
        vec![Ev::Warded(Threat::Ghost(GhostKind::Widow))]
    );
    assert_eq!((run.x, run.y, run.courage, run.salt), (x, y, COURAGE, 0));
    assert!(run.ghost.is_none());

    // A stub lying beside you: walking onto it adds to the clock.
    let to = (run.x + side.delta().0, run.y);
    run.rooms[1].charm = Some(Charm {
        kind: CharmKind::Stub,
        x: to.0,
        y: to.1,
        taken: false,
    });
    run.rooms[1].items.retain(|p| (p.x, p.y) != to);
    let before = run.haunt_left;
    let evs = run.walk(side);
    assert!(evs.contains(&Ev::Charm(CharmKind::Stub)), "{evs:?}");
    assert_eq!(run.haunt_left, before + STUB_SECONDS);
    assert_eq!(run.stats.charms, 1);
}

#[test]
fn the_pendulum_cuts_whoever_stands_in_its_row() {
    let mut run = second_room(PENDULUM_FROM);
    run.founder_in = 0.0;
    let p = run.current().pendulum.expect("a pendulum");
    // Stand in its row, away from the blade, and wait.
    run.x = run.current().w - 1;
    run.y = p.row;
    run.rooms[1].items.retain(|i| (i.x, i.y) != (run.x, run.y));
    let evs = tick(&mut run, 10.0);
    assert!(evs.iter().any(|e| matches!(e, Ev::Swing { .. })));
    assert!(
        evs.contains(&Ev::Caught {
            by: Threat::Blade,
            courage: COURAGE - 1
        }),
        "{evs:?}"
    );
    // Out of its row, it only swishes past.
    run.y = if p.row + 1 < run.current().h {
        p.row + 1
    } else {
        p.row - 1
    };
    let evs = tick(&mut run, 10.0);
    assert!(
        !evs.iter().any(|e| matches!(e, Ev::Caught { .. })),
        "{evs:?}"
    );
}

#[test]
fn deep_water_holds_you_and_calls_the_next_ghost_sooner() {
    let mut run = second_room(WATER_FROM);
    let room = run.current().clone();
    // Stand next to the water, on dry ground, and step in.
    let ((wx, wy), side, from) = room
        .water
        .iter()
        .flat_map(|&w| {
            Side::ALL
                .iter()
                .map(move |&s| (w, s, (w.0 - s.delta().0, w.1 - s.delta().1)))
        })
        .find(|&(_, _, c)| {
            c.0 >= 0
                && c.1 >= 0
                && c.0 < room.w
                && c.1 < room.h
                && !room.water.contains(&c)
                && !room.rotten.contains(&c)
                && room.item_at(c.0, c.1).is_none()
        })
        .expect("dry ground beside the water");
    run.x = from.0;
    run.y = from.1;
    run.ghost_in = 30.0;
    let evs = run.walk(side);
    assert_eq!(evs[0], Ev::Splash);
    assert_eq!(run.ghost_in, 30.0 - SPLASH_CALL);
    assert_eq!(run.walk(side), vec![Ev::Wading]);
    assert_eq!((run.x, run.y), (wx, wy));
    tick(&mut run, WADE_TIME + 0.1);
    assert_ne!(run.walk(side.opposite()), vec![Ev::Wading]);
    assert_eq!((run.x, run.y), from);
}

#[test]
fn dust_comes_before_a_stone_and_stepping_off_dodges_it() {
    let mut run = second_room(STONES_FROM);
    run.founder_in = 0.0;
    run.stone_in = 0.1;
    let evs = tick(&mut run, 0.2);
    assert!(evs.contains(&Ev::Dust), "{evs:?}");
    let st = run.stone.expect("a stone coming");
    assert_eq!((st.x, st.y), (run.x, run.y));
    // Step off, and the stone misses.
    let side = Side::ALL
        .into_iter()
        .find(|s| {
            let (dx, dy) = s.delta();
            let c = (run.x + dx, run.y + dy);
            let room = run.current();
            c.0 >= 0 && c.1 >= 0 && c.0 < room.w && c.1 < room.h && !room.rotten.contains(&c)
        })
        .unwrap();
    bare_walk(&mut run, side);
    let evs = tick(&mut run, STONE_DELAY + 0.1);
    assert!(
        evs.iter().any(|e| matches!(e, Ev::StoneFell { .. })),
        "{evs:?}"
    );
    assert_eq!(run.courage, COURAGE);
    // Stay put next time, and it hits.
    run.stone_in = 0.1;
    let evs = tick(&mut run, STONE_DELAY + 0.5);
    assert!(evs.contains(&Ev::Caught {
        by: Threat::Stone,
        courage: COURAGE - 1
    }));
}

#[test]
fn no_stone_starts_while_a_ghost_is_passing() {
    let mut run = second_room(STONES_FROM);
    run.founder_in = 0.0;
    run.summon(Some(GhostKind::Widow), Some(0.0));
    while run.ghost.is_some_and(|g| g.phase != GhostPhase::Near) {
        run.update(0.05);
    }
    run.stone_in = 0.05;
    let evs = run.update(0.05);
    assert!(!evs.contains(&Ev::Dust));
    assert!(run.stone.is_none());
}

/// A run in the second room of floor 81 with the founder beside you,
/// winding up.
fn founder_beside() -> Run {
    let mut run = second_room(FOUNDER_FROM);
    run.rooms[1].pendulum = None;
    run.rooms[1].stones = false;
    run.rooms[1].shadow = None;
    assert_eq!(run.founder_health, 3);
    // He waits while you stand at the door he comes in by.
    let evs = tick(&mut run, FOUNDER_DELAY + 1.0);
    assert!(run.founder.is_none(), "{evs:?}");
    // Three steps into the room, and he comes.
    let room = run.current().clone();
    let (sx, sy) = room.inside(room.entry);
    let far = (0..room.w)
        .flat_map(|x| (0..room.h).map(move |y| (x, y)))
        .find(|&(x, y)| {
            (x - sx).abs() + (y - sy).abs() == 3
                && room.item_at(x, y).is_none()
                && !room.rotten.contains(&(x, y))
                && room.charm_at(x, y).is_none()
        })
        .unwrap();
    (run.x, run.y) = far;
    let evs = tick(&mut run, 0.1);
    assert!(
        evs.iter().any(|e| matches!(e, Ev::FounderEnters { .. })),
        "{evs:?}"
    );
    for _ in 0..400 {
        if run.founder.is_some_and(|f| f.wind.is_some()) {
            break;
        }
        run.update(0.05);
    }
    assert!(run.founder_beside());
    run
}

#[test]
fn the_founder_follows_you_in_and_his_blow_lands_if_you_stay() {
    let mut run = founder_beside();
    let evs = tick(&mut run, 2.0);
    assert!(evs.contains(&Ev::Opening), "{evs:?}");
    assert!(evs.contains(&Ev::Caught {
        by: Threat::Founder,
        courage: COURAGE - 1
    }));
    assert_eq!(run.stats.founder_hits, 1);
    // He comes back through the door, in time.
    assert!(run.founder.is_none());
    assert!(run.founder_in > FOUNDER_DELAY - 2.0);
}

#[test]
fn striking_too_soon_is_blocked_and_at_the_note_it_lands() {
    let mut run = founder_beside();
    assert_eq!(run.interact(), vec![Ev::Parried]);
    assert_eq!(run.founder_health, 3);
    let mut hits = 0;
    for _ in 0..1200 {
        let evs = run.update(0.05);
        if evs.contains(&Ev::Opening) {
            let evs = run.interact();
            if run.founder_health == 0 {
                assert!(matches!(evs[0], Ev::Banished { .. }), "{evs:?}");
                break;
            }
            assert_eq!(
                evs,
                vec![Ev::Hit {
                    left: run.founder_health
                }]
            );
            hits += 1;
        }
        assert_eq!(run.courage, COURAGE, "his blow never landed");
    }
    assert_eq!(hits, 2);
    assert!(run.stats.banished);
    assert!(!run.founder_active());
    // He stays down for the rest of the floor.
    solve_run_room(&mut run);
    let evs = tick(&mut run, FOUNDER_DELAY + 5.0);
    assert!(!evs.iter().any(|e| matches!(e, Ev::FounderEnters { .. })));
}

#[test]
fn stepping_away_makes_his_blow_miss() {
    let mut run = founder_beside();
    let f = run.founder.unwrap();
    let away = Side::ALL
        .into_iter()
        .find(|s| {
            let (dx, dy) = s.delta();
            let c = (run.x + dx, run.y + dy);
            let room = run.current();
            (c.0 - f.x).abs() + (c.1 - f.y).abs() > 1
                && c.0 >= 0
                && c.1 >= 0
                && c.0 < room.w
                && c.1 < room.h
                && !room.rotten.contains(&c)
                && !room.holes.contains(&c)
        })
        .expect("a way to step away");
    bare_walk(&mut run, away);
    let evs = tick(&mut run, 1.6);
    assert!(evs.contains(&Ev::FounderMissed), "{evs:?}");
    assert_eq!(run.courage, COURAGE);
}

#[test]
fn salt_turns_the_founder_back() {
    let mut run = founder_beside();
    run.salt = 1;
    let evs = tick(&mut run, 2.0);
    assert!(evs.contains(&Ev::Warded(Threat::Founder)), "{evs:?}");
    assert_eq!(run.courage, COURAGE);
    assert!(!run.founder_beside());
}

#[test]
fn the_shifting_house_moves_the_exit_door() {
    let mut run = second_room(SHIFTS_FROM);
    run.founder_in = 0.0;
    run.rooms[1].pendulum = None;
    run.rooms[1].stones = false;
    let before = run.current().exit;
    run.shift_in = 0.05;
    let evs = run.update(0.1);
    let after = run.current().exit;
    assert_ne!(before, after);
    assert!(evs.contains(&Ev::Shift(after.side)), "{evs:?}");
    assert_ne!(after.side, run.current().entry.side);
    // The room can still be solved and left by the new door.
    solve_run_room(&mut run);
    assert_eq!(run.room, 2);
}

#[test]
fn practice_costs_no_courage_and_stops_the_clock() {
    let mut run = Run::practice(haunted_hallways::tutorial::rooms());
    let clock = run.haunt_left;
    tick(&mut run, 30.0);
    assert_eq!(run.haunt_left, clock);
    run.room = 2;
    run.x = 0;
    run.y = 3;
    let evs = run.walk(Side::East);
    assert!(matches!(
        evs[0],
        Ev::Caught {
            by: Threat::Board,
            ..
        }
    ));
    assert_eq!(run.courage, COURAGE);
    assert!(!run.finished());
}

#[test]
fn every_deep_floor_can_be_walked_through() {
    for floor in [
        PENDULUM_FROM,
        WATER_FROM,
        STONES_FROM,
        FOUNDER_FROM,
        SHIFTS_FROM,
        FLOORS,
    ] {
        let mut run = Run::new(Mode::Floor(floor), SEED);
        run.ghost_in = 1e9;
        let rooms = run.rooms.len();
        for _ in 0..rooms {
            solve_run_room(&mut run);
        }
        assert!(run.finished() || run.next_floor_waiting, "floor {floor}");
    }
}

/// The haunt screen on a prepared run, under a stand-in launcher.
fn screen(run: Run) -> hearken::Headless<parlor_core::Parlor> {
    use haunted_hallways::haunt::Haunt;
    let mut h = hearken::Headless::new(Default::default(), crate::common::Base);
    h.app.ctx.game.today = Some(crate::common::TODAY);
    let mut haunt = Haunt::with_seed(run.mode, SEED);
    haunt.run = run;
    h.app.ctx.push(haunt);
    h.frame(0.0);
    h
}

#[test]
fn the_screen_teaches_each_deep_danger_the_first_time() {
    for (floor, line) in [
        (PENDULUM_FROM, "A pendulum swings along row"),
        (WATER_FROM, "Flooded. Lapping means deep water."),
        (STONES_FROM, "Stones fall here. When dust trickles"),
        (SHIFTS_FROM, "This room shifts: a rumble moves"),
    ] {
        let h = screen(second_room(floor));
        assert!(
            crate::common::spoke(&h, line),
            "floor {floor}: {:?}",
            h.spoken()
        );
    }
}

#[test]
fn a_fight_with_the_founder_by_keys() {
    use crate::common::{played, spoke};
    use haunted_hallways::sound;
    use hearken::Key;
    let mut run = second_room(FOUNDER_FROM);
    run.rooms[1].pendulum = None;
    run.rooms[1].stones = false;
    run.rooms[1].shadow = None;
    let room = run.current().clone();
    let (sx, sy) = room.inside(room.entry);
    run.x = (0..room.w).max_by_key(|&x| (x - sx).abs()).unwrap();
    run.y = if sy == 0 { 2 } else { sy };
    run.founder_in = 0.5;
    let mut h = screen(run);
    let mut struck = false;
    for _ in 0..60 * 30 {
        h.clear();
        h.frame(1.0 / 60.0);
        if played(&h, sound::OPENING) {
            h.press(Key::Space);
            struck = true;
            break;
        }
    }
    assert!(struck, "his blow came to its height");
    assert!(spoke(&h, "Hit! 2 more hits."), "{:?}", h.spoken());
}
