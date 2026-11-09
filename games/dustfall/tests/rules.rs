//! A night's rules, one at a time: the turn, the raiders' phase in its
//! order, breaches, charges, walls, mines, flak, the colony phase, and
//! saving.

use dustfall::rules::colony::{DOME_MAX, MISSILE_COST, SHIELD_START};
use dustfall::rules::grid::{Sector, Work, WorkKind};
use dustfall::rules::raiders::{Kind, State, Weapon};
use dustfall::rules::{Building, Event, Hitter, Input, Phase, Refusal, Run};

fn s(col: u8, row: u8) -> Sector {
    Sector::new(col, row)
}

/// A run at the start of `night`'s first turn, with `before` applied in
/// the colony phase first.
fn night(seed: u64, night: u8, before: &[Input]) -> Run {
    let mut run = Run::at_night(seed, night);
    for input in before {
        run.apply(*input)
            .unwrap_or_else(|why| panic!("{input:?}: {why:?}"));
    }
    run.apply(Input::Ready).unwrap();
    run.take_events();
    run
}

/// Ends turns without acting until something stops the night.
fn wait(run: &mut Run, turns: u32) {
    for _ in 0..turns {
        if !matches!(run.phase, Phase::Turn { .. }) {
            break;
        }
        run.apply(Input::EndTurn).unwrap();
    }
}

fn has(events: &[Event], f: impl Fn(&Event) -> bool) -> bool {
    events.iter().any(f)
}

#[test]
fn a_new_run_waits_in_the_colony_with_the_first_forecast() {
    let mut run = Run::new(7);
    assert_eq!(run.phase, Phase::Colony);
    assert_eq!(run.night, 1);
    assert_eq!(run.colony.resources.power, 12);
    assert_eq!(run.colony.resources.metal, 24);
    assert_eq!(run.colony.resources.food, 6);
    assert_eq!(run.colony.shield, SHIELD_START);
    assert_eq!(run.colony.dome, DOME_MAX);
    assert_eq!(run.forecast.len(), 2);
    assert_eq!(run.forecast_words(), "2 drifters");
    let events = run.take_events();
    assert!(matches!(events[0], Event::ColonyOpens { night: 1, .. }));
    // Nothing from the night works here.
    assert_eq!(run.apply(Input::Fire), Err(Refusal::WrongPhase));
    assert_eq!(run.apply(Input::EndTurn), Err(Refusal::WrongPhase));
}

#[test]
fn ready_puts_the_raiders_on_the_horizon() {
    let mut run = Run::new(7);
    run.take_events();
    run.apply(Input::Ready).unwrap();
    assert_eq!(run.phase, Phase::Turn { actions: 2 });
    assert_eq!(run.raiders.len(), 2);
    assert!(run
        .raiders
        .iter()
        .all(|r| r.at.row == 6 && r.kind == Kind::Drifter));
    let events = run.take_events();
    assert!(matches!(&events[0], Event::NightOpens { night: 1, arrivals } if arrivals.len() == 2));
    assert!(matches!(
        events[1],
        Event::YourTurn {
            turn: 1,
            actions: 2
        }
    ));
    assert_eq!(run.apply(Input::Ready), Err(Refusal::WrongPhase));
}

#[test]
fn the_gun_slides_one_column_and_stops_at_the_edges() {
    let mut run = night(7, 1, &[]);
    assert_eq!(run.gun, 2);
    run.apply(Input::Left).unwrap();
    assert_eq!(run.gun, 1);
    assert_eq!(run.phase, Phase::Turn { actions: 1 });
    // The second action ends the turn: the raiders step.
    run.apply(Input::Left).unwrap();
    assert_eq!(run.gun, 0);
    assert_eq!(run.phase, Phase::Turn { actions: 2 });
    assert!(run.raiders.iter().all(|r| r.at.row == 5));
    // At the edge the move is refused and costs nothing.
    assert_eq!(run.apply(Input::Left), Err(Refusal::Edge));
    assert_eq!(run.phase, Phase::Turn { actions: 2 });
    let events = run.take_events();
    assert!(has(&events, |e| matches!(e, Event::Moved { col: 1, .. })));
    assert!(has(&events, |e| matches!(e, Event::Stepped { .. })));
    assert!(has(&events, |e| matches!(
        e,
        Event::YourTurn {
            turn: 2,
            actions: 2
        }
    )));
}

#[test]
fn firing_up_an_empty_column_is_a_dry_shot_that_still_costs() {
    let mut run = night(7, 1, &[]);
    while run.raiders.iter().any(|r| r.at.col == run.gun) {
        run.apply(Input::Right)
            .unwrap_or_else(|_| run.apply(Input::Left).unwrap());
    }
    let before = run.phase;
    run.apply(Input::Fire).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(e, Event::DryShot { .. })),
        "{events:?}"
    );
    assert_ne!(run.phase, before);
}

#[test]
fn the_gun_brings_a_drifter_down_and_takes_its_power() {
    let mut run = night(7, 1, &[]);
    let target = run.nearest_first()[0].clone();
    let power = run.colony.resources.power;
    let metal = run.colony.resources.metal;
    while run.gun != target.at.col {
        let input = if run.gun < target.at.col {
            Input::Right
        } else {
            Input::Left
        };
        run.apply(input).unwrap();
    }
    run.apply(Input::Fire).unwrap();
    let events = run.take_events();
    assert!(
        has(
            &events,
            |e| matches!(e, Event::Down { id, kind: Kind::Drifter, by: Hitter::Gun, .. } if *id == target.id)
        ),
        "{events:?}"
    );
    assert!(run.raider(target.id).is_none());
    assert_eq!(run.colony.resources.power, power + 1);
    assert_eq!(run.colony.resources.metal, metal + 1);
    assert_eq!(run.score, 10);
    assert_eq!(run.downed, 1);
}

#[test]
fn an_unwatched_night_breaches_the_shield_then_the_dome() {
    let mut run = night(7, 1, &[]);
    // Five phases bring the drifters to row 1; the sixth breaches.
    wait(&mut run, 5);
    assert!(
        run.raiders.iter().all(|r| r.at.row == 1),
        "{:?}",
        run.raiders
    );
    assert_eq!(run.colony.shield, 50.min(SHIELD_START + 5 * 10));
    let shield = run.colony.shield;
    wait(&mut run, 1);
    let events = run.take_events();
    let breaches = events
        .iter()
        .filter(|e| matches!(e, Event::Breach { .. }))
        .count();
    assert_eq!(breaches, 2, "{events:?}");
    assert!(run.raiders.is_empty());
    // Two drifters take 20 off the shield and nothing off the dome.
    assert_eq!(run.colony.shield, shield - 20);
    assert_eq!(run.colony.dome, DOME_MAX);
    // The night is held with a breach: 50 points, not 100, and night 2
    // opens in the colony with the food eaten.
    assert!(has(&events, |e| matches!(
        e,
        Event::Held {
            night: 1,
            clean: false,
            ..
        }
    )));
    assert!(has(&events, |e| matches!(
        e,
        Event::ColonyOpens {
            night: 2,
            hungry: false,
            ..
        }
    )));
    assert_eq!(run.phase, Phase::Colony);
    assert_eq!(run.night, 2);
    assert_eq!(run.score, 50);
    assert_eq!(run.nights_held, 1);
    assert_eq!(run.tech, 1);
    assert_eq!(run.colony.resources.food, 4);
}

#[test]
fn ending_early_charges_the_shield_five_an_action() {
    let mut run = night(7, 1, &[]);
    run.apply(Input::EndTurn).unwrap();
    assert_eq!(run.colony.shield, SHIELD_START + 10);
    run.apply(Input::Left).unwrap();
    run.apply(Input::EndTurn).unwrap();
    assert_eq!(run.colony.shield, SHIELD_START + 15);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::TurnEnded { charged: 5 }
    )));
}

#[test]
fn a_breach_through_the_shield_reaches_the_dome_and_darkens_the_building() {
    // Night 3 brings a hulk, whose breach is bigger than the shield.
    let mut run = night(
        7,
        3,
        &[Input::Buy {
            building: Building::Solar,
            col: 2,
        }],
    );
    let mut guard = 0;
    while matches!(run.phase, Phase::Turn { .. }) && guard < 40 {
        // Keep the gun off every raider's column so nothing is hit, and
        // fire dry shots so no shield is charged.
        if run.raiders.iter().any(|r| r.at.col == run.gun) {
            let input = if run.gun == 0 {
                Input::Right
            } else {
                Input::Left
            };
            run.apply(input).unwrap();
        } else {
            run.apply(Input::Fire).unwrap();
        }
        guard += 1;
    }
    let events = run.take_events();
    assert!(
        run.colony.dome < DOME_MAX,
        "the dome took nothing: {events:?}"
    );
    assert!(has(&events, |e| matches!(e, Event::Breach { .. })));
    // The building goes dark only when a breach over its column reaches
    // the dome.
    let centre_hit = events.iter().any(|e| {
        matches!(
            e,
            Event::Breach {
                col: 2,
                darkened: Some(Building::Solar),
                ..
            }
        )
    });
    let dark = run.colony.slot(2).map(|s| s.dark).unwrap_or(false);
    assert_eq!(centre_hit, dark, "{events:?}");
}

#[test]
fn a_drifter_charges_from_row_three_and_moving_dodges_it() {
    let mut run = night(11, 2, &[]);
    wait(&mut run, 3);
    let charges = run.charges();
    assert!(
        !charges.is_empty(),
        "no drifter wound up: {:?}",
        run.raiders
    );
    let (_, kind, col) = charges[0];
    assert_eq!(kind, Kind::Drifter);
    assert_eq!(col, run.gun);
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::Charging {
            weapon: Weapon::Bolt,
            ..
        }
    )));
    // Move off the column: the bolt passes.
    let input = if run.gun == 0 {
        Input::Right
    } else {
        Input::Left
    };
    run.apply(input).unwrap();
    let shield = run.colony.shield;
    run.apply(Input::EndTurn).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Shot {
                dodged: true,
                weapon: Weapon::Bolt,
                ..
            }
        )),
        "{events:?}"
    );
    assert!(run.colony.shield >= shield, "the bolt landed");
}

#[test]
fn a_charge_hits_the_shield_when_the_gun_stays() {
    let mut run = night(11, 2, &[]);
    wait(&mut run, 3);
    assert!(!run.charges().is_empty());
    run.take_events();
    let shield = run.colony.shield;
    // Fire twice without moving: the bolt lands.
    run.apply(Input::Fire).unwrap();
    if matches!(run.phase, Phase::Turn { .. }) {
        run.apply(Input::Fire).unwrap();
    }
    let events = run.take_events();
    let shot = events
        .iter()
        .find(|e| matches!(e, Event::Shot { dodged: false, .. }))
        .unwrap_or_else(|| panic!("{events:?}"));
    if let Event::Shot { shield: after, .. } = shot {
        assert_eq!(*after, shield.saturating_sub(8));
    }
}

#[test]
fn only_two_charges_wind_up_at_once() {
    // Night 9 has three drifters and a hulk. Let them all come into range.
    let mut run = night(3, 9, &[]);
    wait(&mut run, 4);
    assert!(run.charges().len() <= 2, "{:?}", run.charges());
}

#[test]
fn drifters_hold_their_fire_on_the_first_night() {
    let mut run = night(11, 1, &[]);
    wait(&mut run, 5);
    assert!(run.charges().is_empty());
}

#[test]
fn a_hulk_rests_every_other_phase_and_its_siphon_stops_the_charge() {
    let mut run = night(5, 3, &[]);
    let hulk = run
        .raiders
        .iter()
        .find(|r| r.kind == Kind::Hulk)
        .unwrap()
        .id;
    let start = run.raider(hulk).unwrap().at.row;
    wait(&mut run, 1);
    assert_eq!(run.raider(hulk).unwrap().at.row, start - 1);
    wait(&mut run, 1);
    assert_eq!(
        run.raider(hulk).unwrap().at.row,
        start - 1,
        "it should rest"
    );
    wait(&mut run, 1);
    assert_eq!(run.raider(hulk).unwrap().at.row, start - 2);
    // Bring the gun under it and wait until it siphons.
    let col = run.raider(hulk).unwrap().at.col;
    let mut guard = 0;
    while run.raider(hulk).is_some() && !run.siphoned && guard < 20 {
        if run.gun != col {
            let input = if run.gun < col {
                Input::Right
            } else {
                Input::Left
            };
            run.apply(input).unwrap();
        } else {
            run.apply(Input::EndTurn).unwrap();
        }
        guard += 1;
        if !matches!(run.phase, Phase::Turn { .. }) {
            break;
        }
    }
    let events = run.take_events();
    if run.siphoned {
        let shield = run.colony.shield;
        run.apply(Input::EndTurn).unwrap();
        let after = run.take_events();
        assert!(
            has(&after, |e| matches!(e, Event::TurnEnded { charged: 0 })),
            "{after:?}"
        );
        assert!(run.colony.shield <= shield);
    } else {
        assert!(
            has(&events, |e| matches!(
                e,
                Event::Shot {
                    weapon: Weapon::Siphon,
                    ..
                }
            )) || run.raider(hulk).is_none()
                || !matches!(run.phase, Phase::Turn { .. }),
            "the hulk never fired: {events:?}"
        );
    }
}

#[test]
fn a_wall_takes_two_bashes_then_the_raider_walks_through() {
    let mut walls = Vec::new();
    for col in 0..5 {
        walls.push(Input::Place {
            work: WorkKind::Wall,
            at: s(col, 5),
        });
    }
    let mut run = night(7, 1, &walls);
    assert_eq!(run.colony.resources.metal, 24 - 15);
    wait(&mut run, 1);
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(e, Event::Bashed { down: false, .. })),
        "{events:?}"
    );
    assert!(run.raiders.iter().all(|r| r.at.row == 6));
    assert!(run
        .raiders
        .iter()
        .all(|r| matches!(r.state, State::Bashing { .. })));
    // The nearest raider's wall falls first; the other, seeing an open
    // way through the gap, leaves its own half-bashed wall for it.
    wait(&mut run, 1);
    let events = run.take_events();
    let fallen = events.iter().find_map(|e| match e {
        Event::Bashed {
            id,
            down: true,
            wall,
            ..
        } => Some((*id, *wall)),
        _ => None,
    });
    let (basher, wall) = fallen.unwrap_or_else(|| panic!("{events:?}"));
    assert!(run.grid.get(wall).is_none());
    // Then it walks through.
    wait(&mut run, 1);
    let events = run.take_events();
    assert!(
        has(
            &events,
            |e| matches!(e, Event::Stepped { id, to, .. } if *id == basher && *to == wall)
        ),
        "{events:?}"
    );
    assert!(run
        .raiders
        .iter()
        .all(|r| matches!(r.state, State::Approaching)));
}

#[test]
fn a_mine_goes_off_under_the_first_raider_to_step_on_it() {
    let run0 = Run::at_night(7, 1);
    let col = run0.forecast[0].col;
    let mut run = night(
        7,
        1,
        &[Input::Place {
            work: WorkKind::Mine,
            at: s(col, 5),
        }],
    );
    let metal = run.colony.resources.metal;
    assert_eq!(metal, 22);
    wait(&mut run, 1);
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Down {
                by: Hitter::Mine,
                kind: Kind::Drifter,
                ..
            }
        )),
        "{events:?}"
    );
    assert_eq!(run.grid.get(s(col, 5)), None, "the mine is spent");
}

#[test]
fn a_powered_flak_post_fires_at_the_nearest_raider_it_covers() {
    // A hulk walks under a post; a drifter would go round it.
    let run0 = Run::at_night(7, 3);
    let col = run0
        .forecast
        .iter()
        .find(|a| a.kind == Kind::Hulk)
        .map(|a| a.col)
        .unwrap();
    let mut run = night(
        7,
        3,
        &[Input::Place {
            work: WorkKind::Flak,
            at: s(col, 4),
        }],
    );
    // Phase 1 brings the hulk to row 5, covered by the post at row 4.
    wait(&mut run, 1);
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Hit {
                by: Hitter::Flak(from),
                kind: Kind::Hulk,
                left: 2,
                ..
            } if *from == s(col, 4)
        )),
        "{events:?}"
    );
}

#[test]
fn a_missile_takes_the_raider_nearest_the_domes() {
    let mut run = night(7, 1, &[Input::MakeMissile]);
    assert_eq!(run.colony.missiles, 1);
    assert_eq!(run.colony.resources.power, 12 - MISSILE_COST.0);
    assert_eq!(run.colony.resources.metal, 24 - MISSILE_COST.1);
    run.apply(Input::Missile).unwrap();
    let events = run.take_events();
    assert!(
        has(&events, |e| matches!(
            e,
            Event::Down {
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
    let mut run = Run::new(7);
    run.take_events();
    assert_eq!(
        run.apply(Input::Buy {
            building: Building::Solar,
            col: 0
        }),
        Err(Refusal::SlotClosed)
    );
    run.apply(Input::Buy {
        building: Building::Solar,
        col: 2,
    })
    .unwrap();
    assert_eq!(run.colony.resources.metal, 16);
    assert_eq!(
        run.apply(Input::Buy {
            building: Building::Hydroponics,
            col: 2
        }),
        Err(Refusal::SlotTaken)
    );
    run.apply(Input::Buy {
        building: Building::Hydroponics,
        col: 1,
    })
    .unwrap();
    assert_eq!(run.colony.resources.metal, 10);
    assert_eq!(run.colony.resources.power, 10);
    run.colony.resources.metal = 5;
    assert_eq!(
        run.apply(Input::Buy {
            building: Building::Workshop,
            col: 3
        }),
        Err(Refusal::CantAfford {
            power: 0,
            metal: 10
        })
    );
    run.colony.resources.metal = 10;
    assert_eq!(
        run.apply(Input::Place {
            work: WorkKind::Wall,
            at: s(2, 1)
        }),
        Err(Refusal::RowOpen)
    );
    assert_eq!(
        run.apply(Input::Place {
            work: WorkKind::Wall,
            at: s(2, 6)
        }),
        Err(Refusal::RowOpen)
    );
    run.apply(Input::Place {
        work: WorkKind::Wall,
        at: s(2, 3),
    })
    .unwrap();
    assert_eq!(
        run.apply(Input::Place {
            work: WorkKind::Mine,
            at: s(2, 3)
        }),
        Err(Refusal::SectorTaken)
    );
    assert_eq!(run.apply(Input::RepairDome), Err(Refusal::DomeWhole));
    assert_eq!(
        run.apply(Input::RepairBuilding { col: 4 }),
        Err(Refusal::NothingThere)
    );
    assert_eq!(
        run.apply(Input::RepairBuilding { col: 2 }),
        Err(Refusal::NotDark)
    );
    let events = run.take_events();
    assert!(has(&events, |e| matches!(
        e,
        Event::Bought {
            building: Building::Solar,
            col: 2
        }
    )));
    assert!(has(&events, |e| matches!(
        e,
        Event::Placed {
            work: WorkKind::Wall,
            ..
        }
    )));
}

#[test]
fn missiles_stop_at_three() {
    let mut run = Run::new(7);
    run.colony.resources.power = 100;
    run.colony.resources.metal = 100;
    for _ in 0..3 {
        run.apply(Input::MakeMissile).unwrap();
    }
    assert_eq!(run.apply(Input::MakeMissile), Err(Refusal::MissilesFull));
}

#[test]
fn after_a_night_the_colony_eats_produces_and_pays_for_power() {
    let before = [
        Input::Buy {
            building: Building::Solar,
            col: 1,
        },
        Input::Buy {
            building: Building::ShieldCoil,
            col: 2,
        },
        Input::Place {
            work: WorkKind::Flak,
            at: s(0, 3),
        },
    ];
    let mut run = night(7, 1, &before);
    // 24 metal less 8, 8 and 6; 12 power untouched.
    assert_eq!(run.colony.resources.metal, 2);
    assert_eq!(run.colony.resources.power, 12);
    run.colony.shield = 0;
    wait(&mut run, 12);
    let events = run.take_events();
    assert_eq!(run.phase, Phase::Colony, "{events:?}");
    // Eaten 2 food; solar made 3 power; the post drew 1 and the coil 2.
    assert_eq!(run.colony.resources.food, 4);
    let kills: u32 = run.take.power;
    assert_eq!(run.colony.resources.power, 12 + 3 - 1 - 2 + kills);
    assert!(
        run.colony.shield >= 10,
        "the coil put 10 on: {}",
        run.colony.shield
    );
    assert_eq!(run.grid.get(s(0, 3)), Some(Work::Flak { powered: true }));
}

#[test]
fn an_unpaid_post_goes_dark_for_the_night() {
    let mut run = Run::new(7);
    run.take_events();
    run.colony.resources.power = 0;
    run.apply(Input::Place {
        work: WorkKind::Flak,
        at: s(0, 3),
    })
    .unwrap();
    run.apply(Input::Place {
        work: WorkKind::Flak,
        at: s(4, 3),
    })
    .unwrap();
    run.apply(Input::Ready).unwrap();
    run.colony.resources.power = 0;
    // Keep every drifter alive: hold the gun away from them.
    let mut guard = 0;
    while matches!(run.phase, Phase::Turn { .. }) && guard < 40 {
        run.apply(Input::EndTurn).unwrap();
        guard += 1;
    }
    let events = run.take_events();
    assert_eq!(run.phase, Phase::Colony);
    // Whatever the kills brought, at most one post could be paid from it.
    let unpowered = events.iter().find_map(|e| match e {
        Event::ColonyOpens { unpowered, .. } => Some(unpowered.clone()),
        _ => None,
    });
    let unpowered = unpowered.expect("the colony opened");
    let paid = run.colony.resources.power;
    assert!(!unpowered.is_empty() || paid > 0, "{events:?}");
    for at in unpowered {
        assert_eq!(run.grid.get(at), Some(Work::Flak { powered: false }));
    }
}

#[test]
fn hunger_leaves_one_action_a_turn() {
    let mut run = Run::new(7);
    run.colony.resources.food = 0;
    assert_eq!(run.actions_a_turn(), 1);
    run.apply(Input::Ready).unwrap();
    assert_eq!(run.phase, Phase::Turn { actions: 1 });
    run.apply(Input::Left).unwrap();
    // One action ends the turn.
    assert!(run.raiders.iter().all(|r| r.at.row == 5));
}

#[test]
fn a_saved_run_resumes_on_the_same_night() {
    let mut run = Run::at_night(99, 4);
    run.apply(Input::Buy {
        building: Building::Hydroponics,
        col: 2,
    })
    .unwrap();
    run.apply(Input::Place {
        work: WorkKind::Wall,
        at: s(2, 3),
    })
    .unwrap();
    let json = serde_json::to_string(&run).unwrap();
    let mut back: Run = serde_json::from_str(&json).unwrap();
    assert_eq!(back.forecast, run.forecast);
    assert_eq!(back.colony, run.colony);
    assert_eq!(back.grid, run.grid);
    run.apply(Input::Ready).unwrap();
    back.apply(Input::Ready).unwrap();
    assert_eq!(back.raiders, run.raiders);
    // And the night plays the same from either copy.
    for _ in 0..6 {
        if !matches!(run.phase, Phase::Turn { .. }) {
            break;
        }
        run.apply(Input::EndTurn).unwrap();
        back.apply(Input::EndTurn).unwrap();
        assert_eq!(back.raiders, run.raiders);
        assert_eq!(back.colony, run.colony);
    }
}

#[test]
fn the_scan_lists_raiders_nearest_first_and_h_knows_their_route() {
    let mut run = night(7, 3, &[]);
    wait(&mut run, 2);
    let order = run.nearest_first();
    for pair in order.windows(2) {
        assert!((pair[0].at.row, pair[0].at.col) <= (pair[1].at.row, pair[1].at.col));
    }
    for raider in &order {
        let route = run.route_of(raider.id).expect("a route");
        assert_eq!(
            route.last().map(|at| at.row),
            Some(1),
            "{raider:?}: {route:?}"
        );
        let phases = run.phases_to_land(raider.id).unwrap();
        assert!(phases >= route.len() as u32 / 2, "{raider:?}");
    }
}
