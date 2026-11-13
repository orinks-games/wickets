//! Against the clock: one clock that drops feed. Each drop made adds
//! seconds, a crash takes some away, and the ride ends when the clock runs
//! out. There is no crash limit and no finish line: the clock is the only
//! end, and the score is the drops made.
//!
//! The street is the endless ride's, with its hazards closing up the
//! further you go, but every drop has clear road around it, so a drop's
//! doorbell never rings while a hazard is warning.

use crate::course::{Course, District, ParcelSpot, BIKE_LANE_LINE, ENDLESS_M};
use parlor_core::Rng;

/// The mode's name, as the title and the ride say it.
pub const NAME: &str = "Against the clock";

/// The stage of the mode: it ships on alpha first.
pub const STAGE: parlor_core::channel::Stage = parlor_core::channel::Stage::Alpha;

/// Whether this copy offers the mode.
pub fn offered() -> bool {
    parlor_core::channel::shows(STAGE)
}

/// The records board for a district's best drops.
pub fn board(district: District) -> String {
    format!("clock/{}", district.id())
}

/// Seconds on the clock at the start.
pub const START_S: f64 = 60.0;
/// The clock never holds more than this, so time cannot be banked forever.
pub const MAX_S: f64 = 90.0;
/// Seconds a drop in a shop doorway adds.
pub const DOORWAY_S: f64 = 14.0;
/// Seconds a drop at a parcel locker, out by the centre line, adds.
pub const LOCKER_S: f64 = 18.0;
/// Extra seconds for a drop made this soon after the last one.
pub const QUICK_S: f64 = 4.0;
/// How soon after the last drop a drop counts as quick, seconds.
pub const QUICK_WINDOW: f64 = 30.0;
/// Seconds a crash takes off the clock.
pub const CRASH_S: f64 = 8.0;
/// Under this many seconds the clock ticks.
pub const LOW_S: f64 = 15.0;
/// Under this many seconds the tick doubles.
pub const LAST_S: f64 = 5.0;
/// The clock is called out as it falls past these.
pub const CALLS: [f64; 2] = [30.0, 10.0];
/// Clear road before and after every drop, metres: more than any bike
/// covers in a hazard's or a doorbell's warning lead, so the two never
/// overlap.
pub const DROP_CLEAR: f64 = 40.0;
/// A gap between hazards can hold a drop when the last drop is at least
/// this far behind the hazard that opens it, metres: so never the gap
/// straight after a drop.
pub const DROP_SPACING: f64 = 60.0;
/// How often a gap that can hold a drop does.
pub const DROP_CHANCE: f64 = 0.85;
/// A gap always holds a drop once the last is this far back, metres.
pub const DROP_DUE: f64 = 150.0;
/// Where the first drop is, metres from the start.
pub const FIRST_DROP: f64 = 30.0;

/// Where a drop sits: a shop doorway by the parked cars, or a parcel
/// locker on the traffic island.
pub fn is_locker(spot: &ParcelSpot) -> bool {
    spot.x > BIKE_LANE_LINE
}

/// The seconds a drop at `spot` adds, before any quick bonus.
pub fn drop_secs(spot: &ParcelSpot) -> f64 {
    if is_locker(spot) {
        LOCKER_S
    } else {
        DOORWAY_S
    }
}

/// What a drop did to the clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Added {
    /// Seconds added, after the cap.
    pub secs: f64,
    /// Whether it came soon enough after the last drop for the bonus.
    pub quick: bool,
}

/// The clock and its drops.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropClock {
    /// Seconds left.
    pub left: f64,
    /// Drops made.
    pub drops: u32,
    /// Of those, at parcel lockers.
    pub lockers: u32,
    /// Quick drops made.
    pub quick: u32,
    /// Seconds lost to crashes.
    pub lost: f64,
    /// The world's clock at the last drop.
    last_drop: Option<f64>,
}

impl Default for DropClock {
    fn default() -> Self {
        DropClock::new()
    }
}

impl DropClock {
    pub fn new() -> DropClock {
        DropClock {
            left: START_S,
            drops: 0,
            lockers: 0,
            quick: 0,
            lost: 0.0,
            last_drop: None,
        }
    }

    /// Runs the clock down by `dt`, and returns the call-out it fell past,
    /// if any. Never below zero.
    pub fn tick(&mut self, dt: f64) -> Option<f64> {
        let before = self.left;
        self.left = (self.left - dt).max(0.0);
        self.fell_past(before)
    }

    /// The call-out the clock fell past since it stood at `before`, by
    /// ticking or by a crash: the lowest, if it fell past more than one.
    pub fn fell_past(&self, before: f64) -> Option<f64> {
        CALLS
            .into_iter()
            .rev()
            .find(|&c| before > c && self.left <= c)
    }

    /// Whether the clock has run out.
    pub fn out(&self) -> bool {
        self.left <= 0.0
    }

    /// A drop made at `spot`, at world time `now`.
    pub fn drop_at(&mut self, spot: &ParcelSpot, now: f64) -> Added {
        let quick = self.last_drop.is_some_and(|t| now - t <= QUICK_WINDOW);
        let want = drop_secs(spot) + if quick { QUICK_S } else { 0.0 };
        let before = self.left;
        self.left = (self.left + want).min(MAX_S);
        self.drops += 1;
        self.lockers += is_locker(spot) as u32;
        self.quick += quick as u32;
        self.last_drop = Some(now);
        Added {
            secs: self.left - before,
            quick,
        }
    }

    /// A crash: takes `CRASH_S` off, or what is left. Returns the seconds
    /// taken.
    pub fn crash(&mut self) -> f64 {
        let lost = CRASH_S.min(self.left);
        self.left -= lost;
        self.lost += lost;
        lost
    }

    /// Seconds between ticks now, or `None` while the clock is above
    /// `LOW_S`.
    pub fn tick_gap(&self) -> Option<f64> {
        if self.left > LOW_S || self.out() {
            None
        } else if self.left > LAST_S {
            Some(1.0)
        } else {
            Some(0.5)
        }
    }
}

/// A street for a ride against the clock through `district`: the endless
/// ride's hazards, closing up the further you go, and drops in widened
/// gaps with `DROP_CLEAR` of clear road on each side.
pub fn course(district: District, seed: u64) -> Course {
    let base = Course::endless(district, seed);
    let mut rng = Rng::seeded(seed ^ 0x636c_6f63);
    let spot = |rng: &mut Rng| {
        let spots = crate::course::PARCEL_SPOTS;
        spots[rng.below(spots.len() as u32) as usize]
    };
    let mut hazards = Vec::with_capacity(base.hazards.len());
    // The first drop comes straight away, before the first hazard.
    let (x, value) = spot(&mut rng);
    let mut parcels = vec![ParcelSpot {
        s: FIRST_DROP,
        x,
        value,
    }];
    // Hazards keep their order and spacing; a drop pushes everything
    // after it along by the room it needs.
    let mut shift = (FIRST_DROP + DROP_CLEAR - base.hazards.first().map_or(0.0, |h| h.s)).max(0.0);
    let mut prev: Option<f64> = None;
    for h in &base.hazards {
        let s = h.s + shift;
        if let Some(p) = prev {
            let gap = s - p;
            let last = parcels.last().map_or(f64::NEG_INFINITY, |d| d.s);
            let extra = (2.0 * DROP_CLEAR - gap).max(0.0);
            let at = p + (gap + extra) / 2.0;
            // Never in the gap straight after a drop; then most gaps, and
            // always once the last drop is far back.
            let since = p - last;
            if since >= DROP_SPACING && (since >= DROP_DUE || rng.chance(DROP_CHANCE)) {
                let (x, value) = spot(&mut rng);
                parcels.push(ParcelSpot { s: at, x, value });
                shift += extra;
            }
        }
        let s = h.s + shift;
        if s > ENDLESS_M - 50.0 {
            break;
        }
        let mut h = *h;
        h.s = s;
        hazards.push(h);
        prev = Some(s);
    }
    parcels.retain(|p| p.s < ENDLESS_M - 50.0);
    Course {
        name: format!("Against the clock, {}", district.name()),
        hazards,
        parcels,
        ..base
    }
}

/// "1 drop", "14 drops".
pub fn drop_words(n: u32) -> String {
    match n {
        1 => "1 drop".to_string(),
        n => format!("{n} drops"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::MIN_ENDLESS_GAP;

    fn doorway() -> ParcelSpot {
        ParcelSpot {
            s: 0.0,
            x: crate::course::PARCEL_SPOTS[0].0,
            value: 3,
        }
    }

    fn locker() -> ParcelSpot {
        ParcelSpot {
            s: 0.0,
            x: crate::course::PARCEL_SPOTS[1].0,
            value: 6,
        }
    }

    #[test]
    fn drops_feed_the_clock_and_quick_ones_feed_it_more() {
        let mut c = DropClock::new();
        c.tick(20.0);
        let a = c.drop_at(&doorway(), 20.0);
        assert_eq!(
            a,
            Added {
                secs: DOORWAY_S,
                quick: false
            }
        );
        let a = c.drop_at(&locker(), 30.0);
        assert_eq!(
            a,
            Added {
                secs: LOCKER_S + QUICK_S,
                quick: true
            }
        );
        let a = c.drop_at(&doorway(), 30.0 + QUICK_WINDOW + 0.1);
        assert!(!a.quick);
        assert_eq!((c.drops, c.lockers, c.quick), (3, 1, 1));
    }

    #[test]
    fn the_clock_is_capped() {
        let mut c = DropClock::new();
        c.left = MAX_S - 2.0;
        assert_eq!(c.drop_at(&locker(), 1.0).secs, 2.0);
        assert_eq!(c.left, MAX_S);
    }

    #[test]
    fn a_crash_costs_seconds_but_never_below_zero() {
        let mut c = DropClock::new();
        assert_eq!(c.crash(), CRASH_S);
        assert_eq!(c.left, START_S - CRASH_S);
        c.left = 3.0;
        assert_eq!(c.crash(), 3.0);
        assert!(c.out());
    }

    #[test]
    fn the_clock_calls_30_and_10_once_and_ticks_low() {
        let mut c = DropClock::new();
        let mut calls = Vec::new();
        let mut t = 0.0;
        while !c.out() {
            if let Some(n) = c.tick(0.25) {
                calls.push(n);
            }
            t += 0.25;
        }
        assert_eq!(calls, vec![30.0, 10.0]);
        assert_eq!(t, START_S);
        let mut c = DropClock::new();
        assert_eq!(c.tick_gap(), None);
        c.left = 12.0;
        assert_eq!(c.tick_gap(), Some(1.0));
        c.left = 4.0;
        assert_eq!(c.tick_gap(), Some(0.5));
    }

    #[test]
    fn every_drop_has_clear_road_around_it() {
        for d in District::ALL {
            for seed in 0..20 {
                let c = course(d, seed);
                assert!(c.parcels.len() > 100, "{d:?} {seed}: {}", c.parcels.len());
                for p in &c.parcels {
                    let near = c
                        .hazards
                        .iter()
                        .map(|h| (h.s - p.s).abs())
                        .fold(f64::INFINITY, f64::min);
                    assert!(near >= DROP_CLEAR - 1e-6, "{d:?} {seed}: {near}");
                }
                assert_eq!(c.parcels[0].s, FIRST_DROP);
                for w in c.parcels.windows(2) {
                    let gap = w[1].s - w[0].s;
                    assert!(
                        (2.0 * DROP_CLEAR - 1e-6..300.0).contains(&gap),
                        "{d:?} {seed}: {gap}"
                    );
                }
                assert!(c
                    .hazards
                    .windows(2)
                    .all(|w| w[1].s - w[0].s >= MIN_ENDLESS_GAP - 1e-6));
            }
        }
    }
}
