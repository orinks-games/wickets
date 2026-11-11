//! The raiders: what comes down out of the dust, and the state each one is
//! in.
//!
//! A raider's life is one small state machine, moved only by
//! [`super::Run::tick`]:
//!
//! ```text
//! arrive -> Falling --(the dome)--> breach
//!    |         |
//!    |         +--(a mothership at mid range)--> Hovering
//!    |
//!    +--(a mule or the carrier)--> Crossing --(over the edge)--> leaves, or turns
//!
//! Any state leaves the sky as Down (shot, mined or flakked).
//! ```
//!
//! A raider with a weapon winds up a charge now and then, whatever state
//! it is in; the charge is a timer beside the state, not a state of its
//! own, so a drifter keeps falling while its bolt winds up.

use serde::{Deserialize, Serialize};

use super::lanes::{Range, HORIZON};

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
    /// A shot at the lane the gun was in as the wind-up began; the gun
    /// off that lane when it fires dodges it.
    Bolt,
    /// Like a bolt, and the shield does not recharge for a while after
    /// it hits.
    Siphon,
    /// Hits the colony wherever the gun is.
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

    /// Whether moving the gun off the named lane dodges it.
    pub fn dodgeable(self) -> bool {
        !matches!(self, Weapon::Siege)
    }

    /// Ticks between the wind-up and the shot.
    pub fn wind_up(self) -> u32 {
        match self {
            Weapon::Siege => 20,
            _ => 12,
        }
    }

    /// Ticks a raider waits after one shot before the next wind-up.
    pub fn rest(self) -> u32 {
        match self {
            Weapon::Bolt => 40,
            Weapon::Siphon => 50,
            Weapon::Siege => 60,
        }
    }
}

/// A charge winding up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Charge {
    pub weapon: Weapon,
    /// The lane aimed at.
    pub lane: u8,
    /// Ticks until it fires.
    pub ticks: u32,
}

/// A quirk a raider of night 7 or later may come down with. The forecast
/// names it with a working scanner, or with the quirks unlock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Quirk {
    /// One hit more than its kind.
    Armoured,
    /// A quarter faster than its kind.
    Hasty,
    /// Changes lane once, at mid range.
    Weaving,
}

impl Quirk {
    pub const ALL: [Quirk; 3] = [Quirk::Armoured, Quirk::Hasty, Quirk::Weaving];

    /// "armoured", as it goes before the kind's name.
    pub fn name(self) -> &'static str {
        match self {
            Quirk::Armoured => "armoured",
            Quirk::Hasty => "hasty",
            Quirk::Weaving => "weaving",
        }
    }

    /// What it does, as a sentence.
    pub fn about(self) -> &'static str {
        match self {
            Quirk::Armoured => "Takes 1 hit more than its kind.",
            Quirk::Hasty => "Falls a quarter faster than its kind.",
            Quirk::Weaving => "Changes lane once, at mid range.",
        }
    }

    /// Which kinds can come down with it.
    pub fn fits(self, kind: Kind) -> bool {
        match self {
            Quirk::Armoured => matches!(
                kind,
                Kind::Drifter | Kind::Skitter | Kind::Hulk | Kind::Sapper
            ),
            Quirk::Hasty => matches!(kind, Kind::Drifter | Kind::Skitter | Kind::Sapper),
            Quirk::Weaving => matches!(kind, Kind::Skitter | Kind::Sapper),
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

    /// Too well hulled for flak or a mine, and for the gun while it is
    /// still at far range: only missiles hurt it out there.
    pub fn armoured(self) -> bool {
        matches!(self, Kind::Mothership | Kind::Carrier)
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
            // These never reach the dome.
            Kind::Mule | Kind::Mothership | Kind::Carrier => 0,
        }
    }

    /// Range units it falls a tick: from the horizon to the dome in about
    /// 20 seconds for a drifter, 8 for a skitter, 33 for a hulk.
    pub fn speed(self) -> u32 {
        match self {
            Kind::Drifter => 5,
            Kind::Skitter => 12,
            Kind::Hulk => 3,
            Kind::Sapper => 7,
            Kind::Mothership => 6,
            // These cross; see `cross_every`.
            Kind::Mule | Kind::Carrier => 0,
        }
    }

    /// Ticks a crossing raider spends over each lane.
    pub fn cross_every(self) -> u32 {
        match self {
            Kind::Mule => 15,
            Kind::Carrier => 25,
            _ => 0,
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

    /// The range a raider must be within before it winds up its weapon.
    pub fn fires_from(self) -> Range {
        match self {
            Kind::Hulk => Range::Near,
            _ => Range::Mid,
        }
    }

    /// Whether it crosses the sky sideways instead of falling.
    pub fn crosses(self) -> bool {
        matches!(self, Kind::Mule | Kind::Carrier)
    }

    /// Whether it is hostile. A mule only carries cargo.
    pub fn hostile(self) -> bool {
        !matches!(self, Kind::Mule)
    }

    pub fn drop(self) -> Take {
        match self {
            Kind::Drifter => Take::new(1, 2, 0),
            Kind::Skitter => Take::new(0, 1, 0),
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
    /// Falling down its lane toward the dome.
    Falling,
    /// Crossing the lanes sideways: a mule or the carrier.
    Crossing,
    /// A mothership holding at mid range, shifting a lane now and then.
    Hovering,
}

/// One raider in the sky.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Raider {
    /// Stable through the night, so the screen and tests can follow one.
    pub id: u32,
    pub kind: Kind,
    /// Hits left.
    pub hits: u8,
    pub lane: u8,
    /// Range units from the dome: the horizon is [`HORIZON`], the dome 0.
    pub height: u32,
    pub state: State,
    /// Which way a crossing or hovering raider shifts: 1 right, -1 left.
    pub heading: i8,
    /// Ticks since it arrived.
    pub age: u32,
    /// Ticks since its last shot, or since it arrived.
    pub since_shot: u32,
    /// Shots it has fired this night.
    pub fired: u32,
    /// The charge winding up, if one is.
    pub charge: Option<Charge>,
    /// Its quirk, from night 7.
    pub quirk: Option<Quirk>,
    /// A weaving raider has made its weave.
    pub weaved: bool,
}

impl Raider {
    pub fn new(id: u32, kind: Kind, lane: u8, height: u32) -> Raider {
        let state = if kind.crosses() {
            State::Crossing
        } else {
            State::Falling
        };
        Raider {
            id,
            kind,
            hits: kind.hits(),
            lane,
            height,
            state,
            heading: if lane == 0 { 1 } else { -1 },
            age: 0,
            since_shot: 0,
            fired: 0,
            charge: None,
            quirk: None,
            weaved: false,
        }
    }

    /// At the horizon.
    pub fn arriving(id: u32, kind: Kind, lane: u8) -> Raider {
        Raider::new(id, kind, lane, HORIZON)
    }

    /// With a quirk; an armoured raider takes a hit more.
    pub fn with_quirk(mut self, quirk: Option<Quirk>) -> Raider {
        self.quirk = quirk;
        if quirk == Some(Quirk::Armoured) {
            self.hits += 1;
        }
        self
    }

    pub fn range(&self) -> Range {
        Range::of(self.height)
    }

    pub fn is_charging(&self) -> bool {
        self.charge.is_some()
    }

    pub fn has(&self, quirk: Quirk) -> bool {
        self.quirk == Some(quirk)
    }

    /// Whether it can still weave this night.
    pub fn can_weave(&self) -> bool {
        self.has(Quirk::Weaving) && !self.weaved
    }

    /// "Hulk" or "Armoured hulk".
    pub fn title(&self) -> String {
        match self.quirk {
            Some(q) => {
                let mut s = q.name().to_string();
                if let Some(first) = s.get_mut(0..1) {
                    first.make_ascii_uppercase();
                }
                format!("{s} {}", self.kind.name())
            }
            None => self.kind.title().to_string(),
        }
    }

    /// "a hulk" or "an armoured hulk".
    pub fn a(&self) -> String {
        match self.quirk {
            Some(Quirk::Armoured) => format!("an armoured {}", self.kind.name()),
            Some(q) => format!("a {} {}", q.name(), self.kind.name()),
            None if self.kind == Kind::Carrier => "the carrier".to_string(),
            None => format!("a {}", self.kind.name()),
        }
    }

    /// "Hulk, left, near" or "Armoured hulk, left, near".
    pub fn words(&self) -> String {
        format!(
            "{}, {}",
            self.title(),
            super::lanes::place_words(self.lane, self.height)
        )
    }
}

/// Raiders sorted nearest the dome first, then far left to far right,
/// then by arrival.
pub fn nearest_first(raiders: &mut [Raider]) {
    raiders.sort_by_key(|r| (r.height, r.lane, r.id));
}
