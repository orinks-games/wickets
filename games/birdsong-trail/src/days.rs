//! The bird year on the real calendar: which season the trails are in,
//! and the special days that open a special walk.
//!
//! - International Dawn Chorus Day, the first Sunday in May: the dawn
//!   chorus, every bird of the season singing at once.
//! - The spring and autumn equinoxes: a migration count, flocks passing
//!   overhead, north in spring and south in autumn.
//!
//! Each counts from one day before to one day after its date, as the
//! parlor's holidays do, so the local date in any time zone falls inside.
//! Everything here is a plain function of the date, so tests can check
//! any year.

use parlor_core::calendar::{self, Holiday, DAY_SLACK};
use parlor_core::{Date, Happening, Season};

/// A special walk that a day of the bird year opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Special {
    DawnChorus,
    SpringMigration,
    AutumnMigration,
}

impl Special {
    pub const ALL: [Special; 3] = [
        Special::DawnChorus,
        Special::SpringMigration,
        Special::AutumnMigration,
    ];

    /// The walk's name, as a row and the launcher say it.
    pub fn name(self) -> &'static str {
        match self {
            Special::DawnChorus => "Dawn chorus",
            Special::SpringMigration => "Spring migration",
            Special::AutumnMigration => "Autumn migration",
        }
    }

    /// The day that opens it, as a player hears it.
    pub fn day_name(self) -> &'static str {
        match self {
            Special::DawnChorus => "International Dawn Chorus Day",
            Special::SpringMigration => "the spring equinox",
            Special::AutumnMigration => "the autumn equinox",
        }
    }

    /// What the walk is, in one sentence.
    pub fn line(self) -> &'static str {
        match self {
            Special::DawnChorus => {
                "Every bird of the season sings at once, and you pick them out one by one."
            }
            Special::SpringMigration => {
                "Flocks pass overhead on their way north, and you count each one as it goes."
            }
            Special::AutumnMigration => {
                "Flocks pass overhead on their way south, and you count each one as it goes."
            }
        }
    }

    /// Its date in `year`.
    pub fn date(self, year: i32) -> Date {
        match self {
            Special::DawnChorus => dawn_chorus_day(year),
            Special::SpringMigration => holiday_date(Holiday::SpringEquinox, year),
            Special::AutumnMigration => holiday_date(Holiday::AutumnEquinox, year),
        }
    }

    /// Whole days from `today` to its next date that still counts as the
    /// day: 0 or less means it is the day.
    pub fn days_until(self, today: Date) -> i64 {
        (today.year - 1..=today.year + 1)
            .map(|y| self.date(y).days() - today.days())
            .filter(|&n| n >= -DAY_SLACK)
            .min()
            .unwrap_or(i64::MAX)
    }

    /// Whether `today` counts as its day.
    pub fn is_on(self, today: Date) -> bool {
        self.days_until(today) <= DAY_SLACK
    }
}

fn holiday_date(holiday: Holiday, year: i32) -> Date {
    holiday.date(year).unwrap_or_else(|| Date::new(year, 1, 1))
}

/// International Dawn Chorus Day: the first Sunday in May.
pub fn dawn_chorus_day(year: i32) -> Date {
    calendar::nth_weekday(year, 5, 6, 1)
}

/// The special walk open on `today`, if any.
pub fn special(today: Date) -> Option<Special> {
    Special::ALL.into_iter().find(|s| s.is_on(today))
}

/// The next special walk after today's, with the days until it.
pub fn next_special(today: Date) -> (Special, i64) {
    Special::ALL
        .into_iter()
        .filter(|s| !s.is_on(today))
        .map(|s| (s, s.days_until(today)))
        .min_by_key(|&(_, n)| n)
        .unwrap_or((Special::DawnChorus, 0))
}

/// A sentence naming the next special walk.
pub fn countdown(today: Date) -> String {
    let (s, days) = next_special(today);
    let name = s.name().to_lowercase();
    format!(
        "Next: the {name}, on {}, {}.",
        s.day_name(),
        calendar::days_phrase(days)
    )
}

/// The season the trails are in on `today`.
pub fn season(today: Date) -> Season {
    Season::of(today)
}

/// The game's event on `date`, for the launcher.
pub fn event(date: Date) -> Option<Happening> {
    special(date).map(|s| Happening::new(s.name(), s.line()))
}
