//! The dust storm: a column of dust that crosses the grid over the night,
//! one column a raiders' phase, far left to far right and back.
//!
//! In the column it covers, raiders are hidden from the scan and from H,
//! they lose their step, flak posts cannot fire, and a wall takes one bash
//! less to fall.

use serde::{Deserialize, Serialize};

use super::grid::{Sector, COLS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Storm {
    pub col: u8,
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
        Storm { col: 0, dir: 1 }
    }

    /// Whether the storm covers `at`.
    pub fn covers(self, at: Sector) -> bool {
        at.col == self.col
    }

    /// One raiders' phase on: the next column, turning at the edges.
    pub fn advance(&mut self) {
        let next = self.col as i8 + self.dir;
        if (0..COLS as i8).contains(&next) {
            self.col = next as u8;
        } else {
            self.dir = -self.dir;
            self.col = (self.col as i8 + self.dir) as u8;
        }
    }
}
