//! The real calendar, as the meadow follows it: which season the evening
//! is, how many fireflies are out, midsummer night, the Fourth of July
//! fireworks, and the keepsake each midsummer brings.
//!
//! Every function here is a plain function of a [`Date`], so tests can
//! walk the year without a clock.

use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening, Season};

use crate::species::Species;

/// The meadow's season on `date`: meteorological, northern hemisphere.
pub fn season(date: Date) -> Season {
    Season::of(date)
}

/// Whether midsummer night is open: the summer solstice, give or take a
/// day for time zones.
pub fn is_midsummer(date: Date) -> bool {
    calendar::is_day(Holiday::SummerSolstice, date)
}

/// "Midsummer is in 242 days.", for the locked row's help.
pub fn midsummer_countdown(date: Date) -> String {
    calendar::countdown(Holiday::SummerSolstice, date)
        .unwrap_or_else(|| "Midsummer comes each June.".to_string())
}

/// Whether midsummer is only a few days off, so the title can say so.
pub fn midsummer_soon(date: Date) -> Option<i64> {
    match calendar::nearness(Holiday::SummerSolstice, date) {
        Some(Nearness::Season { days }) => Some(days),
        _ => None,
    }
}

/// Whether distant fireworks pop on the horizon: the Fourth of July
/// weekend. That is July 3 to 5 every year, and also any Saturday or
/// Sunday from July 1 to July 7, so a weekend next to the Fourth counts.
pub fn fireworks(date: Date) -> bool {
    if date.month != 7 {
        return false;
    }
    let weekend = date.weekday() >= 5;
    (3..=5).contains(&date.day) || (weekend && (1..=7).contains(&date.day))
}

/// What the game holds on `date`, for the launcher.
pub fn event(date: Date) -> Option<Happening> {
    if is_midsummer(date) {
        Some(Happening::new(
            "Midsummer night",
            "The whole meadow blinks together round a bonfire, with this year's keepsake.",
        ))
    } else if fireworks(date) {
        Some(Happening::new(
            "Fourth of July fireworks",
            "Fireworks go off far away on the horizon while you walk the summer meadow.",
        ))
    } else {
        None
    }
}

/// The wild fireflies of an evening, as the season allows them.
#[derive(Clone, Debug, PartialEq)]
pub struct Wild {
    /// How many are out at once.
    pub count: usize,
    /// Whether a caught one is replaced by another.
    pub respawn: bool,
    /// Only these species, when set; else the meadow's own.
    pub only: Option<Vec<Species>>,
    /// Seconds before the first one shows itself.
    pub delay: f64,
}

/// How many fireflies a season's evening holds in a meadow that holds
/// `summer` of them at the height of the year.
pub fn wild(date: Date, summer: usize) -> Wild {
    match (season(date), date.month) {
        (Season::Summer, _) => Wild {
            count: summer,
            respawn: true,
            only: None,
            delay: 0.0,
        },
        // Spring: only the first few, and only the early species.
        (Season::Spring, m) => Wild {
            count: match m {
                3 => 1,
                4 => 2,
                _ => 3,
            },
            respawn: true,
            only: Some(vec![Species::Dipper, Species::TwinSpark]),
            delay: 0.0,
        },
        // Autumn: September keeps two, then one rare late firefly.
        (Season::Autumn, 9) => Wild {
            count: 2,
            respawn: false,
            only: Some(vec![Species::SlowEmber, Species::LateEmber]),
            delay: 0.0,
        },
        (Season::Autumn, m) => Wild {
            count: 1,
            respawn: false,
            only: Some(vec![Species::LateEmber]),
            delay: if m == 10 { 15.0 } else { 20.0 },
        },
        (Season::Winter, _) => Wild {
            count: 0,
            respawn: false,
            only: Some(Vec::new()),
            delay: 0.0,
        },
    }
}

/// How a season's evening sounds, in a few words: "a summer evening".
pub fn evening_words(season: Season) -> &'static str {
    match season {
        Season::Spring => "a spring evening",
        Season::Summer => "a summer evening",
        Season::Autumn => "an autumn evening",
        Season::Winter => "a winter evening",
    }
}

/// What a season's meadow holds, for the Play row's help.
pub fn season_words(date: Date) -> String {
    let lead = match season(date) {
        Season::Summer => "Summer: the meadow is full of fireflies.",
        Season::Spring => "Spring: only the first few fireflies are out, and the frogs sing.",
        Season::Autumn if date.month == 9 => {
            "Early autumn: crickets, and the last two fireflies of the year. Challenges are summer evenings."
        }
        Season::Autumn => {
            "Autumn: crickets, falling leaves, and perhaps one rare late firefly. Challenges are summer evenings."
        }
        Season::Winter => "No fireflies are out. Challenges are summer evenings.",
    };
    let mut line = lead.to_string();
    if fireworks(date) {
        line.push_str(" Fireworks tonight, far off on the horizon.");
    }
    line
}

/// The keepsakes, one for each year's midsummer night, in turn.
pub const KEEPSAKES: [&str; 8] = [
    "a ribbon from the bonfire",
    "a pressed meadow flower",
    "a carved wooden firefly",
    "a jar lid painted with stars",
    "a smooth stone from the creek",
    "a sprig of dried lavender",
    "a little brass bell",
    "a paper lantern",
];

/// The keepsake midsummer night gives in `year`.
pub fn keepsake(year: i32) -> &'static str {
    KEEPSAKES[(year - 2024).rem_euclid(KEEPSAKES.len() as i32) as usize]
}

/// The month name, for memories: "June".
pub fn month_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "Month 12",
    ];
    NAMES[(month.clamp(1, 12) - 1) as usize]
}
