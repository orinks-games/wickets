//! What the game remembers between visits: ranks and pipes tuned.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::Rank;
use crate::ID;

/// The score board of the evening's tuning.
pub const RANK_BOARD: &str = "rank";

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Ranks tuned on the evening's tuning.
    #[serde(default)]
    pub ranks: u32,
    /// Pipes tuned anywhere, challenges and practice included.
    #[serde(default)]
    pub pipes: u32,
    /// Pipes set at the first try with no hint, on the evening's tuning
    /// and the challenges.
    #[serde(default)]
    pub perfect: u32,
    /// Practice ranks tuned.
    #[serde(default)]
    pub practice_ranks: u32,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has tuned a pipe at all.
    pub fn any(&self) -> bool {
        self.ranks > 0 || self.pipes > 0 || self.practice_ranks > 0
    }
}

/// Saves a tuned rank of the evening's tuning.
pub fn save_rank(ctx: &mut Ctx<Parlor>, rank: &Rank) -> ScoreResult {
    let mut p = Progress::load(ctx);
    p.ranks += 1;
    p.pipes += rank.tuned() as u32;
    p.perfect += rank.perfect() as u32;
    p.store(ctx);
    Records::record(ctx, ID, RANK_BOARD, rank.score as u64)
}

/// Adds a challenge or practice rank to the tallies.
pub fn save_other(ctx: &mut Ctx<Parlor>, rank: &Rank, practice: bool) {
    let mut p = Progress::load(ctx);
    p.pipes += rank.tuned() as u32;
    if practice {
        p.practice_ranks += 1;
    } else {
        p.perfect += rank.perfect() as u32;
    }
    p.store(ctx);
}
