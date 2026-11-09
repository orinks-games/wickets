//! The raiders: what comes down out of the dust, and the state each one is
//! in.
//!
//! A raider's life is one small state machine, moved only by
//! [`super::Run`] in the raiders' phase:
//!
//! ```text
//! arrive -> Approaching --(in range of the gun)--> Charging --(fires)--> Approaching
//!              |  ^                                   |
//!              |  +--(wall falls)-- Bashing <---------+ (a wall in its way)
//!              |
//!              +--(no open route)--> Holding --(a route opens)--> Approaching
//!
//! Any state leaves the grid as Down (shot, mined or flakked), Breached
//! (reached row 1) or Gone (a mule that crossed).
//! ```
//!
//! A mothership that has reached its post, and the carrier and mules,
//! which cross rather than approach, use Holding and Approaching too; their
//! habits live in the step logic, not in extra states.

use serde::{Deserialize, Serialize};

use super::grid::Sector;

/// What a downed raider leaves behind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Take {
    pub power: u32,
    pub metal: u32,
    pub food: u32,
}

impl Take {
    pub const fn new(power: u32, metal: u32, food: u32) -> Take {
        Take { power, metal, food }
    }

    pub fn is_empty(self) -> bool {
        self.power == 0 && self.metal == 0 && self.food == 0
    }

    pub fn add(&mut self, other: Take) {
        self.power += other.power;
        self.metal += other.metal;
        self.food += other.food;
    }

    /// "2 power, 4 metal", or "" when there is nothing.
    pub fn words(self) -> String {
        let mut parts = Vec::new();
        if self.power > 0 {
            parts.push(format!("{} power", self.power));
        }
        if self.metal > 0 {
            parts.push(format!("{} metal", self.metal));
        }
        if self.food > 0 {
            parts.push(format!("{} food", self.food));
        }
        parts.join(", ")
    }
}

/// The kinds of raider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    Drifter,
    Skitter,
    Hulk,
    Sapper,
    Mule,
    Mothership,
    Carrier,
}

/// A raider's weapon, fired after a wind-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weapon {
    /// A shot at the gun's column; moving off the column dodges it.
    Bolt,
    /// Like a bolt, and the shield cannot be charged the turn after it hits.
    Siphon,
    /// Hits the colony wherever the gun is. Two turns to wind up.
    Siege,
}

impl Weapon {
    /// Damage to the shield; what the shield cannot take reaches the dome.
    pub fn damage(self) -> u32 {
        match self {
            Weapon::Bolt => 8,
            Weapon::Siphon => 6,
            Weapon::Siege => 15,
        }
    }

    /// Damage the dome takes however much shield is left.
    pub fn through(self) -> u32 {
        match self {
            Weapon::Siege => 5,
            _ => 0,
        }
    }

    /// Whether moving the gun off the named column dodges it.
    pub fn dodgeable(self) -> bool {
        !matches!(self, Weapon::Siege)
    }

    /// Raiders' phases between the wind-up and the shot.
    pub fn wind_up(self) -> u8 {
        match self {
            Weapon::Siege => 2,
            _ => 1,
        }
    }
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::Drifter,
        Kind::Skitter,
        Kind::Hulk,
        Kind::Sapper,
        Kind::Mule,
        Kind::Mothership,
        Kind::Carrier,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Drifter => "drifter",
            Kind::Skitter => "skitter",
            Kind::Hulk => "hulk",
            Kind::Sapper => "sapper",
            Kind::Mule => "mule",
            Kind::Mothership => "mothership",
            Kind::Carrier => "carrier",
        }
    }

    /// The name with its first letter up, for the start of a line.
    pub fn title(self) -> &'static str {
        match self {
            Kind::Drifter => "Drifter",
            Kind::Skitter => "Skitter",
            Kind::Hulk => "Hulk",
            Kind::Sapper => "Sapper",
            Kind::Mule => "Mule",
            Kind::Mothership => "Mothership",
            Kind::Carrier => "Carrier",
        }
    }

    /// Hits to bring one down.
    pub fn hits(self) -> u8 {
        match self {
            Kind::Drifter | Kind::Skitter => 1,
            Kind::Hulk => 3,
            Kind::Sapper | Kind::Mule => 2,
            Kind::Mothership => 8,
            Kind::Carrier => 12,
        }
    }

    /// Damage to the shield when it breaches; the rest reaches the dome.
    pub fn breach(self) -> u32 {
        match self {
            Kind::Drifter => 10,
            Kind::Skitter => 6,
            Kind::Hulk => 20,
            Kind::Sapper => 4,
            // These never reach row 1.
            Kind::Mule | Kind::Mothership | Kind::Carrier => 0,
        }
    }

    /// Steps along its route in one raiders' phase. A hulk's one step comes
    /// every other phase (see [`Raider::rest`]).
    pub fn steps(self) -> u8 {
        match self {
            Kind::Skitter => 2,
            _ => 1,
        }
    }

    pub fn weapon(self) -> Option<Weapon> {
        match self {
            Kind::Drifter => Some(Weapon::Bolt),
            Kind::Hulk => Some(Weapon::Siphon),
            Kind::Mothership | Kind::Carrier => Some(Weapon::Siege),
            _ => None,
        }
    }

    /// The nearest row a raider must reach before it winds up its weapon.
    pub fn range(self) -> u8 {
        match self {
            Kind::Drifter => 3,
            Kind::Hulk => 2,
            _ => u8::MAX,
        }
    }

    /// Whether it crosses the grid sideways along the crossing row instead
    /// of approaching the domes.
    pub fn crosses(self) -> bool {
        matches!(self, Kind::Mule | Kind::Carrier)
    }

    /// Whether it is hostile. A mule only carries cargo.
    pub fn hostile(self) -> bool {
        !matches!(self, Kind::Mule)
    }

    pub fn drop(self) -> Take {
        match self {
            Kind::Drifter => Take::new(2, 0, 0),
            Kind::Skitter => Take::new(1, 0, 0),
            Kind::Hulk => Take::new(0, 4, 0),
            Kind::Sapper => Take::new(0, 3, 0),
            Kind::Mule => Take::new(0, 3, 4),
            Kind::Mothership => Take::new(6, 6, 0),
            Kind::Carrier => Take::default(),
        }
    }

    /// Points for bringing one down.
    pub fn score(self) -> u64 {
        match self {
            Kind::Drifter | Kind::Skitter => 10,
            Kind::Hulk => 30,
            Kind::Sapper => 25,
            Kind::Mule => 20,
            Kind::Mothership => 100,
            Kind::Carrier => 300,
        }
    }

    /// Tech points for bringing one down.
    pub fn tech(self) -> u32 {
        match self {
            Kind::Mothership => 3,
            _ => 0,
        }
    }
}

/// Where a raider is in its life. Only the run moves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    /// Walking its route toward its goal.
    Approaching,
    /// Winding up a shot at `col`; it fires when `phases` reaches zero at
    /// the start of a raiders' phase. It stands still meanwhile.
    Charging { col: u8, phases: u8 },
    /// Breaking the wall at `wall`, which is the next step of its route.
    Bashing { wall: Sector },
    /// Standing still: no open route this phase, or a mothership at its
    /// post.
    Holding,
}

/// One raider on the grid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Raider {
    /// Stable through the night, so the screen and tests can follow one.
    pub id: u32,
    pub kind: Kind,
    /// Hits left.
    pub hits: u8,
    pub at: Sector,
    pub state: State,
    /// A hulk steps only on phases where this is false; it flips each phase.
    pub rest: bool,
    /// Which way a crossing raider is going: 1 right, -1 left.
    pub heading: i8,
    /// Which side a skitter of a pair leans to when routes tie: -1 left,
    /// 1 right, 0 no lean.
    pub lean: i8,
    /// Raiders' phases a mothership has spent at its post.
    pub posted: u32,
}

impl Raider {
    pub fn new(id: u32, kind: Kind, at: Sector) -> Raider {
        Raider {
            id,
            kind,
            hits: kind.hits(),
            at,
            state: State::Approaching,
            rest: false,
            heading: 1,
            lean: 0,
            posted: 0,
        }
    }

    pub fn is_charging(&self) -> bool {
        matches!(self.state, State::Charging { .. })
    }

    /// "Hulk, left, row 2".
    pub fn words(&self) -> String {
        format!("{}, {}", self.kind.title(), self.at.words())
    }
}

/// Raiders sorted nearest the domes first, then far left to far right,
/// then by arrival.
pub fn nearest_first(raiders: &mut [Raider]) {
    raiders.sort_by_key(|r| (r.at.row, r.at.col, r.id));
}
