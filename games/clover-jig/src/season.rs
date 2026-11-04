//! Saint Patrick's Day on the real calendar.
//!
//! - Shamrock week, the 7 days before the day: shamrocks are hidden in the
//!   tunes of every set.
//! - The day itself, March 16 to 18 so it holds in every time zone: the
//!   village ceili opens, shamrocks and all, with the full band, the set
//!   dance and a four-leaf clover once a night.
//!
//! Everything here is a plain function of the date, so tests can walk the
//! calendar.

use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening, Period};

/// The holiday the game keeps.
pub const HOLIDAY: Holiday = Holiday::StPatricksDay;

/// What the calendar holds today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Festival {
    /// The rest of the year.
    Off,
    /// Shamrock week: `days` to go.
    Shamrocks { days: i64 },
    /// Saint Patrick's Day: ceili night.
    Ceili,
}

pub fn festival(today: Date) -> Festival {
    match calendar::nearness(HOLIDAY, today) {
        Some(Nearness::Day) => Festival::Ceili,
        Some(Nearness::Season { days }) => Festival::Shamrocks { days },
        None => Festival::Off,
    }
}

/// Whether shamrocks hide in the tunes today: shamrock week and the day.
pub fn shamrocks_on(today: Date) -> bool {
    festival(today) != Festival::Off
}

/// Whether the ceili is open today.
pub fn ceili_open(today: Date) -> bool {
    festival(today) == Festival::Ceili
}

/// The event the launcher names, from `INFO.event`.
pub fn event(today: Date) -> Option<Happening> {
    match festival(today) {
        Festival::Ceili => Some(Happening::new(
            "Ceili night",
            "The village ceili is open, with the full band, the set dance and a four-leaf clover to find.",
        )),
        Festival::Shamrocks { .. } => Some(Happening::new(
            "Shamrock week",
            "Shamrocks hide in the tunes: hit a step on its chime for a luck point.",
        )),
        Festival::Off => None,
    }
}

/// "Saint Patrick's Day is in 146 days."
pub fn countdown(today: Date) -> String {
    calendar::countdown(HOLIDAY, today)
        .unwrap_or_else(|| "Saint Patrick's Day is on March 17.".to_string())
}

/// F1 on the ceili night row.
pub fn ceili_help(today: Date) -> String {
    match festival(today) {
        Festival::Ceili => "The village ceili, open tonight: the full band plays a set dance of four tunes, and a four-leaf clover hides in it once a night.".to_string(),
        _ => format!(
            "The village ceili opens on Saint Patrick's Day, March 17. {}",
            countdown(today)
        ),
    }
}

/// A line about shamrocks for the Play row's help, in shamrock week and on
/// the day.
pub fn shamrock_line(today: Date) -> Option<String> {
    match festival(today) {
        // Shamrocks stay through the day and its slack, so count those too.
        Festival::Shamrocks { days } => Some(format!(
            "Shamrock week: shamrocks hide in the tunes for {}.",
            match days + 1 + calendar::DAY_SLACK {
                1 => "1 more day".to_string(),
                n => format!("{n} more days"),
            }
        )),
        Festival::Ceili => Some("Saint Patrick's Day: shamrocks hide in the tunes.".to_string()),
        Festival::Off => None,
    }
}

/// The night a date belongs to, for the clover's once a night.
pub fn night_key(today: Date) -> String {
    today.period_key(Period::Daily)
}

/// The year a ceili keepsake is for. The day never crosses a new year.
pub fn keepsake_year(today: Date) -> i32 {
    today.year
}
