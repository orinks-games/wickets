//! Easter morning: a golden-egg hunt on Easter itself.
//!
//! Easter moves: it is the first Sunday after the spring full moon, from
//! the 22nd of March to the 25th of April. On the day (give or take a
//! day, for time zones) the title's Easter morning row opens three gentle
//! rounds full of golden eggs, worth double, with no bees and no egg
//! target. In the ten days before, the row tells you how the Easter bunny
//! is getting on. Each Easter played keeps a keepsake.
//!
//! Everything here is a plain function of the date or the year, so tests
//! can pin it.

use hearken::Ctx;
use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening, Pace, Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::ID;

/// The holiday the hunt is for.
pub const HOLIDAY: Holiday = Holiday::Easter;
/// The row's name.
pub const NAME: &str = "Easter morning";

/// What the launcher names on the day.
pub fn event(date: Date) -> Option<Happening> {
    calendar::is_day(HOLIDAY, date).then(|| {
        Happening::new(
            NAME,
            "A golden-egg hunt at sunrise, with golden eggs worth double. Open for Easter.",
        )
    })
}

/// Whether the hunt is open on `date`.
pub fn is_open(date: Date) -> bool {
    calendar::is_day(HOLIDAY, date)
}

/// Days to go in the run-up to Easter, if `date` is in it.
pub fn run_up(date: Date) -> Option<i64> {
    match calendar::nearness(HOLIDAY, date) {
        Some(Nearness::Season { days }) => Some(days),
        _ => None,
    }
}

/// The year whose Easter `date` belongs to.
pub fn year_of(date: Date) -> i32 {
    HOLIDAY.next(date).map_or(date.year, |(d, _)| d.year)
}

/// "Easter is in 12 days."
pub fn countdown(date: Date) -> String {
    calendar::countdown(HOLIDAY, date).unwrap_or_default()
}

/// What the bunny is up to with `days` to go, in the run-up.
pub fn bunny_line(days: i64) -> &'static str {
    match days {
        9.. => "The Easter bunny is counting baskets in the shed.",
        7..=8 => "The Easter bunny is painting eggs: blue, then yellow, then a few in gold.",
        5..=6 => "The Easter bunny is polishing the golden eggs until they ring.",
        3..=4 => "The Easter bunny is out in the orchard, choosing hiding places.",
        _ => "The Easter bunny is fast asleep, ready for an early start.",
    }
}

/// The row's help for `date`.
pub fn row_help(date: Date, (best, relaxed): (Option<u64>, Option<u64>)) -> String {
    if is_open(date) {
        let mut best = best.map_or(String::new(), |b| format!(" Your best is {b}."));
        if let Some(r) = relaxed {
            best.push_str(&format!(" Your relaxed best is {r}."));
        }
        format!(
            "Three rounds at sunrise, full of golden eggs worth double, with no bees and no egg target.{best}"
        )
    } else if run_up(date).is_some() {
        format!(
            "{} Until then, Enter says how the Easter bunny is getting on.",
            countdown(date)
        )
    } else {
        format!(
            "{} On Easter Sunday, a golden-egg hunt opens here.",
            countdown(date)
        )
    }
}

/// What Enter on the locked row says.
pub fn locked_line(date: Date) -> String {
    match run_up(date) {
        Some(days) => format!("{} {}", countdown(date), bunny_line(days)),
        None => format!("Easter morning opens on Easter Sunday. {}", countdown(date)),
    }
}

/// The key keepsakes are saved under.
pub fn keepsake_key() -> String {
    format!("{ID}/easter")
}

/// One Easter, kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keepsake {
    pub year: i32,
    /// The best hunt's score that year.
    pub best: u64,
    /// The most golden eggs caught in one hunt that year.
    pub golden: u32,
    /// Hunts played to the end that year.
    pub hunts: u32,
}

/// Every Easter's keepsake, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keepsakes {
    pub years: Vec<Keepsake>,
}

/// What keeping a hunt changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    First,
    Better { previous: u64 },
    Short { best: u64 },
}

impl Kept {
    /// What keeping a hunt at `pace` meant: relaxed hunts keep their own.
    pub fn line(self, year: i32, pace: Pace) -> String {
        let word = pace.best_word();
        match self {
            Kept::First if pace.separate() => {
                format!("Your relaxed keepsake for Easter {year} is saved.")
            }
            Kept::First => format!("Your keepsake for Easter {year} is saved."),
            Kept::Better { previous } => {
                format!("A new {word} for Easter {year}, up from {previous}.")
            }
            Kept::Short { best } => format!("Your {word} for Easter {year} is {best}."),
        }
    }
}

impl Keepsakes {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Keepsakes {
        Records::load(ctx, &keepsake_key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &keepsake_key(), self);
    }

    /// The keepsakes of hunts at `pace`: relaxed hunts keep their own.
    pub fn load_at(ctx: &mut Ctx<Parlor>, pace: Pace) -> Keepsakes {
        Records::load(ctx, &pace.board(&keepsake_key()))
    }

    pub fn store_at(&self, ctx: &mut Ctx<Parlor>, pace: Pace) {
        Records::store(ctx, &pace.board(&keepsake_key()), self);
    }

    pub fn year(&self, year: i32) -> Option<&Keepsake> {
        self.years.iter().find(|k| k.year == year)
    }

    /// Keeps a hunt played in `year`.
    pub fn keep(&mut self, year: i32, score: u64, golden: u32) -> Kept {
        if self.year(year).is_none() {
            self.years.push(Keepsake {
                year,
                ..Default::default()
            });
            self.years.sort_by_key(|k| k.year);
        }
        let Some(k) = self.years.iter_mut().find(|k| k.year == year) else {
            return Kept::First;
        };
        let kept = if k.hunts == 0 {
            Kept::First
        } else if score > k.best {
            Kept::Better { previous: k.best }
        } else {
            Kept::Short { best: k.best }
        };
        k.hunts += 1;
        k.best = k.best.max(score);
        k.golden = k.golden.max(golden);
        kept
    }

    /// The Records page.
    pub fn page(&self, today: Date) -> String {
        if self.years.is_empty() {
            return format!(
                "Keepsakes. None yet: Easter morning opens on Easter Sunday. {}",
                countdown(today)
            );
        }
        format!("Keepsakes. {}", self.lines(Pace::Normal))
    }

    /// The Records page of relaxed hunts' keepsakes, when there are any.
    pub fn relaxed_page(&self) -> Option<String> {
        (!self.years.is_empty())
            .then(|| format!("Relaxed keepsakes. {}", self.lines(Pace::Relaxed)))
    }

    /// "Easter 2026: best 300, 4 golden eggs in one hunt." for each year.
    fn lines(&self, pace: Pace) -> String {
        let lines: Vec<String> = self
            .years
            .iter()
            .map(|k| {
                let golden = match k.golden {
                    1 => "1 golden egg".to_string(),
                    n => format!("{n} golden eggs"),
                };
                format!(
                    "Easter {}: {} {}, {golden} in one hunt.",
                    k.year,
                    pace.best_word(),
                    k.best
                )
            })
            .collect();
        lines.join(" ")
    }
}
