//! The rules on their own, driven by `dt` with a seeded random source.

use basket_run::rules::*;
use parlor_core::Rng;

const SEED: u64 = 0xE66;

/// A run in play with nothing scheduled, so a test drops what it wants.
fn quiet(mode: Mode) -> Run {
    let mut run = Run::new(mode, Rng::seeded(SEED));
    run.schedule.clear();
    // Long enough that no test runs into the bell by accident.
    run.spec.duration = 1000.0;
    run.update(LEAD_IN + 0.01);
    assert_eq!(run.phase, Phase::Playing);
    run.take_events();
    run
}

/// Runs until nothing is falling, collecting events.
fn settle(run: &mut Run) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..400 {
        run.update(0.05);
        events.extend(run.take_events());
        if run.falling.is_empty() {
            break;
        }
    }
    events
}

fn catch_at(run: &mut Run, kind: Kind) -> Vec<Event> {
    let lane = run.lane;
    run.drop_now(kind, lane);
    settle(run)
}

#[test]
fn the_meadow_ramps_up_and_never_rushes_a_listener() {
    let mut prev = meadow_round(1);
    assert_eq!(prev.lanes, 3);
    for n in 2..=ROUNDS {
        let r = meadow_round(n);
        assert!(r.lanes >= prev.lanes, "round {n} lanes");
        assert!(r.fall <= prev.fall, "round {n} warning time");
        assert!(r.gap <= prev.gap, "round {n} gap");
        assert!(r.duration >= prev.duration, "round {n} length");
        assert!(r.fall >= MIN_FALL);
        prev = r;
    }
    assert_eq!(meadow_round(ROUNDS).lanes, 5);
    for n in [1, 5, 20, 100] {
        let e = round_spec(Mode::Endless, n);
        assert!(e.fall >= MIN_FALL, "endless round {n}");
        assert!(e.gap >= 1.0);
        assert_eq!(e.quota, 0.0);
    }
    assert_eq!(round_spec(Mode::Endless, 100).fall, MIN_FALL);
    assert!(round_spec(Mode::Endless, 10).gap < round_spec(Mode::Endless, 1).gap);
}

#[test]
fn a_round_plan_is_the_same_for_the_same_seed_and_lands_in_time() {
    for n in 1..=ROUNDS {
        let spec = meadow_round(n);
        let a = plan_round(&spec, &mut Rng::seeded(n as u64));
        let b = plan_round(&spec, &mut Rng::seeded(n as u64));
        assert_eq!(a, b);
        assert!(!a.is_empty());
        assert!(a.windows(2).all(|w| w[0].at <= w[1].at), "sorted");
        for p in &a {
            assert!(p.lane < spec.lanes);
            assert!(
                p.at + spec.fall <= spec.duration,
                "round {n}: lands before the bell"
            );
        }
    }
    let first = plan_round(&meadow_round(1), &mut Rng::seeded(3));
    assert!(
        first.iter().all(|p| p.kind == Kind::Egg),
        "round 1 is only eggs"
    );
    let later: Vec<Planned> = (0..10)
        .flat_map(|s| plan_round(&meadow_round(8), &mut Rng::seeded(s)))
        .collect();
    for kind in Kind::ALL {
        assert!(
            later.iter().any(|p| p.kind == kind),
            "round 8 drops {kind:?}"
        );
    }
}

#[test]
fn the_round_starts_after_a_lead_in_with_the_player_at_the_cart() {
    let mut run = Run::new(Mode::Meadow, Rng::seeded(SEED));
    assert!(matches!(run.phase, Phase::LeadIn(_)));
    assert_eq!(run.lane, run.cart_lane());
    assert_eq!(run.cart_lane(), 0);
    run.update(LEAD_IN / 2.0);
    assert!(run.take_events().is_empty());
    assert_eq!(
        run.step(1),
        Move::Moved {
            lane: 1,
            at_cart: false
        }
    );
    run.update(LEAD_IN);
    assert_eq!(run.take_events(), vec![Event::Go]);
    assert_eq!(run.phase, Phase::Playing);
}

#[test]
fn stepping_stops_at_the_fences() {
    let mut run = quiet(Mode::Meadow);
    assert_eq!(run.step(-1), Move::Edge);
    assert_eq!(
        run.step(1),
        Move::Moved {
            lane: 1,
            at_cart: false
        }
    );
    assert_eq!(
        run.step(1),
        Move::Moved {
            lane: 2,
            at_cart: false
        }
    );
    assert_eq!(run.step(1), Move::Edge);
    assert_eq!(run.lane, 2);
    assert_eq!(
        run.step(-1),
        Move::Moved {
            lane: 1,
            at_cart: false
        }
    );
    assert_eq!(
        run.step(-1),
        Move::Moved {
            lane: 0,
            at_cart: true
        }
    );
}

#[test]
fn an_egg_in_your_lane_lands_in_the_basket() {
    let mut run = quiet(Mode::Meadow);
    let events = catch_at(&mut run, Kind::Egg);
    assert!(events.contains(&Event::Caught {
        kind: Kind::Egg,
        lane: 0,
        value: EGG_POINTS,
        held: 1
    }));
    assert_eq!(run.basket, vec![EGG_POINTS]);
    assert_eq!(run.streak, 1);
    assert_eq!(run.caught_total, 1);
}

#[test]
fn an_egg_takes_its_whole_fall_to_land() {
    let mut run = quiet(Mode::Meadow);
    run.drop_now(Kind::Egg, 0);
    let fall = run.spec.fall;
    let mut t = 0.0;
    while t + 0.05 < fall - 0.01 {
        run.update(0.05);
        t += 0.05;
    }
    assert_eq!(run.falling.len(), 1, "still falling at {t}");
    assert!(run.falling[0].progress() > 0.9);
    run.update(0.1);
    assert!(run.falling.is_empty());
}

#[test]
fn a_missed_egg_cracks_and_breaks_the_streak() {
    let mut run = quiet(Mode::Meadow);
    for _ in 0..3 {
        catch_at(&mut run, Kind::Egg);
    }
    assert_eq!(run.streak, 3);
    run.drop_now(Kind::Egg, 2);
    let events = settle(&mut run);
    assert!(events.contains(&Event::Cracked {
        kind: Kind::Egg,
        lane: 2
    }));
    assert!(events.contains(&Event::StreakBroken(3)));
    assert_eq!(run.streak, 0);
    assert_eq!(run.cracks_round, 1);
    assert_eq!(run.basket.len(), 3, "the basket keeps what it had");
}

#[test]
fn a_streak_raises_each_eggs_value_up_to_double() {
    let mut run = quiet(Mode::Meadow);
    run.spec.capacity = 20;
    let mut values = Vec::new();
    for _ in 0..13 {
        catch_at(&mut run, Kind::Egg);
        values.push(*run.basket.last().unwrap());
    }
    assert_eq!(&values[..4], &[10, 11, 12, 13]);
    assert_eq!(values[10], 20);
    assert_eq!(values[12], 20, "capped at double");
    catch_at(&mut run, Kind::Golden);
    assert_eq!(*run.basket.last().unwrap(), GOLDEN_POINTS * 2);
}

#[test]
fn a_streak_of_five_is_announced() {
    let mut run = quiet(Mode::Meadow);
    run.spec.capacity = 10;
    let mut events = Vec::new();
    for _ in 0..5 {
        events.extend(catch_at(&mut run, Kind::Egg));
    }
    assert!(events.contains(&Event::Streak(5)));
}

#[test]
fn a_full_basket_says_so_and_bounces_the_next_egg_out() {
    let mut run = quiet(Mode::Meadow);
    let cap = run.capacity();
    let mut events = Vec::new();
    for _ in 0..cap {
        events.extend(catch_at(&mut run, Kind::Egg));
    }
    assert!(events.contains(&Event::BasketFull));
    assert!(run.basket_full());
    let events = catch_at(&mut run, Kind::Egg);
    assert!(events.contains(&Event::Overflow {
        kind: Kind::Egg,
        lane: 0
    }));
    assert_eq!(run.basket.len(), cap);
    assert_eq!(run.cracks_round, 1);
}

#[test]
fn mud_in_the_basket_spoils_it_and_mud_elsewhere_does_nothing() {
    let mut run = quiet(Mode::Meadow);
    catch_at(&mut run, Kind::Egg);
    catch_at(&mut run, Kind::Egg);
    run.drop_now(Kind::Mud, 2);
    let events = settle(&mut run);
    assert!(events.contains(&Event::MudMissed { lane: 2 }));
    assert_eq!(run.basket.len(), 2);
    let events = catch_at(&mut run, Kind::Mud);
    assert!(events.contains(&Event::Splat {
        lane: 0,
        spoiled: 2
    }));
    assert!(run.basket.is_empty());
    assert_eq!(run.streak, 0);
    assert_eq!(run.lives, LIVES, "mud costs eggs, not baskets");
}

#[test]
fn bee_stings_break_baskets_until_the_run_ends() {
    let mut run = quiet(Mode::Meadow);
    catch_at(&mut run, Kind::Egg);
    run.drop_now(Kind::Bee, 1);
    let events = settle(&mut run);
    assert!(events.contains(&Event::BeeMissed { lane: 1 }));
    let events = catch_at(&mut run, Kind::Bee);
    assert!(events.contains(&Event::Stung { lane: 0, lives: 2 }));
    assert_eq!(run.basket.len(), 1, "a sting keeps the eggs");
    catch_at(&mut run, Kind::Bee);
    let events = catch_at(&mut run, Kind::Bee);
    assert!(events.contains(&Event::OutOfBaskets));
    assert_eq!(run.lives, 0);
    assert_eq!(run.over(), Some(false));
    assert_eq!(run.step(1), Move::Idle);
    assert_eq!(run.bank(), Bank::Idle);
}

#[test]
fn practice_stings_cost_nothing() {
    let mut run = quiet(Mode::Practice);
    for _ in 0..4 {
        catch_at(&mut run, Kind::Bee);
    }
    assert_eq!(run.lives, LIVES);
    assert_eq!(run.over(), None);
}

#[test]
fn banking_needs_the_cart_and_eggs() {
    let mut run = quiet(Mode::Meadow);
    assert_eq!(run.bank(), Bank::Empty);
    run.step(1);
    catch_at(&mut run, Kind::Egg);
    catch_at(&mut run, Kind::Egg);
    assert_eq!(
        run.bank(),
        Bank::NotAtCart {
            lanes: 1,
            left: true
        }
    );
    run.step(-1);
    assert_eq!(
        run.bank(),
        Bank::Banked {
            eggs: 2,
            points: 21,
            full: false
        }
    );
    assert_eq!(run.score, 21);
    assert_eq!(run.banked_round, 2);
    assert!(run.basket.is_empty());
}

#[test]
fn a_full_basket_banks_a_bonus_and_playing_by_ear_a_tenth_more() {
    let mut run = quiet(Mode::Meadow);
    let cap = run.capacity();
    for _ in 0..cap {
        catch_at(&mut run, Kind::Egg);
    }
    let sum: u64 = run.basket.iter().sum();
    assert_eq!(
        run.bank(),
        Bank::Banked {
            eggs: cap,
            points: sum + FULL_BANK_BONUS,
            full: true
        }
    );
    let mut ear = quiet(Mode::Meadow);
    ear.by_ear = true;
    catch_at(&mut ear, Kind::Golden);
    assert_eq!(
        ear.bank(),
        Bank::Banked {
            eggs: 1,
            points: 55,
            full: false
        }
    );
}

#[test]
fn the_clock_warns_ticks_and_rings_and_unbanked_eggs_are_lost() {
    let mut run = quiet(Mode::Meadow);
    catch_at(&mut run, Kind::Egg);
    run.clock = run.spec.duration - WARNING_AT - 0.01;
    let mut events = Vec::new();
    for _ in 0..300 {
        run.update(0.05);
        events.extend(run.take_events());
        if run.phase != Phase::Playing {
            break;
        }
    }
    assert!(events.contains(&Event::Warning));
    let ticks: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            Event::Tick(n) => Some(*n),
            _ => None,
        })
        .collect();
    assert_eq!(ticks, vec![5, 4, 3, 2, 1]);
    assert!(events.contains(&Event::TimeUp { lost: 1 }));
    assert!(run.basket.is_empty());
}

#[test]
fn banking_too_few_eggs_ends_the_meadow_run() {
    let mut run = quiet(Mode::Meadow);
    let needed = run.needed();
    assert!(needed > 0);
    run.clock = run.spec.duration;
    run.update(0.05);
    let events = run.take_events();
    assert!(events.contains(&Event::RoundFailed { banked: 0, needed }));
    assert_eq!(run.over(), Some(false));
}

#[test]
fn banking_enough_clears_the_round_and_the_next_moves_the_cart() {
    let mut run = quiet(Mode::Meadow);
    let needed = run.needed();
    run.spec.capacity = 50;
    for _ in 0..needed {
        catch_at(&mut run, Kind::Egg);
    }
    run.bank();
    let before = run.score;
    run.clock = run.spec.duration;
    run.update(0.05);
    let events = run.take_events();
    assert!(events.contains(&Event::RoundCleared {
        round: 1,
        banked: needed,
        bonus: ROUND_BONUS + PERFECT_BONUS,
        perfect: true
    }));
    assert_eq!(run.score, before + ROUND_BONUS + PERFECT_BONUS);
    assert_eq!(run.phase, Phase::Between);
    assert_eq!(run.step(1), Move::Idle);
    run.next_round();
    assert_eq!(run.round, 2);
    assert!(matches!(run.phase, Phase::LeadIn(_)));
    assert_eq!(run.cart_lane(), run.lanes() - 1, "the cart moves right");
    assert_eq!(run.lane, run.cart_lane(), "you start at the cart");
    assert_eq!(run.banked_round, 0);
    assert!(!run.schedule.is_empty());
}

#[test]
fn a_cracked_egg_costs_the_perfect_bonus() {
    let mut run = quiet(Mode::Meadow);
    let needed = run.needed();
    run.spec.capacity = 50;
    for _ in 0..needed {
        catch_at(&mut run, Kind::Egg);
    }
    run.drop_now(Kind::Egg, 2);
    settle(&mut run);
    run.bank();
    run.clock = run.spec.duration;
    run.update(0.05);
    assert!(run.take_events().contains(&Event::RoundCleared {
        round: 1,
        banked: needed,
        bonus: ROUND_BONUS,
        perfect: false
    }));
}

#[test]
fn clearing_round_eight_wins_the_meadow() {
    let mut run = Run::at_round(Mode::Meadow, Rng::seeded(SEED), ROUNDS);
    run.schedule.clear();
    run.update(LEAD_IN + 0.01);
    assert_eq!(run.round, ROUNDS);
    assert_eq!(run.lanes(), 5);
    let needed = run.needed().max(1);
    run.spec.capacity = 50;
    // Round 8 is even: the cart is on the right, where the player starts.
    assert!(run.at_cart());
    for _ in 0..needed {
        catch_at(&mut run, Kind::Egg);
    }
    run.bank();
    run.clock = run.spec.duration;
    run.update(0.05);
    let events = run.take_events();
    assert!(events.contains(&Event::Won));
    assert_eq!(run.over(), Some(true));
}

#[test]
fn endless_has_no_target_but_every_fifth_crack_breaks_a_basket() {
    let mut run = quiet(Mode::Endless);
    assert_eq!(run.needed(), 0);
    assert_eq!(run.cracks_to_spare(), Some(CRACKS_PER_BASKET));
    let mut events = Vec::new();
    for _ in 0..CRACKS_PER_BASKET {
        run.drop_now(Kind::Egg, 2);
        events.extend(settle(&mut run));
    }
    assert!(events.contains(&Event::CrackedBasket { lives: LIVES - 1 }));
    assert_eq!(run.lives, LIVES - 1);
    assert_eq!(run.cracks_to_spare(), Some(CRACKS_PER_BASKET));
    run.clock = run.spec.duration;
    run.update(0.05);
    assert_eq!(run.phase, Phase::Between, "no target to miss");
    run.next_round();
    assert_eq!(run.round, 2);
    assert_eq!(run.cart_lane(), 0);
    assert_eq!(quiet(Mode::Meadow).cracks_to_spare(), None);
}

#[test]
fn the_daily_meadow_is_one_long_round() {
    let fresh = Run::new(Mode::Daily, Rng::seeded(SEED));
    assert_eq!(fresh.spec.duration, DAILY_SECS);
    assert!(fresh.twist.is_some(), "a daily always has its twist");
    let mut run = quiet(Mode::Daily);
    assert_eq!(run.lanes(), 5);
    assert_eq!(run.needed(), 0);
    run.clock = run.spec.duration;
    run.update(0.05);
    let events = run.take_events();
    assert!(events.contains(&Event::Finished));
    assert_eq!(run.over(), Some(true));
    let a = Run::new(Mode::Daily, Rng::seeded(42)).schedule;
    let b = Run::new(Mode::Daily, Rng::seeded(42)).schedule;
    assert_eq!(a, b, "the same seed, the same drops for everyone");
}

#[test]
fn each_kind_is_introduced_once_when_asked() {
    let mut run = quiet(Mode::Practice);
    run.introduce = true;
    run.mark_introduced(Kind::Golden);
    run.drop_now(Kind::Egg, 0);
    run.drop_now(Kind::Egg, 1);
    run.drop_now(Kind::Golden, 2);
    let events = run.take_events();
    let intros: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::Introduce(_)))
        .collect();
    assert_eq!(intros, vec![&Event::Introduce(Kind::Egg)]);
    assert!(
        events
            .iter()
            .position(|e| *e == Event::Introduce(Kind::Egg))
            < events.iter().position(|e| matches!(e, Event::Spawn { .. })),
        "the introduction comes before the drop"
    );
    assert!(run.was_introduced(Kind::Egg));
}

#[test]
fn a_scheduled_round_drops_its_plan_in_order() {
    let mut run = Run::new(Mode::Meadow, Rng::seeded(SEED));
    let planned = run.schedule.len();
    assert_eq!(run.eggs_planned as usize, planned, "round 1 is all eggs");
    let mut spawned = 0;
    let mut landed = 0;
    for _ in 0..2000 {
        run.update(0.05);
        for e in run.take_events() {
            match e {
                Event::Spawn { .. } => spawned += 1,
                Event::Caught { .. } | Event::Cracked { .. } | Event::Overflow { .. } => {
                    landed += 1
                }
                _ => {}
            }
        }
        if run.over().is_some() {
            break;
        }
    }
    assert_eq!(spawned, planned);
    assert_eq!(landed, planned);
    assert_eq!(run.over(), Some(false), "standing still banks nothing");
}

#[test]
fn hints_and_status_say_what_matters() {
    let mut run = quiet(Mode::Meadow);
    run.drop_now(Kind::Egg, 2);
    let hint = run.hint();
    assert!(hint.starts_with("Move right"), "{hint}");
    assert!(hint.contains("lane 3"), "{hint}");
    let status = run.status();
    assert!(
        status.starts_with("Lane 1 of 3, at the cart, basket 0 of 4."),
        "{status}"
    );
    assert!(status.contains("Egg in lane 3, 2 lanes right"), "{status}");
    assert!(status.contains("3 baskets left."), "{status}");
    run.step(1);
    run.step(1);
    assert!(run.hint().starts_with("Stay here"));
    run.falling.clear();
    run.drop_now(Kind::Bee, 2);
    let hint = run.hint();
    assert!(hint.contains("bee is coming down in your lane"), "{hint}");
    assert!(hint.ends_with("Step left."), "{hint}");
    run.falling.clear();
    run.basket = vec![10; run.capacity()];
    assert!(run.hint().contains("full"), "{}", run.hint());
    assert!(run.hint().contains("in lane 1"), "{}", run.hint());
}

#[test]
fn the_round_intro_names_lanes_cart_target_and_basket() {
    let run = Run::new(Mode::Meadow, Rng::seeded(SEED));
    let intro = run.round_intro();
    assert!(
        intro.starts_with("Round 1 of 8. 3 lanes, the cart in lane 1 on the left."),
        "{intro}"
    );
    assert!(
        intro.contains(&format!(
            "Bank {} of {} eggs in 45 seconds.",
            run.needed(),
            run.eggs_planned
        )),
        "{intro}"
    );
    assert!(intro.contains("The basket holds 4."), "{intro}");
}

#[test]
fn start_now_ends_the_lead_in_once() {
    let mut run = Run::new(Mode::Meadow, Rng::seeded(7));
    assert!(matches!(run.phase, Phase::LeadIn(_)));
    assert!(run.start_now());
    assert_eq!(run.phase, Phase::Playing);
    assert!(!run.start_now(), "only from the lead-in");
    assert!(run.take_events().is_empty(), "the screen sounds the start");
    run.update(1.0);
    assert!(run.clock > 0.0, "the clock runs at once");
}

#[test]
fn hints_name_each_kind_with_its_article() {
    let mut run = Run::at_round(Mode::Meadow, Rng::seeded(7), 3);
    run.start_now();
    run.drop_now(Kind::Egg, run.lane);
    assert!(
        run.hint().starts_with("Stay here: an egg lands"),
        "{}",
        run.hint()
    );
    let mut run = Run::at_round(Mode::Meadow, Rng::seeded(7), 3);
    run.start_now();
    run.drop_now(Kind::Bee, run.lane);
    assert!(
        run.hint().starts_with("A bee is coming down"),
        "{}",
        run.hint()
    );
    for kind in Kind::ALL {
        assert!(kind.with_article().ends_with(kind.name()));
    }
}

#[test]
fn two_new_kinds_at_once_are_both_introduced() {
    let mut run = Run::at_round(Mode::Practice, Rng::seeded(7), 1);
    run.introduce = true;
    run.start_now();
    run.drop_now(Kind::Egg, 0);
    run.drop_now(Kind::Mud, 2);
    let intros: Vec<Event> = run
        .take_events()
        .into_iter()
        .filter(|e| matches!(e, Event::Introduce(_)))
        .collect();
    assert_eq!(
        intros,
        [Event::Introduce(Kind::Egg), Event::Introduce(Kind::Mud)]
    );
}
