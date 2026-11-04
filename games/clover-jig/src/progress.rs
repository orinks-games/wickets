//! What the game remembers: stars for each tune, the sets opened, the
//! tallies, the luck from shamrock weeks and the keepsake of each ceili.

use std::collections::{BTreeMap, BTreeSet};

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::{Run, Tally};
use crate::tunes::SETS;
use crate::ID;

/// The score board of ceili night.
pub const CEILI_BOARD: &str = "ceili";

/// The score board of a set: "set 2".
pub fn set_board(number: u32) -> String {
    format!("set {number}")
}

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

/// One year's ceili night.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keepsake {
    /// Set dances finished that year.
    #[serde(default)]
    pub nights: u32,
    #[serde(default)]
    pub best: u32,
    #[serde(default)]
    pub clover: bool,
    /// Luck points from shamrocks caught at the ceili.
    #[serde(default)]
    pub luck: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The most stars each tune has earned, by tune id.
    #[serde(default)]
    pub stars: BTreeMap<String, u32>,
    /// Sets danced to the end.
    #[serde(default)]
    pub sets_danced: u32,
    /// Tunes danced to the end, every mode.
    #[serde(default)]
    pub tunes_danced: u32,
    /// Steps danced in time, every mode.
    #[serde(default)]
    pub steps: u32,
    #[serde(default)]
    pub spot_on: u32,
    /// The longest run of steps in time.
    #[serde(default)]
    pub best_streak: u32,
    /// Steps danced in the practice room.
    #[serde(default)]
    pub practice_steps: u32,
    /// Luck points, from shamrocks caught.
    #[serde(default)]
    pub luck: u32,
    /// Shamrocks heard, caught or not.
    #[serde(default)]
    pub shamrocks_heard: u32,
    /// Four-leaf clovers found.
    #[serde(default)]
    pub clovers: u32,
    /// The nights the clover has come out, by date: once a night.
    #[serde(default)]
    pub clover_nights: BTreeSet<String>,
    /// Each year's ceili night.
    #[serde(default)]
    pub keepsakes: BTreeMap<i32, Keepsake>,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has danced at all.
    pub fn any(&self) -> bool {
        self.tunes_danced > 0 || self.practice_steps > 0 || self.steps > 0
    }

    pub fn stars_for(&self, tune: &str) -> u32 {
        self.stars.get(tune).copied().unwrap_or(0)
    }

    /// Whether every tune of set `number` has a star.
    pub fn set_cleared(&self, number: u32) -> bool {
        SETS.iter()
            .find(|s| s.number == number)
            .is_some_and(|s| s.tunes().iter().all(|t| self.stars_for(t.id) > 0))
    }

    /// Whether set `number` is open: set 1 always, the rest once the set
    /// before is cleared.
    pub fn set_open(&self, number: u32) -> bool {
        number <= 1 || self.set_cleared(number - 1)
    }

    /// The set the Play row dances: the first not cleared, or the last.
    pub fn current_set(&self) -> u32 {
        SETS.iter()
            .map(|s| s.number)
            .find(|&n| !self.set_cleared(n))
            .unwrap_or(SETS.len() as u32)
    }

    /// Stars over every tune of the sets, out of 36.
    pub fn total_stars(&self) -> u32 {
        SETS.iter()
            .flat_map(|s| s.tunes())
            .map(|t| self.stars_for(t.id))
            .sum()
    }

    fn add_run(&mut self, run: &Run) {
        let tally: Tally = run.tally();
        self.tunes_danced += run.results.len() as u32;
        self.steps += tally.hits();
        self.spot_on += tally.spot_on;
        self.best_streak = self.best_streak.max(run.best_streak());
        for r in &run.results {
            self.shamrocks_heard += r.shamrocks;
            self.luck += r.luck;
            if r.clover {
                self.clovers += 1;
            }
        }
    }
}

/// What a finished set changed.
#[derive(Clone, Debug, PartialEq)]
pub struct SetSaved {
    pub score: ScoreResult,
    /// Tunes that earned more stars than ever, by name, with the stars.
    pub better: Vec<(&'static str, u32)>,
    /// The set this run opened, if any.
    pub opened: Option<u32>,
    pub best_streak: u32,
    pub streak_record: bool,
    /// The set after this one, when it is open.
    pub next_open: Option<u32>,
}

/// Saves a finished set.
pub fn save_set(ctx: &mut Ctx<Parlor>, number: u32, run: &Run) -> SetSaved {
    let mut p = Progress::load(ctx);
    let was_open: Vec<bool> = SETS.iter().map(|s| p.set_open(s.number)).collect();
    let streak_record = run.best_streak() > p.best_streak;
    let mut better = Vec::new();
    for r in &run.results {
        let old = p.stars_for(r.tune.id);
        if r.stars > old {
            p.stars.insert(r.tune.id.to_string(), r.stars);
            better.push((r.tune.name, r.stars));
        }
    }
    p.add_run(run);
    p.sets_danced += 1;
    let opened = SETS
        .iter()
        .zip(&was_open)
        .find(|(s, was)| !**was && p.set_open(s.number))
        .map(|(s, _)| s.number);
    let next_open =
        ((number as usize) < SETS.len() && p.set_open(number + 1)).then_some(number + 1);
    p.store(ctx);
    let score = Records::record(ctx, ID, &set_board(number), run.score() as u64);
    SetSaved {
        score,
        better,
        opened,
        best_streak: p.best_streak,
        streak_record,
        next_open,
    }
}

/// What a finished ceili night changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CeiliSaved {
    pub score: ScoreResult,
    pub year: i32,
    pub keepsake: Keepsake,
    /// The clover was found for the first time this year.
    pub first_clover: bool,
}

/// Saves a finished ceili night under `year`'s keepsake.
pub fn save_ceili(ctx: &mut Ctx<Parlor>, year: i32, run: &Run) -> CeiliSaved {
    let mut p = Progress::load(ctx);
    p.add_run(run);
    let k = p.keepsakes.entry(year).or_default();
    let first_clover = run.clover() && !k.clover;
    k.nights += 1;
    k.best = k.best.max(run.score());
    k.clover |= run.clover();
    k.luck += run.luck();
    let keepsake = *k;
    p.store(ctx);
    let score = Records::record(ctx, ID, CEILI_BOARD, run.score() as u64);
    CeiliSaved {
        score,
        year,
        keepsake,
        first_clover,
    }
}

/// Notes that the clover has come out tonight, so it does not come again.
pub fn clover_called(ctx: &mut Ctx<Parlor>, night: &str) {
    let mut p = Progress::load(ctx);
    if p.clover_nights.insert(night.to_string()) {
        p.store(ctx);
    }
}

/// Whether the clover is still to come out tonight.
pub fn clover_due(ctx: &mut Ctx<Parlor>, night: &str) -> bool {
    !Progress::load(ctx).clover_nights.contains(night)
}

/// Saves the tallies of a challenge run, which keeps its own score.
pub fn save_challenge(ctx: &mut Ctx<Parlor>, run: &Run) {
    let mut p = Progress::load(ctx);
    p.add_run(run);
    p.store(ctx);
}

/// Adds steps danced in the practice room.
pub fn save_practice(ctx: &mut Ctx<Parlor>, steps: u32) {
    if steps == 0 {
        return;
    }
    let mut p = Progress::load(ctx);
    p.practice_steps += steps;
    p.store(ctx);
}
