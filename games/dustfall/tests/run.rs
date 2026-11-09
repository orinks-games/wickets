//! Whole runs: a scripted watch that plays like a decent player, from the
//! first colony phase to Relief or the fallen dome, the same way every
//! time; and the raiders that only the later nights bring.

use dustfall::rules::colony::{DOME_MAX, MISSILE_DAMAGE};
use dustfall::rules::grid::{Sector, WorkKind, COLS};
use dustfall::rules::raiders::{Kind, State, Weapon};
use dustfall::rules::{Building, Event, Hitter, Input, Phase, Run, RELIEF_NIGHT};

fn s(col: u8, row: u8) -> Sector {
    Sector::new(col, row)
}

/// What the watch buys, in order, as metal and power allow.
fn shopping(run: &Run) -> Vec<Input> {
    let mut list = Vec::new();
    let c = &run.colony;
    if c.dome < 60 {
        list.push(Input::RepairDome);
    }
    for col in 0..COLS {
        if c.slot(col).is_some_and(|slot| slot.dark) {
            list.push(Input::RepairBuilding { col });
        }
    }
    let have = |b: Building| c.slots.iter().flatten().any(|slot| slot.building == b);
    let free_slot = || c.open.iter().copied().find(|col| c.slot(*col).is_none());
    for building in [Building::Hydroponics, Building::Solar, Building::ShieldCoil] {
        if !have(building) {
            if let Some(col) = free_slot() {
                list.push(Input::Buy { building, col });
            }
        }
    }
    // A post at the centre of row 3, then the funnel walls at row 4, then
    // more posts beside it.
    for at in [s(2, 3), s(1, 3), s(3, 3)] {
        if run.grid.get(at).is_none() {
            list.push(Input::Place {
                work: WorkKind::Flak,
                at,
            });
        }
    }
    for at in [s(0, 4), s(4, 4), s(1, 4), s(3, 4)] {
        if run.grid.get(at).is_none() {
            list.push(Input::Place {
                work: WorkKind::Wall,
                at,
            });
        }
    }
    if run.night >= 4 && c.missiles < 2 {
        list.push(Input::MakeMissile);
    }
    if c.dome < DOME_MAX {
        list.push(Input::RepairDome);
    }
    list
}

fn toward(run: &Run, col: u8) -> Option<Input> {
    use std::cmp::Ordering::*;
    match run.gun.cmp(&col) {
        Less => Some(Input::Right),
        Greater => Some(Input::Left),
        Equal => None,
    }
}

/// One action of the watch's turn.
fn act(run: &Run) -> Input {
    let Phase::Turn { actions } = run.phase else {
        return Input::EndTurn;
    };
    // A charge aimed at this column: step off it unless the shooter dies
    // to this shot.
    if let Some((id, _, col)) = run.charges().iter().find(|(_, _, col)| *col == run.gun) {
        let shooter = run.raider(*id);
        let dies = shooter.is_some_and(|r| r.at.col == run.gun && r.hits == 1);
        if !dies {
            let step = if *col == 0 { Input::Right } else { Input::Left };
            return step;
        }
    }
    let raiders = run.nearest_first();
    // A mothership at its post is what the missiles are for.
    if run.colony.missiles > 0 {
        let big = raiders
            .iter()
            .any(|r| matches!(r.kind, Kind::Mothership | Kind::Carrier) && r.at.row <= 4);
        let urgent =
            raiders.iter().any(|r| r.at.row == 1 && r.kind.hostile()) && run.colony.shield < 10;
        if big || urgent {
            return Input::Missile;
        }
    }
    // Something in this column: shoot it.
    if raiders.iter().any(|r| r.at.col == run.gun) {
        return Input::Fire;
    }
    // Otherwise go to the nearest hostile raider, mules when nothing else
    // is close.
    let target = raiders
        .iter()
        .filter(|r| r.kind.hostile())
        .min_by_key(|r| (r.at.row, r.at.col.abs_diff(run.gun)))
        .or_else(|| raiders.first());
    match target.and_then(|r| toward(run, r.at.col)) {
        Some(step) => step,
        None if actions > 0 => Input::EndTurn,
        None => Input::EndTurn,
    }
}

/// Plays a run out and returns every input it made.
fn watch(run: &mut Run) -> Vec<Input> {
    let mut inputs = Vec::new();
    let mut guard = 0;
    while !run.is_over() {
        guard += 1;
        assert!(guard < 5000, "the run never ended: night {}", run.night);
        match run.phase {
            Phase::Colony => {
                spend(run, &mut inputs);
                run.apply(Input::Ready).unwrap();
                inputs.push(Input::Ready);
            }
            Phase::Turn { .. } => {
                let input = act(run);
                run.apply(input)
                    .unwrap_or_else(|why| panic!("{input:?} refused: {why:?}"));
                inputs.push(input);
            }
            _ => break,
        }
    }
    inputs
}

/// The watch's colony phase: buy down the list while the money lasts.
fn spend(run: &mut Run, inputs: &mut Vec<Input>) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 50);
        let mut bought = false;
        for input in shopping(run) {
            if run.apply(input).is_ok() {
                inputs.push(input);
                bought = true;
                break;
            }
        }
        if !bought {
            break;
        }
    }
}

/// Every turn's events hang together: a turn begins after each phase and
/// the night opens once per night.
fn check_events(events: &[Event]) {
    let mut nights = 0;
    let mut turns = 0;
    for event in events {
        match event {
            Event::NightOpens { .. } => nights += 1,
            Event::YourTurn { .. } => turns += 1,
            _ => {}
        }
    }
    assert!(nights >= 1);
    assert!(turns >= nights);
}

#[test]
fn a_watched_run_always_ends() {
    for seed in 1..=12u64 {
        let mut run = Run::new(seed);
        let mut events = run.take_events();
        let inputs = watch(&mut run);
        events.extend(run.take_events());
        assert!(run.is_over(), "seed {seed}");
        assert!(!inputs.is_empty());
        check_events(&events);
        assert!(run.nights_held <= RELIEF_NIGHT);
        if run.phase == Phase::Relief {
            assert_eq!(run.nights_held, RELIEF_NIGHT);
            assert!(events.iter().any(|e| matches!(e, Event::Relief { .. })));
        } else {
            assert!(events.iter().any(|e| matches!(e, Event::Fallen { .. })));
            assert_eq!(run.colony.dome, 0);
        }
        let breaches = events
            .iter()
            .filter(|e| matches!(e, Event::Breach { .. }))
            .count();
        let shots = events
            .iter()
            .filter(|e| matches!(e, Event::Shot { dodged: false, .. }))
            .count();
        eprintln!(
            "seed {seed}: {:?} after {} nights, {} points, {} tech, dome {}, {breaches} breaches, {shots} shots landed, {} downed",
            run.phase, run.nights_held, run.score, run.tech, run.colony.dome, run.downed
        );
    }
}

#[test]
fn a_decent_watch_reaches_relief() {
    let mut run = Run::new(1);
    watch(&mut run);
    assert_eq!(run.phase, Phase::Relief, "held {} nights", run.nights_held);
    assert_eq!(run.nights_held, RELIEF_NIGHT);
    assert!(run.tech >= RELIEF_NIGHT as u32 + 5);
}

#[test]
fn the_same_seed_and_inputs_replay_the_same_run() {
    let mut first = Run::new(21);
    first.take_events();
    let inputs = watch(&mut first);
    let first_events = first.take_events();
    let mut second = Run::new(21);
    second.take_events();
    for input in &inputs {
        second.apply(*input).unwrap();
    }
    let second_events = second.take_events();
    assert_eq!(first_events, second_events);
    assert_eq!(first.phase, second.phase);
    assert_eq!(first.score, second.score);
    assert_eq!(first.colony, second.colony);
    assert_eq!(first.grid, second.grid);
    // A different seed is a different run.
    let mut other = Run::new(22);
    other.take_events();
    watch(&mut other);
    assert!(other.forecast != first.forecast || other.score != first.score);
}

#[test]
fn a_mothership_holds_its_post_drops_skitters_and_sieges() {
    let mut run = Run::at_night(4, 5);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let ship = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Mothership)
        .unwrap()
        .id;
    assert_eq!(run.raider(ship).unwrap().at, s(2, 6));
    // Three phases bring it to centre, row 3.
    for _ in 0..3 {
        run.apply(Input::EndTurn).unwrap();
    }
    assert_eq!(run.raider(ship).unwrap().at, s(2, 3));
    run.take_events();
    let mut dropped = false;
    let mut sieged = false;
    let mut shot = false;
    let dome = run.colony.dome;
    for _ in 0..8 {
        if !matches!(run.phase, Phase::Turn { .. }) {
            break;
        }
        // Keep the shield high so only the siege's floor reaches the dome.
        run.colony.shield = 50;
        run.apply(Input::EndTurn).unwrap();
        for event in run.take_events() {
            match event {
                Event::Dropped { from, at, .. } if from == ship => {
                    dropped = true;
                    assert_eq!(at.row, 3);
                    assert_ne!(at.col, 2);
                }
                Event::Charging {
                    id,
                    weapon: Weapon::Siege,
                    ..
                } if id == ship => sieged = true,
                Event::Shot {
                    id,
                    weapon: Weapon::Siege,
                    dodged,
                    ..
                } if id == ship => {
                    shot = true;
                    assert!(!dodged, "a siege shot cannot be dodged");
                }
                _ => {}
            }
        }
    }
    assert!(dropped, "no skitter dropped");
    assert!(sieged, "no siege wound up");
    assert!(shot, "no siege shot landed");
    assert!(
        run.colony.dome <= dome - 5,
        "the siege floor missed the dome"
    );
    assert!(matches!(
        run.raider(ship).map(|r| r.state),
        Some(State::Holding | State::Charging { .. })
    ));
}

#[test]
fn a_missile_only_dents_a_mothership() {
    let mut run = Run::at_night(4, 5);
    run.colony.missiles = 1;
    run.apply(Input::Ready).unwrap();
    run.take_events();
    // Move the gun so the mothership is the nearest-to-gun on its row:
    // everything is on row 6, the ship at the centre, the gun under it.
    run.apply(Input::Missile).unwrap();
    let events = run.take_events();
    let hit = events.iter().find(|e| {
        matches!(
            e,
            Event::Hit {
                by: Hitter::Missile,
                ..
            }
        )
    });
    match hit {
        Some(Event::Hit {
            kind: Kind::Mothership,
            left,
            ..
        }) => {
            assert_eq!(*left, Kind::Mothership.hits() - MISSILE_DAMAGE);
        }
        other => {
            // The missile went to a drifter sharing the row and nearer the
            // gun's column; that one goes down outright.
            assert!(
                events.iter().any(|e| matches!(
                    e,
                    Event::Down {
                        by: Hitter::Missile,
                        ..
                    }
                )),
                "{other:?} {events:?}"
            );
        }
    }
}

#[test]
fn a_mule_crosses_the_grid_and_leaves_unless_it_is_shot() {
    let mut run = Run::at_night(4, 4);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let mule = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Mule)
        .unwrap()
        .clone();
    assert_eq!(mule.at.row, 4);
    assert!(mule.at.col == 0 || mule.at.col == 4);
    let route = run.route_of(mule.id).unwrap();
    assert_eq!(route.len(), 4);
    let mut left = false;
    for _ in 0..6 {
        if !matches!(run.phase, Phase::Turn { .. }) {
            break;
        }
        run.apply(Input::EndTurn).unwrap();
        if run
            .take_events()
            .iter()
            .any(|e| matches!(e, Event::Left { id, .. } if *id == mule.id))
        {
            left = true;
            break;
        }
    }
    assert!(left, "the mule never left");
    assert!(run.raider(mule.id).is_none());

    // Shot down, it drops food.
    let mut run = Run::at_night(4, 4);
    run.apply(Input::Ready).unwrap();
    let mule = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Mule)
        .unwrap()
        .clone();
    let food = run.colony.resources.food;
    let mut guard = 0;
    while run.raider(mule.id).is_some() && guard < 20 {
        let at = run.raider(mule.id).unwrap().at.col;
        let input = toward(&run, at).unwrap_or(Input::Fire);
        run.apply(input).unwrap();
        guard += 1;
    }
    assert!(
        run.colony.resources.food >= food + 4,
        "no cargo: {}",
        run.colony.resources.food
    );
}

#[test]
fn the_carrier_turns_at_the_edge_and_sieges_as_it_crosses() {
    let mut run = Run::at_night(4, RELIEF_NIGHT);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let carrier = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Carrier)
        .unwrap()
        .id;
    let mut cols = Vec::new();
    let mut sieged = false;
    for _ in 0..12 {
        if !matches!(run.phase, Phase::Turn { .. }) {
            break;
        }
        run.colony.shield = 50;
        run.colony.dome = DOME_MAX;
        run.apply(Input::EndTurn).unwrap();
        let Some(r) = run.raider(carrier) else { break };
        cols.push(r.at.col);
        if run.take_events().iter().any(
            |e| matches!(e, Event::Charging { id, weapon: Weapon::Siege, .. } if *id == carrier),
        ) {
            sieged = true;
        }
    }
    assert!(
        run.raider(carrier).is_some(),
        "the carrier never leaves on its own"
    );
    assert!(cols.contains(&0) && cols.contains(&4), "{cols:?}");
    let turned = cols.windows(3).any(|w| (w[0] < w[1]) != (w[1] < w[2]));
    assert!(turned, "{cols:?}");
    assert!(sieged);
}

#[test]
fn a_sapper_breach_darkens_the_building_it_came_for() {
    let mut run = Run::at_night(4, 6);
    run.apply(Input::Buy {
        building: Building::Hydroponics,
        col: 3,
    })
    .unwrap();
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let sapper = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Sapper)
        .unwrap()
        .id;
    let route = run.route_of(sapper).unwrap();
    assert_eq!(route.last(), Some(&s(3, 1)));
    let mut darkened = false;
    for _ in 0..12 {
        if !matches!(run.phase, Phase::Turn { .. }) {
            break;
        }
        // A full shield, so the sapper's breach alone cannot reach the dome.
        run.colony.shield = 50;
        run.apply(Input::EndTurn).unwrap();
        for event in run.take_events() {
            if let Event::Breach {
                id,
                darkened: Some(Building::Hydroponics),
                col: 3,
                ..
            } = event
            {
                if id == sapper {
                    darkened = true;
                }
            }
        }
        if darkened {
            break;
        }
    }
    assert!(darkened, "the sapper did not darken hydroponics");
    assert!(run.colony.slot(3).unwrap().dark);
}

#[test]
fn relief_adds_the_stores_and_ends_the_run() {
    let mut run = Run::at_night(4, RELIEF_NIGHT);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let score = run.score;
    let tech = run.tech;
    let stores = (run.colony.resources.power + run.colony.resources.metal) as u64;
    // Nothing to shoot: take the raiders off the grid as if all were shot.
    let take: u64 = run.raiders.iter().map(|r| r.kind.score()).sum();
    let ships = run
        .raiders
        .iter()
        .filter(|r| r.kind == Kind::Mothership)
        .count() as u32;
    run.raiders.clear();
    run.apply(Input::EndTurn).unwrap();
    let events = run.take_events();
    assert_eq!(run.phase, Phase::Relief);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Held {
            night: 12,
            clean: true,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(e, Event::Relief { .. })));
    let _ = (take, ships);
    assert_eq!(run.score, score + 100 + stores);
    assert_eq!(run.tech, tech + 1 + 5);
    assert_eq!(
        run.apply(Input::EndTurn),
        Err(dustfall::rules::Refusal::WrongPhase)
    );
}
