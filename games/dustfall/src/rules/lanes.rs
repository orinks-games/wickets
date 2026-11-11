//! The five lanes the raiders fall down, and the range along each: the
//! horizon at the top, the dome at the bottom.

use serde::{Deserialize, Serialize};

/// Lanes across the sky.
pub const LANES: u8 = 5;
/// The lane dead ahead of the dome.
pub const CENTRE: u8 = 2;

/// Range units from the dome to the horizon.
pub const HORIZON: u32 = 1000;
/// Below this a raider is at mid range: drifters wind up, mines go off,
/// the mothership holds, a mule crosses.
pub const MID: u32 = 600;
/// Below this a raider is near: hulks siphon, the danger layer joins.
pub const NEAR: u32 = 300;
/// A flak post reaches raiders nearer than this.
pub const FLAK_REACH: u32 = 850;
/// Where the carrier crosses: just inside mid range, where the gun bites.
pub const CARRIER_HEIGHT: u32 = 580;

/// Rules ticks a second. Every speed and timer is in ticks.
pub const TICKS_A_SECOND: u32 = 10;

/// Ticks for `secs` seconds.
pub const fn ticks(secs: u32) -> u32 {
    secs * TICKS_A_SECOND
}

/// The lane's name, as the player hears it.
pub fn lane_name(lane: u8) -> &'static str {
    match lane {
        0 => "far left",
        1 => "left",
        2 => "centre",
        3 => "right",
        _ => "far right",
    }
}

/// How far down a raider is, in the four words the player hears.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Range {
    Far,
    Mid,
    Near,
    /// At the dome: it breaches this tick.
    Dome,
}

impl Range {
    pub fn of(height: u32) -> Range {
        if height == 0 {
            Range::Dome
        } else if height <= NEAR {
            Range::Near
        } else if height <= MID {
            Range::Mid
        } else {
            Range::Far
        }
    }

    /// "far", "mid range", "near", "at the dome".
    pub fn name(self) -> &'static str {
        match self {
            Range::Far => "far",
            Range::Mid => "mid range",
            Range::Near => "near",
            Range::Dome => "at the dome",
        }
    }
}

/// "Hulk, far left, near": a raider's place in words.
pub fn place_words(lane: u8, height: u32) -> String {
    format!("{}, {}", lane_name(lane), Range::of(height).name())
}
