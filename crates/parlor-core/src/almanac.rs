//! The parlor's almanac: what the sky and the calendar are doing today,
//! for any game to build an event on.
//!
//! [`crate::calendar`] knows the holidays. This module adds the rest of
//! the year: the moon's phases and named full moons, the meteor showers,
//! a few odd days (Friday the 13th, Leap Day), and [`Window`], a span of
//! days a game's event is open. Every function is a plain function of a
//! [`Date`], so tests pin dates and every player sees the same almanac.
//!
//! A game builds its `INFO.event` from these, and the story mode reads
//! [`omens`] to pick games that suit the day.

use crate::calendar::{self, Holiday, Nearness};
use crate::Date;

/// The moon's mean synodic month, in days.
pub const SYNODIC_MONTH: f64 = 29.530_588_853;

/// A new moon to count from: 6 January 2000 at 18:14 UTC, in days since
/// 1970-01-01.
fn reference_new_moon() -> f64 {
    Date::new(2000, 1, 6).days() as f64 + (18.0 + 14.0 / 60.0) / 24.0
}

/// The moon's age at noon UTC on `date`, in days since new moon.
pub fn moon_age(date: Date) -> f64 {
    (date.days() as f64 + 0.5 - reference_new_moon()).rem_euclid(SYNODIC_MONTH)
}

/// Days from noon on `date` to the nearest mean full moon: negative when
/// it has passed.
pub fn days_to_full(date: Date) -> f64 {
    SYNODIC_MONTH / 2.0 - moon_age(date)
}

/// How far from the mean full or new moon still counts as that night. The
/// true moon wanders up to about 14 hours from the mean one, so a day
/// either side holds it.
pub const MOON_SLACK: f64 = 1.0;

/// The moon's phase, in eight steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    New,
    WaxingCrescent,
    FirstQuarter,
    WaxingGibbous,
    Full,
    WaningGibbous,
    LastQuarter,
    WaningCrescent,
}

impl Phase {
    /// "a new moon", "a waxing crescent", "a full moon".
    pub fn words(self) -> &'static str {
        match self {
            Phase::New => "a new moon",
            Phase::WaxingCrescent => "a waxing crescent moon",
            Phase::FirstQuarter => "a first quarter moon",
            Phase::WaxingGibbous => "a waxing gibbous moon",
            Phase::Full => "a full moon",
            Phase::WaningGibbous => "a waning gibbous moon",
            Phase::LastQuarter => "a last quarter moon",
            Phase::WaningCrescent => "a waning crescent moon",
        }
    }

    /// How lit the moon is, 0 at new to 1 at full.
    pub fn light(self) -> f32 {
        match self {
            Phase::New => 0.0,
            Phase::WaxingCrescent | Phase::WaningCrescent => 0.25,
            Phase::FirstQuarter | Phase::LastQuarter => 0.5,
            Phase::WaxingGibbous | Phase::WaningGibbous => 0.75,
            Phase::Full => 1.0,
        }
    }
}

/// The moon's phase on `date`. New and Full hold for a day either side,
/// to match [`full_moon`] and [`is_new_moon`].
pub fn phase(date: Date) -> Phase {
    let age = moon_age(date);
    let half = SYNODIC_MONTH / 2.0;
    if age <= MOON_SLACK || age >= SYNODIC_MONTH - MOON_SLACK {
        return Phase::New;
    }
    if (age - half).abs() <= MOON_SLACK {
        return Phase::Full;
    }
    let eighth = SYNODIC_MONTH / 8.0;
    match ((age / eighth).round() as i64).rem_euclid(8) {
        0 | 1 if age < half => Phase::WaxingCrescent,
        2 => Phase::FirstQuarter,
        3 | 4 if age < half => Phase::WaxingGibbous,
        4 | 5 => Phase::WaningGibbous,
        6 => Phase::LastQuarter,
        _ => Phase::WaningCrescent,
    }
}

/// Whether `date` is a new moon night.
pub fn is_new_moon(date: Date) -> bool {
    phase(date) == Phase::New
}

/// The day of the new moon that `date` belongs to, on a new moon night,
/// so the nights of one new moon can share a seed.
pub fn new_moon_day(date: Date) -> Option<Date> {
    if !is_new_moon(date) {
        return None;
    }
    let age = moon_age(date);
    let offset = if age > SYNODIC_MONTH / 2.0 {
        SYNODIC_MONTH - age
    } else {
        -age
    };
    Some(Date::from_days(
        (date.days() as f64 + 0.5 + offset).floor() as i64
    ))
}

/// A full moon night.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FullMoon {
    /// "the Hunter's Moon".
    pub name: &'static str,
    /// The month it is named for, 1 to 12.
    pub month: u32,
    /// The second full moon of a calendar month.
    pub blue: bool,
}

/// The traditional name of a month's full moon.
pub fn moon_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "the Wolf Moon",
        "the Snow Moon",
        "the Worm Moon",
        "the Pink Moon",
        "the Flower Moon",
        "the Strawberry Moon",
        "the Buck Moon",
        "the Sturgeon Moon",
        "the Harvest Moon",
        "the Hunter's Moon",
        "the Beaver Moon",
        "the Cold Moon",
    ];
    NAMES[(month.clamp(1, 12) - 1) as usize]
}

/// The full moon, when `date` is a full moon night.
pub fn full_moon(date: Date) -> Option<FullMoon> {
    let to_full = days_to_full(date);
    if to_full.abs() > MOON_SLACK {
        return None;
    }
    let full = Date::from_days((date.days() as f64 + 0.5 + to_full).floor() as i64);
    let before =
        Date::from_days((date.days() as f64 + 0.5 + to_full - SYNODIC_MONTH).floor() as i64);
    let blue = before.year == full.year && before.month == full.month;
    Some(FullMoon {
        name: if blue {
            "the Blue Moon"
        } else {
            moon_name(full.month)
        },
        month: full.month,
        blue,
    })
}

/// The next day (from `today`, counting today) on which `test` holds, and
/// the days until it, looking up to `within` days ahead.
pub fn next_day(today: Date, within: i64, test: impl Fn(Date) -> bool) -> Option<(Date, i64)> {
    (0..=within)
        .map(|n| (Date::from_days(today.days() + n), n))
        .find(|&(d, _)| test(d))
}

/// The next full moon night from `today`, and the days until it.
pub fn next_full_moon(today: Date) -> (Date, i64) {
    next_day(today, 40, |d| full_moon(d).is_some()).unwrap_or((today, 0))
}

/// The yearly meteor showers worth staying up for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shower {
    Quadrantids,
    Lyrids,
    EtaAquariids,
    Perseids,
    Orionids,
    Leonids,
    Geminids,
}

impl Shower {
    pub const ALL: [Shower; 7] = [
        Shower::Quadrantids,
        Shower::Lyrids,
        Shower::EtaAquariids,
        Shower::Perseids,
        Shower::Orionids,
        Shower::Leonids,
        Shower::Geminids,
    ];

    /// "the Perseids".
    pub fn name(self) -> &'static str {
        match self {
            Shower::Quadrantids => "the Quadrantids",
            Shower::Lyrids => "the Lyrids",
            Shower::EtaAquariids => "the Eta Aquariids",
            Shower::Perseids => "the Perseids",
            Shower::Orionids => "the Orionids",
            Shower::Leonids => "the Leonids",
            Shower::Geminids => "the Geminids",
        }
    }

    /// The usual peak night, month and day.
    pub fn peak_day(self) -> (u32, u32) {
        match self {
            Shower::Quadrantids => (1, 3),
            Shower::Lyrids => (4, 22),
            Shower::EtaAquariids => (5, 6),
            Shower::Perseids => (8, 12),
            Shower::Orionids => (10, 21),
            Shower::Leonids => (11, 17),
            Shower::Geminids => (12, 14),
        }
    }
}

/// The shower peaking on `date`, a day either side.
pub fn shower_peak(date: Date) -> Option<Shower> {
    Shower::ALL.into_iter().find(|s| {
        let (m, d) = s.peak_day();
        (Date::new(date.year, m, d).days() - date.days()).abs() <= 1
    })
}

/// Something the almanac marks on a day: a story mode or a game can build
/// on any of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Omen {
    FullMoon(FullMoon),
    NewMoon,
    Meteors(Shower),
    /// The longest or shortest day, or an equinox.
    Solstice(Holiday),
    FridayThe13th,
    LeapDay,
    /// A holiday from [`calendar`], on the day.
    Holiday(Holiday),
}

impl Omen {
    /// A few words for a line: "the Hunter's Moon", "Friday the 13th".
    pub fn name(self) -> String {
        match self {
            Omen::FullMoon(m) => m.name.to_string(),
            Omen::NewMoon => "the new moon".to_string(),
            Omen::Meteors(s) => s.name().to_string(),
            Omen::Solstice(h) | Omen::Holiday(h) => h.name().to_string(),
            Omen::FridayThe13th => "Friday the 13th".to_string(),
            Omen::LeapDay => "Leap Day".to_string(),
        }
    }
}

fn is_turning_point(h: Holiday) -> bool {
    matches!(
        h,
        Holiday::SpringEquinox
            | Holiday::SummerSolstice
            | Holiday::AutumnEquinox
            | Holiday::WinterSolstice
    )
}

/// Every omen on `date`, holidays first, then the sky, then odd days.
pub fn omens(date: Date) -> Vec<Omen> {
    let mut out = Vec::new();
    for h in Holiday::ALL {
        if calendar::is_day(h, date) {
            out.push(if is_turning_point(h) {
                Omen::Solstice(h)
            } else {
                Omen::Holiday(h)
            });
        }
    }
    if let Some(s) = shower_peak(date) {
        out.push(Omen::Meteors(s));
    }
    if let Some(m) = full_moon(date) {
        out.push(Omen::FullMoon(m));
    } else if is_new_moon(date) {
        out.push(Omen::NewMoon);
    }
    if date.day == 13 && date.weekday() == 4 {
        out.push(Omen::FridayThe13th);
    }
    if date.month == 2 && date.day == 29 {
        out.push(Omen::LeapDay);
    }
    out
}

/// A span of days a game's event is open, the same every year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    /// A holiday's day (one day either side), as [`calendar::is_day`].
    Day(Holiday),
    /// A holiday's run-up and day, as [`calendar::nearness`].
    RunUp(Holiday),
    /// From one month and day to another, inclusive. It may wrap the new
    /// year: `Range((12, 20), (1, 6))`.
    Range((u32, u32), (u32, u32)),
    /// Whole months, inclusive, possibly wrapping: `Months(11, 2)`.
    Months(u32, u32),
    /// Any full moon night.
    FullMoon,
    /// Any new moon night.
    NewMoon,
    /// A shower's peak, a day either side.
    Meteors(Shower),
    /// One weekday a month: `Monthly(Week::First, 1)` is the first
    /// Tuesday. Weekdays count from Monday, 0, as [`Date::weekday`].
    Monthly(Week, u32),
    /// One weekday of one month, every year: `Yearly(Week::Third, 6, 7)`
    /// is the third Sunday of July. Weekdays count from Monday, 0.
    Yearly(Week, u32, u32),
    /// One day of every month: `DayOfMonth(1)` is the 1st.
    DayOfMonth(u32),
    /// The month's last weekend: its last Saturday and the Sunday after,
    /// which may fall in the next month.
    LastWeekend,
}

/// Which week of the month a [`Window::Monthly`] day falls in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Week {
    First,
    Second,
    Third,
    Fourth,
    Last,
}

impl Week {
    /// "first", "second", "last".
    pub fn name(self) -> &'static str {
        match self {
            Week::First => "first",
            Week::Second => "second",
            Week::Third => "third",
            Week::Fourth => "fourth",
            Week::Last => "last",
        }
    }

    /// Whether `date` is in this week of its month, counting each
    /// weekday on its own: the 8th to the 14th are all second.
    pub fn holds(self, date: Date) -> bool {
        let nth = (date.day - 1) / 7;
        match self {
            Week::First => nth == 0,
            Week::Second => nth == 1,
            Week::Third => nth == 2,
            Week::Fourth => nth == 3,
            Week::Last => Date::from_days(date.days() + 7).month != date.month,
        }
    }
}

/// "Monday" to "Sunday", Monday as 0.
pub fn weekday_name(weekday: u32) -> &'static str {
    [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ][weekday as usize % 7]
}

/// A month name.
pub fn month_name(month: u32) -> &'static str {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "Month 12",
    ][(month as usize + 11) % 12]
}

/// "Tuesday 3 November".
pub fn date_words(date: Date) -> String {
    format!(
        "{} {} {}",
        weekday_name(date.weekday()),
        date.day,
        month_name(date.month)
    )
}

/// "1st", "2nd", "23rd".
fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn md(m: u32, d: u32) -> u32 {
    m * 100 + d
}

fn within(x: u32, from: u32, to: u32) -> bool {
    if from <= to {
        (from..=to).contains(&x)
    } else {
        x >= from || x <= to
    }
}

impl Window {
    /// Whether the window is open on `date`.
    pub fn is_open(self, date: Date) -> bool {
        match self {
            Window::Day(h) => calendar::is_day(h, date),
            Window::RunUp(h) => calendar::nearness(h, date).is_some(),
            Window::Range((m1, d1), (m2, d2)) => {
                within(md(date.month, date.day), md(m1, d1), md(m2, d2))
            }
            Window::Months(a, b) => within(date.month, a, b),
            Window::FullMoon => full_moon(date).is_some(),
            Window::NewMoon => is_new_moon(date),
            Window::Meteors(s) => shower_peak(date) == Some(s),
            Window::Monthly(week, weekday) => date.weekday() == weekday && week.holds(date),
            Window::Yearly(week, weekday, month) => {
                date.month == month && date.weekday() == weekday && week.holds(date)
            }
            Window::DayOfMonth(day) => date.day == day,
            Window::LastWeekend => match date.weekday() {
                5 => Week::Last.holds(date),
                6 => Week::Last.holds(Date::from_days(date.days() - 1)),
                _ => false,
            },
        }
    }

    /// How often it comes round, for help: "on the first Tuesday of every
    /// month". Only for the monthly windows; others say "on its nights".
    pub fn every(self) -> String {
        match self {
            Window::Monthly(week, weekday) => format!(
                "on the {} {} of every month",
                week.name(),
                weekday_name(weekday)
            ),
            Window::Yearly(week, weekday, month) => format!(
                "on the {} {} of {}",
                week.name(),
                weekday_name(weekday),
                month_name(month)
            ),
            Window::DayOfMonth(day) => format!("on the {} of every month", ordinal(day)),
            Window::LastWeekend => "on the last weekend of every month".to_string(),
            Window::FullMoon => "on every full moon".to_string(),
            Window::NewMoon => "on every new moon".to_string(),
            _ => "on its nights".to_string(),
        }
    }

    /// When it next opens, for a locked row: "Tuesday 3 November, in 9
    /// days", or "today" when open.
    pub fn next_words(self, today: Date) -> String {
        match self.next_open(today) {
            Some((_, 0)) => "today".to_string(),
            Some((day, days)) => format!("{}, {}", date_words(day), calendar::days_phrase(days)),
            None => "later".to_string(),
        }
    }

    /// The next day the window opens, from `today` (today itself when it
    /// is open), and the days until then.
    pub fn next_open(self, today: Date) -> Option<(Date, i64)> {
        next_day(today, 800, |d| self.is_open(d))
    }

    /// "tomorrow", "in 12 days", or "today" when open.
    pub fn when(self, today: Date) -> String {
        match self.next_open(today) {
            Some((_, days)) => calendar::days_phrase(days),
            None => "later".to_string(),
        }
    }

    /// The day a [`Window::Yearly`] rule falls on in `year`; `None` for
    /// the other windows.
    pub fn date_in(self, year: i32) -> Option<Date> {
        let Window::Yearly(_, _, month) = self else {
            return None;
        };
        let first = Date::new(year, month, 1).days();
        (0..31)
            .map(|n| Date::from_days(first + n))
            .take_while(|d| d.month == month)
            .find(|&d| self.is_open(d))
    }

    /// The days left while open, counting today: 1 on the last day.
    pub fn days_left(self, today: Date) -> Option<i64> {
        if !self.is_open(today) {
            return None;
        }
        Some(
            (1..400)
                .find(|&n| !self.is_open(Date::from_days(today.days() + n)))
                .unwrap_or(400),
        )
    }
}

/// Whether a holiday is in its run-up (not yet the day) on `date`.
pub fn in_run_up(holiday: Holiday, date: Date) -> bool {
    matches!(
        calendar::nearness(holiday, date),
        Some(Nearness::Season { .. })
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> Date {
        Date::new(y, m, day)
    }

    #[test]
    fn full_moons_match_known_dates() {
        // Full moons: 25 Nov 2026 (approx), 3 Jan 2026, 1 Aug 2026.
        for date in [d(2026, 1, 3), d(2026, 8, 28), d(2026, 10, 26)] {
            assert!(full_moon(date).is_some(), "{date:?}");
            assert_eq!(phase(date), Phase::Full);
        }
        assert_eq!(
            full_moon(d(2026, 10, 26)).unwrap().name,
            "the Hunter's Moon"
        );
        assert!(full_moon(d(2026, 10, 12)).is_none());
    }

    #[test]
    fn new_moons_and_phases_cycle() {
        assert!(is_new_moon(d(2026, 10, 10)));
        let seen: std::collections::HashSet<_> = (0..30)
            .map(|n| phase(Date::from_days(d(2026, 1, 1).days() + n)))
            .collect();
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn omens_name_odd_days() {
        assert!(omens(d(2026, 11, 13)).contains(&Omen::FridayThe13th));
        assert!(omens(d(2028, 2, 29)).contains(&Omen::LeapDay));
        assert!(omens(d(2026, 8, 12)).contains(&Omen::Meteors(Shower::Perseids)));
        assert!(omens(d(2026, 12, 21))
            .iter()
            .any(|o| matches!(o, Omen::Solstice(Holiday::WinterSolstice))));
        assert!(omens(d(2026, 10, 31)).contains(&Omen::Holiday(Holiday::Halloween)));
    }

    #[test]
    fn windows_open_and_wrap() {
        let w = Window::Range((12, 20), (1, 6));
        assert!(w.is_open(d(2026, 12, 31)));
        assert!(w.is_open(d(2027, 1, 6)));
        assert!(!w.is_open(d(2027, 1, 7)));
        assert_eq!(w.next_open(d(2027, 12, 18)).unwrap().1, 2);
        assert_eq!(w.days_left(d(2027, 1, 5)), Some(2));
        assert!(Window::Months(11, 2).is_open(d(2027, 2, 10)));
        assert!(!Window::Months(11, 2).is_open(d(2027, 3, 1)));
        assert!(Window::RunUp(Holiday::Halloween).is_open(d(2026, 10, 22)));
        assert_eq!(Window::FullMoon.when(d(2026, 10, 26)), "today");
    }

    #[test]
    fn every_window_kind_opens_within_a_year() {
        let start = d(2026, 1, 1);
        for w in [
            Window::FullMoon,
            Window::NewMoon,
            Window::Meteors(Shower::Geminids),
            Window::Day(Holiday::Easter),
            Window::Range((6, 1), (6, 3)),
            Window::Monthly(Week::Last, 6),
            Window::Yearly(Week::Third, 6, 7),
            Window::DayOfMonth(31),
            Window::LastWeekend,
        ] {
            assert!(w.next_open(start).unwrap().1 < 366, "{w:?}");
        }
    }

    #[test]
    fn monthly_windows_open_once_a_month() {
        // November 2026 starts on a Sunday.
        let first_tuesday = Window::Monthly(Week::First, 1);
        assert!(first_tuesday.is_open(d(2026, 11, 3)));
        assert!(!first_tuesday.is_open(d(2026, 11, 10)));
        assert!(Window::Monthly(Week::Second, 2).is_open(d(2026, 11, 11)));
        assert!(Window::Monthly(Week::Third, 4).is_open(d(2026, 11, 20)));
        assert!(Window::Monthly(Week::Last, 0).is_open(d(2026, 11, 30)));
        assert!(!Window::Monthly(Week::Last, 0).is_open(d(2026, 11, 23)));
        assert!(Window::DayOfMonth(1).is_open(d(2026, 12, 1)));
        // October 2026's last Saturday is the 31st: its Sunday is 1 November.
        assert!(Window::LastWeekend.is_open(d(2026, 10, 31)));
        assert!(Window::LastWeekend.is_open(d(2026, 11, 1)));
        assert!(!Window::LastWeekend.is_open(d(2026, 10, 25)));
        for w in [first_tuesday, Window::DayOfMonth(1), Window::LastWeekend] {
            let year: Vec<_> = (0..365)
                .map(|n| Date::from_days(d(2027, 1, 1).days() + n))
                .filter(|&day| w.is_open(day))
                .collect();
            let nights = if w == Window::LastWeekend { 24 } else { 12 };
            assert_eq!(year.len(), nights, "{w:?}");
        }
    }

    #[test]
    fn monthly_windows_say_when() {
        let w = Window::Monthly(Week::First, 1);
        assert_eq!(w.every(), "on the first Tuesday of every month");
        assert_eq!(
            w.next_words(d(2026, 10, 25)),
            "Tuesday 3 November, in 9 days"
        );
        assert_eq!(w.next_words(d(2026, 11, 3)), "today");
        assert_eq!(Window::DayOfMonth(1).every(), "on the 1st of every month");
        assert_eq!(Window::DayOfMonth(22).every(), "on the 22nd of every month");
    }

    #[test]
    fn yearly_windows_find_the_third_sunday_of_july() {
        let ice_cream_day = Window::Yearly(Week::Third, 6, 7);
        // National Ice Cream Days, with the early edge (the 15th, when 1
        // July is a Sunday) and the late edge (the 21st, when it is a
        // Monday).
        for (y, day) in [
            (2024, 21),
            (2025, 20),
            (2026, 19),
            (2027, 18),
            (2028, 16),
            (2029, 15),
            (2030, 21),
            (2031, 20),
            (2032, 18),
            (2033, 17),
        ] {
            assert_eq!(ice_cream_day.date_in(y), Some(d(y, 7, day)), "{y}");
            assert!(ice_cream_day.is_open(d(y, 7, day)), "{y}");
            assert!(!ice_cream_day.is_open(d(y, 7, day - 7)), "{y}");
            assert!(!ice_cream_day.is_open(d(y, 7, day + 7)), "{y}");
        }
        assert_eq!(Window::Monthly(Week::Third, 3).date_in(2027), None);
    }

    #[test]
    fn yearly_windows_open_once_a_year() {
        let w = Window::Yearly(Week::Third, 6, 7);
        for y in 2026..=2125 {
            let start = d(y, 1, 1).days();
            let open: Vec<Date> = (0..)
                .map(|n| Date::from_days(start + n))
                .take_while(|day| day.year == y)
                .filter(|&day| w.is_open(day))
                .collect();
            assert_eq!(open.len(), 1, "{y}");
            let day = open[0];
            assert_eq!(day.weekday(), 6, "{y}");
            assert_eq!(day.month, 7, "{y}");
            assert!((15..=21).contains(&day.day), "{y}");
            assert_eq!(w.date_in(y), Some(day), "{y}");
        }
    }

    #[test]
    fn last_week_yearly() {
        // Memorial Day: the last Monday of May.
        let w = Window::Yearly(Week::Last, 0, 5);
        for y in 2026..=2060 {
            let day = w.date_in(y).unwrap();
            assert_eq!(day.weekday(), 0, "{y}");
            assert_eq!(day.month, 5, "{y}");
            assert!((25..=31).contains(&day.day), "{y}");
        }
    }

    #[test]
    fn yearly_says_when() {
        let w = Window::Yearly(Week::Third, 6, 7);
        assert_eq!(w.every(), "on the third Sunday of July");
        let today = d(2026, 10, 30);
        let days = d(2027, 7, 18).days() - today.days();
        assert_eq!(
            w.next_words(today),
            format!("Sunday 18 July, in {days} days")
        );
        assert_eq!(w.next_words(d(2027, 7, 18)), "today");
    }

    #[test]
    fn a_one_day_range_opens_once_a_year() {
        // Peanut Butter Day, 24 January.
        let w = Window::Range((1, 24), (1, 24));
        for y in 2027..=2126 {
            let start = d(y, 1, 1).days();
            let open: Vec<Date> = (0..)
                .map(|n| Date::from_days(start + n))
                .take_while(|day| day.year == y)
                .filter(|&day| w.is_open(day))
                .collect();
            assert_eq!(open, vec![d(y, 1, 24)], "{y}");
        }
    }
}
