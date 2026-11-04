//! What the game keeps between sessions, beside the best scores.

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

const KEY: &str = "sonar-dive/progress";

/// The dive the deep start begins at, and the dive that unlocks it.
pub const DEEP_UNLOCK: u32 = 4;

/// The player's progress and preferences.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    /// Speak a summary of every ping's echoes.
    #[serde(default)]
    pub readout: bool,
    /// The deepest dive reached outside practice.
    #[serde(default)]
    pub deepest: u32,
    /// Expeditions finished outside practice.
    #[serde(default)]
    pub expeditions: u32,
}

impl Saved {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Saved {
        Records::load(ctx, KEY)
    }

    /// The save as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> Saved {
        Records::peek(ctx, KEY)
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, KEY, self);
    }

    pub fn deep_unlocked(&self) -> bool {
        self.deepest >= DEEP_UNLOCK
    }
}
