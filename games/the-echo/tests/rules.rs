//! The rank's rules, with no screen.

use parlor_core::Period;
use the_echo::challenge::Plan;
use the_echo::rules::*;

fn tune(r: &mut Rank) -> Vec<Ev> {
    let off = r.pipe().off;
    let mut evs = Vec::new();
    for _ in 0..off.abs() {
        evs.extend(r.nudge(off < 0));
    }
    evs.extend(r.set());
    evs
}

#[test]
fn a_rank_runs_low_to_high_and_starts_out_of_tune() {
    for seed in 0..50 {
        let r = Rank::new(RANK, seed);
        assert_eq!(r.pipes.len(), RANK.pipes);
        let notes: Vec<i32> = r.pipes.iter().map(|p| p.note).collect();
        let mut sorted = notes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(notes, sorted, "seed {seed}");
        for p in &r.pipes {
            assert!((RANK.near..=RANK.far).contains(&p.off.abs()), "{p:?}");
        }
        assert_eq!(r.tune.len(), TUNE_LEN);
    }
}

#[test]
fn the_same_seed_gives_the_same_rank() {
    let a = Rank::new(RANK, 9).pipes;
    assert_eq!(a, Rank::new(RANK, 9).pipes);
    assert_ne!(a, Rank::new(RANK, 10).pipes);
}

#[test]
fn a_pipe_in_tune_sounds_at_its_true_note() {
    let mut r = Rank::fixed(RANK, &[(0, 2), (12, -1)]);
    assert!(r.pipe().pitch() > r.pipe().true_pitch());
    r.nudge(false);
    r.nudge(false);
    assert_eq!(r.pipe().pitch(), r.pipe().true_pitch());
    assert!((ratio(12, 0) - 2.0).abs() < 1e-5);
}

#[test]
fn setting_too_soon_costs_and_tuning_scores() {
    let mut r = Rank::fixed(RANK, &[(0, 3), (4, -1)]);
    assert_eq!(r.set(), [Ev::NotYet { near: false }]);
    r.nudge(false);
    r.nudge(false);
    assert_eq!(r.set(), [Ev::NotYet { near: true }]);
    r.nudge(false);
    assert_eq!(
        r.set(),
        [
            Ev::InTune {
                index: 0,
                points: PIPE_POINTS - 2 * TRY_COST
            },
            Ev::Chose { index: 1 }
        ]
    );
    assert_eq!(
        r.hint(),
        [Ev::Hint {
            sharp: false,
            steps: 1
        }]
    );
    r.nudge(true);
    let evs = r.set();
    assert_eq!(
        evs[0],
        Ev::InTune {
            index: 1,
            points: PIPE_POINTS - HINT_COST
        }
    );
    assert_eq!(evs[1], Ev::RankTuned);
    assert!(r.finished());
    assert_eq!(r.score, 6 + 7);
    assert_eq!(r.perfect(), 0);
}

#[test]
fn a_pipe_never_scores_below_the_floor() {
    let mut r = Rank::fixed(RANK, &[(0, 1)]);
    for _ in 0..10 {
        r.set();
    }
    r.nudge(false);
    assert_eq!(
        r.set()[0],
        Ev::InTune {
            index: 0,
            points: PIPE_FLOOR
        }
    );
}

#[test]
fn practice_hints_are_free() {
    let mut r = Rank::fixed(PRACTICE, &[(0, 1)]);
    r.hint();
    r.hint();
    r.nudge(false);
    assert_eq!(
        r.set()[0],
        Ev::InTune {
            index: 0,
            points: PIPE_POINTS
        }
    );
}

#[test]
fn the_rank_has_ends_and_tuned_pipes_stay_put() {
    let mut r = Rank::fixed(RANK, &[(0, 1), (2, 1)]);
    assert_eq!(r.choose(false), [Ev::End { first: true }]);
    assert_eq!(r.choose(true), [Ev::Chose { index: 1 }]);
    assert_eq!(r.choose(true), [Ev::End { first: false }]);
    r.choose(false);
    tune(&mut r);
    r.choose(false);
    assert_eq!(r.nudge(true), [Ev::AlreadyTuned]);
    assert_eq!(r.set(), [Ev::AlreadyTuned]);
}

#[test]
fn setting_moves_on_to_the_next_pipe_out_of_tune() {
    let mut r = Rank::fixed(RANK, &[(0, 1), (2, 1), (4, 1)]);
    r.choose(true);
    r.choose(true);
    let evs = tune(&mut r);
    assert!(evs.contains(&Ev::Chose { index: 0 }), "{evs:?}");
}

#[test]
fn a_pipe_goes_no_further_than_the_limit() {
    let mut r = Rank::fixed(RANK, &[(0, LIMIT)]);
    assert_eq!(r.nudge(true), [Ev::Limit]);
    assert_eq!(r.nudge(false), [Ev::Moved { up: false }]);
}

#[test]
fn every_seeded_rank_can_be_tuned() {
    for seed in 0..20 {
        let mut r = Rank::new(RANK, seed);
        while !r.finished() {
            tune(&mut r);
        }
        assert_eq!(r.score, PIPE_POINTS * RANK.pipes as u32);
        assert_eq!(r.perfect(), RANK.pipes);
    }
}

#[test]
fn notes_read_by_name() {
    assert_eq!(note_name(0), "G 4");
    assert_eq!(note_name(-5), "D 4");
    assert_eq!(note_name(16), "B 5");
    assert_eq!(steps_words(1), "1 step");
}

#[test]
fn challenge_ranks_grow_through_the_periods() {
    let d = Plan::new(Period::Daily, 1).layout;
    let w = Plan::new(Period::Weekly, 1).layout;
    let m = Plan::new(Period::Monthly, 1).layout;
    assert!(d.pipes < w.pipes && w.pipes < m.pipes);
    assert_eq!(m.pipes, NOTES.len());
    assert_eq!(
        Plan::new(Period::Weekly, 4).rank().pipes,
        Plan::new(Period::Weekly, 4).rank().pipes
    );
}

#[test]
fn the_tuned_line_says_none_for_no_first_tries() {
    use the_echo::play::tuned_line;
    let mut r = Rank::new(RANK, 7);
    for p in r.pipes.iter_mut() {
        p.tuned = true;
        p.tries = 1;
    }
    let n = r.pipes.len();
    assert_eq!(
        tuned_line(&r),
        format!("Rank tuned: {n} pipes, none at the first try without a hint.")
    );
    r.pipes[0].tries = 0;
    assert_eq!(
        tuned_line(&r),
        format!("Rank tuned: {n} pipes, 1 at the first try without a hint.")
    );
}

#[test]
fn a_hint_on_a_pipe_already_in_tune_is_free() {
    let mut r = Rank::fixed(RANK, &[(0, 1)]);
    r.hint();
    r.nudge(false);
    assert_eq!(r.hint(), [Ev::HintInTune]);
    assert_eq!(
        r.set()[0],
        Ev::InTune {
            index: 0,
            points: PIPE_POINTS - HINT_COST
        }
    );
}
