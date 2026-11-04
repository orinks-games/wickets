//! The challenge rules on their own: the daily twists, the weekly marathon
//! and the monthly festival's patterns.

use basket_run::challenge::{challenge_name, challenge_rule, challenge_tag};
use basket_run::rules::*;
use parlor_core::{Challenge, Date, Period, Rng};

const SEED: u64 = 0xE66;
const DAY: Date = Date::new(2026, 10, 16);

/// A run past its lead-in.
fn playing(mut run: Run) -> Run {
    run.start_now();
    run.take_events();
    run
}

/// Runs `secs` of play, collecting events, with `player` choosing a step
/// (-1, 0 or 1) before every tick.
fn drive(run: &mut Run, secs: f64, mut player: impl FnMut(&Run) -> i32) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..(secs / 0.05).round() as usize {
        let step = player(run);
        if step != 0 {
            run.step(step);
        }
        run.update(0.05);
        events.extend(run.take_events());
        if run.over().is_some() || run.phase == Phase::Between {
            break;
        }
    }
    events
}

/// Steps toward the lane of the next egg to land; else stays.
fn chase_eggs(run: &Run) -> i32 {
    let next = run
        .falling
        .iter()
        .filter(|f| f.kind.is_egg())
        .min_by(|a, b| a.left.total_cmp(&b.left));
    match next {
        Some(f) => (f.lane as i32 - run.lane as i32).signum(),
        None => 0,
    }
}

/// A festival run in play with only one pattern of the given kind left,
/// starting to fall now. Returns the pattern's group.
fn festival_with(pattern: Pattern) -> (Run, Group) {
    for seed in 0..200 {
        let mut run = playing(Run::new(Mode::Festival, Rng::seeded(seed)));
        let Some(group) = run.groups.iter().find(|g| g.pattern == pattern).copied() else {
            continue;
        };
        assert!(run.drop_pattern_now(pattern));
        run.schedule.retain(|p| p.group == group.id);
        run.spec.duration = 1000.0;
        return (run, group);
    }
    panic!("no festival round with a {pattern:?}");
}

#[test]
fn periods_and_modes_match_up() {
    for period in Period::ALL {
        let mode = Mode::for_period(period);
        assert_eq!(mode.period(), Some(period));
        assert_eq!(mode.board(), None, "challenges keep their own bests");
    }
    assert_eq!(Mode::Meadow.period(), None);
    assert_eq!(Mode::Daily.rounds(), Some(1));
    assert_eq!(Mode::Weekly.rounds(), Some(WEEKLY_ROUNDS));
    assert_eq!(Mode::Festival.rounds(), Some(FESTIVAL_ROUNDS));
}

#[test]
fn the_daily_twist_comes_from_the_seed_and_is_stated() {
    let ch = Challenge::on(DAY, basket_run::ID, Period::Daily);
    let run = Run::new(Mode::Daily, ch.rng());
    assert_eq!(run.twist, Some(Twist::for_seed(ch.seed)));
    assert!(run.round_intro().starts_with(&format!(
        "Today's meadow, with the {} twist.",
        run.twist.unwrap().name()
    )));
    assert!(run.status().contains("Twist: "), "{}", run.status());
    assert!(challenge_tag(&ch).contains(run.twist.unwrap().name()));
    assert!(challenge_rule(&ch).contains(run.twist.unwrap().describe()));
    // Different days bring different twists.
    let twists: std::collections::HashSet<Twist> = (1..=28)
        .map(|d| {
            Twist::for_seed(Challenge::on(Date::new(2026, 2, d), "basket-run", Period::Daily).seed)
        })
        .collect();
    assert!(twists.len() >= 4, "{twists:?}");
}

#[test]
fn each_twist_changes_the_round_as_it_says() {
    let plan = |t: Twist| -> Vec<Planned> {
        (0..6)
            .flat_map(|s| Run::with_twist(Mode::Daily, Rng::seeded(s), Some(t)).schedule)
            .collect()
    };
    let plain = round_spec(Mode::Daily, 1);
    let golden = Run::with_twist(Mode::Daily, Rng::seeded(SEED), Some(Twist::GoldenRush));
    assert!(golden.spec.golden >= plain.golden * 3.0);
    let goldens = plan(Twist::GoldenRush)
        .iter()
        .filter(|p| p.kind == Kind::Golden)
        .count();
    let normal: usize = (0..6)
        .map(|s| {
            plan_round(&plain, &mut Rng::seeded(s))
                .iter()
                .filter(|p| p.kind == Kind::Golden)
                .count()
        })
        .sum();
    assert!(goldens > normal * 2, "{goldens} golden against {normal}");

    let swarm = plan(Twist::BeeSwarm);
    assert!(
        swarm.iter().all(|p| p.kind != Kind::Mud),
        "no mud in a bee swarm"
    );
    assert!(swarm.iter().filter(|p| p.kind == Kind::Bee).count() > 20);

    let fast = Run::with_twist(Mode::Daily, Rng::seeded(SEED), Some(Twist::FastFall));
    assert_eq!(fast.spec.fall, MIN_FALL);
    assert!(fast.spec.gap < plain.gap);

    let mud = plan(Twist::MudSlide);
    assert!(
        mud.iter().all(|p| p.kind != Kind::Bee),
        "no bees in a mud slide"
    );
    assert!(mud.iter().filter(|p| p.kind == Kind::Mud).count() > 20);

    let mut small = playing(Run::with_twist(
        Mode::Daily,
        Rng::seeded(SEED),
        Some(Twist::SmallBasket),
    ));
    assert_eq!(small.capacity(), 3);
    small.schedule.clear();
    small.spec.duration = 1000.0;
    let mut events = Vec::new();
    for _ in 0..3 {
        let lane = small.lane;
        small.drop_now(Kind::Egg, lane);
        events.extend(drive(&mut small, 3.0, |_| 0));
    }
    assert!(events.contains(&Event::BasketFull), "{events:?}");
    for t in Twist::ALL {
        assert!(!t.describe().is_empty() && !t.name().is_empty());
    }
}

#[test]
fn the_daily_is_one_round_whose_cracks_cost_baskets() {
    let mut run = playing(Run::new(Mode::Daily, Rng::seeded(SEED)));
    assert_eq!(run.lanes(), 5);
    assert_eq!(run.needed(), 0);
    assert_eq!(run.cracks_to_spare(), Some(CRACKS_PER_BASKET));
    let events = drive(&mut run, 200.0, |_| 0);
    assert!(run.over().is_some());
    assert!(
        events.contains(&Event::Finished) || events.contains(&Event::OutOfBaskets),
        "{events:?}"
    );
}

#[test]
fn the_weekly_marathon_runs_rounds_four_to_eight_with_targets() {
    let mut run = playing(Run::new(Mode::Weekly, Rng::seeded(SEED)));
    assert!(run.twist.is_none());
    for n in 1..=WEEKLY_ROUNDS {
        let spec = round_spec(Mode::Weekly, n);
        let meadow = meadow_round(n + WEEKLY_FROM - 1);
        assert_eq!(
            (spec.number, spec.lanes, spec.fall),
            (n, meadow.lanes, meadow.fall)
        );
        assert!(spec.quota > 0.0, "round {n} has an egg target");
    }
    assert!(run
        .round_intro()
        .starts_with("Marathon, round 1 of 5. 4 lanes"));
    assert!(run.needed() > 0);
    assert_eq!(run.cart_lane(), 0);
    assert_eq!(run.cracks_to_spare(), None);
    // Each round cleared by banking its target, the cart swapping sides.
    for n in 1..=WEEKLY_ROUNDS {
        assert_eq!(run.round, n);
        assert_eq!(
            run.cart_lane(),
            if n % 2 == 0 { run.lanes() - 1 } else { 0 }
        );
        run.schedule.clear();
        run.banked_round = run.needed();
        run.clock = run.spec.duration;
        run.update(0.05);
        let events = run.take_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::RoundCleared { round, .. } if *round == n)),
            "{events:?}"
        );
        if n < WEEKLY_ROUNDS {
            assert_eq!(run.phase, Phase::Between);
            run.next_round();
            run.start_now();
        } else {
            assert!(events.contains(&Event::Won));
            assert_eq!(run.over(), Some(true));
        }
    }
    assert_eq!(run.rounds_cleared, WEEKLY_ROUNDS);
}

#[test]
fn the_weekly_marathon_ends_on_a_missed_target() {
    let mut run = playing(Run::new(Mode::Weekly, Rng::seeded(SEED)));
    let events = drive(&mut run, 200.0, |_| 0);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::RoundFailed { .. })),
        "{events:?}"
    );
    assert_eq!(run.over(), Some(false));
}

#[test]
fn festival_rounds_hold_every_pattern_and_land_in_time() {
    for seed in 0..12 {
        let mut next = 1;
        for n in 1..=FESTIVAL_ROUNDS {
            let spec = round_spec(Mode::Festival, n);
            assert_eq!(spec.lanes, 5);
            assert!(spec.fall >= MIN_FALL);
            let (plan, groups) =
                plan_festival(&spec, &mut Rng::seeded(seed * 7 + n as u64), &mut next);
            for p in Pattern::ALL {
                assert!(
                    groups.iter().any(|g| g.pattern == p),
                    "seed {seed} round {n}: no {p:?}"
                );
            }
            for p in &plan {
                assert!(p.at + spec.fall <= spec.duration, "lands before the bell");
                assert!(p.kind != Kind::Bee || p.group != 0, "bees only in fences");
            }
            for g in &groups {
                let drops: Vec<&Planned> = plan.iter().filter(|p| p.group == g.id).collect();
                assert_eq!(drops.len() as u32, g.to_land);
                match g.pattern {
                    Pattern::Sweep => {
                        assert_eq!(drops.len(), spec.lanes);
                        assert_eq!(drops[0].lane, g.lane);
                        assert!(g.lane == 0 || g.lane == spec.lanes - 1);
                    }
                    Pattern::Fence => {
                        assert!(drops.iter().all(|d| d.at == drops[0].at));
                        let gold: Vec<_> =
                            drops.iter().filter(|d| d.kind == Kind::Golden).collect();
                        assert_eq!(gold.len(), 1);
                        assert_eq!(gold[0].lane, g.lane, "the gap holds the golden egg");
                    }
                    Pattern::Nest => {
                        assert!(drops.iter().all(|d| d.lane == g.lane));
                        assert_eq!(drops.last().unwrap().kind, Kind::Mud);
                    }
                }
            }
        }
        // Group ids never repeat across rounds.
        assert!(next > 3 * FESTIVAL_ROUNDS);
    }
}

#[test]
fn a_sweep_run_along_earns_its_bonus() {
    let (mut run, group) = festival_with(Pattern::Sweep);
    let before = run.score;
    let events = drive(&mut run, 10.0, chase_eggs);
    assert_eq!(
        events.first(),
        Some(&Event::Pattern {
            pattern: Pattern::Sweep,
            lane: group.lane
        }),
        "the call comes before its drops"
    );
    assert!(events.contains(&Event::PatternDone {
        pattern: Pattern::Sweep,
        bonus: PATTERN_BONUS
    }));
    assert_eq!(run.patterns_done, 1);
    assert!(run.score >= before + PATTERN_BONUS);
    assert!(run.status().contains("Patterns caught cleanly: 1."));
}

#[test]
fn a_sweep_left_alone_breaks() {
    let (mut run, _) = festival_with(Pattern::Sweep);
    let events = drive(&mut run, 10.0, |_| 0);
    assert!(events.contains(&Event::PatternBroken {
        pattern: Pattern::Sweep
    }));
    assert_eq!(run.patterns_done, 0);
}

#[test]
fn a_fence_is_passed_in_its_gap_and_stings_elsewhere() {
    let (mut run, group) = festival_with(Pattern::Fence);
    let gap = group.lane;
    let events = drive(&mut run, 6.0, |r| (gap as i32 - r.lane as i32).signum());
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Caught {
            kind: Kind::Golden,
            ..
        }
    )));
    assert!(events.contains(&Event::PatternDone {
        pattern: Pattern::Fence,
        bonus: PATTERN_BONUS
    }));
    assert_eq!(run.lives, LIVES);

    let (mut run, group) = festival_with(Pattern::Fence);
    let wrong = if group.lane == 0 { 1 } else { 0 };
    run.lane = wrong;
    run.update(0.05);
    let hint = run.hint();
    assert!(hint.contains("bee is coming down in your lane"), "{hint}");
    let to = group.lane;
    if to.abs_diff(wrong) > 1 {
        assert!(hint.contains(&format!("to lane {}", to + 1)), "{hint}");
    }
    let events = drive(&mut run, 6.0, |_| 0);
    assert!(events.iter().any(|e| matches!(e, Event::Stung { .. })));
    assert!(events.contains(&Event::PatternBroken {
        pattern: Pattern::Fence
    }));
}

#[test]
fn a_nest_is_caught_then_dodged() {
    let (mut run, group) = festival_with(Pattern::Nest);
    let lane = group.lane;
    run.lane = lane;
    // Until the mud has started to fall behind the eggs.
    while !run.falling.iter().any(|f| f.kind == Kind::Mud) {
        run.update(0.05);
    }
    run.take_events();
    let hint = run.hint();
    assert!(hint.starts_with("Stay here:"), "{hint}");
    assert!(
        hint.contains("Then step aside: a mud clod follows"),
        "{hint}"
    );
    let away = if lane == 0 { 1 } else { -1 };
    let events = drive(&mut run, 8.0, |r| {
        let eggs_left =
            r.falling.iter().any(|f| f.kind.is_egg()) || r.schedule.iter().any(|p| p.kind.is_egg());
        if !eggs_left && r.lane == lane {
            away
        } else {
            0
        }
    });
    let caught = events
        .iter()
        .filter(|e| matches!(e, Event::Caught { .. }))
        .count();
    assert_eq!(caught, 3);
    assert!(events.iter().any(|e| matches!(e, Event::MudMissed { .. })));
    assert!(events.contains(&Event::PatternDone {
        pattern: Pattern::Nest,
        bonus: PATTERN_BONUS
    }));

    // Staying put lets the mud spoil the nest.
    let (mut run, group) = festival_with(Pattern::Nest);
    run.lane = group.lane;
    let events = drive(&mut run, 8.0, |_| 0);
    assert!(events.contains(&Event::Splat {
        lane: group.lane,
        spoiled: 3
    }));
    assert!(events.contains(&Event::PatternBroken {
        pattern: Pattern::Nest
    }));
}

#[test]
fn the_festival_is_three_rounds_and_cracks_cost_baskets() {
    let mut run = playing(Run::new(Mode::Festival, Rng::seeded(SEED)));
    assert!(run
        .round_intro()
        .starts_with("Festival, round 1 of 3. 5 lanes"));
    assert_eq!(run.cracks_to_spare(), Some(CRACKS_PER_BASKET));
    assert!(round_spec(Mode::Festival, 3).fall < round_spec(Mode::Festival, 1).fall);
    for n in 1..=FESTIVAL_ROUNDS {
        assert_eq!(run.round, n);
        assert!(!run.groups.is_empty());
        run.schedule.clear();
        run.clock = run.spec.duration;
        run.update(0.05);
        let events = run.take_events();
        if n < FESTIVAL_ROUNDS {
            assert_eq!(run.phase, Phase::Between, "{events:?}");
            run.next_round();
            run.start_now();
        } else {
            assert!(events.contains(&Event::Won));
        }
    }
    assert_eq!(run.over(), Some(true));
}

#[test]
fn every_challenge_has_a_name_tag_and_rule() {
    for period in Period::ALL {
        let ch = Challenge::on(DAY, basket_run::ID, period);
        assert!(!challenge_name(period).is_empty());
        assert!(!challenge_tag(&ch).is_empty());
        assert!(challenge_rule(&ch).len() > 60);
    }
    let monthly = Challenge::on(DAY, basket_run::ID, Period::Monthly);
    let rule = challenge_rule(&monthly);
    for word in ["patterns", "100 points"] {
        assert!(rule.contains(word), "{rule}");
    }
}
