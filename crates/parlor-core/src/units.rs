//! Metric or US units, one setting for the whole collection.
//!
//! Games keep their rules in metres and metres a second, and only the
//! words change: [`length`], [`speed`] and the rest read the player's
//! choice. The choice is saved in records (`parlor/units`) and kept in a
//! per-thread copy, so word functions with no `Ctx` can read it.
//! [`sync`] refreshes that copy; the launcher and every title call it
//! through `menu_sounds`, and the Settings row calls it on each change.

use std::cell::Cell;

use hearken::screens::SettingsRow;
use hearken::Ctx;
use serde::{Deserialize, Serialize};

use crate::menus::settings;
use crate::{Parlor, Records};

/// The records key the choice is saved under.
pub const KEY: &str = "parlor/units";
/// The Settings row's id. A game that speaks measurements lists
/// [`row`] among its own settings, and the shared copy is then left out.
pub const ROW_ID: &str = "units";

const FEET_PER_METRE: f64 = 3.280_84;
const MILES_PER_KM: f64 = 0.621_371;
const FEET_PER_MILE: f64 = 5280.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Units {
    #[default]
    Metric,
    Us,
}

thread_local! {
    static CURRENT: Cell<Units> = const { Cell::new(Units::Metric) };
}

/// The units words use now.
pub fn current() -> Units {
    CURRENT.with(Cell::get)
}

/// Whether words use feet, miles and miles an hour.
pub fn us() -> bool {
    current() == Units::Us
}

/// Sets the units words use, without saving: for tests, and for [`sync`].
pub fn set_current(units: Units) {
    CURRENT.with(|c| c.set(units));
}

/// Reads the saved choice into the copy words use.
pub fn sync(ctx: &mut Ctx<Parlor>) {
    set_current(Records::load(ctx, KEY));
}

/// Saves a new choice and uses it at once.
pub fn store(ctx: &mut Ctx<Parlor>, units: Units) {
    Records::store(ctx, KEY, &units);
    set_current(units);
}

/// Metres in the current units: feet for US.
pub fn from_metres(m: f64) -> f64 {
    if us() {
        m * FEET_PER_METRE
    } else {
        m
    }
}

/// A whole number of the current length unit: "1 metre", "40 metres",
/// "1 foot", "130 feet".
pub fn count(n: i64) -> String {
    match (us(), n) {
        (false, 1) => "1 metre".to_string(),
        (false, n) => format!("{n} metres"),
        (true, 1) => "1 foot".to_string(),
        (true, n) => format!("{n} feet"),
    }
}

/// The length unit's plural: "metres" or "feet".
pub fn plural() -> &'static str {
    if us() {
        "feet"
    } else {
        "metres"
    }
}

/// A length in whole metres or feet: "4 metres", "13 feet".
pub fn length(m: f64) -> String {
    count(from_metres(m).round() as i64)
}

/// A length rounded for speech: whole units up close, tens beyond
/// `tens_from` metres (or the same distance in feet).
pub fn length_rounded(m: f64, tens_from: f64) -> String {
    let n = from_metres(m.max(0.0));
    if n >= from_metres(tens_from) {
        count(((n / 10.0).round() * 10.0) as i64)
    } else {
        count(n.round() as i64)
    }
}

/// A route or course length: "600 metres", "2200 metres" or "3 kilometres"
/// (only a whole number of kilometres from 2), "550 feet" or "1.9 miles" (miles from 1000 feet).
pub fn route(m: f64) -> String {
    if us() {
        let feet = m * FEET_PER_METRE;
        if feet < 1000.0 {
            count(((feet / 10.0).round() * 10.0) as i64)
        } else {
            miles(feet / FEET_PER_MILE)
        }
    } else if m >= 2000.0 && m.round() % 1000.0 == 0.0 {
        kilometres(m / 1000.0)
    } else {
        count(m.round() as i64)
    }
}

/// A long distance from kilometres, to one decimal: "12.4 kilometres" or
/// "7.7 miles".
pub fn long(km: f64) -> String {
    if us() {
        miles(km * MILES_PER_KM)
    } else {
        kilometres(km)
    }
}

fn kilometres(km: f64) -> String {
    let km = tenths(km);
    if km == "1" {
        "1 kilometre".to_string()
    } else {
        format!("{km} kilometres")
    }
}

fn miles(mi: f64) -> String {
    let mi = tenths(mi);
    if mi == "1" {
        "1 mile".to_string()
    } else {
        format!("{mi} miles")
    }
}

/// One decimal, with no ".0".
fn tenths(x: f64) -> String {
    let s = format!("{x:.1}");
    s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
}

/// Kilometres an hour as a whole number in the current units.
pub fn per_hour_value(kmh: f64) -> i64 {
    if us() {
        (kmh * MILES_PER_KM).round() as i64
    } else {
        kmh.round() as i64
    }
}

/// "kilometres an hour" or "miles an hour".
pub fn per_hour_unit() -> &'static str {
    if us() {
        "miles an hour"
    } else {
        "kilometres an hour"
    }
}

/// A road speed from kilometres an hour: "29 kilometres an hour" or
/// "18 miles an hour".
pub fn per_hour(kmh: f64) -> String {
    format!("{} {}", per_hour_value(kmh), per_hour_unit())
}

/// A speed from metres a second as a road speed, as [`per_hour`].
pub fn speed(m_per_s: f64) -> String {
    per_hour(m_per_s * 3.6)
}

/// A slow speed in metres or feet a second, with `decimals` places:
/// "2.4 metres a second", "7.9 feet a second".
pub fn per_second(m_per_s: f64, decimals: usize) -> String {
    format!(
        "{:.*} {} a second",
        decimals,
        from_metres(m_per_s),
        plural()
    )
}

/// The Units row for the Settings screen.
pub fn row() -> SettingsRow<Parlor> {
    settings::setting(
        ROW_ID,
        |ctx| label(Records::peek(ctx, KEY)),
        |ctx, _delta| {
            let next = match Records::peek::<Units>(ctx, KEY) {
                Units::Metric => Units::Us,
                Units::Us => Units::Metric,
            };
            store(ctx, next);
            Some(match next {
                Units::Metric => "Units: metric. Metres, kilometres and kilometres an hour.",
                Units::Us => "Units: US. Feet, miles and miles an hour.",
            }
            .to_string())
        },
        "How games say distances and speeds: metric, or US feet and miles. Enter, Left or Right changes it.",
    )
}

fn label(units: Units) -> String {
    match units {
        Units::Metric => "Units: metric".to_string(),
        Units::Us => "Units: US".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_words() {
        set_current(Units::Metric);
        assert_eq!(length(1.2), "1 metre");
        assert_eq!(length_rounded(47.0, 20.0), "50 metres");
        assert_eq!(route(600.0), "600 metres");
        assert_eq!(route(3000.0), "3 kilometres");
        assert_eq!(route(2200.0), "2200 metres");
        assert_eq!(long(12.44), "12.4 kilometres");
        assert_eq!(speed(8.0), "29 kilometres an hour");
        assert_eq!(per_second(2.44, 1), "2.4 metres a second");
    }

    #[test]
    fn us_words() {
        set_current(Units::Us);
        assert_eq!(length(0.3), "1 foot");
        assert_eq!(length(4.0), "13 feet");
        assert_eq!(length_rounded(47.0, 20.0), "150 feet");
        assert_eq!(route(150.0), "490 feet");
        assert_eq!(route(3000.0), "1.9 miles");
        assert_eq!(route(1609.0), "1 mile");
        assert_eq!(long(12.44), "7.7 miles");
        assert_eq!(speed(8.0), "18 miles an hour");
        assert_eq!(per_hour(37.0), "23 miles an hour");
        assert_eq!(per_second(2.44, 1), "8.0 feet a second");
        set_current(Units::Metric);
    }
}
