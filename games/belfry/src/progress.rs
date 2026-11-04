//! What the game remembers between visits: peals solved, fewest rows and
//! the solve streak, per mode.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::{Mode, Peal};
use crate::ID;

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    #[serde(default)]
    pub played: u32,
    #[serde(default)]
    pub solved: u32,
    /// The fewest rows a peal was solved in.
    #[serde(default)]
    pub fewest: Option<u32>,
    /// Peals solved in a row, now.
    #[serde(default)]
    pub streak: u32,
    #[serde(default)]
    pub best_streak: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    pub boards: BTreeMap<String, Board>,
}

/// What finishing a peal changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Saved {
    /// The score's place on the board, when the peal was solved.
    pub score: Option<ScoreResult>,
    /// The fewest rows before this peal.
    pub previous_fewest: Option<u32>,
    /// This peal set a new fewest.
    pub fewest_record: bool,
    pub streak: u32,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    pub fn board(&self, mode: Mode) -> Board {
        self.boards.get(mode.board()).cloned().unwrap_or_default()
    }

    /// Whether any peal has been played, in any mode.
    pub fn any_played(&self) -> bool {
        self.boards.values().any(|b| b.played > 0)
    }
}

/// Saves a finished peal of `mode`: its score, fewest rows and streak.
pub fn save_end(ctx: &mut Ctx<Parlor>, mode: Mode, peal: &Peal) -> Saved {
    let mut progress = Progress::load(ctx);
    let board = progress.boards.entry(mode.board().to_string()).or_default();
    board.played += 1;
    let previous_fewest = board.fewest;
    let mut fewest_record = false;
    let score = if peal.solved() {
        board.solved += 1;
        board.streak += 1;
        board.best_streak = board.best_streak.max(board.streak);
        let rows = peal.rows_used();
        if previous_fewest.is_none_or(|f| rows < f) {
            board.fewest = Some(rows);
            fewest_record = true;
        }
        Some(peal.score())
    } else {
        board.streak = 0;
        None
    };
    let streak = board.streak;
    progress.store(ctx);
    Saved {
        score: score.map(|s| Records::record(ctx, ID, mode.board(), s)),
        previous_fewest,
        fewest_record,
        streak,
    }
}

/// Saves a peal left before it ended, after a row was rung: played, not
/// solved, and the streak starts again.
pub fn save_abandoned(ctx: &mut Ctx<Parlor>, mode: Mode) {
    let mut progress = Progress::load(ctx);
    let board = progress.boards.entry(mode.board().to_string()).or_default();
    board.played += 1;
    board.streak = 0;
    progress.store(ctx);
}
