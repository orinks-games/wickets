//! The garden's calendar: pumpkin seeds in autumn, and Sunday bells once
//! a month.
//!
//! Both touch only your own garden, never a challenge, and neither
//! changes how anything grows or scores.
//!
//! - Pumpkin seeds: from 22 September to Thanksgiving, the seed packet
//!   holds a pumpkin. It grows exactly like a squash and takes the
//!   squash's part in the song, but sings it as a low bassoon. A pumpkin
//!   you plant stays a pumpkin after the season ends.
//! - Full moon: on a full moon night, each garden evening the full moon
//!   rises with a soft glassy swell and an owl's hoot, in place of the
//!   evening tune. S names the moon.

use parlor_core::almanac::{Week, Window};
use parlor_core::calendar::Holiday;
use parlor_core::{Date, Happening};

/// The pumpkin season's first day, month and day.
pub const PUMPKIN_FROM: (u32, u32) = (9, 22);

/// The last day of the pumpkin season in `year`: Thanksgiving.
pub fn pumpkin_until(year: i32) -> Date {
    Holiday::Thanksgiving
        .date(year)
        .unwrap_or(Date::new(year, 11, 26))
}

/// Whether pumpkin seeds are in the packet on `date`.
pub fn pumpkin_open(date: Date) -> bool {
    let from = Date::new(date.year, PUMPKIN_FROM.0, PUMPKIN_FROM.1);
    from.days() <= date.days() && date.days() <= pumpkin_until(date.year).days()
}

/// When the Sunday bells ring: the first Sunday of every month.
pub const SUNDAY_BELLS: Window = Window::Monthly(Week::First, 6);

/// Whether the church bells ring this evening.
pub fn sunday_bells(date: Date) -> bool {
    SUNDAY_BELLS.is_open(date)
}

/// What the launcher names while an event is on: pumpkin seeds.
pub fn event(date: Date) -> Option<Happening> {
    if pumpkin_open(date) {
        return Some(Happening::new(
            "Pumpkin seeds",
            "Until Thanksgiving, plant a pumpkin in your own garden: it grows like a squash and sings a low bassoon.",
        ));
    }
    None
}
