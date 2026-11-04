//! The rules on their own: the bow, the sky, scoring and rounds.

use crate::common::*;
use cupids_quiver::field::meet_angle;
use cupids_quiver::notes::{next_unfound, NOTES};
use cupids_quiver::progress::Progress;
use cupids_quiver::rules::{
    aim_words, degrees_words, offset_words, turn_words, Colour, Event, Kind, Mode, Phase, Plan,
    Range, Release, Run, Target, Twist, AIM_LIMIT, ROSE_BONUS, SWEETHEART_BONUS,
};
use parlor_core::testkit::same_at_frame_rates;
use parlor_core::Rng;

fn heart(id: u32, colour: Colour, angle: f64, speed: f64, range: Range, pair: u32) -> Target {
    Target {
        id,
        kind: Kind::Heart(colour),
        angle,
        speed,
        range,
        pair: Some(pair),
    }
}

fn hits(events: &[Event]) -> Vec<(u32, u64, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Hit {
                id,
                points,
                sweetheart,
                ..
            } => Some((*id, *points, *sweetheart)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_bow_turns_in_steps_and_stops_at_its_limit() {
    let mut run = quiet_run(Mode::Rooftops);
    assert!(run.turn(true));
    assert_eq!(run.aim, 5.0);
    for _ in 0..40 {
        run.turn(true);
    }
    assert_eq!(run.aim, AIM_LIMIT);
    assert!(!run.turn(true));
    run.centre();
    assert_eq!(run.aim, 0.0);
    assert!(run.turn(false));
    assert_eq!(aim_words(run.aim), "5 degrees left");
    assert_eq!(aim_words(0.0), "straight ahead");
    assert_eq!(offset_words(0.0, 20.0), "20 degrees right of your aim");
    assert_eq!(offset_words(10.0, 9.0), "1 degree left of your aim");
    assert_eq!(aim_words(1.0), "1 degree right");
    assert_eq!(aim_words(-1.0), "1 degree left");
    assert_eq!(degrees_words(1), "1 degree");
    assert_eq!(degrees_words(2), "2 degrees");
    assert_eq!(turn_words(10.0, -25.0), "turn 35 left");
    assert_eq!(turn_words(5.0, 5.0), "hold your aim");
}

#[test]
fn letting_go_early_keeps_the_arrow() {
    let mut run = quiet_run(Mode::Rooftops);
    let arrows = run.arrows_left;
    run.begin_draw().unwrap();
    for _ in 0..24 {
        run.update(1.0 / 60.0);
    }
    match run.release() {
        Release::TooSoft(d) => assert!((d - 0.5).abs() < 0.02, "{d}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(run.arrows_left, arrows);
    assert_eq!(run.release(), Release::NotDrawing);
    // A full draw clicks once, and shoots.
    run.begin_draw().unwrap();
    let mut full = 0;
    for _ in 0..80 {
        full += run
            .update(1.0 / 60.0)
            .iter()
            .filter(|e| **e == Event::FullDraw)
            .count();
    }
    assert_eq!(full, 1);
    assert_eq!(run.release(), Release::Shot);
    assert_eq!(run.arrows_left, arrows - 1);
    assert!(!run.cancel_draw());
}

#[test]
fn an_arrow_meets_a_heart_in_line_and_scores_by_range() {
    for (range, points) in [(Range::Near, 10), (Range::Middle, 20), (Range::Far, 30)] {
        let mut run = quiet_run(Mode::Rooftops);
        full_draw(&mut run);
        target(&mut run, heart(900, Colour::Pink, 0.0, 0.0, range, 900));
        assert_eq!(run.in_sights(), Some(900));
        assert_eq!(run.release(), Release::Shot);
        let events = fly(&mut run);
        assert_eq!(hits(&events), [(900, points, false)], "{range:?}");
        assert!(run.sky.is_empty());
    }
}

#[test]
fn a_drifting_heart_needs_leading() {
    // A far heart drifting right at 20 degrees a second moves 14 degrees
    // while the arrow flies.
    let mut run = quiet_run(Mode::Rooftops);
    full_draw(&mut run);
    target(
        &mut run,
        heart(900, Colour::Red, 0.0, 20.0, Range::Far, 900),
    );
    assert_eq!(
        run.in_sights(),
        None,
        "aimed where it is, not where it will be"
    );
    run.release();
    let events = fly(&mut run);
    assert!(events.iter().any(|e| matches!(e, Event::Miss { .. })));
    assert!(hits(&events).is_empty());

    let mut run = quiet_run(Mode::Rooftops);
    full_draw(&mut run);
    let t = heart(900, Colour::Red, 0.0, 20.0, Range::Far, 900);
    let meet = meet_angle(&t);
    assert_eq!(meet, 15.0);
    target(&mut run, t);
    while run.aim < meet {
        run.turn(true);
    }
    assert_eq!(run.in_sights(), Some(900));
    run.release();
    assert_eq!(hits(&fly(&mut run)).len(), 1);
}

#[test]
fn the_nearest_range_in_line_is_hit_first() {
    let mut run = quiet_run(Mode::Rooftops);
    full_draw(&mut run);
    target(&mut run, heart(901, Colour::Red, 0.0, 0.0, Range::Far, 901));
    target(
        &mut run,
        heart(902, Colour::Pink, 2.0, 0.0, Range::Near, 902),
    );
    run.release();
    let h = hits(&fly(&mut run));
    assert_eq!(h, [(902, 10, false)]);
    assert_eq!(run.sky.len(), 1);
}

#[test]
fn both_hearts_of_a_pair_in_a_row_make_a_sweetheart_pair() {
    let mut run = quiet_run(Mode::Rooftops);
    full_draw(&mut run);
    target(
        &mut run,
        heart(901, Colour::Violet, 0.0, 0.0, Range::Near, 7),
    );
    target(
        &mut run,
        heart(902, Colour::Violet, 40.0, 0.0, Range::Near, 7),
    );
    run.release();
    assert_eq!(hits(&fly(&mut run)), [(901, 10, false)]);
    assert_eq!(run.last_pair, Some(7));
    assert_eq!(run.sweetheart_target().map(|t| t.id), Some(902));
    full_draw(&mut run);
    while run.aim < 40.0 {
        run.turn(true);
    }
    run.release();
    // 10 for the range, 5 for a second hit in a row, and the pair bonus.
    assert_eq!(
        hits(&fly(&mut run)),
        [(902, 10 + 5 + SWEETHEART_BONUS, true)]
    );
    assert_eq!(run.tally.pairs, 1);
    assert_eq!(run.last_pair, None);
}

#[test]
fn a_miss_between_breaks_the_pair() {
    let mut run = quiet_run(Mode::Rooftops);
    full_draw(&mut run);
    target(
        &mut run,
        heart(901, Colour::Violet, 0.0, 0.0, Range::Near, 7),
    );
    target(
        &mut run,
        heart(902, Colour::Violet, 40.0, 0.0, Range::Near, 7),
    );
    run.release();
    fly(&mut run);
    // A shot at the empty sky to the left.
    full_draw(&mut run);
    for _ in 0..8 {
        run.turn(false);
    }
    run.release();
    assert!(fly(&mut run)
        .iter()
        .any(|e| matches!(e, Event::Miss { .. })));
    assert_eq!(run.last_pair, None);
    full_draw(&mut run);
    while run.aim < 40.0 {
        run.turn(true);
    }
    run.release();
    assert_eq!(hits(&fly(&mut run)), [(902, 10, false)]);
}

#[test]
fn roses_golden_hearts_and_notes_score_their_own_way() {
    for (kind, points) in [
        (Kind::Rose, 20 + ROSE_BONUS),
        (Kind::Golden, 100),
        (Kind::Letter, 40),
    ] {
        let mut run = quiet_run(Mode::Rooftops);
        full_draw(&mut run);
        target(
            &mut run,
            Target {
                id: 900,
                kind,
                angle: 0.0,
                speed: 0.0,
                range: Range::Middle,
                pair: None,
            },
        );
        run.release();
        assert_eq!(hits(&fly(&mut run)), [(900, points, false)], "{kind:?}");
    }
}

#[test]
fn hearts_drift_in_as_pairs_from_either_edge_and_drift_away() {
    let mut run = Run::new(Mode::Rooftops, Rng::seeded(SEED));
    let mut spawned = Vec::new();
    let mut gone = 0;
    for _ in 0..60 * 40 {
        for e in run.update(1.0 / 60.0) {
            match e {
                Event::Spawn { kind, partner, .. } => spawned.push((kind, partner)),
                Event::Gone { .. } => gone += 1,
                _ => {}
            }
        }
    }
    assert!(spawned.len() >= 4, "{spawned:?}");
    assert!(gone >= 2, "nothing drifted away");
    // Each second heart follows a first of its colour.
    let mut seconds = 0;
    for (i, (kind, partner)) in spawned.iter().enumerate() {
        if *partner {
            seconds += 1;
            assert!(
                spawned[..i].iter().any(|(k, p)| k == kind && !p),
                "{spawned:?}"
            );
        }
    }
    assert!(seconds >= 2, "{spawned:?}");
    assert!(run.sky.iter().all(|t| t.angle.abs() <= 81.0));
    assert!(
        !spawned.iter().any(|(k, _)| *k == Kind::Rose),
        "no roses out of season"
    );
}

#[test]
fn a_round_ends_when_the_quiver_is_spent() {
    let mut run = Run::new(Mode::Rooftops, Rng::seeded(SEED));
    let arrows = run.arrows_left;
    let mut over = None;
    for _ in 0..arrows {
        assert_eq!(draw_and_shoot(&mut run), Release::Shot);
        for e in fly(&mut run) {
            if let Event::RoundOver { more, .. } = e {
                over = Some(more);
            }
        }
    }
    assert_eq!(run.arrows_left, 0);
    let more = over.expect("the round ended");
    // Straight-ahead shooting rarely makes the goal: the evening ends.
    assert_eq!(more, run.round_hits >= run.plan.goal);
    assert!(run.sky.is_empty());
    if !more {
        assert_eq!(run.phase, Phase::Over);
        assert!(!run.cleared());
        assert_eq!(
            run.begin_draw(),
            Err(cupids_quiver::rules::Refusal::NotPlaying)
        );
    }
}

/// A careful archer: turns to where the best target will be met, draws,
/// and lets go when a target is in the sights.
fn bot(run: &mut Run) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..60 * 600 {
        if run.phase == Phase::Between {
            run.next_round();
        }
        if run.phase == Phase::Over {
            break;
        }
        let want = run
            .sky
            .iter()
            .filter(|t| meet_angle(t).abs() < AIM_LIMIT - 5.0)
            .map(|t| (meet_angle(t), t.range))
            .min_by(|a, b| {
                a.1.cmp(&b.1)
                    .then((a.0 - run.aim).abs().total_cmp(&(b.0 - run.aim).abs()))
            })
            .map(|(a, _)| a);
        if let Some(want) = want {
            if want > run.aim {
                run.turn(true);
            } else if want < run.aim {
                run.turn(false);
            }
        }
        if !run.drawing() && run.flying.is_empty() {
            let _ = run.begin_draw();
        }
        if run.draw == Some(1.0) && run.in_sights().is_some() {
            run.release();
        }
        events.extend(run.update(1.0 / 60.0));
    }
    events
}

#[test]
fn a_careful_archer_clears_the_rooftops() {
    let mut cleared = 0;
    for seed in 1..=6 {
        let mut run = Run::new(Mode::Rooftops, Rng::seeded(seed));
        bot(&mut run);
        assert_eq!(run.phase, Phase::Over);
        if run.cleared() {
            cleared += 1;
            assert_eq!(run.round, 6);
        }
        assert!(run.tally.hits > 10, "seed {seed}: {:?}", run.tally);
    }
    assert!(cleared >= 3, "only {cleared} of 6 evenings cleared");
}

#[test]
fn rounds_grow_busier() {
    let plans: Vec<Plan> = (1..=6)
        .map(|r| Plan::new(Mode::Rooftops, r, false, None))
        .collect();
    for w in plans.windows(2) {
        assert!(w[1].arrows >= w[0].arrows);
        assert!(w[1].live >= w[0].live);
        assert!(w[1].speed_hi > w[0].speed_hi);
        assert!(w[1].tolerance < w[0].tolerance);
    }
    for p in &plans {
        assert!(p.goal > 0 && p.goal < p.arrows, "{p:?}");
    }
    assert!(Plan::new(Mode::Moonlight, 1, false, None).speed_hi > plans[2].speed_hi);
    assert_eq!(Plan::new(Mode::Night, 1, false, None).goal, 0);
    assert!(Plan::new(Mode::Night, 1, false, None).golden > 0.0);
    assert_eq!(Plan::new(Mode::Rooftops, 1, false, None).golden, 0.0);
}

#[test]
fn roses_drift_only_in_season_and_never_in_the_daily() {
    assert!(Plan::new(Mode::Rooftops, 1, true, None).roses);
    assert!(!Plan::new(Mode::Rooftops, 1, false, None).roses);
    assert!(!Plan::new(Mode::Daily, 1, true, None).roses);
    assert!(Plan::new(Mode::Monthly, 1, false, None).roses);
    let mut run = Run::new(Mode::Rooftops, Rng::seeded(SEED));
    run.set_roses(true);
    let mut roses = 0;
    for _ in 0..60 * 120 {
        for e in run.update(1.0 / 60.0) {
            if let Event::Spawn {
                kind: Kind::Rose, ..
            } = e
            {
                roses += 1;
            }
        }
    }
    assert!(roses > 0);
}

#[test]
fn the_practice_field_never_ends_and_holds_one_heart() {
    let mut run = Run::new(Mode::Practice, Rng::seeded(SEED));
    for _ in 0..30 {
        assert_eq!(draw_and_shoot(&mut run), Release::Shot);
        fly(&mut run);
        assert!(run.sky.len() <= 1, "{:?}", run.sky);
        assert!(run.sky.iter().all(|t| t.range != Range::Far));
    }
    assert_eq!(run.phase, Phase::Playing);
    assert_eq!(run.tally.shots, 30);
}

#[test]
fn partners_take_turns_and_share_the_score() {
    let mut run = Run::new(Mode::ForTwo, Rng::seeded(SEED));
    assert_eq!(
        run.arrows_left,
        Plan::new(Mode::Night, 1, false, None).arrows * 2
    );
    let seats = |run: &Run| run.seats.as_ref().unwrap().current();
    assert_eq!(seats(&run), 0);
    full_draw(&mut run);
    target(
        &mut run,
        heart(900, Colour::Red, 0.0, 0.0, Range::Near, 900),
    );
    run.release();
    assert_eq!(seats(&run), 1);
    let h = fly(&mut run);
    assert!(h.iter().any(|e| matches!(e, Event::Hit { shooter: 0, .. })));
    draw_and_shoot(&mut run);
    assert_eq!(seats(&run), 0);
    assert_eq!(run.tally.by_seat[0], 1);
}

#[test]
fn challenges_play_every_round() {
    for (mode, rounds) in [(Mode::Daily, 1), (Mode::Weekly, 3), (Mode::Monthly, 5)] {
        let mut run = Run::with_twist(mode, Some(Twist::Crowded), Rng::seeded(SEED));
        let mut played = 1;
        for _ in 0..200 {
            if run.phase == Phase::Over {
                break;
            }
            if run.phase == Phase::Between {
                run.next_round();
                played += 1;
            }
            draw_and_shoot(&mut run);
            fly(&mut run);
        }
        assert_eq!(run.phase, Phase::Over, "{mode:?}");
        assert_eq!(played, rounds, "{mode:?}");
    }
}

#[test]
fn twists_change_the_plan() {
    let base = Plan::new(Mode::Daily, 1, false, None);
    assert!(Plan::new(Mode::Daily, 1, false, Some(Twist::Breeze)).speed_hi > base.speed_hi);
    assert!(Plan::new(Mode::Daily, 1, false, Some(Twist::FarSky)).only_far);
    assert_eq!(
        Plan::new(Mode::Daily, 1, false, Some(Twist::Crowded)).live,
        base.live + 2
    );
    assert_eq!(
        Plan::new(Mode::Daily, 1, false, Some(Twist::Sweethearts)).bonus,
        2 * SWEETHEART_BONUS
    );
    assert_eq!(base.arrows, 15);
}

#[test]
fn the_sky_plays_the_same_at_any_frame_rate() {
    let result = same_at_frame_rates(|dt| {
        let mut run = Run::new(Mode::Rooftops, Rng::seeded(SEED));
        let frames = (30.0 / dt).round() as usize;
        let per_second = (1.0 / dt).round() as usize;
        let mut log = Vec::new();
        for i in 0..frames {
            // Inputs land on whole seconds, which every frame rate reaches.
            if i % per_second == 0 {
                let second = i / per_second;
                match second % 3 {
                    0 => {
                        let _ = run.begin_draw();
                    }
                    1 => {
                        run.release();
                    }
                    _ => {
                        run.turn(second.is_multiple_of(2));
                    }
                }
            }
            for e in run.update(dt) {
                log.push(format!("{e:?}"));
            }
        }
        (run.score, run.tally.clone(), log)
    });
    assert!(result.1.shots > 0);
}

#[test]
fn love_notes_are_found_in_order_and_run_out() {
    let mut p = Progress::default();
    assert_eq!(next_unfound(&p.notes), Some(0));
    for i in 0..NOTES.len() {
        assert_eq!(p.find_note(), Some(i as u8));
    }
    assert_eq!(p.find_note(), None);
    assert_eq!(p.notes_found(), NOTES.len());
}
