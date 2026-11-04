//! What the game remembers between visits: the longest run, the shortest
//! window reached, and a few tallies.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::Shift;
use crate::ID;

/// The score board of the night shift.
pub const SHIFT_BOARD: &str = "shift";

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The most trains passed in one night shift.
    #[serde(default)]
    pub best_run: u32,
    /// The shortest window reached on a night shift, in milliseconds.
    #[serde(default)]
    pub shortest_window: Option<u32>,
    /// Night shifts worked to the end.
    #[serde(default)]
    pub shifts: u32,
    /// Trains passed in every run, challenges included.
    #[serde(default)]
    pub trains: u32,
    /// Calls answered in the practice room.
    #[serde(default)]
    pub practice_calls: u32,
    /// Thirteenth trains passed on Friday the 13th night shifts: a keepsake.
    #[serde(default)]
    pub thirteenth_trains: u32,
}

/// What a finished night shift changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    pub score: ScoreResult,
    /// The best run before this one, when this one beat it.
    pub run_record: Option<u32>,
    /// This run reached a shorter window than ever before.
    pub window_record: bool,
    pub best_run: u32,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has worked the box at all.
    pub fn any(&self) -> bool {
        self.shifts > 0 || self.trains > 0 || self.practice_calls > 0
    }
}

/// Saves a finished night shift.
pub fn save_shift(ctx: &mut Ctx<Parlor>, shift: &Shift) -> Saved {
    let mut p = Progress::load(ctx);
    let run_record = (shift.passed > p.best_run).then_some(p.best_run);
    let window_record = shift.passed > 0 && p.shortest_window.is_none_or(|w| shift.window < w);
    p.best_run = p.best_run.max(shift.passed);
    if window_record {
        p.shortest_window = Some(shift.window);
    }
    p.shifts += 1;
    p.trains += shift.passed;
    p.store(ctx);
    let score = Records::record(ctx, ID, SHIFT_BOARD, shift.score as u64);
    Saved {
        score,
        run_record,
        window_record,
        best_run: p.best_run,
    }
}

/// Saves a finished steam gala shift: its score on its own board, and its
/// trains in the tally. The night shift's records stay as they are.
pub fn save_gala(ctx: &mut Ctx<Parlor>, shift: &Shift) -> ScoreResult {
    let mut p = Progress::load(ctx);
    p.trains += shift.passed;
    p.store(ctx);
    Records::record(ctx, ID, crate::events::GALA_BOARD, shift.score as u64)
}

/// Writes the keepsake: a thirteenth train seen through.
pub fn save_thirteenth(ctx: &mut Ctx<Parlor>) {
    let mut p = Progress::load(ctx);
    p.thirteenth_trains += 1;
    p.store(ctx);
}

/// Adds a challenge run's trains to the tally.
pub fn save_trains(ctx: &mut Ctx<Parlor>, trains: u32) {
    let mut p = Progress::load(ctx);
    p.trains += trains;
    p.store(ctx);
}

/// Adds calls answered in the practice room.
pub fn save_practice(ctx: &mut Ctx<Parlor>, calls: u32) {
    if calls == 0 {
        return;
    }
    let mut p = Progress::load(ctx);
    p.practice_calls += calls;
    p.store(ctx);
}
