//! What the game remembers: stars on each pond, the Frost Fair's ribbons,
//! the lantern skates, and the tallies.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{challenges, Date, Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::events;
use crate::ponds::{pond, PONDS};
use crate::rules::{stars_for, Skate, Tally};
use crate::ID;

/// The score board of a pond: "pond 2".
pub fn pond_board(number: u32) -> String {
    format!("pond {number}")
}

/// The score board of the Frost Fair.
pub const FAIR_BOARD: &str = "frost fair";
/// The score board of the lantern skate.
pub const LANTERN_BOARD: &str = "lantern skate";

/// The pond whose first star keeps the game all year: the mill pond,
/// opened by a star on the village pond, so a player skates at least two
/// nights in winter.
pub const KEEP_POND: u32 = 2;

pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The most stars each pond has earned, by number.
    #[serde(default)]
    pub stars: BTreeMap<u32, u32>,
    /// Skates on the ponds and challenges, to the end.
    #[serde(default)]
    pub skates: u32,
    #[serde(default)]
    pub lanterns: u32,
    #[serde(default)]
    pub drifts: u32,
    #[serde(default)]
    pub cracks: u32,
    #[serde(default)]
    pub spins: u32,
    #[serde(default)]
    pub best_line: u32,
    /// Lanterns taken in practice.
    #[serde(default)]
    pub practice_lanterns: u32,
    /// The winters the Frost Fair was skated on its day, by the year of
    /// their December: one ribbon each, with a star.
    #[serde(default)]
    pub ribbons: Vec<i32>,
    /// The winters the Frost Fair was skated on its day at all, which
    /// keeps it open the rest of that winter.
    #[serde(default)]
    pub fairs: Vec<i32>,
    /// The day of the last lantern skate, as days since 1970, and the
    /// lanterns taken in order that day at best.
    #[serde(default)]
    pub lantern_day: i64,
    #[serde(default)]
    pub lantern_best_today: u32,
    /// Lantern skates whose whole tune was played.
    #[serde(default)]
    pub whole_tunes: u32,
    #[serde(default)]
    pub lantern_skates: u32,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has skated at all.
    pub fn any(&self) -> bool {
        self.skates > 0 || self.practice_lanterns > 0 || self.lantern_skates > 0
    }

    pub fn stars_for(&self, number: u32) -> u32 {
        self.stars.get(&number).copied().unwrap_or(0)
    }

    /// Whether the game is kept all year: a star on [`KEEP_POND`].
    pub fn kept(&self) -> bool {
        self.stars_for(KEEP_POND) > 0
    }

    /// Whether pond `number` is open: the first always, the rest once the
    /// pond before has a star.
    pub fn pond_open(&self, number: u32) -> bool {
        number <= 1 || self.stars_for(number - 1) > 0
    }

    /// The pond the Play row skates: the first open one with no star, or
    /// else the last open one.
    pub fn current_pond(&self) -> u32 {
        let open: Vec<u32> = PONDS
            .iter()
            .map(|p| p.number)
            .filter(|&n| self.pond_open(n))
            .collect();
        open.iter()
            .copied()
            .find(|&n| self.stars_for(n) == 0)
            .or(open.last().copied())
            .unwrap_or(1)
    }

    pub fn total_stars(&self) -> u32 {
        PONDS.iter().map(|p| self.stars_for(p.number)).sum()
    }

    /// Whether the Frost Fair is open on `today`: on its day, and the rest
    /// of a winter in which it was skated on its day.
    pub fn fair_open(&self, today: Date) -> bool {
        events::winter(today)
            && (events::fair_day(today) || self.fairs.contains(&events::winter_of(today)))
    }

    /// The lanterns taken in order on `today`'s lantern skate, at best.
    pub fn lantern_today(&self, today: Date) -> Option<u32> {
        (self.lantern_day == today.days()).then_some(self.lantern_best_today)
    }

    fn add(&mut self, t: &Tally) {
        self.skates += 1;
        self.lanterns += t.taken;
        self.drifts += t.drifts;
        self.cracks += t.cracks;
        self.spins += t.spins;
        self.best_line = self.best_line.max(t.best_line);
    }
}

/// What a finished pond changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PondSaved {
    pub score: ScoreResult,
    pub stars: u32,
    /// More stars than ever on this pond, if so, and only when it had
    /// some before: a first star is news enough.
    pub better: Option<u32>,
    /// The pond this one opened, if any.
    pub opened: Option<u32>,
    /// The pond after this one, when it is open.
    pub next_open: Option<u32>,
    /// Whether this skate kept the game all year, for the first time.
    pub kept: bool,
}

pub fn save_pond(ctx: &mut Ctx<Parlor>, number: u32, skate: &Skate) -> PondSaved {
    let mut p = Progress::load(ctx);
    let next = number + 1;
    let in_range = (next as usize) <= PONDS.len();
    let was_open = p.pond_open(next);
    let was_kept = p.kept();
    let stars = stars_for(skate.score, pond(number).marks);
    let had = p.stars_for(number);
    if stars > had {
        p.stars.insert(number, stars);
    }
    let better = (stars > had && had > 0).then_some(stars);
    p.add(&skate.tally);
    let opened = (in_range && !was_open && p.pond_open(next)).then_some(next);
    let next_open = (in_range && p.pond_open(next)).then_some(next);
    p.store(ctx);
    let score = Records::record(ctx, ID, &pond_board(number), skate.score as u64);
    PondSaved {
        score,
        stars,
        better,
        opened,
        next_open,
        kept: !was_kept && p.kept(),
    }
}

/// What a finished Frost Fair changed.
#[derive(Clone, Debug, PartialEq)]
pub struct FairSaved {
    pub score: ScoreResult,
    pub stars: u32,
    /// A ribbon earned for this winter, for the first time.
    pub ribbon: bool,
    /// Skated on the day: the fair stays open the rest of the winter.
    pub on_the_day: bool,
}

pub fn save_fair(ctx: &mut Ctx<Parlor>, skate: &Skate) -> FairSaved {
    let today = challenges::today(ctx);
    let mut p = Progress::load(ctx);
    p.add(&skate.tally);
    let stars = stars_for(skate.score, crate::ponds::FAIR_MARKS);
    let winter = events::winter_of(today);
    let on_the_day = events::fair_day(today);
    let mut ribbon = false;
    if on_the_day {
        if !p.fairs.contains(&winter) {
            p.fairs.push(winter);
        }
        if stars > 0 && !p.ribbons.contains(&winter) {
            p.ribbons.push(winter);
            ribbon = true;
        }
    }
    p.store(ctx);
    let score = Records::record(ctx, ID, FAIR_BOARD, skate.score as u64);
    FairSaved {
        score,
        stars,
        ribbon,
        on_the_day,
    }
}

/// What a finished lantern skate changed.
#[derive(Clone, Debug, PartialEq)]
pub struct LanternSaved {
    /// Lanterns taken in order today, at best, before this skate.
    pub before: Option<u32>,
    pub whole: bool,
}

pub fn save_lanterns(ctx: &mut Ctx<Parlor>, skate: &Skate) -> LanternSaved {
    let today = challenges::today(ctx);
    let mut p = Progress::load(ctx);
    let before = p.lantern_today(today);
    let taken = skate.tally.taken;
    p.lantern_day = today.days();
    p.lantern_best_today = before.unwrap_or(0).max(taken);
    p.lantern_skates += 1;
    let whole = taken == crate::ponds::LANTERN_PAIRS;
    if whole {
        p.whole_tunes += 1;
    }
    p.lanterns += taken;
    p.store(ctx);
    Records::record(ctx, ID, LANTERN_BOARD, taken as u64);
    LanternSaved { before, whole }
}

pub fn save_challenge(ctx: &mut Ctx<Parlor>, skate: &Skate) {
    let mut p = Progress::load(ctx);
    p.add(&skate.tally);
    p.store(ctx);
}

pub fn save_practice(ctx: &mut Ctx<Parlor>, taken: u32) {
    if taken == 0 {
        return;
    }
    let mut p = Progress::load(ctx);
    p.practice_lanterns += taken;
    p.store(ctx);
}
