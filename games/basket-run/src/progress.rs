//! Saved progress: rounds cleared, what has been introduced, and the
//! callout setting. Best scores live in `Records`' score boards.

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::{Kind, ROUNDS};
use crate::ID;

const KEY: &str = "basket-run/progress";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The highest meadow round cleared.
    #[serde(default)]
    pub cleared: u32,
    /// The tutorial has been read once (it opens before the first run).
    #[serde(default)]
    pub tutorial_seen: bool,
    /// Kinds already explained the first time they fell.
    #[serde(default)]
    pub introduced: Vec<String>,
    /// Spoken lane callouts are off (play by ear for a bonus).
    #[serde(default)]
    pub quiet_calls: bool,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, KEY)
    }

    /// The progress as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> Progress {
        Records::peek(ctx, KEY)
    }

    pub fn save(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, KEY, self);
    }

    /// Endless opens once the meadow is cleared.
    pub fn endless_open(&self) -> bool {
        self.cleared >= ROUNDS
    }

    pub fn has_met(&self, kind: Kind) -> bool {
        self.introduced.iter().any(|k| k == kind.id())
    }

    pub fn meet(&mut self, kind: Kind) {
        if !self.has_met(kind) {
            self.introduced.push(kind.id().to_string());
        }
    }
}

/// "best 540, relaxed best 610", leaving out a board with no score, or
/// "no score yet".
pub fn best_line(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    parlor_core::pace::bests(ctx, ID, board, "best").unwrap_or_else(|| "no score yet".to_string())
}
