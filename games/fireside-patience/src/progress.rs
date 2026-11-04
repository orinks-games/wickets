//! What the game remembers between visits: deals started and won, and the
//! fewest moves a relaxed deal was won in.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::points;
use crate::ID;

/// The relaxed score board: 1000 points less one a move.
pub const RELAXED_BOARD: &str = "relaxed";

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Relaxed deals started.
    #[serde(default)]
    pub deals: u32,
    /// Relaxed deals won.
    #[serde(default)]
    pub wins: u32,
    /// The fewest moves a relaxed deal was won in.
    #[serde(default)]
    pub fewest: Option<u32>,
}

/// What winning a relaxed deal changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved {
    pub score: ScoreResult,
    pub wins: u32,
    /// The fewest moves before this win.
    pub previous_fewest: Option<u32>,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }
}

/// Notes a relaxed deal started.
pub fn note_deal(ctx: &mut Ctx<Parlor>) {
    let mut p = Progress::load(ctx);
    p.deals += 1;
    p.store(ctx);
}

/// Saves a won relaxed deal.
pub fn save_win(ctx: &mut Ctx<Parlor>, moves: u32) -> Saved {
    let mut p = Progress::load(ctx);
    p.wins += 1;
    let previous_fewest = p.fewest;
    if previous_fewest.is_none_or(|f| moves < f) {
        p.fewest = Some(moves);
    }
    p.store(ctx);
    let score = Records::record(ctx, ID, RELAXED_BOARD, points(moves));
    Saved {
        score,
        wins: p.wins,
        previous_fewest,
    }
}
