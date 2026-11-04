//! The dance as plain rules: plans, following, gongs, envelopes,
//! firecrackers, the finales and the frame rate.

use crate::common::*;
use dragon_dance::rules::{
    self, plan, stars, Dance, Event, Finale, Grade, Mark, Move, Plan, Planned, Side, Thing,
    COMBO_BONUS, GONG_GOOD, GONG_PERFECT, LEAD_IN, START_SEGMENTS, STUMBLE_POINTS,
};
use dragon_dance::streets::{self, STREETS};
use dragon_dance::Mode;
use parlor_core::testkit;

/// Runs the dance until `beat` has just played, with no keys.
fn run_to(d: &mut Dance, beat: i64) -> Vec<Event> {
    let mut out = Vec::new();
    while d.beat() < beat && !d.finished {
        d.update(1.0 / 60.0);
        out.extend(d.take_events());
    }
    out
}

/// A tiny street of 3 lanes and 8 bars, with the marks given.
fn written(marks: Vec<Planned>) -> Dance {
    let street = rules::Street {
        bars: 8,
        finale: Finale::Calm,
        ..streets::PRACTICE
    };
    let mut marks = marks;
    marks.sort_by_key(|p| p.beat);
    Dance::with_plan(
        street,
        Plan {
            marks,
            start: 1,
            beats: 32,
        },
    )
}

fn at(beat: i64, mark: Mark) -> Planned {
    Planned { beat, mark }
}

#[test]
fn a_plan_is_the_same_for_the_same_seed() {
    for street in STREETS {
        assert_eq!(plan(&street, 7), plan(&street, 7), "{}", street.name);
    }
    assert_ne!(plan(&STREETS[3], 7), plan(&STREETS[3], 8));
}

#[test]
fn firecrackers_are_never_in_the_pearls_lane() {
    for street in STREETS
        .iter()
        .chain([&streets::NEW_YEAR, &streets::DAILY, &streets::MONTHLY])
    {
        for seed in 0..40 {
            let p = plan(street, seed);
            for m in &p.marks {
                if let Mark::Firecracker { lane } = m.mark {
                    assert_ne!(lane, p.pearl_at(m.beat), "{} seed {seed}", street.name);
                    assert!(lane < street.lanes);
                }
            }
        }
    }
}

#[test]
fn the_pearl_moves_on_a_bars_first_beat_and_rests_at_each_end() {
    for street in STREETS {
        for seed in 0..20 {
            let p = plan(&street, seed);
            for m in &p.marks {
                if let Mark::Pearl { lane } = m.mark {
                    assert_eq!(m.beat % 4, 0);
                    assert!(m.beat >= 8, "{}: moved in the first bars", street.name);
                    assert!(
                        m.beat < p.beats - 4,
                        "{}: moved in the last bar",
                        street.name
                    );
                    assert!(lane < street.lanes);
                    let before = p.pearl_at(m.beat - 1);
                    let jump = before.abs_diff(lane);
                    assert!(jump >= 1 && jump <= if street.jumps { 2 } else { 1 });
                }
            }
        }
    }
}

#[test]
fn gentle_streets_keep_firecrackers_away_from_a_moving_pearl() {
    for street in STREETS.iter().filter(|s| !s.tight) {
        for seed in 0..20 {
            let p = plan(street, seed);
            for m in &p.marks {
                if let Mark::Firecracker { .. } = m.mark {
                    let bar_start = m.beat - m.beat % 4;
                    let moved = p
                        .marks
                        .iter()
                        .any(|o| o.beat == bar_start && matches!(o.mark, Mark::Pearl { .. }));
                    assert!(!moved, "{} seed {seed}", street.name);
                }
            }
        }
    }
}

#[test]
fn the_first_street_has_no_firecrackers_and_later_ones_do() {
    for seed in 0..10 {
        let p = plan(&STREETS[0], seed);
        assert_eq!(p.count(|m| matches!(m, Mark::Firecracker { .. })), 0);
    }
    let any = (0..10)
        .any(|seed| plan(&STREETS[1], seed).count(|m| matches!(m, Mark::Firecracker { .. })) > 0);
    assert!(any);
}

#[test]
fn every_street_has_gongs_shops_and_a_sensible_length() {
    for (i, street) in STREETS.iter().enumerate() {
        let p = plan(street, 3);
        assert!(
            p.count(|m| matches!(m, Mark::Gong { .. })) >= 4,
            "{}",
            street.name
        );
        assert!(p.count(|m| matches!(m, Mark::Shop { .. })) >= 10);
        let secs = street.seconds();
        assert!(
            (60..=110).contains(&secs),
            "{}: {secs} seconds",
            street.name
        );
        if i > 0 {
            let before = STREETS[i - 1];
            assert!(street.bpm > before.bpm);
            assert!(street.lanes >= before.lanes);
        }
    }
}

#[test]
fn following_the_pearl_scores_the_dragons_length_each_beat() {
    let mut d = written(vec![]);
    run_to(&mut d, 3);
    assert_eq!(d.follow_beats, 4);
    assert_eq!(d.score, 4 * START_SEGMENTS as u64);
    assert_eq!(d.combo, 4);
}

#[test]
fn the_lead_in_counts_eight_beats_before_the_street() {
    let mut d = written(vec![]);
    let events = run_to(&mut d, 0);
    let counts: Vec<i64> = events
        .iter()
        .filter_map(|e| match e {
            Event::CountIn { left } => Some(*left),
            _ => None,
        })
        .collect();
    assert_eq!(counts, (1..=LEAD_IN).rev().collect::<Vec<_>>());
    assert!(events.contains(&Event::Start));
}

#[test]
fn one_beat_off_the_pearl_is_free_and_two_break_the_combo() {
    let mut d = written(vec![]);
    run_to(&mut d, 5);
    assert_eq!(d.combo, 6);
    d.step(-1);
    let events = run_to(&mut d, 6);
    assert_eq!(d.combo, 6);
    assert!(!events.iter().any(|e| matches!(e, Event::LostPearl { .. })));
    d.step(1);
    run_to(&mut d, 7);
    assert_eq!(d.combo, 7);
    d.step(1);
    run_to(&mut d, 8);
    let events = run_to(&mut d, 9);
    assert!(events.contains(&Event::LostPearl { combo: 7 }));
    assert_eq!(d.combo, 0);
}

#[test]
fn the_beat_the_pearl_moves_is_a_grace_beat() {
    let mut d = written(vec![at(8, Mark::Pearl { lane: 2 })]);
    let events = run_to(&mut d, 8);
    assert!(events.contains(&Event::PearlMoved { from: 1, to: 2 }));
    assert_eq!(d.pearl, 2);
    // Still in lane 2 of 3's middle: not broken on the move's own beat.
    assert_eq!(d.combo, 8);
    assert_eq!(d.step(1), Move::Moved { lane: 2 });
    run_to(&mut d, 9);
    assert_eq!(d.combo, 9);
}

#[test]
fn the_edge_of_the_street_stops_the_head() {
    let mut d = written(vec![]);
    assert_eq!(d.step(-1), Move::Moved { lane: 0 });
    assert_eq!(d.step(-1), Move::Edge { left: true });
    assert_eq!(d.step(1), Move::Moved { lane: 1 });
    assert_eq!(d.step(1), Move::Moved { lane: 2 });
    assert_eq!(d.step(1), Move::Edge { left: false });
}

#[test]
fn rearing_on_the_gong_scores_by_timing() {
    let mut d = written(vec![at(6, Mark::Gong { great: false })]);
    let events = run_to(&mut d, 4);
    assert!(events.contains(&Event::Roll { great: false }));
    run_to(&mut d, 6);
    d.rear();
    let events = d.take_events();
    assert!(
        matches!(events[..], [Event::Rear { grade: Grade::Perfect, points, .. }] if points == GONG_PERFECT),
        "{events:?}"
    );
    // A second rear, with the gong taken, is a stray that breaks the combo.
    d.rear();
    assert_eq!(d.take_events(), [Event::Stray { broke: true }]);
    assert_eq!(d.combo, 0);
}

#[test]
fn a_rear_a_little_off_the_gong_is_good_and_far_off_is_a_stray() {
    let mut d = written(vec![at(6, Mark::Gong { great: false })]);
    run_to(&mut d, 6);
    let gong = d.time_of(6);
    while d.clock < gong + 0.15 {
        d.update(1.0 / 240.0);
    }
    d.take_events();
    d.rear();
    assert!(matches!(
        d.take_events()[..],
        [Event::Rear { grade: Grade::Good, points, .. }] if points == GONG_GOOD
    ));

    let mut d = written(vec![at(6, Mark::Gong { great: false })]);
    run_to(&mut d, 4);
    d.rear();
    assert_eq!(d.take_events(), [Event::Stray { broke: true }]);
    // An early stray does not use the gong up.
    run_to(&mut d, 6);
    d.rear();
    assert!(matches!(d.take_events()[..], [Event::Rear { .. }]));
}

#[test]
fn a_gong_passed_without_a_rear_is_missed_once() {
    let mut d = written(vec![at(6, Mark::Gong { great: false })]);
    let events = run_to(&mut d, 8);
    let missed = events
        .iter()
        .filter(|e| matches!(e, Event::GongMissed { .. }))
        .count();
    assert_eq!(missed, 1);
    assert_eq!(d.gongs, 1);
    d.rear();
    assert_eq!(d.take_events(), [Event::Stray { broke: true }]);
}

#[test]
fn a_rear_just_before_the_gong_counts() {
    let mut d = written(vec![at(6, Mark::Gong { great: false })]);
    let gong = d.time_of(6);
    while d.clock < gong - 0.05 {
        d.update(1.0 / 240.0);
    }
    d.take_events();
    d.rear();
    assert!(matches!(
        d.take_events()[..],
        [Event::Rear {
            grade: Grade::Perfect,
            ..
        }]
    ));
}

#[test]
fn an_envelope_in_your_lane_grows_the_dragon() {
    let mut d = written(vec![
        at(7, Mark::Envelope { lane: 1 }),
        at(11, Mark::Envelope { lane: 0 }),
    ]);
    let events = run_to(&mut d, 3);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Warn {
            thing: Thing::Envelope,
            lane: 1,
            ..
        }
    )));
    assert_eq!(d.incoming().len(), 1);
    let events = run_to(&mut d, 7);
    assert!(events.contains(&Event::Envelope {
        lane: 1,
        points: 40,
        segments: START_SEGMENTS + 1
    }));
    let events = run_to(&mut d, 11);
    assert!(events.contains(&Event::EnvelopeMissed { lane: 0 }));
    assert_eq!(d.segments, START_SEGMENTS + 1);
    assert_eq!(d.envelopes, 1);
}

#[test]
fn the_run_up_doubles_envelopes() {
    let mut d = written(vec![at(7, Mark::Envelope { lane: 1 })]);
    d.run_up = true;
    let events = run_to(&mut d, 7);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Envelope { points: 80, .. })));
}

#[test]
fn firecrackers_make_the_tail_stumble_but_the_dance_goes_on() {
    let mut d = written(vec![
        at(13, Mark::Firecracker { lane: 1 }),
        at(17, Mark::Firecracker { lane: 0 }),
    ]);
    let events = run_to(&mut d, 9);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Warn {
            thing: Thing::Firecracker,
            lane: 1,
            ..
        }
    )));
    let events = run_to(&mut d, 13);
    let before = 13 * START_SEGMENTS as u64;
    assert!(events.contains(&Event::Stumble {
        lane: 1,
        lost: STUMBLE_POINTS
    }));
    assert_eq!(d.stumbles, 1);
    assert_eq!(d.combo, 1);
    assert_eq!(d.score, before - STUMBLE_POINTS + START_SEGMENTS as u64);
    let events = run_to(&mut d, 17);
    assert!(events.contains(&Event::Crackled { lane: 0 }));
    assert!(!d.finished);
    run_to(&mut d, 100);
    assert!(d.finished);
}

#[test]
fn a_stumble_never_takes_the_score_below_nothing_and_practice_costs_nothing() {
    let mut d = written(vec![at(0, Mark::Firecracker { lane: 1 })]);
    let events = run_to(&mut d, 0);
    assert!(events.contains(&Event::Stumble { lane: 1, lost: 0 }));
    let mut d = written(vec![at(5, Mark::Firecracker { lane: 1 })]);
    d.forgiving = true;
    let events = run_to(&mut d, 5);
    assert!(events.contains(&Event::Stumble { lane: 1, lost: 0 }));
    assert_eq!(d.stumbles, 1);
}

#[test]
fn shops_cheer_for_a_good_combo() {
    let mut d = written(vec![
        at(
            1,
            Mark::Shop {
                side: Side::Left,
                name: "the bakery",
            },
        ),
        at(
            9,
            Mark::Shop {
                side: Side::Right,
                name: "the tea house",
            },
        ),
    ]);
    let events = run_to(&mut d, 1);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Shop {
            cheer: false,
            points: 0,
            ..
        }
    )));
    let events = run_to(&mut d, 9);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Shop {
            name: "the tea house",
            cheer: true,
            points: 20,
            ..
        }
    )));
}

#[test]
fn every_sixteen_beats_on_the_pearl_bring_a_combo_bonus() {
    let mut d = written(vec![]);
    let events = run_to(&mut d, 15);
    assert!(events.contains(&Event::Combo {
        combo: 16,
        bonus: COMBO_BONUS
    }));
    assert_eq!(d.score, 16 * START_SEGMENTS as u64 + COMBO_BONUS);
}

#[test]
fn the_new_year_parade_counts_down_to_a_great_gong() {
    let mut d = rules_dance(Mode::NewYear(2027));
    assert_eq!(d.street.finale, Finale::Midnight);
    let mut events = Vec::new();
    while !d.finished {
        d.update(1.0 / 60.0);
        let got = d.take_events();
        if got.contains(&Event::Gong { great: true }) {
            d.rear();
            events.extend(got);
            events.extend(d.take_events());
        } else {
            events.extend(got);
        }
    }
    let counts: Vec<i64> = events
        .iter()
        .filter_map(|e| match e {
            Event::Countdown { n } => Some(*n),
            _ => None,
        })
        .collect();
    assert_eq!(counts, [5, 4, 3, 2, 1]);
    let midnight = events.iter().position(|e| *e == Event::Midnight).unwrap();
    assert!(matches!(
        events[midnight + 1..].iter().find(|e| matches!(e, Event::Rear { .. })),
        Some(Event::Rear { great: true, points, .. }) if *points == 2 * GONG_PERFECT
    ));
    assert_eq!(events.last(), Some(&Event::Finished));
}

#[test]
fn the_lantern_walk_is_calm() {
    for seed in 0..10 {
        let p = plan(&streets::LANTERN_WALK, seed);
        assert_eq!(p.count(|m| matches!(m, Mark::Firecracker { .. })), 0);
        assert_eq!(p.count(|m| matches!(m, Mark::Gong { .. })), 0);
        assert_eq!(p.count(|m| matches!(m, Mark::Envelope { .. })), 0);
        assert!(p.count(|m| matches!(m, Mark::Lantern { .. })) >= 6);
    }
    let mut d = rules_dance(Mode::Lantern(2027));
    let done = autopilot(&mut d, 1.0 / 60.0, true);
    assert_eq!(done.stumbles, 0);
    assert!(!d.lanterns.is_empty());
    // A walk's lanterns are all different.
    let mut kinds = d.lanterns.clone();
    kinds.sort();
    kinds.dedup();
    assert_eq!(kinds.len(), d.lanterns.len());
}

#[test]
fn a_good_dancer_earns_three_stars_and_a_still_one_earns_none() {
    for (i, street) in STREETS.iter().enumerate() {
        let mut d = Dance::new(*street, SEED + i as u64);
        let done = autopilot(&mut d, 1.0 / 60.0, true);
        assert_eq!(done.stumbles, 0, "{}", street.name);
        assert_eq!(
            d.stars(),
            3,
            "{}: {} of {}",
            street.name,
            d.score,
            d.best_score()
        );
        assert!(d.score <= d.best_score());
    }
    // Standing at the far left edge and never rearing.
    let mut d = Dance::new(STREETS[4], SEED);
    while d.step(-1) != (Move::Edge { left: true }) {}
    while !d.finished {
        d.update(1.0 / 30.0);
        d.take_events();
    }
    assert!(d.stars() <= 1, "{} of {}", d.score, d.best_score());
}

#[test]
fn stars_follow_the_share_of_the_best_score() {
    assert_eq!(stars(0, 1000), 0);
    assert_eq!(stars(349, 1000), 0);
    assert_eq!(stars(350, 1000), 1);
    assert_eq!(stars(600, 1000), 2);
    assert_eq!(stars(800, 1000), 3);
    assert_eq!(stars(5, 0), 0);
}

#[test]
fn the_same_dance_at_any_frame_rate() {
    for mode in [
        Mode::Street(7),
        Mode::NewYear(2027),
        Mode::Challenge(parlor_core::Period::Monthly),
    ] {
        let done = testkit::same_at_frame_rates(|dt| autopilot(&mut rules_dance(mode), dt, true));
        assert!(done.score > 0);
        assert!(done.envelopes > 0);
    }
}

#[test]
fn status_reads_in_a_fixed_order() {
    let mut d = written(vec![
        at(6, Mark::Firecracker { lane: 0 }),
        at(7, Mark::Gong { great: false }),
    ]);
    assert!(
        d.status().starts_with("Lane 2 of 3, on the pearl"),
        "{}",
        d.status()
    );
    assert!(
        d.status().ends_with("dragon 6 long, combo 0, counting in."),
        "{}",
        d.status()
    );
    run_to(&mut d, 3);
    d.step(1);
    assert_eq!(
        d.status(),
        "Lane 3 of 3, pearl in lane 2, 1 lane to your left, firecrackers in lane 1 in 3 beats, gong in 4 beats. 24 points, dragon 6 long, combo 4, bar 1 of 8."
    );
}
