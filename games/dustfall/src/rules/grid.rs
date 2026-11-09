//! The approach grid: five columns across, six rows deep, the domes along
//! the near edge, and the works (walls, flak posts, mines) placed on it.

use serde::{Deserialize, Serialize};

/// Columns across the grid, far left to far right.
pub const COLS: u8 = 5;
/// Rows deep. Row 1 is the near row, where a raider breaches; row
/// [`FAR_ROW`] is the horizon, where raiders arrive.
pub const ROWS: u8 = 6;
/// The row a raider breaches from.
pub const NEAR_ROW: u8 = 1;
/// The horizon.
pub const FAR_ROW: u8 = ROWS;
/// The centre column's index.
pub const CENTRE: u8 = 2;
/// Works go in rows 2 to 5; the near row and the horizon stay open.
pub const WORK_ROWS: std::ops::RangeInclusive<u8> = 2..=5;
/// The row mules and the carrier cross along.
pub const CROSSING_ROW: u8 = 4;
/// A wall takes this many bashes before it falls.
pub const WALL_HITS: u8 = 2;
/// A mine's damage to the raider that steps on it.
pub const MINE_DAMAGE: u8 = 2;

/// The name of a column, as every line says it.
pub fn column_name(col: u8) -> &'static str {
    match col {
        0 => "far left",
        1 => "left",
        2 => "centre",
        3 => "right",
        _ => "far right",
    }
}

/// One cell of the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Sector {
    pub col: u8,
    pub row: u8,
}

impl Sector {
    pub const fn new(col: u8, row: u8) -> Sector {
        Sector { col, row }
    }

    /// Whether the sector is on the grid.
    pub fn valid(self) -> bool {
        self.col < COLS && (NEAR_ROW..=FAR_ROW).contains(&self.row)
    }

    /// The sector one row nearer the domes.
    pub fn down(self) -> Option<Sector> {
        (self.row > NEAR_ROW).then(|| Sector::new(self.col, self.row - 1))
    }

    pub fn left(self) -> Option<Sector> {
        (self.col > 0).then(|| Sector::new(self.col - 1, self.row))
    }

    pub fn right(self) -> Option<Sector> {
        (self.col + 1 < COLS).then(|| Sector::new(self.col + 1, self.row))
    }

    /// The sector and its four neighbours, as a flak post covers them.
    pub fn around(self) -> Vec<Sector> {
        let mut out = vec![self];
        let up = (self.row < FAR_ROW).then(|| Sector::new(self.col, self.row + 1));
        for s in [self.down(), self.left(), self.right(), up]
            .into_iter()
            .flatten()
        {
            out.push(s);
        }
        out
    }

    /// Steps between two sectors, as a raider walks them.
    pub fn distance(self, other: Sector) -> u32 {
        self.col.abs_diff(other.col) as u32 + self.row.abs_diff(other.row) as u32
    }

    /// "centre, row 3".
    pub fn words(self) -> String {
        format!("{}, row {}", column_name(self.col), self.row)
    }

    fn index(self) -> usize {
        self.col as usize * ROWS as usize + (self.row - NEAR_ROW) as usize
    }
}

/// The kinds of work a player places on the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkKind {
    Wall,
    Flak,
    Mine,
}

impl WorkKind {
    pub const ALL: [WorkKind; 3] = [WorkKind::Wall, WorkKind::Flak, WorkKind::Mine];

    pub fn name(self) -> &'static str {
        match self {
            WorkKind::Wall => "wall",
            WorkKind::Flak => "flak post",
            WorkKind::Mine => "mine",
        }
    }

    /// Metal to place one.
    pub fn cost(self) -> u32 {
        match self {
            WorkKind::Wall => 3,
            WorkKind::Flak => 6,
            WorkKind::Mine => 2,
        }
    }

    /// Power drawn each night.
    pub fn upkeep(self) -> u32 {
        match self {
            WorkKind::Flak => 1,
            _ => 0,
        }
    }

    /// A fresh work of this kind.
    pub fn build(self) -> Work {
        match self {
            WorkKind::Wall => Work::Wall { hits: WALL_HITS },
            WorkKind::Flak => Work::Flak { powered: true },
            WorkKind::Mine => Work::Mine,
        }
    }
}

/// A work on the grid, with its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Work {
    /// Blocks the sector until bashed down.
    Wall { hits: u8 },
    /// Fires once a raiders' phase while powered.
    Flak { powered: bool },
    /// Goes off under the first raider to step here.
    Mine,
}

impl Work {
    pub fn kind(self) -> WorkKind {
        match self {
            Work::Wall { .. } => WorkKind::Wall,
            Work::Flak { .. } => WorkKind::Flak,
            Work::Mine => WorkKind::Mine,
        }
    }
}

/// Why a work cannot go where the player asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    /// Row 1 and row 6 stay open.
    RowOpen,
    /// Something is there already.
    Taken,
}

/// The grid's works.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grid {
    cells: Vec<Option<Work>>,
}

impl Default for Grid {
    fn default() -> Self {
        Grid::new()
    }
}

impl Grid {
    pub fn new() -> Grid {
        Grid {
            cells: vec![None; COLS as usize * ROWS as usize],
        }
    }

    pub fn get(&self, at: Sector) -> Option<Work> {
        self.cells.get(at.index()).copied().flatten()
    }

    pub fn set(&mut self, at: Sector, work: Option<Work>) {
        if at.valid() {
            self.cells[at.index()] = work;
        }
    }

    /// Whether a work can be placed at `at`.
    pub fn can_place(&self, at: Sector) -> Result<(), Blocked> {
        if !at.valid() || !WORK_ROWS.contains(&at.row) {
            return Err(Blocked::RowOpen);
        }
        if self.get(at).is_some() {
            return Err(Blocked::Taken);
        }
        Ok(())
    }

    pub fn is_wall(&self, at: Sector) -> bool {
        matches!(self.get(at), Some(Work::Wall { .. }))
    }

    /// Whether a powered flak post covers `at`: one stands there or in one
    /// of the four sectors around it.
    pub fn flak_covers(&self, at: Sector) -> bool {
        at.around()
            .into_iter()
            .any(|s| matches!(self.get(s), Some(Work::Flak { powered: true })))
    }

    /// Every work, far left to far right, near row first in each column.
    pub fn works(&self) -> Vec<(Sector, Work)> {
        let mut out = Vec::new();
        for col in 0..COLS {
            for row in NEAR_ROW..=FAR_ROW {
                let at = Sector::new(col, row);
                if let Some(w) = self.get(at) {
                    out.push((at, w));
                }
            }
        }
        out
    }

    /// The sectors of powered flak posts, in grid order.
    pub fn powered_flak(&self) -> Vec<Sector> {
        self.works()
            .into_iter()
            .filter(|(_, w)| matches!(w, Work::Flak { powered: true }))
            .map(|(s, _)| s)
            .collect()
    }

    /// Bashes the wall at `at` once. Returns whether it fell.
    pub fn bash(&mut self, at: Sector) -> bool {
        match self.get(at) {
            Some(Work::Wall { hits }) if hits > 1 => {
                self.set(at, Some(Work::Wall { hits: hits - 1 }));
                false
            }
            Some(Work::Wall { .. }) => {
                self.set(at, None);
                true
            }
            _ => false,
        }
    }
}
