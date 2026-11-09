//! Orders, quirks, perks, the long watch, the dust storm and the waves
//! table, each through the rules alone.

use dustfall::rules::colony::{DOME_MAX, MISSILES_MAX, SHIELD_MAX, SHIELD_START};
use dustfall::rules::lanes::MID;
use dustfall::rules::nights::{kinds, quirks_at_most, waves, QUIRKS_FROM, WAVE_LULL};
use dustfall::rules::orders::{OVERCLOCK_POWER, OVERCLOCK_SHIELD, RATIONING_FOOD};
use dustfall::rules::perks::{
    BIG_SHIELD, DEEP_RACK, HARD_VACUUM_SHIELD, MINES_A_NIGHT_MORE, THICK_DOME,
};
use dustfall::rules::storm::STORM_EVERY;
use dustfall::rules::{
    Arrival, Building, Colony, Event, Input, Kind, Order, Perks, Phase, Quirk, Raider, Refusal,
    Run, Storm, LANES, RELIEF_NIGHT,
};

fn has(events: &[Event], f: impl Fn(&Event) -> bool) -> bool {
    events.iter().any(f)
}

/// Shoots until the night is held or the dome falls.
fn hold(run: &mut Run) {
    for _ in 0..20_000 {
        if run.phase != Phase::Night {
            return;
        }
        let target = run.nearest_first().first().map(|r| r.lane);
        match target {
            Some(lane) if lane < run.gun => run.apply(Input::Left).unwrap(),
            Some(lane) if lane > run.gun => run.apply(Input::Right).unwrap(),
            Some(_) if run.cooldown == 0 => run.apply(Input::Fire).unwrap(),
            _ => {}
        }
        run.tick();
    }
    panic!("the night never ended: {:?}", run.phase);
}

/// Ticks between a wave going down and the next being called.
fn lull_of(run: &mut Run) -> u32 {
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let mut clear_at = None;
    for _ in 0..20_000 {
        if run.phase != Phase::Night {
            break;
        }
        let target = run.nearest_first().first().map(|r| r.lane);
        match target {
            Some(lane) if lane < run.gun => run.apply(Input::Left).unwrap(),
            Some(lane) if lane > run.gun => run.apply(Input::Right).unwrap(),
            Some(_) if run.cooldown == 0 => run.apply(Input::Fire).unwrap(),
            _ => {}
        }
        run.tick();
        for e in run.take_events() {
            match e {
                Event::WaveClear { wave: 1, .. } => clear_at = Some(run.tick),
                Event::WaveComing { wave: 2, .. } => {
                    return run.tick - clear_at.expect("cleared first")
                }
                _ => {}
            }
        }
    }
    panic!("no second wave");
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
    assert_eq!(
        run.colony.resources.power,
        power + 2 - OVERCLOCK_POWER,
        "two drifters' power, then the bill"
    );
    assert!(has(&run.take_events(), |e| matches!(
        e,
        Event::OrderPaid {
            order: Order::Overclock
        }
    )));
    assert_eq!(run.order, None, "an order lasts one night");
}

#[test]
fn rationing_feeds_the_colony_and_packs_the_waves() {
    let plain = lull_of(&mut Run::new(3));
    assert_eq!(plain, WAVE_LULL);
    let mut run = with_order(3, Order::Rationing);
    let food = run.colony.resources.food;
    run.apply(Input::TakeOrder(Order::Rationing)).unwrap();
    assert_eq!(run.colony.resources.food, food + RATIONING_FOOD);
    assert_eq!(lull_of(&mut run), WAVE_LULL * 2 / 3);
}

#[test]
fn salvage_doubles_the_metal_and_speeds_the_skitters() {
    let mut run = with_order(3, Order::Salvage);
    run.apply(Input::TakeOrder(Order::Salvage)).unwrap();
    let skitter = Raider::new(1, Kind::Skitter, 0, 900);
    assert_eq!(run.speed_of(&skitter), Kind::Skitter.speed() * 3 / 2);
    run.apply(Input::Ready).unwrap();
    run.waves.clear();
    run.take_events();
    run.raiders.push(Raider::new(2, Kind::Drifter, 2, 800));
    let metal = run.colony.resources.metal;
    run.apply(Input::Fire).unwrap();
    assert_eq!(
        run.colony.resources.metal,
        metal + 2 * Kind::Drifter.drop().metal
    );
}

// ----- quirks -----

#[test]
fn quirks_come_from_night_seven_and_fit_their_kinds() {
    for night in 1..QUIRKS_FROM {
        assert_eq!(quirks_at_most(night), 0);
        let run = Run::at_night(5, night);
        assert!(
            run.forecast.iter().all(|a| a.quirk.is_none()),
            "night {night}"
        );
    }
    assert_eq!(quirks_at_most(QUIRKS_FROM), 1);
    assert_eq!(quirks_at_most(9), 2);
    let mut any = false;
    for seed in 1..20 {
        let run = Run::at_night(seed, 9);
        let quirked: Vec<_> = run.forecast.iter().filter(|a| a.quirk.is_some()).collect();
        assert!(quirked.len() <= 2, "seed {seed}: {quirked:?}");
        for a in quirked {
            assert!(a.quirk.unwrap().fits(a.kind), "{a:?}");
            any = true;
        }
    }
    assert!(any, "some night 9 carries a quirk");
    assert!(!Quirk::Weaving.fits(Kind::Hulk));
    assert!(!Quirk::Armoured.fits(Kind::Mothership));
    assert_eq!(Run::at_night(5, 9).forecast, Run::at_night(5, 9).forecast);
}

#[test]
fn a_weaving_raider_slips_a_lane_away_from_the_gun_at_mid_range() {
    let mut run = Run::at_night(1, 9);
    run.apply(Input::Ready).unwrap();
    run.waves.clear();
    run.take_events();
    run.apply(Input::Left).unwrap();
    let weaver = Raider::new(7, Kind::Skitter, 2, MID + 5).with_quirk(Some(Quirk::Weaving));
    assert!(weaver.can_weave());
    run.raiders.push(weaver);
    run.tick();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Weaved { from: 2, to: 3, .. }
        )),
        "away from the gun at the left: {events:?}"
    );
    let after = run.raider(7).unwrap();
    assert_eq!(after.lane, 3);
    assert!(!after.can_weave(), "once a night");
}

// ----- perks -----

#[test]
fn perks_set_the_colony_up() {
    let plain = Colony::new();
    assert_eq!(plain.open, vec![1, 2, 3]);
    assert_eq!(
        (plain.dome_max, plain.shield_max, plain.missiles_max),
        (DOME_MAX, SHIELD_MAX, MISSILES_MAX)
    );
    assert_eq!(plain.mines_a_night, 1);
    let perks = Perks {
        extra_slots: 2,
        big_shield: true,
        thick_dome: true,
        start_missiles: 1,
        more_mines: true,
        deep_rack: true,
        lane_forecast: true,
        forecast_quirks: true,
        hard_vacuum: false,
        quiet: false,
    };
    let full = Colony::with(perks);
    assert_eq!(full.open, vec![0, 1, 2, 3, 4]);
    assert_eq!(full.dome, THICK_DOME);
    assert_eq!(full.shield_max, BIG_SHIELD);
    assert_eq!(full.missiles, 1);
    assert_eq!(full.missiles_max, DEEP_RACK);
    assert_eq!(full.mines_a_night, MINES_A_NIGHT_MORE);
    let mut run = Run::with_perks(1, perks);
    run.apply(Input::LayMine { lane: 0 }).unwrap();
    run.apply(Input::LayMine { lane: 4 }).unwrap();
    assert_eq!(
        run.apply(Input::LayMine { lane: 2 }),
        Err(Refusal::MinesLaid)
    );
    let hard = Perks {
        hard_vacuum: true,
        quiet: true,
        ..Perks::default()
    };
    assert_eq!(Colony::with(hard).shield_max, HARD_VACUUM_SHIELD);
    assert_eq!(hard.bonus_percent(), 50);
    assert_eq!(Run::with_perks(1, hard).with_bonus(200), 300);
}

// ----- the long watch -----

#[test]
fn the_long_watch_rounds_the_late_nights_with_more_drifters() {
    let run = Run::long_watch(1, Perks::default());
    assert!(run.endless());
    assert_eq!(run.nights(), 0);
    assert_eq!(kinds(13).len(), kinds(9).len() + 1);
    assert_eq!(kinds(16).len(), kinds(12).len() + 1);
    assert_eq!(kinds(17).len(), kinds(9).len() + 2);
    assert_eq!(
        waves(13).len(),
        waves(9).len(),
        "the same waves, one fuller"
    );
    let mut run = Run::prepared(1, 1, 1);
    run.relief = 0;
    run.apply(Input::Ready).unwrap();
    hold(&mut run);
    assert_eq!(run.phase, Phase::Colony, "the long watch goes on");
    assert_eq!(run.night, 2);
}

// ----- the storm -----

#[test]
fn the_storm_drifts_a_lane_at_a_time_and_turns_at_the_edges() {
    let mut storm = Storm::new();
    assert_eq!(storm.lane, 0);
    let mut path = Vec::new();
    for _ in 0..9 {
        storm.advance();
        path.push(storm.lane);
    }
    assert_eq!(path, vec![1, 2, 3, 4, 3, 2, 1, 0, 1]);
    assert!(storm.covers(1));
}

#[test]
fn raiders_in_the_dust_are_not_heard_and_the_flak_under_it_is_silent() {
    let mut run = Run::in_storm(2, Perks::default());
    run.apply(Input::Buy {
        building: Building::Flak,
        lane: 1,
    })
    .unwrap();
    run.apply(Input::Ready).unwrap();
    run.waves.clear();
    run.pending = vec![Arrival {
        kind: Kind::Drifter,
        lane: 0,
        quirk: None,
        wave: 1,
        at: u32::MAX,
    }];
    let events = run.take_events();
    assert!(has(&events, |e| matches!(e, Event::StormMoved { lane: 0 })));
    assert!(run.in_dust(0));
    run.ticks(STORM_EVERY);
    assert!(has(&run.take_events(), |e| matches!(
        e,
        Event::StormMoved { lane: 1 }
    )));
    assert!(run.in_dust(1) && !run.in_dust(0));
    run.raiders.push(Raider::new(1, Kind::Hulk, 1, 800));
    run.raiders.push(Raider::new(2, Kind::Hulk, 3, 800));
    let heard: Vec<u32> = run.heard().iter().map(|r| r.id).collect();
    assert_eq!(heard, vec![2], "the hulk in the dust is not heard");
    run.ticks(STORM_EVERY - 1);
    let events = run.take_events();
    assert!(
        !has(&events, |e| matches!(
            e,
            Event::Hit {
                by: dustfall::rules::Hitter::Flak(_),
                ..
            }
        )),
        "{events:?}"
    );
    run.ticks(30);
    assert!(
        has(&run.take_events(), |e| matches!(
            e,
            Event::Hit {
                by: dustfall::rules::Hitter::Flak(1),
                ..
            }
        )),
        "the post fires once the dust has moved on"
    );
}

// ----- the waves table -----

#[test]
fn every_night_has_its_waves() {
    for night in 1..=RELIEF_NIGHT {
        let w = waves(night);
        assert!(w.len() >= 2, "night {night} has more than one wave");
        assert!(w.iter().all(|wave| !wave.is_empty()), "night {night}");
        let flat: Vec<Kind> = w.iter().flatten().copied().collect();
        assert_eq!(flat, kinds(night));
        let run = Run::at_night(1, night);
        assert_eq!(run.waves_total() as usize, w.len());
        assert!(run.forecast.iter().all(|a| a.lane < LANES));
    }
    assert!(kinds(5).contains(&Kind::Mothership) && kinds(10).contains(&Kind::Mothership));
    assert!(kinds(RELIEF_NIGHT).contains(&Kind::Carrier));
    assert!(Run::at_night(1, 4)
        .forecast_words()
        .ends_with(", in 3 waves"));
}
