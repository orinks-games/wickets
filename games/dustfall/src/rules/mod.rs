//! The rules of Dustfall, with no sound, speech or clock of their own, so
//! every rule can be tested alone and the same code can referee an online
//! room later. Everything here is whole numbers and a seeded [`Rng`], so a
//! replay of the same seed, ticks and inputs lands on the same run
//! anywhere.
//!
//! Two state machines. The run is moved by [`Run::apply`] and
//! [`Run::tick`]:
//!
//! ```text
//! Colony --Ready--> Night --(sky clear, night 12)--> Relief
//!   ^                 |
//!   +--(sky clear)----+--(dome at 0)--> Fallen
//! ```
//!
//! The night is real time: the screen calls [`Run::tick`] ten times a
//! second and applies the player's inputs between ticks. The raiders come
//! in waves: a wave is called, its raiders come over the horizon a second
//! apart, and once the sky is clear of them a lull passes and the next
//! wave is called. Each tick the storm drifts, the wave's next raiders
//! arrive, the shield recharges, every raider falls or crosses or fires,
//! the flak posts fire, and the night ends when the last wave is down.
//! Each raider's own machine is in [`raiders`]. The screen turns the [`Event`]s a tick
//! raises into sound and speech; nothing here is spoken.
//!
//! A run with `relief` 0 is the long watch: Relief lands on night 12 as a
//! milestone and the nights go on until the dome falls. A run with a
//! [`Storm`] is the dust storm event. [`Perks`] carry what the player's
//! unlocks change, so a replay needs nothing but the run.

pub mod colony;
pub mod lanes;
pub mod nights;
pub mod orders;
pub mod perks;
pub mod raiders;
pub mod storm;

use parlor_core::Rng;
use serde::{Deserialize, Serialize};

pub use colony::{Building, Colony, Resources, Slot};
pub use lanes::{lane_name, place_words, Range, CENTRE, HORIZON, LANES, MID, NEAR, TICKS_A_SECOND};
pub use nights::RELIEF_NIGHT;
pub use orders::Order;
pub use perks::Perks;
pub use raiders::{Charge, Kind, Quirk, Raider, State, Take, Weapon};
pub use storm::Storm;

/// Points for holding a night, and as much again for one with no breach.
pub const NIGHT_HELD_POINTS: u64 = 50;
pub const CLEAN_NIGHT_POINTS: u64 = 50;
/// Tech points for a night held, and for Relief.
pub const NIGHT_HELD_TECH: u32 = 1;
pub const RELIEF_TECH: u32 = 5;
/// Bolts and siphons that can be winding up at once, so the warnings stay
/// readable. Siege shots are not counted.
pub const MAX_CHARGES: usize = 2;
/// Ticks the gun takes to fire again, fed and hungry.
pub const GUN_COOLDOWN: u32 = 3;
pub const GUN_COOLDOWN_HUNGRY: u32 = 6;
/// A hovering mothership shifts a lane every this many ticks, and drops a
/// drifter with every second siege shot.
pub const MOTHERSHIP_SHIFT_EVERY: u32 = 30;
pub const MOTHERSHIP_DROP_EVERY: u32 = 2;
/// The carrier lets a skitter down over every other lane it crosses.
pub const CARRIER_DROP_LANES: [u8; 3] = [0, 2, 4];
/// The pace of a practice night, in percent of the real one.
pub const PRACTICE_PACE: u32 = 30;

/// Where the run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Between nights: spending, then Ready.
    Colony,
    /// The night, ticking.
    Night,
    /// Relief landed: the run is won.
    Relief,
    /// The dome fell: the run is lost.
    Fallen,
}

/// A raider the forecast promises: its kind, the lane it enters at, its
/// quirk, the wave it comes in, and the tick it comes over the horizon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrival {
    pub kind: Kind,
    pub lane: u8,
    #[serde(default)]
    pub quirk: Option<Quirk>,
    /// The wave it comes in, from 1.
    #[serde(default)]
    pub wave: u8,
    /// The tick of the night it arrives on; set as its wave is called.
    #[serde(default)]
    pub at: u32,
}

/// Everything a player can do. Each is one input of a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    /// Slide the gun one lane.
    Left,
    Right,
    /// Shoot up the gun's lane.
    Fire,
    /// Launch a missile at the raider nearest the dome.
    Missile,
    /// Colony phase: a building in the slot under `lane`.
    Buy {
        building: Building,
        lane: u8,
    },
    /// Colony phase: a mine in a lane for the night to come.
    LayMine {
        lane: u8,
    },
    RepairDome,
    RepairBuilding {
        lane: u8,
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
    /// The gun has not reloaded.
    Reloading,
    NoMissiles,
    /// Nothing in the sky to fire a missile at.
    NoTarget,
    /// Short of power or metal; what it would take.
    CantAfford {
        power: u32,
        metal: u32,
    },
    SlotClosed,
    SlotTaken,
    /// A mine waits in that lane already.
    LaneMined,
    /// As many mines as may be laid before one night.
    MinesLaid,
    /// No building under that lane.
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
    /// The flak post under this lane.
    Flak(u8),
}

/// What happened, in order, for the screen to tell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// The night has begun; `coming` raiders will come over the horizon.
    NightOpens {
        night: u8,
        coming: usize,
        waves: u8,
    },
    /// The last raider of a wave is gone, and another wave is to come.
    WaveClear {
        wave: u8,
        of: u8,
    },
    /// A wave is called: its raiders start arriving a second from now.
    WaveComing {
        wave: u8,
        of: u8,
        kinds: Vec<Kind>,
    },
    /// A raider came over the horizon, or in from the edge.
    Arrived {
        id: u32,
        kind: Kind,
        lane: u8,
        quirk: Option<Quirk>,
    },
    /// A raider came into a nearer range.
    Ranged {
        id: u32,
        kind: Kind,
        lane: u8,
        range: Range,
    },
    /// The gun slid to `lane`; `under` is the nearest raider in it.
    Moved {
        lane: u8,
        under: Option<(Kind, Range)>,
    },
    /// The gun fired up an empty lane.
    DryShot {
        lane: u8,
    },
    /// The gun's shot glanced off an armoured hull still at far range.
    Glanced {
        id: u32,
        kind: Kind,
        lane: u8,
        height: u32,
    },
    Hit {
        id: u32,
        kind: Kind,
        lane: u8,
        height: u32,
        left: u8,
        by: Hitter,
    },
    Down {
        id: u32,
        kind: Kind,
        lane: u8,
        height: u32,
        take: Take,
        by: Hitter,
    },
    /// A raider began winding up a shot at `lane`.
    Charging {
        id: u32,
        kind: Kind,
        weapon: Weapon,
        lane: u8,
    },
    /// A wound-up shot went off.
    Shot {
        id: u32,
        kind: Kind,
        weapon: Weapon,
        lane: u8,
        dodged: bool,
        shield: u32,
        dome: u32,
    },
    Breach {
        id: u32,
        kind: Kind,
        lane: u8,
        shield: u32,
        dome: u32,
        darkened: Option<Building>,
    },
    /// A weaving raider changed lane.
    Weaved {
        id: u32,
        kind: Kind,
        from: u8,
        to: u8,
    },
    /// A crossing raider is over a new lane.
    Crossed {
        id: u32,
        kind: Kind,
        lane: u8,
    },
    /// A mothership reached mid range and holds there.
    Hovering {
        id: u32,
        lane: u8,
    },
    /// A hovering mothership shifted to `lane`.
    Shifted {
        id: u32,
        lane: u8,
    },
    /// A mothership or the carrier let a raider out.
    Dropped {
        from: u32,
        id: u32,
        kind: Kind,
        lane: u8,
    },
    /// A mule crossed the sky and left.
    Left {
        id: u32,
        kind: Kind,
    },
    /// The mine in `lane` went off.
    MineBlew {
        lane: u8,
    },
    /// The dust storm drifted to `lane`.
    StormMoved {
        lane: u8,
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
        /// Lanes whose building went without power.
        unpowered: Vec<u8>,
        /// Lanes whose building is dark.
        dark: Vec<u8>,
    },
    Bought {
        building: Building,
        lane: u8,
    },
    MineLaid {
        lane: u8,
    },
    DomeRepaired {
        dome: u32,
    },
    BuildingRepaired {
        building: Building,
        lane: u8,
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

fn full_pace() -> u32 {
    100
}

fn no_flak() -> Vec<u32> {
    vec![0; LANES as usize]
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
    /// The pace of the night in percent: 100, or slower for practice.
    #[serde(default = "full_pace")]
    pub pace: u32,
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
    /// Ticks since the night opened.
    #[serde(default)]
    pub tick: u32,
    pub colony: Colony,
    /// The raiders in the sky.
    pub raiders: Vec<Raider>,
    /// The raiders of the current wave still to come, soonest first.
    #[serde(default)]
    pub pending: Vec<Arrival>,
    /// The waves still to be called tonight.
    #[serde(default)]
    pub waves: Vec<Vec<Arrival>>,
    /// The wave in the sky, from 1; 0 before the first is called.
    #[serde(default)]
    pub wave: u8,
    /// The tick the next wave is called on, once the sky is clear.
    #[serde(default)]
    wave_at: u32,
    /// The gun's lane.
    pub gun: u8,
    /// Ticks until the gun can fire again.
    #[serde(default)]
    pub cooldown: u32,
    /// The shield does not recharge before this tick.
    #[serde(default)]
    pub siphoned_until: u32,
    /// The tick the storm last drifted on.
    #[serde(default)]
    storm_at: u32,
    /// The tick each lane's flak post last fired on.
    #[serde(default = "no_flak")]
    flak_at: Vec<u32>,
    /// Mines laid this colony phase.
    #[serde(default)]
    pub mines_laid: u32,
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
        let mut run = Run {
            seed,
            first: 1,
            relief: RELIEF_NIGHT,
            perks,
            pace: 100,
            offered: Vec::new(),
            order: None,
            storm: None,
            night: 1,
            phase: Phase::Colony,
            tick: 0,
            colony: Colony::with(perks),
            raiders: Vec::new(),
            pending: Vec::new(),
            waves: Vec::new(),
            wave: 0,
            wave_at: 0,
            gun: CENTRE,
            cooldown: 0,
            siphoned_until: 0,
            storm_at: 0,
            flak_at: no_flak(),
            mines_laid: 0,
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

    /// The tutorial's run: night 1 at a third of the pace, so there is
    /// time to hear each thing.
    pub fn practice(seed: u64) -> Run {
        let mut run = Run::new(seed);
        run.pace = PRACTICE_PACE;
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
            (Phase::Night, Input::Left) => self.slide(-1),
            (Phase::Night, Input::Right) => self.slide(1),
            (Phase::Night, Input::Fire) => self.fire(),
            (Phase::Night, Input::Missile) => self.missile(),
            (Phase::Colony, Input::Buy { building, lane }) => self.buy(building, lane),
            (Phase::Colony, Input::LayMine { lane }) => self.lay_mine(lane),
            (Phase::Colony, Input::RepairDome) => self.repair_dome(),
            (Phase::Colony, Input::RepairBuilding { lane }) => self.repair_building(lane),
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

    /// The raiders nearest the dome first.
    pub fn nearest_first(&self) -> Vec<&Raider> {
        let mut all: Vec<&Raider> = self.raiders.iter().collect();
        all.sort_by_key(|r| (r.height, r.lane, r.id));
        all
    }

    pub fn raider(&self, id: u32) -> Option<&Raider> {
        self.raiders.iter().find(|r| r.id == id)
    }

    /// The nearest raider in `lane`.
    pub fn under(&self, lane: u8) -> Option<&Raider> {
        self.raiders
            .iter()
            .filter(|r| r.lane == lane)
            .min_by_key(|r| (r.height, r.id))
    }

    /// Whether the dust storm covers `lane` now.
    pub fn in_dust(&self, lane: u8) -> bool {
        self.storm.is_some_and(|s| s.covers(lane))
    }

    /// The raiders the scan can hear: not the ones in the dust.
    pub fn heard(&self) -> Vec<&Raider> {
        self.nearest_first()
            .into_iter()
            .filter(|r| !self.in_dust(r.lane))
            .collect()
    }

    /// Seconds since the night opened.
    pub fn seconds(&self) -> u32 {
        self.tick / TICKS_A_SECOND
    }

    /// Whether nothing is in the sky and nothing is still to come.
    pub fn sky_clear(&self) -> bool {
        self.raiders.is_empty() && self.pending.is_empty() && self.waves.is_empty()
    }

    /// Whether the wave in the sky is over, with more to come.
    fn wave_clear(&self) -> bool {
        self.raiders.is_empty() && self.pending.is_empty() && !self.waves.is_empty()
    }

    /// Raiders still to come tonight, this wave's and the waves after.
    pub fn to_come(&self) -> usize {
        self.pending.len() + self.waves.iter().map(Vec::len).sum::<usize>()
    }

    /// How many waves the night has.
    pub fn waves_total(&self) -> u8 {
        self.forecast.iter().map(|a| a.wave).max().unwrap_or(0)
    }

    /// Ticks until the next wave is called, while the sky waits for it.
    pub fn next_wave_in(&self) -> Option<u32> {
        self.wave_clear()
            .then(|| self.wave_at.saturating_sub(self.tick))
    }

    /// The score with the run's bonus on it, for Hard vacuum and the
    /// call-outs off.
    pub fn with_bonus(&self, score: u64) -> u64 {
        score + score * u64::from(self.perks.bonus_percent()) / 100
    }

    /// The charges winding up: raider id, its kind and the lane aimed at.
    pub fn charges(&self) -> Vec<(u32, Kind, u8)> {
        self.nearest_first()
            .into_iter()
            .filter_map(|r| r.charge.map(|c| (r.id, r.kind, c.lane)))
            .collect()
    }

    /// Range units a raider falls a tick, with its quirk, the night's
    /// order and the run's pace.
    pub fn speed_of(&self, raider: &Raider) -> u32 {
        let mut speed = raider.kind.speed() * 100;
        if raider.has(Quirk::Hasty) {
            speed = speed * 125 / 100;
        }
        if raider.kind == Kind::Skitter && self.order == Some(Order::Salvage) {
            speed = speed * 150 / 100;
        }
        speed = speed * self.pace / 100;
        ((speed + 50) / 100).max(1)
    }

    /// "2 drifters, a skitter pair, a mule, in 3 waves".
    pub fn forecast_words(&self) -> String {
        let kinds: Vec<Kind> = self.forecast.iter().map(|a| a.kind).collect();
        let waves = self.waves_total();
        if waves > 1 {
            format!("{}, in {waves} waves", nights::describe(&kinds))
        } else {
            nights::describe(&kinds)
        }
    }

    /// The forecast's waves, each as "a skitter pair and a sapper".
    pub fn wave_words(&self) -> Vec<String> {
        (1..=self.waves_total())
            .map(|w| {
                let kinds: Vec<Kind> = self
                    .forecast
                    .iter()
                    .filter(|a| a.wave == w)
                    .map(|a| a.kind)
                    .collect();
                nights::describe(&kinds)
            })
            .collect()
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

    // ----- The gun -----

    fn slide(&mut self, dir: i8) -> Result<(), Refusal> {
        let lane = self.gun as i8 + dir;
        if !(0..LANES as i8).contains(&lane) {
            return Err(Refusal::Edge);
        }
        self.gun = lane as u8;
        let under = self.under(self.gun).map(|r| (r.kind, r.range()));
        self.events.push(Event::Moved {
            lane: self.gun,
            under,
        });
        Ok(())
    }

    fn fire(&mut self) -> Result<(), Refusal> {
        if self.cooldown > 0 {
            return Err(Refusal::Reloading);
        }
        self.cooldown = if self.colony.hungry() {
            GUN_COOLDOWN_HUNGRY
        } else {
            GUN_COOLDOWN
        };
        match self
            .under(self.gun)
            .map(|r| (r.id, r.kind, r.lane, r.height))
        {
            // A mothership or the carrier is too well hulled for the gun
            // until it is at mid range: it has to be let in and gone to.
            Some((id, kind, lane, height))
                if kind.armoured() && Range::of(height) == Range::Far =>
            {
                self.events.push(Event::Glanced {
                    id,
                    kind,
                    lane,
                    height,
                });
            }
            Some((id, ..)) => {
                self.damage(id, 1, Hitter::Gun);
            }
            None => self.events.push(Event::DryShot { lane: self.gun }),
        }
        Ok(())
    }

    fn missile(&mut self) -> Result<(), Refusal> {
        if self.colony.missiles == 0 {
            return Err(Refusal::NoMissiles);
        }
        let gun = self.gun;
        let target = self
            .raiders
            .iter()
            .min_by_key(|r| (r.height, r.lane.abs_diff(gun), r.lane, r.id))
            .map(|r| r.id)
            .ok_or(Refusal::NoTarget)?;
        self.colony.missiles -= 1;
        self.damage(target, colony::MISSILE_DAMAGE, Hitter::Missile);
        Ok(())
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
                lane: raider.lane,
                height: raider.height,
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
            lane: raider.lane,
            height: raider.height,
            take,
            by,
        });
        true
    }

    // ----- The night, a tick at a time -----

    /// One tick of the night: a tenth of a second. Nothing happens in any
    /// other phase.
    pub fn tick(&mut self) {
        if self.phase != Phase::Night {
            return;
        }
        self.tick += 1;
        self.cooldown = self.cooldown.saturating_sub(1);
        if let Some(storm) = &mut self.storm {
            if self.tick - self.storm_at >= storm::STORM_EVERY {
                storm.advance();
                self.storm_at = self.tick;
                let lane = storm.lane;
                self.events.push(Event::StormMoved { lane });
            }
        }
        // A wave shot down between ticks starts its lull here.
        self.wave_down();
        self.call_waves();
        while self.pending.first().is_some_and(|a| a.at <= self.tick) {
            let arrival = self.pending.remove(0);
            self.arrive(arrival);
        }
        self.recharge();
        let ids: Vec<u32> = self.raiders.iter().map(|r| r.id).collect();
        for id in ids {
            if self.raider(id).is_some() {
                self.act(id);
            }
            if self.phase != Phase::Night {
                return;
            }
        }
        self.flak();
        if self.phase != Phase::Night {
            return;
        }
        self.wave_down();
        if self.sky_clear() {
            self.hold_night();
        }
    }

    /// Ticks the night on `n` times.
    pub fn ticks(&mut self, n: u32) {
        for _ in 0..n {
            self.tick();
        }
    }

    /// Calls the next wave once the lull after the last is over, and
    /// starts the lull once a wave is down.
    fn call_waves(&mut self) {
        if !self.wave_clear() {
            return;
        }
        let of = self.waves_total();
        if self.tick >= self.wave_at {
            let mut wave = self.waves.remove(0);
            self.wave += 1;
            let mut stagger = nights::WAVE_STAGGER;
            if self.order == Some(Order::Rationing) {
                stagger = stagger * 2 / 3;
            }
            let stagger = (stagger * 100 / self.pace.max(1)).max(1);
            let mut at = self.tick + nights::WAVE_LEAD * 100 / self.pace.max(1);
            let mut pair = false;
            for arrival in &mut wave {
                arrival.at = at;
                // The second of a skitter pair is right behind the first.
                if arrival.kind == Kind::Skitter && !pair {
                    pair = true;
                    at += stagger / 2;
                } else {
                    pair = false;
                    at += stagger;
                }
            }
            let kinds = wave.iter().map(|a| a.kind).collect();
            self.pending = wave;
            self.events.push(Event::WaveComing {
                wave: self.wave,
                of,
                kinds,
            });
        }
    }

    /// Starts the lull once the wave in the sky is down.
    fn wave_down(&mut self) {
        if !self.wave_clear() || self.wave_at >= self.tick {
            return;
        }
        let mut lull = nights::WAVE_LULL;
        if self.order == Some(Order::Rationing) {
            lull = lull * 2 / 3;
        }
        self.wave_at = self.tick + lull;
        self.events.push(Event::WaveClear {
            wave: self.wave,
            of: self.waves_total(),
        });
    }

    /// The shield comes back a point at a time, faster with a coil, not
    /// at all for a while after a siphon.
    fn recharge(&mut self) {
        if self.tick < self.siphoned_until {
            return;
        }
        let every = if self.colony.has(Building::ShieldCoil) {
            colony::RECHARGE_EVERY_COIL
        } else {
            colony::RECHARGE_EVERY
        };
        if self.tick.is_multiple_of(every) {
            self.colony.charge(1);
        }
    }

    fn arrive(&mut self, arrival: Arrival) {
        let id = self.next_id;
        self.next_id += 1;
        let height = match arrival.kind {
            Kind::Mule => MID,
            Kind::Carrier => lanes::CARRIER_HEIGHT,
            _ => HORIZON,
        };
        let mut raider =
            Raider::new(id, arrival.kind, arrival.lane, height).with_quirk(arrival.quirk);
        if let Some(weapon) = arrival.kind.weapon() {
            // Ready to fire as soon as it is in range.
            raider.since_shot = weapon.rest();
        }
        self.raiders.push(raider);
        self.events.push(Event::Arrived {
            id,
            kind: arrival.kind,
            lane: arrival.lane,
            quirk: arrival.quirk,
        });
    }

    /// One raider's tick.
    fn act(&mut self, id: u32) {
        let Some(raider) = self.raider(id).cloned() else {
            return;
        };
        if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
            r.age += 1;
            r.since_shot += 1;
        }
        if let Some(charge) = raider.charge {
            if charge.ticks > 1 {
                if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                    r.charge = Some(Charge {
                        ticks: charge.ticks - 1,
                        ..charge
                    });
                }
            } else {
                self.shoot(id, charge);
                if self.phase != Phase::Night {
                    return;
                }
            }
        }
        match raider.state {
            State::Falling => self.fall_tick(id, &raider),
            State::Hovering => self.hover_tick(id, &raider),
            State::Crossing => self.cross_tick(id, &raider),
        }
    }

    fn fall_tick(&mut self, id: u32, raider: &Raider) {
        let speed = self.speed_of(raider);
        let before = raider.height;
        let after = before.saturating_sub(speed);
        if raider.kind == Kind::Mothership && after <= MID {
            if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                r.height = MID;
                r.state = State::Hovering;
                r.age = 0;
                r.heading = if self.rng.below(2) == 0 { -1 } else { 1 };
            }
            self.events.push(Event::Hovering {
                id,
                lane: raider.lane,
            });
            return;
        }
        if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
            r.height = after;
        }
        let (was, now) = (Range::of(before), Range::of(after));
        if now != was && now != Range::Dome {
            self.events.push(Event::Ranged {
                id,
                kind: raider.kind,
                lane: raider.lane,
                range: now,
            });
        }
        if before >= MID && after < MID && self.colony.mined(raider.lane) && !raider.kind.armoured()
        {
            self.colony.mines[raider.lane as usize] = false;
            self.events.push(Event::MineBlew { lane: raider.lane });
            if self.damage(id, colony::MINE_DAMAGE, Hitter::Mine) {
                return;
            }
        }
        if after < MID && raider.can_weave() {
            self.weave(id, raider);
        }
        if after == 0 {
            return self.breach(id);
        }
        self.consider_charge(id);
    }

    fn hover_tick(&mut self, id: u32, raider: &Raider) {
        if raider.age > 0 && raider.age.is_multiple_of(MOTHERSHIP_SHIFT_EVERY) {
            let mut heading = raider.heading;
            let mut next = raider.lane as i8 + heading;
            if !(0..LANES as i8).contains(&next) {
                heading = -heading;
                next = raider.lane as i8 + heading;
            }
            if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                r.lane = next as u8;
                r.heading = heading;
            }
            self.events.push(Event::Shifted {
                id,
                lane: next as u8,
            });
        }
        self.consider_charge(id);
    }

    fn cross_tick(&mut self, id: u32, raider: &Raider) {
        let every = raider.kind.cross_every() * 100 / self.pace.max(1);
        if raider.age.is_multiple_of(every.max(1)) {
            let next = raider.lane as i8 + raider.heading;
            if !(0..LANES as i8).contains(&next) {
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
            } else {
                let lane = next as u8;
                if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
                    r.lane = lane;
                }
                self.events.push(Event::Crossed {
                    id,
                    kind: raider.kind,
                    lane,
                });
                if raider.kind == Kind::Carrier && CARRIER_DROP_LANES.contains(&lane) {
                    self.drop_from(id, Kind::Skitter, lane);
                }
            }
        }
        self.consider_charge(id);
    }

    /// A weaving raider slips a lane away from the gun.
    fn weave(&mut self, id: u32, raider: &Raider) {
        let from = raider.lane;
        let side: i8 = if from > self.gun || (from == self.gun && self.rng.below(2) == 1) {
            1
        } else {
            -1
        };
        let mut to = from as i8 + side;
        if !(0..LANES as i8).contains(&to) {
            to = from as i8 - side;
        }
        let to = to as u8;
        if let Some(r) = self.raiders.iter_mut().find(|r| r.id == id) {
            r.lane = to;
            r.weaved = true;
        }
        self.events.push(Event::Weaved {
            id,
            kind: raider.kind,
            from,
            to,
        });
    }

    /// A raider with a weapon, rested and in range, winds up at the gun's
    /// lane.
    fn consider_charge(&mut self, id: u32) {
        let gun = self.gun;
        let docile = nights::docile(self.night);
        let pending = self
            .raiders
            .iter()
            .filter(|r| r.charge.is_some_and(|c| c.weapon != Weapon::Siege))
            .count();
        let Some(raider) = self.raiders.iter_mut().find(|r| r.id == id) else {
            return;
        };
        let Some(weapon) = raider.kind.weapon() else {
            return;
        };
        if raider.charge.is_some()
            || raider.since_shot < weapon.rest()
            || raider.range() < raider.kind.fires_from()
            || raider.range() == Range::Dome
            || (docile && raider.kind == Kind::Drifter)
            || (weapon != Weapon::Siege && pending >= MAX_CHARGES)
        {
            return;
        }
        raider.charge = Some(Charge {
            weapon,
            lane: gun,
            ticks: weapon.wind_up(),
        });
        self.events.push(Event::Charging {
            id,
            kind: raider.kind,
            weapon,
            lane: gun,
        });
    }

    /// A wound-up charge goes off.
    fn shoot(&mut self, id: u32, charge: Charge) {
        let Some(raider) = self.raiders.iter_mut().find(|r| r.id == id) else {
            return;
        };
        raider.charge = None;
        raider.since_shot = 0;
        raider.fired += 1;
        let (kind, fired, lane) = (raider.kind, raider.fired, raider.lane);
        let dodged = charge.weapon.dodgeable() && self.gun != charge.lane;
        if !dodged {
            self.colony
                .hit(charge.weapon.damage(), charge.weapon.through());
            if charge.weapon == Weapon::Siphon {
                self.siphoned_until = self.tick + colony::SIPHON_TICKS;
            }
        }
        self.events.push(Event::Shot {
            id,
            kind,
            weapon: charge.weapon,
            lane: charge.lane,
            dodged,
            shield: self.colony.shield,
            dome: self.colony.dome,
        });
        if self.colony.fallen() {
            return self.fall();
        }
        if kind == Kind::Mothership && fired % MOTHERSHIP_DROP_EVERY == 0 {
            self.drop_from(id, Kind::Drifter, lane);
        }
    }

    /// A mothership or the carrier lets a raider out over `lane`.
    fn drop_from(&mut self, from: u32, kind: Kind, lane: u8) {
        let id = self.next_id;
        self.next_id += 1;
        let mut raider = Raider::arriving(id, kind, lane);
        if let Some(weapon) = kind.weapon() {
            raider.since_shot = weapon.rest();
        }
        self.raiders.push(raider);
        self.events.push(Event::Dropped {
            from,
            id,
            kind,
            lane,
        });
    }

    /// A raider at the dome.
    fn breach(&mut self, id: u32) {
        let Some(index) = self.raiders.iter().position(|r| r.id == id) else {
            return;
        };
        let raider = self.raiders.remove(index);
        if !raider.kind.hostile() {
            return;
        }
        let lane = raider.lane;
        let (_, to_dome) = self.colony.hit(raider.kind.breach(), 0);
        self.breached = true;
        let mut darkened = None;
        if to_dome > 0 || raider.kind == Kind::Sapper {
            if let Some(slot) = self.colony.slot_mut(lane) {
                if !slot.dark {
                    slot.dark = true;
                    darkened = Some(slot.building);
                }
            }
        }
        self.events.push(Event::Breach {
            id,
            kind: raider.kind,
            lane,
            shield: self.colony.shield,
            dome: self.colony.dome,
            darkened,
        });
        if self.colony.fallen() {
            self.fall();
        }
    }

    /// Every working flak post fires up its lane when it is ready and has
    /// something to fire at. A post in the dust cannot see to fire. Flak
    /// scatters off a mothership's or the carrier's hull.
    fn flak(&mut self) {
        for lane in self.colony.flak_lanes() {
            if self.in_dust(lane) || self.tick - self.flak_at[lane as usize] < colony::FLAK_EVERY {
                continue;
            }
            let target = self
                .raiders
                .iter()
                .filter(|r| r.lane == lane && !r.kind.armoured() && r.height < lanes::FLAK_REACH)
                .min_by_key(|r| (r.height, r.id))
                .map(|r| r.id);
            if let Some(id) = target {
                self.flak_at[lane as usize] = self.tick;
                self.damage(id, 1, Hitter::Flak(lane));
            }
        }
    }

    // ----- Nights -----

    fn ready(&mut self) {
        self.rng = Rng::seeded(mix(self.seed, self.night, 1));
        self.raiders.clear();
        self.tick = 0;
        self.cooldown = 0;
        self.siphoned_until = 0;
        self.storm_at = 0;
        self.flak_at = no_flak();
        self.take = Take::default();
        self.breached = false;
        let (buildings, hydro) = self.colony.building_lanes();
        let mut waves: Vec<Vec<Arrival>> = Vec::new();
        for mut arrival in self.forecast.clone() {
            if arrival.kind == Kind::Sapper {
                let lanes = if !hydro.is_empty() {
                    &hydro
                } else {
                    &buildings
                };
                if !lanes.is_empty() {
                    arrival.lane = lanes[self.rng.below(lanes.len() as u32) as usize];
                }
            }
            arrival.at = 0;
            let wave = arrival.wave.max(1) as usize;
            while waves.len() < wave {
                waves.push(Vec::new());
            }
            waves[wave - 1].push(arrival);
        }
        waves.retain(|w| !w.is_empty());
        let coming = waves.iter().map(Vec::len).sum();
        let count = waves.len() as u8;
        self.pending = Vec::new();
        self.waves = waves;
        self.wave = 0;
        self.wave_at = nights::FIRST_WAVE;
        self.phase = Phase::Night;
        self.events.push(Event::NightOpens {
            night: self.night,
            coming,
            waves: count,
        });
        if let Some(storm) = &mut self.storm {
            *storm = Storm::new();
            self.events.push(Event::StormMoved { lane: storm.lane });
        }
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

    /// After a night: the colony eats, the buildings produce and draw
    /// their power, and the forecast is read.
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
        for lane in 0..LANES {
            let Some(slot) = self.colony.slot(lane).copied() else {
                continue;
            };
            if slot.dark {
                dark.push(lane);
                continue;
            }
            match slot.building {
                Building::Solar => self.colony.resources.power += 3,
                Building::Hydroponics => self.colony.resources.food += 2,
                _ => {}
            }
        }
        let mut unpowered = Vec::new();
        for lane in 0..LANES {
            let Some(slot) = self.colony.slot(lane).copied() else {
                continue;
            };
            if slot.dark || slot.building.upkeep() == 0 {
                continue;
            }
            let paid = self.colony.resources.spend(slot.building.upkeep(), 0);
            if let Some(s) = self.colony.slot_mut(lane) {
                s.powered = paid;
            }
            if !paid {
                unpowered.push(lane);
            }
            if paid && slot.building == Building::ShieldCoil {
                self.colony.charge(10);
            }
        }
        self.mines_laid = 0;
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
        let mut pair_lane = None;
        let mut last = None;
        for (w, wave) in nights::waves(self.night).into_iter().enumerate() {
            for kind in wave {
                let lane = match kind {
                    Kind::Mothership => CENTRE,
                    Kind::Mule | Kind::Carrier => {
                        if rng.below(2) == 0 {
                            0
                        } else {
                            LANES - 1
                        }
                    }
                    Kind::Skitter => match pair_lane.take() {
                        Some(lane) => lane,
                        None => {
                            let lane = rng.below(LANES as u32) as u8;
                            pair_lane = Some(lane);
                            lane
                        }
                    },
                    _ => {
                        // Two in a row never take the same lane.
                        let mut lane = rng.below(LANES as u32) as u8;
                        if last == Some(lane) {
                            lane = (lane + 1 + rng.below(LANES as u32 - 1) as u8) % LANES;
                        }
                        lane
                    }
                };
                last = Some(lane);
                out.push(Arrival {
                    kind,
                    lane,
                    quirk: None,
                    wave: w as u8 + 1,
                    at: 0,
                });
            }
        }
        // Quirks come after the lanes, so the lanes of the first nights
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

    fn buy(&mut self, building: Building, lane: u8) -> Result<(), Refusal> {
        if !self.colony.open.contains(&lane) {
            return Err(Refusal::SlotClosed);
        }
        if self.colony.slot(lane).is_some() {
            return Err(Refusal::SlotTaken);
        }
        let (metal, power) = building.cost();
        if !self.colony.resources.spend(power, metal) {
            return Err(Refusal::CantAfford { power, metal });
        }
        self.colony.slots[lane as usize] = Some(Slot {
            building,
            dark: false,
            powered: true,
        });
        self.events.push(Event::Bought { building, lane });
        Ok(())
    }

    fn lay_mine(&mut self, lane: u8) -> Result<(), Refusal> {
        if lane >= LANES || self.colony.mined(lane) {
            return Err(Refusal::LaneMined);
        }
        if self.mines_laid >= self.colony.mines_a_night {
            return Err(Refusal::MinesLaid);
        }
        if !self.colony.resources.spend(0, colony::MINE_COST) {
            return Err(Refusal::CantAfford {
                power: 0,
                metal: colony::MINE_COST,
            });
        }
        self.colony.mines[lane as usize] = true;
        self.mines_laid += 1;
        self.events.push(Event::MineLaid { lane });
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

    fn repair_building(&mut self, lane: u8) -> Result<(), Refusal> {
        let slot = self
            .colony
            .slot(lane)
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
        if let Some(s) = self.colony.slot_mut(lane) {
            s.dark = false;
            s.powered = true;
        }
        self.events.push(Event::BuildingRepaired {
            building: slot.building,
            lane,
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
