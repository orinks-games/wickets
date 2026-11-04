//! The outing's rules, driven directly.

use crate::common;

use common::listen_and_gather;
use parlor_core::{Challenge, Date, Period, Rng};
use tide_pools::rules::{
    generate, list_words, minutes_words, Conditions, Ev, Lean, Mode, Modifier, Outing, Place, Plan,
    Species, Tide, TimeOfDay, Weather, GATHER_WINDOW,
};
use tide_pools::Aquarium;

fn practice() -> Outing {
    Outing::new(Plan::practice(), Rng::seeded(3))
}

fn beach(place: Place, seed: u64) -> Outing {
    let mut rng = Rng::seeded(seed);
    let plan = Plan::beach(place, &mut rng);
    Outing::new(plan, rng)
}

#[test]
fn the_practice_pools_are_always_the_same() {
    let o = practice();
    assert_eq!(o.pools.len(), 3);
    let species: Vec<Vec<Species>> = o
        .pools
        .iter()
        .map(|p| p.finds.iter().map(|f| f.species).collect())
        .collect();
    assert_eq!(
        species,
        vec![
            vec![Species::Anemone],
            vec![Species::Crab],
            vec![Species::Periwinkle, Species::Starfish]
        ]
    );
    assert_eq!(o.at, 0);
    assert_eq!(o.lean, Lean::Edge);
    assert_eq!(o.total(), 4);
}

#[test]
fn walking_stops_at_the_ends_of_the_rocks() {
    let mut o = practice();
    assert_eq!(o.walk(-1), vec![Ev::Edge { left: true }]);
    assert_eq!(
        o.walk(1),
        vec![Ev::Arrive {
            pool: 1,
            of: 3,
            cleared: false
        }]
    );
    o.walk(1);
    assert_eq!(o.walk(1), vec![Ev::Edge { left: false }]);
    assert_eq!(o.at, 2);
}

#[test]
fn leaning_has_three_places_and_walking_brings_you_to_the_edge() {
    let mut o = practice();
    assert_eq!(
        o.step_back(),
        vec![Ev::Lean {
            lean: Lean::Back,
            changed: true
        }]
    );
    assert_eq!(
        o.step_back(),
        vec![Ev::Lean {
            lean: Lean::Back,
            changed: false
        }]
    );
    o.lean_in();
    assert_eq!(o.lean, Lean::Edge);
    o.lean_in();
    assert_eq!(o.lean, Lean::In);
    assert_eq!(
        o.lean_in(),
        vec![Ev::Lean {
            lean: Lean::In,
            changed: false
        }]
    );
    o.walk(1);
    assert_eq!(o.lean, Lean::Edge, "walking stands you up");
}

#[test]
fn hearing_range_narrows_as_you_lean_in() {
    let mut o = beach(Place::Reef, 5);
    o.walk(1);
    o.walk(1);
    o.walk(1);
    o.step_back();
    let mut offsets = std::collections::HashSet::new();
    for _ in 0..800 {
        for e in o.update(0.05) {
            if let Ev::Call { offset, lean, .. } = e {
                assert_eq!(lean, Lean::Back);
                assert!(offset.abs() <= 2, "{offset}");
                offsets.insert(offset);
            }
        }
    }
    assert!(
        offsets.iter().any(|o| *o != 0),
        "neighbours heard from back"
    );
    o.lean_in();
    o.lean_in();
    for _ in 0..800 {
        for e in o.update(0.05) {
            if let Ev::Call { offset, .. } = e {
                assert_eq!(offset, 0, "only this pool when leaning in");
            }
        }
    }
}

#[test]
fn hushed_shores_carry_only_the_pool_in_front() {
    let mut rng = Rng::seeded(4);
    let mut plan = Plan::beach(Place::Point, &mut rng);
    plan.modifier = Modifier::Hushed;
    let mut o = Outing::new(plan, rng);
    o.step_back();
    assert_eq!(o.range(), 0);
    for _ in 0..800 {
        for e in o.update(0.05) {
            if let Ev::Call { offset, .. } = e {
                assert_eq!(offset, 0);
            }
        }
    }
}

#[test]
fn you_must_lean_in_to_reach_the_water() {
    let mut o = practice();
    assert_eq!(o.gather(), vec![Ev::OutOfReach]);
    o.step_back();
    assert_eq!(o.gather(), vec![Ev::OutOfReach]);
}

#[test]
fn reaching_in_before_hearing_anything_finds_only_water() {
    let mut o = practice();
    o.lean_in();
    assert_eq!(o.gather(), vec![Ev::NotYet]);
    assert!(o.gathered.is_empty());
}

#[test]
fn gathering_what_you_hear() {
    let mut o = practice();
    o.lean_in();
    let evs = listen_and_gather(&mut o, Species::Anemone);
    assert_eq!(
        evs,
        vec![Ev::Gathered {
            species: Species::Anemone,
            points: 5,
            target: false
        }]
    );
    assert_eq!(o.points, 5);
    assert_eq!(o.gathered, vec![Species::Anemone]);
    assert_eq!(o.gather(), vec![Ev::EmptyPool]);
    o.step_back();
    o.lean_in();
    assert_eq!(
        o.lean_in(),
        vec![Ev::Lean {
            lean: Lean::In,
            changed: false
        }]
    );
}

#[test]
fn leaning_in_over_an_emptied_pool_finds_only_water() {
    let mut o = practice();
    o.lean_in();
    listen_and_gather(&mut o, Species::Anemone);
    o.step_back();
    let evs = o.lean_in();
    assert!(evs.contains(&Ev::OnlyWater), "{evs:?}");
    o.walk(1);
    let evs = o.walk(-1);
    assert_eq!(
        evs,
        vec![Ev::Arrive {
            pool: 0,
            of: 3,
            cleared: true
        }]
    );
}

#[test]
fn a_sound_can_be_reached_for_only_a_little_while() {
    let mut o = practice();
    o.lean_in();
    // Wait for the anemone, then let the moment pass before reaching.
    loop {
        let evs = o.update(0.05);
        if evs.iter().any(|e| matches!(e, Ev::Call { offset: 0, .. })) {
            break;
        }
    }
    // Its next call is at least 1.6 seconds off; stay inside that.
    o.update(1.5);
    const { assert!(1.5 < GATHER_WINDOW) };
    assert!(matches!(o.gather()[0], Ev::Gathered { .. }));
}

#[test]
fn shy_creatures_hide_until_you_keep_still() {
    let mut o = practice();
    o.walk(1);
    let evs = o.lean_in();
    assert!(evs.contains(&Ev::Hide { count: 1 }), "{evs:?}");
    assert!(o.hiding_here());
    // Reaching in too soon tucks it away again.
    assert_eq!(o.gather(), vec![Ev::TooSoon]);
    let mut emerged = false;
    for _ in 0..200 {
        if o.update(0.05).iter().any(|e| {
            *e == Ev::Emerge {
                species: Species::Crab,
            }
        }) {
            emerged = true;
            break;
        }
    }
    assert!(emerged);
    assert!(!o.hiding_here());
    let evs = listen_and_gather(&mut o, Species::Crab);
    assert!(matches!(
        evs[0],
        Ev::Gathered {
            species: Species::Crab,
            points: 15,
            ..
        }
    ));
}

#[test]
fn hiding_creatures_make_no_sound() {
    let mut o = practice();
    o.walk(1);
    o.lean_in();
    // The cove's crab hides for 2.1 seconds.
    let mut t = 0.0;
    while t < 2.0 {
        for e in o.update(0.05) {
            assert!(!matches!(e, Ev::Call { .. }), "{e:?} at {t}");
        }
        t += 0.05;
    }
}

#[test]
fn stepping_back_and_leaning_in_again_startles_them_again() {
    let mut o = practice();
    o.walk(1);
    o.lean_in();
    for _ in 0..100 {
        o.update(0.05);
    }
    assert!(!o.hiding_here());
    o.step_back();
    let evs = o.lean_in();
    assert!(evs.contains(&Ev::Hide { count: 1 }));
}

#[test]
fn gathering_everything_ends_the_trip_early() {
    let mut o = practice();
    o.lean_in();
    listen_and_gather(&mut o, Species::Anemone);
    o.walk(1);
    o.lean_in();
    listen_and_gather(&mut o, Species::Crab);
    o.walk(1);
    o.lean_in();
    let first = loop {
        let evs = o.update(0.05);
        if let Some(Ev::Call { species, .. }) = evs
            .iter()
            .rev()
            .find(|e| matches!(e, Ev::Call { offset: 0, .. }))
        {
            break *species;
        }
    };
    o.gather();
    let other = if first == Species::Periwinkle {
        Species::Starfish
    } else {
        Species::Periwinkle
    };
    let evs = listen_and_gather(&mut o, other);
    assert_eq!(evs.last(), Some(&Ev::AllGathered));
    assert!(o.finished);
    assert_eq!(o.remaining(), 0);
    assert_eq!(o.score(), 5 + 15 + 5 + 10);
    assert!(o.walk(1).is_empty());
    assert!(o.update(1.0).is_empty());
}

#[test]
fn the_tide_turns_warns_and_comes_in() {
    let mut o = beach(Place::Cove, 9);
    let total = o.plan.tide_secs;
    let mut seen = Vec::new();
    let mut t = 0.0;
    while !o.finished {
        for e in o.update(0.5) {
            match e {
                Ev::TideTurned | Ev::TideWarning(_) | Ev::TideIn => seen.push((e, t)),
                _ => {}
            }
        }
        t += 0.5;
        assert!(t < total + 5.0);
    }
    let kinds: Vec<Ev> = seen.iter().map(|(e, _)| e.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            Ev::TideTurned,
            Ev::TideWarning(60),
            Ev::TideWarning(20),
            Ev::TideIn
        ]
    );
    let turned = seen[0].1;
    assert!((turned - total / 2.0).abs() <= 1.0, "{turned}");
    assert_eq!(o.tide_left, 0.0);
    assert!((o.tide_in() - 1.0).abs() < 1e-9);
    assert!(o.gather().is_empty());
}

#[test]
fn tides_change_the_length_of_a_trip() {
    for seed in 0..60 {
        let mut rng = Rng::seeded(seed);
        let plan = Plan::beach(Place::Point, &mut rng);
        let want = 240.0
            + match plan.conditions.tide {
                Tide::Neap => -30.0,
                Tide::Ordinary => 0.0,
                Tide::Spring => 45.0,
            };
        assert_eq!(plan.tide_secs, want);
    }
}

#[test]
fn places_grow_longer_and_shyer() {
    let mut last = 0;
    let mut shy = 0.0;
    for place in Place::ALL {
        assert!(place.pools() > last);
        assert!(place.shy_scale() > shy);
        last = place.pools();
        shy = place.shy_scale();
        for seed in 0..30 {
            let o = beach(place, seed);
            assert_eq!(o.pools.len(), place.pools());
            assert!(o.total() >= place.pools().div_ceil(2), "enough to find");
        }
    }
    assert!(Place::Reef.commons().len() > Place::Cove.commons().len());
}

#[test]
fn rares_only_turn_up_when_the_day_allows() {
    let mut found = 0;
    for seed in 0..300 {
        let o = beach(Place::Reef, seed);
        for p in &o.pools {
            for f in &p.finds {
                if f.species.rare() {
                    found += 1;
                    assert!(
                        o.plan.conditions.allows(f.species),
                        "{:?} on {:?}",
                        f.species,
                        o.plan.conditions
                    );
                }
            }
        }
    }
    assert!(found > 20, "rares do turn up: {found}");
}

#[test]
fn each_rare_has_its_own_condition() {
    let c = Conditions {
        weather: Weather::Sunny,
        tide: Tide::Spring,
        time: TimeOfDay::Dusk,
    };
    assert_eq!(
        c.rares(),
        vec![Species::Cowrie, Species::Seahorse, Species::Octopus]
    );
    let c = Conditions {
        weather: Weather::Foggy,
        tide: Tide::Ordinary,
        time: TimeOfDay::Morning,
    };
    assert_eq!(c.rares(), vec![Species::SandDollar]);
    let mut rng = Rng::seeded(1);
    for rare in Species::RARE {
        assert!(Conditions::allowing(rare, &mut rng).allows(rare));
        assert!(rare.when().is_some());
        assert_eq!(rare.points(), 50);
    }
    for common in Species::COMMON {
        assert!(common.when().is_none());
        assert!(c.allows(common));
    }
}

#[test]
fn species_have_stable_ids_and_distinct_sounds() {
    let mut words = std::collections::HashSet::new();
    for s in Species::ALL {
        assert_eq!(Species::from_id(s.id()), Some(s));
        assert!(words.insert(s.sound_words()), "{:?}", s);
    }
    assert_eq!(Species::from_id("dragon"), None);
    assert_eq!(Species::Crab.count_words(1), "1 shore crab");
    assert_eq!(Species::Crab.count_words(3), "3 shore crabs");
    assert_eq!(Species::Starfish.count_words(2), "2 starfish");
}

#[test]
fn a_busy_shore_has_something_in_every_pool() {
    for seed in 0..40 {
        let mut rng = Rng::seeded(seed);
        let mut plan = Plan::beach(Place::Point, &mut rng);
        plan.modifier = Modifier::Crowded;
        let pools = generate(&plan, &mut rng);
        assert!(pools.iter().all(|p| !p.finds.is_empty()));
    }
}

#[test]
fn a_quick_tide_doubles_every_find() {
    let mut plan = Plan::practice();
    plan.modifier = Modifier::QuickTide;
    let mut o = Outing::new(plan, Rng::seeded(2));
    o.lean_in();
    let evs = listen_and_gather(&mut o, Species::Anemone);
    assert!(matches!(evs[0], Ev::Gathered { points: 10, .. }));
}

#[test]
fn tonights_three_species_each_score_a_bonus() {
    let mut plan = Plan::practice();
    plan.targets = vec![Species::Anemone, Species::Crab, Species::Starfish];
    let mut o = Outing::new(plan, Rng::seeded(2));
    o.lean_in();
    let evs = listen_and_gather(&mut o, Species::Anemone);
    assert!(matches!(evs[0], Ev::Gathered { target: true, .. }));
    assert_eq!(o.bonus(), 100);
    assert!(!o.goal_met());
    o.walk(1);
    o.lean_in();
    listen_and_gather(&mut o, Species::Crab);
    assert_eq!(o.score(), 5 + 15 + 200);
}

#[test]
fn the_daily_is_three_species_at_dusk_with_a_twist() {
    let d = Date::new(2026, 10, 16);
    let challenge = Challenge::on(d, tide_pools::ID, Period::Daily);
    let plan = Plan::challenge(Period::Daily, &mut challenge.rng());
    assert_eq!(plan.mode, Mode::Challenge(Period::Daily));
    assert_eq!(plan.targets.len(), 3);
    let mut t = plan.targets.clone();
    t.dedup();
    assert_eq!(t.len(), 3);
    assert_eq!(plan.conditions.time, TimeOfDay::Dusk);
    assert!(Modifier::DAILY.contains(&plan.modifier));
    let o = tide_pools::outing::challenge_outing(&challenge);
    for target in &plan.targets {
        assert!(
            o.pools
                .iter()
                .any(|p| p.finds.iter().any(|f| f.species == *target)),
            "{target:?} is on the shore"
        );
    }
    // Everyone gets the same shore.
    let again = tide_pools::outing::challenge_outing(&challenge);
    assert_eq!(o.pools, again.pools);
    let tomorrow = Challenge::on(Date::new(2026, 10, 17), tide_pools::ID, Period::Daily);
    let other = tide_pools::outing::challenge_outing(&tomorrow);
    assert!(o.pools != other.pools || o.plan.targets != other.plan.targets);
}

#[test]
fn the_weekly_hides_one_rare_find_on_a_long_reef() {
    for day in 1..=28 {
        let d = Date::new(2026, 2, day);
        let challenge = Challenge::on(d, tide_pools::ID, Period::Weekly);
        let o = tide_pools::outing::challenge_outing(&challenge);
        let Modifier::RareHunt(rare) = o.plan.modifier else {
            panic!("a rare hunt")
        };
        assert!(rare.rare());
        assert!(o.plan.conditions.allows(rare));
        assert_eq!(o.pools.len(), 12);
        let at: Vec<usize> = o
            .pools
            .iter()
            .enumerate()
            .filter(|(_, p)| p.finds.iter().any(|f| f.species == rare))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(at.len(), 1, "exactly one {rare:?}");
        assert!(at[0] >= 2, "not right by the start");
        assert!(o.plan.tide_secs > Plan::challenge(Period::Daily, &mut Rng::seeded(1)).tide_secs);
    }
}

#[test]
fn the_monthly_great_low_tide_brings_out_the_rares() {
    let challenge = Challenge::on(Date::new(2026, 10, 16), tide_pools::ID, Period::Monthly);
    let o = tide_pools::outing::challenge_outing(&challenge);
    assert_eq!(o.plan.modifier, Modifier::GreatLowTide);
    assert_eq!(o.plan.conditions.tide, Tide::Spring);
    assert_eq!(o.pools.len(), 15);
    let rares = o
        .pools
        .iter()
        .flat_map(|p| p.finds.iter())
        .filter(|f| f.species.rare())
        .count();
    assert!(rares >= 2, "{rares}");
}

#[test]
fn the_great_low_tide_scores_every_kind() {
    let mut plan = Plan::practice();
    plan.modifier = Modifier::GreatLowTide;
    let mut o = Outing::new(plan, Rng::seeded(2));
    o.lean_in();
    listen_and_gather(&mut o, Species::Anemone);
    assert_eq!(o.score(), 5 + 40);
}

#[test]
fn the_rare_hunt_pays_300() {
    let mut plan = Plan::practice();
    plan.modifier = Modifier::RareHunt(Species::Anemone);
    let mut o = Outing::new(plan, Rng::seeded(2));
    assert_eq!(o.bonus(), 0);
    o.lean_in();
    listen_and_gather(&mut o, Species::Anemone);
    assert!(o.goal_met());
    assert_eq!(o.score(), 305);
}

#[test]
fn hints_point_to_the_nearest_pool_with_something_left() {
    let mut o = practice();
    assert_eq!(o.nearest(), Some(0));
    o.lean_in();
    listen_and_gather(&mut o, Species::Anemone);
    assert_eq!(o.nearest(), Some(1));
    o.walk(1);
    o.lean_in();
    listen_and_gather(&mut o, Species::Crab);
    o.walk(-1);
    assert_eq!(o.nearest(), Some(2), "two pools to the right");
    o.walk(1);
    o.walk(1);
    assert_eq!(o.nearest(), Some(0));
}

#[test]
fn time_is_spoken_kindly() {
    assert_eq!(minutes_words(180.0), "3 minutes");
    assert_eq!(minutes_words(150.0), "2 and a half minutes");
    assert_eq!(minutes_words(60.0), "1 minute");
    assert_eq!(minutes_words(95.0), "a minute and a half");
    assert_eq!(minutes_words(42.0), "40 seconds");
    assert_eq!(minutes_words(2.0), "10 seconds");
    assert_eq!(list_words(&["a", "b", "c"]), "a, b and c");
    assert_eq!(list_words(&["a"]), "a");
}

#[test]
fn the_aquarium_counts_and_opens_new_shores() {
    let mut a = Aquarium::default();
    assert!(a.open(Place::Cove));
    assert!(!a.open(Place::Point));
    let new = a.add(
        &[Species::Crab, Species::Crab, Species::Starfish],
        "at Sandy Cove",
    );
    assert_eq!(new, vec![Species::Crab, Species::Starfish]);
    assert_eq!(a.count(Species::Crab), 2);
    assert_eq!(a.kinds(), 2);
    assert_eq!(a.creatures(), 3);
    // Only a finished trip counts as an outing.
    assert_eq!(a.outings, 0);
    let new = a.add(&[Species::Crab, Species::Anemone, Species::Mussel], "later");
    assert_eq!(new, vec![Species::Anemone, Species::Mussel]);
    assert!(a.open(Place::Point));
    assert!(!a.open(Place::Reef));
    assert_eq!(
        a.first_note(Species::Crab).as_deref(),
        Some("First found at Sandy Cove.")
    );
    assert_eq!(a.missing().len(), Species::ALL.len() - 4);
}

#[test]
fn the_practice_pools_have_no_tide() {
    let mut o = practice();
    for _ in 0..1500 {
        let evs = o.update(1.0);
        assert!(
            !evs.iter()
                .any(|e| matches!(e, Ev::TideTurned | Ev::TideWarning(_) | Ev::TideIn)),
            "{evs:?}"
        );
    }
    assert!(!o.finished);
    assert_eq!(o.tide_in(), 0.0);
}
