//! Pumpkin night: Halloween on lane nine.
//!
//! From 17 October to 1 November the title has a Pumpkin night row: a
//! full game where the pins are pumpkins. They fall with a hollow,
//! squashy clatter, they are still spoken by number, and a strike is
//! carved into a grin. Scoring is the standard game's, on a board of its
//! own.

use parlor_core::almanac::Window;
use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening};

/// The holiday.
pub const HOLIDAY: Holiday = Holiday::Halloween;
/// The row's and the mode's name.
pub const NAME: &str = "Pumpkin night";
/// Its best-score board.
pub const BOARD: &str = "pumpkin night";

/// Whether Pumpkin night is on: the two weeks before Halloween, and
/// Halloween itself.
pub fn is_open(date: Date) -> bool {
    calendar::nearness(HOLIDAY, date).is_some()
}

/// When it is on, for the locked row.
const WINDOW: Window = Window::RunUp(HOLIDAY);

/// What Enter on the locked row says, out of season.
pub fn locked_line(date: Date) -> String {
    format!("{NAME} opens {}.", WINDOW.next_words(date))
}

/// The locked row's help.
pub fn locked_help(date: Date) -> String {
    format!(
        "Opens {}, for two weeks up to Halloween: ten frames with pumpkins for pins.",
        WINDOW.next_words(date)
    )
}

/// What the launcher names while it is on.
pub fn event(date: Date) -> Option<Happening> {
    is_open(date).then(|| {
        Happening::new(
            NAME,
            "Bowl ten frames with pumpkins for pins, until 1 November.",
        )
    })
}

/// The row's help.
pub fn row_help(date: Date, best: Option<u64>) -> String {
    let best = match best {
        Some(b) => format!(" Your best is {b}."),
        None => String::new(),
    };
    let left = match calendar::nearness(HOLIDAY, date) {
        Some(Nearness::Season { .. }) => " It stays until 1 November.",
        _ => " Tonight is Halloween, and it ends after 1 November.",
    };
    format!("Ten frames with pumpkins for pins, and a strike carves a grin. Scored the standard way.{best}{left}")
}
