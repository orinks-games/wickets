//! The sea's calendar.
//!
//! On every new moon night, the darkest of the month, the title has a Glow
//! dive row: an expedition on a sea made from that new moon's date, the
//! same for every player, where the water glows and each dive also hides
//! a glow pearl. Its echo is a glassy chime of its own, and it keeps its
//! own best haul. On other nights the row stays in its place, locked, and
//! says when it opens.
//!
//! On the spring and autumn equinoxes (a day either side, for time zones)
//! whales travel the whale road: far off to the north, beyond the dive
//! site, their song carries now and then. It is only a landmark, never
//! something to find, and it never sounds while a ping's echoes are on
//! their way back. Challenges keep their sea as it is.
//!
//! Everything here is a plain function of the date, so tests pin it.

use parlor_core::almanac::{self, Window};
use parlor_core::calendar::Holiday;
use parlor_core::{Date, Happening, Rng};

use crate::screens::Mode;

/// The new moon row's name.
pub const GLOW_DIVE: &str = "Glow dive";
/// Its best-haul board.
/// The key is older than the row's name.
pub const GLOW_BOARD: &str = "full moon";
/// What Play again says once the new moon has passed.
pub const GLOW_OVER: &str =
    "The new moon has passed, and the glow dive is over until the next new moon";
/// The equinox touch's name.
pub const WHALE_ROAD: &str = "The whale road";
/// Seconds between far whale songs on the whale road.
pub const ROAD_EVERY: f64 = 20.0;
/// The first far song, this many seconds into a dive.
pub const ROAD_FIRST: f64 = 8.0;
/// How far beyond the site's edge the whale road runs, in metres.
pub const ROAD_BEYOND: f32 = 40.0;

/// Whether the glow dive is open on `date`: any new moon night.
pub fn glow_open(date: Date) -> bool {
    Window::NewMoon.is_open(date)
}

/// What Enter on the locked Glow dive row says.
pub fn glow_locked_line(date: Date) -> String {
    format!("The glow dive opens {}.", Window::NewMoon.next_words(date))
}

/// The locked Glow dive row's help.
pub fn glow_locked_help(date: Date) -> String {
    format!(
        "Opens {}, {}: on the darkest nights the sea glows, and a glow pearl lies on every dive.",
        Window::NewMoon.every(),
        Window::NewMoon.next_words(date)
    )
}

/// Whether whales travel the whale road on `date`: either equinox.
pub fn whale_road_open(date: Date) -> bool {
    Window::Day(Holiday::SpringEquinox).is_open(date)
        || Window::Day(Holiday::AutumnEquinox).is_open(date)
}

/// Whether a dive of `mode` on `date` hears the whale road.
pub fn whale_road(mode: Mode, date: Date) -> bool {
    whale_road_open(date) && !matches!(mode, Mode::Challenge(_))
}

/// The glowing sea of the new moon `date` belongs to: the same for its
/// every night and every player.
pub fn sea_rng(date: Date) -> Rng {
    let day = almanac::new_moon_day(date).unwrap_or(date);
    Rng::seeded(0x5045_4152_4c00_0000 ^ day.days() as u64)
}

/// The whale road's line, while whales travel it.
const WHALE_ROAD_LINE: &str =
    "Whales sing far off to the north on every dive for the equinox, as a landmark.";

/// What the launcher names while an event is on: both when a new moon
/// falls on an equinox.
pub fn event(date: Date) -> Option<Happening> {
    let moon = glow_open(date).then(|| {
        "The new moon makes the sea glow: dive a sea the same for every player, with a glow pearl on every dive.".to_string()
    });
    match (whale_road_open(date), moon) {
        (true, Some(moon)) => Some(
            Happening::new(
                format!("{WHALE_ROAD} and the glow dive"),
                format!("{WHALE_ROAD_LINE} {moon}"),
            )
            .on_row(GLOW_DIVE),
        ),
        (true, None) => Some(Happening::new(WHALE_ROAD, WHALE_ROAD_LINE)),
        (false, Some(moon)) => Some(Happening::new(GLOW_DIVE, moon)),
        (false, None) => None,
    }
}
