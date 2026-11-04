//! The real calendar in the tea house: a blend for each season, and a
//! special guest with a special brew on certain holidays.
//!
//! Everything here is a plain function of the date, so tests can ask what
//! is on the shelf on any day of any year.

use hearken::text::sentence_start;
use parlor_core::calendar::{self, nearness, zodiac, Holiday, Nearness, Season};
use parlor_core::{Date, Happening};

use crate::teas::{Tea, HOLIDAY_TEAS, SEASONAL, TEAS};

/// A day the tea house keeps, with its own guest and brew.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Occasion {
    LunarNewYear,
    Valentines,
    SpringEquinox,
    Midsummer,
    AutumnEquinox,
    Halloween,
    Thanksgiving,
}

impl Occasion {
    pub const ALL: [Occasion; 7] = [
        Occasion::LunarNewYear,
        Occasion::Valentines,
        Occasion::SpringEquinox,
        Occasion::Midsummer,
        Occasion::AutumnEquinox,
        Occasion::Halloween,
        Occasion::Thanksgiving,
    ];

    /// The calendar's holidays that count as this day.
    pub fn holidays(self) -> &'static [Holiday] {
        match self {
            Occasion::LunarNewYear => &[Holiday::LunarNewYear],
            Occasion::Valentines => &[Holiday::ValentinesDay],
            Occasion::SpringEquinox => &[Holiday::SpringEquinox],
            Occasion::Midsummer => &[Holiday::SummerSolstice],
            Occasion::AutumnEquinox => &[Holiday::AutumnEquinox],
            Occasion::Halloween => &[Holiday::Halloween],
            Occasion::Thanksgiving => &[Holiday::Thanksgiving],
        }
    }

    /// The holiday the run-up counts toward.
    pub fn main_holiday(self) -> Holiday {
        self.holidays()[0]
    }

    /// A stable id for the saved keepsakes.
    pub fn id(self) -> &'static str {
        match self {
            Occasion::LunarNewYear => "lunar_new_year",
            Occasion::Valentines => "valentines",
            Occasion::SpringEquinox => "spring_equinox",
            Occasion::Midsummer => "midsummer",
            Occasion::AutumnEquinox => "autumn_equinox",
            Occasion::Halloween => "halloween",
            Occasion::Thanksgiving => "thanksgiving",
        }
    }

    pub fn by_id(id: &str) -> Option<Occasion> {
        Occasion::ALL.into_iter().find(|o| o.id() == id)
    }

    /// "Halloween", "the spring equinox", as a player hears it.
    pub fn name(self) -> &'static str {
        self.main_holiday().name()
    }

    /// The day's special brew.
    pub fn tea(self) -> &'static Tea {
        let at = Occasion::ALL
            .iter()
            .position(|&o| o == self)
            .expect("every occasion is listed");
        &HOLIDAY_TEAS[at]
    }

    /// How many cups the day's brew pours. The midnight pot pours one for
    /// each regular, which the guest list works out.
    pub fn cups(self) -> u8 {
        match self {
            Occasion::Valentines => 2,
            _ => 1,
        }
    }

    /// The date of this occasion that `today` falls on or before, with
    /// whole days to go: 0 or less is the day.
    pub fn next(self, today: Date) -> Option<(Date, i64)> {
        self.holidays()
            .iter()
            .filter_map(|h| h.next(today))
            .min_by_key(|&(_, days)| days.abs())
    }

    /// Whether `today` counts as this occasion's day.
    pub fn is_on(self, today: Date) -> bool {
        self.holidays().iter().any(|&h| calendar::is_day(h, today))
    }

    /// The year the occasion belongs to on `today`.
    pub fn year(self, today: Date) -> i32 {
        self.next(today).map(|(d, _)| d.year).unwrap_or(today.year)
    }

    /// The launcher's few words: "Halloween night".
    pub fn title(self) -> &'static str {
        match self {
            Occasion::LunarNewYear => "Lunar New Year tea",
            Occasion::Valentines => "Valentine's tea for two",
            Occasion::SpringEquinox => "Cherry blossom day",
            Occasion::Midsummer => "Midsummer iced mint",
            Occasion::AutumnEquinox => "Equinox barley",
            Occasion::Halloween => "Halloween night",
            Occasion::Thanksgiving => "Thanksgiving apple tea",
        }
    }

    /// One sentence on what is special today.
    pub fn line(self, year: i32) -> String {
        match self {
            Occasion::LunarNewYear => format!(
                "Mei from the bakery brings oolong and tangerines for the year of the {}.",
                zodiac(year)
            ),
            Occasion::Valentines => {
                "An old couple shares a pot of rose tea, poured into two cups.".to_string()
            }
            Occasion::SpringEquinox => {
                "A gardener comes in for cherry blossom tea on the first day of spring.".to_string()
            }
            Occasion::Midsummer => {
                "On the longest day, a boy with a kite asks for iced mint tea.".to_string()
            }
            Occasion::AutumnEquinox => {
                "A farmer from the valley brings roasted barley for the equinox.".to_string()
            }
            Occasion::Halloween => {
                "A cloaked stranger visits the tea house for a smoky cup.".to_string()
            }
            Occasion::Thanksgiving => {
                "A grandmother with a pie stops in for spiced apple tea.".to_string()
            }
        }
    }

    /// The keepsake the day's guest leaves.
    pub fn keepsake(self, year: i32) -> String {
        match self {
            Occasion::LunarNewYear => {
                format!("a paper {} cut from red paper", zodiac(year).to_lowercase())
            }
            Occasion::Valentines => "a pressed rose in wax paper".to_string(),
            Occasion::SpringEquinox => "a cherry blossom bookmark".to_string(),
            Occasion::Midsummer => "a blue ribbon from a kite's tail".to_string(),
            Occasion::AutumnEquinox => "a sheaf of barley tied with string".to_string(),
            Occasion::Halloween => "an old brass key, cold to the touch".to_string(),
            Occasion::Thanksgiving => "a hand-written apple pie recipe".to_string(),
        }
    }

    /// What a regular says in the run-up, after their name.
    pub fn hint(self) -> &'static str {
        match self {
            Occasion::LunarNewYear => {
                "The bakery has red lanterns in the window already. Mei says she will bring you something for the new year."
            }
            Occasion::Valentines => {
                "There is an old couple on my street, fifty years married. They say they will come here for Valentine's Day."
            }
            Occasion::SpringEquinox => {
                "The cherry trees by the river are in bud. Spring is nearly here."
            }
            Occasion::Midsummer => {
                "The evenings are so long now. On midsummer the light hardly leaves at all."
            }
            Occasion::AutumnEquinox => {
                "The valley farms are bringing the barley in. The equinox is close."
            }
            Occasion::Halloween => {
                "The lanes are full of carved pumpkins. They say a cloaked stranger takes tea somewhere in town on Halloween."
            }
            Occasion::Thanksgiving => {
                "My neighbour is baking pies already, for Thanksgiving. The whole stairwell smells of apples."
            }
        }
    }

    /// "Halloween is in 9 days." for the run-up.
    pub fn countdown(self, today: Date) -> Option<String> {
        let (_, days) = self.next(today)?;
        Some(format!(
            "{} is {}.",
            sentence_start(self.name()),
            calendar::days_phrase(days.max(0))
        ))
    }
}

/// The occasion `today` counts as, if any. When two overlap, the one whose
/// date is nearest wins.
pub fn occasion_on(today: Date) -> Option<Occasion> {
    Occasion::ALL
        .into_iter()
        .filter(|o| o.is_on(today))
        .min_by_key(|o| o.next(today).map(|(_, d)| d.abs()).unwrap_or(i64::MAX))
}

/// The occasion whose run-up `today` is in, with the days to go: the
/// nearest one, and never one that is already the day.
pub fn coming(today: Date) -> Option<(Occasion, i64)> {
    Occasion::ALL
        .into_iter()
        .filter(|o| !o.is_on(today))
        .filter_map(|o| match nearness(o.main_holiday(), today) {
            Some(Nearness::Season { days }) => Some((o, days)),
            _ => None,
        })
        .min_by_key(|&(_, days)| days)
}

/// The season's blend for `today`.
pub fn blend(today: Date) -> &'static Tea {
    let at = match Season::of(today) {
        Season::Spring => 0,
        Season::Summer => 1,
        Season::Autumn => 2,
        Season::Winter => 3,
    };
    &SEASONAL[at]
}

/// The shelf on `today`: the starter teas and the ones bought, in the
/// catalogue's order, then the season's blend, then the day's brew.
pub fn shelf(owned: &dyn Fn(&str) -> bool, today: Date) -> Vec<&'static Tea> {
    let mut out: Vec<&'static Tea> = TEAS
        .iter()
        .filter(|t| t.price().is_none() || owned(t.id))
        .collect();
    out.push(blend(today));
    if let Some(o) = occasion_on(today) {
        out.push(o.tea());
    }
    out
}

/// What the launcher names on `date`.
pub fn event(date: Date) -> Option<Happening> {
    let o = occasion_on(date)?;
    Some(Happening::new(o.title(), o.line(o.year(date))))
}
