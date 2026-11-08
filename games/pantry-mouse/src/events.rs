//! Pantries for special nights.
//!
//! From the day before Thanksgiving to 2 days after, the title has a
//! Leftovers pantry row: the pantry run with 4 turkey scraps hidden among the crumbs, 50
//! points each. On the first Tuesday of every month it has a Bin night row:
//! the kitchen bin is full before collection, and one cheese rind waits in
//! every pantry, 100 points. On other days the Bin night row stays in its
//! place, locked, and says when it opens. Each treat has its own sound,
//! and Space plays the nearest one after the crumb ticks. Each row keeps
//! its own best score, so the pantry's board never changes.
//!
//! Everything here is a plain function of the date, so tests pin it.

use parlor_core::almanac::{self, Week, Window};
use parlor_core::calendar::{self, Holiday};
use parlor_core::{Date, Happening};

use crate::rules::{Run, Treat};

/// The Thanksgiving row's name.
pub const LEFTOVERS: &str = "Leftovers pantry";
/// Its best-score board.
pub const LEFTOVERS_BOARD: &str = "leftovers";
/// Turkey scraps in each leftovers pantry.
pub const TURKEY_SCRAPS: u32 = 4;
/// The monthly row's name.
pub const BIN_NIGHT: &str = "Bin night";
/// Its best-score board. The key is older than the row's name.
pub const BIN_NIGHT_BOARD: &str = "moonlit";
/// When Bin night is open: the first Tuesday of every month.
pub const BIN_NIGHT_WINDOW: Window = Window::Monthly(Week::First, 1);
/// Cheese rinds in each Bin night pantry.
pub const CHEESE_RINDS: u32 = 1;
/// What Play again says once an event pantry has closed.
pub const CLOSED: &str = "That pantry has closed for now. It opens again on its next night.";

/// Whether the Leftovers pantry is open: from the day before
/// Thanksgiving, which gives time zones some slack, to 2 days after.
pub fn leftovers_open(date: Date) -> bool {
    Holiday::Thanksgiving
        .date(date.year)
        .is_some_and(|t| (-1..=2).contains(&(date.days() - t.days())))
}

/// When the Leftovers pantry next opens: "Wednesday 25 November, in 10
/// days".
fn leftovers_next_words(date: Date) -> String {
    match almanac::next_day(date, 800, leftovers_open) {
        Some((day, days)) => format!(
            "{}, {}",
            almanac::date_words(day),
            calendar::days_phrase(days)
        ),
        None => "at Thanksgiving".to_string(),
    }
}

/// What Enter on the locked Leftovers row says.
pub fn leftovers_locked_line(date: Date) -> String {
    format!("{LEFTOVERS} opens {}.", leftovers_next_words(date))
}

/// The locked Leftovers row's help.
pub fn leftovers_locked_help(date: Date, best: Option<u64>) -> String {
    format!(
        "Opens {}, around Thanksgiving: {TURKEY_SCRAPS} turkey scraps in every pantry, 50 points each.{}",
        leftovers_next_words(date),
        best_words(best)
    )
}

/// Whether Bin night is open on `date`.
pub fn bin_night_open(date: Date) -> bool {
    BIN_NIGHT_WINDOW.is_open(date)
}

/// What Enter on the locked Bin night row says.
pub fn bin_night_locked_line(date: Date) -> String {
    format!("Bin night opens {}.", BIN_NIGHT_WINDOW.next_words(date))
}

/// The locked Bin night row's help.
pub fn bin_night_locked_help(date: Date, best: Option<u64>) -> String {
    format!(
        "Opens {}, {}: a cheese rind in every pantry, 100 points.{}",
        BIN_NIGHT_WINDOW.every(),
        BIN_NIGHT_WINDOW.next_words(date),
        best_words(best)
    )
}

/// A leftovers run.
pub fn leftovers_run(seed: u64) -> Run {
    Run::pantry(seed).with_treats(Treat::Turkey, TURKEY_SCRAPS)
}

/// A Bin night run.
pub fn bin_night_run(seed: u64) -> Run {
    Run::pantry(seed).with_treats(Treat::CheeseRind, CHEESE_RINDS)
}

/// What the launcher names while a row is open.
pub fn event(date: Date) -> Option<Happening> {
    if leftovers_open(date) {
        return Some(Happening::new(
            LEFTOVERS,
            "Thanksgiving leftovers: 4 turkey scraps hide in every pantry, 50 points each.",
        ));
    }
    bin_night_open(date).then(|| {
        Happening::new(
            BIN_NIGHT,
            "The bin is full before collection: a cheese rind waits in every pantry, 100 points.",
        )
    })
}

/// " Your best is 900." for an event row's help, or nothing.
fn best_words(best: Option<u64>) -> String {
    best.map_or(String::new(), |b| format!(" Your best is {b}."))
}

/// The Leftovers pantry row's help.
pub fn leftovers_help(best: Option<u64>) -> String {
    format!(
        "Thanksgiving leftovers: the pantry run with 4 turkey scraps hidden among the crumbs, 50 points each. Open from the day before Thanksgiving to 2 days after.{}",
        best_words(best)
    )
}

/// The Bin night row's help.
pub fn bin_night_help(best: Option<u64>) -> String {
    format!(
        "Bin night, the first Tuesday of the month: the bin is full before collection, and the pantry run has a cheese rind in every pantry, 100 points. Open all day.{}",
        best_words(best)
    )
}
