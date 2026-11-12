//! Saved progress: best scores, best times per crossing and the furthest
//! crossing reached.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Pace, Parlor, Records, ScoreResult};

use crate::rules::{Mode, Run, SKYLINE_LEN};
use crate::ID;

const TIMES: &str = "tightrope/times";
/// The board holding the furthest skyline crossing finished.
pub const FURTHEST: &str = "furthest";

/// The best time on a board ("crossing 3"), in seconds.
pub fn best_time(ctx: &mut Ctx<Parlor>, board: &str) -> Option<f64> {
    let times: BTreeMap<String, u64> = Records::load(ctx, TIMES);
    times.get(board).map(|&ms| ms as f64 / 1000.0)
}

/// What saving a time did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeResult {
    First,
    NewBest { previous: f64 },
    Short { best: f64 },
}

/// Saves a time on `board` if it beats the best (lower is better).
pub fn record_time(ctx: &mut Ctx<Parlor>, board: &str, secs: f64) -> TimeResult {
    let mut times: BTreeMap<String, u64> = Records::load(ctx, TIMES);
    let ms = (secs * 1000.0).round().max(0.0) as u64;
    let result = match times.get(board).copied() {
        None => TimeResult::First,
        Some(prev) if ms < prev => TimeResult::NewBest {
            previous: prev as f64 / 1000.0,
        },
        Some(best) => TimeResult::Short {
            best: best as f64 / 1000.0,
        },
    };
    if !matches!(result, TimeResult::Short { .. }) {
        times.insert(board.to_string(), ms);
        Records::store(ctx, TIMES, &times);
    }
    result
}

/// The furthest skyline crossing finished, 0 for none.
pub fn furthest(ctx: &mut Ctx<Parlor>) -> u32 {
    Records::best(ctx, ID, FURTHEST).unwrap_or(0) as u32
}

/// Skyline crossings open in "Single crossing": every one finished, and
/// the next.
pub fn unlocked(ctx: &mut Ctx<Parlor>) -> u32 {
    (furthest(ctx) + 1).min(SKYLINE_LEN)
}

/// The night skyline opens once the last skyline crossing is finished.
pub fn night_unlocked(ctx: &mut Ctx<Parlor>) -> bool {
    furthest(ctx) >= SKYLINE_LEN
}

/// Whether the training wire has been walked to the end.
pub fn trained(ctx: &mut Ctx<Parlor>) -> bool {
    Records::best(ctx, ID, "training").is_some()
}

/// The best times on `board` and on its relaxed board.
pub fn best_times(ctx: &mut Ctx<Parlor>, board: &str) -> (Option<f64>, Option<f64>) {
    (
        best_time(ctx, board),
        best_time(ctx, &Pace::Relaxed.board(board)),
    )
}

/// Saves what a finished run at `pace` earned, and says what it meant: one
/// line per record. Relaxed scores and times go on their own boards; the
/// furthest crossing and the practice wire count at every pace.
pub fn save_run(ctx: &mut Ctx<Parlor>, run: &Run, pace: Pace) -> Vec<String> {
    let mut lines = Vec::new();
    if let Mode::Challenge(_) = run.mode {
        return lines;
    }
    if run.mode == Mode::Training {
        if run.over() == Some(true) {
            Records::record(ctx, ID, "training", 1);
        }
        return lines;
    }
    let board = pace.board(&run.mode.board());
    let relaxed = if pace.separate() { "relaxed " } else { "" };
    match Records::record(ctx, ID, &board, run.score) {
        ScoreResult::First => lines.push(format!(
            "First {relaxed}{} score saved: {}.",
            run.mode.name(),
            run.score
        )),
        ScoreResult::NewBest { previous } => lines.push(format!(
            "New {relaxed}best {} score: {}, beating {previous}.",
            run.mode.name(),
            run.score
        )),
        ScoreResult::Short { best } => lines.push(format!(
            "Your {relaxed}best {} score stays {best}.",
            run.mode.name()
        )),
    }
    for &(number, secs) in &run.times {
        if number == 0 {
            continue;
        }
        let (board, name) = if run.mode == Mode::Night {
            (
                format!("night {number}"),
                format!("night crossing {number}"),
            )
        } else {
            (format!("crossing {number}"), format!("crossing {number}"))
        };
        match record_time(ctx, &pace.board(&board), secs) {
            TimeResult::First => {
                lines.push(format!("First {relaxed}time on {name}: {secs:.1} seconds."))
            }
            TimeResult::NewBest { previous } => lines.push(format!(
                "New {relaxed}best time on {name}: {secs:.1} seconds, beating {previous:.1}."
            )),
            TimeResult::Short { .. } => {}
        }
    }
    if run.mode != Mode::Night {
        let reached = run.times.iter().map(|&(n, _)| n).max().unwrap_or(0);
        if reached > 0 {
            let before = furthest(ctx);
            Records::record(ctx, ID, FURTHEST, reached as u64);
            if reached > before {
                lines.push(format!("Furthest crossing: {reached} of {SKYLINE_LEN}."));
                if reached < SKYLINE_LEN {
                    lines.push(format!(
                        "Crossing {} is open in Crossing select.",
                        reached + 1
                    ));
                } else if before < SKYLINE_LEN {
                    lines.push("The night skyline is open.".to_string());
                }
            }
        }
    }
    lines
}
