//! The dust storm: a lane of dust that drifts across the sky over the
//! night, one lane every few seconds, far left to far right and back.
//!
//! In the lane it covers, raiders are heard by nobody: no engine, no
//! call, no scan. They fall on regardless. The flak post under it is
//! silent.

use serde::{Deserialize, Serialize};

use super::lanes::{ticks, LANES};

/// Ticks the storm rests on a lane before drifting to the next.
pub const STORM_EVERY: u32 = ticks(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Storm {
    pub lane: u8,
    /// 1 toward the far right, -1 toward the far left.
    pub dir: i8,
}

impl Default for Storm {
    fn default() -> Self {
        Storm::new()
    }
}

impl Storm {
    /// A storm at the far left, heading right.
    pub fn new() -> Storm {
        Storm { lane: 0, dir: 1 }
    }

    /// Whether the storm covers `lane`.
    pub fn covers(self, lane: u8) -> bool {
        lane == self.lane
    }

    /// Drifts to the next lane, turning at the edges.
    pub fn advance(&mut self) {
        let next = self.lane as i8 + self.dir;
        if (0..LANES as i8).contains(&next) {
            self.lane = next as u8;
        } else {
            self.dir = -self.dir;
            self.lane = (self.lane as i8 + self.dir) as u8;
        }
    }
}
