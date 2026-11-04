//! The lake's calendar: the Lantern Festival, and every full moon.
//!
//! - Lantern Festival night: the fifteenth day of the Lunar New Year (New
//!   Year's Day plus 14), a day either side. A row of its own on the
//!   title, locked out of season: twice as many lanterns, a voice joining
//!   the song with every second one, and temple bells taking the choir's
//!   place as the last voice. It keeps a board of its own.
//! - Full moon: on any full moon night, a wolf howls now and then from
//!   far off across the lake on an ordinary night or a free paddle. It is
//!   named the first time, and S says the moon is full. Nothing else
//!   changes, so the boards and the challenges stay as they are.

use parlor_core::almanac::{self, Window};
use parlor_core::calendar::{self, DAY_SLACK};
use parlor_core::{Date, Happening};

/// The Lantern Festival row's name.
pub const FESTIVAL: &str = "Lantern Festival night";
/// What the title says when the festival closed before a replay.
pub const FESTIVAL_ENDED: &str = "Lantern Festival night has ended.";
/// Its board: the most lanterns gathered on it.
pub const FESTIVAL_BOARD: &str = "festival";
/// How many lanterns float on Lantern Festival night.
pub const FESTIVAL_LANTERNS: usize = 2 * crate::rules::SONG_LAYERS;

/// The Lantern Festival in `year`: 14 days after Lunar New Year's Day,
/// while the lunar table lasts.
pub fn festival_day(year: i32) -> Option<Date> {
    calendar::lunar_new_year(year).map(|d| Date::from_days(d.days() + 14))
}

/// Whether Lantern Festival night is open: its day, a day either side.
pub fn festival_open(date: Date) -> bool {
    festival_day(date.year).is_some_and(|d| (d.days() - date.days()).abs() <= DAY_SLACK)
}

/// The next Lantern Festival from `today`, and the days until it.
pub fn next_festival(today: Date) -> Option<(Date, i64)> {
    almanac::next_day(today, 400, festival_open)
}

/// What Enter on the locked festival row says.
pub fn festival_locked_line(today: Date) -> String {
    format!("{FESTIVAL} opens {}.", festival_when(today))
}

/// When the festival next opens: "Friday 19 February, in 113 days".
fn festival_when(today: Date) -> String {
    match next_festival(today) {
        Some((day, days)) => format!(
            "{}, {}",
            almanac::date_words(day),
            calendar::days_phrase(days)
        ),
        None => "15 days into the Lunar New Year".to_string(),
    }
}

/// The locked festival row's help.
pub fn festival_locked_help(today: Date, best: Option<u64>) -> String {
    let best = match best {
        Some(n) => format!(" Your most is {n} lanterns."),
        None => String::new(),
    };
    format!(
        "Opens {}, for the Lantern Festival: {FESTIVAL_LANTERNS} lanterns, with temple bells.{best}",
        festival_when(today)
    )
}

/// Tonight's full moon by name ("the Hunter's Moon"), if it is one.
pub fn full_moon(date: Date) -> Option<&'static str> {
    Window::FullMoon
        .is_open(date)
        .then(|| almanac::full_moon(date).map_or("the full moon", |m| m.name))
}

/// What the launcher names while an event is on: the festival first.
pub fn event(date: Date) -> Option<Happening> {
    if festival_open(date) {
        return Some(Happening::new(
            FESTIVAL,
            "Twenty lanterns on the lake, and temple bells ring in the song's last voice.",
        ));
    }
    full_moon(date).map(|moon| {
        Happening::new(
            "Full moon",
            format!("Under {moon}, a wolf howls far off across the lake."),
        )
    })
}

/// The festival row's help.
pub fn festival_help(date: Date, best: Option<u64>) -> String {
    let best = match best {
        Some(n) => format!(" Your most is {n} lanterns."),
        None => String::new(),
    };
    let last = if festival_day(date.year).is_some_and(|d| d.days() + DAY_SLACK == date.days()) {
        " Tonight is its last night."
    } else {
        ""
    };
    format!(
        "The Lantern Festival, 15 days into the Lunar New Year. {FESTIVAL_LANTERNS} lanterns drift on a wide lake, and a new voice joins the song with every second one. Temple bells ring in the last voice, in place of the choir. Kept on a board of its own.{best}{last}"
    )
}
