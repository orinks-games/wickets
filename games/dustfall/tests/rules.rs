//! The rules of one night and one colony phase, through the rules alone:
//! the waves, the gun, charges, mines, flak, missiles and the shop.

use dustfall::rules::colony::{FLAK_EVERY, MINE_COST, SHIELD_START};
use dustfall::rules::lanes::{FLAK_REACH, HORIZON, MID, NEAR};
use dustfall::rules::nights::{FIRST_WAVE, WAVE_LEAD, WAVE_LULL};
use dustfall::rules::{
    Arrival, Building, Event, Hitter, Input, Kind, Phase, Quirk, Raider, Refusal, Run, Weapon,
    GUN_COOLDOWN, GUN_COOLDOWN_HUNGRY, MAX_CHARGES,
};

fn has(events: &[Event], f: impl Fn(&Event) -> bool) -> bool {
    events.iter().any(f)
}

/// Ticks until `f` holds of an event, returning the events so far.
fn tick_until(run: &mut Run, most: u32, f: impl Fn(&Event) -> bool) -> Vec<Event> {
    let mut all = Vec::new();
    for _ in 0..most {
        run.tick();
        let events = run.take_events();
        let found = has(&events, &f);
        all.extend(events);
        if found {
            return all;
        }
    }
    panic!("never happened in {most} ticks: {all:?}");
}

/// A night under way with nothing in the sky and nothing coming, so a
/// test can put its own raiders up. One arrival is left pending for a
/// tick that never comes, so the night does not end on its own.
fn empty_night(seed: u64, night: u8) -> Run {
    let mut run = Run::at_night(seed, night);
    run.apply(Input::Ready).unwrap();
    keep_open(&mut run);
    run.take_events();
    run
}

fn keep_open(run: &mut Run) {
    run.waves.clear();
    run.pending = vec![Arrival {
        kind: Kind::Drifter,
        lane: 0,
        quirk: None,
        wave: 1,
        at: u32::MAX,
    }];
}

/// A raider in the sky, rested and ready to fire.
fn put(run: &mut Run, kind: Kind, lane: u8, height: u32) -> u32 {
    let id = 900 + run.raiders.len() as u32;
    let mut raider = Raider::new(id, kind, lane, height);
    if let Some(weapon) = kind.weapon() {
        raider.since_shot = weapon.rest();
    }
    run.raiders.push(raider);
    id
}

/// Shoots until the night is held or the dome falls: the gun slides under
/// the nearest raider and fires whenever it can.
pub fn hold(run: &mut Run) {
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

#[test]
fn a_new_run_waits_in_the_colony_with_the_first_forecast() {
    let mut run = Run::new(1);
    assert_eq!(run.phase, Phase::Colony);
    assert_eq!(run.night, 1);
    assert_eq!(run.forecast.len(), 2);
    assert!(run.forecast.iter().all(|a| a.kind == Kind::Drifter));
    assert_eq!(run.waves_total(), 2, "night 1 comes in two waves of one");
    assert_eq!(run.forecast_words(), "2 drifters, in 2 waves");
    assert_eq!(run.wave_words(), vec!["a drifter", "a drifter"]);
    assert_eq!(run.colony.shield, SHIELD_START);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::ColonyOpens { night: 1, .. }
    )));
    assert_eq!(run.apply(Input::Fire), Err(Refusal::WrongPhase));
}

#[test]
fn ready_opens_the_night_and_the_first_wave_is_called() {
    let mut run = Run::new(1);
    run.take_events();
    run.apply(Input::Ready).unwrap();
    assert_eq!(run.phase, Phase::Night);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::NightOpens {
            night: 1,
            coming: 2,
            waves: 2
        }
    )));
    assert!(run.raiders.is_empty());
    assert_eq!(run.to_come(), 2);
    assert_eq!(run.next_wave_in(), Some(FIRST_WAVE));
    run.ticks(FIRST_WAVE - 1);
    assert!(run.take_events().is_empty(), "nothing before the call");
    run.tick();
    let events = run.take_events();
    assert!(
        has(
            &events,
            |e| matches!(e, Event::WaveComing { wave: 1, of: 2, kinds } if kinds == &[Kind::Drifter])
        ),
        "{events:?}"
    );
    assert_eq!(run.wave, 1);
    assert_eq!(run.pending.len(), 1);
    run.ticks(WAVE_LEAD);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::Arrived {
            kind: Kind::Drifter,
            ..
        }
    )));
    assert_eq!(run.raiders.len(), 1);
    assert_eq!(
        run.raiders[0].height,
        HORIZON - run.speed_of(&run.raiders[0])
    );
    assert_eq!(run.to_come(), 1, "the second wave is still to come");
}

#[test]
fn the_gun_slides_a_lane_and_stops_at_the_edges() {
    let mut run = empty_night(1, 1);
    assert_eq!(run.gun, 2);
    run.apply(Input::Left).unwrap();
    run.apply(Input::Left).unwrap();
    assert_eq!(run.gun, 0);
    assert_eq!(run.apply(Input::Left), Err(Refusal::Edge));
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::Moved {
            lane: 0,
            under: None
        }
    )));
    for _ in 0..4 {
        run.apply(Input::Right).unwrap();
    }
    assert_eq!(run.gun, 4);
    assert_eq!(run.apply(Input::Right), Err(Refusal::Edge));
}

#[test]
fn the_gun_reloads_between_shots_and_names_a_dry_one() {
    let mut run = empty_night(1, 1);
    run.apply(Input::Fire).unwrap();
    assert!(has(&run.take_events(), |e| matches!(
        e,
        Event::DryShot { lane: 2 }
    )));
    assert_eq!(run.cooldown, GUN_COOLDOWN);
    assert_eq!(run.apply(Input::Fire), Err(Refusal::Reloading));
    run.ticks(GUN_COOLDOWN);
    assert_eq!(run.apply(Input::Fire), Ok(()));
}

#[test]
fn the_gun_glances_off_a_far_mothership_and_bites_at_mid_range() {
    let mut run = empty_night(1, 5);
    let id = put(&mut run, Kind::Mothership, 2, MID + 50);
    run.apply(Input::Fire).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Glanced {
                kind: Kind::Mothership,
                lane: 2,
                ..
            }
        )),
        "{events:?}"
    );
    assert_eq!(run.raider(id).unwrap().hits, Kind::Mothership.hits());
    run.raiders[0].height = MID;
    run.ticks(GUN_COOLDOWN);
    run.apply(Input::Fire).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Hit {
                kind: Kind::Mothership,
                by: Hitter::Gun,
                ..
            }
        )),
        "{events:?}"
    );
}

#[test]
fn the_gun_brings_a_drifter_down_and_takes_its_power() {
    let mut run = empty_night(1, 1);
    put(&mut run, Kind::Drifter, 2, 700);
    let (power, metal) = (run.colony.resources.power, run.colony.resources.metal);
    run.apply(Input::Fire).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Down {
                kind: Kind::Drifter,
                lane: 2,
                by: Hitter::Gun,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(run.raiders.is_empty());
    assert_eq!(run.colony.resources.power, power + 1);
    assert_eq!(run.colony.resources.metal, metal + 2);
    assert_eq!(run.score, Kind::Drifter.score());
    assert_eq!(run.downed, 1);
}

#[test]
fn a_wave_down_brings_the_next_after_a_lull_and_the_night_is_held() {
    let mut run = Run::new(1);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    tick_until(&mut run, 100, |e| matches!(e, Event::Arrived { .. }));
    let lane = run.raiders[0].lane;
    while run.gun != lane {
        let input = if lane < run.gun {
            Input::Left
        } else {
            Input::Right
        };
        run.apply(input).unwrap();
    }
    run.apply(Input::Fire).unwrap();
    assert!(has(&run.take_events(), |e| matches!(e, Event::Down { .. })));
    let at = run.tick;
    tick_until(&mut run, 2, |e| {
        matches!(e, Event::WaveClear { wave: 1, of: 2 })
    });
    assert_eq!(run.next_wave_in(), Some(WAVE_LULL));
    tick_until(&mut run, WAVE_LULL + 1, |e| {
        matches!(e, Event::WaveComing { wave: 2, of: 2, .. })
    });
    assert_eq!(run.tick, at + 1 + WAVE_LULL, "the lull after the wave");
    hold(&mut run);
    assert_eq!(run.phase, Phase::Colony);
    assert_eq!(run.night, 2);
    assert_eq!(run.nights_held, 1);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::Held {
            night: 1,
            clean: true,
            ..
        }
    )));
    assert!(has(&events, |e| matches!(
        e,
        Event::ColonyOpens { night: 2, .. }
    )));
    assert_eq!(
        run.score,
        2 * Kind::Drifter.score() + 100,
        "two kills, a held night, no breach"
    );
}

#[test]
fn an_unwatched_night_breaches_the_shield_then_the_dome() {
    let mut run = Run::at_night(1, 3);
    run.apply(Input::Ready).unwrap();
    run.take_events();
    let mut breaches = Vec::new();
    for _ in 0..5000 {
        if run.phase != Phase::Night {
            break;
        }
        run.tick();
        breaches.extend(
            run.take_events()
                .into_iter()
                .filter(|e| matches!(e, Event::Breach { .. })),
        );
    }
    assert_eq!(breaches.len(), 3, "two drifters and a hulk: {breaches:?}");
    assert!(run.breached);
    assert_eq!(run.colony.shield, 0);
    assert!(run.colony.dome < 100, "the hulk's breach reaches the dome");
    assert_eq!(
        run.phase,
        Phase::Colony,
        "the night is still held once they are through"
    );
}

#[test]
fn a_drifter_charges_from_mid_range_and_leaving_the_lane_dodges() {
    let mut run = empty_night(1, 2);
    let id = put(&mut run, Kind::Drifter, 0, MID + 4);
    run.tick();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Ranged {
                range: dustfall::rules::Range::Mid,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Charging {
                kind: Kind::Drifter,
                weapon: Weapon::Bolt,
                lane: 2,
                ..
            }
        )),
        "it winds up at the gun's lane: {events:?}"
    );
    assert_eq!(run.charges(), vec![(id, Kind::Drifter, 2)]);
    run.apply(Input::Left).unwrap();
    let events = tick_until(&mut run, Weapon::Bolt.wind_up(), |e| {
        matches!(e, Event::Shot { .. })
    });
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Shot {
                dodged: true,
                lane: 2,
                ..
            }
        )),
        "{events:?}"
    );
    assert_eq!(run.colony.shield, SHIELD_START);
}

#[test]
fn a_charge_hits_the_shield_when_the_gun_stays() {
    let mut run = empty_night(1, 2);
    put(&mut run, Kind::Drifter, 0, MID + 4);
    let events = tick_until(&mut run, Weapon::Bolt.wind_up() + 2, |e| {
        matches!(e, Event::Shot { .. })
    });
    assert!(
        has(&events, |e| matches!(e, Event::Shot { dodged: false, .. })),
        "{events:?}"
    );
    assert_eq!(run.colony.shield, SHIELD_START - Weapon::Bolt.damage());
    assert_eq!(run.colony.dome, 100);
}

#[test]
fn only_two_charges_wind_up_at_once() {
    let mut run = empty_night(1, 2);
    for lane in 0..3 {
        put(&mut run, Kind::Drifter, lane, MID - 10);
    }
    run.tick();
    let charging = run
        .take_events()
        .iter()
        .filter(|e| matches!(e, Event::Charging { .. }))
        .count();
    assert_eq!(charging, MAX_CHARGES);
}

#[test]
fn drifters_hold_their_fire_on_the_first_night() {
    let mut run = empty_night(1, 1);
    put(&mut run, Kind::Drifter, 2, MID - 10);
    run.ticks(30);
    assert!(!has(&run.take_events(), |e| matches!(
        e,
        Event::Charging { .. }
    )));
}

#[test]
fn a_hulk_siphons_from_near_range_and_stops_the_recharge() {
    let mut run = empty_night(1, 3);
    run.colony.shield = 10;
    put(&mut run, Kind::Hulk, 2, NEAR + 2);
    let events = tick_until(&mut run, 3, |e| {
        matches!(
            e,
            Event::Charging {
                weapon: Weapon::Siphon,
                ..
            }
        )
    });
    assert!(has(&events, |e| matches!(
        e,
        Event::Ranged {
            range: dustfall::rules::Range::Near,
            ..
        }
    )));
    tick_until(&mut run, Weapon::Siphon.wind_up() + 1, |e| {
        matches!(e, Event::Shot { dodged: false, .. })
    });
    assert_eq!(run.colony.shield, 10 - Weapon::Siphon.damage());
    let after = run.colony.shield;
    run.ticks(40);
    assert_eq!(run.colony.shield, after, "no recharge while siphoned");
    assert!(!run.raiders.is_empty(), "the hulk is still falling");
}

#[test]
fn a_mine_blows_under_the_first_raider_past_mid_range() {
    let mut run = Run::new(2);
    let metal = run.colony.resources.metal;
    run.apply(Input::LayMine { lane: 2 }).unwrap();
    assert_eq!(run.colony.resources.metal, metal - MINE_COST);
    assert!(run.colony.mined(2));
    assert_eq!(
        run.apply(Input::LayMine { lane: 2 }),
        Err(Refusal::LaneMined)
    );
    assert_eq!(
        run.apply(Input::LayMine { lane: 3 }),
        Err(Refusal::MinesLaid)
    );
    assert_eq!(run.colony.mined_lanes(), vec![2]);
    run.apply(Input::Ready).unwrap();
    keep_open(&mut run);
    run.take_events();
    put(&mut run, Kind::Drifter, 2, MID + 2);
    put(&mut run, Kind::Mothership, 3, MID + 2);
    run.tick();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(e, Event::MineBlew { lane: 2 })),
        "{events:?}"
    );
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Down {
                kind: Kind::Drifter,
                by: Hitter::Mine,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(!run.colony.mined(2), "the mine is spent");
    assert!(
        run.raiders.iter().any(|r| r.kind == Kind::Mothership),
        "armour shrugs off mines"
    );
}

#[test]
fn a_powered_flak_post_fires_up_its_own_lane() {
    let mut run = Run::new(2);
    run.apply(Input::Buy {
        building: Building::Flak,
        lane: 2,
    })
    .unwrap();
    run.apply(Input::Ready).unwrap();
    keep_open(&mut run);
    run.take_events();
    let id = put(&mut run, Kind::Hulk, 2, FLAK_REACH - 10);
    put(&mut run, Kind::Hulk, 1, FLAK_REACH - 10);
    let events = tick_until(&mut run, FLAK_EVERY + 1, |e| {
        matches!(
            e,
            Event::Hit {
                by: Hitter::Flak(2),
                ..
            }
        )
    });
    assert!(
        has(
            &events,
            |e| matches!(e, Event::Hit { id: hit, left: 2, .. } if *hit == id)
        ),
        "{events:?}"
    );
    assert_eq!(run.raider(id).unwrap().hits, 2);
    assert!(
        run.raiders.iter().any(|r| r.lane == 1 && r.hits == 3),
        "the post does not reach the next lane"
    );
}

#[test]
fn a_missile_takes_the_raider_nearest_the_dome() {
    let mut run = Run::new(2);
    run.apply(Input::MakeMissile).unwrap();
    run.apply(Input::Ready).unwrap();
    keep_open(&mut run);
    run.take_events();
    put(&mut run, Kind::Drifter, 0, 900);
    put(&mut run, Kind::Hulk, 4, 300);
    run.apply(Input::Missile).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Down {
                kind: Kind::Hulk,
                by: Hitter::Missile,
                ..
            }
        )),
        "{events:?}"
    );
    assert_eq!(run.colony.missiles, 0);
    assert_eq!(run.apply(Input::Missile), Err(Refusal::NoMissiles));
}

#[test]
fn the_colony_phase_sells_what_it_says_and_refuses_the_rest() {
    let mut run = Run::new(3);
    assert_eq!(
        run.apply(Input::Buy {
            building: Building::Solar,
            lane: 0
        }),
        Err(Refusal::SlotClosed)
    );
    run.apply(Input::Buy {
        building: Building::Solar,
        lane: 1,
    })
    .unwrap();
    assert_eq!(run.colony.resources.metal, 16);
    assert_eq!(
        run.apply(Input::Buy {
            building: Building::Workshop,
            lane: 1
        }),
        Err(Refusal::SlotTaken)
    );
    assert_eq!(run.apply(Input::RepairDome), Err(Refusal::DomeWhole));
    run.apply(Input::Buy {
        building: Building::Scanner,
        lane: 2,
    })
    .unwrap();
    assert_eq!(
        (run.colony.resources.metal, run.colony.resources.power),
        (8, 10)
    );
    run.apply(Input::Buy {
        building: Building::Hydroponics,
        lane: 3,
    })
    .unwrap();
    assert_eq!(
        (run.colony.resources.metal, run.colony.resources.power),
        (2, 8)
    );
    assert_eq!(
        run.apply(Input::MakeMissile),
        Err(Refusal::CantAfford { power: 6, metal: 4 })
    );
    assert_eq!(
        run.apply(Input::RepairBuilding { lane: 1 }),
        Err(Refusal::NotDark)
    );
    assert_eq!(
        run.apply(Input::RepairBuilding { lane: 4 }),
        Err(Refusal::NothingThere)
    );
    assert_eq!(run.apply(Input::Left), Err(Refusal::WrongPhase));
    let events = run.take_events();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Bought { .. }))
            .count(),
        3
    );
    assert!(has(&events, |e| matches!(
        e,
        Event::Bought {
            building: Building::Scanner,
            lane: 2
        }
    )));
}

#[test]
fn missiles_stop_at_the_racks_size() {
    let mut run = Run::new(3);
    run.colony.resources.power = 100;
    run.colony.resources.metal = 100;
    for _ in 0..3 {
        run.apply(Input::MakeMissile).unwrap();
    }
    assert_eq!(run.apply(Input::MakeMissile), Err(Refusal::MissilesFull));
    assert_eq!(run.colony.missile_cost(), (6, 4));
    run.apply(Input::Buy {
        building: Building::Workshop,
        lane: 1,
    })
    .unwrap();
    assert_eq!(run.colony.missile_cost(), (3, 2));
}

#[test]
fn after_a_night_the_colony_eats_produces_and_pays_for_power() {
    let mut run = Run::new(1);
    run.apply(Input::Buy {
        building: Building::Solar,
        lane: 1,
    })
    .unwrap();
    run.apply(Input::Buy {
        building: Building::Scanner,
        lane: 2,
    })
    .unwrap();
    run.apply(Input::Ready).unwrap();
    run.take_events();
    hold(&mut run);
    assert_eq!(run.phase, Phase::Colony);
    let r = run.colony.resources;
    assert_eq!(r.food, 4, "the colony ate");
    assert_eq!(
        r.power,
        12 - 2 + 3 - 1 + 2,
        "solar made 3, the scanner drew 1, two drifters gave 2"
    );
    assert_eq!(r.metal, 24 - 8 - 8 + 4);
    assert!(run.colony.slot(2).unwrap().powered);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::ColonyOpens { night: 2, hungry: false, unpowered, .. } if unpowered.is_empty()
    )));
}

#[test]
fn an_unpaid_building_does_nothing_for_the_night() {
    let mut run = Run::new(1);
    run.apply(Input::Buy {
        building: Building::Scanner,
        lane: 1,
    })
    .unwrap();
    run.apply(Input::Buy {
        building: Building::ShieldCoil,
        lane: 2,
    })
    .unwrap();
    // Two drifters bring 2 power: the scanner's 1 is paid, the coil's 2 is not.
    run.colony.resources.power = 0;
    run.apply(Input::Ready).unwrap();
    run.take_events();
    hold(&mut run);
    let events = run.take_events();
    assert!(
        has(
            &events,
            |e| matches!(e, Event::ColonyOpens { unpowered, .. } if unpowered == &[2])
        ),
        "{events:?}"
    );
    assert!(run.colony.slot(1).unwrap().powered);
    assert!(!run.colony.slot(2).unwrap().powered);
    assert!(run.colony.has(Building::Scanner));
    assert!(!run.colony.has(Building::ShieldCoil));
    assert_eq!(run.colony.resources.power, 1);
}

#[test]
fn hunger_slows_the_gun() {
    let mut run = Run::new(1);
    run.colony.resources.food = 0;
    assert!(run.colony.hungry());
    run.apply(Input::Ready).unwrap();
    run.apply(Input::Fire).unwrap();
    assert_eq!(run.cooldown, GUN_COOLDOWN_HUNGRY);
}

#[test]
fn a_saved_run_resumes_on_the_same_night() {
    let mut run = Run::new(5);
    run.apply(Input::Ready).unwrap();
    hold(&mut run);
    assert_eq!(run.night, 2);
    let saved = serde_json::to_string(&run).unwrap();
    let mut back: Run = serde_json::from_str(&saved).unwrap();
    assert_eq!(back.forecast, run.forecast);
    assert_eq!(back.colony, run.colony);
    back.apply(Input::Ready).unwrap();
    run.apply(Input::Ready).unwrap();
    back.take_events();
    run.take_events();
    for _ in 0..200 {
        back.tick();
        run.tick();
    }
    assert_eq!(back.raiders, run.raiders, "the same night from a save");
}

#[test]
fn the_scan_lists_raiders_nearest_first_and_quirks_are_named() {
    let mut run = empty_night(1, 9);
    put(&mut run, Kind::Drifter, 0, 900);
    put(&mut run, Kind::Hulk, 4, 300);
    let far = put(&mut run, Kind::Skitter, 2, 950);
    let order: Vec<u32> = run.nearest_first().iter().map(|r| r.id).collect();
    assert_eq!(order.last(), Some(&far));
    assert_eq!(run.nearest_first()[0].kind, Kind::Hulk);
    let armoured = Raider::new(1, Kind::Drifter, 1, 500).with_quirk(Some(Quirk::Armoured));
    assert_eq!(armoured.hits, 2);
    assert_eq!(armoured.title(), "Armoured drifter");
    assert_eq!(armoured.words(), "Armoured drifter, left, mid range");
    let hasty = Raider::new(2, Kind::Drifter, 1, 500).with_quirk(Some(Quirk::Hasty));
    assert_eq!(run.speed_of(&hasty), 6, "a quarter faster than 5");
    assert_eq!(run.speed_of(&armoured), 5);
}
