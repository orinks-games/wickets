//! Signal Box's calendar events.
//!
//! On the last weekend of every month, the title has a Steam gala row: a
//! night shift where every fifth train is the steam special, worth 3
//! points, which passes with a steam whistle instead of the rush. It keeps
//! its own best score. On other days the row stays in its place, locked,
//! and says when it opens.
//!
//! On Friday the 13th, the ordinary night shift runs the thirteenth train:
//! once its call is answered, it passes with a long ghostly whistle in
//! place of the rush, faded out before the next call. Each event's sound
//! is named in words before the shift, so no long line is queued while
//! calls are coming. The score and the
//! boards do not change; seeing it writes a keepsake in Records.

use parlor_core::almanac::{self, Omen, Window};
use parlor_core::{Date, Happening};

/// The last weekend of every month.
pub const GALA: Window = Window::LastWeekend;
/// The row's and the mode's name.
pub const GALA_NAME: &str = "Steam gala";
/// Its best-score board.
/// The key is older than the row's name.
pub const GALA_BOARD: &str = "moonlight";
/// Every this many trains, the steam special.
pub const SPECIAL_EVERY: u32 = 5;
/// What the special scores, in place of 1.
pub const SPECIAL_POINTS: u32 = 3;

/// The train that runs on Friday the 13th.
pub const THIRTEENTH: u32 = 13;
pub const THIRTEENTH_NAME: &str = "The thirteenth train";

/// Whether `date` is a Friday the 13th.
pub fn is_thirteenth(date: Date) -> bool {
    almanac::omens(date).contains(&Omen::FridayThe13th)
}

/// What Enter on the locked Steam gala row says.
pub fn gala_locked_line(date: Date) -> String {
    format!("The steam gala opens {}.", GALA.next_words(date))
}

/// The locked Steam gala row's help.
pub fn gala_locked_help(date: Date) -> String {
    format!(
        "Opens {}, {}: a night shift where every fifth train is a steam special, worth {SPECIAL_POINTS} points.",
        GALA.every(),
        GALA.next_words(date)
    )
}

/// What the launcher names while an event is on.
pub fn event(date: Date) -> Option<Happening> {
    if is_thirteenth(date) {
        return Some(Happening::new(
            THIRTEENTH_NAME,
            "Friday the 13th: on the night shift, the thirteenth train passes with a ghostly whistle.",
        ));
    }
    GALA.is_open(date).then(|| {
        Happening::new(
            GALA_NAME,
            format!(
                "The heritage line's steam gala: every fifth train is a steam special, worth {SPECIAL_POINTS} points."
            ),
        )
    })
}

/// The thirteenth train in words, before the shift and for S, H and F1.
/// Its sound is named here, before the line opens, never mid-shift.
pub const THIRTEENTH_WORDS: &str =
    "Friday the 13th: the thirteenth train runs tonight. Once you answer its call, it passes with a long, ghostly whistle instead of the rush.";

/// The special in words, before the shift and for S, H and F1. Its
/// sound is named here, before the line opens, never mid-shift.
pub const SPECIAL_WORDS: &str =
    "Every fifth train is the steam special: 3 points, and it passes with steam chuffs and a whistle instead of the rush.";
