//! What the game remembers: stars for each song, the nights opened, and
//! the tallies.

use std::collections::{BTreeMap, BTreeSet};

use hearken::Ctx;
use parlor_core::calendar;
use parlor_core::{challenges, Date, Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::{Run, Tally};
use crate::songs::{Monster, NIGHTS};
use crate::{HOLIDAY, ID};

/// The night used by the original score-based retention rule.
pub const KEEP_NIGHT: u32 = 1;

/// The night that waits for Halloween season all year: Doctor Volt's
/// laboratory, where the dance began.
pub const SEASON_NIGHT: u32 = 4;

/// Whether `today` is in Halloween season, when the game is in Seasonal
/// games and every night can open.
pub fn in_season(today: Date) -> bool {
    calendar::nearness(HOLIDAY, today).is_some()
}

/// Whether night `number` can open on `today`: every night in Halloween
/// season, and all but [`SEASON_NIGHT`] the rest of the year.
pub fn dated(number: u32, today: Date) -> bool {
    number < SEASON_NIGHT || in_season(today)
}

/// The score board of a night: "night 2".
pub fn set_board(number: u32) -> String {
    format!("night {number}")
}

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

/// The records key of the monsters already named in a dance.
pub fn met_key() -> String {
    format!("{ID}/met")
}

/// The monsters already named in a dance on this save, by name.
pub fn met(ctx: &mut Ctx<Parlor>) -> BTreeSet<String> {
    Records::load(ctx, &met_key())
}

/// Notes that `monster` has been named, so later dances leave it unsaid.
pub fn save_met(ctx: &mut Ctx<Parlor>, monster: Monster) {
    let mut met = met(ctx);
    if met.insert(monster.word().to_string()) {
        Records::store(ctx, &met_key(), &met);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// A real game or practice session has been started, even if abandoned.
    #[serde(default)]
    pub played: bool,
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
}

impl Progress {
    /// Starting play keeps the ordinary game; holiday-only content keeps its dates.
    pub fn note_played(ctx: &mut Ctx<Parlor>) {
        let mut progress = Self::load(ctx);
        if !progress.played {
            progress.played = true;
            progress.store(ctx);
        }
    }
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
        NIGHTS
            .iter()
            .find(|s| s.number == number)
            .is_some_and(|s| s.songs().iter().all(|t| self.stars_for(t.id) > 0))
    }

    /// Whether set `number` is earned: set 1 always, the rest once the
    /// set before is cleared.
    pub fn set_open(&self, number: u32) -> bool {
        number <= 1 || self.set_cleared(number - 1)
    }

    /// Playing keeps the ordinary game; old progress and earned access still count.
    pub fn kept(&self) -> bool {
        self.played || self.has_progress()
    }

    /// Whether a finished run, practice or star is saved: what kept the
    /// game before starting play did, and what the first results check
    /// to say it is kept.
    pub fn has_progress(&self) -> bool {
        self.any() || self.stars.values().any(|&stars| stars > 0)
    }

    /// Whether night `number` is open on `today`: earned, and in season
    /// if it is [`SEASON_NIGHT`].
    pub fn night_open(&self, number: u32, today: Date) -> bool {
        self.set_open(number) && dated(number, today)
    }

    /// The next night that is earned but waiting for Halloween, if any.
    pub fn waiting(&self, today: Date) -> Option<u32> {
        NIGHTS
            .iter()
            .map(|s| s.number)
            .find(|&n| self.set_open(n) && !dated(n, today))
    }

    /// The set the Play row dances on `today`: the first open night not
    /// cleared, or else the last open night.
    pub fn current_set(&self, today: Date) -> u32 {
        let open: Vec<u32> = NIGHTS
            .iter()
            .map(|s| s.number)
            .filter(|&n| self.night_open(n, today))
            .collect();
        open.iter()
            .copied()
            .find(|&n| !self.set_cleared(n))
            .or(open.last().copied())
            .unwrap_or(1)
    }

    /// Stars over every song of the nights, out of 36.
    pub fn total_stars(&self) -> u32 {
        NIGHTS
            .iter()
            .flat_map(|s| s.songs())
            .map(|t| self.stars_for(t.id))
            .sum()
    }

    fn add_run(&mut self, run: &Run) {
        let tally: Tally = run.tally();
        self.tunes_danced += run.results.len() as u32;
        self.steps += tally.hits();
        self.spot_on += tally.spot_on;
        self.best_streak = self.best_streak.max(run.best_streak());
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
    /// The set after this one, when it is earned but waits for Halloween.
    pub waiting: Option<u32>,
    /// Whether this run kept the game all year, for the first time.
    pub kept: bool,
    /// Whether the run ended in Halloween season, with the laboratory open.
    pub in_season: bool,
}

/// Saves a finished set.
pub fn save_set(ctx: &mut Ctx<Parlor>, number: u32, run: &Run) -> SetSaved {
    let today = challenges::today(ctx);
    let mut p = Progress::load(ctx);
    let was_open: Vec<bool> = NIGHTS
        .iter()
        .map(|s| p.night_open(s.number, today))
        .collect();
    let was_kept = p.has_progress();
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
    let opened = NIGHTS
        .iter()
        .zip(&was_open)
        .find(|(s, was)| !**was && p.night_open(s.number, today))
        .map(|(s, _)| s.number);
    let in_range = (number as usize) < NIGHTS.len();
    let next = number + 1;
    let next_open = (in_range && p.night_open(next, today)).then_some(next);
    let waiting = (in_range && p.set_open(next) && !dated(next, today)).then_some(next);
    p.store(ctx);
    let score = Records::record(ctx, ID, &set_board(number), run.score() as u64);
    SetSaved {
        score,
        better,
        opened,
        best_streak: p.best_streak,
        streak_record,
        next_open,
        waiting,
        kept: !was_kept && p.kept(),
        in_season: in_season(today),
    }
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
