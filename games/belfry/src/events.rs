//! Belfry's calendar events: rows on the title that open only on their
//! nights, each with its own rules and its own board.
//!
//! On the second Wednesday of every month, Practice night has the tower
//! captain call one place of the peal: its bell is named as the peal
//! begins, and a short, quiet shimmer marks that place just after its bell
//! as you move onto it. On other days its row stays on the title, locked,
//! and says when it opens.
use parlor_core::almanac::{Week, Window};
use parlor_core::{Date, Happening};

use crate::rules::Mode;

/// The second Wednesday of every month.
pub const PRACTICE: Window = Window::Monthly(Week::Second, 2);

/// The row's name and the mode's.
pub const PRACTICE_NAME: &str = "Practice night";

/// The event modes open on `date`, in title order.
pub fn open_modes(date: Date) -> Vec<Mode> {
    let mut out = Vec::new();
    if PRACTICE.is_open(date) {
        out.push(Mode::PracticeNight);
    }
    out
}

/// What Enter on the locked Practice night row says.
pub fn practice_locked_line(date: Date) -> String {
    format!("Practice night opens {}.", PRACTICE.next_words(date))
}

/// The locked Practice night row's help.
pub fn practice_locked_help(date: Date) -> String {
    format!(
        "Opens {}, {}: the tower captain calls one place of the peal for you.",
        PRACTICE.every(),
        PRACTICE.next_words(date)
    )
}

/// What the launcher names while an event is on.
pub fn event(date: Date) -> Option<Happening> {
    if PRACTICE.is_open(date) {
        return Some(Happening::new(
            PRACTICE_NAME,
            "The tower captain calls one place of a 5 place peal for you.",
        ));
    }
    None
}

/// " It stays 2 more days." or " Tonight is its last night."
fn left_words(window: Window, date: Date) -> String {
    match window.days_left(date) {
        Some(2) => " It stays 1 more day.".to_string(),
        Some(n) if n > 2 => format!(" It stays {} more days.", n - 1),
        Some(_) if window == PRACTICE => " Open all day.".to_string(),
        _ => " Tonight is its last night.".to_string(),
    }
}

/// The lead of an event row's help.
pub fn lead(mode: Mode, _date: Date) -> String {
    match mode {
        Mode::PracticeNight => {
            "Practice night: the tower captain calls one place of the peal, and a shimmer marks it"
                .to_string()
        }
        _ => String::new(),
    }
}

/// How long the event row stays, for its help.
pub fn stays(mode: Mode, date: Date) -> String {
    match mode {
        Mode::PracticeNight => left_words(PRACTICE, date),
        _ => String::new(),
    }
}
