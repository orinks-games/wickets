//! What Liberty Lights saves between runs, beside its best scores.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::{Launch, TOTAL_LEVELS};
use crate::ID;

/// The key progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

/// Best-score boards.
pub fn level_board(level: u32) -> String {
    format!("level {level}")
}
pub const TOTAL_BOARD: &str = "total";
pub const FINALE_BOARD: &str = "finale";
pub const MY_SHOW_BOARD: &str = "my show";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    /// The level "Start level" begins; 0 reads as 1.
    pub next_level: u32,
    /// Levels cleared at least once.
    pub cleared: Vec<u32>,
    /// The longest combo ever.
    pub best_combo: u32,
    /// The freestyle show the player saved.
    pub saved_show: Vec<Launch>,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    pub fn next_level(&self) -> u32 {
        self.next_level.clamp(1, TOTAL_LEVELS)
    }

    pub fn is_cleared(&self, level: u32) -> bool {
        self.cleared.contains(&level)
    }

    /// The grand finale opens once level 20 is cleared.
    pub fn finale_open(&self) -> bool {
        self.is_cleared(TOTAL_LEVELS)
    }

    pub fn clear(&mut self, level: u32) {
        if !self.is_cleared(level) {
            self.cleared.push(level);
            self.cleared.sort_unstable();
        }
    }
}

/// Drops the saved show's best score, once a new show takes its place.
pub fn forget_my_show_best(ctx: &mut Ctx<Parlor>) {
    let key = format!("{ID}/scores");
    let mut scores: BTreeMap<String, u64> = Records::load(ctx, &key);
    if scores.remove(MY_SHOW_BOARD).is_some() {
        Records::store(ctx, &key, &scores);
    }
}
