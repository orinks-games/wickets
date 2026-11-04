//! All Hallows: Nevermore Manor's Halloween.
//!
//! For two weeks before Halloween the house stirs. Lantern Jack walks
//! the halls blowing out candles, and the title's "The house stirs" row
//! plays a haunted floor: each one cleared lights one of 13 candles. On
//! Halloween itself (from 30 October to 1 November, for time zones) the
//! Lantern Night opens: 13 floors in a row with the lights out, where
//! every candle you lit adds time to each haunt clock. Clearing it keeps
//! Lantern Jack's ember for the year.
//!
//! Everything here is a plain function of the date or the year, so tests
//! can pin it.

use hearken::Ctx;
use parlor_core::almanac::Window;
use parlor_core::calendar::{self, Holiday, Nearness};
use parlor_core::{Date, Happening, Parlor, Records, Rng};
use serde::{Deserialize, Serialize};

use crate::ID;

/// The holiday.
pub const HOLIDAY: Holiday = Holiday::Halloween;
/// The run-up row's name.
pub const STIRS: &str = "The house stirs";
/// The night's row and mode name.
pub const NIGHT: &str = "The Lantern Night";
/// Candles to light, and floors in the Lantern Night.
pub const CANDLES: u32 = 13;
/// Haunt seconds each lit candle adds to every Lantern Night floor.
pub const CANDLE_SECS: f64 = 10.0;
/// The best-score boards.
pub const STIRS_BOARD: &str = "house stirs";
pub const NIGHT_BOARD: &str = "lantern night";

/// Whether the Lantern Night is open on `date`.
pub fn is_night(date: Date) -> bool {
    calendar::is_day(HOLIDAY, date)
}

/// Whether the house stirs on `date`: the two weeks before Halloween, and
/// Halloween itself, so a late start can still light every candle.
pub fn stirs(date: Date) -> bool {
    calendar::nearness(HOLIDAY, date).is_some()
}

/// Days until Halloween, in the run-up.
pub fn days_to_go(date: Date) -> Option<i64> {
    match calendar::nearness(HOLIDAY, date) {
        Some(Nearness::Season { days }) => Some(days),
        _ => None,
    }
}

/// The year whose Halloween `date` belongs to.
pub fn year_of(date: Date) -> i32 {
    HOLIDAY.next(date).map_or(date.year, |(d, _)| d.year)
}

/// What the launcher names in the run-up and on the night.
pub fn event(date: Date) -> Option<Happening> {
    match calendar::nearness(HOLIDAY, date)? {
        Nearness::Day => Some(Happening::new(
            NIGHT,
            "Thirteen floors with the lights out against Lantern Jack, open until 1 November.",
        )),
        Nearness::Season { days } => Some(Happening::new(
            STIRS,
            format!(
                "Clear haunted floors to light candles for the Lantern Night, which opens {}.",
                calendar::days_phrase(days - 1)
            ),
        )),
    }
}

/// Whether the event on `date` is already finished: in the run-up, every
/// candle lit, so the title has no House stirs row left to play. The
/// Lantern Night can always be played again.
pub fn event_done(ctx: &mut Ctx<Parlor>, date: Date) -> bool {
    stirs(date) && !is_night(date) && Ledger::load(ctx).year(year_of(date)).candles >= CANDLES
}

/// The floor that lights candle `candle` (1 to 13) in `year`: the same
/// for every player, and deeper for later candles.
pub fn candle_floor(year: i32, candle: u32) -> u32 {
    let mut rng = Rng::seeded(0x4c41_4e54_0000_0000 ^ ((year as u64) << 8) ^ candle as u64);
    1 + (candle.clamp(1, CANDLES) - 1) * 2 + rng.below(3)
}

/// One year's Halloween.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Year {
    pub year: i32,
    pub candles: u32,
    /// Lantern Nights played to the end, won or not.
    pub nights: u32,
    /// Whether a Lantern Night was cleared: Lantern Jack's ember.
    pub ember: bool,
}

/// Every year's Halloween, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ledger {
    pub years: Vec<Year>,
}

/// The Records key.
pub fn key() -> String {
    format!("{ID}/allhallows")
}

impl Ledger {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Ledger {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    pub fn year(&self, year: i32) -> Year {
        self.years
            .iter()
            .find(|y| y.year == year)
            .cloned()
            .unwrap_or(Year {
                year,
                ..Default::default()
            })
    }

    fn year_mut(&mut self, year: i32) -> &mut Year {
        if !self.years.iter().any(|y| y.year == year) {
            self.years.push(Year {
                year,
                ..Default::default()
            });
            self.years.sort_by_key(|y| y.year);
        }
        let at = self.years.iter().position(|y| y.year == year).unwrap_or(0);
        &mut self.years[at]
    }

    /// Lights the next candle of `year`, and says how many are lit.
    pub fn light(&mut self, year: i32) -> u32 {
        let y = self.year_mut(year);
        y.candles = (y.candles + 1).min(CANDLES);
        y.candles
    }

    /// A Lantern Night played to the end. True the first time it is won.
    pub fn night_done(&mut self, year: i32, won: bool) -> bool {
        let y = self.year_mut(year);
        y.nights += 1;
        let first = won && !y.ember;
        y.ember |= won;
        first
    }

    /// The Records page.
    pub fn page(&self) -> String {
        if self.years.is_empty() {
            return "Halloween. Nothing yet: the house stirs two weeks before Halloween, and the Lantern Night opens on the night itself.".to_string();
        }
        let years: Vec<String> = self
            .years
            .iter()
            .map(|y| {
                let ember = if y.ember {
                    ", Lantern Jack's ember kept"
                } else {
                    ""
                };
                format!("{}, {} of {CANDLES} candles lit{ember}", y.year, y.candles)
            })
            .collect();
        format!("Halloween. {}.", years.join("; "))
    }
}

/// The run-up row's label, help and floor for `date`, given the year's
/// candles: None once all 13 are lit.
pub fn stirs_row(date: Date, lit: u32) -> Option<(String, String, u32)> {
    if !stirs(date) || lit >= CANDLES {
        return None;
    }
    let candle = lit + 1;
    let floor = candle_floor(year_of(date), candle);
    let when = night_when(date);
    Some((
        format!("{STIRS}, candle {candle} of {CANDLES}: floor {floor}"),
        format!("Lantern Jack is blowing out the house's candles. Clear floor {floor}, with restless ghosts, to light candle {candle}. {when}"),
        floor,
    ))
}

/// "The Lantern Night opens in 3 days." while the house stirs, or "The
/// Lantern Night is open now." on the night.
fn night_when(date: Date) -> String {
    match days_to_go(date) {
        Some(days) => format!(
            "The Lantern Night opens {}.",
            calendar::days_phrase(days - 1)
        ),
        None => "The Lantern Night is open now.".to_string(),
    }
}

/// The run-up row's label once every candle is lit while the house
/// stirs. It keeps its place on the title.
pub fn stirs_done_label() -> String {
    format!("{STIRS}, all {CANDLES} candles lit")
}

/// What Enter on the run-up row says once every candle is lit.
pub fn stirs_done_line(date: Date) -> String {
    format!("All {CANDLES} candles are lit. {}", night_when(date))
}

/// What Enter on the locked run-up row says, out of season.
pub fn stirs_locked_line(date: Date) -> String {
    format!("{STIRS} from {}.", Window::RunUp(HOLIDAY).next_words(date))
}

/// The locked run-up row's help, out of season.
pub fn stirs_locked_help(date: Date) -> String {
    format!(
        "From {}, two weeks before Halloween, clear haunted floors to light candles for the Lantern Night.",
        Window::RunUp(HOLIDAY).next_words(date)
    )
}

/// The Lantern Night row's help, open or not.
pub fn night_help(date: Date, lit: u32) -> String {
    if is_night(date) {
        format!(
            "{CANDLES} floors from floor 1 with the lights out and restless ghosts, and courage that lasts the night. {}",
            candles_words(lit)
        )
    } else {
        locked_line(date)
    }
}

/// What Enter on the locked row says. The night opens on 30 October,
/// the day before Halloween.
pub fn locked_line(date: Date) -> String {
    match days_to_go(date) {
        Some(days) => format!(
            "{NIGHT} opens {}, the night before Halloween. Light candles for it while the house stirs.",
            calendar::days_phrase(days - 1)
        ),
        None => format!(
            "{NIGHT} opens {}.",
            Window::Day(HOLIDAY).next_words(date)
        ),
    }
}

/// What the candles lit add: "Your 1 candle adds 10 seconds to every
/// haunt clock."
pub fn candles_words(lit: u32) -> String {
    match lit {
        0 => "You lit no candles, so the haunt clocks are as usual.".to_string(),
        1 => format!("Your 1 candle adds {} to every haunt clock.", secs_words(1)),
        n => format!(
            "Your {n} candles add {} to every haunt clock.",
            secs_words(n)
        ),
    }
}

/// "no time", "10 seconds" or "2 minutes 10 seconds", for `lit` candles.
pub fn secs_words(lit: u32) -> String {
    if lit == 0 {
        "no time".to_string()
    } else {
        crate::rules::clock_words(lit as f64 * CANDLE_SECS)
    }
}

/// What the run-up floor says before its first room.
pub fn stirs_opening() -> String {
    "The house stirs: Lantern Jack is blowing out candles, and the ghosts are restless. Clear this floor to light a candle.".to_string()
}

/// What the Lantern Night says before its first room.
pub fn night_opening(lit: u32) -> String {
    let candles = candles_words(lit);
    format!("The Lantern Night. Lantern Jack has put out every light in the house, so things call out softly and the ghosts are restless. {CANDLES} floors from floor 1, and your courage lasts the night. {candles}")
}

/// The line after a run-up floor is cleared.
pub fn lit_line(lit: u32) -> String {
    if lit >= CANDLES {
        format!("Candle {CANDLES} of {CANDLES} is lit. Every candle will burn for you on the Lantern Night.")
    } else {
        format!("Candle {lit} of {CANDLES} is lit. Somewhere above you, Lantern Jack laughs.")
    }
}

/// The line after the Lantern Night is cleared.
pub fn ember_line(first: bool) -> String {
    if first {
        "On the last stair you find Lantern Jack's turnip lantern, still warm, and Jack is gone with the dawn. You keep his ember, and it is in Records under Halloween.".to_string()
    } else {
        "On the last stair you find Lantern Jack's turnip lantern again, still warm. Jack is gone with the dawn.".to_string()
    }
}
