//! What the game remembers between visits: safes opened and best times.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::{Mode, Run, ATTACK_OPENS, SAFES};
use crate::ID;

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The highest safe ever cracked, 0 for none.
    #[serde(default)]
    pub cracked: u32,
    /// Best time per safe, in tenths of a second.
    #[serde(default)]
    pub best: BTreeMap<u32, u64>,
    /// Best time attack time per safe, in tenths of a second.
    #[serde(default)]
    pub attack_best: BTreeMap<u32, u64>,
    /// Safes opened, all told.
    #[serde(default)]
    pub opened: u32,
    /// Wheels set in practice, all told.
    #[serde(default)]
    pub practice_sets: u32,
}

/// What opening a safe changed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Saved {
    /// The time beat the old best, or was the first.
    pub record: bool,
    /// The old best, in seconds.
    pub previous: Option<f64>,
    /// The safe this opened for play.
    pub opened: Option<u32>,
    /// This crack opened time attack.
    pub attack: bool,
    pub score: Option<ScoreResult>,
}

fn tenths(secs: f64) -> u64 {
    (secs.max(0.0) * 10.0).round() as u64
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Safes open in Safe select: every one cracked, and the next.
    pub fn open_safes(&self) -> u32 {
        (self.cracked + 1).min(SAFES)
    }

    /// The safe to play next: the first not yet cracked.
    pub fn next_safe(&self) -> u32 {
        self.open_safes()
    }

    pub fn attack_open(&self) -> bool {
        self.cracked >= ATTACK_OPENS
    }

    pub fn all_cracked(&self) -> bool {
        self.cracked >= SAFES
    }

    pub fn best_time(&self, safe: u32) -> Option<f64> {
        self.best.get(&safe).map(|t| *t as f64 / 10.0)
    }

    pub fn attack_time(&self, safe: u32) -> Option<f64> {
        self.attack_best.get(&safe).map(|t| *t as f64 / 10.0)
    }

    /// A safe opened in `secs`, in this mode.
    pub fn opened(&mut self, mode: Mode, secs: f64) -> Saved {
        let mut saved = Saved::default();
        let n = mode.number();
        let table = match mode {
            Mode::Safe(_) => &mut self.best,
            Mode::TimeAttack(_) => &mut self.attack_best,
            Mode::Practice | Mode::Challenge { .. } | Mode::Unlucky | Mode::Blackout => {
                return saved
            }
        };
        let t = tenths(secs);
        saved.previous = table.get(&n).map(|b| *b as f64 / 10.0);
        saved.record = table.get(&n).is_none_or(|&b| t < b);
        if saved.record {
            table.insert(n, t);
        }
        self.opened += 1;
        if matches!(mode, Mode::Safe(_)) && n > self.cracked {
            let had_attack = self.attack_open();
            self.cracked = n;
            if n < SAFES {
                saved.opened = Some(n + 1);
            }
            saved.attack = !had_attack && self.attack_open();
        }
        saved
    }
}

/// Saves the end of a safe: its time if opened, and its score.
pub fn save_end(ctx: &mut Ctx<Parlor>, run: &Run) -> Saved {
    let mut progress = Progress::load(ctx);
    let mut saved = Saved::default();
    if run.outcome.is_some_and(|o| o.won()) {
        saved = progress.opened(run.mode, run.time);
        progress.store(ctx);
        if run.score > 0 {
            saved.score = Some(Records::record(ctx, ID, &run.mode.board(), run.score));
        }
    }
    saved
}

/// Saves practice wheels as they are set.
pub fn save_practice(ctx: &mut Ctx<Parlor>, sets: u32) {
    let mut progress = Progress::load(ctx);
    progress.practice_sets += sets;
    progress.store(ctx);
}
