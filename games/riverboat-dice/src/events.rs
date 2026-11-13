//! The river's calendar: high water in spring, and Showboat Saturday.
//!
//! - High water, March to May: the spring river runs fast, and a rushing
//!   current takes the paddle wheel's place under saloon nights. It is
//!   named when a night starts, and the status mentions it. The rules do
//!   not change, and challenges never hear it.
//! - The full moon table, on any full moon night: a row of its own on the
//!   title, with a board of its own. It is a saloon night where doubles
//!   with no 1 double what the table holds, with a bright chime.

use parlor_core::almanac::{Week, Window};
use parlor_core::{Date, Happening};

/// When the river is high.
pub const HIGH_WATER: Window = Window::Months(3, 5);
/// When Showboat Saturday is open: the second Saturday of every month.
pub const MOON: Window = Window::Monthly(Week::Second, 5);
/// Showboat Saturday's row and mode name.
pub const MOON_NAME: &str = "Showboat Saturday";
/// Its best-night board. The key is older than the row's name.
pub const MOON_BOARD: &str = "full moon";

/// What Play again says once Showboat Saturday is over.
pub const MOON_ENDED: &str = "Showboat Saturday is over, so its table has closed.";

/// The line a high water night starts with.
pub const HIGH_WATER_LINE: &str =
    "High water. The spring river runs fast under the boat, and you hear its rushing current. The rules do not change.";

/// Whether the river is high on `date`.
pub fn high_water(date: Date) -> bool {
    HIGH_WATER.is_open(date)
}

/// Whether Showboat Saturday is open on `date`.
pub fn moon_open(date: Date) -> bool {
    MOON.is_open(date)
}

/// What Enter on the locked Showboat Saturday row says.
pub fn locked_line(date: Date) -> String {
    format!("Showboat Saturday opens {}.", MOON.next_words(date))
}

/// The locked Showboat Saturday row's help.
pub fn locked_help(date: Date) -> String {
    format!(
        "Opens {}, {}: a saloon night where doubles with no 1 double what the table holds.",
        MOON.every(),
        MOON.next_words(date)
    )
}

/// What the launcher names while an event is on: Showboat Saturday first,
/// then high water.
pub fn event(date: Date) -> Option<Happening> {
    if moon_open(date) {
        return Some(Happening::new(
            MOON_NAME,
            "The showboat is in: doubles double what the table holds, at a table of its own.",
        ));
    }
    high_water(date).then(|| {
        Happening::new(
            "High water",
            "The spring river runs fast and loud under the boat, until the end of May.",
        )
    })
}

/// Showboat Saturday's row help.
pub fn row_help(best: Option<u64>) -> String {
    let best = match best {
        Some(b) => format!(" Your best is {b}."),
        None => String::new(),
    };
    format!(
        "A saloon night on the showboat. Doubles with no 1 double what the table holds. The passengers keep their rules. Scored on a board of its own. Open all day.{best}"
    )
}
