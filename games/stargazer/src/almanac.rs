//! The real calendar of the sky: which season's stars are up, the year's
//! meteor showers on their peak nights, and the full moon.
//!
//! Everything here is a plain function of a [`Date`], so tests can walk it
//! across years. Dates are UTC like the rest of the parlor; a peak counts
//! from one day before to one day after, which holds the local date in
//! every time zone.

use parlor_core::calendar::days_phrase;
use parlor_core::{Date, Happening, Season};

/// The season whose evening sky is up on `date`.
pub fn sky_season(date: Date) -> Season {
    Season::of(date)
}

/// The season's sky by month alone, 1 to 12.
pub fn season_of_month(month: u32) -> Season {
    sky_season(Date::new(2000, month.clamp(1, 12), 1))
}

/// The year's great meteor showers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shower {
    Quadrantids,
    Lyrids,
    EtaAquariids,
    Perseids,
    Orionids,
    Leonids,
}

/// Days either side of a peak that still count as the peak night.
pub const PEAK_SLACK: i64 = 1;
/// The run-up: from this many days before the peak, a few early meteors.
pub const RUN_UP: i64 = 4;

impl Shower {
    pub const ALL: [Shower; 6] = [
        Shower::Quadrantids,
        Shower::Lyrids,
        Shower::EtaAquariids,
        Shower::Perseids,
        Shower::Orionids,
        Shower::Leonids,
    ];

    /// The saved id: "perseids".
    pub fn id(self) -> &'static str {
        match self {
            Shower::Quadrantids => "quadrantids",
            Shower::Lyrids => "lyrids",
            Shower::EtaAquariids => "eta_aquariids",
            Shower::Perseids => "perseids",
            Shower::Orionids => "orionids",
            Shower::Leonids => "leonids",
        }
    }

    /// "Perseids".
    pub fn name(self) -> &'static str {
        match self {
            Shower::Quadrantids => "Quadrantids",
            Shower::Lyrids => "Lyrids",
            Shower::EtaAquariids => "Eta Aquariids",
            Shower::Perseids => "Perseids",
            Shower::Orionids => "Orionids",
            Shower::Leonids => "Leonids",
        }
    }

    /// "Perseid", as in "Perseid meteor shower".
    pub fn adjective(self) -> &'static str {
        match self {
            Shower::Quadrantids => "Quadrantid",
            Shower::Lyrids => "Lyrid",
            Shower::EtaAquariids => "Eta Aquariid",
            Shower::Perseids => "Perseid",
            Shower::Orionids => "Orionid",
            Shower::Leonids => "Leonid",
        }
    }

    /// "Perseid meteor shower".
    pub fn title(self) -> String {
        format!("{} meteor shower", self.adjective())
    }

    /// The peak's month and day, the same every year.
    pub fn peak_day(self) -> (u32, u32) {
        match self {
            Shower::Quadrantids => (1, 3),
            Shower::Lyrids => (4, 22),
            Shower::EtaAquariids => (5, 6),
            Shower::Perseids => (8, 12),
            Shower::Orionids => (10, 21),
            Shower::Leonids => (11, 17),
        }
    }

    pub fn peak(self, year: i32) -> Date {
        let (m, d) = self.peak_day();
        Date::new(year, m, d)
    }

    /// The peak as words: "12 August".
    pub fn peak_words(self) -> String {
        let (m, d) = self.peak_day();
        format!("{d} {}", month_name(m))
    }

    /// Where its shooting stars seem to come from.
    pub fn radiant(self) -> &'static str {
        match self {
            Shower::Quadrantids => "the northeast, below the handle of the Big Dipper",
            Shower::Lyrids => "the east, near bright Vega in Lyra",
            Shower::EtaAquariids => "low in the east, in Aquarius",
            Shower::Perseids => "the northeast, in Perseus",
            Shower::Orionids => "the east, beside Orion's raised club",
            Shower::Leonids => "the east, in the Sickle of Leo",
        }
    }

    /// A little of its story, for the shower's start and its Tab key.
    pub fn lore(self) -> &'static str {
        match self {
            Shower::Quadrantids => "The Quadrantids are named after a constellation that is no longer on the star maps, Quadrans Muralis, the wall quadrant. Their peak is short and sharp, only a few hours.",
            Shower::Lyrids => "The Lyrids are one of the oldest showers on record. Watchers in China wrote of stars falling like rain from this part of the sky over two and a half thousand years ago.",
            Shower::EtaAquariids => "The Eta Aquariids are crumbs of Halley's Comet, which the Earth passes through every May. They are fast, and best in the hours before dawn.",
            Shower::Perseids if parlor_core::units::us() => "The Perseids are dust from the comet Swift Tuttle, burning up about 60 miles overhead. On warm August nights they are the most watched shower of the year.",
            Shower::Perseids => "The Perseids are dust from the comet Swift Tuttle, burning up about 100 kilometres overhead. On warm August nights they are the most watched shower of the year.",
            Shower::Orionids => "The Orionids are more crumbs of Halley's Comet, met again on the other side of the Earth's orbit. They are swift, and some leave glowing trails.",
            Shower::Leonids => "The Leonids come from the comet Tempel Tuttle. Some years they have fallen as a storm, thousands an hour, as in 1833 and 1966.",
        }
    }
}

/// "January" for 1.
pub fn month_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
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
    NAMES[(month.clamp(1, 12) - 1) as usize]
}

/// How strong tonight's shower is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strength {
    /// The peak night, give or take [`PEAK_SLACK`].
    Peak,
    /// The run-up: `days` to the peak, from 2 to [`RUN_UP`].
    RunUp { days: i64 },
}

/// A shower that is falling tonight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowerNight {
    pub shower: Shower,
    pub strength: Strength,
    /// This shower's peak, the one tonight belongs to.
    pub peak: Date,
}

impl ShowerNight {
    pub fn is_peak(&self) -> bool {
        self.strength == Strength::Peak
    }

    /// "peak night" or "2 days before the peak".
    pub fn strength_words(&self) -> String {
        match self.strength {
            Strength::Peak => "peak night".to_string(),
            Strength::RunUp { days } => format!("{days} days before the peak"),
        }
    }

    /// The saved key for its wishes: "perseids-2026".
    pub fn wish_key(&self) -> String {
        format!("{}-{}", self.shower.id(), self.peak.year)
    }
}

/// The shower's peak nearest `today` that has not yet passed by more than
/// [`PEAK_SLACK`], with the whole days until it.
pub fn next_peak(shower: Shower, today: Date) -> (Date, i64) {
    (today.year - 1..=today.year + 1)
        .map(|y| shower.peak(y))
        .map(|d| (d, d.days() - today.days()))
        .filter(|&(_, n)| n >= -PEAK_SLACK)
        .min_by_key(|&(_, n)| n)
        .expect("a peak next year at the latest")
}

/// The shower falling tonight, if any.
pub fn shower_on(today: Date) -> Option<ShowerNight> {
    Shower::ALL.into_iter().find_map(|shower| {
        let (peak, days) = next_peak(shower, today);
        let strength = if days <= PEAK_SLACK {
            Strength::Peak
        } else if days <= RUN_UP {
            Strength::RunUp { days }
        } else {
            return None;
        };
        Some(ShowerNight {
            shower,
            strength,
            peak,
        })
    })
}

/// The next shower to open, with its peak and the days until its run-up
/// starts: 0 when one is falling tonight.
pub fn next_shower(today: Date) -> (Shower, Date, i64) {
    Shower::ALL
        .into_iter()
        .map(|shower| {
            let (peak, days) = next_peak(shower, today);
            (shower, peak, (days - RUN_UP).max(0))
        })
        .min_by_key(|&(_, _, n)| n)
        .expect("seven showers")
}

/// "The next shower is the Leonids, peaking 17 November, in 26 days."
pub fn next_shower_words(today: Date) -> String {
    next_shower_words_after(today, today)
}

/// The next shower whose peak falls after `after`, counted from `today`.
pub fn next_shower_words_after(after: Date, today: Date) -> String {
    let (shower, peak, _) = next_shower(after);
    let days = peak.days() - today.days();
    format!(
        "The next shower is the {}, peaking {}, {}.",
        shower.name(),
        shower.peak_words(),
        days_phrase(days)
    )
}

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

/// Days from noon on `date` to the nearest mean full moon: negative when it
/// has passed.
pub fn days_to_full(date: Date) -> f64 {
    SYNODIC_MONTH / 2.0 - moon_age(date)
}

/// How far from the mean full moon still counts as a full moon night. The
/// true full moon wanders up to about 14 hours from the mean one, so a
/// day either side holds it.
pub const FULL_SLACK: f64 = 1.0;

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

/// Every full moon name, January first.
pub fn moon_names() -> Vec<&'static str> {
    (1..=12).map(moon_name).collect()
}

/// The full moon, when `date` is a full moon night.
pub fn full_moon(date: Date) -> Option<FullMoon> {
    let to_full = days_to_full(date);
    if to_full.abs() > FULL_SLACK {
        return None;
    }
    // The day the mean full moon falls on, and the one before it.
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

/// The next full moon night from `today`, and the days until it.
pub fn next_full_moon(today: Date) -> (Date, i64) {
    (0..40)
        .map(|n| Date::from_days(today.days() + n))
        .find(|&d| full_moon(d).is_some())
        .map(|d| (d, d.days() - today.days()))
        .unwrap_or((today, 0))
}

/// What the game holds on `date`, for the launcher: a meteor shower, or
/// else a full moon.
pub fn event(date: Date) -> Option<Happening> {
    if let Some(night) = shower_on(date) {
        let line = match night.strength {
            Strength::Peak => {
                "Shooting stars streak over the hill tonight. Make a wish on each one.".to_string()
            }
            Strength::RunUp { days } => format!(
                "The {} build toward their peak {}. A few early shooting stars fall tonight.",
                night.shower.name(),
                days_phrase(days)
            ),
        };
        return Some(Happening::new(night.shower.title(), line).on_row(crate::SHOWER_ROW));
    }
    full_moon(date).map(|moon| {
        Happening::new(
            moon.name,
            "A full moon rises over the hill. Greet it to write it in your star journal.",
        )
    })
}
