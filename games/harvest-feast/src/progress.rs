//! What Harvest Feast saves between runs, beside its best scores.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::pace::{self, Pace};
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::COURSES;
use crate::ID;

/// The key progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

/// The best-score board of a course.
pub fn course_board(n: u32) -> String {
    format!("course {n}")
}

/// The board a course played at `pace` records on: relaxed courses keep
/// their own, normal and quick share one.
pub fn paced_course_board(n: u32, pace: Pace) -> String {
    pace.board(&course_board(n))
}

/// The game's pace: the Pace setting, else the old relaxed course switch,
/// else normal.
pub fn pace(ctx: &Ctx<Parlor>) -> Pace {
    pace::stored(ctx, ID).unwrap_or(if Progress::peek(ctx).relaxed {
        Pace::Relaxed
    } else {
        Pace::Normal
    })
}

/// Carries saves from before the Pace setting over, each part on its own
/// so the order things happen in cannot lose one: the old relaxed course
/// boards move to the relaxed boards' names whenever one is still there,
/// keeping the higher score when both exist, and the old relaxed course
/// switch becomes the Pace setting while none is saved. Runs wherever the
/// game first reads its records: the title, Records, Course select and the
/// kitchen. Doing it again changes nothing.
pub fn migrate_pace(ctx: &mut Ctx<Parlor>) {
    let key = format!("{ID}/scores");
    let mut scores: BTreeMap<String, u64> = Records::load(ctx, &key);
    let mut moved = false;
    for n in 1..=COURSES {
        if let Some(old) = scores.remove(&format!("course {n} relaxed")) {
            let board = paced_course_board(n, Pace::Relaxed);
            let best = scores.get(&board).copied().unwrap_or(0).max(old);
            scores.insert(board, best);
            moved = true;
        }
    }
    if moved {
        Records::store(ctx, &key, &scores);
    }
    let progress = Progress::load(ctx);
    Records::load::<Option<Pace>>(ctx, &pace::key(ID));
    if pace::stored(ctx, ID).is_none() {
        let value = if progress.relaxed {
            Pace::Relaxed
        } else {
            Pace::Normal
        };
        pace::set(ctx, ID, value);
    }
}

/// The best second-helpings run.
pub const HELPINGS_BOARD: &str = "second helpings";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    /// Best stars on each course, index 0 for course 1.
    pub stars: Vec<u32>,
    /// Whether the kitchen speaks when a dish needs you.
    pub callouts: bool,
    /// The relaxed course switch from before the Pace setting, read once
    /// by [`migrate_pace`].
    pub relaxed: bool,
    /// The most second-helpings rounds served in one run.
    pub best_rounds: u32,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            stars: Vec::new(),
            callouts: true,
            relaxed: false,
            best_rounds: 0,
        }
    }
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    /// The progress as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> Progress {
        Records::peek(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Best stars on course `n`.
    pub fn stars(&self, n: u32) -> u32 {
        self.stars
            .get((n as usize).wrapping_sub(1))
            .copied()
            .unwrap_or(0)
    }

    /// Keeps the higher of `stars` and the old best. True when it improved.
    pub fn earn(&mut self, n: u32, stars: u32) -> bool {
        let i = (n.clamp(1, COURSES) - 1) as usize;
        if self.stars.len() <= i {
            self.stars.resize(i + 1, 0);
        }
        if stars > self.stars[i] {
            self.stars[i] = stars;
            true
        } else {
            false
        }
    }

    pub fn served(&self, n: u32) -> bool {
        self.stars(n) >= 1
    }

    /// A course is open once the one before it is served.
    pub fn open(&self, n: u32) -> bool {
        n <= 1 || self.served(n - 1)
    }

    /// The first course not yet served (the last when all are).
    pub fn next_course(&self) -> u32 {
        (1..=COURSES).find(|&n| !self.served(n)).unwrap_or(COURSES)
    }

    pub fn total_stars(&self) -> u32 {
        (1..=COURSES).map(|n| self.stars(n)).sum()
    }

    /// Second helpings opens once the full feast is served.
    pub fn helpings_open(&self) -> bool {
        self.served(COURSES)
    }
}
