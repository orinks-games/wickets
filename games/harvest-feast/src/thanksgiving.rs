//! Thanksgiving dinner: the whole family's dinner, cooked on the day.
//!
//! On Thanksgiving (the fourth Thursday of November, give or take a day
//! for time zones) the title's Thanksgiving dinner row opens the biggest
//! meal in the game: twelve dishes in three waves on all five stations,
//! five of them cooked only on this day, from the sweet potato casserole
//! to the carving and the pecan pie. Each year a different relative asks
//! for one dish, and it scores double. In the ten days before, the row
//! reads the year's menu and guests instead. Each year cooked keeps a
//! keepsake.
//!
//! Everything here is a plain function of the date or the year, so tests
//! can pin it.

use hearken::Ctx;
use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening, Pace, Parlor, Records, Rng};
use serde::{Deserialize, Serialize};

use crate::rules::{Course, DishKind, Outcome, Plan};
use crate::ID;

/// The holiday the dinner is for.
pub const HOLIDAY: Holiday = Holiday::Thanksgiving;
/// The row's and the meal's name.
pub const NAME: &str = "Thanksgiving dinner";
/// The best-score board of the dinner.
pub const BOARD: &str = "thanksgiving dinner";

/// What the launcher names on the day.
pub fn event(date: Date) -> Option<Happening> {
    calendar::is_day(HOLIDAY, date).then(|| {
        Happening::new(
            NAME,
            "Cook the family's twelve-dish dinner, with five dishes made only today.",
        )
    })
}

/// Whether the dinner is open on `date`.
pub fn is_open(date: Date) -> bool {
    calendar::is_day(HOLIDAY, date)
}

/// Whether `date` is in the run-up, when the row reads the menu.
pub fn is_run_up(date: Date) -> bool {
    matches!(
        calendar::nearness(HOLIDAY, date),
        Some(Nearness::Season { .. })
    )
}

/// The year whose Thanksgiving `date` belongs to.
pub fn year_of(date: Date) -> i32 {
    HOLIDAY.next(date).map_or(date.year, |(d, _)| d.year)
}

/// "Thanksgiving is in 12 days."
pub fn countdown(date: Date) -> String {
    calendar::countdown(HOLIDAY, date).unwrap_or_default()
}

/// Who might come to dinner.
const RELATIVES: [&str; 8] = [
    "Grandpa Joe",
    "Aunt Marge",
    "Cousin Lily",
    "Uncle Walt",
    "Great-aunt Ida",
    "Little Sam",
    "Nana Rose",
    "Cousin Ray",
];

/// What the family says as the dinner bell rings, one a year.
const TOASTS: [&str; 6] = [
    "Someone raises a glass: to the cook, and to everyone at this table.",
    "Everyone holds hands, and Grandpa says a short thank you.",
    "Little Sam asks for seconds before the first plates are passed.",
    "Aunt Marge says it's the best dinner since the year the oven caught fire.",
    "The whole table goes quiet, then someone asks for the gravy.",
    "Nana Rose tells the story of her first Thanksgiving, as she does every year.",
];

/// Each dish of the dinner, in the order the recipe calls them: three
/// waves on all five stations, each dish waiting for the one before it at
/// its station.
pub fn dinner() -> Course {
    use DishKind::*;
    let at = |kind, start_at| Plan {
        kind,
        start_at,
        after: None,
    };
    let then = |kind, start_at, after| Plan {
        kind,
        start_at,
        after: Some(after),
    };
    Course {
        number: 0,
        name: NAME,
        blurb: "Twelve dishes in three waves on all five stations, for the whole family.",
        speed: 0.8,
        plan: vec![
            at(Turkey, 1.0),
            at(Celery, 2.0),
            at(CranberrySauce, 4.0),
            at(MapleCarrots, 6.0),
            at(Cornbread, 8.0),
            then(SweetPotatoes, 2.0, 0),
            then(Gravy, 2.0, 2),
            then(Potatoes, 2.0, 3),
            then(Carving, 1.0, 1),
            then(PecanPie, 2.0, 5),
            then(Cream, 2.0, 8),
            then(Cider, 2.0, 7),
        ],
    }
}

/// One year's family: who comes, who asks for what, and the toast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Family {
    pub year: i32,
    /// Three guests, the first of them asking for the favourite.
    pub guests: [&'static str; 3],
    /// The dish that scores double this year.
    pub favourite: DishKind,
    pub toast: &'static str,
}

/// The family coming to dinner in `year`: the same for every player.
pub fn family(year: i32) -> Family {
    let mut rng = Rng::seeded(0x5448_414e_4b53_0000 ^ year as u64);
    let mut names = RELATIVES.to_vec();
    rng.shuffle(&mut names);
    let favourite = *rng.pick(&DishKind::THANKSGIVING);
    let toast = *rng.pick(&TOASTS);
    Family {
        year,
        guests: [names[0], names[1], names[2]],
        favourite,
        toast,
    }
}

impl Family {
    /// "Grandpa Joe, Aunt Marge and Cousin Lily are coming. Grandpa Joe
    /// asked for the pecan pie: it scores double."
    pub fn line(&self) -> String {
        format!(
            "{}, {} and {} are coming. {} asked for the {}: it scores double.",
            self.guests[0],
            self.guests[1],
            self.guests[2],
            self.guests[0],
            self.favourite.name()
        )
    }

    /// The favourite's quality, as extra points: its quality times ten
    /// again, so it counts twice.
    pub fn bonus(&self, outcome: &Outcome) -> u64 {
        outcome
            .qualities
            .iter()
            .filter(|(k, _)| *k == self.favourite)
            .map(|(_, q)| *q as u64 * 10)
            .sum()
    }
}

/// The menu as a sentence, for the run-up and the intro.
pub fn menu_line() -> String {
    let dishes: Vec<&str> = dinner().plan.iter().map(|p| p.kind.name()).collect();
    let (last, rest) = dishes.split_last().map_or(("", &[][..]), |(l, r)| (*l, r));
    format!("On the menu: {}, and {last}.", rest.join(", "))
}

/// The row's help for `date`.
pub fn row_help(date: Date, best: Option<u64>) -> String {
    if is_open(date) {
        let best = match best {
            Some(b) => format!(" Your best is {b} points."),
            None => String::new(),
        };
        format!(
            "The family's dinner: twelve dishes on all five stations, five made only today.{best}"
        )
    } else if is_run_up(date) {
        format!(
            "{} Until then, Enter reads this year's menu and who is coming.",
            countdown(date)
        )
    } else {
        format!(
            "{} On the day, the family's twelve-dish dinner opens here.",
            countdown(date)
        )
    }
}

/// What Enter on the locked row says: the countdown, and in the run-up
/// the year's menu and guests.
pub fn locked_line(date: Date) -> String {
    if is_run_up(date) {
        let f = family(year_of(date));
        format!("{} {} {}", countdown(date), f.line(), menu_line())
    } else {
        format!(
            "Thanksgiving dinner opens on Thanksgiving Day. {}",
            countdown(date)
        )
    }
}

/// The key keepsakes are saved under.
pub fn keepsake_key() -> String {
    format!("{ID}/thanksgiving")
}

/// One year's dinner, kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keepsake {
    pub year: i32,
    /// The best dinner's points that year, at normal or quick pace.
    pub best: u64,
    /// The best relaxed dinner's points that year, kept apart.
    pub relaxed_best: u64,
    /// The most stars that year.
    pub stars: u32,
    /// The favourite's best quality that year.
    pub favourite: u32,
    /// Dinners cooked that year.
    pub dinners: u32,
}

/// Every year's keepsake, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keepsakes {
    pub years: Vec<Keepsake>,
}

/// What keeping a dinner changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    First,
    Better { previous: u64 },
    Short { best: u64 },
}

impl Keepsakes {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Keepsakes {
        Records::load(ctx, &keepsake_key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &keepsake_key(), self);
    }

    pub fn year(&self, year: i32) -> Option<&Keepsake> {
        self.years.iter().find(|k| k.year == year)
    }

    /// Keeps a dinner cooked in `year` at normal or quick pace.
    pub fn keep(&mut self, year: i32, points: u64, stars: u32, favourite: u32) -> Kept {
        self.keep_at(year, points, stars, favourite, Pace::Normal)
    }

    /// Keeps a dinner cooked in `year` at `pace`. Stars and the favourite
    /// count at every pace; a relaxed dinner's points have their own best.
    pub fn keep_at(
        &mut self,
        year: i32,
        points: u64,
        stars: u32,
        favourite: u32,
        pace: Pace,
    ) -> Kept {
        if self.year(year).is_none() {
            self.years.push(Keepsake {
                year,
                ..Default::default()
            });
            self.years.sort_by_key(|k| k.year);
        }
        let Some(k) = self.years.iter_mut().find(|k| k.year == year) else {
            return Kept::First;
        };
        let best = if pace.separate() {
            &mut k.relaxed_best
        } else {
            &mut k.best
        };
        let kept = if *best == 0 {
            Kept::First
        } else if points > *best {
            Kept::Better { previous: *best }
        } else {
            Kept::Short { best: *best }
        };
        *best = (*best).max(points);
        k.dinners += 1;
        k.stars = k.stars.max(stars);
        k.favourite = k.favourite.max(favourite);
        kept
    }

    /// The Records page.
    pub fn page(&self, today: Date) -> String {
        if self.years.is_empty() {
            return format!(
                "Keepsakes. None yet: Thanksgiving dinner opens on Thanksgiving Day. {}",
                countdown(today)
            );
        }
        let lines: Vec<String> = self
            .years
            .iter()
            .map(|k| {
                let stars = match k.stars {
                    1 => "1 star".to_string(),
                    n => format!("{n} stars"),
                };
                let relaxed = if k.relaxed_best > 0 {
                    format!(", relaxed best {} points", k.relaxed_best)
                } else {
                    String::new()
                };
                format!(
                    "Thanksgiving {}: best {} points{relaxed}, {stars}, {} at quality {}.",
                    k.year,
                    k.best,
                    family(k.year).favourite.name(),
                    k.favourite
                )
            })
            .collect();
        format!("Keepsakes. {}", lines.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Meal;

    /// Thanksgiving: the fourth Thursday of November.
    const DAYS: [(i32, u32); 6] = [
        (2024, 28),
        (2025, 27),
        (2026, 26),
        (2027, 25),
        (2028, 23),
        (2029, 22),
    ];

    #[test]
    fn the_dinner_opens_on_the_fourth_thursday_give_or_take_a_day() {
        for (year, day) in DAYS {
            let on = Date::new(year, 11, day);
            assert_eq!(on.weekday(), 3, "{year}");
            assert!(is_open(on), "{year}");
            assert!(is_open(Date::new(year, 11, day - 1)), "{year}");
            assert!(is_open(Date::new(year, 11, day + 1)), "{year}");
            assert!(!is_open(Date::new(year, 11, day - 2)), "{year}");
            assert!(!is_open(Date::new(year, 11, day + 2)), "{year}");
            assert!(event(on).is_some());
            assert!(event(Date::new(year, 11, day + 2)).is_none());
            assert_eq!(year_of(Date::new(year, 11, day + 1)), year);
        }
        // A fixed date is the day in one year and not in another.
        assert!(is_open(Date::new(2026, 11, 26)));
        assert!(!is_open(Date::new(2028, 11, 26)));
        assert_eq!(year_of(Date::new(2026, 12, 31)), 2027);
    }

    #[test]
    fn the_run_up_reads_the_menu() {
        assert!(is_run_up(Date::new(2026, 11, 16)));
        assert!(is_run_up(Date::new(2026, 11, 24)));
        assert!(!is_run_up(Date::new(2026, 11, 15)));
        assert!(!is_run_up(Date::new(2026, 11, 26)));
        let line = locked_line(Date::new(2026, 11, 20));
        assert!(line.starts_with("Thanksgiving is in 6 days."), "{line}");
        assert!(line.contains("asked for the"), "{line}");
        assert!(line.contains("On the menu: turkey,"), "{line}");
        let far = locked_line(Date::new(2026, 3, 1));
        assert!(far.starts_with("Thanksgiving dinner opens on Thanksgiving Day."));
        assert!(row_help(Date::new(2026, 11, 26), Some(5000)).contains("5000"));
    }

    #[test]
    fn the_dinner_is_twelve_dishes_on_every_station() {
        let c = dinner();
        assert_eq!(c.plan.len(), 12);
        let meal = Meal::new(&c);
        assert_eq!(meal.stations().len(), 5);
        for kind in DishKind::THANKSGIVING {
            assert!(c.plan.iter().any(|p| p.kind == kind), "{kind:?}");
            assert!(!DishKind::ALL.contains(&kind), "{kind:?}");
            assert!(!kind.steps().is_empty());
        }
        for p in &c.plan {
            if let Some(after) = p.after {
                assert_eq!(c.plan[after].kind.station(), p.kind.station());
            }
        }
    }

    #[test]
    fn each_year_brings_its_own_family() {
        let a = family(2026);
        assert_eq!(a, family(2026));
        let years: Vec<Family> = (2024..2034).map(family).collect();
        assert!(years.iter().any(|f| f.favourite != a.favourite));
        assert!(years.iter().any(|f| f.guests != a.guests));
        for f in &years {
            assert!(DishKind::THANKSGIVING.contains(&f.favourite));
            assert!(f.line().ends_with("it scores double."));
        }
    }

    #[test]
    fn keepsakes_keep_each_years_best() {
        let mut k = Keepsakes::default();
        assert_eq!(k.keep(2026, 4000, 2, 80), Kept::First);
        assert_eq!(k.keep(2026, 5000, 1, 90), Kept::Better { previous: 4000 });
        assert_eq!(k.keep(2026, 100, 0, 0), Kept::Short { best: 5000 });
        assert_eq!(k.keep(2025, 10, 0, 0), Kept::First);
        assert_eq!(k.years[0].year, 2025);
        let y = k.year(2026).unwrap();
        assert_eq!((y.best, y.stars, y.favourite, y.dinners), (5000, 2, 90, 3));
        assert!(k
            .page(Date::new(2026, 11, 26))
            .contains("Thanksgiving 2026: best 5000 points, 2 stars"));
        assert_eq!(
            k.keep_at(2026, 9000, 3, 95, Pace::Relaxed),
            Kept::First,
            "a relaxed dinner keeps its own best"
        );
        let year = k.year(2026).unwrap();
        assert_eq!((year.best, year.relaxed_best, year.stars), (5000, 9000, 3));
        assert!(k
            .page(Date::new(2026, 11, 26))
            .contains("best 5000 points, relaxed best 9000 points, 3 stars"));
    }
}
