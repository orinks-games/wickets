//! The game's calendar.
//!
//! - The Frost Fair, on the winter solstice: the river freezes, the stalls
//!   set up on the ice, and you skate the long course once a year for a
//!   ribbon. Skated on its day, it stays open the rest of that winter.
//! - The lantern skate, every evening, by your clock: the day's lanterns
//!   light in pairs, and you take the first lit of each. Every player
//!   skates the same lanterns that day, and they play the day's tune.

use hearken::Ctx;
use parlor_core::calendar::{self, Holiday, Season};
use parlor_core::{Date, Happening, Parlor, Rng};

use crate::ponds::LANTERN_PAIRS;

/// The Frost Fair's row and title.
pub const FAIR_ROW: &str = "The Frost Fair";
/// The lantern skate's row and title.
pub const LANTERN_ROW: &str = "Lantern skate";

/// The Frost Fair's holiday.
pub const FAIR_DAY: Holiday = Holiday::WinterSolstice;

/// The hours, by your clock, the lantern skate is lit: from 4 in the
/// afternoon to 9 in the evening.
pub const DUSK_FROM: u32 = 16;
pub const DUSK_TO: u32 = 21;

/// Whether `date` is in the winter the game is listed in: December to
/// February.
pub fn winter(date: Date) -> bool {
    Season::of(date) == Season::Winter
}

/// Whether `date` is the winter solstice, the Frost Fair's day.
pub fn fair_day(date: Date) -> bool {
    calendar::is_day(FAIR_DAY, date)
}

/// The winter `date` belongs to, named by the year its December falls in.
pub fn winter_of(date: Date) -> i32 {
    if date.month <= 6 {
        date.year - 1
    } else {
        date.year
    }
}

/// Whether the lantern skate is lit at `hour`.
pub fn dusk(hour: u32) -> bool {
    (DUSK_FROM..DUSK_TO).contains(&hour)
}

/// Whether the lantern skate is lit now, by your clock.
pub fn dusk_now(ctx: &Ctx<Parlor>) -> bool {
    dusk(parlor_core::story::now(ctx).hour)
}

/// The day's tune: a note for each lantern pair, up and down the scale,
/// home at the end. The same for every player on `date`.
pub fn tune(date: Date) -> Vec<usize> {
    let mut rng = Rng::seeded(0x50f7_1a7e ^ date.days() as u64);
    let mut note: i32 = *rng.pick(&[0, 2, 4]);
    let mut out = Vec::new();
    for i in 0..LANTERN_PAIRS {
        out.push(note as usize);
        let left = LANTERN_PAIRS - 1 - i;
        // Head for home over the last few notes.
        let step = if left <= 3 {
            (-note).signum() * note.abs().min(2)
        } else {
            *rng.pick(&[-2, -1, -1, 1, 1, 2, 0])
        };
        note = (note + step).clamp(0, 7);
    }
    if let Some(last) = out.last_mut() {
        *last = 0;
    }
    out
}

/// The seed of the day's lantern skate.
pub fn lantern_seed(date: Date) -> u64 {
    0x1a_77e2_u64 ^ (date.days() as u64).wrapping_mul(0x9e37_79b9)
}

/// What the launcher names on the Frost Fair's day.
pub fn event(date: Date) -> Option<Happening> {
    if !winter(date) || !fair_day(date) {
        return None;
    }
    Some(
        Happening::new(
            FAIR_ROW,
            "The river is frozen, and the fair is out on the ice today.",
        )
        .on_row(FAIR_ROW),
    )
}

/// Words for the Frost Fair's day.
pub fn fair_date_words() -> &'static str {
    "the winter solstice, in December"
}

/// The midnight skate's row and title.
pub const MIDNIGHT_ROW: &str = "Midnight skate";

/// Whether the midnight skate is open at `t`: from 11 at night on the
/// 31st of December to 1 in the morning on the 1st of January.
pub fn midnight_open(t: hearken::clock::LocalTime) -> bool {
    (t.month == 12 && t.day == 31 && t.hour == 23) || (t.month == 1 && t.day == 1 && t.hour == 0)
}

/// Seconds from `t` to the new year's midnight, or 0 once it has passed.
pub fn secs_to_midnight(t: hearken::clock::LocalTime) -> i64 {
    if t.month == 12 && t.day == 31 {
        ((23 - t.hour as i64) * 60 + (59 - t.minute as i64)) * 60 + 60
    } else {
        0
    }
}

/// Whether `date` is the 31st of December or the 1st of January, when the
/// new year's song joins the music.
pub fn new_year(date: Date) -> bool {
    (date.month == 12 && date.day == 31) || (date.month == 1 && date.day == 1)
}
