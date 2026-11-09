//! The fourth build's rules: orders, quirks, perks, the long watch and the
//! dust storm, each through the rules alone.

use dustfall::rules::colony::{DOME_MAX, MISSILES_MAX, SHIELD_MAX, SHIELD_START};
use dustfall::rules::nights::{quirks_at_most, wave, QUIRKS_FROM};
use dustfall::rules::orders::{OVERCLOCK_POWER, OVERCLOCK_SHIELD, RATIONING_FOOD};
use dustfall::rules::perks::{BIG_SHIELD, DEEP_RACK, HARD_VACUUM_SHIELD, THICK_DOME};
use dustfall::rules::{
    Colony, Event, Input, Kind, Order, Perks, Phase, Quirk, Refusal, Run, Sector, Storm, WorkKind,
    COLS, RELIEF_NIGHT,
};

fn s(col: u8, row: u8) -> Sector {
    Sector::new(col, row)
}

fn has(events: &[Event], f: impl Fn(&Event) -> bool) -> bool {
    events.iter().any(f)
}

/// Shoots until the night is held: the gun goes under the nearest raider
/// and fires, and the turn ends when nothing is in reach.
fn hold(run: &mut Run) {
    for _ in 0..400 {
        if !matches!(run.phase, Phase::Turn { .. }) {
            return;
        }
        let target = run.nearest_first().first().map(|r| r.at.col);
        match target {
            None => run.apply(Input::EndTurn).unwrap(),
            Some(col) if col == run.gun => {
                run.apply(Input::Fire).unwrap();
            }
            Some(col) if col < run.gun => run.apply(Input::Left).unwrap(),
            Some(_) => run.apply(Input::Right).unwrap(),
        }
    }
    panic!("the night never ended: {:?}", run.phase);
}

// ----- orders -----

#[test]
fn two_orders_are_offered_a_night_and_one_may_be_taken() {
    let run = Run::new(7);
    assert_eq!(run.offered.len(), 2);
    assert_eq!(
        run.offered,
        Run::new(7).offered,
        "the same for the same seed"
    );
    let mut run = run;
    let not_offered = Order::ALL
        .into_iter()
        .find(|o| !run.offered.contains(o))
        .unwrap();
    assert_eq!(
        run.apply(Input::TakeOrder(not_offered)),
        Err(Refusal::NoSuchOrder)
    );
    let taken = run.offered[0];
    run.apply(Input::TakeOrder(taken)).unwrap();
    assert_eq!(run.order, Some(taken));
    assert_eq!(
        run.apply(Input::TakeOrder(run.offered[1])),
        Err(Refusal::OrderTaken)
    );
    let events = run.take_events();
    assert!(has(
        &events,
        |e| matches!(e, Event::OrderTaken { order } if *order == taken)
    ));
}

fn with_order(seed: u64, order: Order) -> Run {
    let mut run = Run::new(seed);
    run.offered = vec![order];
    run
}

#[test]
fn overclock_charges_the_shield_now_and_costs_power_after_the_night() {
    let mut run = with_order(3, Order::Overclock);
    let power = run.colony.resources.power;
    run.apply(Input::TakeOrder(Order::Overclock)).unwrap();
    assert_eq!(run.colony.shield, SHIELD_START + OVERCLOCK_SHIELD);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    hold(&mut run);
    assert_eq!(run.phase, Phase::Colony);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::OrderPaid {
            order: Order::Overclock
        }
    )));
    // Two drifters' power came in, and the overclock's went out.
    assert_eq!(run.colony.resources.power, power + 2 - OVERCLOCK_POWER);
    assert_eq!(run.order, None, "the order is spent with the night");
    assert_eq!(run.offered.len(), 2, "two more for the next night");
}

#[test]
fn rationing_feeds_the_colony_and_the_raiders_step_first() {
    let mut run = with_order(3, Order::Rationing);
    let food = run.colony.resources.food;
    run.apply(Input::TakeOrder(Order::Rationing)).unwrap();
    assert_eq!(run.colony.resources.food, food + RATIONING_FOOD);
    run.apply(Input::Ready).unwrap();
    assert!(
        run.raiders.iter().all(|r| r.at.row == 5),
        "{:?}",
        run.raiders
    );
    let events = run.take_events();
    let steps = events
        .iter()
        .position(|e| matches!(e, Event::Stepped { .. }))
        .unwrap();
    let turn = events
        .iter()
        .position(|e| matches!(e, Event::YourTurn { .. }))
        .unwrap();
    assert!(
        steps < turn,
        "the steps come before the first turn: {events:?}"
    );
}

#[test]
fn salvage_doubles_the_metal_and_hurries_the_skitters() {
    let mut run = with_order(3, Order::Salvage);
    run.apply(Input::TakeOrder(Order::Salvage)).unwrap();
    assert_eq!(run.steps_of(Kind::Skitter), 3);
    assert_eq!(run.steps_of(Kind::Drifter), 1);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    hold(&mut run);
    let events = run.take_events();
    let downs: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::Down { .. }))
        .collect();
    assert_eq!(downs.len(), 2);
    for d in downs {
        let Event::Down { take, .. } = d else {
            unreachable!()
        };
        assert_eq!(take.metal, Kind::Drifter.drop().metal * 2, "{d:?}");
    }
}

// ----- quirks -----

#[test]
fn quirks_come_from_night_seven_and_never_more_than_two() {
    assert_eq!(quirks_at_most(QUIRKS_FROM - 1), 0);
    assert_eq!(quirks_at_most(QUIRKS_FROM), 1);
    assert_eq!(quirks_at_most(QUIRKS_FROM + 1), 1);
    assert_eq!(quirks_at_most(QUIRKS_FROM + 2), 2);
    for seed in 0..40u64 {
        for night in 1..=12u8 {
            let run = Run::at_night(seed, night);
            let quirks = run.forecast.iter().filter(|a| a.quirk.is_some()).count();
            assert!(quirks <= quirks_at_most(night), "seed {seed} night {night}");
            for a in &run.forecast {
                if let Some(q) = a.quirk {
                    assert!(q.fits(a.kind), "{a:?}");
                }
            }
        }
    }
    // Night 9 has quirks often enough to be heard of.
    let any = (0..40u64).any(|seed| Run::at_night(seed, 9).quirk_words().is_some());
    assert!(any);
}

/// A run before `night` whose forecast carries `quirk`, from the first
/// seed that does.
fn with_quirk(night: u8, quirk: Quirk) -> Run {
    (0..2000u64)
        .map(|seed| Run::at_night(seed, night))
        .find(|run| run.forecast.iter().any(|a| a.quirk == Some(quirk)))
        .unwrap_or_else(|| panic!("no seed brings {quirk:?} on night {night}"))
}

#[test]
fn an_armoured_raider_takes_a_hit_more_and_is_named() {
    let mut run = with_quirk(9, Quirk::Armoured);
    let words = run.quirk_words().unwrap();
    assert!(words.contains("an armoured"), "{words}");
    assert!(words.ends_with("among them."), "{words}");
    run.apply(Input::Ready).unwrap();
    let armoured = run
        .raiders
        .iter()
        .find(|r| r.quirk == Some(Quirk::Armoured))
        .unwrap();
    assert_eq!(armoured.hits, armoured.kind.hits() + 1);
    assert!(
        armoured.title().starts_with("Armoured "),
        "{}",
        armoured.title()
    );
    assert!(armoured.a().starts_with("an armoured "), "{}", armoured.a());
}

#[test]
fn a_hasty_raider_steps_before_the_rest() {
    let mut run = with_quirk(9, Quirk::Hasty);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let hasty = run
        .raiders
        .iter()
        .find(|r| r.quirk == Some(Quirk::Hasty))
        .unwrap()
        .id;
    run.apply(Input::EndTurn).unwrap();
    let events = run.take_events();
    let first_step = events
        .iter()
        .find_map(|e| match e {
            Event::Stepped { id, .. } => Some(*id),
            _ => None,
        })
        .unwrap();
    assert_eq!(first_step, hasty, "{events:?}");
}

#[test]
fn a_burrowing_raider_passes_under_one_wall_in_a_phase() {
    let mut run = with_quirk(9, Quirk::Burrowing);
    // Wall off row 5 whole, so every raider must go through.
    for col in 0..COLS {
        run.apply(Input::Place {
            work: WorkKind::Wall,
            at: s(col, 5),
        })
        .unwrap();
    }
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let burrower = run
        .raiders
        .iter()
        .find(|r| r.quirk == Some(Quirk::Burrowing))
        .unwrap()
        .clone();
    assert!(burrower.can_burrow());
    let landing = run.phases_to_land(burrower.id).unwrap();
    run.apply(Input::EndTurn).unwrap();
    let events = run.take_events();
    assert!(
        has(
            &events,
            |e| matches!(e, Event::Burrowed { id, .. } if *id == burrower.id)
        ),
        "{events:?}"
    );
    let after = run.raider(burrower.id).unwrap();
    assert_eq!(after.at.row, 5);
    assert!(after.burrowed && !after.can_burrow());
    assert!(run.grid.is_wall(after.at), "the wall still stands");
    // It counted one phase for the wall, where a basher counts two.
    let plain = run
        .raiders
        .iter()
        .find(|r| r.id != burrower.id && !r.kind.crosses())
        .map(|r| r.id);
    if let Some(other) = plain {
        assert!(run.phases_to_land(other).unwrap_or(0) + 1 >= landing);
    }
}

// ----- perks -----

#[test]
fn perks_set_the_colony_up() {
    let plain = Colony::new();
    assert_eq!(plain.dome_max, DOME_MAX);
    assert_eq!(plain.shield_max, SHIELD_MAX);
    assert_eq!(plain.missiles_max, MISSILES_MAX);
    assert_eq!(plain.open, vec![1, 2, 3]);
    let perks = Perks {
        extra_slots: 2,
        big_shield: true,
        thick_dome: true,
        start_missiles: 1,
        start_walls: true,
        deep_rack: true,
        ..Perks::default()
    };
    let run = Run::with_perks(5, perks);
    assert_eq!(run.colony.open, vec![0, 1, 2, 3, 4]);
    assert_eq!(run.colony.dome, THICK_DOME);
    assert_eq!(run.colony.dome_max, THICK_DOME);
    assert_eq!(run.colony.shield_max, BIG_SHIELD);
    assert_eq!(run.colony.missiles, 1);
    assert_eq!(run.colony.missiles_max, DEEP_RACK);
    assert!(run.grid.is_wall(s(1, 3)) && run.grid.is_wall(s(3, 3)));
    let mut colony = run.colony.clone();
    assert_eq!(colony.charge(100), BIG_SHIELD - SHIELD_START);
    assert_eq!(run.with_bonus(100), 100, "no bonus without a hard mode");
}

#[test]
fn hard_vacuum_caps_the_shield_and_pays_a_quarter_more() {
    let perks = Perks {
        hard_vacuum: true,
        big_shield: true,
        ..Perks::default()
    };
    let mut run = Run::with_perks(3, perks);
    assert_eq!(
        run.colony.shield_max, HARD_VACUUM_SHIELD,
        "hard vacuum beats the bigger shield"
    );
    assert_eq!(run.with_bonus(100), 125);
    let quiet = Perks {
        quiet: true,
        hard_vacuum: true,
        ..Perks::default()
    };
    assert_eq!(Run::with_perks(3, quiet).with_bonus(100), 150);
    // The bonus lands on the score as the run ends.
    let mut plain = Run::new(3);
    plain.relief = 1;
    plain.apply(Input::Ready).unwrap();
    hold(&mut plain);
    assert_eq!(plain.phase, Phase::Relief);
    run.relief = 1;
    run.apply(Input::Ready).unwrap();
    hold(&mut run);
    assert_eq!(run.phase, Phase::Relief);
    assert_eq!(run.score, plain.score + plain.score / 4);
}

// ----- the long watch -----

#[test]
fn the_long_watch_goes_on_past_relief() {
    let mut run = Run::long_watch(3, Perks::default());
    assert!(run.endless());
    assert_eq!(run.nights(), 0);
    // Night 1's two drifters, held as if it were night 12.
    run.night = RELIEF_NIGHT;
    let tech = run.tech;
    run.apply(Input::Ready).unwrap();
    run.take_events();
    hold(&mut run);
    assert_eq!(run.phase, Phase::Colony, "no Relief: the watch goes on");
    assert_eq!(run.night, RELIEF_NIGHT + 1);
    assert!(
        run.tech > tech + 1,
        "Relief's tech still lands: {}",
        run.tech
    );
    let events = run.take_events();
    assert!(!has(&events, |e| matches!(e, Event::Relief { .. })));
    assert!(has(&events, |e| matches!(
        e,
        Event::ColonyOpens { night: 13, .. }
    )));
    assert_eq!(run.forecast.len(), wave(13).len());
}

#[test]
fn the_nights_past_relief_come_round_harder() {
    assert_eq!(wave(13).len(), wave(9).len() + 1);
    assert_eq!(wave(14).len(), wave(10).len() + 1);
    assert_eq!(wave(16).len(), wave(12).len() + 1);
    assert_eq!(wave(17).len(), wave(9).len() + 2);
    assert_eq!(wave(21).len(), wave(9).len() + 3);
    assert!(wave(16).contains(&Kind::Carrier));
}

// ----- the dust storm -----

#[test]
fn the_storm_crosses_the_grid_and_turns_at_the_edges() {
    let mut storm = Storm::new();
    let mut seen = vec![storm.col];
    for _ in 0..(COLS as usize * 2 - 2) {
        storm.advance();
        seen.push(storm.col);
    }
    assert_eq!(seen, vec![0, 1, 2, 3, 4, 3, 2, 1, 0]);
    assert!(storm.covers(s(0, 3)) && !storm.covers(s(1, 3)));
}

#[test]
fn a_raider_in_the_dust_is_not_heard_and_loses_its_step() {
    // A seed whose night 1 drifters enter at the left and elsewhere: the
    // storm reaches the left on the first raiders' phase.
    let seed = (0..500u64)
        .find(|seed| {
            let run = Run::in_storm(*seed, Perks::default());
            run.forecast.iter().any(|a| a.col == 1) && run.forecast.iter().any(|a| a.col > 1)
        })
        .expect("a seed");
    let mut run = Run::in_storm(seed, Perks::default());
    assert_eq!(run.storm, Some(Storm::new()));
    run.apply(Input::Ready).unwrap();
    let events = run.take_events();
    assert!(has(&events, |e| matches!(e, Event::StormMoved { col: 0 })));
    assert_eq!(run.heard().len(), 2, "nothing in the far left yet");
    run.apply(Input::EndTurn).unwrap();
    let events = run.take_events();
    assert!(has(&events, |e| matches!(e, Event::StormMoved { col: 1 })));
    assert_eq!(run.storm.unwrap().col, 1);
    let in_dust = run.raiders.iter().find(|r| r.at.col == 1).unwrap();
    assert_eq!(in_dust.at.row, 6, "it lost its step");
    assert!(run.in_dust(in_dust.at));
    let heard = run.heard();
    assert!(heard.iter().all(|r| r.at.col != 1), "{heard:?}");
    assert!(run.raiders.iter().any(|r| r.at.col != 1 && r.at.row == 5));
}

#[test]
fn flak_in_the_dust_is_silent() {
    let seed = (0..500u64)
        .find(|seed| {
            Run::in_storm(*seed, Perks::default())
                .forecast
                .iter()
                .all(|a| a.col == 1)
        })
        .expect("a seed with both drifters at the left");
    let mut run = Run::in_storm(seed, Perks::default());
    run.apply(Input::Place {
        work: WorkKind::Flak,
        at: s(1, 5),
    })
    .unwrap();
    run.apply(Input::Ready).unwrap();
    run.take_events();
    // The storm is at the left for this phase: the post beside the
    // drifters cannot fire, and the drifters do not move.
    run.apply(Input::EndTurn).unwrap();
    let events = run.take_events();
    assert!(
        !has(&events, |e| matches!(
            e,
            Event::Hit { .. } | Event::Down { .. }
        )),
        "{events:?}"
    );
    assert_eq!(run.raiders.len(), 2);
}
