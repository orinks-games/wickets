//! What the game remembers between visits: the deepest pantry reached and
//! a few tallies. The best score is a `Records` board.

use hearken::Ctx;
use parlor_core::{Pace, Parlor, Records, ScoreResult};
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
    /// What saving the score meant: `None` for a 0, which goes on no board.
    pub score: Option<ScoreResult>,
    /// The deepest pantry before this run, when this run went deeper.
    pub deeper: Option<u32>,
    pub deepest: u32,
    /// An event pantry's run, on its own board: it leaves the pantry's
    /// records alone.
    pub event: bool,
    /// The pace played: a relaxed run keeps its own best score.
    pub pace: Pace,
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

/// Saves a finished pantry run played at `pace`. A relaxed run keeps its
/// own best score; the deepest pantry counts at every pace.
pub fn save_run(ctx: &mut Ctx<Parlor>, run: &Run, pace: Pace) -> Saved {
    let mut p = Progress::load(ctx);
    let deeper = (run.pantry > p.deepest).then_some(p.deepest);
    p.deepest = p.deepest.max(run.pantry);
    p.runs += 1;
    p.crumbs += run.eaten;
    p.store(ctx);
    let score = parlor_core::pace::record(ctx, ID, &pace.board(PANTRY_BOARD), run.score as u64);
    Saved {
        score,
        deeper,
        deepest: p.deepest,
        event: false,
        pace,
    }
}

/// Saves a finished event pantry run on its own `board`. The pantry's
/// best, deepest pantry and run count are left alone; the crumbs count
/// in the tally. A relaxed run keeps its own best score.
pub fn save_event_run(ctx: &mut Ctx<Parlor>, run: &Run, board: &str, pace: Pace) -> Saved {
    let mut p = Progress::load(ctx);
    p.crumbs += run.eaten;
    p.store(ctx);
    let score = parlor_core::pace::record(ctx, ID, &pace.board(board), run.score as u64);
    Saved {
        score,
        deeper: None,
        deepest: p.deepest,
        event: true,
        pace,
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
