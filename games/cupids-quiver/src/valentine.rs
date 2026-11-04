//! Valentine's Day on the real calendar: what the sky holds on a date.
//!
//! Plain functions of [`Date`], so tests can walk them across years:
//!
//! - in the run-up (the 7 days before), roses drift among the hearts as
//!   bonus targets, and the music changes;
//! - on the day itself, give or take a day for time zones, Valentine's
//!   night and the game for two open, with golden hearts and a keepsake
//!   for the year;
//! - the rest of the year, those rows are locked and say how long to wait.

use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening};

/// The holiday the game celebrates.
pub const HOLIDAY: Holiday = Holiday::ValentinesDay;

/// Where a date sits against Valentine's Day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    /// Valentine's Day, give or take a day.
    Day,
    /// The run-up: `days` whole days to go.
    Season { days: i64 },
    /// The rest of the year: `days` until the next Valentine's Day.
    Off { days: i64 },
}

/// Where `date` sits against Valentine's Day.
pub fn gate(date: Date) -> Gate {
    match calendar::nearness(HOLIDAY, date) {
        Some(Nearness::Day) => Gate::Day,
        Some(Nearness::Season { days }) => Gate::Season { days },
        None => Gate::Off {
            days: days_until(date),
        },
    }
}

/// Whole days from `date` until the next Valentine's Day, 0 or less on it.
pub fn days_until(date: Date) -> i64 {
    HOLIDAY.next(date).map(|(_, days)| days).unwrap_or(365)
}

/// Whether Valentine's night and the game for two are open.
pub fn night_open(date: Date) -> bool {
    gate(date) == Gate::Day
}

/// Whether roses drift in the sky: the run-up and the day itself.
pub fn roses_on(date: Date) -> bool {
    matches!(gate(date), Gate::Day | Gate::Season { .. })
}

/// The year a keepsake is kept under: the Valentine's Day this date counts
/// for, so 15 February still files under that year's day.
pub fn keepsake_year(date: Date) -> i32 {
    match HOLIDAY.next(date) {
        Some((day, _)) => day.year,
        None => date.year,
    }
}

/// "Valentine's Day is in 9 days." for a locked row's help.
pub fn countdown(date: Date) -> String {
    calendar::countdown(HOLIDAY, date).unwrap_or_else(|| "Valentine's Day is soon.".to_string())
}

/// A locked Valentine row's help: when it opens and how long to wait.
pub fn locked_help(date: Date) -> String {
    format!(
        "Opens on Valentine's Day, the 14th of February. {}",
        countdown(date)
    )
}

/// What the launcher names on the lobby and the game list.
pub fn event(date: Date) -> Option<Happening> {
    match gate(date) {
        Gate::Day => Some(Happening::new(
            "Valentine's night",
            "Golden hearts, a game for two, and a keepsake for this year.",
        )),
        Gate::Season { days } => Some(Happening::new(
            "Roses in the sky",
            format!(
                "Roses drift among the hearts for bonus points, and Valentine's Day is in {days} days."
            ),
        )),
        Gate::Off { .. } => None,
    }
}

/// The keepsake a year's Valentine's night leaves in the box.
pub fn keepsake_item(year: i32) -> &'static str {
    const ITEMS: [&str; 8] = [
        "a pressed red rose",
        "a ribbon of red silk",
        "a silver arrowhead",
        "two paper hearts stitched together",
        "a card with a gold edge",
        "a brass key on a ribbon",
        "a feather from Cupid's own wing",
        "a tiny glass heart",
    ];
    ITEMS[(year - 2024).rem_euclid(ITEMS.len() as i32) as usize]
}
