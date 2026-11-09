//! The rules of Dustfall, with no sound, speech or clock, so every rule can
//! be tested alone and the same code can referee an online room later.
//! Everything here is whole numbers and a seeded [`Rng`], so a replay of the
//! same seed and inputs lands on the same run anywhere.
//!
//! Two state machines, both moved only by [`Run::apply`]:
//!
//! ```text
//! The run:    Colony --Ready--> Turn --(night held, night 12)--> Relief
//!               ^                 |
//!               +--(night held)---+--(dome at 0)--> Fallen
//!
//! The turn:   Turn { actions } --Left, Right, Fire, Missile--> Turn { actions - 1 }
//!                  |                                                |
//!                  +--EndTurn, or the last action--> the raiders' phase, whole:
//!                       1 charges fire, 2 breaches, 3 steps, 4 wind-ups, 5 flak,
//!                     then Turn { actions } again, or the night is held or lost.
//! ```
//!
//! Each raider's own machine is in [`raiders`]. The screen turns the
//! [`Event`]s a step raises into sound and speech; nothing here is spoken.
//!
//! A run with `relief` 0 is the long watch: Relief lands on night 12 as a
//! milestone and the nights go on until the dome falls. A run with a
//! [`Storm`] is the dust storm event. [`Perks`] carry what the player's
//! unlocks change, so a replay needs nothing but the run.

pub mod colony;
pub mod grid;
pub mod nights;
pub mod orders;
pub mod perks;
pub mod raiders;
pub mod route;
pub mod storm;

use parlor_core::Rng;
use serde::{Deserialize, Serialize};

pub use colony::{Building, Colony, Resources, Slot};
pub use grid::{column_name, Grid, Sector, Work, WorkKind, CENTRE, COLS, FAR_ROW, NEAR_ROW, ROWS};
pub use nights::RELIEF_NIGHT;
pub use orders::Order;
pub use perks::Perks;
pub use raiders::{Kind, Quirk, Raider, State, Take, Weapon};
pub use storm::Storm;

/// Points for holding a night, and as much again for one with no breach.
pub const NIGHT_HELD_POINTS: u64 = 50;
pub const CLEAN_NIGHT_POINTS: u64 = 50;
/// Tech points for a night held, and for Relief.
pub const NIGHT_HELD_TECH: u32 = 1;
pub const RELIEF_TECH: u32 = 5;
/// Charges that can be winding up at once, so the warnings stay readable.
/// A mothership's siege is not counted.
pub const MAX_CHARGES: usize = 2;
/// At its post a mothership drops a skitter every this many phases, and
/// winds up a siege shot every this many.
pub const MOTHERSHIP_DROP_EVERY: u32 = 2;
pub const MOTHERSHIP_SIEGE_EVERY: u32 = 3;
/// The carrier winds up a siege shot every this many phases.
pub const CARRIER_SIEGE_EVERY: u32 = 3;

/// Where the run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Between nights: spending, then Ready.
    Colony,
    /// The player's turn, with `actions` left to spend. The raiders' phase
    /// runs whole when the turn ends, so it is never a state the player
    /// waits in.
    Turn { actions: u8 },
    /// Relief landed: the run is won.
    Relief,
    /// The dome fell: the run is lost.
    Fallen,
}

/// A raider the forecast promises, with the column it enters at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrival {
    pub kind: Kind,
    pub col: u8,
    #[serde(default)]
    pub quirk: Option<Quirk>,
}

/// Everything a player can do. Each is one input of a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    /// Slide the gun one column.
    Left,
    Right,
    /// Shoot up the gun's column.
    Fire,
    /// Launch a missile at the raider nearest the domes.
    Missile,
    /// Stop early: each unspent action charges the shield.
    EndTurn,
    /// Colony phase: a building in the slot under `col`.
    Buy {
        building: Building,
        col: u8,
    },
    /// Colony phase: a work on the grid.
    Place {
        work: WorkKind,
        at: Sector,
    },
    RepairDome,
    RepairBuilding {
        col: u8,
    },
    MakeMissile,
    /// Colony phase: one of the night's two orders.
    TakeOrder(Order),
    /// Colony phase: start the night.
    Ready,
}

/// Why an input did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Not in the phase the input belongs to.
    WrongPhase,
    /// The gun is at the edge already.
    Edge,
    NoMissiles,
    /// Nothing on the grid to fire a missile at.
    NoTarget,
    /// Short of power or metal; what it would take.
    CantAfford {
        power: u32,
        metal: u32,
    },
    SlotClosed,
    SlotTaken,
    /// Works go in rows 2 to 5.
    RowOpen,
    SectorTaken,
    /// No building under that column.
    NothingThere,
    /// The building is not dark.
    NotDark,
    DomeWhole,
    MissilesFull,
    /// Not one of tonight's two orders.
    NoSuchOrder,
    /// An order is taken already this night.
    OrderTaken,
}

/// What brought a raider down or hurt it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hitter {
    Gun,
    Missile,
    Mine,
    Flak(Sector),
}

/// What happened, in order, for the screen to tell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// The night's raiders are on the grid.
    NightOpens {
        night: u8,
        arrivals: Vec<(u32, Kind, Sector)>,
    },
    YourTurn {
        turn: u32,
        actions: u8,
    },
    /// The gun slid to `col`; `under` is the nearest raider in that
    /// column as it stood then, before the raiders moved.
    Moved {
        col: u8,
        under: Option<(Kind, u8)>,
    },
    /// The gun fired up an empty column.
    DryShot {
        col: u8,
    },
    Hit {
        id: u32,
        kind: Kind,
        at: Sector,
        left: u8,
        by: Hitter,
    },
    Down {
        id: u32,
        kind: Kind,
        at: Sector,
        take: Take,
        by: Hitter,
    },
    /// The turn ended early; `charged` went on the shield.
    TurnEnded {
        charged: u32,
    },
    /// A wound-up shot went off.
    Shot {
        id: u32,
        kind: Kind,
        weapon: Weapon,
        col: u8,
        dodged: bool,
        shield: u32,
        dome: u32,
    },
    Breach {
        id: u32,
        kind: Kind,
        col: u8,
        shield: u32,
        dome: u32,
        darkened: Option<Building>,
    },
    Stepped {
        id: u32,
        kind: Kind,
        from: Sector,
        to: Sector,
    },
    Bashed {
        id: u32,
        kind: Kind,
        wall: Sector,
        down: bool,
    },
    Charging {
        id: u32,
        kind: Kind,
        weapon: Weapon,
        col: u8,
    },
    /// A burrowing raider passed under a wall, which still stands.
    Burrowed {
        id: u32,
        kind: Kind,
        wall: Sector,
    },
    /// The dust storm covers `col` for this raiders' phase.
    StormMoved {
        col: u8,
    },
    /// A mothership let a skitter out beside it.
    Dropped {
        from: u32,
        id: u32,
        at: Sector,
    },
    /// A mule crossed the grid and left.
    Left {
        id: u32,
        kind: Kind,
    },
    Held {
        night: u8,
        take: Take,
        clean: bool,
    },
    Relief {
        score: u64,
        tech: u32,
    },
    Fallen {
        night: u8,
        score: u64,
        tech: u32,
    },
    ColonyOpens {
        night: u8,
        hungry: bool,
        unpowered: Vec<Sector>,
        dark: Vec<u8>,
    },
    Bought {
        building: Building,
        col: u8,
    },
    Placed {
        work: WorkKind,
        at: Sector,
    },
    DomeRepaired {
        dome: u32,
    },
    BuildingRepaired {
        building: Building,
        col: u8,
    },
    MissileMade {
        missiles: u32,
    },
    OrderTaken {
        order: Order,
    },
    /// The night is over and an order's cost came due.
    OrderPaid {
        order: Order,
    },
}

fn idle_rng() -> Rng {
    Rng::seeded(0)
}

/// Stores a skipped night would have brought, for a run that starts late.
pub const SKIPPED_NIGHT_METAL: u32 = 5;
pub const SKIPPED_NIGHT_POWER: u32 = 2;

fn first_night() -> u8 {
    1
}

fn relief_night() -> u8 {
    RELIEF_NIGHT
}

/// One run, from the first colony phase to Relief or the fallen dome.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    seed: u64,
    /// The night the run started on: 1, or later for a challenge.
    #[serde(default = "first_night")]
    pub first: u8,
    /// The night that brings Relief once held; 0 for the long watch, where
    /// Relief lands on night 12 and the nights go on.
    #[serde(default = "relief_night")]
    pub relief: u8,
    /// What the player's unlocks and chosen modifiers changed.
    #[serde(default)]
    pub perks: Perks,
    /// The orders offered before this night, and the one taken.
    #[serde(default)]
    pub offered: Vec<Order>,
    #[serde(default)]
    pub order: Option<Order>,
    /// The dust storm, in a storm run.
    #[serde(default)]
    pub storm: Option<Storm>,
    /// The night in progress, or the one coming next in the colony phase.
    pub night: u8,
    pub phase: Phase,
    /// The turn of the night, from 1.
    pub turn: u32,
    pub grid: Grid,
    pub colony: Colony,
    pub raiders: Vec<Raider>,
    /// The gun's column.
    pub gun: u8,
    /// A siphon hit: the shield takes no charge this turn.
    pub siphoned: bool,
    pub forecast: Vec<Arrival>,
    /// What the night's kills have brought in.
    pub take: Take,
    /// Whether anything breached this night.
    pub breached: bool,
    pub score: u64,
    pub tech: u32,
    pub nights_held: u8,
    pub downed: u32,
    pub motherships_downed: u32,
    next_id: u32,
    /// The night's random source. A run saves only between nights, and
    /// Ready reseeds this from the seed and the night, so it is not saved.
    #[serde(skip, default = "idle_rng")]
    rng: Rng,
    #[serde(skip)]
    events: Vec<Event>,
}

impl Run {
    /// A run in its first colony phase, night 1 forecast.
    pub fn new(seed: u64) -> Run {
        Run::with_perks(seed, Perks::default())
    }

    /// A run set up as `perks` say.
    pub fn with_perks(seed: u64, perks: Perks) -> Run {
        let mut grid = Grid::new();
        if perks.start_walls {
            for col in [CENTRE - 1, CENTRE + 1] {
                grid.set(Sector::new(col, 3), Some(WorkKind::Wall.build()));
            }
        }
        let mut run = Run {
            seed,
            first: 1,
            relief: RELIEF_NIGHT,
            perks,
            offered: Vec::new(),
            order: None,
            storm: None,
            night: 1,
            phase: Phase::Colony,
            turn: 0,
            grid,
            colony: Colony::with(perks),
            raiders: Vec::new(),
            gun: CENTRE,
            siphoned: false,
            forecast: Vec::new(),
            take: Take::default(),
            breached: false,
            score: 0,
            tech: 0,
            nights_held: 0,
            downed: 0,
            motherships_downed: 0,
            next_id: 1,
            rng: idle_rng(),
            events: Vec::new(),
        };
        run.read_the_sky();
        run.events.push(Event::ColonyOpens {
            night: 1,
            hungry: false,
            unpowered: Vec::new(),
            dark: Vec::new(),
        });
        run
    }

    /// The long watch: Relief lands on night 12 and the nights go on until
    /// the dome falls.
    pub fn long_watch(seed: u64, perks: Perks) -> Run {
        let mut run = Run::with_perks(seed, perks);
        run.relief = 0;
        run
    }

    /// A run through the dust storm.
    pub fn in_storm(seed: u64, perks: Perks) -> Run {
        let mut run = Run::with_perks(seed, perks);
        run.storm = Some(Storm::new());
        run
    }

    /// A run in the colony phase before `night`, as if the nights before
    /// were held with nothing bought: for tests, the tutorial and a
    /// challenge that starts late.
    pub fn at_night(seed: u64, night: u8) -> Run {
        let mut run = Run::new(seed);
        run.night = night.clamp(1, RELIEF_NIGHT);
        run.first = run.night;
        run.read_the_sky();
        run
    }

    /// The forecast and the orders for the night coming.
    fn read_the_sky(&mut self) {
        self.forecast = self.draw_forecast();
        self.offered = Order::offered(self.seed, self.night);
        self.order = None;
    }

    /// Whether the run goes on past Relief.
    pub fn endless(&self) -> bool {
        self.relief == 0
    }

    /// A run of the nights `first` to `last`, in the colony phase before
    /// `first`, with the stores the skipped nights would have brought.
    /// Relief lands once `last` is held.
    pub fn prepared(seed: u64, first: u8, last: u8) -> Run {
        let mut run = Run::at_night(seed, first);
        run.relief = last.clamp(run.night, RELIEF_NIGHT);
        let skipped = u32::from(run.night - 1);
        run.colony.resources.metal += SKIPPED_NIGHT_METAL * skipped;
        run.colony.resources.power += SKIPPED_NIGHT_POWER * skipped;
        run
    }

    /// How many nights the run holds in all; 0 for the long watch.
    pub fn nights(&self) -> u8 {
        if self.endless() {
            0
        } else {
            self.relief + 1 - self.first
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The events since the last take, oldest first.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn is_over(&self) -> bool {
        matches!(self.phase, Phase::Relief | Phase::Fallen)
    }

    /// Does one thing. On a refusal nothing changed.
    pub fn apply(&mut self, input: Input) -> Result<(), Refusal> {
        match (self.phase, input) {
            (Phase::Turn { .. }, Input::Left) => self.slide(-1),
            (Phase::Turn { .. }, Input::Right) => self.slide(1),
            (Phase::Turn { .. }, Input::Fire) => {
                self.fire();
                Ok(())
            }
            (Phase::Turn { .. }, Input::Missile) => self.missile(),
            (Phase::Turn { actions }, Input::EndTurn) => {
                self.end_turn(actions);
                Ok(())
            }
            (Phase::Colony, Input::Buy { building, col }) => self.buy(building, col),
            (Phase::Colony, Input::Place { work, at }) => self.place(work, at),
            (Phase::Colony, Input::RepairDome) => self.repair_dome(),
            (Phase::Colony, Input::RepairBuilding { col }) => self.repair_building(col),
            (Phase::Colony, Input::MakeMissile) => self.make_missile(),
            (Phase::Colony, Input::TakeOrder(order)) => self.take_order(order),
            (Phase::Colony, Input::Ready) => {
                self.ready();
                Ok(())
            }
            _ => Err(Refusal::WrongPhase),
        }
    }

    // ----- Queries the screen reads without changing anything -----

    /// The raiders nearest the domes first.
    pub fn nearest_first(&self) -> Vec<&Raider> {
        let mut all: Vec<&Raider> = self.raiders.iter().collect();
        all.sort_by_key(|r| (r.at.row, r.at.col, r.id));
        all
    }

    pub fn raider(&self, id: u32) -> Option<&Raider> {
        self.raiders.iter().find(|r| r.id == id)
    }

    /// The route a raider will walk from where it stands, nearest step
    /// first. A crossing raider's route is the rest of its crossing.
    pub fn route_of(&self, id: u32) -> Option<Vec<Sector>> {
        let raider = self.raider(id)?;
        if raider.kind.crosses() {
            let mut out = Vec::new();
            let mut col = raider.at.col as i8 + raider.heading;
            while (0..COLS as i8).contains(&col) {
                out.push(Sector::new(col as u8, raider.at.row));
                col += raider.heading;
            }
            return Some(out);
        }
        let (buildings, hydro) = self.colony.building_columns();
        let goal = route::goal(raider.kind, &buildings, &hydro);
        route::plan(&self.grid, raider, &goal, self.gun)
    }

    /// Raiders' phases until the raider reaches the end of its route.
    pub fn phases_to_land(&self, id: u32) -> Option<u32> {
        let raider = self.raider(id)?;
        let route = self.route_of(id)?;
        let walls = route.iter().filter(|s| self.grid.is_wall(**s)).count() as u32;
        let mut bashes = walls * u32::from(grid::WALL_HITS);
        if raider.can_burrow() && walls > 0 {
            // The first wall is passed under in one phase.
            bashes -= u32::from(grid::WALL_HITS) - 1;
        }
        Some(
            route::phases_with_steps(
                raider.kind,
                raider.rest,
                route.len(),
                self.steps_of(raider.kind),
            ) + bashes,
        )
    }

    /// Steps a raider of `kind` takes a phase: skitters take three under
    /// Salvage.
    pub fn steps_of(&self, kind: Kind) -> u8 {
        if kind == Kind::Skitter && self.order == Some(Order::Salvage) {
            orders::SALVAGE_SKITTER_STEPS
        } else {
            kind.steps()
        }
    }

    /// Whether the dust storm covers `at` this phase.
    pub fn in_dust(&self, at: Sector) -> bool {
        self.storm.is_some_and(|s| s.covers(at))
    }

    /// The raiders the scan can hear: not the ones in the dust.
    pub fn heard(&self) -> Vec<&Raider> {
        self.nearest_first()
            .into_iter()
            .filter(|r| !self.in_dust(r.at))
            .collect()
    }

    /// The score with the run's bonus on it, for Hard vacuum and the
    /// call-outs off.
    pub fn with_bonus(&self, score: u64) -> u64 {
        score + score * u64::from(self.perks.bonus_percent()) / 100
    }

    /// The charges winding up: raider id, its kind and the column aimed at.
    pub fn charges(&self) -> Vec<(u32, Kind, u8)> {
        self.nearest_first()
            .into_iter()
            .filter_map(|r| match r.state {
                State::Charging { col, .. } => Some((r.id, r.kind, col)),
                _ => None,
            })
            .collect()
    }

    /// Actions a turn as the colony stands.
    pub fn actions_a_turn(&self) -> u8 {
        self.colony.actions()
    }

    /// "2 drifters, a skitter pair, a mule".
    pub fn forecast_words(&self) -> String {
        let kinds: Vec<Kind> = self.forecast.iter().map(|a| a.kind).collect();
        nights::describe(&kinds)
    }

    /// "An armoured hulk and a hasty drifter among them." when the forecast
    /// carries quirks, for a scanner or the quirks unlock to read.
    pub fn quirk_words(&self) -> Option<String> {
        let named: Vec<String> = self
            .forecast
            .iter()
            .filter_map(|a| {
                let q = a.quirk?;
                Some(match q {
                    Quirk::Armoured => format!("an armoured {}", a.kind.name()),
                    q => format!("a {} {}", q.name(), a.kind.name()),
                })
            })
            .collect();
        let list = match named.as_slice() {
            [] => return None,
            [one] => one.clone(),
            [a, b] => format!("{a} and {b}"),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        };
        let mut line = format!("{list} among them.");
        if let Some(first) = line.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        Some(line)
    }

    // ----- The player's turn -----

    fn slide(&mut self, dir: i8) -> Result<(), Refusal> {
        let col = self.gun as i8 + dir;
        if !(0..COLS as i8).contains(&col) {
            return Err(Refusal::Edge);
        }
        self.gun = col as u8;
        let under = self
            .raiders
            .iter()
            .filter(|r| r.at.col == self.gun)
            .min_by_key(|r| (r.at.row, r.id))
            .map(|r| (r.kind, r.at.row));
        self.events.push(Event::Moved {
            col: self.gun,
            under,
        });
        self.spend_action();
        Ok(())
    }

    fn fire(&mut self) {
        let target = self
            .raiders
            .iter()
            .filter(|r| r.at.col == self.gun)
            .min_by_key(|r| (r.at.row, r.id))
            .map(|r| r.id);
        match target {
            Some(id) => {
                self.damage(id, 1, Hitter::Gun);
                if self.hold_if_clear() {
                    return;
                }
            }
            None => self.events.push(Event::DryShot { col: self.gun }),
        }
        self.spend_action();
    }

    /// The last raider fell to the player's own shot: the night is held at
    /// once, and the actions left after that shot charge the shield.
    /// Returns whether it was.
    fn hold_if_clear(&mut self) -> bool {
        if !self.raiders.is_empty() {
            return false;
        }
        if let Phase::Turn { actions } = self.phase {
            self.end_turn(actions.saturating_sub(1));
        }
        true
    }

    fn missile(&mut self) -> Result<(), Refusal> {
        if self.colony.missiles == 0 {
            return Err(Refusal::NoMissiles);
        }
        let gun = self.gun;
        let target = self
            .raiders
            .iter()
            .min_by_key(|r| (r.at.row, r.at.col.abs_diff(gun), r.at.col, r.id))
            .map(|r| r.id)
            .ok_or(Refusal::NoTarget)?;
        self.colony.missiles -= 1;
        self.damage(target, colony::MISSILE_DAMAGE, Hitter::Missile);
        if !self.hold_if_clear() {
            self.spend_action();
        }
        Ok(())
    }

    fn spend_action(&mut self) {
        if let Phase::Turn { actions } = self.phase {
            let left = actions.saturating_sub(1);
            if left == 0 {
                // The last action ends the turn with nothing left to charge.
                self.events.push(Event::TurnEnded { charged: 0 });
                self.raiders_phase();
            } else {
                self.phase = Phase::Turn { actions: left };
            }
        }
    }

    fn end_turn(&mut self, actions: u8) {
        let charged = if self.siphoned {
            0
        } else {
            self.colony
                .charge(actions as u32 * colony::SHIELD_PER_ACTION)
        };
        self.events.push(Event::TurnEnded { charged });
        self.raiders_phase();
    }

    /// Hurts a raider. Returns whether it went down.
    fn damage(&mut self, id: u32, amount: u8, by: Hitter) -> bool {
        let Some(index) = self.raiders.iter().position(|r| r.id == id) else {
            return false;
        };
        let raider = &mut self.raiders[index];
        raider.hits = raider.hits.saturating_sub(amount);
        if raider.hits > 0 {
            self.events.push(Event::Hit {
                id,
                kind: raider.kind,
                at: raider.at,
                left: raider.hits,
                by,
            });
            return false;
        }
        let raider = self.raiders.remove(index);
        let mut take = raider.kind.drop();
        if self.order == Some(Order::Salvage) {
            take.metal *= 2;
        }
        self.colony.resources.add(take);
        self.take.add(take);
        self.score += raider.kind.score();
        self.tech += raider.kind.tech();
        self.downed += 1;
        if raider.kind == Kind::Mothership {
            self.motherships_downed += 1;
        }
        self.events.push(Event::Down {
            id,
            kind: raider.kind,
            at: raider.at,
            take,
            by,
        });
        true
    }

    // ----- The raiders' phase -----

    /// The raiders in the order they act: the hasty first, then nearest
    /// the domes first.
    fn phase_order(&self) -> Vec<u32> {
        let mut all: Vec<&Raider> = self.raiders.iter().collect();
        all.sort_by_key(|r| (!r.has(Quirk::Hasty), r.at.row, r.at.col, r.id));
        all.iter().map(|r| r.id).collect()
    }

    fn raiders_phase(&mut self) {
        self.siphoned = false;
        if let Some(storm) = &mut self.storm {
            storm.advance();
            let col = storm.col;
            self.events.push(Event::StormMoved { col });
        }
        let order = self.phase_order();

        // 1. Charges fire.
        for &id in &order {
            let Some(raider) = self.raiders.iter_mut().find(|r| r.id == id) else {
                continue;
            };
            let State::Charging { col, phases } = raider.state else {
                continue;
            };
            if phases > 1 {
                raider.state = State::Charging {
                    col,
                    phases: phases - 1,
                };
                continue;
            }
            raider.state = State::Approaching;
            let (kind, id) = (raider.kind, raider.id);
            let Some(weapon) = kind.weapon() else {
                continue;
            };
            let dodged = weapon.dodgeable() && self.gun != col;
            if !dodged {
                self.colony.hit(weapon.damage(), weapon.through());
                if weapon == Weapon::Siphon {
                    self.siphoned = true;
                }
            }
            self.events.push(Event::Shot {
                id,
                kind,
                weapon,
                col,
                dodged,
                shield: self.colony.shield,
                dome: self.colony.dome,
            });
            if self.colony.fallen() {
                return self.fall();
            }
        }

        // 2. Breaches. A raider breaches where its route ends: a sapper
        // walking along the near row to its building has not arrived yet.
        let (buildings, hydro) = self.colony.building_columns();
        for &id in &order {
            let Some(index) = self.raiders.iter().position(|r| r.id == id) else {
                continue;
            };
            let raider = &self.raiders[index];
            if raider.at.row != NEAR_ROW || !raider.kind.hostile() || raider.kind.crosses() {
                continue;
            }
            if !route::goal(raider.kind, &buildings, &hydro).reached(raider.at) {
                continue;
            }
            let raider = self.raiders.remove(index);
            let col = raider.at.col;
            let (_, to_dome) = self.colony.hit(raider.kind.breach(), 0);
            self.breached = true;
            let mut darkened = None;
            if to_dome > 0 || raider.kind == Kind::Sapper {
                if let Some(slot) = self.colony.slot_mut(col) {
                    if !slot.dark {
                        slot.dark = true;
                        darkened = Some(slot.building);
                    }
                }
            }
            self.events.push(Event::Breach {
                id,
                kind: raider.kind,
                col,
                shield: self.colony.shield,
                dome: self.colony.dome,
                darkened,
            });
            if self.colony.fallen() {
                return self.fall();
            }
        }

        // 3. Steps.
        for &id in &order {
            if self.raider(id).is_none() {
                continue;
            }
            self.step(id);
        }

        // 4. Wind-ups.
        let mut pending = self
            .raiders
            .iter()
            .filter(|r| r.is_charging() && r.kind.weapon() != Some(Weapon::Siege))
            .count();
        let docile = nights::docile(self.night);
        let order = self.phase_order();
        for &id in &order {
            if pending >= MAX_CHARGES {
                break;
            }
            let gun = self.gun;
            let Some(raider) = self.raiders.iter_mut().find(|r| r.id == id) else {
                continue;
            };
            let Some(weapon) = raider.kind.weapon() else {
                continue;
            };
            if weapon == Weapon::Siege
                || raider.is_charging()
                || raider.at.row < 2
                || raider.at.row > raider.kind.range()
                || (docile && raider.kind == Kind::Drifter)
            {
                continue;
            }
            raider.state = State::Charging {
                col: gun,
                phases: weapon.wind_up(),
            };
            pending += 1;
            self.events.push(Event::Charging {
                id,
                kind: raider.kind,
                weapon,
                col: gun,
            });
        }

        // 5. Flak. A post in the dust cannot see to fire.
        for post in self.grid.powered_flak() {
            if self.in_dust(post) {
                continue;
            }
            let covered = post.around();
            // Flak scatters off a mothership's or the carrier's hull.
            let target = self
                .raiders
                .iter()
                .filter(|r| covered.contains(&r.at) && !r.kind.armoured())
                .min_by_key(|r| (r.at.row, r.at.col, r.id))
                .map(|r| r.id);
            if let Some(id) = target {
                self.damage(id, 1, Hitter::Flak(post));
            }
        }

        if self.raiders.is_empty() {
            return self.hold_night();
        }
        self.turn += 1;
        let actions = self.colony.actions();
        self.phase = Phase::Turn { actions };
        self.events.push(Event::YourTurn {
            turn: self.turn,
            actions,
        });
    }

    /// One raider's move for the phase.
    fn step(&mut self, id: u32) {
        let Some(raider) = self.raider(id).cloned() else {
            return;
        };
        if raider.is_charging() || self.in_dust(raider.at) {
            // In the dust a raider loses its step.
            return;
        }
        if raider.kind.crosses() {
            return self.cross(id);
        }
        let (buildings, hydro) = self.colony.building_columns();
        let goal = route::goal(raider.kind, &buildings, &hydro);
        let Some(route) = route::plan(&self.grid, &raider, &goal, self.gun) else {
            self.set_state(id, State::Holding);
            return;
        };
        if route.is_empty() {
            // At its goal. Only a mothership stops short of row 1.
            if raider.kind == Kind::Mothership {
                self.set_state(id, State::Holding);
                self.mothership_acts(id);
            }
            return;
        }
        // A wall in the way is bashed every phase, resting or not; a
        // burrowing raider passes under its first one in a single phase.
        if self.grid.is_wall(route[0]) {
            if raider.can_burrow() {
                if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                    r.burrowed = true;
                    r.at = route[0];
                    r.state = State::Approaching;
                }
                self.events.push(Event::Burrowed {
                    id,
                    kind: raider.kind,
                    wall: route[0],
                });
                return;
            }
            // In the dust a wall takes one bash less.
            let down =
                self.grid.bash(route[0]) || (self.in_dust(route[0]) && self.grid.bash(route[0]));
            self.set_state(id, State::Bashing { wall: route[0] });
            self.events.push(Event::Bashed {
                id,
                kind: raider.kind,
                wall: route[0],
                down,
            });
            return;
        }
        if raider.kind == Kind::Hulk {
            let rest = raider.rest;
            if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                r.rest = !rest;
            }
            if rest {
                return;
            }
        }
        self.set_state(id, State::Approaching);
        let steps = self.steps_of(raider.kind);
        for &to in route.iter().take(steps as usize) {
            if self.grid.is_wall(to) {
                break;
            }
            let from = self.raider(id).map(|r| r.at).unwrap_or(to);
            if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                r.at = to;
            }
            self.events.push(Event::Stepped {
                id,
                kind: raider.kind,
                from,
                to,
            });
            if self.grid.get(to) == Some(Work::Mine) {
                self.grid.set(to, None);
                if self.damage(id, grid::MINE_DAMAGE, Hitter::Mine) {
                    return;
                }
            }
            if to.row == NEAR_ROW {
                break;
            }
        }
    }

    /// A mule or the carrier takes its sideways step.
    fn cross(&mut self, id: u32) {
        let Some(raider) = self.raider(id).cloned() else {
            return;
        };
        let next = raider.at.col as i8 + raider.heading;
        if !(0..COLS as i8).contains(&next) {
            if raider.kind == Kind::Mule {
                self.raiders.retain(|r| r.id != id);
                self.events.push(Event::Left {
                    id,
                    kind: raider.kind,
                });
                return;
            }
            // The carrier turns at the edge and comes back.
            if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                r.heading = -r.heading;
            }
            return self.cross(id);
        }
        let to = Sector::new(next as u8, raider.at.row);
        if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
            r.at = to;
            r.posted += 1;
        }
        self.events.push(Event::Stepped {
            id,
            kind: raider.kind,
            from: raider.at,
            to,
        });
        if raider.kind == Kind::Carrier && (raider.posted + 1) % CARRIER_SIEGE_EVERY == 0 {
            let gun = self.gun;
            self.set_state(
                id,
                State::Charging {
                    col: gun,
                    phases: Weapon::Siege.wind_up(),
                },
            );
            self.events.push(Event::Charging {
                id,
                kind: Kind::Carrier,
                weapon: Weapon::Siege,
                col: gun,
            });
        }
    }

    /// A mothership at its post lets out skitters and winds up siege shots.
    fn mothership_acts(&mut self, id: u32) {
        let gun = self.gun;
        let Some(raider) = self.raiders.iter_mut().find(|r| r.id == id) else {
            return;
        };
        raider.posted += 1;
        let (posted, at) = (raider.posted, raider.at);
        if posted % MOTHERSHIP_SIEGE_EVERY == 0 {
            raider.state = State::Charging {
                col: gun,
                phases: Weapon::Siege.wind_up(),
            };
            self.events.push(Event::Charging {
                id,
                kind: Kind::Mothership,
                weapon: Weapon::Siege,
                col: gun,
            });
        } else if posted % MOTHERSHIP_DROP_EVERY == 0 {
            let side: i8 = if self.rng.below(2) == 0 { -1 } else { 1 };
            let col = (at.col as i8 + side).clamp(0, COLS as i8 - 1) as u8;
            let spot = Sector::new(col, at.row);
            let skitter = self.spawn(Kind::Skitter, spot, side, None);
            self.events.push(Event::Dropped {
                from: id,
                id: skitter,
                at: spot,
            });
        }
    }

    fn set_state(&mut self, id: u32, state: State) {
        if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
            r.state = state;
        }
    }

    fn spawn(&mut self, kind: Kind, at: Sector, lean: i8, quirk: Option<Quirk>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let mut raider = Raider::new(id, kind, at).with_quirk(quirk);
        raider.lean = lean;
        raider.heading = if at.col == 0 { 1 } else { -1 };
        self.raiders.push(raider);
        id
    }

    // ----- Nights -----

    fn ready(&mut self) {
        self.rng = Rng::seeded(mix(self.seed, self.night, 1));
        self.raiders.clear();
        self.turn = 1;
        self.take = Take::default();
        self.breached = false;
        self.siphoned = false;
        let forecast = self.forecast.clone();
        let mut arrivals = Vec::new();
        let mut pair = 0;
        for arrival in forecast {
            let row = if arrival.kind.crosses() {
                grid::CROSSING_ROW
            } else {
                FAR_ROW
            };
            let lean = if arrival.kind == Kind::Skitter {
                pair += 1;
                if pair % 2 == 1 {
                    -1
                } else {
                    1
                }
            } else {
                0
            };
            let at = Sector::new(arrival.col, row);
            let id = self.spawn(arrival.kind, at, lean, arrival.quirk);
            arrivals.push((id, arrival.kind, at));
        }
        let actions = self.colony.actions();
        self.phase = Phase::Turn { actions };
        self.events.push(Event::NightOpens {
            night: self.night,
            arrivals,
        });
        if let Some(storm) = &mut self.storm {
            *storm = Storm::new();
            self.events.push(Event::StormMoved { col: storm.col });
        }
        if self.order == Some(Order::Rationing) {
            // The raiders step once before the first turn.
            for id in self.phase_order() {
                if self.raider(id).is_some() {
                    self.step(id);
                }
            }
        }
        self.events.push(Event::YourTurn { turn: 1, actions });
    }

    fn take_order(&mut self, order: Order) -> Result<(), Refusal> {
        if self.order.is_some() {
            return Err(Refusal::OrderTaken);
        }
        if !self.offered.contains(&order) {
            return Err(Refusal::NoSuchOrder);
        }
        self.order = Some(order);
        match order {
            Order::Overclock => {
                self.colony.charge(orders::OVERCLOCK_SHIELD);
            }
            Order::Rationing => self.colony.resources.food += orders::RATIONING_FOOD,
            Order::Salvage => {}
        }
        self.events.push(Event::OrderTaken { order });
        Ok(())
    }

    fn hold_night(&mut self) {
        let clean = !self.breached;
        self.nights_held += 1;
        self.score += NIGHT_HELD_POINTS;
        if clean {
            self.score += CLEAN_NIGHT_POINTS;
        }
        self.tech += NIGHT_HELD_TECH;
        if self.endless() && self.night == RELIEF_NIGHT {
            // Relief lands and the long watch goes on.
            self.tech += RELIEF_TECH;
        }
        self.events.push(Event::Held {
            night: self.night,
            take: self.take,
            clean,
        });
        if !self.endless() && self.night >= self.relief {
            self.score += (self.colony.resources.power + self.colony.resources.metal) as u64;
            self.score = self.with_bonus(self.score);
            self.tech += RELIEF_TECH;
            self.phase = Phase::Relief;
            self.events.push(Event::Relief {
                score: self.score,
                tech: self.tech,
            });
            return;
        }
        self.night += 1;
        self.open_colony();
    }

    fn fall(&mut self) {
        self.phase = Phase::Fallen;
        self.score = self.with_bonus(self.score);
        self.events.push(Event::Fallen {
            night: self.night,
            score: self.score,
            tech: self.tech,
        });
    }

    /// After a night: the colony eats, the buildings produce, the works and
    /// buildings draw their power, and the forecast is read.
    fn open_colony(&mut self) {
        if let Some(order) = self.order.take() {
            if order == Order::Overclock {
                let r = &mut self.colony.resources;
                r.power = r.power.saturating_sub(orders::OVERCLOCK_POWER);
                self.events.push(Event::OrderPaid { order });
            }
        }
        let r = &mut self.colony.resources;
        r.food = r.food.saturating_sub(colony::FOOD_A_NIGHT);
        let mut dark = Vec::new();
        for col in 0..COLS {
            let Some(slot) = self.colony.slot(col).copied() else {
                continue;
            };
            if slot.dark {
                dark.push(col);
                continue;
            }
            match slot.building {
                Building::Solar => self.colony.resources.power += 3,
                Building::Hydroponics => self.colony.resources.food += 2,
                _ => {}
            }
        }
        let mut unpowered = Vec::new();
        for (at, work) in self.grid.works() {
            if let Work::Flak { .. } = work {
                let paid = self.colony.resources.spend(WorkKind::Flak.upkeep(), 0);
                self.grid.set(at, Some(Work::Flak { powered: paid }));
                if !paid {
                    unpowered.push(at);
                }
            }
        }
        for col in 0..COLS {
            let Some(slot) = self.colony.slot(col).copied() else {
                continue;
            };
            if slot.dark || slot.building.upkeep() == 0 {
                continue;
            }
            let paid = self.colony.resources.spend(slot.building.upkeep(), 0);
            if let Some(s) = self.colony.slot_mut(col) {
                s.powered = paid;
            }
            if paid && slot.building == Building::ShieldCoil {
                self.colony.charge(10);
            }
        }
        self.read_the_sky();
        self.phase = Phase::Colony;
        self.events.push(Event::ColonyOpens {
            night: self.night,
            hungry: self.colony.hungry(),
            unpowered,
            dark,
        });
    }

    fn draw_forecast(&self) -> Vec<Arrival> {
        let mut rng = Rng::seeded(mix(self.seed, self.night, 0));
        let mut out: Vec<Arrival> = Vec::new();
        let mut pair_col = None;
        for kind in nights::wave(self.night) {
            let col = match kind {
                Kind::Mothership => CENTRE,
                Kind::Mule | Kind::Carrier => {
                    if rng.below(2) == 0 {
                        0
                    } else {
                        COLS - 1
                    }
                }
                Kind::Skitter => match pair_col.take() {
                    Some(col) => col,
                    None => {
                        let col = rng.below(COLS as u32) as u8;
                        pair_col = Some(col);
                        col
                    }
                },
                _ => rng.below(COLS as u32) as u8,
            };
            out.push(Arrival {
                kind,
                col,
                quirk: None,
            });
        }
        // Quirks come after the columns, so the columns of the first nights
        // are what they always were.
        let mut quirks = nights::quirks_at_most(self.night);
        let mut slots: Vec<usize> = (0..out.len()).collect();
        rng.shuffle(&mut slots);
        for i in slots {
            if quirks == 0 {
                break;
            }
            let fits: Vec<Quirk> = Quirk::ALL
                .into_iter()
                .filter(|q| q.fits(out[i].kind))
                .collect();
            if fits.is_empty() {
                continue;
            }
            out[i].quirk = Some(fits[rng.below(fits.len() as u32) as usize]);
            quirks -= 1;
        }
        out
    }

    // ----- The colony phase -----

    fn buy(&mut self, building: Building, col: u8) -> Result<(), Refusal> {
        if !self.colony.open.contains(&col) {
            return Err(Refusal::SlotClosed);
        }
        if self.colony.slot(col).is_some() {
            return Err(Refusal::SlotTaken);
        }
        let (metal, power) = building.cost();
        if !self.colony.resources.spend(power, metal) {
            return Err(Refusal::CantAfford { power, metal });
        }
        self.colony.slots[col as usize] = Some(Slot {
            building,
            dark: false,
            powered: true,
        });
        self.events.push(Event::Bought { building, col });
        Ok(())
    }

    fn place(&mut self, work: WorkKind, at: Sector) -> Result<(), Refusal> {
        match self.grid.can_place(at) {
            Err(grid::Blocked::RowOpen) => return Err(Refusal::RowOpen),
            Err(grid::Blocked::Taken) => return Err(Refusal::SectorTaken),
            Ok(()) => {}
        }
        let metal = work.cost();
        if !self.colony.resources.spend(0, metal) {
            return Err(Refusal::CantAfford { power: 0, metal });
        }
        self.grid.set(at, Some(work.build()));
        self.events.push(Event::Placed { work, at });
        Ok(())
    }

    fn repair_dome(&mut self) -> Result<(), Refusal> {
        if self.colony.dome >= self.colony.dome_max {
            return Err(Refusal::DomeWhole);
        }
        if !self.colony.resources.spend(0, colony::DOME_REPAIR_COST) {
            return Err(Refusal::CantAfford {
                power: 0,
                metal: colony::DOME_REPAIR_COST,
            });
        }
        self.colony.dome = (self.colony.dome + colony::DOME_REPAIR).min(self.colony.dome_max);
        self.events.push(Event::DomeRepaired {
            dome: self.colony.dome,
        });
        Ok(())
    }

    fn repair_building(&mut self, col: u8) -> Result<(), Refusal> {
        let slot = self
            .colony
            .slot(col)
            .copied()
            .ok_or(Refusal::NothingThere)?;
        if !slot.dark {
            return Err(Refusal::NotDark);
        }
        if !self.colony.resources.spend(0, colony::BUILDING_REPAIR_COST) {
            return Err(Refusal::CantAfford {
                power: 0,
                metal: colony::BUILDING_REPAIR_COST,
            });
        }
        if let Some(s) = self.colony.slot_mut(col) {
            s.dark = false;
            s.powered = true;
        }
        self.events.push(Event::BuildingRepaired {
            building: slot.building,
            col,
        });
        Ok(())
    }

    fn make_missile(&mut self) -> Result<(), Refusal> {
        if self.colony.missiles >= self.colony.missiles_max {
            return Err(Refusal::MissilesFull);
        }
        let (power, metal) = self.colony.missile_cost();
        if !self.colony.resources.spend(power, metal) {
            return Err(Refusal::CantAfford { power, metal });
        }
        self.colony.missiles += 1;
        self.events.push(Event::MissileMade {
            missiles: self.colony.missiles,
        });
        Ok(())
    }
}

/// A seed for one night's draws, so a saved run resumes on the same
/// forecast and the same night.
pub(crate) fn mix(seed: u64, night: u8, salt: u64) -> u64 {
    seed ^ (night as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ salt.wrapping_mul(0xD1B5_4A32_D192_ED03)
}
