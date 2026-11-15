//! What the game remembers between visits: the player's choices, best
//! times, ride points and training.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{units, Pace, Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::bike::Bike;
use crate::course::{District, COURSES};
use crate::ID;

/// How much the game says aloud about the cues it plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallOuts {
    /// Every cue is named every time.
    All,
    /// Each cue is named the first time it plays in a session.
    #[default]
    FirstTime,
    /// Sound only. Status and Hint still speak.
    Off,
}

impl CallOuts {
    pub fn word(self) -> &'static str {
        match self {
            CallOuts::All => "every time",
            CallOuts::FirstTime => "once per session",
            CallOuts::Off => "off",
        }
    }

    pub fn next(self) -> CallOuts {
        match self {
            CallOuts::All => CallOuts::FirstTime,
            CallOuts::FirstTime => CallOuts::Off,
            CallOuts::Off => CallOuts::All,
        }
    }
}

/// The player's choices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Choices {
    #[serde(default)]
    pub bike: Bike,
    /// The time trial course the Play row starts, from 0.
    #[serde(default)]
    pub course: usize,
    #[serde(default)]
    pub call_outs: CallOuts,
    /// The district the endless ride runs in.
    #[serde(default)]
    pub district: District,
}

impl Choices {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Choices {
        let mut c: Choices = Records::load(ctx, &format!("{ID}/choices"));
        c.course = c.course.min(COURSES.len() - 1);
        // A bike or district not owned, as from an old save, falls back.
        let progress = Progress::load(ctx);
        if !progress.owns_bike(c.bike) {
            c.bike = Bike::Road;
        }
        if !progress.owns_district(c.district) {
            c.district = District::Downtown;
        }
        c
    }

    /// The choices as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> Choices {
        Records::peek(ctx, &format!("{ID}/choices"))
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &format!("{ID}/choices"), self);
    }
}

/// Totals across every ride.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Rides finished, of any kind.
    #[serde(default)]
    pub rides: u32,
    /// Training lessons passed, the highest so far.
    #[serde(default)]
    pub lessons: u32,
    /// Metres ridden in all.
    #[serde(default)]
    pub metres: f64,
    /// The longest jump, seconds of air.
    #[serde(default)]
    pub best_air: f64,
    #[serde(default)]
    pub crashes: u32,
    /// Dollars to spend in the bike shop, from tips and deliveries.
    #[serde(default)]
    pub cash: u32,
    /// Dollars earned in all.
    #[serde(default)]
    pub cash_total: u32,
    /// Bikes and districts bought in the bike shop.
    #[serde(default)]
    pub bikes: Vec<Bike>,
    #[serde(default)]
    pub districts: Vec<District>,
    /// Deliveries made from the job board.
    #[serde(default)]
    pub deliveries: u32,
}

/// Something the bike shop sells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Bike(Bike),
    District(District),
}

impl Item {
    /// Everything for sale, cheapest first.
    pub fn all() -> Vec<Item> {
        let mut items: Vec<Item> = Bike::ALL
            .into_iter()
            .filter(|b| b.price() > 0)
            .map(Item::Bike)
            .chain(
                District::ALL
                    .into_iter()
                    .filter(|d| d.price() > 0)
                    .map(Item::District),
            )
            .collect();
        items.sort_by_key(|i| i.price());
        items
    }

    pub fn price(self) -> u32 {
        match self {
            Item::Bike(b) => b.price(),
            Item::District(d) => d.price(),
        }
    }

    /// "Mountain bike", "Riverside".
    pub fn name(self) -> String {
        match self {
            Item::Bike(b) => hearken::text::sentence_start(b.name()),
            Item::District(d) => d.name().to_string(),
        }
    }

    /// What the shop row says it gives, in a few words: "hops curbs".
    pub fn perk(self) -> &'static str {
        match self {
            Item::Bike(Bike::Road) => "fastest on the flat",
            Item::Bike(Bike::Bmx) => "quick start, big hop",
            Item::Bike(Bike::Mountain) => "hops curbs",
            Item::Bike(Bike::Electric) => "motor help",
            Item::Bike(Bike::Moped) => "an engine, no pedals",
            Item::District(_) => "",
        }
    }

    /// The shop row's name: "Mountain bike", "Riverside district".
    pub fn row_name(self) -> String {
        match self {
            Item::District(d) => format!("{} district", d.name()),
            Item::Bike(_) => self.name(),
        }
    }

    pub fn about(self) -> String {
        match self {
            Item::Bike(b) => b.about(),
            Item::District(d) => d.about().to_string(),
        }
    }
}

/// What came of trying to buy something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bought {
    Done,
    Owned,
    /// Dollars still needed.
    Short(u32),
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &format!("{ID}/progress"))
    }

    pub fn owns_bike(&self, bike: Bike) -> bool {
        bike.price() == 0 || self.bikes.contains(&bike)
    }

    pub fn owns_district(&self, district: District) -> bool {
        district.price() == 0 || self.districts.contains(&district)
    }

    pub fn owns(&self, item: Item) -> bool {
        match item {
            Item::Bike(b) => self.owns_bike(b),
            Item::District(d) => self.owns_district(d),
        }
    }

    /// Bikes the rider can choose, in shop order.
    pub fn my_bikes(&self) -> Vec<Bike> {
        Bike::ALL
            .into_iter()
            .filter(|&b| self.owns_bike(b))
            .collect()
    }

    pub fn my_districts(&self) -> Vec<District> {
        District::ALL
            .into_iter()
            .filter(|&d| self.owns_district(d))
            .collect()
    }

    /// Spends dollars on `item`, if there are enough and it is not owned.
    pub fn buy(&mut self, item: Item) -> Bought {
        if self.owns(item) {
            return Bought::Owned;
        }
        let price = item.price();
        if self.cash < price {
            return Bought::Short(price - self.cash);
        }
        self.cash -= price;
        match item {
            Item::Bike(b) => self.bikes.push(b),
            Item::District(d) => self.districts.push(d),
        }
        Bought::Done
    }

    /// Adds dollars from a ride.
    pub fn bank(&mut self, dollars: u32) {
        self.cash += dollars;
        self.cash_total += dollars;
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &format!("{ID}/progress"), self);
    }
}

/// The board an endless ride's best distance is kept on, in metres.
pub fn endless_board(district: District) -> String {
    format!("endless/{}", district.id())
}

/// "1.2 kilometres", "850 metres".
pub fn distance_words(m: f64) -> String {
    if m >= 1000.0 {
        units::long(m / 1000.0)
    } else {
        metres_words(m)
    }
}

/// "1 dollar", "12 dollars".
pub fn dollar_words(n: u32) -> String {
    match n {
        1 => "1 dollar".to_string(),
        n => format!("{n} dollars"),
    }
}

/// The records key for best times, in milliseconds, lower is better.
pub fn times_key() -> String {
    format!("{ID}/times")
}

/// The board a time trial is kept on: `tt/<course>/<bike>`.
pub fn board(course: &str, bike: Bike) -> String {
    format!("tt/{course}/{}", bike.id())
}

pub fn best_time(ctx: &mut Ctx<Parlor>, board: &str) -> Option<f64> {
    let times: BTreeMap<String, u64> = Records::load(ctx, &times_key());
    times.get(board).map(|&ms| ms as f64 / 1000.0)
}

/// What finishing a time trial changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    /// The old best time, seconds.
    pub previous: Option<f64>,
    /// Whether this time is the new best.
    pub record: bool,
    pub points: ScoreResult,
    /// The pace ridden: a relaxed ride keeps its own time and points.
    pub pace: Pace,
}

/// The best times on `board` and on its relaxed board.
pub fn best_times(ctx: &mut Ctx<Parlor>, board: &str) -> (Option<f64>, Option<f64>) {
    (
        best_time(ctx, board),
        best_time(ctx, &Pace::Relaxed.board(board)),
    )
}

/// Keeps a time trial's time, when it is the best, and its ride points,
/// on `board`'s own board for `pace`: relaxed rides keep their own.
pub fn save_time(ctx: &mut Ctx<Parlor>, board: &str, secs: f64, points: u64, pace: Pace) -> Saved {
    let board = &pace.board(board);
    let mut times: BTreeMap<String, u64> = Records::load(ctx, &times_key());
    let ms = (secs * 1000.0).round() as u64;
    let previous = times.get(board).copied();
    let record = previous.is_none_or(|p| ms < p);
    if record {
        times.insert(board.to_string(), ms);
        Records::store(ctx, &times_key(), &times);
    }
    let points = Records::record(ctx, ID, board, points);
    Saved {
        previous: previous.map(|p| p as f64 / 1000.0),
        record,
        points,
        pace,
    }
}

/// "1 minute 12.4 seconds", "48.0 seconds".
/// A whole-second span, for a deadline: "2 minutes 30 seconds",
/// "3 minutes", "45 seconds".
pub fn span_words(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    let (m, s) = (total / 60, total % 60);
    let mins = match m {
        1 => "1 minute".to_string(),
        m => format!("{m} minutes"),
    };
    let secs = match s {
        1 => "1 second".to_string(),
        s => format!("{s} seconds"),
    };
    match (m, s) {
        (0, _) => secs,
        (_, 0) => mins,
        _ => format!("{mins} {secs}"),
    }
}

pub fn time_words(secs: f64) -> String {
    let tenths = (secs.max(0.0) * 10.0).round() as u64;
    let minutes = tenths / 600;
    let rest = (tenths % 600) as f64 / 10.0;
    let secs = format!("{rest:.1} seconds");
    match minutes {
        0 => secs,
        1 => format!("1 minute {secs}"),
        m => format!("{m} minutes {secs}"),
    }
}

/// Speed in words: "29 kilometres an hour", or miles an hour with US units.
pub fn speed_words(v: f64) -> String {
    units::speed(v)
}

/// A speed in whole kilometres an hour, or miles an hour with US units.
pub fn kmh(v: f64) -> i64 {
    units::per_hour_value(v * 3.6)
}

/// "1 metre", "40 metres", rounded for speech; feet with US units.
pub fn metres_words(m: f64) -> String {
    units::length_rounded(m, 20.0)
}

/// "half a metre", "1.5 metres": a short lateral distance. With US
/// units, whole feet: "2 feet".
pub fn near_words(m: f64) -> String {
    if units::us() {
        return match units::from_metres(m.max(0.0)).round() as i64 {
            0 => "under 1 foot".to_string(),
            n => units::count(n),
        };
    }
    let halves = (m.max(0.0) * 2.0).round() as u32;
    match halves {
        0 => "under half a metre".to_string(),
        1 => "half a metre".to_string(),
        2 => "1 metre".to_string(),
        h if h % 2 == 0 => format!("{} metres", h / 2),
        h => format!("{}.5 metres", h / 2),
    }
}

/// "1 second", "2 seconds", to the nearest whole.
pub fn secs_words(s: f64) -> String {
    match s.round().max(1.0) as u32 {
        1 => "1 second".to_string(),
        n => format!("{n} seconds"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_read_well() {
        assert_eq!(time_words(48.04), "48.0 seconds");
        assert_eq!(time_words(72.44), "1 minute 12.4 seconds");
        assert_eq!(time_words(130.0), "2 minutes 10.0 seconds");
        assert_eq!(speed_words(8.0), "29 kilometres an hour");
        assert_eq!(span_words(150.0), "2 minutes 30 seconds");
        assert_eq!(span_words(180.0), "3 minutes");
        assert_eq!(span_words(61.0), "1 minute 1 second");
        assert_eq!(metres_words(1.2), "1 metre");
        assert_eq!(metres_words(47.0), "50 metres");
        assert_eq!(near_words(0.4), "half a metre");
        assert_eq!(near_words(1.6), "1.5 metres");
        assert_eq!(near_words(0.1), "under half a metre");
        assert_eq!(secs_words(0.3), "1 second");
        assert_eq!(CallOuts::Off.next(), CallOuts::All);
    }

    #[test]
    fn us_words_read_well() {
        units::set_current(units::Units::Us);
        assert_eq!(speed_words(8.0), "18 miles an hour");
        assert_eq!(metres_words(1.2), "4 feet");
        assert_eq!(metres_words(47.0), "150 feet");
        assert_eq!(near_words(0.4), "1 foot");
        assert_eq!(near_words(0.1), "under 1 foot");
        units::set_current(units::Units::Metric);
    }
}
