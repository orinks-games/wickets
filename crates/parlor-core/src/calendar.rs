//! The holidays and seasons on the real calendar, so seasonal games can
//! hold something special on the days they celebrate.
//!
//! A seasonal game asks [`nearness`] how close today is to its holiday:
//! [`Nearness::Day`] on the day itself, [`Nearness::Season`] in the run-up,
//! `None` the rest of the year. "Today" comes from
//! [`crate::challenges::today`], so tests pin it with
//! `ctx.game.today = Some(Date::new(..))`, and a debug build can try an
//! event early with `ECHO_PARLOR_TODAY=2026-10-31`.
//!
//! Dates are UTC like the challenges, so the day counts as "the day" from
//! one day before to one day after, which holds the holiday's local date
//! in every time zone.
//!
//! A game that holds an event sets [`crate::GameInfo::event`], and the
//! launcher names it on the lobby and the game list.

use crate::Date;

/// A holiday or turning point of the year that a game can celebrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Holiday {
    NewYearsDay,
    /// The first day of the lunar year, from a table of known dates.
    LunarNewYear,
    ValentinesDay,
    StPatricksDay,
    SpringEquinox,
    AprilFools,
    /// Western Easter Sunday.
    Easter,
    SummerSolstice,
    IndependenceDay,
    AutumnEquinox,
    Halloween,
    /// The United States' Thanksgiving, the fourth Thursday of November.
    Thanksgiving,
    WinterSolstice,
    NewYearsEve,
}

impl Holiday {
    pub const ALL: [Holiday; 14] = [
        Holiday::NewYearsDay,
        Holiday::LunarNewYear,
        Holiday::ValentinesDay,
        Holiday::StPatricksDay,
        Holiday::SpringEquinox,
        Holiday::AprilFools,
        Holiday::Easter,
        Holiday::SummerSolstice,
        Holiday::IndependenceDay,
        Holiday::AutumnEquinox,
        Holiday::Halloween,
        Holiday::Thanksgiving,
        Holiday::WinterSolstice,
        Holiday::NewYearsEve,
    ];

    /// The holiday's name, as a player hears it: "Halloween".
    pub fn name(self) -> &'static str {
        match self {
            Holiday::NewYearsDay => "New Year's Day",
            Holiday::LunarNewYear => "Lunar New Year",
            Holiday::ValentinesDay => "Valentine's Day",
            Holiday::StPatricksDay => "Saint Patrick's Day",
            Holiday::SpringEquinox => "the spring equinox",
            Holiday::AprilFools => "April Fools' Day",
            Holiday::Easter => "Easter",
            Holiday::SummerSolstice => "midsummer",
            Holiday::IndependenceDay => "the Fourth of July",
            Holiday::AutumnEquinox => "the autumn equinox",
            Holiday::Halloween => "Halloween",
            Holiday::Thanksgiving => "Thanksgiving",
            Holiday::WinterSolstice => "the winter solstice",
            Holiday::NewYearsEve => "New Year's Eve",
        }
    }

    /// How many days before the day its season starts.
    pub fn lead_days(self) -> i64 {
        match self {
            Holiday::Halloween => 14,
            Holiday::Thanksgiving | Holiday::Easter | Holiday::LunarNewYear => 10,
            Holiday::ValentinesDay | Holiday::IndependenceDay | Holiday::StPatricksDay => 7,
            Holiday::NewYearsEve | Holiday::NewYearsDay => 6,
            Holiday::SpringEquinox
            | Holiday::SummerSolstice
            | Holiday::AutumnEquinox
            | Holiday::WinterSolstice => 3,
            Holiday::AprilFools => 0,
        }
    }

    /// The holiday's date in `year`, or `None` when it is not known,
    /// such as a Lunar New Year outside the table.
    pub fn date(self, year: i32) -> Option<Date> {
        let fixed = |m, d| Some(Date::new(year, m, d));
        match self {
            Holiday::NewYearsDay => fixed(1, 1),
            Holiday::LunarNewYear => lunar_new_year(year),
            Holiday::ValentinesDay => fixed(2, 14),
            Holiday::StPatricksDay => fixed(3, 17),
            Holiday::SpringEquinox => fixed(3, 20),
            Holiday::AprilFools => fixed(4, 1),
            Holiday::Easter => Some(easter(year)),
            Holiday::SummerSolstice => fixed(6, 21),
            Holiday::IndependenceDay => fixed(7, 4),
            Holiday::AutumnEquinox => fixed(9, 22),
            Holiday::Halloween => fixed(10, 31),
            Holiday::Thanksgiving => Some(nth_weekday(year, 11, 3, 4)),
            Holiday::WinterSolstice => fixed(12, 21),
            Holiday::NewYearsEve => fixed(12, 31),
        }
    }

    /// The holiday's next date that still counts as the day from `today`
    /// (up to one day past), with the whole days until it: 0 or less means
    /// it is the day.
    pub fn next(self, today: Date) -> Option<(Date, i64)> {
        (today.year - 1..=today.year + 1)
            .filter_map(|y| self.date(y))
            .map(|d| (d, d.days() - today.days()))
            .filter(|&(_, n)| n >= -DAY_SLACK)
            .min_by_key(|&(_, n)| n)
    }
}

/// How many days either side of a holiday still count as the day, so the
/// local date in any time zone falls inside it.
pub const DAY_SLACK: i64 = 1;

/// How close today is to a holiday.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nearness {
    /// The day itself, give or take [`DAY_SLACK`].
    Day,
    /// The run-up: `days` whole days to go, at least 2.
    Season { days: i64 },
}

/// How close `today` is to `holiday`, or `None` outside its season.
pub fn nearness(holiday: Holiday, today: Date) -> Option<Nearness> {
    let (_, days) = holiday.next(today)?;
    if days <= DAY_SLACK {
        Some(Nearness::Day)
    } else if days <= holiday.lead_days() {
        Some(Nearness::Season { days })
    } else {
        None
    }
}

/// Whether `today` counts as `holiday`'s day.
pub fn is_day(holiday: Holiday, today: Date) -> bool {
    nearness(holiday, today) == Some(Nearness::Day)
}

/// "today", "tomorrow" or "in 12 days", for "Halloween is in 12 days."
pub fn days_phrase(days: i64) -> String {
    match days {
        i64::MIN..=0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        n => format!("in {n} days"),
    }
}

/// "Halloween is in 12 days." or "Halloween is today.", for a title's
/// help line when the event is not on.
pub fn countdown(holiday: Holiday, today: Date) -> Option<String> {
    let (_, days) = holiday.next(today)?;
    let name = hearken::text::sentence_start(holiday.name());
    Some(format!("{name} is {}.", days_phrase(days.max(0))))
}

/// The four seasons, by the meteorological calendar of the northern
/// hemisphere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub const ALL: [Season; 4] = [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
    ];

    pub fn of(date: Date) -> Season {
        match date.month {
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            9..=11 => Season::Autumn,
            _ => Season::Winter,
        }
    }

    /// "spring", "summer", "autumn", "winter".
    pub fn name(self) -> &'static str {
        match self {
            Season::Spring => "spring",
            Season::Summer => "summer",
            Season::Autumn => "autumn",
            Season::Winter => "winter",
        }
    }
}

/// A special event a game is holding today, for the launcher to name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Happening {
    /// A few words, as "Haunted Hallways, Halloween night is on": the
    /// launcher adds "is on".
    pub title: String,
    /// One sentence on what is special, for the row's help.
    pub line: String,
    /// How the game's title row for it starts, when that is not `title`
    /// (see [`Happening::on_row`]).
    pub row: Option<String>,
}

impl Happening {
    pub fn new(title: impl Into<String>, line: impl Into<String>) -> Self {
        Happening {
            title: title.into(),
            line: line.into(),
            row: None,
        }
    }

    /// Names the start of the title row that plays the event, when it
    /// differs from the title: Stargazer's "Perseid meteor shower" is
    /// played from "Meteor shower, the Perseids".
    pub fn on_row(mut self, row: impl Into<String>) -> Self {
        self.row = Some(row.into());
        self
    }

    /// The start of the title row the launcher and story mode open the
    /// game's title on. With no row of that name, the title opens as
    /// usual, on Play, which then holds the event.
    pub fn focus(&self) -> String {
        self.row.clone().unwrap_or_else(|| self.title.clone())
    }
}

/// The `n`th (from 1) `weekday` (Monday 0) of `month`.
pub fn nth_weekday(year: i32, month: u32, weekday: u32, n: u32) -> Date {
    let first = Date::new(year, month, 1);
    let offset = (weekday + 7 - first.weekday()) % 7;
    Date::from_days(first.days() + offset as i64 + 7 * (n as i64 - 1))
}

/// Western Easter Sunday (the anonymous Gregorian computus).
pub fn easter(year: i32) -> Date {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    Date::new(year, month as u32, day as u32)
}

/// Lunar New Year's Day for 2024 to 2040.
const LUNAR_NEW_YEAR: [(u32, u32); 17] = [
    (2, 10), // 2024
    (1, 29),
    (2, 17),
    (2, 6),
    (1, 26),
    (2, 13),
    (2, 3), // 2030
    (1, 23),
    (2, 11),
    (1, 31),
    (2, 19),
    (2, 8),
    (1, 28),
    (2, 15),
    (2, 4),
    (1, 24),
    (2, 12), // 2040
];

/// Lunar New Year's Day, for the years in the table.
pub fn lunar_new_year(year: i32) -> Option<Date> {
    let at = usize::try_from(year - 2024).ok()?;
    LUNAR_NEW_YEAR.get(at).map(|&(m, d)| Date::new(year, m, d))
}

/// The zodiac animal of the lunar year that starts in `year`: 2026 is
/// the Horse.
pub fn zodiac(year: i32) -> &'static str {
    const ANIMALS: [&str; 12] = [
        "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Goat", "Monkey", "Rooster",
        "Dog", "Pig",
    ];
    ANIMALS[(year - 2020).rem_euclid(12) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easter_dates() {
        assert_eq!(easter(2024), Date::new(2024, 3, 31));
        assert_eq!(easter(2025), Date::new(2025, 4, 20));
        assert_eq!(easter(2026), Date::new(2026, 4, 5));
        assert_eq!(easter(2027), Date::new(2027, 3, 28));
        assert_eq!(easter(2038), Date::new(2038, 4, 25));
    }

    #[test]
    fn thanksgiving_is_the_fourth_thursday() {
        assert_eq!(
            Holiday::Thanksgiving.date(2026),
            Some(Date::new(2026, 11, 26))
        );
        assert_eq!(
            Holiday::Thanksgiving.date(2027),
            Some(Date::new(2027, 11, 25))
        );
        assert_eq!(
            Holiday::Thanksgiving.date(2024),
            Some(Date::new(2024, 11, 28))
        );
    }

    #[test]
    fn lunar_new_year_table() {
        assert_eq!(lunar_new_year(2026), Some(Date::new(2026, 2, 17)));
        assert_eq!(lunar_new_year(2023), None);
        assert_eq!(lunar_new_year(2041), None);
        assert_eq!(zodiac(2026), "Horse");
        assert_eq!(zodiac(2024), "Dragon");
        for y in 2024..=2040 {
            let d = lunar_new_year(y).unwrap();
            assert!(Date::parse(&format!("{y}-{:02}-{:02}", d.month, d.day)).is_some());
            assert!((1..=2).contains(&d.month));
        }
    }

    #[test]
    fn the_day_holds_one_day_either_side() {
        let h = Holiday::Halloween;
        assert_eq!(nearness(h, Date::new(2026, 10, 30)), Some(Nearness::Day));
        assert_eq!(nearness(h, Date::new(2026, 10, 31)), Some(Nearness::Day));
        assert_eq!(nearness(h, Date::new(2026, 11, 1)), Some(Nearness::Day));
        assert_eq!(nearness(h, Date::new(2026, 11, 2)), None);
        assert_eq!(
            nearness(h, Date::new(2026, 10, 22)),
            Some(Nearness::Season { days: 9 })
        );
        assert_eq!(nearness(h, Date::new(2026, 10, 1)), None);
    }

    #[test]
    fn next_crosses_the_year() {
        let (d, n) = Holiday::NewYearsDay.next(Date::new(2026, 12, 30)).unwrap();
        assert_eq!(d, Date::new(2027, 1, 1));
        assert_eq!(n, 2);
        let (d, n) = Holiday::NewYearsEve.next(Date::new(2027, 1, 1)).unwrap();
        assert_eq!(d, Date::new(2026, 12, 31));
        assert_eq!(n, -1);
        assert!(is_day(Holiday::NewYearsEve, Date::new(2027, 1, 1)));
    }

    #[test]
    fn every_holiday_has_a_date_this_year() {
        for h in Holiday::ALL {
            assert!(h.next(Date::new(2026, 6, 1)).is_some(), "{h:?}");
        }
    }

    #[test]
    fn countdown_reads_well() {
        assert_eq!(
            countdown(Holiday::Halloween, Date::new(2026, 10, 22)).unwrap(),
            "Halloween is in 9 days."
        );
        assert_eq!(
            countdown(Holiday::Halloween, Date::new(2026, 10, 30)).unwrap(),
            "Halloween is tomorrow."
        );
        assert_eq!(
            countdown(Holiday::SummerSolstice, Date::new(2026, 6, 21)).unwrap(),
            "Midsummer is today."
        );
    }

    #[test]
    fn seasons() {
        assert_eq!(Season::of(Date::new(2026, 12, 1)), Season::Winter);
        assert_eq!(Season::of(Date::new(2026, 2, 28)), Season::Winter);
        assert_eq!(Season::of(Date::new(2026, 3, 1)), Season::Spring);
        assert_eq!(Season::of(Date::new(2026, 10, 22)), Season::Autumn);
    }

    #[test]
    fn nth_weekday_counts_from_the_first() {
        // Mother's Day 2026: the second Sunday of May.
        assert_eq!(nth_weekday(2026, 5, 6, 2), Date::new(2026, 5, 10));
    }
}
