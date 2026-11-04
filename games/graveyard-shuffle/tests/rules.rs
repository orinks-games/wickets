//! The dance as plain data: judging, the beat clock, the calls and the run.

use graveyard_shuffle::play::{plans, Mode};
use graveyard_shuffle::rules::*;
use graveyard_shuffle::songs::{Monster, NIGHTS, SONGS};
use parlor_core::testkit::same_at_frame_rates;
use parlor_core::{FixedStep, Rng};

fn plan(tune: usize, set: usize) -> Plan {
    Plan::new(&SONGS[tune], &NIGHTS[set].level, &mut Rng::seeded(7))
}

/// Runs a dance for `secs` in frames of `dt`, collecting its events.
fn run_for(dance: &mut Dance, secs: f64, dt: f64) -> Vec<Ev> {
    let mut clock = FixedStep::hz(1000.0);
    let mut out = Vec::new();
    for _ in 0..(secs / dt).round() as usize {
        clock.advance(dt, |_| dance.tick(&mut out));
    }
    out
}

#[test]
fn judging_windows() {
    assert_eq!(judge(0), Some(Judgment::SpotOn));
    assert_eq!(judge(SPOT_ON_MS), Some(Judgment::SpotOn));
    assert_eq!(judge(-SPOT_ON_MS), Some(Judgment::SpotOn));
    assert_eq!(judge(SPOT_ON_MS + 1), Some(Judgment::Good));
    assert_eq!(judge(-GOOD_MS), Some(Judgment::Good));
    assert_eq!(judge(GOOD_MS + 1), Some(Judgment::Close));
    assert_eq!(judge(-WINDOW_MS), Some(Judgment::Close));
    assert_eq!(judge(WINDOW_MS + 1), None);
    assert_eq!(judge(-WINDOW_MS - 1), None);
    assert_eq!(Judgment::SpotOn.points(), 3);
    assert_eq!(Judgment::Good.points(), 2);
    assert_eq!(Judgment::Close.points(), 1);
    assert_eq!(Judgment::Missed.points(), 0);
    assert_eq!(Judgment::Wrong(Monster::Mummy).points(), 0);
}

#[test]
fn the_window_fits_between_the_fastest_beats() {
    let fastest = SONGS.iter().map(|t| t.beat_ms()).fold(f64::MAX, f64::min);
    assert!(
        2.0 * WINDOW_MS as f64 <= fastest,
        "windows of two beats overlap"
    );
}

#[test]
fn stars_from_the_share_of_points() {
    assert_eq!(stars(0, 0), 0);
    assert_eq!(stars(49, 100), 0);
    assert_eq!(stars(50, 100), 1);
    assert_eq!(stars(74, 100), 1);
    assert_eq!(stars(75, 100), 2);
    assert_eq!(stars(90, 100), 3);
    assert_eq!(stars_words(1), "1 star");
    assert_eq!(stars_words(0), "no stars");
    assert_eq!(stars_words(3), "3 stars");
}

#[test]
fn calls_keep_to_their_set() {
    for (s, set) in NIGHTS.iter().enumerate() {
        for t in set.songs {
            let p = plan(t, s);
            assert!(!p.calls.is_empty());
            let mut last = 0;
            for c in &p.calls {
                assert!(
                    set.level.monsters.contains(&c.step),
                    "{:?} in set {}",
                    c.step,
                    set.number
                );
                assert!(c.beat >= FIRST_STEP_BEAT && c.beat < TUNE_BEATS);
                assert!(c.beat > last || last == 0);
                last = c.beat;
                if set.level.offbeat == 0.0 {
                    assert_eq!(c.beat % 2, 0, "set 1 dances on the downbeat only");
                }
            }
        }
    }
}

#[test]
fn later_sets_call_more_steps() {
    let count = |s: usize| -> usize {
        (0..20)
            .map(|seed| {
                NIGHTS[s]
                    .songs
                    .iter()
                    .map(|&t| {
                        Plan::new(&SONGS[t], &NIGHTS[s].level, &mut Rng::seeded(seed))
                            .calls
                            .len()
                    })
                    .sum::<usize>()
            })
            .sum()
    };
    assert!(count(0) < count(1));
    assert!(count(1) < count(2));
    assert!(count(2) < count(3));
    assert!(NIGHTS.windows(2).all(|w| {
        let slow = w[0].songs().map(|t| t.bpm).into_iter().max().unwrap();
        let fast = w[1].songs().map(|t| t.bpm).into_iter().min().unwrap();
        slow <= fast
    }));
}

#[test]
fn calls_are_the_same_for_the_same_seed() {
    let a = plans(Mode::Set(2), 9);
    let b = plans(Mode::Set(2), 9);
    let c = plans(Mode::Set(2), 10);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn a_tune_counts_in_then_plays_its_parts_on_the_beat() {
    let p = plan(0, 0);
    let mut d = Dance::new(p, false);
    let beat = d.plan.tune.beat_ms();
    let evs = run_for(&mut d, 70.0, 1.0 / 60.0);
    let counts: Vec<u32> = evs
        .iter()
        .filter_map(|e| match e {
            Ev::Count(n) => Some(*n),
            _ => None,
        })
        .collect();
    assert_eq!(counts, [1, 2, 3, 4]);
    let parts: Vec<bool> = evs
        .iter()
        .filter_map(|e| match e {
            Ev::Part { b, .. } => Some(*b),
            _ => None,
        })
        .collect();
    assert_eq!(parts, [false, false, true, true, false, false, true, true]);
    assert_eq!(d.time_of(0), (LEAD_BEATS as f64 * beat).round() as i64);
    assert!(evs.contains(&Ev::Ended));
    assert!(d.ended());
    // Nobody danced: every call missed, none scored.
    assert_eq!(d.score(), 0);
    assert_eq!(d.tally().missed as usize, d.plan.calls.len());
}

#[test]
fn each_call_comes_one_beat_before_its_step() {
    let p = plan(3, 1);
    let mut d = Dance::new(p, false);
    let mut clock = FixedStep::hz(1000.0);
    let mut seen = 0;
    for _ in 0..(70.0 * 60.0) as usize {
        let mut out = Vec::new();
        clock.advance(1.0 / 60.0, |_| d.tick(&mut out));
        for e in out {
            if let Ev::Call { index, .. } = e {
                let c = d.plan.calls[index];
                let call_at = d.time_of(c.beat as i64 - 1);
                assert!(d.now() >= call_at && d.now() - call_at <= 17);
                seen += 1;
            }
        }
    }
    assert_eq!(seen, d.plan.calls.len());
}

/// Dances a tune at frame time `dt`: by call index, 0 lands 20
/// milliseconds late, 1 lands 90 late, 2 lands 190 early, 3 is skipped
/// and 4 is the wrong step.
fn dance_pattern(dt: f64) -> (Vec<Option<Judgment>>, u32, u32) {
    let p = plan(9, 3);
    let mut d = Dance::new(p, false);
    let mut clock = FixedStep::hz(1000.0);
    let mut pressed = vec![false; d.plan.calls.len()];
    let frames = (75.0 / dt).round() as usize;
    for _ in 0..frames {
        let mut out = Vec::new();
        clock.advance(dt, |_| d.tick(&mut out));
        for (i, c) in d.plan.calls.clone().iter().enumerate() {
            if pressed[i] {
                continue;
            }
            let beat = d.time_of(c.beat as i64);
            let (offset, step) = match i % 5 {
                0 => (20, c.step),
                1 => (90, c.step),
                2 => (-190, c.step),
                3 => continue,
                _ => (
                    0,
                    if c.step == Monster::Vampire {
                        Monster::Werewolf
                    } else {
                        Monster::Vampire
                    },
                ),
            };
            if d.now() >= beat + offset {
                pressed[i] = true;
                d.press(step);
            }
        }
    }
    let judged = (0..d.plan.calls.len()).map(|i| d.judgment(i)).collect();
    (judged, d.score(), d.best_streak)
}

#[test]
fn judging_is_the_same_at_any_frame_rate() {
    let (judged, score, _) = same_at_frame_rates(dance_pattern);
    for (i, j) in judged.iter().enumerate() {
        let want = match i % 5 {
            0 => Judgment::SpotOn,
            1 => Judgment::Good,
            2 => Judgment::Close,
            3 => Judgment::Missed,
            _ => {
                let p = plan(9, 3);
                let s = p.calls[i].step;
                Judgment::Wrong(if s == Monster::Vampire {
                    Monster::Werewolf
                } else {
                    Monster::Vampire
                })
            }
        };
        assert_eq!(*j, Some(want), "call {i}");
    }
    assert!(score > 0);
}

#[test]
fn the_whole_clock_is_the_same_at_any_frame_rate() {
    same_at_frame_rates(|dt| {
        let mut d = Dance::new(plan(5, 1), false);
        let mut clock = FixedStep::hz(1000.0);
        let mut log = Vec::new();
        for _ in 0..(50.0 / dt).round() as usize {
            let mut out = Vec::new();
            clock.advance(dt, |_| d.tick(&mut out));
            log.extend(out);
        }
        (log, d.now())
    });
}

#[test]
fn a_press_far_from_any_beat_is_a_stray() {
    let mut d = Dance::new(plan(0, 0), false);
    run_for(&mut d, 0.1, 1.0 / 60.0);
    assert_eq!(d.press(Monster::Mummy), vec![Ev::Stray(Monster::Mummy)]);
}

#[test]
fn streaks_clap_and_add_a_bonus_and_a_miss_resets_them() {
    let mut d = Dance::new(plan(9, 3), false);
    let mut clock = FixedStep::hz(1000.0);
    let mut streaks = 0;
    let n = d.plan.calls.len();
    assert!(n > 2 * STREAK_EVERY as usize);
    for i in 0..n {
        let c = d.plan.calls[i];
        let at = d.time_of(c.beat as i64);
        while d.now() < at {
            let mut out = Vec::new();
            clock.advance(0.001, |_| d.tick(&mut out));
        }
        if i == STREAK_EVERY as usize + 2 {
            continue;
        }
        for e in d.press(c.step) {
            if matches!(e, Ev::Streak { .. }) {
                streaks += 1;
            }
        }
        if i == STREAK_EVERY as usize - 1 {
            assert_eq!(d.streak, STREAK_EVERY);
            assert_eq!(d.bonus, STREAK_BONUS);
        }
    }
    assert!(streaks >= 2);
    assert_eq!(d.tally().missed, 1);
    assert_eq!(d.score(), d.points() + d.bonus);
    assert_eq!(d.points(), (n as u32 - 1) * 3);
}

#[test]
fn practice_waits_on_the_beat_for_the_right_step() {
    let mut d = Dance::new(plan(1, 0), true);
    let first = d.plan.calls[0];
    let evs = run_for(&mut d, 20.0, 1.0 / 60.0);
    assert!(evs.contains(&Ev::Wait {
        index: 0,
        step: first.step
    }));
    let held = d.now();
    assert_eq!(held, d.time_of(first.beat as i64));
    run_for(&mut d, 2.0, 1.0 / 60.0);
    assert_eq!(d.now(), held, "the clock holds while it waits");
    let wrong = if first.step == Monster::Vampire {
        Monster::Mummy
    } else {
        Monster::Vampire
    };
    let evs = d.press(wrong);
    assert!(matches!(evs[0], Ev::Wrong { .. }));
    assert_eq!(d.waiting(), Some(0));
    assert_eq!(d.judgment(0), None);
    let evs = d.press(first.step);
    assert!(evs.contains(&Ev::Go));
    assert_eq!(d.judgment(0), Some(Judgment::Waited));
    assert_eq!(d.waiting(), None);
    run_for(&mut d, 0.1, 1.0 / 60.0);
    assert!(d.now() > held);
}

#[test]
fn practice_never_misses_a_step() {
    let mut d = Dance::new(plan(1, 0), true);
    for _ in 0..3000 {
        run_for(&mut d, 0.05, 1.0 / 60.0);
        if let Some(i) = d.waiting() {
            let s = d.plan.calls[i].step;
            d.press(s);
        }
        if d.ended() {
            break;
        }
    }
    run_for(&mut d, 5.0, 1.0 / 60.0);
    assert!(d.ended());
    assert_eq!(d.tally().missed, 0);
    assert_eq!(d.tally().waited as usize, d.plan.calls.len());
}

#[test]
fn a_count_in_holds_the_tune_for_three_beats() {
    let mut d = Dance::new(plan(0, 0), false);
    run_for(&mut d, 4.0, 1.0 / 60.0);
    let before = d.now();
    d.count_in();
    assert!(d.counting_in());
    let beat = d.plan.tune.beat_ms();
    let evs = run_for(
        &mut d,
        (COUNT_IN_BEATS as f64 * beat - 50.0) / 1000.0,
        1.0 / 60.0,
    );
    assert_eq!(d.now(), before, "the tune holds through the count");
    let counts = evs.iter().filter(|e| matches!(e, Ev::Count(_))).count();
    assert_eq!(counts, COUNT_IN_BEATS as usize);
    assert_eq!(d.press(Monster::Vampire), vec![Ev::Stray(Monster::Vampire)]);
    let evs = run_for(&mut d, 0.2, 1.0 / 60.0);
    assert!(evs.contains(&Ev::Go));
    assert!(d.now() > before);
}

#[test]
fn a_count_in_waits_for_the_music() {
    let mut d = Dance::new(plan(0, 0), false);
    d.count_in();
    assert!(
        !d.counting_in(),
        "nothing to count back into before the music"
    );
}

#[test]
fn a_run_strings_its_tunes_together() {
    let mut run = Run::new(plans(Mode::Set(1), 3), false);
    assert!(!run.started());
    assert_eq!(run.update(1.0), vec![], "nothing moves before the start");
    run.start();
    let mut evs = Vec::new();
    for _ in 0..(200.0 * 60.0) as usize {
        evs.extend(run.update(1.0 / 60.0));
        if run.finished() {
            break;
        }
    }
    assert!(run.finished());
    let done: Vec<usize> = evs
        .iter()
        .filter_map(|e| match e {
            RunEv::TuneDone(i) => Some(*i),
            _ => None,
        })
        .collect();
    assert_eq!(done, [0, 1, 2]);
    let next: Vec<usize> = evs
        .iter()
        .filter_map(|e| match e {
            RunEv::Next(i) => Some(*i),
            _ => None,
        })
        .collect();
    assert_eq!(next, [1, 2]);
    assert_eq!(evs.last(), Some(&RunEv::Finished));
    assert_eq!(run.results.len(), 3);
    assert_eq!(run.score(), 0);
}
