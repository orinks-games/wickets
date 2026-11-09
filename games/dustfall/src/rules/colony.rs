//! The colony: its three resources, the dome and shield, the missiles, and
//! the buildings in the slots under the grid's columns.

use serde::{Deserialize, Serialize};

use super::grid::COLS;
use super::perks::Perks;
use super::raiders::Take;

/// Integrity the dome starts with and holds at most.
pub const DOME_MAX: u32 = 100;
/// Shield at the start of a run.
pub const SHIELD_START: u32 = 20;
/// The most shield the colony holds.
pub const SHIELD_MAX: u32 = 50;
/// Shield each unspent action adds when a turn ends early.
pub const SHIELD_PER_ACTION: u32 = 5;
/// Missiles carried at most.
pub const MISSILES_MAX: u32 = 3;
/// A missile's damage: enough to destroy any raider but a mothership or
/// the carrier outright.
pub const MISSILE_DAMAGE: u8 = 4;
/// Power and metal for a missile without a workshop, and with one.
pub const MISSILE_COST: (u32, u32) = (6, 4);
pub const MISSILE_COST_WORKSHOP: (u32, u32) = (3, 2);
/// Metal to put [`DOME_REPAIR`] back on the dome.
pub const DOME_REPAIR_COST: u32 = 5;
pub const DOME_REPAIR: u32 = 20;
/// Metal to bring a dark building back.
pub const BUILDING_REPAIR_COST: u32 = 4;
/// Food the colony eats a night.
pub const FOOD_A_NIGHT: u32 = 2;
/// Actions a turn while fed, and while hungry.
pub const ACTIONS_FED: u8 = 2;
pub const ACTIONS_HUNGRY: u8 = 1;
/// Slots open at the start: the columns under the middle of the grid.
pub const OPEN_SLOTS: [u8; 3] = [1, 2, 3];

/// What the colony has in store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resources {
    pub power: u32,
    pub metal: u32,
    pub food: u32,
}

impl Resources {
    pub const START: Resources = Resources {
        power: 12,
        metal: 24,
        food: 6,
    };

    pub fn add(&mut self, take: Take) {
        self.power += take.power;
        self.metal += take.metal;
        self.food += take.food;
    }

    /// Whether `power` and `metal` are in store.
    pub fn covers(&self, power: u32, metal: u32) -> bool {
        self.power >= power && self.metal >= metal
    }

    /// Spends `power` and `metal`; false, and nothing spent, when short.
    pub fn spend(&mut self, power: u32, metal: u32) -> bool {
        if !self.covers(power, metal) {
            return false;
        }
        self.power -= power;
        self.metal -= metal;
        true
    }

    /// "Power 9, metal 17, food 4".
    pub fn words(&self) -> String {
        format!(
            "Power {}, metal {}, food {}",
            self.power, self.metal, self.food
        )
    }
}

/// The buildings, one to a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Building {
    Solar,
    Hydroponics,
    Scanner,
    Workshop,
    ShieldCoil,
}

impl Building {
    pub const ALL: [Building; 5] = [
        Building::Solar,
        Building::Hydroponics,
        Building::Scanner,
        Building::Workshop,
        Building::ShieldCoil,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Building::Solar => "solar array",
            Building::Hydroponics => "hydroponics",
            Building::Scanner => "scanner",
            Building::Workshop => "workshop",
            Building::ShieldCoil => "shield coil",
        }
    }

    /// The name with its article: "a solar array", "hydroponics".
    pub fn a(self) -> String {
        match self {
            Building::Hydroponics => self.name().to_string(),
            _ => format!("a {}", self.name()),
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Building::Solar => "Solar array",
            Building::Hydroponics => "Hydroponics",
            Building::Scanner => "Scanner",
            Building::Workshop => "Workshop",
            Building::ShieldCoil => "Shield coil",
        }
    }

    /// Metal and power to build one.
    pub fn cost(self) -> (u32, u32) {
        match self {
            Building::Solar => (8, 0),
            Building::Hydroponics => (6, 2),
            Building::Scanner => (8, 2),
            Building::Workshop => (10, 0),
            Building::ShieldCoil => (8, 0),
        }
    }

    /// Power drawn each night to run.
    pub fn upkeep(self) -> u32 {
        match self {
            Building::Scanner => 1,
            Building::ShieldCoil => 2,
            _ => 0,
        }
    }
}

/// A slot under a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub building: Building,
    /// Hit by a breach: it does nothing until repaired.
    pub dark: bool,
    /// Whether its power was paid this night.
    pub powered: bool,
}

impl Slot {
    /// Whether it does its job this night.
    pub fn works(&self) -> bool {
        !self.dark && self.powered
    }
}

/// The colony as a whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Colony {
    pub resources: Resources,
    pub dome: u32,
    pub shield: u32,
    pub missiles: u32,
    /// Which columns have an open slot.
    pub open: Vec<u8>,
    /// The building under each column, if any.
    pub slots: Vec<Option<Slot>>,
    /// The caps this colony plays to: the defaults, or what its perks set.
    #[serde(default = "dome_max")]
    pub dome_max: u32,
    #[serde(default = "shield_max")]
    pub shield_max: u32,
    #[serde(default = "missiles_max")]
    pub missiles_max: u32,
}

fn dome_max() -> u32 {
    DOME_MAX
}

fn shield_max() -> u32 {
    SHIELD_MAX
}

fn missiles_max() -> u32 {
    MISSILES_MAX
}

impl Default for Colony {
    fn default() -> Self {
        Colony::new()
    }
}

impl Colony {
    pub fn new() -> Colony {
        Colony::with(Perks::default())
    }

    /// A colony as `perks` set it up: more slots open, a thicker dome, a
    /// bigger shield, a deeper rack, missiles in it from the start.
    pub fn with(perks: Perks) -> Colony {
        let mut open = OPEN_SLOTS.to_vec();
        if perks.extra_slots >= 1 {
            open.insert(0, 0);
        }
        if perks.extra_slots >= 2 {
            open.push(COLS - 1);
        }
        Colony {
            resources: Resources::START,
            dome: perks.dome_max(),
            shield: SHIELD_START,
            missiles: perks.start_missiles.min(perks.missiles_max()),
            open,
            slots: vec![None; COLS as usize],
            dome_max: perks.dome_max(),
            shield_max: perks.shield_max(),
            missiles_max: perks.missiles_max(),
        }
    }

    pub fn slot(&self, col: u8) -> Option<&Slot> {
        self.slots.get(col as usize).and_then(|s| s.as_ref())
    }

    pub fn slot_mut(&mut self, col: u8) -> Option<&mut Slot> {
        self.slots.get_mut(col as usize).and_then(|s| s.as_mut())
    }

    /// Whether a working building of `kind` stands anywhere.
    pub fn has(&self, kind: Building) -> bool {
        self.slots
            .iter()
            .flatten()
            .any(|s| s.building == kind && s.works())
    }

    /// Columns with any building, and with hydroponics, for the sappers.
    pub fn building_columns(&self) -> (Vec<u8>, Vec<u8>) {
        let mut all = Vec::new();
        let mut hydro = Vec::new();
        for (col, slot) in self.slots.iter().enumerate() {
            if let Some(s) = slot {
                all.push(col as u8);
                if s.building == Building::Hydroponics {
                    hydro.push(col as u8);
                }
            }
        }
        (all, hydro)
    }

    /// What a missile costs now.
    pub fn missile_cost(&self) -> (u32, u32) {
        if self.has(Building::Workshop) {
            MISSILE_COST_WORKSHOP
        } else {
            MISSILE_COST
        }
    }

    /// Actions a turn: two while there is food, one while hungry.
    pub fn actions(&self) -> u8 {
        if self.resources.food > 0 {
            ACTIONS_FED
        } else {
            ACTIONS_HUNGRY
        }
    }

    pub fn hungry(&self) -> bool {
        self.resources.food == 0
    }

    /// Adds shield up to the cap. Returns what was added.
    pub fn charge(&mut self, amount: u32) -> u32 {
        let added = amount.min(self.shield_max.saturating_sub(self.shield));
        self.shield += added;
        added
    }

    /// Takes `damage` on the shield first, then the dome, with at least
    /// `through` reaching the dome. Returns (shield damage, dome damage).
    pub fn hit(&mut self, damage: u32, through: u32) -> (u32, u32) {
        let on_shield = damage.min(self.shield);
        let mut to_dome = damage - on_shield;
        if to_dome < through {
            to_dome = through.min(damage);
        }
        self.shield -= on_shield;
        self.dome = self.dome.saturating_sub(to_dome);
        (on_shield, to_dome)
    }

    pub fn fallen(&self) -> bool {
        self.dome == 0
    }
}
