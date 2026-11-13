//! The cabin's monthly Rabbit rabbit deal.

use parlor_core::almanac::Window;
use parlor_core::{Date, Happening, Rng};

use crate::pool::RELAXED;

/// When the Rabbit rabbit deal is open: the 1st of every month.
pub const RABBIT: Window = Window::DayOfMonth(1);
/// The Rabbit rabbit row's name.
pub const RABBIT_NAME: &str = "Rabbit rabbit deal";
/// Its best-score board. The key is older than the row's name.
pub const RABBIT_BOARD: &str = "moonlit";

/// An event's deal, as the table plays it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventDeal {
    /// The Rabbit rabbit deal: turning the stock is free.
    Rabbit,
}

impl EventDeal {
    pub fn name(self) -> String {
        RABBIT_NAME.to_string()
    }

    pub fn board(self) -> &'static str {
        RABBIT_BOARD
    }

    pub fn board_words(self) -> &'static str {
        "Rabbit rabbit"
    }
}

/// The Rabbit rabbit deal's seed: one deal from the proved pool for each
/// month, the same for every player.
pub fn rabbit_seed(date: Date) -> u64 {
    let start = date.days() as u64;
    let mut rng = Rng::seeded(0x700E ^ start.wrapping_mul(104_729));
    RELAXED[rng.below(RELAXED.len() as u32) as usize]
}

/// What Enter on the locked Rabbit rabbit row says.
pub fn rabbit_locked_line(date: Date) -> String {
    format!("The Rabbit rabbit deal opens {}.", RABBIT.next_words(date))
}

/// The locked Rabbit rabbit row's help.
pub fn rabbit_locked_help(date: Date) -> String {
    format!(
        "Opens {}, {}: one lucky deal for everyone, where turning the stock costs no moves.",
        RABBIT.every(),
        RABBIT.next_words(date)
    )
}

pub fn event(date: Date) -> Option<Happening> {
    RABBIT.is_open(date).then(|| {
        Happening::new(
            RABBIT_NAME,
            "Rabbit rabbit, a lucky month: one deal for everyone, and turning the stock costs no moves.",
        )
    })
}

/// What Play again says when the event's window has closed.
pub fn ended_words(_deal: EventDeal) -> &'static str {
    "The 1st of the month is over, so the Rabbit rabbit deal has ended."
}

/// The Rabbit rabbit row's help.
pub fn rabbit_help(best: Option<u64>) -> String {
    let best = match best {
        Some(b) => format!(" Your best is {b}."),
        None => String::new(),
    };
    format!(
        "Rabbit rabbit, for a lucky month: one deal for everyone today, and it can always be won. Turning the stock costs no moves. Scored on a board of its own. Open all day.{best}"
    )
}
