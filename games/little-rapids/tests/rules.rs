//! The stream's rules, without sound or speech.

use little_rapids::rules::{
    Boat, Course, Kind, Mode, Phase, Run, Stream, BEAT, COMBO_STEP, MAX_MULT, SOGGY_SINKS,
};
use parlor_core::Rng;

const DT: f64 = 0.02;

/// Where a decent player wants the boat now.
fn target(run: &Run) -> f64 {
    let pos = run.beat_pos();
    let mut soon: Vec<_> = run
        .things
        .iter()
        .filter(|t| t.beats_left(pos) > -0.1 && t.beats_left(pos) < 3.0)
        .collect();
    soon.sort_by_key(|t| t.arrive);
    if let Some(t) = soon.first() {
        let x = t.x_at(t.arrive as f64);
        return match t.kind {
            k if k.is_catch() => x,
            Kind::Gate | Kind::Falls => x,
            Kind::Mill => 1.5 * -(t.side as f64),
            Kind::Stone | Kind::Duck => {
                if x > 0.0 {
                    x - 1.6
                } else {
                    x + 1.6
                }
            }
            _ => run.current(),
        };
    }
    run.current()
}

/// Plays a run to its end, steering like a decent player.
fn sail(mut run: Run) -> Run {
    run.start_now();
    let mut guard = 0;
    while !run.is_over() {
        let want = target(&run);
        let ahead = run.x + run.vx * 0.12;
        run.steer = if want > ahead + 0.08 {
            1
        } else if want < ahead - 0.08 {
            -1
        } else {
            0
        };
        run.update(DT);
        run.take_events();
        guard += 1;
        assert!(guard < 200_000, "the run never ended");
    }
    run
}

fn idle(mut run: Run) -> Run {
    run.start_now();
    while !run.is_over() {
        run.update(DT);
        run.take_events();
    }
    run
}

#[test]
fn a_good_player_reaches_every_mouth_with_three_stars() {
    for stream in Stream::ALL {
        for boat in Boat::ALL {
            for seed in 1..=4 {
                let run = sail(Run::new(Course::stream(stream), boat, Rng::seeded(seed)));
                assert!(run.reached_mouth(), "{stream:?} {boat:?} seed {seed}");
                assert!(
                    run.catch_share() >= 0.85,
                    "{stream:?} {boat:?} seed {seed}: caught {} of {}",
                    run.caught,
                    run.offered
                );
                assert_eq!(run.stars(), 3, "{stream:?} {boat:?} seed {seed}");
            }
        }
    }
}

#[test]
fn every_challenge_can_be_sailed() {
    for mode in [Mode::Daily, Mode::Weekly, Mode::Monthly] {
        for seed in 1..=3 {
            let run = sail(Run::new(
                Course::challenge(mode),
                Boat::Paper,
                Rng::seeded(seed),
            ));
            assert!(run.reached_mouth(), "{mode:?}");
            assert!(
                run.catch_share() >= 0.8,
                "{mode:?}: {} of {}",
                run.caught,
                run.offered
            );
        }
    }
}

#[test]
fn a_boat_left_alone_still_reaches_the_mouth() {
    let run = idle(Run::new(
        Course::stream(Stream::GardenBrook),
        Boat::Paper,
        Rng::seeded(3),
    ));
    assert!(run.reached_mouth());
    assert!(run.stars() >= 1);
    assert!(
        run.catch_share() < 0.85,
        "an idle boat should not earn every star"
    );
}

#[test]
fn the_stream_runs_its_length_in_bars() {
    let run = idle(Run::new(
        Course::stream(Stream::GardenBrook),
        Boat::Paper,
        Rng::seeded(1),
    ));
    let beats = Stream::GardenBrook.bars() * 4;
    assert!(run.beat >= beats, "beat {} of {beats}", run.beat);
    assert!(run.time <= (beats as f64 + 8.0) * BEAT + 5.0);
}

#[test]
fn bumping_the_bank_sinks_an_endless_boat_on_the_fifth() {
    let mut run = Run::new(
        Course::endless(&[Stream::GardenBrook]),
        Boat::Paper,
        Rng::seeded(2),
    );
    run.start_now();
    let mut guard = 0;
    while !run.is_over() {
        run.steer = 1;
        run.update(DT);
        run.take_events();
        guard += 1;
        assert!(guard < 100_000);
    }
    assert_eq!(run.soggy, SOGGY_SINKS);
    assert!(!run.reached_mouth());
    assert_eq!(run.stars(), 0);
}

#[test]
fn bumps_cost_the_combo_but_not_a_stream_run() {
    let mut run = Run::new(
        Course::stream(Stream::GardenBrook),
        Boat::Paper,
        Rng::seeded(2),
    );
    run.start_now();
    run.combo = 12;
    for _ in 0..2000 {
        run.steer = -1;
        run.update(DT);
        run.take_events();
    }
    assert!(run.bumps > 5);
    assert_eq!(run.soggy, 0);
    assert!(!run.is_over());
}

#[test]
fn the_leaf_boat_keeps_half_its_combo() {
    assert_eq!(Boat::Leaf.after_bump(12), 6);
    assert_eq!(Boat::Paper.after_bump(12), 0);
    assert_eq!(Boat::Newspaper.after_bump(12), 0);
}

#[test]
fn the_multiplier_steps_every_ten_and_stops_at_five() {
    let mut run = Run::new(
        Course::stream(Stream::GardenBrook),
        Boat::Paper,
        Rng::seeded(1),
    );
    run.combo = COMBO_STEP - 1;
    assert_eq!(run.multiplier(), 1);
    run.combo = COMBO_STEP;
    assert_eq!(run.multiplier(), 2);
    run.combo = 500;
    assert_eq!(run.multiplier(), MAX_MULT);
}

#[test]
fn drift_scores_nothing() {
    let run = sail(Run::new(Course::drift(), Boat::Paper, Rng::seeded(5)));
    assert!(!Mode::Drift.scored());
    assert!(run.caught > 0);
}

#[test]
fn the_same_seed_floats_the_same_stream() {
    let a = Run::new(
        Course::stream(Stream::MillRace),
        Boat::Paper,
        Rng::seeded(9),
    );
    let b = Run::new(
        Course::stream(Stream::MillRace),
        Boat::Paper,
        Rng::seeded(9),
    );
    let ka: Vec<_> = a.things.iter().map(|t| (t.kind, t.arrive)).collect();
    let kb: Vec<_> = b.things.iter().map(|t| (t.kind, t.arrive)).collect();
    assert_eq!(ka, kb);
}

#[test]
fn each_stream_floats_its_own_hazard() {
    for stream in [Stream::RainGutter, Stream::MillRace, Stream::MarketCanal] {
        let mut run = Run::new(Course::stream(stream), Boat::Paper, Rng::seeded(4));
        let hazard = stream.hazard().unwrap();
        let mut seen = false;
        run.start_now();
        while !run.is_over() {
            seen |= run.things.iter().any(|t| t.kind == hazard);
            run.update(0.05);
            run.take_events();
        }
        assert!(seen, "{stream:?} never floated a {hazard:?}");
    }
}

#[test]
fn the_lead_in_waits_for_the_player() {
    let mut run = Run::new(
        Course::stream(Stream::GardenBrook),
        Boat::Paper,
        Rng::seeded(1),
    );
    run.update(1.0);
    assert!(matches!(run.phase, Phase::LeadIn(_)));
    assert!(run.start_now());
    assert!(run.is_playing());
}

#[test]
fn a_fresh_save_starts_with_the_settings_on() {
    let p = little_rapids::progress::Progress::default();
    assert_eq!(p.clean_width, 1);
    assert!(p.trinket_names);
    assert!(p.bank_lapping);
    assert!(!p.gentle);
}
