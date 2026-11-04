//! What the game remembers between visits: games bowled, strikes in a
//! row, a perfect game, and a few tallies.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::lane::Session;
use crate::ID;

/// The score board of a full game.
pub const GAME_BOARD: &str = "game";

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Full games bowled to the tenth frame.
    #[serde(default)]
    pub games: u32,
    /// The most strikes in a row in one session.
    #[serde(default)]
    pub best_streak: u32,
    /// Strikes and spares in every session, challenges included.
    #[serde(default)]
    pub strikes: u32,
    #[serde(default)]
    pub spares: u32,
    /// A 300 game has been bowled.
    #[serde(default)]
    pub perfect: bool,
    /// The best three-game series, from the monthly challenge.
    #[serde(default)]
    pub best_series: Option<u32>,
}

/// What a finished full game changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    pub score: ScoreResult,
    /// The longest strike run before this game, when this game beat it.
    pub streak_record: Option<u32>,
    /// This was the first perfect game.
    pub first_perfect: bool,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has bowled at all.
    pub fn any(&self) -> bool {
        self.games > 0 || self.strikes > 0 || self.spares > 0
    }
}

/// Adds a session's strikes, spares and games to the tallies. Returns the
/// longest strike run before it, when the session beat it.
fn tally(p: &mut Progress, session: &Session) -> Option<u32> {
    let record = (session.best_streak > p.best_streak).then_some(p.best_streak);
    p.best_streak = p.best_streak.max(session.best_streak);
    p.strikes += session.strikes;
    p.spares += session.spares;
    p.games += session.games.len() as u32;
    record
}

/// Saves a finished full game, its score on `board`.
pub fn save_game(ctx: &mut Ctx<Parlor>, session: &Session, board: &str) -> Saved {
    let mut p = Progress::load(ctx);
    let streak_record = tally(&mut p, session);
    let total = session.games.first().map(|g| g.score()).unwrap_or(0);
    let first_perfect = total == 300 && !p.perfect;
    p.perfect |= total == 300;
    p.store(ctx);
    let score = Records::record(ctx, ID, board, total as u64);
    Saved {
        score,
        streak_record,
        first_perfect,
    }
}

/// Saves a finished challenge session's tallies, and a series total.
pub fn save_challenge(ctx: &mut Ctx<Parlor>, session: &Session) {
    let mut p = Progress::load(ctx);
    tally(&mut p, session);
    if session.games.iter().any(|g| g.score() == 300) {
        p.perfect = true;
    }
    if session.series() == 3 && session.games.len() == 3 {
        let series = session.series_total();
        p.best_series = Some(p.best_series.map_or(series, |b| b.max(series)));
    }
    p.store(ctx);
}
