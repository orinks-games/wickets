//! The real calendar behind the parade: which lunar year a date falls in,
//! its zodiac animal, the run-up to Lunar New Year, the New Year parade on
//! the day itself and the Lantern walk on the fifteenth day of the year.
//!
//! Everything here is a plain function of a [`Date`], so tests can try any
//! day of any year. Lunar New Year comes from the table in
//! `parlor_core::calendar`, which runs from 2024 to 2040; outside it the
//! events are simply not offered.

use parlor_core::calendar::{self, Holiday, Nearness, DAY_SLACK};
use parlor_core::{Date, Happening};

/// The Lantern Festival is the fifteenth day of the new year: New Year's
/// Day plus 14.
pub const LANTERN_OFFSET: i64 = 14;

/// The last year whose Lunar New Year the calendar knows.
pub const LAST_KNOWN_YEAR: i32 = 2040;

/// The twelve animals, in the zodiac's order from the Rat.
pub const ANIMALS: [&str; 12] = [
    "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Goat", "Monkey", "Rooster", "Dog",
    "Pig",
];

/// Where `animal` stands in [`ANIMALS`], from 0.
pub fn animal_index(animal: &str) -> Option<usize> {
    ANIMALS.iter().position(|a| *a == animal)
}

/// The lunar year `date` falls in: the calendar year from its Lunar New
/// Year on, the year before until then. Outside the table the turn of the
/// lunar year is taken as February 5, near the middle of the dates it can
/// fall on, so the animal is right for most of the year.
pub fn lunar_year(date: Date) -> i32 {
    match calendar::lunar_new_year(date.year) {
        Some(start) if date < start => date.year - 1,
        Some(_) => date.year,
        None if (date.month, date.day) < (2, 5) => date.year - 1,
        None => date.year,
    }
}

/// The zodiac animal of the lunar year `date` falls in.
pub fn year_animal(date: Date) -> &'static str {
    calendar::zodiac(lunar_year(date))
}

/// Whether the calendar knows Lunar New Year in `year`.
pub fn known(year: i32) -> bool {
    calendar::lunar_new_year(year).is_some()
}

/// The Lantern Festival in `year`, when its New Year is known.
pub fn lantern_day(year: i32) -> Option<Date> {
    calendar::lunar_new_year(year).map(|d| Date::from_days(d.days() + LANTERN_OFFSET))
}

/// The year whose date from `day_of` counts as `date`, one day either way.
fn day_year(date: Date, day_of: fn(i32) -> Option<Date>) -> Option<i32> {
    (date.year - 1..=date.year + 1)
        .find(|&y| day_of(y).is_some_and(|d| (d.days() - date.days()).abs() <= DAY_SLACK))
}

/// The year whose Lunar New Year `date` counts as, one day either side.
pub fn new_year_on(date: Date) -> Option<i32> {
    day_year(date, calendar::lunar_new_year)
}

/// The year whose Lantern Festival `date` counts as, one day either side.
pub fn lantern_on(date: Date) -> Option<i32> {
    day_year(date, lantern_day)
}

/// The next date from `day_of` that still counts as the day from `date`,
/// with the whole days until it.
fn next_day(date: Date, day_of: fn(i32) -> Option<Date>) -> Option<(Date, i64)> {
    (date.year - 1..=date.year + 1)
        .filter_map(day_of)
        .map(|d| (d, d.days() - date.days()))
        .filter(|&(_, n)| n >= -DAY_SLACK)
        .min_by_key(|&(_, n)| n)
}

/// The next Lunar New Year from `date`, with the days until it.
pub fn next_new_year(date: Date) -> Option<(Date, i64)> {
    next_day(date, calendar::lunar_new_year)
}

/// The next Lantern Festival from `date`, with the days until it.
pub fn next_lantern(date: Date) -> Option<(Date, i64)> {
    next_day(date, lantern_day)
}

/// What the calendar holds for the parade on a date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    /// Lunar New Year, give or take a day: the New Year parade is open.
    NewYear { year: i32 },
    /// The Lantern Festival, give or take a day: the Lantern walk is open.
    Lantern { year: i32 },
    /// The ten days before New Year: lanterns go up, lion-dance drums
    /// join in and red envelopes count double.
    RunUp { days: i64, year: i32 },
    /// Nothing special today.
    Off,
}

/// What `date` holds.
pub fn gate(date: Date) -> Gate {
    if let Some(year) = new_year_on(date) {
        return Gate::NewYear { year };
    }
    if let Some(year) = lantern_on(date) {
        return Gate::Lantern { year };
    }
    if let Some(Nearness::Season { days }) = calendar::nearness(Holiday::LunarNewYear, date) {
        if let Some((d, _)) = next_new_year(date) {
            return Gate::RunUp { days, year: d.year };
        }
    }
    Gate::Off
}

/// Whether `date` is in the run-up to New Year.
pub fn run_up(date: Date) -> bool {
    matches!(gate(date), Gate::RunUp { .. })
}

/// The event the launcher names on `date`, if any.
pub fn event(date: Date) -> Option<Happening> {
    match gate(date) {
        Gate::NewYear { year } => Some(Happening::new(
            "New Year parade",
            format!(
                "The Year of the {} parade is open, ending at midnight firecrackers, with a keepsake for the year.",
                calendar::zodiac(year)
            ),
        )),
        Gate::Lantern { .. } => Some(Happening::new(
            "Lantern walk",
            "A calm lantern walk is open for the Lantern Festival, the fifteenth day of the new year.",
        )),
        Gate::RunUp { days, .. } => Some(Happening::new(
            "Lanterns going up",
            format!(
                "Lion-dance drums join the parade and red envelopes count double. Lunar New Year is {}.",
                calendar::days_phrase(days)
            ),
        )),
        Gate::Off => None,
    }
}

/// "February 17".
pub fn date_words(date: Date) -> String {
    const MONTHS: [&str; 12] = [
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
    ];
    let month = MONTHS[(date.month.clamp(1, 12) - 1) as usize];
    format!("{month} {}", date.day)
}

/// Said when a date is past the calendar's table.
pub fn unknown_words() -> String {
    format!(
        "Lunar New Year dates are known only up to {LAST_KNOWN_YEAR}, so the festival days cannot open after that."
    )
}

/// Why the New Year parade is locked on `date`, and when it opens.
pub fn new_year_locked(date: Date) -> String {
    match next_new_year(date) {
        Some((d, n)) => format!(
            "Opens for Lunar New Year, on {}, {}: the Year of the {}.",
            date_words(d),
            calendar::days_phrase(n.max(0)),
            calendar::zodiac(d.year)
        ),
        None => unknown_words(),
    }
}

/// Why the Lantern walk is locked on `date`, and when it opens.
pub fn lantern_locked(date: Date) -> String {
    match next_lantern(date) {
        Some((d, n)) => format!(
            "Opens for the Lantern Festival, the fifteenth day of the new year, on {}, {}.",
            date_words(d),
            calendar::days_phrase(n.max(0))
        ),
        None => unknown_words(),
    }
}

/// One line on the season, for the Play row's help: empty on an ordinary day.
pub fn season_words(date: Date) -> String {
    match gate(date) {
        Gate::NewYear { year } => format!(
            "Happy New Year: the Year of the {} parade is open.",
            calendar::zodiac(year)
        ),
        Gate::Lantern { .. } => "The Lantern walk is open today.".to_string(),
        Gate::RunUp { days, .. } => format!(
            "Lanterns are up and lion-dance drums play along. Red envelopes count double. Lunar New Year is {}.",
            calendar::days_phrase(days)
        ),
        Gate::Off => String::new(),
    }
}

/// The keepsake a year's New Year parade leaves for its animal.
pub fn keepsake(animal: &str) -> &'static str {
    match animal {
        "Rat" => "a jade rat on a red cord",
        "Ox" => "a brass ox bell",
        "Tiger" => "a pair of tiger-head slippers",
        "Rabbit" => "a rabbit lantern of white paper",
        "Dragon" => "a gold dragon scale",
        "Snake" => "a coiled snake of red silk",
        "Horse" => "a paper-cut galloping horse",
        "Goat" => "a woollen goat with curled horns",
        "Monkey" => "a clay monkey holding a peach",
        "Rooster" => "a painted rooster fan",
        "Dog" => "a little lucky dog charm",
        "Pig" => "a round pig money box",
        _ => "a red paper charm",
    }
}

/// What the year's animal says as the New Year parade begins.
pub fn greeting(animal: &str) -> &'static str {
    match animal {
        "Rat" => {
            "The Rat: Quick feet, bright eyes. The first of the twelve welcomes you to the street."
        }
        "Ox" => "The Ox: Slow and steady, dragon. A patient year carries the heaviest loads.",
        "Tiger" => "The Tiger: Dance boldly. This is a brave year, and the drums know it.",
        "Rabbit" => "The Rabbit: Gently now. A soft step goes a long way in a quiet year.",
        "Dragon" => "The Dragon: Cousin, welcome. Lead the street as only a dragon can.",
        "Snake" => {
            "The Snake: Wind through the lanes with care. A wise year rewards a clever path."
        }
        "Horse" => "The Horse: Gallop on, dragon. The year is wide and the road is open.",
        "Goat" => "The Goat: Take your time. A kind year has room for everyone in the street.",
        "Monkey" => "The Monkey: Ha, a dragon. Show me your tricks, and I will show you mine.",
        "Rooster" => "The Rooster: Up with the sun. A year this bright deserves a proud parade.",
        "Dog" => "The Dog: Faithful friend, the street is safe with you. Lead it home.",
        "Pig" => "The Pig: Plenty for all this year. Dance, eat, and be merry.",
        _ => "A voice from the crowd: Happy New Year.",
    }
}

/// The lanterns the Lantern walk can hang, by number.
pub const LANTERNS: [&str; 12] = [
    "rabbit lantern",
    "lotus lantern",
    "carp lantern",
    "palace lantern",
    "butterfly lantern",
    "moon lantern",
    "star lantern",
    "peony lantern",
    "crane lantern",
    "pagoda lantern",
    "melon lantern",
    "dragon lantern",
];

/// A lantern's name, by number.
pub fn lantern_name(kind: usize) -> &'static str {
    LANTERNS[kind % LANTERNS.len()]
}

/// "the Year of the Horse", for the middle of a sentence; wrap it in
/// `sentence_start` to open one.
pub fn year_of(animal: &str) -> String {
    format!("the Year of the {animal}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lunar_year_turns_on_new_years_day() {
        // 2026 begins on February 17: the Snake holds until then.
        assert_eq!(lunar_year(Date::new(2026, 2, 16)), 2025);
        assert_eq!(year_animal(Date::new(2026, 2, 16)), "Snake");
        assert_eq!(lunar_year(Date::new(2026, 2, 17)), 2026);
        assert_eq!(year_animal(Date::new(2026, 2, 17)), "Horse");
        // The calendar year turns first, the lunar year later.
        assert_eq!(year_animal(Date::new(2025, 12, 31)), "Snake");
        assert_eq!(year_animal(Date::new(2026, 1, 1)), "Snake");
        // 2025 began on January 29.
        assert_eq!(year_animal(Date::new(2025, 1, 28)), "Dragon");
        assert_eq!(year_animal(Date::new(2025, 1, 29)), "Snake");
        // Before the table's first year, the next year's date still decides.
        assert_eq!(year_animal(Date::new(2024, 1, 15)), "Rabbit");
        assert_eq!(year_animal(Date::new(2024, 2, 10)), "Dragon");
    }

    #[test]
    fn every_known_year_names_its_animal_from_its_first_day() {
        for y in 2024..=LAST_KNOWN_YEAR {
            let d = calendar::lunar_new_year(y).unwrap();
            assert_eq!(lunar_year(d), y);
            assert_eq!(lunar_year(Date::from_days(d.days() - 1)), y - 1);
            assert_eq!(year_animal(d), calendar::zodiac(y));
        }
        assert_eq!(year_animal(Date::new(2030, 6, 1)), "Dog");
        assert_eq!(year_animal(Date::new(2040, 6, 1)), "Monkey");
    }

    #[test]
    fn years_past_the_table_guess_the_animal_and_offer_nothing() {
        assert!(!known(2041));
        assert_eq!(year_animal(Date::new(2041, 6, 1)), "Rooster");
        assert_eq!(year_animal(Date::new(2041, 1, 20)), "Monkey");
        for (m, d) in [(1, 20), (2, 1), (2, 14), (3, 1), (12, 31)] {
            let date = Date::new(2041, m, d);
            assert_eq!(gate(date), Gate::Off, "{date:?}");
            assert_eq!(event(date), None);
        }
        assert_eq!(new_year_locked(Date::new(2041, 3, 1)), unknown_words());
        assert_eq!(lantern_locked(Date::new(2041, 3, 1)), unknown_words());
        // After the last known Lantern Festival, nothing is next.
        assert_eq!(next_new_year(Date::new(2040, 3, 1)), None);
        assert_eq!(next_lantern(Date::new(2040, 3, 1)), None);
        assert!(new_year_locked(Date::new(2040, 3, 1)).contains("2040"));
    }

    #[test]
    fn new_years_day_counts_one_day_either_side() {
        let day = Date::new(2026, 2, 17);
        for d in [-1, 0, 1] {
            let date = Date::from_days(day.days() + d);
            assert_eq!(gate(date), Gate::NewYear { year: 2026 }, "{date:?}");
        }
        assert_eq!(
            gate(Date::new(2026, 2, 15)),
            Gate::RunUp {
                days: 2,
                year: 2026
            }
        );
        assert_eq!(gate(Date::new(2026, 2, 19)), Gate::Off);
        // 2028 begins on January 26; its eve falls on the 25th.
        assert_eq!(new_year_on(Date::new(2028, 1, 25)), Some(2028));
    }

    #[test]
    fn the_run_up_is_ten_days() {
        let day = Date::new(2027, 2, 6);
        assert_eq!(
            gate(Date::from_days(day.days() - 10)),
            Gate::RunUp {
                days: 10,
                year: 2027
            }
        );
        assert_eq!(gate(Date::from_days(day.days() - 11)), Gate::Off);
        assert!(run_up(Date::new(2027, 2, 1)));
        assert!(!run_up(Date::new(2027, 2, 6)));
    }

    #[test]
    fn the_run_up_near_the_turn_of_the_calendar_year() {
        // 2031 begins on January 23, so its run-up starts on January 13.
        assert_eq!(
            gate(Date::new(2031, 1, 13)),
            Gate::RunUp {
                days: 10,
                year: 2031
            }
        );
        // New Year's Eve of the calendar is still too far out.
        assert_eq!(gate(Date::new(2030, 12, 31)), Gate::Off);
        // The eve of 2039's January 24 counts as the day.
        assert_eq!(new_year_on(Date::new(2039, 1, 23)), Some(2039));
        // Near the end of a year, the next festival is in the next year.
        let (d, n) = next_new_year(Date::new(2026, 12, 30)).unwrap();
        assert_eq!(d, Date::new(2027, 2, 6));
        assert_eq!(n, 38);
    }

    #[test]
    fn the_lantern_festival_is_the_fifteenth_day() {
        assert_eq!(lantern_day(2026), Some(Date::new(2026, 3, 3)));
        assert_eq!(lantern_day(2025), Some(Date::new(2025, 2, 12)));
        assert_eq!(lantern_day(2024), Some(Date::new(2024, 2, 24)));
        assert_eq!(lantern_day(2041), None);
        assert_eq!(gate(Date::new(2026, 3, 2)), Gate::Lantern { year: 2026 });
        assert_eq!(gate(Date::new(2026, 3, 4)), Gate::Lantern { year: 2026 });
        assert_eq!(gate(Date::new(2026, 3, 5)), Gate::Off);
        // A late Lantern Festival, in March.
        assert_eq!(gate(Date::new(2034, 3, 5)), Gate::Lantern { year: 2034 });
    }

    #[test]
    fn events_name_the_day() {
        let ny = event(Date::new(2026, 2, 17)).unwrap();
        assert_eq!(ny.title, "New Year parade");
        assert!(ny.line.contains("Year of the Horse"));
        let lw = event(Date::new(2026, 3, 3)).unwrap();
        assert_eq!(lw.title, "Lantern walk");
        let up = event(Date::new(2026, 2, 10)).unwrap();
        assert_eq!(up.title, "Lanterns going up");
        assert!(up.line.ends_with("Lunar New Year is in 7 days."));
        assert_eq!(event(Date::new(2026, 10, 22)), None);
    }

    #[test]
    fn locked_rows_count_down() {
        assert_eq!(
            new_year_locked(Date::new(2026, 10, 22)),
            "Opens for Lunar New Year, on February 6, in 107 days: the Year of the Goat."
        );
        assert_eq!(
            lantern_locked(Date::new(2026, 10, 22)),
            "Opens for the Lantern Festival, the fifteenth day of the new year, on February 20, in 121 days."
        );
        assert!(new_year_locked(Date::new(2027, 2, 5)).contains("tomorrow"));
    }

    #[test]
    fn every_animal_has_a_keepsake_and_a_greeting() {
        let mut seen = std::collections::HashSet::new();
        for a in ANIMALS {
            assert!(seen.insert(keepsake(a)), "{a}");
            assert!(greeting(a).starts_with(&format!("The {a}: ")), "{a}");
            assert_eq!(ANIMALS[animal_index(a).unwrap()], a);
        }
        for y in 2020..2032 {
            assert!(animal_index(calendar::zodiac(y)).is_some());
        }
    }
}
