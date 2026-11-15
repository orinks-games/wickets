//! The shore's calendar: king tides in winter, and moonlit pools on every
//! full moon.
//!
//! Each event is a row of its own on the title, with a board of its own.
//! While it is closed the row stays in its place, locked, and says when
//! it opens. The ordinary shores never change.
//!
//! - King tide: within a day of a new or full moon, November to
//!   February. The sea goes far out, so two extra pools open at each end
//!   of the shore. This is a game's take on the big winter spring tides,
//!   not a tide table.
//! - Moonlit pools: on any full moon night. A night trip: gulls sleep and
//!   a night heron calls, and the moon jellyfish and the baby octopus can
//!   come out whatever the weather.

use parlor_core::almanac::{self, Window};
use parlor_core::{Date, Happening};

/// One of the shore's events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    KingTide,
    Moonlit,
}

impl Event {
    pub const ALL: [Event; 2] = [Event::KingTide, Event::Moonlit];

    /// The row's name.
    pub fn name(self) -> &'static str {
        match self {
            Event::KingTide => "King tide",
            Event::Moonlit => "Moonlit pools",
        }
    }

    /// "the king tide", for a result line.
    pub fn the(self) -> &'static str {
        match self {
            Event::KingTide => "the king tide",
            Event::Moonlit => "the moonlit pools",
        }
    }

    /// Its best-score board.
    pub fn board(self) -> &'static str {
        match self {
            Event::KingTide => "king tide",
            Event::Moonlit => "moonlit",
        }
    }

    /// Whether it is open on `date`.
    pub fn is_open(self, date: Date) -> bool {
        match self {
            Event::KingTide => {
                Window::Months(11, 2).is_open(date)
                    && (Window::FullMoon.is_open(date) || Window::NewMoon.is_open(date))
            }
            Event::Moonlit => Window::FullMoon.is_open(date),
        }
    }

    /// What the title says when its window closed before a replay.
    pub fn ended(self) -> &'static str {
        match self {
            Event::KingTide => "The king tide has ended.",
            Event::Moonlit => "The moonlit pools have ended.",
        }
    }

    /// When it next opens: "Tuesday 24 November, in 26 days".
    fn next_words(self, date: Date) -> String {
        match almanac::next_day(date, 800, |d| self.is_open(d)) {
            Some((day, days)) => format!(
                "{}, {}",
                almanac::date_words(day),
                parlor_core::calendar::days_phrase(days)
            ),
            None => "later".to_string(),
        }
    }

    /// What Enter on its locked row says.
    pub fn locked_line(self, date: Date) -> String {
        let verb = match self {
            Event::KingTide => "opens",
            Event::Moonlit => "open",
        };
        format!("{} {verb} {}.", self.name(), self.next_words(date))
    }

    /// Its locked row's help.
    pub fn locked_help(self, date: Date) -> String {
        let what = match self {
            Event::KingTide => {
                "King tides come within a day of a new or full moon, November to February, with two extra pools at each end of the shore."
            }
            Event::Moonlit => {
                "They open on every full moon: a night trip, when moon jellyfish come out."
            }
        };
        format!("{} {what}", self.locked_line(date))
    }

    /// Days left while open, counting today: 1 on the last day.
    pub fn days_left(self, date: Date) -> Option<i64> {
        if !self.is_open(date) {
            return None;
        }
        (1..10)
            .find(|&n| !self.is_open(Date::from_days(date.days() + n)))
            .or(Some(10))
    }
}

/// Every event open on `date`, king tide first.
pub fn open_on(date: Date) -> Vec<Event> {
    Event::ALL.into_iter().filter(|e| e.is_open(date)).collect()
}

/// "the Hunter's Moon", or "the full moon" when none is up.
pub fn moon_name(date: Date) -> &'static str {
    almanac::full_moon(date).map_or("the full moon", |m| m.name)
}

/// What the launcher names while an event is on.
pub fn event(date: Date) -> Option<Happening> {
    let first = *open_on(date).first()?;
    Some(match first {
        Event::KingTide => Happening::new(
            "King tide",
            "The sea is far out today, and two extra pools open at each end of the shore.",
        ),
        Event::Moonlit => Happening::new(
            "Moonlit pools",
            format!(
                "A night trip under {}, when moon jellyfish and baby octopuses come out.",
                moon_name(date)
            ),
        ),
    })
}

fn left_words(event: Event, date: Date) -> &'static str {
    match event.days_left(date) {
        Some(1) => " Today is its last day.",
        Some(_) => " It stays until tomorrow.",
        None => "",
    }
}

/// An event's best score and relaxed best score.
pub type Bests = (Option<u64>, Option<u64>);

/// The row's help, for the shore it visits.
pub fn row_help(event: Event, date: Date, place: &str, (best, relaxed): Bests) -> String {
    let mut best = match best {
        Some(b) => format!(" Your best is {b}."),
        None => String::new(),
    };
    if let Some(r) = relaxed {
        best.push_str(&format!(" Your relaxed best is {r}."));
    }
    let left = left_words(event, date);
    match event {
        Event::KingTide => format!(
            "King tide. The sea is far out today, so two extra pools open at each end of {place}, on a spring tide. Scored on a board of its own.{best} King tides come within a day of a new or full moon, from November to February.{left}"
        ),
        Event::Moonlit => format!(
            "A night trip to {place} under {}. Moon jellyfish and baby octopuses can come out whatever the weather. Scored on a board of its own.{best} Moonlit pools open on every full moon.{left}",
            moon_name(date)
        ),
    }
}
