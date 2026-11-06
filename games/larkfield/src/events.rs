//! Larkfield's calendar: a barge fair and a music night, once a month
//! each. Both touch your own hamlet, never a challenge.
//!
//! - Barge fair, the third Tuesday: the barge comes every day and pays a
//!   coin more for everything.
//! - Music night, the fourth Sunday: a band from the next village plays
//!   on the green after supper, and everyone enjoys it.

use parlor_core::almanac::{Week, Window};
use parlor_core::{Date, Happening};

/// The third Tuesday of every month.
pub const BARGE_FAIR: Window = Window::Monthly(Week::Third, 1);
/// The fourth Sunday of every month.
pub const MUSIC_NIGHT: Window = Window::Monthly(Week::Fourth, 6);

pub fn barge_fair(date: Date) -> bool {
    BARGE_FAIR.is_open(date)
}

pub fn music_night(date: Date) -> bool {
    MUSIC_NIGHT.is_open(date)
}

/// What the launcher names while an event is on.
pub fn event(date: Date) -> Option<Happening> {
    if barge_fair(date) {
        return Some(Happening::new(
            "Barge fair",
            "The barge comes to Larkfield every day today, and pays a coin more for everything.",
        ));
    }
    if music_night(date) {
        return Some(Happening::new(
            "Music night",
            "A band from the next village plays on Larkfield's green after supper today.",
        ));
    }
    None
}
