//! What a player's unlocks and chosen modifiers change about a new run.
//! The rules read them and never the save, so a replay carries its own.

use serde::{Deserialize, Serialize};

/// The shield's cap under Hard vacuum.
pub const HARD_VACUUM_SHIELD: u32 = 30;
/// The shield's cap with the bigger shield unlocked.
pub const BIG_SHIELD: u32 = 70;
/// Dome integrity with the thicker dome unlocked.
pub const THICK_DOME: u32 = 110;
/// Missiles the deeper rack holds.
pub const DEEP_RACK: u32 = 5;
/// Mines that may be laid for one night, and with the unlock.
pub const MINES_A_NIGHT: u32 = 1;
pub const MINES_A_NIGHT_MORE: u32 = 2;
/// Score bonus, in percent, for Hard vacuum and for playing with the
/// call-outs off.
pub const BONUS_PERCENT: u32 = 25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Perks {
    /// Slots open past the first three: 0, 1 or 2.
    #[serde(default)]
    pub extra_slots: u8,
    #[serde(default)]
    pub big_shield: bool,
    #[serde(default)]
    pub thick_dome: bool,
    /// Missiles in the rack at the start.
    #[serde(default)]
    pub start_missiles: u32,
    /// Two mines may be laid a night instead of one.
    #[serde(default)]
    pub more_mines: bool,
    #[serde(default)]
    pub deep_rack: bool,
    /// The forecast names the lanes without a scanner.
    #[serde(default)]
    pub lane_forecast: bool,
    /// The forecast names quirks without a scanner.
    #[serde(default)]
    pub forecast_quirks: bool,
    /// The shield holds 30 at most, for a quarter more score.
    #[serde(default)]
    pub hard_vacuum: bool,
    /// Played with the call-outs off, for a quarter more score.
    #[serde(default)]
    pub quiet: bool,
}

impl Perks {
    pub fn shield_max(self) -> u32 {
        if self.hard_vacuum {
            HARD_VACUUM_SHIELD
        } else if self.big_shield {
            BIG_SHIELD
        } else {
            super::colony::SHIELD_MAX
        }
    }

    pub fn dome_max(self) -> u32 {
        if self.thick_dome {
            THICK_DOME
        } else {
            super::colony::DOME_MAX
        }
    }

    pub fn missiles_max(self) -> u32 {
        if self.deep_rack {
            DEEP_RACK
        } else {
            super::colony::MISSILES_MAX
        }
    }

    /// Mines that may be laid before one night.
    pub fn mines_a_night(self) -> u32 {
        if self.more_mines {
            MINES_A_NIGHT_MORE
        } else {
            MINES_A_NIGHT
        }
    }

    /// The score bonus in percent: a quarter for Hard vacuum, a quarter for
    /// the call-outs off.
    pub fn bonus_percent(self) -> u32 {
        BONUS_PERCENT * (u32::from(self.hard_vacuum) + u32::from(self.quiet))
    }
}
