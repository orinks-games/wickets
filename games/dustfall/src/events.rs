//! Dustfall's calendar event: the dust storm, on the fourth Monday of
//! every month. A storm colony plays the twelve nights with a lane of dust
//! drifting across the sky all night. In the dust a raider is heard by
//! nobody and falls on regardless, and the flak post under it is silent.
//! On other days the row stays on the title, locked, and says when it
//! opens.

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
            "A lane of dust drifts across the sky all night: nothing in it is heard, and it falls on regardless.",
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
    "A colony through twelve nights with a lane of dust drifting across the sky, a lane at a time. Raiders in the dust cannot be heard and fall on regardless, and the flak post under it is silent. Its score has its own record.".to_string()
}
