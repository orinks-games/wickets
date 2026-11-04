//! What the game remembers between runs: how far each part is open, and
//! whether the story has been finished.

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::gifts::SHIFTS;
use crate::night::NIGHTS;
use crate::ID;

/// Best-score boards.
pub const GIFT_BOARD: &str = "gift-sort";
pub const COUNT_BOARD: &str = "countdown";
pub const STORY_BOARD: &str = "story";
pub const BLIZZARD_BOARD: &str = "blizzard";

/// The board for one night of Midnight Countdown.
pub fn night_board(night: u32) -> String {
    format!("countdown night {night}")
}

const KEY: &str = "winter-wonderland/progress";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The highest Gift Sort shift opened (0 or 1: only the first).
    #[serde(default)]
    pub shift: u32,
    /// The highest Midnight Countdown night opened.
    #[serde(default)]
    pub night: u32,
    /// The story has been finished once: the blizzard story is open.
    #[serde(default)]
    pub story: bool,
    /// Every Gift Sort shift has been cleared in one run.
    #[serde(default)]
    pub all_gifts: bool,
    /// Every night has reached midnight.
    #[serde(default)]
    pub all_nights: bool,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, KEY)
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, KEY, self);
    }

    /// Shifts the player may start from: 1 to this.
    pub fn shifts_open(&self) -> u32 {
        self.shift.clamp(1, SHIFTS)
    }

    /// Nights the player may choose: 1 to this.
    pub fn nights_open(&self) -> u32 {
        self.night.clamp(1, NIGHTS)
    }

    /// Opens shift `level` (when it exists). True when it was newly opened.
    pub fn open_shift(ctx: &mut Ctx<Parlor>, level: u32) -> bool {
        let mut p = Progress::load(ctx);
        let level = level.min(SHIFTS);
        if level > p.shifts_open() {
            p.shift = level;
            p.store(ctx);
            return true;
        }
        false
    }

    /// Opens night `level` (when it exists). True when it was newly opened.
    pub fn open_night(ctx: &mut Ctx<Parlor>, level: u32) -> bool {
        let mut p = Progress::load(ctx);
        let level = level.min(NIGHTS);
        if level > p.nights_open() {
            p.night = level;
            p.store(ctx);
            return true;
        }
        false
    }

    /// Marks something done; true when it was not already.
    pub fn mark(ctx: &mut Ctx<Parlor>, set: impl Fn(&mut Progress) -> &mut bool) -> bool {
        let mut p = Progress::load(ctx);
        let flag = set(&mut p);
        if *flag {
            return false;
        }
        *flag = true;
        p.store(ctx);
        true
    }
}

/// Records `score` on `board` and says how it went, for results pages.
pub fn record(ctx: &mut Ctx<Parlor>, board: &str, score: u64) -> String {
    match Records::record(ctx, ID, board, score) {
        ScoreResult::First => "Your first score here is saved.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best. The old best was {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// The best on `board`, for menu labels.
pub fn best_text(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    match Records::best(ctx, ID, board) {
        Some(best) => format!("Best {best}"),
        None => "Not played yet".to_string(),
    }
}
