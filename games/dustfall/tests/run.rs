//! Whole nights and whole colonies through the rules: a watched run always
//! ends, the same inputs replay the same run, and each kind of raider does
//! what it says.

use dustfall::rules::colony::MINE_COST;
use dustfall::rules::lanes::CARRIER_HEIGHT;
use dustfall::rules::nights::RELIEF_NIGHT;
use dustfall::rules::raiders::State;
use dustfall::rules::{Building, Event, Hitter, Input, Kind, Phase, Run};

fn has(events: &[Event], f: impl Fn(&Event) -> bool) -> bool {
    events.iter().any(f)
}

/// Spends the colony phase: a flak post and power first, then missiles, a
/// mine in the centre, and the dome when it is hurt.
fn shopping(run: &Run) -> Vec<Input> {
    let mut inputs = Vec::new();
    let c = &run.colony;
    let free: Vec<u8> = c
        .open
        .iter()
        .copied()
        .filter(|l| c.slot(*l).is_none())
        .collect();
    let wants = [
        Building::Solar,
        Building::Flak,
        Building::Hydroponics,
        Building::ShieldCoil,
    ];
    let mut r = c.resources;
    for (building, lane) in wants.iter().zip(free) {
        let (metal, power) = building.cost();
        if r.covers(power, metal) {
            r.spend(power, metal);
            inputs.push(Input::Buy {
                building: *building,
                lane,
            });
        }
    }
    if c.dome < c.dome_max && r.metal >= 5 {
        r.spend(0, 5);
        inputs.push(Input::RepairDome);
    }
    if !c.mined(2) && r.metal >= MINE_COST {
        r.spend(0, MINE_COST);
        inputs.push(Input::LayMine { lane: 2 });
    }
    let (power, metal) = c.missile_cost();
    let mut missiles = c.missiles;
    while missiles < c.missiles_max && r.covers(power, metal) {
        r.spend(power, metal);
        missiles += 1;
        inputs.push(Input::MakeMissile);
    }
    inputs
}

/// The gun's next input, if any: a missile at a mothership, else slide
/// under the nearest raider and fire.
fn act(run: &Run) -> Option<Input> {
    let nearest = run.nearest_first();
    let target = nearest.first()?;
    if run.colony.missiles > 0 && target.kind.armoured() && target.hits > 4 {
        return Some(Input::Missile);
    }
    if target.lane < run.gun {
        Some(Input::Left)
    } else if target.lane > run.gun {
        Some(Input::Right)
    } else if run.cooldown == 0 {
        Some(Input::Fire)
    } else {
        None
    }
}

/// Plays `run` to its end, returning what was done and when.
fn watch(run: &mut Run) -> Vec<(u32, Input)> {
    let mut log = Vec::new();
    let mut ticks = 0u32;
    while !run.is_over() {
        ticks += 1;
        assert!(
            ticks < 200_000,
            "a run that never ends on night {}",
            run.night
        );
        match run.phase {
            Phase::Colony => {
                for input in shopping(run) {
                    if run.apply(input).is_ok() {
                        log.push((ticks, input));
                    }
                }
                run.apply(Input::Ready).unwrap();
                log.push((ticks, Input::Ready));
            }
            Phase::Night => {
                if let Some(input) = act(run) {
                    if run.apply(input).is_ok() {
                        log.push((ticks, input));
                    }
                }
                run.tick();
            }
            _ => break,
        }
        check_events(&run.take_events());
    }
    log
}

/// Every event's numbers hold together.
fn check_events(events: &[Event]) {
    for e in events {
        match e {
            Event::Shot { shield, dome, .. } | Event::Breach { shield, dome, .. } => {
                assert!(*shield <= 70 && *dome <= 110, "{e:?}");
            }
            Event::Hit { left, .. } => assert!(*left > 0, "{e:?}"),
            Event::Arrived { lane, .. } | Event::Dropped { lane, .. } => {
                assert!(*lane < 5, "{e:?}");
            }
            _ => {}
        }
    }
}

#[test]
fn a_watched_run_always_ends() {
    for seed in 1..=6 {
        let mut run = Run::new(seed);
        watch(&mut run);
        assert!(run.is_over(), "seed {seed}: {:?}", run.phase);
        assert!(
            run.nights_held >= 2,
            "seed {seed} fell on night {}",
            run.night
        );
    }
}

#[test]
fn the_same_seed_and_inputs_replay_the_same_run() {
    let mut first = Run::new(11);
    let log = watch(&mut first);
    let mut again = Run::new(11);
    let mut ticks = 0u32;
    let mut at = 0;
    while !again.is_over() {
        ticks += 1;
        while at < log.len() && log[at].0 == ticks {
            again.apply(log[at].1).unwrap();
            at += 1;
        }
        if again.phase == Phase::Night {
            again.tick();
        }
        again.take_events();
        assert!(ticks < 200_000);
    }
    assert_eq!(at, log.len(), "every input was replayed");
    assert_eq!(again.phase, first.phase);
    assert_eq!(again.score, first.score);
    assert_eq!(again.night, first.night);
    assert_eq!(again.colony, first.colony);
}

#[test]
fn a_mothership_holds_at_mid_range_shifts_lanes_and_drops_drifters() {
    let mut run = Run::at_night(4, 5);
    run.apply(Input::Ready).unwrap();
    // Only the mothership's wave.
    run.waves
        .retain(|w| w.iter().any(|a| a.kind == Kind::Mothership));
    assert_eq!(run.waves.len(), 1);
    run.take_events();
    let mut seen = (false, false, false, false);
    for _ in 0..3000 {
        if run.phase != Phase::Night {
            break;
        }
        run.tick();
        for e in run.take_events() {
            match e {
                Event::Hovering { .. } => seen.0 = true,
                Event::Shifted { .. } => seen.1 = true,
                Event::Dropped {
                    kind: Kind::Drifter,
                    ..
                } => seen.2 = true,
                Event::Shot { dodged: false, .. } => seen.3 = true,
                _ => {}
            }
        }
        if seen == (true, true, true, true) {
            break;
        }
    }
    assert_eq!(
        seen,
        (true, true, true, true),
        "hovers, shifts, drops, sieges"
    );
    let ship = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Mothership)
        .unwrap();
    assert_eq!(ship.state, State::Hovering);
    assert!(run.colony.dome < 100, "siege shots cannot be dodged");
}

#[test]
fn a_missile_only_dents_a_mothership() {
    let mut run = Run::at_night(4, 5);
    run.colony.resources.power = 50;
    run.colony.resources.metal = 50;
    run.apply(Input::MakeMissile).unwrap();
    run.apply(Input::Ready).unwrap();
    run.waves.clear();
    run.take_events();
    run.raiders
        .push(dustfall::rules::Raider::new(50, Kind::Mothership, 2, 700));
    run.apply(Input::Missile).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Hit {
                kind: Kind::Mothership,
                left: 4,
                by: Hitter::Missile,
                ..
            }
        )),
        "{events:?}"
    );
}

#[test]
fn a_mule_crosses_the_sky_and_leaves_unless_it_is_shot() {
    let mut run = Run::at_night(4, 4);
    run.apply(Input::Ready).unwrap();
    run.waves.retain(|w| w.iter().any(|a| a.kind == Kind::Mule));
    run.take_events();
    let mut crossed = 0;
    let mut left = false;
    for _ in 0..2000 {
        run.tick();
        for e in run.take_events() {
            match e {
                Event::Crossed {
                    kind: Kind::Mule, ..
                } => crossed += 1,
                Event::Left {
                    kind: Kind::Mule, ..
                } => left = true,
                _ => {}
            }
        }
        if left {
            break;
        }
    }
    assert!(left, "the mule left");
    assert_eq!(crossed, 4, "four lanes crossed from one edge to the other");
    assert_eq!(run.colony.dome, 100, "a mule never attacks");
    assert_eq!(run.phase, Phase::Colony, "a mule gone is a wave down");
}

#[test]
fn the_carrier_turns_at_the_edge_and_lets_skitters_down() {
    let mut run = Run::at_night(4, RELIEF_NIGHT);
    run.apply(Input::Ready).unwrap();
    run.waves
        .retain(|w| w.iter().any(|a| a.kind == Kind::Carrier));
    run.take_events();
    let mut crossed = 0;
    let mut dropped = 0;
    for _ in 0..400 {
        run.tick();
        for e in run.take_events() {
            match e {
                Event::Crossed {
                    kind: Kind::Carrier,
                    ..
                } => crossed += 1,
                Event::Dropped {
                    kind: Kind::Skitter,
                    ..
                } => dropped += 1,
                _ => {}
            }
        }
    }
    let carrier = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Carrier)
        .unwrap();
    assert_eq!(carrier.height, CARRIER_HEIGHT);
    assert!(
        crossed > 4,
        "it turned at the edge and came back: {crossed}"
    );
    assert!(dropped >= 2, "skitters let down: {dropped}");
}

#[test]
fn a_sapper_takes_the_lane_over_the_hydroponics_and_darkens_it() {
    let mut run = Run::at_night(4, 6);
    run.apply(Input::Buy {
        building: Building::Hydroponics,
        lane: 1,
    })
    .unwrap();
    run.apply(Input::Ready).unwrap();
    assert!(
        run.waves
            .iter()
            .flatten()
            .any(|a| a.kind == Kind::Sapper && a.lane == 1),
        "the sapper heads for the hydroponics: {:?}",
        run.waves
    );
    run.take_events();
    let mut sapper_breached = false;
    let mut darkened = Vec::new();
    for _ in 0..5000 {
        if run.phase != Phase::Night {
            break;
        }
        run.tick();
        for e in run.take_events() {
            if let Event::Breach {
                kind,
                lane,
                darkened: d,
                ..
            } = e
            {
                if kind == Kind::Sapper {
                    sapper_breached = true;
                    assert_eq!(lane, 1);
                }
                if let Some(b) = d {
                    darkened.push((lane, b));
                }
            }
        }
    }
    assert!(sapper_breached);
    assert_eq!(
        darkened,
        vec![(1, Building::Hydroponics)],
        "darkened once, by whoever got there first"
    );
    assert!(run.colony.slot(1).unwrap().dark);
    assert_eq!(run.apply(Input::RepairBuilding { lane: 1 }), Ok(()));
    assert!(!run.colony.slot(1).unwrap().dark);
}

#[test]
fn relief_adds_the_stores_and_ends_the_run() {
    let mut run = Run::prepared(9, 1, 1);
    assert_eq!(run.nights(), 1);
    watch(&mut run);
    assert_eq!(run.phase, Phase::Relief);
    let r = run.colony.resources;
    assert_eq!(
        run.score,
        2 * Kind::Drifter.score() + 100 + u64::from(r.power + r.metal)
    );
    assert_eq!(run.tech, 1 + 5);
}
