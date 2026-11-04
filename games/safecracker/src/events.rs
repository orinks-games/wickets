//! Safecracker's calendar events: bonus safes on the title, open on their
//! nights only, each with its own rules and its own board.
//!
//! On Friday the 13th, the unlucky safe has one wheel whose number is
//! always 13, and a black cat asleep on top yowls softly just after the
//! clunk at 13. On every new moon, the blackout safe is cracked with the
//! power cut: rain on the windows instead of the room's hum, and the dial's
//! tens give the plain click, so you count from the zero notch.

use parlor_core::almanac::{self, Omen, Window};
use parlor_core::{Date, Happening};

use crate::rules::Mode;

/// The number one wheel of the unlucky safe always has.
pub const UNLUCKY_NUMBER: u32 = 13;

/// Every new moon night, a day either side.
pub const BLACKOUT: Window = Window::NewMoon;

pub const UNLUCKY_NAME: &str = "The unlucky safe";
pub const BLACKOUT_NAME: &str = "The blackout safe";

/// Whether `date` is a Friday the 13th.
pub fn is_unlucky(date: Date) -> bool {
    almanac::omens(date).contains(&Omen::FridayThe13th)
}

/// The event modes open on `date`, in title order.
pub fn open_modes(date: Date) -> Vec<Mode> {
    let mut out = Vec::new();
    if is_unlucky(date) {
        out.push(Mode::Unlucky);
    }
    if BLACKOUT.is_open(date) {
        out.push(Mode::Blackout);
    }
    out
}

/// What the launcher names while an event is on.
pub fn event(date: Date) -> Option<Happening> {
    if is_unlucky(date) {
        return Some(Happening::new(
            UNLUCKY_NAME,
            "Friday the 13th: a bonus safe with a 13 in its combination, and a black cat asleep on top.",
        ));
    }
    BLACKOUT.is_open(date).then(|| {
        Happening::new(
            BLACKOUT_NAME,
            "A new moon and a power cut: crack a bonus safe in the dark, counting from the zero notch.",
        )
    })
}

/// The bonus safes' modes, in title order.
pub const MODES: [Mode; 2] = [Mode::Unlucky, Mode::Blackout];

/// A bonus safe's row name.
pub fn name(mode: Mode) -> &'static str {
    match mode {
        Mode::Unlucky => UNLUCKY_NAME,
        _ => BLACKOUT_NAME,
    }
}

/// When a bonus safe next opens: "Friday 13 November, in 15 days".
fn next_words(mode: Mode, date: Date) -> String {
    let next = match mode {
        Mode::Unlucky => almanac::next_day(date, 800, is_unlucky),
        _ => BLACKOUT.next_open(date),
    };
    match next {
        Some((day, days)) => format!(
            "{}, {}",
            almanac::date_words(day),
            parlor_core::calendar::days_phrase(days)
        ),
        None => "later".to_string(),
    }
}

/// What Enter on a locked bonus safe row says.
pub fn locked_line(mode: Mode, date: Date) -> String {
    format!("{} opens {}.", name(mode), next_words(mode, date))
}

/// A locked bonus safe row's help.
pub fn locked_help(mode: Mode, date: Date) -> String {
    let when = match mode {
        Mode::Unlucky => "on every Friday the 13th",
        _ => "on every new moon",
    };
    format!("Opens {}: a bonus safe {when}.", next_words(mode, date))
}

/// How long an event row stays, for its help.
pub fn stays(mode: Mode, date: Date) -> &'static str {
    let left = match mode {
        Mode::Blackout => BLACKOUT.days_left(date).unwrap_or(1),
        _ => 1,
    };
    if left > 1 {
        " It stays while the moon is dark."
    } else {
        " Tonight is its last night."
    }
}
