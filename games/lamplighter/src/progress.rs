//! What the game remembers between visits: rounds, lamps and the fastest
//! hall.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::Hall;
use crate::ID;

/// The score board of the evening round.
pub const ROUND_BOARD: &str = "round";

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Evening rounds lit to the end.
    #[serde(default)]
    pub rounds: u32,
    /// The fastest evening round, in whole seconds.
    #[serde(default)]
    pub fastest: Option<u32>,
    /// Lamps lit in every hall, challenges and practice included.
    #[serde(default)]
    pub lamps: u32,
    /// Practice halls lit.
    #[serde(default)]
    pub practice_halls: u32,
}

/// What a finished evening round changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    pub score: ScoreResult,
    /// The fastest round before this one, when this one beat it.
    pub fastest_record: Option<Option<u32>>,
    pub fastest: Option<u32>,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has lit a lamp at all.
    pub fn any(&self) -> bool {
        self.rounds > 0 || self.lamps > 0 || self.practice_halls > 0
    }
}

/// Whole seconds a hall took.
pub fn secs(hall: &Hall) -> u32 {
    hall.elapsed.max(0.0).round() as u32
}

/// Saves a finished evening round.
pub fn save_round(ctx: &mut Ctx<Parlor>, hall: &Hall) -> Saved {
    let mut p = Progress::load(ctx);
    let t = secs(hall);
    let fastest_record = p.fastest.is_none_or(|f| t < f).then_some(p.fastest);
    if fastest_record.is_some() {
        p.fastest = Some(t);
    }
    p.rounds += 1;
    p.lamps += hall.lit;
    p.store(ctx);
    let score = Records::record(ctx, ID, ROUND_BOARD, hall.score as u64);
    Saved {
        score,
        fastest_record,
        fastest: p.fastest,
    }
}

/// Adds lamps lit in a challenge or a practice hall to the tally.
pub fn save_lamps(ctx: &mut Ctx<Parlor>, lamps: u32, practice_hall: bool) {
    if lamps == 0 && !practice_hall {
        return;
    }
    let mut p = Progress::load(ctx);
    p.lamps += lamps;
    if practice_hall {
        p.practice_halls += 1;
    }
    p.store(ctx);
}
