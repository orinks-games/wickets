//! The rules on their own: layouts, walking, puzzles, ghosts and the clock.

use crate::common;

use common::{route, solve_run_room, SEED};
use haunted_hallways::progress::{self, Progress};
use haunted_hallways::rules::*;

fn floor_run(n: u32) -> Run {
    Run::new(Mode::Floor(n), SEED)
}

#[test]
fn layouts_are_the_same_for_the_same_seed_and_differ_between_seeds() {
    let a = Run::plan(Mode::Climb(1), SEED, 2);
    let b = Run::plan(Mode::Climb(1), SEED, 2);
    let c = Run::plan(Mode::Climb(1), SEED + 1, 2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn the_ground_floor_teaches_find_then_match_then_riddle() {
    for seed in 0..20 {
        let rooms = Run::plan(Mode::Climb(1), seed, 1);
        assert_eq!(rooms.len(), 3);
        assert!(matches!(rooms[0].puzzle, Puzzle::Find(_)));
        assert!(matches!(rooms[1].puzzle, Puzzle::Match(_)));
        assert!(matches!(rooms[2].puzzle, Puzzle::Riddle(_)));
    }
}

#[test]
fn every_floor_lays_out_sound_rooms() {
    let floors: Vec<u32> = (1..=FLOORS).chain([101, 137, 250]).collect();
    for seed in 0..6 {
        for &floor in &floors {
            let spec = floor_spec(floor, false);
            let rooms = Run::plan(Mode::Floor(floor), seed, floor);
            assert_eq!(rooms.len(), spec.rooms);
            let mut found = Vec::new();
            let mut offerings = 0;
            for (i, room) in rooms.iter().enumerate() {
                assert_ne!(room.entry.side, room.exit.side);
                assert!((0..room.w).contains(&room.exit.at));
                if i > 0 {
                    // Each room's entry lines up with the last room's exit.
                    let prev = rooms[i - 1].exit;
                    assert_eq!(room.entry.side, prev.side.opposite());
                    assert_eq!(room.entry.at, prev.at);
                } else {
                    assert!(room.rotten.is_empty() && room.shadow.is_none());
                    assert!(!room.dark && !room.mirror && room.mimics() == 0);
                }
                let start = room.inside(room.entry);
                match room.puzzle {
                    Puzzle::Offering(want) => {
                        offerings += 1;
                        assert!(i > 0, "never an offering first");
                        assert!(
                            found.contains(&want),
                            "the spirit wants something found earlier"
                        );
                        found.retain(|&f| f != want);
                        assert!(room.items.is_empty());
                    }
                    p => {
                        let real: Vec<&Placed> = room.items.iter().filter(|i| !i.mimic).collect();
                        let wanted = p.wanted();
                        let decoys = if matches!(p, Puzzle::Missing(_)) {
                            spec.decoys.max(2)
                        } else {
                            spec.decoys
                        };
                        assert_eq!(real.len(), wanted.len() + decoys.min(9 - wanted.len()));
                        for w in &wanted {
                            assert_eq!(real.iter().filter(|it| it.item == *w).count(), 1);
                        }
                        for m in room.items.iter().filter(|i| i.mimic) {
                            assert_eq!(m.item, p.item(), "a mimic copies the answer");
                        }
                        for (j, it) in room.items.iter().enumerate() {
                            assert!((0..room.w).contains(&it.x) && (0..room.h).contains(&it.y));
                            assert_ne!((it.x, it.y), start);
                            assert!(!room.rotten.contains(&(it.x, it.y)));
                            for other in &room.items[j + 1..] {
                                assert_ne!((it.x, it.y), (other.x, other.y));
                                if !it.mimic && !other.mimic {
                                    assert_ne!(it.item, other.item);
                                }
                            }
                        }
                        found.extend(wanted);
                    }
                }
                // Rotten boards never cut off a thing or the exit.
                let mut must: Vec<(i32, i32)> = room.items.iter().map(|p| (p.x, p.y)).collect();
                must.push(room.inside(room.exit));
                for m in must {
                    assert!(
                        reachable(room, start, m),
                        "floor {floor} seed {seed} room {i}: {m:?} cut off"
                    );
                }
                assert!(room.rotten.len() <= spec.rotten);
            }
            if floor >= 2 {
                assert!(offerings >= 1, "floor {floor} seed {seed} has an offering");
            }
        }
    }
}

/// Whether `to` can be walked to from `from` without a rotten board.
fn reachable(room: &Room, from: (i32, i32), to: (i32, i32)) -> bool {
    let mut seen = vec![from];
    let mut i = 0;
    while i < seen.len() {
        let (x, y) = seen[i];
        i += 1;
        if (x, y) == to {
            return true;
        }
        for s in Side::ALL {
            let (dx, dy) = s.delta();
            let n = (x + dx, y + dy);
            if n.0 >= 0
                && n.1 >= 0
                && n.0 < room.w
                && n.1 < room.h
                && !room.rotten.contains(&n)
                && !seen.contains(&n)
            {
                seen.push(n);
            }
        }
    }
    false
}

#[test]
fn floors_grow_harder_and_nightmare_is_harder_still() {
    for n in 1..FLOORS + 20 {
        let (a, b) = (floor_base(n, false), floor_base(n + 1, false));
        assert!(b.rooms >= a.rooms && b.size >= a.size && b.decoys >= a.decoys);
        assert!(b.ghost_speed >= a.ghost_speed);
        assert!(b.ghosts.len() >= a.ghosts.len());
        assert!(b.penalty >= a.penalty && b.hint_cost >= a.hint_cost);
        assert!(b.rotten >= a.rotten && b.seance_len >= a.seance_len);
    }
    let gap = |n| floor_base(n, false).ghost_gap.1;
    assert!(gap(100) < gap(50) && gap(50) < gap(1));
    assert_eq!(floor_spec(1, false).rooms, 3);
    assert_eq!(floor_spec(100, false).size, 9);
    assert_eq!(floor_spec(500, false).rooms, 8);
    for n in 1..=FLOORS {
        let (easy, hard) = (floor_spec(n, false), floor_spec(n, true));
        assert!(hard.haunt < easy.haunt);
        assert!(hard.ghost_gap.1 < easy.ghost_gap.1);
        assert!(hard.ghost_speed > easy.ghost_speed);
        assert!(!hard.call_freeze);
        assert_eq!(easy.call_freeze, n <= 4);
    }
}

#[test]
fn each_danger_arrives_on_its_own_floor() {
    let has = |n: u32| floor_spec(n, false);
    assert!(!has(OFFERINGS_FROM - 1).offerings && has(OFFERINGS_FROM).offerings);
    assert!(!has(MISSING_FROM - 1).missing && has(MISSING_FROM).missing);
    assert!(has(ROTTEN_FROM - 1).rotten == 0 && has(ROTTEN_FROM).rotten > 0);
    assert!(has(SEANCES_FROM - 1).seance_len == 0 && has(SEANCES_FROM).seance_len == 2);
    assert_eq!(has(40).seance_len, 3);
    assert_eq!(has(75).seance_len, 4);
    assert!(has(SHADOW_FROM - 1).shadow == 0.0 && has(SHADOW_FROM).shadow > 0.0);
    assert!(!has(HOUND_FROM - 1).ghosts.contains(&GhostKind::Hound));
    assert!(has(HOUND_FROM).ghosts.contains(&GhostKind::Hound));
    assert!(has(MIMICS_FROM - 1).mimics == 0 && has(MIMICS_FROM).mimics > 0);
    for n in 1..TWISTS_FROM {
        assert!(floor_twists(n).is_empty());
    }
    for n in TWISTS_FROM..=FLOORS + 10 {
        let twists = floor_twists(n);
        assert_eq!(twists, floor_twists(n), "the same every time");
        for t in &twists {
            assert!(n >= t.from_floor(), "{t:?} on floor {n}");
            assert!(floor_spec(n, false).twists.contains(t));
        }
        if n > 40 {
            assert!(!twists.is_empty());
        }
    }
    assert_eq!(floor_twists(150).len(), 3);
    assert_eq!(wing(1).name, "the ground floor");
    assert_eq!(wing(55).name, "the clock tower");
    assert_eq!(wing(100).name, "the heart of the house");
    assert_eq!(wing(101).name, "the endless dark");
}

#[test]
fn you_start_inside_the_entry_door_and_walls_stop_you() {
    let mut run = floor_run(1);
    let room = run.current().clone();
    assert_eq!((run.x, run.y), room.inside(room.entry));
    assert_eq!(room.entry.side, Side::South);
    assert_eq!(run.walk(Side::South), vec![Ev::DoorShut]);
    for _ in 0..room.w {
        run.walk(Side::West);
    }
    assert_eq!(run.x, 0);
    assert_eq!(run.walk(Side::West), vec![Ev::Bump(Side::West)]);
    assert_eq!(run.x, 0);
    assert_eq!(run.walk(Side::North)[0], Ev::Step);
    assert_eq!(run.y, 1);
}

#[test]
fn the_exit_is_locked_until_the_room_is_solved() {
    let mut run = floor_run(1);
    let room = run.current().clone();
    for side in route(&run, room.inside(room.exit)) {
        run.walk(side);
    }
    assert_eq!(
        run.walk(room.exit.side),
        vec![Ev::DoorLocked { sealed: false }]
    );
    assert_eq!(run.interact(), vec![Ev::AtDoor { open: false }]);
    assert_eq!(run.room, 0);
}

#[test]
fn the_right_thing_solves_the_room_and_the_wrong_one_costs_time() {
    let mut run = floor_run(1);
    let room = run.current().clone();
    let target = room.puzzle.item();
    let decoy = *room.items.iter().find(|p| p.item != target).unwrap();
    for side in route(&run, (decoy.x, decoy.y)) {
        run.walk(side);
    }
    let before = run.haunt_left;
    assert_eq!(run.interact(), vec![Ev::WrongItem(decoy.item)]);
    assert_eq!(run.haunt_left, before - WRONG_PENALTY);
    assert!(!run.current().solved);

    let goal = run.goal();
    let mut evs = Vec::new();
    for side in route(&run, goal) {
        evs.extend(run.walk(side));
    }
    assert!(evs.contains(&Ev::OnItem {
        item: target,
        spent: false
    }));
    let evs = run.interact();
    assert_eq!(evs[0], Ev::PickedUp(target));
    assert!(matches!(evs[1], Ev::Solved { points: 100, door } if door == room.exit.side));
    assert_eq!(run.inventory, vec![target]);
    assert_eq!(run.score, 100);
    // Nothing is left to take; the decoy is now just furniture.
    assert_eq!(run.interact(), vec![Ev::NothingHere]);
}

#[test]
fn walking_through_the_open_door_enters_the_next_room() {
    let mut run = floor_run(1);
    let evs = solve_run_room(&mut run);
    assert!(evs.contains(&Ev::DoorOpened));
    assert!(evs.contains(&Ev::EnterRoom { index: 1, of: 3 }));
    assert_eq!(run.room, 1);
    let room = run.current();
    assert_eq!((run.x, run.y), room.inside(room.entry));
}

#[test]
fn an_offering_takes_only_what_the_spirit_wants() {
    // Find a floor-2 layout whose offering comes with two things carried.
    let mut run = floor_run(2);
    loop {
        let room = run.current().clone();
        if let Puzzle::Offering(want) = room.puzzle {
            if run.inventory.len() >= 2 {
                let door = room.inside(room.exit);
                assert!(matches!(run.interact(), v if v == vec![Ev::NotAtDoor(room.exit.side)]));
                for side in route(&run, door) {
                    run.walk(side);
                }
                assert_eq!(
                    run.walk(room.exit.side),
                    vec![Ev::DoorLocked { sealed: true }]
                );
                while run.held() == Some(want) {
                    run.cycle();
                }
                let wrong = run.held().unwrap();
                let before = run.haunt_left;
                assert_eq!(run.interact(), vec![Ev::OfferRefused(wrong)]);
                assert_eq!(run.haunt_left, before - WRONG_PENALTY);
                while run.held() != Some(want) {
                    run.cycle();
                }
                let carried = run.inventory.len();
                let evs = run.interact();
                assert_eq!(evs[0], Ev::OfferAccepted(want));
                assert_eq!(run.inventory.len(), carried - 1);
                assert!(run.current().solved);
                return;
            }
        }
        solve_run_room(&mut run);
        if run.next_floor_waiting || run.finished() {
            run = Run::new(Mode::Floor(2), run.seed + 1);
        }
    }
}

#[test]
fn tab_cycles_what_you_carry() {
    let mut run = floor_run(1);
    assert_eq!(run.cycle(), vec![Ev::EmptyHands]);
    solve_run_room(&mut run);
    solve_run_room(&mut run);
    assert_eq!(run.inventory.len(), 2);
    let first = run.inventory[0];
    assert_eq!(
        run.cycle(),
        vec![Ev::Selected {
            item: first,
            index: 0,
            of: 2
        }]
    );
}

#[test]
fn things_call_out_from_where_they_lie_and_faster_when_near() {
    let mut run = floor_run(1);
    let room = run.current().clone();
    let target = room.items[0];
    let mut times = Vec::new();
    let mut t = 0.0;
    for _ in 0..200 {
        t += 0.05;
        for ev in run.update(0.05) {
            if let Ev::Emit { item, dx, dy } = ev {
                if item == target.item {
                    assert_eq!(
                        (dx, dy),
                        ((target.x - run.x) as f32, (target.y - run.y) as f32)
                    );
                    times.push(t);
                }
            }
        }
    }
    assert!(times.len() >= 3);
    let far_gap = times[2] - times[1];
    // Now stand next to it.
    let near = (target.x, (target.y - 1).max(0));
    for side in route(&run, near) {
        run.walk(side);
    }
    let mut near_times = Vec::new();
    for _ in 0..200 {
        t += 0.05;
        for ev in run.update(0.05) {
            if matches!(ev, Ev::Emit { item, .. } if item == target.item) {
                near_times.push(t);
            }
        }
    }
    let near_gap = near_times[2] - near_times[1];
    assert!(near_gap <= far_gap, "{near_gap} vs {far_gap}");
}

#[test]
fn a_solved_room_breathes_a_draught_from_its_open_door() {
    let mut run = floor_run(1);
    let goal = run.goal();
    for side in route(&run, goal) {
        run.walk(side);
    }
    run.interact();
    let mut drafts = 0;
    for _ in 0..100 {
        for ev in run.update(0.05) {
            assert!(!matches!(ev, Ev::Emit { .. }), "the room falls silent");
            if matches!(ev, Ev::Draft { .. }) {
                drafts += 1;
            }
        }
    }
    assert!(drafts >= 2);
}

fn wait_for_near(run: &mut Run) -> Vec<Ev> {
    let mut all = Vec::new();
    for _ in 0..400 {
        let evs = run.update(0.05);
        let near = evs.iter().any(|e| matches!(e, Ev::GhostNear(_)));
        all.extend(evs);
        if near {
            // Past the grace for a step already pressed as the stab sounds.
            all.extend(run.update(STAB_GRACE));
            return all;
        }
    }
    panic!("the ghost never came near: {all:?}");
}

#[test]
fn moving_while_the_ghost_passes_costs_courage_and_sends_you_back() {
    let mut run = floor_run(1);
    solve_run_room(&mut run);
    run.walk(Side::North);
    run.summon(Some(GhostKind::Widow), Some(0.0));
    wait_for_near(&mut run);
    assert_eq!(run.ghost.unwrap().phase, GhostPhase::Near);
    let evs = run.walk(Side::West);
    assert_eq!(
        evs,
        vec![Ev::Caught {
            by: Threat::Ghost(GhostKind::Widow),
            courage: COURAGE - 1
        }]
    );
    let room = run.current();
    assert_eq!((run.x, run.y), room.inside(room.entry));
    assert!(run.ghost.is_none());
}

#[test]
fn keeping_still_lets_the_ghost_pass_for_points() {
    let mut run = floor_run(1);
    solve_run_room(&mut run);
    let score = run.score;
    run.summon(Some(GhostKind::Butler), Some(1.0));
    wait_for_near(&mut run);
    // Picking up and checking are quiet: only footsteps give you away.
    run.interact();
    run.cycle();
    let mut passed = false;
    for _ in 0..400 {
        if run.update(0.05).iter().any(|e| {
            matches!(
                e,
                Ev::GhostPassed {
                    kind: GhostKind::Butler,
                    points: 25
                }
            )
        }) {
            passed = true;
            break;
        }
    }
    assert!(passed);
    assert_eq!(run.ghosts_evaded, 1);
    assert_eq!(run.score, score + 25);
    assert_eq!(run.courage, COURAGE);
}

#[test]
fn walking_while_it_is_still_far_is_safe() {
    let mut run = floor_run(1);
    run.summon(Some(GhostKind::Child), Some(2.0));
    run.update(0.1);
    assert_eq!(run.ghost.unwrap().phase, GhostPhase::Approaching);
    assert_eq!(run.walk(Side::North)[0], Ev::Step);
}

#[test]
fn three_catches_chase_you_out() {
    let mut run = floor_run(1);
    for left in (0..COURAGE).rev() {
        run.summon(None, None);
        wait_for_near(&mut run);
        let evs = run.walk(Side::North);
        assert!(evs.contains(&Ev::Caught {
            by: Threat::Ghost(evs_kind(&evs)),
            courage: left
        }));
    }
    assert_eq!(run.outcome, Some(Outcome::ChasedOut));
    assert!(
        run.walk(Side::North).is_empty(),
        "nothing moves once it is over"
    );
}

fn evs_kind(evs: &[Ev]) -> GhostKind {
    evs.iter()
        .find_map(|e| match e {
            Ev::Caught {
                by: Threat::Ghost(kind),
                ..
            } => Some(*kind),
            _ => None,
        })
        .unwrap()
}

#[test]
fn ghosts_come_on_their_own_but_not_in_the_very_first_room() {
    let mut run = floor_run(1);
    for _ in 0..1200 {
        assert!(!run
            .update(0.1)
            .iter()
            .any(|e| matches!(e, Ev::GhostComing(_))));
    }
    let mut run = floor_run(1);
    solve_run_room(&mut run);
    let mut came = false;
    for _ in 0..1200 {
        if run
            .update(0.1)
            .iter()
            .any(|e| matches!(e, Ev::GhostComing(_)))
        {
            came = true;
            break;
        }
    }
    assert!(came);
}

#[test]
fn the_ghost_is_heard_from_its_direction_and_grows_near() {
    let mut run = floor_run(1);
    let g = run.summon(Some(GhostKind::Widow), Some(std::f32::consts::FRAC_PI_2));
    assert_eq!(g.from_name(), "east");
    let (x, _) = g.offset();
    assert!(x > 10.0, "it starts out east: {x}");
    run.update(2.0);
    let (x2, _) = run.ghost.unwrap().offset();
    assert!(x2 < x);
}

#[test]
fn the_clock_tolls_and_then_the_house_keeps_you() {
    let mut run = floor_run(1);
    let mut warnings = Vec::new();
    let mut ended = None;
    for _ in 0..3000 {
        for ev in run.update(0.1) {
            match ev {
                Ev::Warning(s) => warnings.push(s),
                Ev::Ended(o) => ended = Some(o),
                _ => {}
            }
        }
        if ended.is_some() {
            break;
        }
    }
    assert_eq!(warnings, vec![60.0, 30.0, 10.0]);
    assert_eq!(ended, Some(Outcome::TimeUp));
    assert!(run.update(1.0).is_empty());
}

#[test]
fn a_hint_points_the_way_for_five_seconds() {
    let mut run = floor_run(1);
    let before = run.haunt_left;
    let evs = run.hint();
    let Ev::Hint(text) = &evs[0] else {
        panic!("{evs:?}")
    };
    let target = run.current().target().unwrap();
    assert!(text.contains(target.item.name()), "{text}");
    assert!(text.contains("step"), "{text}");
    assert_eq!(run.haunt_left, before - floor_spec(1, false).hint_cost);
    assert_eq!(run.hints, 1);
}

#[test]
fn clearing_a_floor_scores_time_and_courage() {
    let mut run = floor_run(1);
    let mut evs = Vec::new();
    for _ in 0..3 {
        evs.extend(solve_run_room(&mut run));
    }
    let bonus = evs
        .iter()
        .find_map(|e| match e {
            Ev::FloorCleared { bonus, .. } => Some(*bonus),
            _ => None,
        })
        .expect("the floor is cleared");
    assert_eq!(
        bonus,
        floor_spec(1, false).haunt as u64 * 5 + COURAGE as u64 * 100
    );
    assert_eq!(run.score, 300 + bonus);
    assert!(evs.contains(&Ev::Ended(Outcome::Escaped)));
    assert_eq!(run.floor_times.len(), 1);
}

#[test]
fn the_climb_goes_floor_by_floor_to_the_top() {
    let mut run = Run::new(Mode::Climb(1), SEED);
    for floor in 1..=4 {
        assert_eq!(run.spec.number, floor);
        assert_eq!(run.courage, COURAGE);
        assert!(run.inventory.is_empty());
        let rooms = run.rooms.len();
        for _ in 0..rooms {
            run.update(0.2);
            solve_run_room(&mut run);
        }
        assert!(run.next_floor_waiting);
        assert!(
            run.walk(Side::North).is_empty(),
            "nothing moves between floors"
        );
        let evs = run.next_floor();
        assert!(matches!(evs[0], Ev::FloorBegins { number, .. } if number == floor + 1));
    }
    assert_eq!(run.deepest, 4);
    // A climb from near the top ends at floor 100.
    let mut run = Run::new(Mode::Climb(FLOORS), SEED);
    assert_eq!(run.mode.last_floor(), FLOORS);
    for _ in 0..run.rooms.len() {
        solve_run_room(&mut run);
    }
    assert_eq!(run.outcome, Some(Outcome::Escaped));
    assert!(run.total_time >= 0.0);
}

#[test]
fn words_for_directions_and_times() {
    assert_eq!(directions(0, 0), "right here");
    assert_eq!(directions(1, 2), "2 steps north and 1 step east");
    assert_eq!(directions(-3, -1), "1 step south and 3 steps west");
    assert_eq!(clock_words(240.0), "4 minutes");
    assert_eq!(clock_words(61.0), "1 minute 1 second");
    assert_eq!(clock_words(9.2), "10 seconds");
    assert_eq!(time_short(65.34), "1 minute 5.3 seconds");
    assert_eq!(time_short(1.5), "1.5 seconds");
    assert_eq!(compass(0.0), "north");
    assert_eq!(compass(std::f32::consts::PI), "south");
}

#[test]
fn progress_keeps_the_faster_time_and_opens_floors() {
    let mut p = Progress::default();
    assert_eq!(p.open_floors(), 1);
    let first = p.floor_done(Mode::Floor(1), 1, 100.0);
    assert!(first.record && first.previous.is_none());
    assert_eq!(first.opened, Some(2));
    let slower = p.floor_done(Mode::Floor(1), 1, 120.0);
    assert!(!slower.record);
    assert_eq!(p.floor_time(1), Some(100.0));
    let faster = p.floor_done(Mode::Climb(1), 1, 90.0);
    assert!(faster.record);
    assert_eq!(faster.previous, Some(100.0));
    assert_eq!(faster.opened, None);
    assert_eq!(p.open_floors(), 2);
    assert_eq!(p.frontier(), 2);
    for f in 2..=FLOORS {
        let s = p.floor_done(Mode::Climb(2), f, 50.0);
        assert_eq!(s.nightmare, f == progress::NIGHTMARE_AT);
        assert_eq!(s.endless, f == FLOORS);
    }
    assert!(p.nightmare_open() && p.endless_open());
    assert_eq!(p.open_floors(), FLOORS);
    assert_eq!(p.escapes, 1);
    // Challenges, Nightmare and Endless night leave best times alone.
    let before = p.floor_best.clone();
    p.floor_done(Mode::Nightmare, 30, 1.0);
    p.floor_done(Mode::Endless, 120, 1.0);
    assert_eq!(p.floor_best, before);
    assert_eq!((p.nightmare_deepest, p.endless_deepest), (30, 120));
}

#[test]
fn the_ghost_is_gone_the_moment_it_is_past_and_walking_is_safe_again() {
    let mut run = floor_run(1);
    solve_run_room(&mut run);
    run.summon(Some(GhostKind::Widow), Some(0.0));
    wait_for_near(&mut run);
    let mut t = 0.0;
    loop {
        let evs = run.update(0.05);
        t += 0.05;
        if evs.iter().any(|e| matches!(e, Ev::GhostPassed { .. })) {
            break;
        }
        assert!(t < 30.0, "it never passed");
        assert_eq!(run.ghost.unwrap().phase, GhostPhase::Near);
    }
    // Let go right after the near window, not after it fades far away.
    let near_window = 2.0 * GHOST_NEAR / GhostKind::Widow.speed();
    assert!(t <= near_window as f64 + 0.2, "{t} vs {near_window}");
    assert!(run.ghost.is_none());
    // A step that stays inside the room, whichever way this room lies.
    let room = run.current().clone();
    let side = *Side::ALL
        .iter()
        .find(|s| {
            let (dx, dy) = s.delta();
            let (x, y) = (run.x + dx, run.y + dy);
            x >= 0 && y >= 0 && x < room.w && y < room.h
        })
        .unwrap();
    assert_eq!(run.walk(side)[0], Ev::Step);
    assert_eq!(run.courage, COURAGE);
}

#[test]
fn a_penalty_past_a_toll_mark_tolls_once_with_the_true_time() {
    let mut run = floor_run(1);
    run.update(run.haunt_left - 35.0);
    assert!((run.haunt_left - 35.0).abs() < 1e-6);
    // Two wrong touches' worth: past 30 at once.
    run.haunt_left -= 2.0 * WRONG_PENALTY;
    let warnings: Vec<f64> = run
        .update(0.1)
        .into_iter()
        .filter_map(|e| match e {
            Ev::Warning(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(warnings, vec![15.0]);
    // From 15 seconds straight past 10 seconds.
    run.haunt_left = 9.5;
    let warnings: Vec<f64> = run
        .update(0.1)
        .into_iter()
        .filter_map(|e| match e {
            Ev::Warning(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(warnings, vec![10.0]);
}

#[test]
fn the_way_to_the_exit_ends_with_its_wall() {
    use haunted_hallways::rules::Run;
    let mut run = Run::practice(haunted_hallways::tutorial::rooms());
    // The foyer's exit is in the north wall, at column 3.
    run.x = 2;
    run.y = 2;
    assert_eq!(run.exit_words(), "2 steps north");
    run.x = 0;
    assert_eq!(run.exit_words(), "2 steps east and 2 steps north");
    run.y = 4;
    assert_eq!(run.exit_words(), "2 steps east, then north");
    run.x = 2;
    assert_eq!(run.exit_words(), "north of you");
    // The library's is in the east wall, at row 4.
    run.room = 1;
    run.x = 4;
    run.y = 1;
    assert_eq!(run.exit_words(), "2 steps north, then east");
}
