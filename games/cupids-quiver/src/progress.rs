//! What the game remembers between visits: rooftops cleared, totals, the
//! love notes found and the keepsakes of each Valentine's Day played.

use std::collections::{BTreeMap, BTreeSet};

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::notes::{self, NOTES};
use crate::rules::{Mode, Run};
use crate::valentine;
use crate::ID;

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

/// One year's Valentine's night.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keepsake {
    #[serde(default)]
    pub night_best: Option<u64>,
    #[serde(default)]
    pub two_best: Option<u64>,
    #[serde(default)]
    pub plays: u32,
    #[serde(default)]
    pub pairs: u32,
    #[serde(default)]
    pub golden: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// The most rooftops cleared in one run.
    #[serde(default)]
    pub best_roofs: u32,
    /// Runs of every mode but practice and the challenges.
    #[serde(default)]
    pub runs: u32,
    #[serde(default)]
    pub hearts: u32,
    #[serde(default)]
    pub pairs: u32,
    #[serde(default)]
    pub roses: u32,
    #[serde(default)]
    pub golden: u32,
    /// The furthest moonlight round reached.
    #[serde(default)]
    pub moon_round: u32,
    /// Love notes found, by index into [`NOTES`], in the order found.
    #[serde(default)]
    pub notes: Vec<u8>,
    /// Each Valentine's Day played, by year.
    #[serde(default)]
    pub keepsakes: BTreeMap<i32, Keepsake>,
    /// Target call-outs off: no voice names each target as it drifts in.
    #[serde(default)]
    pub quiet: bool,
    /// Aim call-outs off: turning the bow ticks without saying the aim.
    #[serde(default)]
    pub quiet_aim: bool,
    /// The kinds of target and cue already explained in words, so play
    /// explains each only the first time.
    #[serde(default)]
    pub taught: BTreeSet<String>,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    /// The progress as last read, for a Settings label.
    pub fn peek(ctx: &Ctx<Parlor>) -> Progress {
        Records::peek(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether moonlight is open: all six rooftops cleared once.
    pub fn moonlight_open(&self) -> bool {
        self.best_roofs >= crate::rules::ROOFTOPS.len() as u32
    }

    /// Keeps the next love note, and returns its index, or `None` once
    /// every note is kept.
    pub fn find_note(&mut self) -> Option<u8> {
        let next = notes::next_unfound(&self.notes)?;
        self.notes.push(next);
        Some(next)
    }

    pub fn notes_found(&self) -> usize {
        self.notes.len().min(NOTES.len())
    }
}

/// What finishing a run changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub score: Option<ScoreResult>,
    /// Rooftops cleared this run, and whether that is a new furthest.
    pub roofs: u32,
    pub roofs_record: bool,
    /// Moonlight opened by this run.
    pub opened_moonlight: bool,
    /// The year a keepsake was kept for, on Valentine's night.
    pub keepsake: Option<i32>,
    /// Whether this was the first play of that year's night.
    pub first_of_year: bool,
}

/// Rooftops a finished run cleared.
pub fn roofs_cleared(run: &Run) -> u32 {
    if run.cleared() {
        run.round
    } else {
        run.round.saturating_sub(1)
    }
}

/// Saves a finished run: its score, totals and any keepsake.
pub fn save_end(ctx: &mut Ctx<Parlor>, run: &Run) -> Saved {
    let today = parlor_core::challenges::today(ctx);
    let mut progress = Progress::load(ctx);
    let was_open = progress.moonlight_open();
    progress.runs += 1;
    progress.hearts += run.tally.hits;
    progress.pairs += run.tally.pairs;
    progress.roses += run.tally.roses;
    progress.golden += run.tally.golden;
    let mut roofs = 0;
    let mut roofs_record = false;
    match run.mode {
        Mode::Rooftops => {
            roofs = roofs_cleared(run);
            if roofs > progress.best_roofs {
                progress.best_roofs = roofs;
                roofs_record = true;
            }
        }
        Mode::Moonlight => progress.moon_round = progress.moon_round.max(run.round),
        _ => {}
    }
    let mut keepsake = None;
    let mut first_of_year = false;
    if run.mode.is_valentine() {
        let year = valentine::keepsake_year(today);
        let k = progress.keepsakes.entry(year).or_default();
        first_of_year = k.plays == 0;
        k.plays += 1;
        k.pairs += run.tally.pairs;
        k.golden += run.tally.golden;
        let best = if run.mode == Mode::Night {
            &mut k.night_best
        } else {
            &mut k.two_best
        };
        *best = Some(best.unwrap_or(0).max(run.score));
        keepsake = Some(year);
    }
    let opened_moonlight = !was_open && progress.moonlight_open();
    progress.store(ctx);
    Saved {
        score: run
            .mode
            .board()
            .map(|board| Records::record(ctx, ID, board, run.score)),
        roofs,
        roofs_record,
        opened_moonlight,
        keepsake,
        first_of_year,
    }
}
