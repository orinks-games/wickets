//! Landings for special nights.
//!
//! On every full moon night the title's Full moon pad row opens: one moon
//! made from that full moon's date, the same for every player, named for
//! the moon (the Harvest Moon, the Hunter's Moon). On other nights the row
//! stays in its place, locked, and says when it opens. Crickets sing quietly
//! behind mission control's radio click. On 20 July, the day of the first
//! moon landing (19 to 21 July, for time zones), the Tranquility pad opens:
//! a low gravity landing where the radio beeps with the Quindar tone of
//! the Apollo flights. Each keeps its own best score.
//!
//! Everything here is a plain function of the date, so tests pin it.

use parlor_core::almanac::{self, Window};
use parlor_core::calendar;
use parlor_core::{Date, Happening, Rng};

use crate::rules::Moon;

/// The full moon row's name.
pub const FULL_MOON: &str = "Full moon pad";
/// Its best-score board.
pub const FULL_MOON_BOARD: &str = "full moon";
/// The moon landing row's name.
pub const TRANQUILITY: &str = "Tranquility pad";
/// Its best-score board.
pub const TRANQUILITY_BOARD: &str = "tranquility";
/// When the Tranquility pad is open: 20 July, a day either side.
pub const TRANQUILITY_WINDOW: Window = Window::Range((7, 19), (7, 21));
/// What Play again says once the full moon pad has closed.
pub const FULL_MOON_OVER: &str =
    "The full moon has set, and the full moon pad is closed until the next full moon";
/// What Play again says once the Tranquility pad has closed.
pub const TRANQUILITY_OVER: &str =
    "Moon landing day is over, and the Tranquility pad is closed until next July";
/// How many times the best burn a full moon pad's tank holds.
pub const FULL_MOON_MARGIN: f64 = 2.0;

/// The day of the full moon that `date` belongs to, on a full moon night.
pub fn full_moon_day(date: Date) -> Option<Date> {
    almanac::full_moon(date)?;
    let day = date.days() as f64 + 0.5 + almanac::days_to_full(date);
    Some(Date::from_days(day.floor() as i64))
}

/// Tonight's full moon pad, when `date` is a full moon night: seeded by
/// the full moon's own day, so its three nights share one moon.
pub fn full_moon_pad(date: Date) -> Option<Moon> {
    let full = almanac::full_moon(date)?;
    let day = full_moon_day(date)?;
    let mut rng = Rng::seeded(0x4655_4c4c_0000_0000 ^ day.days() as u64);
    let mut moon = Moon::seeded(&mut rng, FULL_MOON_MARGIN, false);
    moon.name = full.name;
    Some(moon)
}

/// The next full moon from `date`, and when: "the Hunter's Moon,
/// tomorrow".
fn next_full_moon_words(date: Date) -> String {
    let (next, days) = almanac::next_full_moon(date);
    let name = almanac::full_moon(next).map_or("the next full moon", |m| m.name);
    format!("{name}, {}", calendar::days_phrase(days))
}

/// What Enter on the locked full moon row says.
pub fn full_moon_locked_line(date: Date) -> String {
    format!(
        "The full moon pad opens for {}.",
        next_full_moon_words(date)
    )
}

/// The locked full moon row's help.
pub fn full_moon_locked_help(date: Date) -> String {
    format!(
        "Opens for {}: one moon for every player, with crickets on the radio.",
        next_full_moon_words(date)
    )
}

/// Whether the Tranquility pad is open on `date`.
pub fn tranquility_open(date: Date) -> bool {
    TRANQUILITY_WINDOW.is_open(date)
}

/// What Enter on the locked Tranquility row says.
pub fn tranquility_locked_line(date: Date) -> String {
    format!(
        "The Tranquility pad opens {}.",
        TRANQUILITY_WINDOW.next_words(date)
    )
}

/// The locked Tranquility row's help.
pub fn tranquility_locked_help(date: Date) -> String {
    format!(
        "Opens {}, for moon landing day: Tranquility Base in low gravity.",
        TRANQUILITY_WINDOW.next_words(date)
    )
}

/// The Tranquility pad: low gravity, a gentle start and a roomy tank.
pub fn tranquility() -> Moon {
    Moon {
        name: "Tranquility Base",
        gravity: 1.0,
        height: 400.0,
        drop: 4.0,
        thrust: 2.6,
        fuel: 34.0,
        safe: 3.0,
    }
}

/// The Tranquility pad's line, while it is open.
const TRANQUILITY_LINE: &str =
    "Land at Tranquility Base in low gravity for moon landing day, with the Quindar tone on the radio.";

/// What the launcher names while a row is open: both rows when a full
/// moon falls on moon landing day.
pub fn event(date: Date) -> Option<Happening> {
    let full = full_moon_pad(date).map(|moon| {
        format!(
            "Land on {} tonight, the same moon for every player, with crickets on the radio.",
            moon.name
        )
    });
    match (tranquility_open(date), full) {
        (true, Some(full)) => Some(Happening::new(
            format!("{TRANQUILITY} and the full moon pad"),
            format!("{TRANQUILITY_LINE} {full}"),
        )),
        (true, None) => Some(Happening::new(TRANQUILITY, TRANQUILITY_LINE)),
        (false, Some(full)) => Some(Happening::new(FULL_MOON, full)),
        (false, None) => None,
    }
}
