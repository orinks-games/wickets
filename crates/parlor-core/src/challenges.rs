//! Daily, weekly and monthly challenges, the same for every player.
//!
//! A challenge is a seed taken from the game's id and the current period
//! ("2026-10-16", "2026-W42", "2026-10"), so everyone who plays a game's
//! daily challenge on the same day gets the same layout. A game turns the
//! seed into its run with [`Challenge::rng`], submits the result with
//! [`Challenges::submit`], and reads streaks with [`Challenges::streak`].
//!
//! Dates are UTC, so a day turns over at the same moment for everyone.

use std::collections::BTreeMap;

use hearken::Ctx;

use crate::{Parlor, Records, Rng, ScoreResult};

/// How long a challenge lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Period {
    Daily,
    Weekly,
    Monthly,
}

impl Period {
    pub const ALL: [Period; 3] = [Period::Daily, Period::Weekly, Period::Monthly];

    /// "Daily", "Weekly", "Monthly".
    pub fn label(self) -> &'static str {
        match self {
            Period::Daily => "Daily",
            Period::Weekly => "Weekly",
            Period::Monthly => "Monthly",
        }
    }

    /// "day", "week", "month", for sentences such as "a 3 day streak".
    pub fn unit(self) -> &'static str {
        match self {
            Period::Daily => "day",
            Period::Weekly => "week",
            Period::Monthly => "month",
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Period::Daily => "daily",
            Period::Weekly => "weekly",
            Period::Monthly => "monthly",
        }
    }
}

/// A calendar date (proleptic Gregorian, UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub const fn new(year: i32, month: u32, day: u32) -> Date {
        Date { year, month, day }
    }

    /// The date `days` after 1970-01-01.
    pub fn from_days(days: i64) -> Date {
        // Howard Hinnant's civil_from_days.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        Date::new((if m <= 2 { y + 1 } else { y }) as i32, m, d)
    }

    /// Days since 1970-01-01.
    pub fn days(self) -> i64 {
        let y = self.year as i64 - if self.month <= 2 { 1 } else { 0 };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let m = self.month as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Today in UTC, by the trusted clock: the computer's clock corrected
    /// by a public time server once one has answered (see
    /// [`hearken::clock`]), so moving the computer's clock does not open a
    /// dated event or challenge early. Offline it is the computer's clock.
    pub fn system_today() -> Date {
        let (secs, _source) = hearken::clock::trusted_unix_now();
        Date::from_days(secs.div_euclid(86_400))
    }

    /// Parses "YYYY-MM-DD".
    pub fn parse(text: &str) -> Option<Date> {
        let mut parts = text.trim().splitn(3, '-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        let date = Date::new(year, month, day);
        (Date::from_days(date.days()) == date).then_some(date)
    }

    /// Monday is 0, Sunday is 6.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday.
        (self.days() + 3).rem_euclid(7) as u32
    }

    /// The ISO 8601 week-numbering year and week.
    pub fn iso_week(self) -> (i32, u32) {
        let thursday = Date::from_days(self.days() - self.weekday() as i64 + 3);
        let jan1 = Date::new(thursday.year, 1, 1);
        (
            thursday.year,
            ((thursday.days() - jan1.days()) / 7 + 1) as u32,
        )
    }

    /// The period's id: "2026-10-16", "2026-W42" or "2026-10".
    pub fn period_key(self, period: Period) -> String {
        match period {
            Period::Daily => format!("{:04}-{:02}-{:02}", self.year, self.month, self.day),
            Period::Weekly => {
                let (y, w) = self.iso_week();
                format!("{y:04}-W{w:02}")
            }
            Period::Monthly => format!("{:04}-{:02}", self.year, self.month),
        }
    }

    /// The first day of the period after the one holding this date.
    pub fn next_period_start(self, period: Period) -> Date {
        match period {
            Period::Daily => Date::from_days(self.days() + 1),
            Period::Weekly => Date::from_days(self.days() - self.weekday() as i64 + 7),
            Period::Monthly => {
                if self.month == 12 {
                    Date::new(self.year + 1, 1, 1)
                } else {
                    Date::new(self.year, self.month + 1, 1)
                }
            }
        }
    }

    /// A date in the period before the one holding this date.
    fn previous_period(self, period: Period) -> Date {
        match period {
            Period::Daily => Date::from_days(self.days() - 1),
            Period::Weekly => Date::from_days(self.days() - 7),
            Period::Monthly => Date::from_days(Date::new(self.year, self.month, 1).days() - 1),
        }
    }
}

/// Today for the parlor: the date a test pinned in `ctx.game.today`; else,
/// in debug builds only, `ECHO_PARLOR_TODAY` ("YYYY-MM-DD"), for trying an
/// event ahead of time; else the trusted clock ([`Date::system_today`]).
/// Release builds ignore the variable, so it can't open an event early.
pub fn today(ctx: &Ctx<Parlor>) -> Date {
    if let Some(date) = ctx.game.today {
        return date;
    }
    preview_today().unwrap_or_else(Date::system_today)
}

/// `ECHO_PARLOR_TODAY`, in debug builds only.
fn preview_today() -> Option<Date> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("ECHO_PARLOR_TODAY")
        .ok()
        .and_then(|v| Date::parse(&v))
}

fn fnv(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// One game's challenge for one period.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Challenge {
    pub game: &'static str,
    pub period: Period,
    /// "2026-10-16", "2026-W42" or "2026-10".
    pub key: String,
    /// The same for every player in this period.
    pub seed: u64,
    /// Whole days until the next one, counting today: 1 for a daily.
    pub days_left: i64,
}

impl Challenge {
    /// The game's current challenge for `period`.
    pub fn current(ctx: &Ctx<Parlor>, game: &'static str, period: Period) -> Challenge {
        Challenge::on(today(ctx), game, period)
    }

    /// The game's challenge for `period` on `date`.
    pub fn on(date: Date, game: &'static str, period: Period) -> Challenge {
        let key = date.period_key(period);
        Challenge {
            game,
            period,
            seed: fnv(&format!("{game}/{}/{key}", period.tag())),
            days_left: date.next_period_start(period).days() - date.days(),
            key,
        }
    }

    /// A random source seeded for this challenge.
    pub fn rng(&self) -> Rng {
        Rng::seeded(self.seed)
    }

    /// "Daily challenge", "Weekly challenge"...
    pub fn title(&self) -> String {
        format!("{} challenge", self.period.label())
    }

    /// "A new one tomorrow.", "A new one in 3 days."
    pub fn resets(&self) -> String {
        match self.days_left {
            i64::MIN..=1 => "A new one tomorrow.".to_string(),
            n => format!("A new one in {n} days."),
        }
    }

    fn slot(&self) -> String {
        format!("{}:{}", self.period.tag(), self.key)
    }
}

/// Saved challenge results.
pub struct Challenges;

impl Challenges {
    fn load(ctx: &mut Ctx<Parlor>, game: &str) -> BTreeMap<String, u64> {
        Records::load(ctx, &format!("{game}/challenges"))
    }

    /// The best score on this challenge, if it has been played.
    pub fn best(ctx: &mut Ctx<Parlor>, challenge: &Challenge) -> Option<u64> {
        Self::load(ctx, challenge.game)
            .get(&challenge.slot())
            .copied()
    }

    /// Whether this challenge has been completed.
    pub fn done(ctx: &mut Ctx<Parlor>, challenge: &Challenge) -> bool {
        Self::best(ctx, challenge).is_some()
    }

    /// Records a finished challenge run (any finish counts toward the
    /// streak; a run abandoned from pause should not be submitted). Keeps
    /// the best score, and marks the day played for the parlor's own streak.
    pub fn submit(ctx: &mut Ctx<Parlor>, challenge: &Challenge, score: u64) -> ScoreResult {
        let key = format!("{}/challenges", challenge.game);
        let mut all: BTreeMap<String, u64> = Records::load(ctx, &key);
        let slot = challenge.slot();
        let result = match all.get(&slot).copied() {
            None => ScoreResult::First,
            Some(previous) if score > previous => ScoreResult::NewBest { previous },
            Some(best) => ScoreResult::Short { best },
        };
        if !matches!(result, ScoreResult::Short { .. }) {
            all.insert(slot, score);
            Records::store(ctx, &key, &all);
        }
        let day = today(ctx).period_key(Period::Daily);
        let mut days: BTreeMap<String, u32> = Records::load(ctx, "parlor/days");
        *days.entry(day).or_default() += 1;
        Records::store(ctx, "parlor/days", &days);
        crate::tutorial::mark_played(ctx, challenge.game);
        result
    }

    /// How many periods in a row this game's `period` challenge has been
    /// finished, counting back from the current period (or from the last
    /// one, while the current one is still to play).
    pub fn streak(ctx: &mut Ctx<Parlor>, game: &str, period: Period) -> u32 {
        let all = Self::load(ctx, game);
        let tag = period.tag();
        let has = |d: Date| all.contains_key(&format!("{tag}:{}", d.period_key(period)));
        let mut date = today(ctx);
        if !has(date) {
            date = date.previous_period(period);
        }
        let mut n = 0;
        while has(date) && n < 10_000 {
            n += 1;
            date = date.previous_period(period);
        }
        n
    }

    /// Days in a row on which any challenge in the parlor was finished.
    pub fn parlor_streak(ctx: &mut Ctx<Parlor>) -> u32 {
        let days: BTreeMap<String, u32> = Records::load(ctx, "parlor/days");
        let mut date = today(ctx);
        if !days.contains_key(&date.period_key(Period::Daily)) {
            date = date.previous_period(Period::Daily);
        }
        let mut n = 0;
        while days.contains_key(&date.period_key(Period::Daily)) && n < 100_000 {
            n += 1;
            date = date.previous_period(Period::Daily);
        }
        n
    }

    /// "Daily challenge, done, best 1200, a 3 day streak." for menus.
    pub fn summary(ctx: &mut Ctx<Parlor>, challenge: &Challenge) -> String {
        let streak = Self::streak(ctx, challenge.game, challenge.period);
        let streak = match streak {
            0 => String::new(),
            n => format!(
                ", {} {n} {} streak",
                crate::words::a_or_an(&n.to_string()),
                challenge.period.unit()
            ),
        };
        match Self::best(ctx, challenge) {
            Some(best) => format!("{}, done, best {best}{streak}", challenge.title()),
            None => format!("{}, not played yet{streak}", challenge.title()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip_through_days() {
        for days in -800_000..800_000i64 {
            if days % 997 == 0 {
                assert_eq!(Date::from_days(days).days(), days);
            }
        }
        assert_eq!(Date::from_days(0), Date::new(1970, 1, 1));
        assert_eq!(Date::new(2026, 10, 16).weekday(), 4); // a Friday
    }

    #[test]
    fn iso_weeks_follow_the_standard() {
        assert_eq!(Date::new(2026, 10, 16).iso_week(), (2026, 42));
        assert_eq!(Date::new(2021, 1, 3).iso_week(), (2020, 53));
        assert_eq!(Date::new(2024, 12, 30).iso_week(), (2025, 1));
    }

    #[test]
    fn periods_have_keys_and_resets() {
        let d = Date::new(2026, 10, 16);
        assert_eq!(d.period_key(Period::Daily), "2026-10-16");
        assert_eq!(d.period_key(Period::Weekly), "2026-W42");
        assert_eq!(d.period_key(Period::Monthly), "2026-10");
        assert_eq!(Challenge::on(d, "g", Period::Daily).days_left, 1);
        assert_eq!(Challenge::on(d, "g", Period::Weekly).days_left, 3);
        assert_eq!(Challenge::on(d, "g", Period::Monthly).days_left, 16);
        assert_eq!(
            Date::new(2026, 12, 5).next_period_start(Period::Monthly),
            Date::new(2027, 1, 1)
        );
    }

    #[test]
    fn seeds_differ_by_game_period_and_day_but_not_by_player() {
        let d = Date::new(2026, 10, 16);
        let a = Challenge::on(d, "a", Period::Daily);
        assert_eq!(a, Challenge::on(d, "a", Period::Daily));
        assert_ne!(a.seed, Challenge::on(d, "b", Period::Daily).seed);
        assert_ne!(a.seed, Challenge::on(d, "a", Period::Weekly).seed);
        assert_ne!(
            a.seed,
            Challenge::on(Date::new(2026, 10, 17), "a", Period::Daily).seed
        );
    }

    #[test]
    fn parse_rejects_impossible_dates() {
        assert_eq!(Date::parse("2026-02-28"), Some(Date::new(2026, 2, 28)));
        assert_eq!(Date::parse("2026-02-30"), None);
        assert_eq!(Date::parse("nope"), None);
    }
}
