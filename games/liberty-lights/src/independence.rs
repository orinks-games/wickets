//! Independence night: the Fourth of July's own show, open on the day.
//!
//! On the Fourth of July (give or take a day, for time zones) the title's
//! Independence night row opens a four-act show built for the day: a
//! salute of booms, stars and stripes, whistles over the river, and a
//! 20-firework grand finale that ends in a closing salute. Booms are the
//! night's signature: each one matched near-perfectly on a show that
//! clears adds a bonus. In the week before, the row plays a rehearsal of
//! the opening salute instead. Each year played keeps a keepsake.
//!
//! Everything here is a plain function of the date or the year, so tests
//! can pin it.

use hearken::Ctx;
use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening, Parlor, Records, Rng};
use serde::{Deserialize, Serialize};

use crate::festival::Act;
use crate::rules::{settle, Judgement, Kind, Launch, Pos, Scoring};
use crate::ID;

/// The holiday the night celebrates.
pub const HOLIDAY: Holiday = Holiday::IndependenceDay;
/// The row's and the mode's name.
pub const NAME: &str = "Independence night";
/// The best-score board of a whole night.
pub const BOARD: &str = "independence night";
/// Acts in the night's show.
pub const ACTS: usize = 4;
/// Fireworks in the grand finale.
pub const FINALE_SIZE: usize = 20;
/// The night's signature firework.
pub const SIGNATURE: Kind = Kind::Boom;

/// What the launcher names on the day.
pub fn event(date: Date) -> Option<Happening> {
    calendar::is_day(HOLIDAY, date).then(|| {
        Happening::new(
            NAME,
            "A four-act Fourth of July show ending in a 20-firework finale, open today only.",
        )
    })
}

/// Whether the night's show is open on `date`.
pub fn is_open(date: Date) -> bool {
    calendar::is_day(HOLIDAY, date)
}

/// Whether `date` is in the week before the Fourth, when the row plays a
/// rehearsal.
pub fn is_rehearsal(date: Date) -> bool {
    matches!(
        calendar::nearness(HOLIDAY, date),
        Some(Nearness::Season { .. })
    )
}

/// The year whose Fourth `date` belongs to: the next one, or the one just
/// past while it still counts as the day.
pub fn year_of(date: Date) -> i32 {
    HOLIDAY.next(date).map_or(date.year, |(d, _)| d.year)
}

/// "The Fourth of July is in 12 days."
pub fn countdown(date: Date) -> String {
    calendar::countdown(HOLIDAY, date).unwrap_or_default()
}

/// The row's help for `date`.
pub fn row_help(date: Date, best: Option<u64>) -> String {
    if is_open(date) {
        let best = match best {
            Some(b) => format!(" Your best tonight is {b} show points."),
            None => String::new(),
        };
        format!("Four acts for the Fourth of July, ending in a {FINALE_SIZE}-firework finale. Booms matched closely pay a bonus.{best}")
    } else if is_rehearsal(date) {
        format!(
            "{} Until then, Enter plays a rehearsal of the opening salute.",
            countdown(date)
        )
    } else {
        format!(
            "{} On the day, a four-act show built for it opens here.",
            countdown(date)
        )
    }
}

/// What Enter on the locked row says when there is no rehearsal.
pub fn locked_line(date: Date) -> String {
    format!(
        "Independence night opens on the Fourth of July. {}",
        countdown(date)
    )
}

/// The seed of a year's show: the same for every player that year.
fn year_rng(year: i32) -> Rng {
    Rng::seeded(0x4a55_4c59_0000_0000 ^ year as u64)
}

/// The opening salute: the same six booms every year, a tradition.
pub fn salute() -> Vec<Launch> {
    use Pos::*;
    [
        (Left, 0),
        (Right, 1500),
        (Left, 3000),
        (Right, 4500),
        (Center, 6000),
        (Center, 7000),
    ]
    .iter()
    .map(|&(pos, ms)| Launch::new(Kind::Boom, pos, ms))
    .collect()
}

/// Stars and stripes: sparkles and crackles in turn across the sky.
fn stars_and_stripes(rng: &mut Rng) -> Vec<Launch> {
    let mut show: Vec<Launch> = (0..8)
        .map(|i| {
            let kind = if i % 2 == 0 {
                Kind::Sparkle
            } else {
                Kind::Crackle
            };
            let pos = Pos::from_index(rng.range(0, 2) as u32);
            Launch::new(kind, pos, i * 1600 + rng.range(0, 2) * 200)
        })
        .collect();
    settle(&mut show, 500);
    show
}

/// Whistles over the river: mostly whistles, some going up early.
fn whistles(rng: &mut Rng) -> Vec<Launch> {
    let mut show: Vec<Launch> = (0..10)
        .map(|i| {
            let kind = if rng.range(0, 2) < 2 {
                Kind::Whistle
            } else {
                Kind::from_index(rng.range(0, 3) as u32)
            };
            let pos = Pos::from_index(rng.range(0, 2) as u32);
            let mut ms = i * 1500;
            if rng.range(0, 1) == 1 {
                ms -= rng.range(200, 600);
            }
            Launch::new(kind, pos, ms)
        })
        .collect();
    settle(&mut show, 0);
    show
}

/// The grand finale: sixteen quick fireworks of every kind, then the
/// closing salute, four booms left, center, right and center.
fn finale(rng: &mut Rng) -> Vec<Launch> {
    let mut show: Vec<Launch> = (0..16)
        .map(|i| {
            let kind = Kind::from_index(rng.range(0, 3) as u32);
            let pos = Pos::from_index(rng.range(0, 2) as u32);
            let mut ms = i * 900;
            if rng.range(0, 1) == 1 {
                ms -= rng.range(100, 400);
            }
            Launch::new(kind, pos, ms)
        })
        .collect();
    settle(&mut show, 0);
    let end = show.last().map_or(0, |l| l.ms) + 1200;
    for (i, pos) in [Pos::Left, Pos::Center, Pos::Right, Pos::Center]
        .into_iter()
        .enumerate()
    {
        show.push(Launch::new(Kind::Boom, pos, end + 500 * i as i32));
    }
    show
}

/// The night's four acts for `year`.
pub fn acts(year: i32) -> Vec<Act> {
    let mut rng = year_rng(year);
    let act = |name: &str, show: Vec<Launch>| Act {
        name: name.to_string(),
        show,
        placed: true,
    };
    vec![
        act("The opening salute", salute()),
        act("Stars and stripes", stars_and_stripes(&mut rng)),
        act("Whistles over the river", whistles(&mut rng)),
        act("The grand finale", finale(&mut rng)),
    ]
}

/// A night being played: which act is next and the points so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NightRun {
    pub year: i32,
    pub acts: Vec<Act>,
    /// The act being played, from 0.
    pub stage: usize,
    /// Show points of the acts finished.
    pub total: u64,
    /// Acts that scored 60 percent or more.
    pub cleared: u32,
    /// Bonus points the booms have paid so far.
    pub boom_bonus: u64,
}

impl NightRun {
    pub fn new(year: i32) -> NightRun {
        NightRun {
            year,
            acts: acts(year),
            stage: 0,
            total: 0,
            cleared: 0,
            boom_bonus: 0,
        }
    }

    pub fn act(&self) -> &Act {
        &self.acts[self.stage.min(self.acts.len() - 1)]
    }

    /// "act 2 of 4".
    pub fn progress(&self) -> String {
        format!("act {} of {}", self.stage + 1, self.acts.len())
    }

    /// The night's scoring: booms pay the signature bonus.
    pub fn scoring(&self) -> Scoring {
        Scoring {
            tough: false,
            signature: Some(SIGNATURE),
        }
    }

    /// Scores the current act into the night and moves on. True when the
    /// last act is over: every act is played, pass or fail.
    pub fn record(&mut self, j: &Judgement) -> bool {
        self.total += j.points;
        self.boom_bonus += j.signature_bonus;
        if j.passed() {
            self.cleared += 1;
        }
        self.stage += 1;
        self.stage >= self.acts.len()
    }
}

/// The key keepsakes are saved under.
pub fn keepsake_key() -> String {
    format!("{ID}/independence")
}

/// One year's Independence night, kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keepsake {
    pub year: i32,
    /// The best night's show points that year.
    pub best: u64,
    /// The most acts cleared in one night that year.
    pub acts_cleared: u32,
    /// Nights played to the end that year.
    pub nights: u32,
}

/// Every year's keepsake, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keepsakes {
    pub years: Vec<Keepsake>,
}

/// What keeping a night changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// The first night kept for that year.
    First,
    /// A better night than the year's best.
    Better { previous: u64 },
    /// Short of the year's best.
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

    /// Keeps a night played to the end in `year`.
    pub fn keep(&mut self, year: i32, total: u64, cleared: u32) -> Kept {
        let at = match self.years.iter().position(|k| k.year == year) {
            Some(at) => at,
            None => {
                self.years.push(Keepsake {
                    year,
                    ..Default::default()
                });
                self.years.sort_by_key(|k| k.year);
                self.years.iter().position(|k| k.year == year).unwrap_or(0)
            }
        };
        let k = &mut self.years[at];
        let kept = if k.nights == 0 {
            Kept::First
        } else if total > k.best {
            Kept::Better { previous: k.best }
        } else {
            Kept::Short { best: k.best }
        };
        k.nights += 1;
        k.best = k.best.max(total);
        k.acts_cleared = k.acts_cleared.max(cleared);
        kept
    }

    /// The Records page.
    pub fn page(&self, today: Date) -> String {
        if self.years.is_empty() {
            return format!(
                "Keepsakes. None yet: Independence night opens on the Fourth of July. {}",
                countdown(today)
            );
        }
        let lines: Vec<String> = self
            .years
            .iter()
            .map(|k| {
                format!(
                    "Independence night {}: best {} show points, {} of {ACTS} acts cleared.",
                    k.year, k.best, k.acts_cleared
                )
            })
            .collect();
        format!("Keepsakes. {}", lines.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_night_opens_on_the_fourth_give_or_take_a_day() {
        for year in [2025, 2026, 2027, 2031] {
            assert!(!is_open(Date::new(year, 7, 2)), "{year}");
            assert!(is_open(Date::new(year, 7, 3)), "{year}");
            assert!(is_open(Date::new(year, 7, 4)), "{year}");
            assert!(is_open(Date::new(year, 7, 5)), "{year}");
            assert!(!is_open(Date::new(year, 7, 6)), "{year}");
            assert!(event(Date::new(year, 7, 4)).is_some());
            assert!(event(Date::new(year, 7, 6)).is_none());
            assert_eq!(year_of(Date::new(year, 7, 5)), year);
            assert_eq!(year_of(Date::new(year, 7, 6)), year + 1);
        }
        assert_eq!(year_of(Date::new(2026, 12, 31)), 2027);
    }

    #[test]
    fn the_week_before_holds_a_rehearsal() {
        assert!(is_rehearsal(Date::new(2026, 6, 27)));
        assert!(is_rehearsal(Date::new(2026, 7, 2)));
        assert!(!is_rehearsal(Date::new(2026, 6, 26)));
        assert!(!is_rehearsal(Date::new(2026, 7, 4)));
        assert!(!is_rehearsal(Date::new(2026, 10, 22)));
        assert_eq!(
            countdown(Date::new(2026, 7, 1)),
            "The Fourth of July is in 3 days."
        );
        assert!(row_help(Date::new(2026, 7, 1), None).contains("rehearsal"));
        assert!(row_help(Date::new(2026, 1, 1), None).starts_with("The Fourth of July is in"));
        assert!(row_help(Date::new(2026, 7, 4), Some(900)).contains("900"));
    }

    #[test]
    fn each_year_has_its_own_show_and_the_same_salute() {
        let a = acts(2026);
        let b = acts(2027);
        assert_eq!(a.len(), ACTS);
        assert_eq!(a, acts(2026));
        assert_ne!(a, b);
        assert_eq!(a[0].show, b[0].show);
        assert!(a[0].show.iter().all(|l| l.kind == Kind::Boom));
        assert_eq!(a[3].show.len(), FINALE_SIZE);
        let closing: Vec<Kind> = a[3].show[16..].iter().map(|l| l.kind).collect();
        assert_eq!(closing, [Kind::Boom; 4]);
        for act in &a {
            assert!(act.show.windows(2).all(|w| w[0].ms <= w[1].ms));
            assert!(act.show.first().is_some_and(|l| l.ms >= 0));
        }
        assert!(a[1]
            .show
            .iter()
            .all(|l| matches!(l.kind, Kind::Sparkle | Kind::Crackle)));
    }

    #[test]
    fn a_night_plays_every_act_and_keeps_the_years_best() {
        let mut run = NightRun::new(2026);
        let show = run.act().show.clone();
        let good = crate::rules::judge_with(&show, &show, false, run.scoring());
        assert_eq!(good.signature_bonus, 6 * crate::rules::SIGNATURE_BONUS);
        assert!(!run.record(&good));
        let bad = crate::rules::judge(&run.act().show.clone(), &[], false);
        assert!(!run.record(&bad));
        assert!(!run.record(&bad));
        assert!(run.record(&bad));
        assert_eq!(run.cleared, 1);
        assert_eq!(run.total, good.points);

        let mut keep = Keepsakes::default();
        assert_eq!(keep.keep(2026, 500, 1), Kept::First);
        assert_eq!(keep.keep(2026, 900, 2), Kept::Better { previous: 500 });
        assert_eq!(keep.keep(2026, 100, 0), Kept::Short { best: 900 });
        assert_eq!(keep.keep(2025, 50, 0), Kept::First);
        assert_eq!(keep.years[0].year, 2025);
        let k = keep.year(2026).unwrap();
        assert_eq!((k.best, k.acts_cleared, k.nights), (900, 2, 3));
        assert!(keep
            .page(Date::new(2026, 7, 4))
            .contains("Independence night 2026: best 900"));
        assert!(Keepsakes::default()
            .page(Date::new(2026, 6, 1))
            .starts_with("Keepsakes. None yet"));
    }
}
