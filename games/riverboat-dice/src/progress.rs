//! What the game remembers between visits: nights played and won, the
//! best night, and the longest anyone stayed in.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::Night;
use crate::ID;

/// The saloon's score board.
pub const SALOON_BOARD: &str = "saloon";

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Saloon nights played to the end.
    #[serde(default)]
    pub nights: u32,
    /// Saloon nights won, shared wins included.
    #[serde(default)]
    pub wins: u32,
    /// The most rolls anyone stayed seated through in one landing, in any
    /// night, challenges included.
    #[serde(default)]
    pub longest_stay: u32,
}

/// What finishing a saloon night changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved {
    pub score: ScoreResult,
    pub won: bool,
    pub wins: u32,
    /// The night set a new longest stay.
    pub longest_record: bool,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// The best saloon night, if any.
    pub fn best(ctx: &mut Ctx<Parlor>) -> Option<u64> {
        Records::best(ctx, ID, SALOON_BOARD)
    }
}

/// Notes the night's longest stay, for any night. Returns whether it beat
/// the record.
pub fn note_stay(ctx: &mut Ctx<Parlor>, night: &Night) -> bool {
    let mut progress = Progress::load(ctx);
    if night.longest_stay > progress.longest_stay {
        progress.longest_stay = night.longest_stay;
        progress.store(ctx);
        true
    } else {
        false
    }
}

/// Saves a finished saloon night.
pub fn save_night(ctx: &mut Ctx<Parlor>, night: &Night) -> Saved {
    let longest_record = note_stay(ctx, night);
    let mut progress = Progress::load(ctx);
    progress.nights += 1;
    let won = night.winners().contains(&crate::rules::YOU);
    if won {
        progress.wins += 1;
    }
    progress.store(ctx);
    let score = Records::record(ctx, ID, SALOON_BOARD, night.you().night as u64);
    Saved {
        score,
        won,
        wins: progress.wins,
        longest_record,
    }
}
