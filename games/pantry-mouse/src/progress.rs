//! What the game remembers between visits: the deepest pantry reached and
//! a few tallies. The best score is a `Records` board.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::Run;
use crate::ID;

/// The score board of the pantry run.
pub const PANTRY_BOARD: &str = "pantry";

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The deepest pantry reached on a pantry run.
    #[serde(default)]
    pub deepest: u32,
    /// Pantry runs played to the end.
    #[serde(default)]
    pub runs: u32,
    /// Crumbs eaten in every run, challenges included.
    #[serde(default)]
    pub crumbs: u32,
    /// Crumbs eaten in the practice pantry.
    #[serde(default)]
    pub practice_crumbs: u32,
}

/// What a finished pantry run changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    pub score: ScoreResult,
    /// The deepest pantry before this run, when this run went deeper.
    pub deeper: Option<u32>,
    pub deepest: u32,
    /// An event pantry's run, on its own board: it leaves the pantry's
    /// records alone.
    pub event: bool,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has played at all.
    pub fn any(&self) -> bool {
        self.runs > 0 || self.crumbs > 0 || self.practice_crumbs > 0
    }
}

/// Saves a finished pantry run.
pub fn save_run(ctx: &mut Ctx<Parlor>, run: &Run) -> Saved {
    let mut p = Progress::load(ctx);
    let deeper = (run.pantry > p.deepest).then_some(p.deepest);
    p.deepest = p.deepest.max(run.pantry);
    p.runs += 1;
    p.crumbs += run.eaten;
    p.store(ctx);
    let score = Records::record(ctx, ID, PANTRY_BOARD, run.score as u64);
    Saved {
        score,
        deeper,
        deepest: p.deepest,
        event: false,
    }
}

/// Saves a finished event pantry run on its own `board`. The pantry's
/// best, deepest pantry and run count are left alone; the crumbs count
/// in the tally.
pub fn save_event_run(ctx: &mut Ctx<Parlor>, run: &Run, board: &str) -> Saved {
    let mut p = Progress::load(ctx);
    p.crumbs += run.eaten;
    p.store(ctx);
    let score = Records::record(ctx, ID, board, run.score as u64);
    Saved {
        score,
        deeper: None,
        deepest: p.deepest,
        event: true,
    }
}

/// Adds a challenge run's crumbs to the tally.
pub fn save_crumbs(ctx: &mut Ctx<Parlor>, crumbs: u32) {
    let mut p = Progress::load(ctx);
    p.crumbs += crumbs;
    p.store(ctx);
}

/// Adds crumbs eaten in the practice pantry.
pub fn save_practice(ctx: &mut Ctx<Parlor>, crumbs: u32) {
    if crumbs == 0 {
        return;
    }
    let mut p = Progress::load(ctx);
    p.practice_crumbs += crumbs;
    p.store(ctx);
}
