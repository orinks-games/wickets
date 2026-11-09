//! Dustfall's calendar event: the dust storm, on the fourth Monday of
//! every month. A storm colony plays the twelve nights with a column of
//! dust crossing the grid each raiders' phase. In the dust a raider is
//! heard by nobody and stands still, a flak post cannot fire, and a wall
//! falls a bash sooner. On other days the row stays on the title, locked,
//! and says when it opens.

use parlor_core::almanac::{Week, Window};
use parlor_core::{Date, Happening};

/// The fourth Monday of every month.
pub const STORM: Window = Window::Monthly(Week::Fourth, 0);

/// The row's name and the mode's.
pub const STORM_NAME: &str = "Dust storm";

/// What the launcher names while the storm is on.
pub fn event(date: Date) -> Option<Happening> {
    if STORM.is_open(date) {
        return Some(Happening::new(
            STORM_NAME,
            "A column of dust crosses the grid all night: nothing is heard in it, and nothing in it moves.",
        ));
    }
    None
}

/// What Enter on the locked Dust storm row says.
pub fn locked_line(date: Date) -> String {
    format!("The dust storm comes {}.", STORM.next_words(date))
}

/// The locked row's help.
pub fn locked_help(date: Date) -> String {
    format!(
        "Comes {}, {}: a colony through twelve nights of dust, with its own record.",
        STORM.every(),
        STORM.next_words(date)
    )
}

/// The open row's help.
pub fn help() -> String {
    "A colony through twelve nights with a column of dust crossing the grid, one column each raiders' phase. Raiders in the dust cannot be heard and lose their step, flak posts in it cannot fire, and a wall in it falls a bash sooner. Its score has its own record.".to_string()
}
