//! The route finder: A star over the grid's sectors, one step a move,
//! with costs that differ by raider. It is what H speaks, and what the
//! raiders' phase walks, so the two never disagree.
//!
//! A raider never steps back toward the horizon: a move is down, left or
//! right. Equal-cost routes are settled the same way every time (down
//! before sideways, then the raider's lean), so a replay finds the same
//! route and a player who has heard a route once can predict the next.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use super::grid::{Grid, Sector, COLS, NEAR_ROW};
use super::raiders::{Kind, Raider};

/// What a raider is heading for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Goal {
    /// Any sector of the near row.
    NearRow,
    /// The near row under one of these columns.
    Columns(Vec<u8>),
    /// One sector, such as a mothership's post.
    At(Sector),
}

impl Goal {
    /// Whether a raider standing at `at` has arrived.
    pub fn reached(&self, at: Sector) -> bool {
        match self {
            Goal::NearRow => at.row == NEAR_ROW,
            Goal::Columns(cols) => at.row == NEAR_ROW && cols.contains(&at.col),
            Goal::At(s) => at == *s,
        }
    }

    /// A guess at the steps left that never overshoots.
    fn guess(&self, at: Sector) -> u32 {
        match self {
            Goal::NearRow => (at.row - NEAR_ROW) as u32,
            Goal::Columns(cols) => {
                let across = cols
                    .iter()
                    .map(|c| c.abs_diff(at.col) as u32)
                    .min()
                    .unwrap_or(0);
                (at.row - NEAR_ROW) as u32 + across
            }
            Goal::At(s) => at.distance(*s),
        }
    }
}

/// What a step costs a raider, by kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Costs {
    /// Entering a wall's sector, counting the bashes to bring it down;
    /// `None` means the raider will not go through a wall.
    pub wall: Option<u32>,
    /// Added to a sector a powered flak post covers.
    pub flak_cover: u32,
    /// Added to every sector of the gun's column.
    pub gun_column: u32,
}

impl Costs {
    /// A raider's costs when it has an open route.
    pub fn open(kind: Kind) -> Costs {
        match kind {
            Kind::Drifter => Costs {
                wall: None,
                flak_cover: 2,
                gun_column: 0,
            },
            Kind::Skitter => Costs {
                wall: None,
                flak_cover: 0,
                gun_column: 2,
            },
            Kind::Hulk | Kind::Mothership => Costs {
                wall: Some(3),
                flak_cover: 0,
                gun_column: 0,
            },
            Kind::Sapper => Costs {
                wall: None,
                flak_cover: 0,
                gun_column: 0,
            },
            Kind::Mule | Kind::Carrier => Costs {
                wall: None,
                flak_cover: 0,
                gun_column: 0,
            },
        }
    }

    /// A raider's costs when every route is walled off and it must bash.
    /// A skitter never prefers a wall, but walled in it bashes like the
    /// rest, so a full row of walls buys bashing time, never a night.
    pub fn walled(kind: Kind) -> Costs {
        let mut costs = Costs::open(kind);
        costs.wall = Some(3);
        costs
    }
}

/// The goal of a raider of `kind`, given the columns with a building and
/// the ones with hydroponics.
pub fn goal(kind: Kind, buildings: &[u8], hydroponics: &[u8]) -> Goal {
    match kind {
        Kind::Sapper if !hydroponics.is_empty() => Goal::Columns(hydroponics.to_vec()),
        Kind::Sapper if !buildings.is_empty() => Goal::Columns(buildings.to_vec()),
        Kind::Mothership => Goal::At(super::grid::Sector::new(super::grid::CENTRE, 3)),
        _ => Goal::NearRow,
    }
}

/// The sectors a raider at `from` walks to reach `goal`, nearest first,
/// not counting `from`. `None` when no route exists under these costs.
pub fn find(
    grid: &Grid,
    from: Sector,
    goal: &Goal,
    costs: Costs,
    gun: u8,
    lean: i8,
) -> Option<Vec<Sector>> {
    if goal.reached(from) {
        return Some(Vec::new());
    }
    let mut open = BinaryHeap::new();
    let mut best: HashMap<Sector, u32> = HashMap::new();
    let mut came: HashMap<Sector, Sector> = HashMap::new();
    let mut seq = 0u32;
    best.insert(from, 0);
    open.push(Reverse((goal.guess(from), 0u32, seq, from)));
    while let Some(Reverse((_, g, _, at))) = open.pop() {
        if best.get(&at).is_some_and(|&b| g > b) {
            continue;
        }
        if goal.reached(at) {
            let mut path = vec![at];
            let mut cur = at;
            while let Some(&prev) = came.get(&cur) {
                if prev == from {
                    break;
                }
                path.push(prev);
                cur = prev;
            }
            path.reverse();
            return Some(path);
        }
        for next in neighbours(at, lean) {
            let Some(step) = step_cost(grid, next, costs, gun) else {
                continue;
            };
            let ng = g + step;
            if best.get(&next).is_none_or(|&b| ng < b) {
                best.insert(next, ng);
                came.insert(next, at);
                seq += 1;
                open.push(Reverse((ng + goal.guess(next), ng, seq, next)));
            }
        }
    }
    None
}

/// Down first, then the side the raider leans to.
fn neighbours(at: Sector, lean: i8) -> Vec<Sector> {
    let sides = if lean > 0 {
        [at.right(), at.left()]
    } else {
        [at.left(), at.right()]
    };
    [at.down(), sides[0], sides[1]]
        .into_iter()
        .flatten()
        .collect()
}

fn step_cost(grid: &Grid, into: Sector, costs: Costs, gun: u8) -> Option<u32> {
    let mut cost = 1;
    if grid.is_wall(into) {
        cost = costs.wall?;
    }
    if costs.flak_cover > 0 && grid.flak_covers(into) {
        cost += costs.flak_cover;
    }
    if costs.gun_column > 0 && into.col == gun && gun < COLS {
        cost += costs.gun_column;
    }
    Some(cost)
}

/// The route a raider will walk: its open route, or, with every way
/// walled off, the one with the fewest bashes. `None` only when it stands
/// on no grid at all.
pub fn plan(grid: &Grid, raider: &Raider, goal: &Goal, gun: u8) -> Option<Vec<Sector>> {
    let open = find(
        grid,
        raider.at,
        goal,
        Costs::open(raider.kind),
        gun,
        raider.lean,
    );
    if open.is_some() {
        return open;
    }
    find(
        grid,
        raider.at,
        goal,
        Costs::walled(raider.kind),
        gun,
        raider.lean,
    )
}

/// Raiders' phases until a raider walking `route` reaches its end, given
/// its steps a phase and whether a hulk rests next phase.
pub fn phases_to_land(kind: Kind, rest: bool, route_len: usize) -> u32 {
    let steps = route_len as u32;
    if steps == 0 {
        return 0;
    }
    match kind {
        Kind::Hulk => steps * 2 - if rest { 0 } else { 1 },
        _ => steps.div_ceil(kind.steps() as u32),
    }
}
