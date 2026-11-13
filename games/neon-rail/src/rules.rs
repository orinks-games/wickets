//! The rules of one run, with no screen and no sound device: a plain struct
//! driven by `update(dt)`, so tests can play a whole run deterministically.
//!
//! This is a port of the NVGT challenge build of Neon Rail: the same three
//! lanes, ten vehicle and hazard types, stress meter, priority traffic,
//! emergency tools, scoring and sector bonuses, with the same numbers. What
//! the run wants heard or said goes out as [`Note`]s, which the run screen
//! turns into sound and speech.

use hearken::text::sentence_start;
use parlor_core::{Period, Rng};

use crate::cues;
use crate::script;

/// How long a boost lasts, in seconds.
pub const BOOST_DURATION: f64 = 0.9;
/// How long a boost takes to recharge.
pub const BOOST_COOLDOWN: f64 = 5.0;
/// How long the rail gun takes to cool.
pub const WEAPON_COOLDOWN: f64 = 0.9;
/// The rail gun's lock window, as the original scored it: lanes away times
/// a tenth of a second, plus the time to arrival, must come in under this.
/// A target in your own lane locks inside a second of arrival, one lane over
/// inside 0.9 seconds, two lanes over inside 0.8.
pub const WEAPON_RANGE: f64 = 1.0;
/// How long a slowdown pulse lasts.
pub const SLOWDOWN_DURATION: f64 = 2.5;
/// How long a slowdown pulse takes to recharge.
pub const SLOWDOWN_COOLDOWN: f64 = 14.0;
/// How long a priority override holds the route.
pub const OVERRIDE_DURATION: f64 = 3.5;
/// How often the overload warning repeats while stress is high.
pub const WARNING_REPEAT: f64 = 5.0;
/// The standard warning time for a hazard.
pub const APPROACH: f64 = 1.8;
/// The warning time for a pickup.
pub const PICKUP_APPROACH: f64 = 2.2;
/// The stress at which the network overloads and the run ends.
pub const MAX_STRESS: i32 = 100;
/// How long before a blockade arrives its boost chime sounds.
pub const BOOST_CHIME_LEAD: f64 = 0.8;
/// The challenge corridor's length.
pub const CHALLENGE_LENGTH: f64 = 120.0;
/// The guided run's length.
pub const TUTORIAL_LENGTH: f64 = 60.0;
/// Sectors in a Black Sky Run, the last one the extraction sprint.
pub const BLACK_SKY_SECTORS: usize = 5;
/// The cold start's sectors: the departure rail, seven chosen routes, then
/// the extraction sprint.
pub const COLD_START_SECTORS: usize = 9;
/// Said the first time a ghost car comes in a run.
pub const GHOST_LINE: &str = "Ghost cars tonight: flow vehicles come in costume. Meet them as usual.";
/// The status key's line while ghost cars run.
pub const GHOST_RULE: &str = "Ghost cars: flow vehicles come in costume. Meet them as usual.";

/// One of the three lanes, or all of them at once (a blockade).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lane {
    Left,
    Center,
    Right,
    All,
}

impl Lane {
    pub fn from_index(i: usize) -> Lane {
        match i {
            0 => Lane::Left,
            1 => Lane::Center,
            _ => Lane::Right,
        }
    }

    /// The lane's number, 0 left to 2 right; `None` for all lanes.
    pub fn index(self) -> Option<usize> {
        match self {
            Lane::Left => Some(0),
            Lane::Center => Some(1),
            Lane::Right => Some(2),
            Lane::All => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lane::Left => "left",
            Lane::Center => "center",
            Lane::Right => "right",
            Lane::All => "all lanes",
        }
    }
}

/// Every kind of traffic, hazard and pickup on the rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Debris,
    Drone,
    Blockade,
    Repair,
    Salvage,
    Flow,
    FastPasser,
    HeavyBlocker,
    Priority,
    Unstable,
}

/// What the player should do with a kind, for hints and the sound check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handling {
    /// Be in its lane when it arrives.
    Join,
    /// Be out of its lane when it arrives.
    Avoid,
    /// Boost through it.
    Boost,
}

impl Kind {
    pub const ALL: [Kind; 10] = [
        Kind::Flow,
        Kind::FastPasser,
        Kind::HeavyBlocker,
        Kind::Priority,
        Kind::Unstable,
        Kind::Debris,
        Kind::Drone,
        Kind::Blockade,
        Kind::Repair,
        Kind::Salvage,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Debris => "debris",
            Kind::Drone => "drone",
            Kind::Blockade => "full blockade",
            Kind::Repair => "repair pickup",
            Kind::Salvage => "salvage",
            Kind::Flow => "flow vehicle",
            Kind::FastPasser => "fast passer",
            Kind::HeavyBlocker => "heavy blocker",
            Kind::Priority => "priority vehicle",
            Kind::Unstable => "unstable vehicle",
        }
    }

    /// The looping signature heard while it approaches.
    pub fn sound(self) -> &'static str {
        match self {
            Kind::Debris => cues::DEBRIS,
            Kind::Drone => cues::DRONE,
            Kind::Blockade => cues::BLOCKADE,
            Kind::Repair => cues::REPAIR,
            Kind::Salvage => cues::SALVAGE,
            Kind::Flow => cues::FLOW,
            Kind::FastPasser => cues::FAST,
            Kind::HeavyBlocker => cues::HEAVY,
            Kind::Priority => cues::PRIORITY,
            Kind::Unstable => cues::UNSTABLE,
        }
    }

    pub fn handling(self) -> Handling {
        match self {
            Kind::Flow | Kind::FastPasser | Kind::Priority | Kind::Repair | Kind::Salvage => {
                Handling::Join
            }
            Kind::Blockade => Handling::Boost,
            Kind::Debris | Kind::Drone | Kind::HeavyBlocker | Kind::Unstable => Handling::Avoid,
        }
    }

    /// Whether the rail gun can lock onto it.
    pub fn is_target(self, heavy_rounds: bool) -> bool {
        matches!(self, Kind::Drone | Kind::Unstable) || (heavy_rounds && self == Kind::HeavyBlocker)
    }

    /// The spoken call-out as it is announced (the original's text).
    pub fn callout(self, lane: Lane) -> String {
        let l = lane.name();
        match self {
            Kind::Debris => format!("Debris {l}."),
            Kind::Drone => format!("Drone {l}."),
            Kind::Blockade => "Full blockade. Boost just before it hits.".to_string(),
            Kind::Repair => format!("Repair pickup {l}."),
            Kind::Salvage => format!("Salvage {l}."),
            Kind::Flow => format!("Flow vehicle {l}. Route cleanly."),
            // The lane first: the window is short, so the player needs
            // where before what.
            Kind::FastPasser => format!("{}, fast passer.", sentence_start(l)),
            Kind::HeavyBlocker => format!("Heavy blocker {l}."),
            Kind::Priority => format!("Priority vehicle {l}. Do not delay it."),
            Kind::Unstable => format!("Unstable vehicle {l}. Clear the lane."),
        }
    }
}

/// One thing coming down the rail.
#[derive(Clone, Debug)]
pub struct Event {
    pub id: u32,
    pub spawn: f64,
    pub resolve: f64,
    pub kind: Kind,
    pub lane: Lane,
    pub value: i32,
    pub announced: bool,
    pub resolved: bool,
    chimed: bool,
}

impl Event {
    pub fn approach(&self) -> f64 {
        self.resolve - self.spawn
    }
}

/// The reason a run the pilot left ends with: it is not a loss.
pub const ABANDONED: &str = "Run abandoned.";

/// The endless rail's intermission row that banks the run.
pub const END_HERE: &str = "End run here";

/// The kinds of stretch a route can take. The challenge corridor and the
/// guided run use authored sectors; a Black Sky Run builds its sectors from
/// these as the player chooses them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    /// The corridor's own authored sector.
    Scripted,
    Departure,
    Standard,
    Salvage,
    Ambush,
    Storm,
    Relay,
    Extraction,
    /// The monthly Black Sky event's escort route: evacuation trains.
    Convoy,
}

impl Route {
    pub fn name(self) -> &'static str {
        match self {
            Route::Scripted => "Corridor",
            Route::Departure => "Departure rail",
            Route::Standard => "Standard rush",
            Route::Salvage => "Salvage stretch",
            Route::Ambush => "Ambush corridor",
            Route::Storm => "Storm rail",
            Route::Relay => "Data relay",
            Route::Extraction => "Extraction sprint",
            Route::Convoy => "Evacuation convoy",
        }
    }

    /// The route's saved key, for the profile's route clears.
    pub fn key(self) -> &'static str {
        match self {
            Route::Scripted => "scripted",
            Route::Departure => "departure",
            Route::Standard => "standard",
            Route::Salvage => "salvage",
            Route::Ambush => "ambush",
            Route::Storm => "storm",
            Route::Relay => "relay",
            Route::Extraction => "extraction",
            Route::Convoy => "convoy",
        }
    }

    /// What the route is like and what it pays, for the route menu.
    pub fn blurb(self) -> &'static str {
        match self {
            Route::Scripted => "The fixed corridor.",
            Route::Departure => "Calm commuter flow to learn the rail.",
            Route::Standard => "Mixed traffic and hazards. Normal pay.",
            Route::Salvage => {
                "More salvage and repairs, fewer threats. Safer, and the sector bonus is smaller."
            }
            Route::Ambush => {
                "Drones and unstable vehicles in numbers. Rail gun work. Sector bonus one and a half times."
            }
            Route::Storm => {
                "Fast traffic with short warning times. The hardest route. Sector bonus nearly double."
            }
            Route::Relay => {
                "Lighter traffic, mostly flow and priority. Recovers one intel fragment and a radio fragment."
            }
            Route::Extraction => "The final corridor. Everything at once, then a last blockade.",
            Route::Convoy => {
                "Evacuation trains run as priority traffic among hunters and debris. Sector bonus one and a quarter times."
            }
        }
    }

    /// The route's blurb in `mode`: cold start pays no intel, so its data
    /// relay recovers only the radio fragment.
    pub fn blurb_in(self, mode: Mode) -> &'static str {
        match (self, mode) {
            (Route::Relay, Mode::ColdStart) => {
                "Lighter traffic, mostly flow and priority. Recovers a radio fragment."
            }
            _ => self.blurb(),
        }
    }

    /// The blurb for a run's route menu: a tuned relay says what it pays.
    pub fn blurb_for(self, run: &Run) -> &'static str {
        if self == Route::Relay && run.loadout.relay_tuner && run.mode != Mode::ColdStart {
            "Lighter traffic, mostly flow and priority. Recovers 2 intel fragments and a radio fragment."
        } else {
            self.blurb_in(run.mode)
        }
    }

    /// The sector clear bonus is multiplied by this.
    pub fn bonus_factor(self) -> f64 {
        match self {
            Route::Salvage => 0.75,
            Route::Ambush => 1.5,
            Route::Storm => 1.75,
            Route::Extraction => 2.0,
            Route::Convoy => 1.25,
            _ => 1.0,
        }
    }

    /// The routes a player can be offered between sectors.
    pub const CHOOSABLE: [Route; 5] = [
        Route::Standard,
        Route::Salvage,
        Route::Ambush,
        Route::Relay,
        Route::Storm,
    ];
}

/// One stretch of the run, with a clear bonus at its end.
#[derive(Clone, Debug)]
pub struct Sector {
    pub start: f64,
    pub end: f64,
    pub name: String,
    pub briefing: String,
    pub route: Route,
}

/// Which kind of run this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The original two-minute, six-sector authored corridor.
    Challenge,
    /// The guided first run.
    Tutorial,
    /// Five sectors with route choices and upgrades between them.
    BlackSky,
    /// Sectors without end, until the hull or the network gives out.
    Endless,
    /// The daily challenge: one seeded sector with a twist.
    Daily,
    /// The weekly challenge: a seeded three-sector route.
    Weekly,
    /// The monthly challenge: the Black Sky event, escorting a convoy.
    Monthly,
    /// The Friday night run: a route made from the date, where salvage
    /// comes as cargo crates worth double.
    FridayNight,
    /// The roguelike run: a bare skimmer, nine sectors, one life, and
    /// nothing carried in or out but the archive and its own board.
    ColdStart,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Challenge => "Challenge corridor",
            Mode::Tutorial => "Guided run",
            Mode::BlackSky => "Black Sky Run",
            Mode::Endless => "Endless rail",
            Mode::Daily => "Daily challenge",
            Mode::Weekly => "Weekly challenge",
            Mode::Monthly => "Black Sky event",
            Mode::FridayNight => "Friday night run",
            Mode::ColdStart => "Cold start",
        }
    }

    /// The best-score board.
    pub fn board(self) -> &'static str {
        match self {
            Mode::Challenge => "challenge",
            Mode::Tutorial => "tutorial",
            Mode::BlackSky => "black sky",
            Mode::Endless => "endless",
            Mode::Daily => "daily",
            Mode::Weekly => "weekly",
            Mode::Monthly => "monthly",
            // The board key is older than the run's name.
            Mode::FridayNight => "full moon",
            Mode::ColdStart => "cold start",
        }
    }

    /// Whether the run is built sector by sector, with a clear bonus
    /// (and, while sectors remain, an intermission) at each sector's end.
    pub fn has_intermissions(self) -> bool {
        matches!(
            self,
            Mode::BlackSky
                | Mode::Endless
                | Mode::Daily
                | Mode::Weekly
                | Mode::Monthly
                | Mode::FridayNight
                | Mode::ColdStart
        )
    }

    /// The challenge period this mode plays, if it is a challenge.
    pub fn period(self) -> Option<Period> {
        match self {
            Mode::Daily => Some(Period::Daily),
            Mode::Weekly => Some(Period::Weekly),
            Mode::Monthly => Some(Period::Monthly),
            _ => None,
        }
    }

    pub fn from_period(period: Period) -> Mode {
        match period {
            Period::Daily => Mode::Daily,
            Period::Weekly => Mode::Weekly,
            Period::Monthly => Mode::Monthly,
        }
    }

    pub fn is_challenge(self) -> bool {
        self.period().is_some()
    }

    /// Whether every sector is built before the run starts, so the route
    /// is the same for every pilot: the challenges and the Friday night run.
    pub fn has_fixed_route(self) -> bool {
        self.is_challenge() || self.is_event()
    }

    /// A calendar run on a board of its own, which pays no rank, credits,
    /// parts, intel or story.
    pub fn is_event(self) -> bool {
        matches!(self, Mode::FridayNight)
    }

    /// Whether the run launches a bare skimmer: a standard loadout with no
    /// rank bonuses or workshop parts, and every route open.
    pub fn bare_skimmer(self) -> bool {
        self.has_fixed_route() || self == Mode::ColdStart
    }
}

/// A modifier a daily or weekly challenge runs under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Twist {
    /// Arrivals come closer together.
    Dense,
    /// No boost, and no blockades to need it.
    NoBoost,
    /// Every warning is shorter.
    Fog,
}

impl Twist {
    pub const ALL: [Twist; 3] = [Twist::Dense, Twist::NoBoost, Twist::Fog];

    pub fn name(self) -> &'static str {
        match self {
            Twist::Dense => "dense traffic",
            Twist::NoBoost => "boost offline",
            Twist::Fog => "fog",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Twist::Dense => "Arrivals come more than a quarter closer together, so there is less time between one decision and the next.",
            Twist::NoBoost => "Your boost is offline. There are no full blockades. Heavy blockers take their place, so every hazard must be dodged or shot.",
            Twist::Fog => "Fog cuts every warning by a quarter, so each arrival comes with less notice.",
        }
    }

    /// The share of the usual space between arrivals.
    pub fn gap_factor(self) -> f64 {
        if self == Twist::Dense {
            0.72
        } else {
            1.0
        }
    }

    /// The share of the usual warning time.
    pub fn warning_factor(self) -> f64 {
        if self == Twist::Fog {
            0.75
        } else {
            1.0
        }
    }

    /// The twist a challenge seed picks.
    pub fn from_seed(seed: u64) -> Twist {
        Twist::ALL[((seed >> 11) % 3) as usize]
    }
}

/// The network's mood, which the music and warnings follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pressure {
    Calm,
    Rising,
    High,
    Recovery,
}

impl Pressure {
    pub fn text(self) -> &'static str {
        match self {
            Pressure::Calm => "Calm flow",
            Pressure::Rising => "Rising pressure",
            Pressure::High => "High stress",
            Pressure::Recovery => "Recovery window",
        }
    }
}

/// Where the run stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Driving,
    /// A sector is clear and the next must be chosen.
    Intermission,
    Over {
        won: bool,
    },
}

/// The skimmer as it launches: rank bonuses, workshop parts and in-run
/// upgrades all land here.
#[derive(Clone, Debug, PartialEq)]
pub struct Loadout {
    pub max_hull: i32,
    pub slowdown_charges: i32,
    pub override_charges: i32,
    pub start_multiplier: i32,
    pub boost_cooldown: f64,
    pub weapon_cooldown: f64,
    /// Pickups one lane away are pulled in.
    pub magnet: bool,
    /// The share of every stress gain shed, 0 to 0.5.
    pub stress_damp: f64,
    /// Hits the shield plating will take.
    pub shields: i32,
    /// The rail gun can also clear heavy blockers.
    pub heavy_rounds: bool,
    /// Dodging pays double.
    pub near_miss: bool,
    /// Hard lock: the rail gun's lock window is half a second longer.
    pub hard_lock: bool,
    /// Scrap mesh: repairs ease more stress, salvage mends a little hull.
    pub scrap_mesh: bool,
    /// Dead man's switch: the first hull breach of the run leaves 20 hull.
    pub dead_man: bool,
    /// Hot chain: the multiplier climbs every three clean plays.
    pub hot_chain: bool,
    /// Relay tuner: a data relay recovers two intel fragments.
    pub relay_tuner: bool,
    /// Hunter jammer: drones and unstable vehicles are announced sooner.
    pub jammer: bool,
    /// Armoured nose: blockade collisions do half damage.
    pub armoured_nose: bool,
    /// Long-range sensors: repairs and salvage are announced sooner.
    pub sensors: bool,
    /// How long a boost lasts, in seconds.
    pub boost_duration: f64,
    /// Chain governor: a hit halves the multiplier instead of resetting it.
    pub governor: bool,
}

/// How much longer the Hard lock upgrade holds the lock window open.
pub const HARD_LOCK_BONUS: f64 = 0.5;
/// How much sooner the Hunter jammer announces drones and unstable vehicles.
pub const JAMMER_LEAD: f64 = 0.5;
/// The hull a Dead man's switch leaves after a breach.
pub const DEAD_MAN_HULL: i32 = 20;

impl Default for Loadout {
    /// The original's skimmer at rank 1.
    fn default() -> Self {
        Loadout {
            max_hull: 100,
            slowdown_charges: 2,
            override_charges: 2,
            start_multiplier: 1,
            boost_cooldown: BOOST_COOLDOWN,
            weapon_cooldown: WEAPON_COOLDOWN,
            magnet: false,
            stress_damp: 0.0,
            shields: 0,
            heavy_rounds: false,
            near_miss: false,
            hard_lock: false,
            scrap_mesh: false,
            dead_man: false,
            hot_chain: false,
            relay_tuner: false,
            jammer: false,
            armoured_nose: false,
            sensors: false,
            boost_duration: BOOST_DURATION,
            governor: false,
        }
    }
}

/// A one-run power-up chosen between sectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Upgrade {
    Shield,
    BoostCoils,
    HeavyRounds,
    Patch,
    PulseCell,
    OverrideCell,
    NearMiss,
    Damper,
    Magnet,
    HardLock,
    ScrapMesh,
    DeadMan,
    HotChain,
}

impl Upgrade {
    pub const ALL: [Upgrade; 9] = [
        Upgrade::Shield,
        Upgrade::BoostCoils,
        Upgrade::HeavyRounds,
        Upgrade::Patch,
        Upgrade::PulseCell,
        Upgrade::OverrideCell,
        Upgrade::NearMiss,
        Upgrade::Damper,
        Upgrade::Magnet,
    ];

    /// The upgrades a pilot earns in cold starts, which then join the pool
    /// between sectors in every run.
    pub const EARNED: [Upgrade; 4] = [
        Upgrade::HardLock,
        Upgrade::ScrapMesh,
        Upgrade::DeadMan,
        Upgrade::HotChain,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Shield => "Shield plating",
            Upgrade::BoostCoils => "Boost coils",
            Upgrade::HeavyRounds => "Heavy rounds",
            Upgrade::Patch => "Hull patch",
            Upgrade::PulseCell => "Pulse cell",
            Upgrade::OverrideCell => "Override cell",
            Upgrade::NearMiss => "Near-miss amplifier",
            Upgrade::Damper => "Stress damper",
            Upgrade::Magnet => "Pickup magnet",
            Upgrade::HardLock => "Hard lock",
            Upgrade::ScrapMesh => "Scrap mesh",
            Upgrade::DeadMan => "Dead man's switch",
            Upgrade::HotChain => "Hot chain",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Upgrade::Shield => {
                "Absorbs the next hit: no hull loss, multiplier kept, half the stress."
            }
            Upgrade::BoostCoils => "Boost recharges one and a half seconds faster.",
            Upgrade::HeavyRounds => "The rail gun can also lock onto heavy blockers.",
            Upgrade::Patch => "Restores 30 hull now.",
            Upgrade::PulseCell => "One more slowdown pulse charge.",
            Upgrade::OverrideCell => "One more priority override charge.",
            Upgrade::NearMiss => "Dodging hazards pays double.",
            Upgrade::Damper => "Every stress gain is a quarter smaller.",
            Upgrade::Magnet => "Pickups one lane away are pulled in.",
            Upgrade::HardLock => "The rail gun locks half a second sooner.",
            Upgrade::ScrapMesh => "Repairs also ease 10 stress. Salvage also restores 2 hull.",
            Upgrade::DeadMan => "Once this run, a hull breach leaves you at 20 hull.",
            Upgrade::HotChain => "The multiplier climbs every 3 clean plays, not 4.",
        }
    }

    /// What it does, short, for a debrief line.
    pub fn gist(self) -> &'static str {
        match self {
            Upgrade::HardLock => "the rail gun locks sooner",
            Upgrade::ScrapMesh => "pickups restore hull and reduce stress",
            Upgrade::DeadMan => "survive one hull breach",
            Upgrade::HotChain => "the multiplier climbs faster",
            _ => "",
        }
    }

    /// Whether taking it again would do nothing.
    fn is_spent(self, l: &Loadout) -> bool {
        match self {
            Upgrade::HeavyRounds => l.heavy_rounds,
            Upgrade::NearMiss => l.near_miss,
            Upgrade::Magnet => l.magnet,
            Upgrade::Damper => l.stress_damp >= 0.5,
            Upgrade::BoostCoils => l.boost_cooldown <= 2.0,
            Upgrade::HardLock => l.hard_lock,
            Upgrade::ScrapMesh => l.scrap_mesh,
            Upgrade::DeadMan => l.dead_man,
            Upgrade::HotChain => l.hot_chain,
            _ => false,
        }
    }
}

/// What the run screen should play or say. The rules never touch a device.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// A cue heard from the skimmer itself.
    Cue(&'static str),
    /// Speech that cuts in.
    Say(String),
    /// Speech that waits its turn.
    SayQueued(String),
    /// An event began its approach: start its sound.
    Spawn(u32),
    /// An event arrived or was shot: stop its sound.
    Despawn(u32),
    /// A cue at an event's lane, as it arrives.
    LaneCue(&'static str, Lane),
    /// A story log was recovered mid-run (the stage reached, and why).
    Story(usize, &'static str),
    /// The guided run reached this tutorial step.
    Tutorial(u8),
}

/// What the run counts for the debrief and lifetime unlocks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub kills: u32,
    pub pickups: u32,
    pub repairs: u32,
    pub hits: u32,
    pub dodges: u32,
    pub threaded: u32,
    pub priority_cleared: u32,
    pub intel_found: u32,
    pub best_multiplier: i32,
}

/// One run in progress.
pub struct Run {
    pub mode: Mode,
    rng: Rng,
    pub loadout: Loadout,
    pub t: f64,
    pub sectors: Vec<Sector>,
    pub events: Vec<Event>,
    next_id: u32,
    /// The player's lane, 0 left to 2 right.
    pub lane: usize,
    pub hull: i32,
    pub score: i64,
    pub multiplier: i32,
    clean_chain: i32,
    pub current_sector: Option<usize>,
    pub sectors_cleared: usize,
    pub stress: i32,
    pub pressure: Pressure,
    announced_pressure: Option<Pressure>,
    recovery_mark: i32,
    pub slowdown_charges: i32,
    pub override_charges: i32,
    pub congestion: i32,
    last_stress_warning: f64,
    boost_end: f64,
    last_boost: f64,
    boost_ready_cued: bool,
    last_weapon: f64,
    last_slowdown: f64,
    slowdown_end: f64,
    override_end: f64,
    last_distance: f64,
    pub phase: Phase,
    pub reason: String,
    tutorial_step: u8,
    pub story_stage: usize,
    /// The story stage the run began at.
    pub story_start: usize,
    pub upgrades: Vec<Upgrade>,
    /// Upgrades the pilot has earned, added to the pool between sectors.
    pub earned: Vec<Upgrade>,
    /// Whether the Dead man's switch has already saved this run.
    dead_man_used: bool,
    pub stats: Stats,
    /// Speak each arrival as it is announced.
    pub callouts: bool,
    /// The last thing said, for the repeat key.
    pub last_prompt: String,
    /// A challenge's modifier.
    pub twist: Option<Twist>,
    /// The Black Sky event's rule: priority traffic is evacuation trains,
    /// paying triple and costing double stress when delayed.
    pub convoy: bool,
    /// Added to each sector's number to set how hard it is built.
    pub depth: usize,
    /// A challenge's rule, spoken by F1 and the status key.
    pub event_rule: String,
    /// A fixed length for every sector built, in seconds, instead of the
    /// length that grows with each sector.
    pub sector_length: Option<f64>,
    /// Halloween's ghost cars: flow vehicles wear a hollow whistle over
    /// their pulse. Sound only: they play exactly as flow vehicles.
    pub ghost_cars: bool,
    /// Whether the ghost cars have been named in words this run.
    pub ghost_named: bool,
    /// Whether a fast passer's call-out has said what to do this run.
    fast_named: bool,
    /// A Friday night run's rule: salvage comes as cargo crates, worth double.
    pub cargo: bool,
    notes: Vec<Note>,
}

impl Run {
    fn blank(mode: Mode, loadout: Loadout, story_stage: usize, rng: Rng) -> Run {
        Run {
            mode,
            rng,
            hull: loadout.max_hull,
            multiplier: loadout.start_multiplier,
            slowdown_charges: loadout.slowdown_charges,
            override_charges: loadout.override_charges,
            last_boost: -loadout.boost_cooldown,
            last_weapon: -loadout.weapon_cooldown,
            loadout,
            t: 0.0,
            sectors: Vec::new(),
            events: Vec::new(),
            next_id: 0,
            lane: 1,
            score: 0,
            clean_chain: 0,
            current_sector: None,
            sectors_cleared: 0,
            stress: 0,
            pressure: Pressure::Calm,
            announced_pressure: None,
            recovery_mark: 0,
            congestion: 0,
            last_stress_warning: -WARNING_REPEAT,
            boost_end: -1.0,
            boost_ready_cued: true,
            last_slowdown: -SLOWDOWN_COOLDOWN,
            slowdown_end: -1.0,
            override_end: -1.0,
            last_distance: 0.0,
            phase: Phase::Driving,
            reason: String::new(),
            tutorial_step: 0,
            story_stage,
            story_start: story_stage,
            upgrades: Vec::new(),
            earned: Vec::new(),
            dead_man_used: false,
            stats: Stats::default(),
            callouts: true,
            last_prompt: String::new(),
            twist: None,
            convoy: false,
            depth: 0,
            event_rule: String::new(),
            sector_length: None,
            ghost_cars: false,
            ghost_named: false,
            fast_named: false,
            cargo: false,
            notes: Vec::new(),
        }
    }

    /// The loop heard while `kind` approaches: its own, or its costume
    /// on an event night.
    pub fn loop_key(&self, kind: Kind) -> &'static str {
        match kind {
            Kind::Flow if self.ghost_cars => cues::GHOST_FLOW,
            Kind::Salvage if self.cargo => cues::CARGO_CRATE,
            k => k.sound(),
        }
    }

    /// A kind's name in this run, for the hint: "cargo crate" for salvage
    /// on a Friday night run.
    pub fn kind_name(&self, kind: Kind) -> &'static str {
        match kind {
            Kind::Salvage if self.cargo => "cargo crate",
            Kind::Flow if self.ghost_cars => "ghost car",
            k => k.name(),
        }
    }

    /// The original two-minute corridor.
    pub fn challenge(loadout: Loadout, story_stage: usize) -> Run {
        let mut run = Run::blank(Mode::Challenge, loadout, story_stage, Rng::seeded(0));
        script::challenge(&mut run);
        run
    }

    /// An empty two-minute corridor with one sector, for building a run
    /// event by event (tests, practice).
    pub fn sandbox(loadout: Loadout) -> Run {
        let mut run = Run::blank(Mode::Challenge, loadout, 5, Rng::seeded(0));
        run.push_sector(0.0, CHALLENGE_LENGTH, "Sandbox", "Practice rail.");
        run
    }

    /// The guided first run.
    pub fn tutorial(loadout: Loadout, story_stage: usize) -> Run {
        let mut run = Run::blank(Mode::Tutorial, loadout, story_stage, Rng::seeded(0));
        script::tutorial(&mut run);
        run
    }

    /// A Black Sky Run or an endless rail, its first sector built.
    pub fn roguelite(mode: Mode, loadout: Loadout, story_stage: usize, rng: Rng) -> Run {
        let mut run = Run::blank(mode, loadout, story_stage, rng);
        run.add_sector(Route::Departure);
        run
    }

    /// A challenge run: every sector along `routes` built up front, so the
    /// whole route is the same for every player whatever upgrades they pick.
    #[allow(clippy::too_many_arguments)]
    pub fn challenge_event(
        mode: Mode,
        loadout: Loadout,
        story_stage: usize,
        rng: Rng,
        routes: &[Route],
        depth: usize,
        twist: Option<Twist>,
        convoy: bool,
    ) -> Run {
        let mut run = Run::blank(mode, loadout, story_stage, rng);
        run.twist = twist;
        run.convoy = convoy;
        run.depth = depth;
        run.build_route(routes);
        if convoy {
            // The city is already on edge when the event begins.
            run.stress = 30;
        }
        run
    }

    // -- building --------------------------------------------------------------

    /// Builds a sector along each of `routes`, back to back after any
    /// already built.
    pub fn build_route(&mut self, routes: &[Route]) {
        let mut start = self.sectors.last().map_or(0.0, |s| s.end);
        for &route in routes {
            self.add_sector_at(route, start);
            start = self.sectors.last().map_or(start, |s| s.end);
        }
    }

    pub fn push_sector(&mut self, start: f64, end: f64, name: &str, briefing: &str) {
        self.sectors.push(Sector {
            start,
            end,
            name: name.to_string(),
            briefing: briefing.to_string(),
            route: Route::Scripted,
        });
    }

    pub fn push_event(&mut self, spawn: f64, kind: Kind, lane: Lane, value: i32, approach: f64) {
        let resolve = spawn + approach;
        // The jammer hears hunters coming: announced sooner, arriving on time.
        let early = (self.loadout.jammer && matches!(kind, Kind::Drone | Kind::Unstable))
            || (self.loadout.sensors && matches!(kind, Kind::Repair | Kind::Salvage));
        let spawn = if early {
            (spawn - JAMMER_LEAD).max(self.t)
        } else {
            spawn
        };
        self.events.push(Event {
            id: self.next_id,
            spawn,
            resolve,
            kind,
            lane,
            value,
            announced: false,
            resolved: false,
            chimed: false,
        });
        self.next_id += 1;
    }

    pub(crate) fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    /// Builds the next sector along `route`, starting now.
    pub fn add_sector(&mut self, route: Route) {
        let start = self.t;
        self.add_sector_at(route, start);
    }

    /// Builds the next sector along `route`, starting at `start`.
    pub fn add_sector_at(&mut self, route: Route, start: f64) {
        let index = self.sectors.len();
        let (end, name, briefing) = script::generate(self, index, start, route);
        self.sectors.push(Sector {
            start,
            end,
            name,
            briefing,
            route,
        });
    }

    /// How many sectors the run holds when it is done; `None` for endless.
    pub fn total_sectors(&self) -> Option<usize> {
        match self.mode {
            Mode::Challenge | Mode::Tutorial => Some(self.sectors.len()),
            Mode::BlackSky => Some(BLACK_SKY_SECTORS),
            Mode::ColdStart => Some(COLD_START_SECTORS),
            Mode::Endless => None,
            Mode::Daily | Mode::Weekly | Mode::Monthly | Mode::FridayNight => {
                Some(self.sectors.len())
            }
        }
    }

    /// When the run is won, if it has a fixed length.
    pub fn length(&self) -> Option<f64> {
        match self.mode {
            Mode::Challenge => Some(CHALLENGE_LENGTH),
            Mode::Tutorial => Some(TUTORIAL_LENGTH),
            _ => None,
        }
    }

    // -- notes -------------------------------------------------------------------

    /// What happened since the last call.
    pub fn drain_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    fn cue(&mut self, key: &'static str) {
        self.notes.push(Note::Cue(key));
    }

    fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last_prompt = text.clone();
        self.notes.push(Note::Say(text));
    }

    fn say_queued(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last_prompt = text.clone();
        self.notes.push(Note::SayQueued(text));
    }

    // -- the clock -------------------------------------------------------------

    pub fn boost_active(&self) -> bool {
        self.t <= self.boost_end
    }

    pub fn slowdown_active(&self) -> bool {
        self.t <= self.slowdown_end
    }

    pub fn override_active(&self) -> bool {
        self.t <= self.override_end
    }

    pub fn boost_ready_in(&self) -> f64 {
        (self.last_boost + self.loadout.boost_cooldown - self.t).max(0.0)
    }

    pub fn weapon_ready_in(&self) -> f64 {
        (self.last_weapon + self.loadout.weapon_cooldown - self.t).max(0.0)
    }

    pub fn slowdown_ready_in(&self) -> f64 {
        (self.last_slowdown + SLOWDOWN_COOLDOWN - self.t).max(0.0)
    }

    /// The sector the clock is in (the last one once past them all).
    pub fn sector_index_at(&self, t: f64) -> Option<usize> {
        if self.sectors.is_empty() {
            return None;
        }
        Some(
            self.sectors
                .iter()
                .position(|s| t >= s.start && t < s.end)
                .unwrap_or(self.sectors.len() - 1),
        )
    }

    pub fn sector(&self) -> Option<&Sector> {
        self.current_sector.and_then(|i| self.sectors.get(i))
    }

    /// One frame of the run.
    pub fn update(&mut self, dt: f64) {
        if self.phase != Phase::Driving {
            return;
        }
        self.t += dt.max(0.0);
        self.update_sectors();
        if self.phase != Phase::Driving {
            return;
        }
        self.update_distance();
        self.update_events();
        self.update_pressure();
        self.warn_stress();
        self.tutorial_prompts();
        self.story_milestones();
        if !self.boost_ready_cued && self.boost_ready_in() <= 0.0 {
            self.boost_ready_cued = true;
            self.cue(cues::BOOST_READY);
        }

        if self.hull <= 0 {
            self.finish(false, "Hull breach.");
        } else if self.stress >= MAX_STRESS {
            self.finish(false, "Network overload.");
        } else if let Some(length) = self.length() {
            if self.t >= length {
                let reason = if self.mode == Mode::Tutorial {
                    "Tutorial extraction complete."
                } else {
                    "Extraction complete."
                };
                self.finish(true, reason);
            }
        }
    }

    /// Sector clear bonuses, then the next sector's briefing. (The original
    /// spoke the briefing first and let the bonus cut it off.)
    fn update_sectors(&mut self) {
        self.clear_sectors();
        if self.phase != Phase::Driving {
            return;
        }
        if let Some(i) = self.sector_index_at(self.t) {
            if self.current_sector != Some(i) && i >= self.sectors_cleared {
                self.current_sector = Some(i);
                let s = &self.sectors[i];
                let text = format!("{}. {}", s.name, s.briefing);
                self.cue(cues::SECTOR_START);
                self.say_queued(text);
            }
        }
    }

    fn clear_sectors(&mut self) {
        while self.sectors_cleared < self.sectors.len()
            && self.t >= self.sectors[self.sectors_cleared].end
        {
            let route = self.sectors[self.sectors_cleared].route;
            let base = (100 + self.sectors_cleared as i64 * 25) * self.multiplier as i64;
            self.score += (base as f64 * route.bonus_factor()).round() as i64;
            self.reduce_stress(8);
            self.congestion = 0;
            if self.multiplier < 5 {
                self.multiplier += 1;
                self.stats.best_multiplier = self.stats.best_multiplier.max(self.multiplier);
            }
            self.clean_chain = 0;
            self.sectors_cleared += 1;
            if route == Route::Relay {
                self.stats.intel_found += self.relay_intel();
            }
            let more = match self.total_sectors() {
                Some(total) => self.sectors_cleared < total,
                None => true,
            };
            if more {
                self.cue(cues::SECTOR_CLEAR);
                let mut text = format!(
                    "Sector clear bonus. Stress easing. Multiplier {}.",
                    self.multiplier
                );
                if route == Route::Relay && self.mode != Mode::ColdStart {
                    if self.relay_intel() == 1 {
                        text.push_str(" Intel fragment recovered from the relay.");
                    } else {
                        text.push_str(" 2 intel fragments recovered from the relay.");
                    }
                }
                self.say(text);
            }
            if self.mode.has_intermissions() {
                if more {
                    self.phase = Phase::Intermission;
                } else {
                    self.finish(true, "Extraction complete.");
                }
                return;
            }
        }
    }

    fn update_distance(&mut self) {
        while self.t - self.last_distance >= 1.0 {
            self.last_distance += 1.0;
            self.score += 8 * self.multiplier as i64;
            if self.pressure == Pressure::Calm {
                self.reduce_stress(1);
            }
        }
    }

    fn update_events(&mut self) {
        for i in 0..self.events.len() {
            let e = &self.events[i];
            if !e.announced && self.t >= e.spawn {
                self.events[i].announced = true;
                self.announce(i);
            }
            let e = &self.events[i];
            if e.announced
                && !e.resolved
                && !e.chimed
                && e.kind == Kind::Blockade
                && self.t >= e.resolve - BOOST_CHIME_LEAD
            {
                self.events[i].chimed = true;
                self.cue(cues::BOOST_CHIME);
            }
            if !self.events[i].resolved && self.t >= self.events[i].resolve {
                self.resolve(i);
            }
        }
    }

    fn announce(&mut self, i: usize) {
        let (id, kind, lane) = (self.events[i].id, self.events[i].kind, self.events[i].lane);
        self.notes.push(Note::Spawn(id));
        match kind {
            Kind::Priority => self.cue(cues::PRIORITY_NOTICE),
            Kind::HeavyBlocker | Kind::Unstable => self.cue(cues::STRESS_WARNING),
            _ => {}
        }
        if self.callouts {
            if self.convoy && kind == Kind::Priority {
                self.say(format!(
                    "Evacuation train {}. Do not delay it.",
                    lane.name()
                ));
            } else if self.cargo && kind == Kind::Salvage {
                self.say(format!("Cargo crate {}.", lane.name()));
            } else if kind == Kind::FastPasser && !self.fast_named {
                // The first one says both ways out; the window is short,
                // so the lane still comes first.
                self.fast_named = true;
                self.say(format!(
                    "{}, fast passer: join it or pulse.",
                    sentence_start(lane.name())
                ));
            } else {
                self.say(kind.callout(lane));
            }
        }
        if self.ghost_cars && kind == Kind::Flow && !self.ghost_named {
            self.ghost_named = true;
            self.say_queued(GHOST_LINE);
        }
    }

    fn resolve(&mut self, i: usize) {
        self.events[i].resolved = true;
        let e = self.events[i].clone();
        self.notes.push(Note::Despawn(e.id));
        let in_lane = e.lane.index() == Some(self.lane);
        let near = e
            .lane
            .index()
            .is_some_and(|l| l.abs_diff(self.lane) <= 1 && self.loadout.magnet);
        let boosted = self.boost_active();
        let slowed = self.slowdown_active();
        let held = self.override_active();
        let dodge = if self.loadout.near_miss { 30 } else { 15 };
        match e.kind {
            Kind::Debris => {
                if in_lane {
                    if boosted {
                        self.smash(35, "Boost smash.", e.lane);
                    } else {
                        self.damage(14, "Debris impact.", 10, true, e.lane);
                    }
                } else {
                    self.dodged(dodge, "", e.lane);
                }
            }
            Kind::Drone => {
                if in_lane {
                    if boosted {
                        self.smash(55, "Boost ram.", e.lane);
                    } else {
                        self.damage(12, "Drone hit.", 9, true, e.lane);
                    }
                } else {
                    self.dodged(dodge, "", e.lane);
                }
            }
            Kind::Blockade => {
                if boosted {
                    self.smash(70, "Blockade breached.", e.lane);
                } else {
                    let hit = if self.loadout.armoured_nose { 9 } else { 18 };
                    self.damage(hit, "Blockade collision.", 14, true, e.lane);
                }
            }
            Kind::Repair => {
                if in_lane || near {
                    self.hull = (self.hull + e.value).clamp(0, self.loadout.max_hull);
                    self.score += 30 * self.multiplier as i64;
                    self.clean_chain += 1;
                    self.stats.pickups += 1;
                    self.stats.repairs += 1;
                    self.reduce_stress(if self.loadout.scrap_mesh { 22 } else { 12 });
                    self.cue(cues::REPAIR_GET);
                    self.say(format!("Repair secured. Hull {}.", self.hull));
                    self.check_multiplier();
                } else {
                    self.say_queued("Repair pickup missed.");
                }
            }
            Kind::Salvage => {
                let moon = self.cargo;
                if in_lane || near {
                    let worth = if moon { 2 } else { 1 };
                    self.score += worth * e.value as i64 * self.multiplier as i64;
                    if self.loadout.scrap_mesh {
                        self.hull = (self.hull + 2).min(self.loadout.max_hull);
                    }
                    self.clean_chain += 1;
                    self.stats.pickups += 1;
                    self.reduce_stress(4);
                    self.cue(cues::SALVAGE_GET);
                    if moon {
                        self.say(format!(
                            "Cargo crate secured, double. Score {}.",
                            self.score
                        ));
                    } else {
                        self.say(format!("Salvage secured. Score {}.", self.score));
                    }
                    self.check_multiplier();
                } else if moon {
                    self.say_queued("Cargo crate missed.");
                } else {
                    self.say_queued("Salvage missed.");
                }
            }
            Kind::Flow => {
                if in_lane {
                    self.stats.threaded += 1;
                    self.clean(18 + e.value, "Flow vehicle threaded.", e.lane);
                    self.reduce_stress(5);
                } else {
                    self.notes.push(Note::LaneCue(cues::MISSED, e.lane));
                    self.add_stress(5, "Flow lane missed.");
                    self.say_queued("Flow lane missed.");
                }
            }
            Kind::FastPasser => {
                if in_lane || slowed {
                    self.stats.threaded += 1;
                    let text = if slowed {
                        "Fast passer stabilised."
                    } else {
                        "Fast passer cleared."
                    };
                    self.clean(28 + e.value, text, e.lane);
                    self.reduce_stress(6);
                } else {
                    self.damage(8, "Fast passer clipped the corridor.", 11, true, e.lane);
                }
            }
            Kind::HeavyBlocker => {
                if in_lane && boosted {
                    self.smash(75, "Heavy blocker forced open.", e.lane);
                    self.reduce_stress(10);
                } else if !in_lane || slowed {
                    let text = if slowed {
                        "Heavy blocker slowed and bypassed."
                    } else {
                        "Heavy blocker bypassed."
                    };
                    let points = if self.loadout.near_miss { 40 } else { 20 };
                    self.dodged(points, text, e.lane);
                    self.reduce_stress(4);
                } else {
                    self.damage(16, "Heavy blocker jammed the corridor.", 18, true, e.lane);
                }
            }
            Kind::Priority => {
                if in_lane || held || slowed {
                    self.stats.priority_cleared += 1;
                    let text = match (held, self.convoy) {
                        (true, false) => "Priority override held the route.",
                        (false, false) => "Priority traffic cleared.",
                        (true, true) => "Override held the route. Evacuation train through.",
                        (false, true) => "Evacuation train through.",
                    };
                    if self.convoy {
                        self.notes.push(Note::LaneCue(cues::CONVOY, e.lane));
                        self.clean(150, text, e.lane);
                    } else {
                        self.clean(50, text, e.lane);
                    }
                    self.reduce_stress(14);
                } else if self.convoy {
                    self.damage(6, "Evacuation train delayed.", 40, false, e.lane);
                } else {
                    self.damage(6, "Priority traffic delayed.", 20, false, e.lane);
                }
            }
            Kind::Unstable => {
                if in_lane {
                    if boosted {
                        self.smash(60, "Unstable vehicle punched through.", e.lane);
                    } else {
                        self.damage(
                            14,
                            "Unstable vehicle burst across the lane.",
                            16,
                            true,
                            e.lane,
                        );
                    }
                } else {
                    let points = if self.loadout.near_miss { 36 } else { 18 };
                    self.dodged(points, "Unstable lane avoided.", e.lane);
                    self.add_stress(4, "Turbulence spread.");
                }
            }
        }
    }

    // -- scoring -----------------------------------------------------------------

    /// The original `reward_clean_play`.
    fn reward(&mut self, base: i32, message: &str) {
        self.score += base as i64 * self.multiplier as i64;
        self.clean_chain += 1;
        if self.congestion > 0 {
            self.congestion -= 1;
        }
        self.check_multiplier();
        if !message.is_empty() {
            self.say_queued(message);
        }
    }

    fn clean(&mut self, base: i32, message: &str, lane: Lane) {
        self.notes.push(Note::LaneCue(cues::CLEAN, lane));
        self.reward(base, message);
    }

    fn dodged(&mut self, base: i32, message: &str, lane: Lane) {
        self.stats.dodges += 1;
        self.notes.push(Note::LaneCue(cues::PASS, lane));
        self.reward(base, message);
    }

    fn smash(&mut self, base: i32, message: &str, lane: Lane) {
        self.notes.push(Note::LaneCue(cues::SMASH, lane));
        self.reward(base, message);
    }

    fn check_multiplier(&mut self) {
        let chain = if self.loadout.hot_chain { 3 } else { 4 };
        if self.clean_chain >= chain {
            self.clean_chain = 0;
            if self.multiplier < 5 {
                self.multiplier += 1;
                self.stats.best_multiplier = self.stats.best_multiplier.max(self.multiplier);
                self.cue(cues::MULTIPLIER);
                self.say_queued(format!("Multiplier {}.", self.multiplier));
            }
        }
    }

    fn damage(&mut self, amount: i32, message: &str, stress: i32, congest: bool, lane: Lane) {
        if self.loadout.shields > 0 {
            self.loadout.shields -= 1;
            self.cue(cues::SHIELD);
            self.add_stress(stress / 2, message);
            let left = self.loadout.shields;
            self.say(format!(
                "{message} Shield absorbed it. {left} shield{} left.",
                if left == 1 { "" } else { "s" }
            ));
            return;
        }
        self.stats.hits += 1;
        self.notes.push(Note::LaneCue(cues::IMPACT, lane));
        self.hull = (self.hull - amount).clamp(0, self.loadout.max_hull);
        self.multiplier = if self.loadout.governor {
            (self.multiplier / 2).max(1)
        } else {
            1
        };
        self.clean_chain = 0;
        self.add_stress(stress, message);
        if congest {
            self.congestion += 1;
        }
        if self.hull <= 0 && self.loadout.dead_man && !self.dead_man_used {
            self.dead_man_used = true;
            self.hull = DEAD_MAN_HULL;
            self.cue(cues::SHIELD);
            self.say(format!("{message} Dead man's switch. Hull {}.", self.hull));
            return;
        }
        self.say(format!("{message} Hull {}.", self.hull));
    }

    pub fn add_stress(&mut self, amount: i32, reason: &str) {
        let mut scaled = amount;
        if self.congestion > 0 {
            scaled += self.congestion * 2;
        }
        if self.loadout.stress_damp > 0.0 {
            scaled = (scaled as f64 * (1.0 - self.loadout.stress_damp)).round() as i32;
        }
        self.stress = (self.stress + scaled).clamp(0, MAX_STRESS);
        self.update_pressure();
        if !reason.is_empty() && scaled >= 14 {
            self.cue(cues::STRESS_WARNING);
        }
    }

    pub fn reduce_stress(&mut self, amount: i32) {
        self.stress = (self.stress - amount).clamp(0, MAX_STRESS);
        self.update_pressure();
        if self.stress <= 25 && self.congestion > 0 {
            self.congestion -= 1;
        }
    }

    /// The pressure states of the challenge build. One fix: in the original
    /// a drop out of high stress was overwritten as "rising" a frame later;
    /// here it is a recovery window until stress climbs again.
    fn update_pressure(&mut self) {
        let s = self.stress;
        let next = if s >= 75 {
            Pressure::High
        } else {
            match self.pressure {
                Pressure::High => {
                    self.recovery_mark = s;
                    Pressure::Recovery
                }
                Pressure::Rising if s < 35 => {
                    self.recovery_mark = s;
                    Pressure::Recovery
                }
                Pressure::Rising => Pressure::Rising,
                Pressure::Recovery if s <= 20 => Pressure::Calm,
                Pressure::Recovery if s >= 35 && s > self.recovery_mark + 10 => Pressure::Rising,
                Pressure::Recovery => {
                    self.recovery_mark = self.recovery_mark.min(s);
                    Pressure::Recovery
                }
                Pressure::Calm if s >= 35 => Pressure::Rising,
                Pressure::Calm => Pressure::Calm,
            }
        };
        self.pressure = next;
        if self.announced_pressure != Some(next) {
            self.announced_pressure = Some(next);
            match next {
                Pressure::Rising => {
                    self.cue(cues::STRESS_WARNING);
                    self.say_queued("Pressure rising.");
                }
                Pressure::High => {
                    self.cue(cues::OVERLOAD);
                    self.say("High stress. Clear the corridor now.");
                }
                Pressure::Recovery => {
                    self.cue(cues::RECOVERY);
                    self.say_queued("Recovery window.");
                }
                Pressure::Calm => {}
            }
        }
    }

    pub fn is_high_stress(&self) -> bool {
        self.pressure == Pressure::High || self.stress >= 75
    }

    fn warn_stress(&mut self) {
        if !self.is_high_stress() || self.t - self.last_stress_warning < WARNING_REPEAT {
            return;
        }
        self.last_stress_warning = self.t;
        self.cue(cues::OVERLOAD);
        self.say_queued(format!("Overload risk. Stress {}.", self.stress));
    }

    fn tutorial_prompts(&mut self) {
        if self.mode != Mode::Tutorial {
            return;
        }
        let at = [0.8, 4.3, 10.3, 17.0, 24.0, 31.0, 36.6, 40.0, 48.5, 53.5];
        let step = self.tutorial_step as usize;
        if step < at.len() && self.t >= at[step] {
            self.tutorial_step += 1;
            self.notes.push(Note::Tutorial(self.tutorial_step));
        }
    }

    fn story_milestones(&mut self) {
        if self.story_stage < 2 && self.t >= 40.0 {
            self.story_stage = 2;
            self.notes
                .push(Note::Story(2, "Mid-run data relay recovered."));
        }
        if self.story_stage < 3 && self.sectors_cleared >= 3 {
            self.story_stage = 3;
            self.notes
                .push(Note::Story(3, "Priority telemetry fragment recovered."));
        }
    }

    fn finish(&mut self, won: bool, reason: &str) {
        if matches!(self.phase, Phase::Over { .. }) {
            return;
        }
        if won {
            self.score += 500 * self.multiplier as i64;
        }
        self.phase = Phase::Over { won };
        self.reason = reason.to_string();
        for e in &mut self.events {
            if e.announced && !e.resolved {
                e.resolved = true;
                self.notes.push(Note::Despawn(e.id));
            }
        }
    }

    /// Ends an endless run at an intermission: a finished run, debriefed
    /// and paid, with no extraction bonus. The sector reached is recorded.
    pub fn end_here(&mut self) {
        if self.mode != Mode::Endless || self.phase != Phase::Intermission {
            return;
        }
        self.phase = Phase::Over { won: true };
        self.reason = format!("Run ended after sector {}.", self.sectors_cleared);
    }

    /// Ends the run where it stands, as when the player gives up.
    pub fn abandon(&mut self) {
        self.finish(false, ABANDONED);
    }

    // -- the player's controls ----------------------------------------------------

    /// Shift one lane left (-1) or right (+1).
    pub fn shift(&mut self, dir: i32) {
        if self.phase != Phase::Driving {
            return;
        }
        let next = self.lane as i32 + dir;
        if !(0..=2).contains(&next) {
            self.cue(cues::EDGE);
            self.say("Rail edge.");
            return;
        }
        self.lane = next as usize;
        self.cue(cues::SHIFT);
        self.say(format!(
            "{} lane.",
            sentence(Lane::from_index(self.lane).name())
        ));
    }

    pub fn boost(&mut self) {
        if self.phase != Phase::Driving {
            return;
        }
        if self.twist == Some(Twist::NoBoost) {
            self.cue(cues::DENIED);
            self.say("Boost offline for this challenge.");
            return;
        }
        let ready_in = self.boost_ready_in();
        if ready_in > 0.0 {
            self.cue(cues::DENIED);
            self.say(format!(
                "Boost recharging. {} remaining.",
                seconds(ready_in)
            ));
            return;
        }
        self.last_boost = self.t;
        self.boost_end = self.t + self.loadout.boost_duration;
        self.boost_ready_cued = false;
        self.cue(cues::BOOST);
        self.say("Boost engaged.");
    }

    pub fn fire(&mut self) {
        if self.phase != Phase::Driving {
            return;
        }
        let ready_in = self.weapon_ready_in();
        if ready_in > 0.0 {
            self.cue(cues::DENIED);
            self.say(format!(
                "Rail gun cooling. {} remaining.",
                seconds(ready_in)
            ));
            return;
        }
        self.last_weapon = self.t;
        let Some(i) = self.weapon_target() else {
            self.cue(cues::NO_LOCK);
            if self.target_out_of_range() {
                self.say("No lock. Fire as it closes in.");
            } else {
                self.say("No lock.");
            }
            return;
        };
        self.cue(cues::RAILGUN);
        self.events[i].resolved = true;
        let (id, lane) = (self.events[i].id, self.events[i].lane);
        self.notes.push(Note::Despawn(id));
        self.notes.push(Note::LaneCue(cues::RAILGUN_HIT, lane));
        self.stats.kills += 1;
        self.reduce_stress(6);
        self.reward(80, &format!("Threat cleared {}.", lane.name()));
    }

    /// The original lock-on: the nearest lane first, then the soonest,
    /// unstable vehicles a little ahead of drones, and only inside the lock
    /// window ([`WEAPON_RANGE`]).
    pub fn weapon_target(&self) -> Option<usize> {
        let range = self.lock_range();
        let mut best: Option<(usize, f64)> = None;
        for (i, e) in self.events.iter().enumerate() {
            if !self.is_live_target(e) {
                continue;
            }
            let priority = self.lock_priority(e);
            if priority >= range * 1000.0 {
                continue;
            }
            if best.is_none_or(|(_, p)| priority < p) {
                best = Some((i, priority));
            }
        }
        best.map(|(i, _)| i)
    }

    /// The lock window, in seconds: longer with Hard lock.
    pub fn lock_range(&self) -> f64 {
        if self.loadout.hard_lock {
            WEAPON_RANGE + HARD_LOCK_BONUS
        } else {
            WEAPON_RANGE
        }
    }

    /// Intel fragments a data relay recovers: two with the Relay tuner.
    fn relay_intel(&self) -> u32 {
        if self.loadout.relay_tuner {
            2
        } else {
            1
        }
    }

    fn is_live_target(&self, e: &Event) -> bool {
        !e.resolved
            && e.announced
            && e.kind.is_target(self.loadout.heavy_rounds)
            && self.t <= e.resolve + 0.15
    }

    /// The original's lock score, in milliseconds: lower locks first.
    fn lock_priority(&self, e: &Event) -> f64 {
        let distance = e.lane.index().map_or(0, |l| l.abs_diff(self.lane)) as f64;
        let mut priority = distance * 100.0 + (e.resolve - self.t) * 1000.0;
        if e.kind == Kind::Unstable {
            priority -= 25.0;
        }
        priority
    }

    /// Whether something could be shot, but is still too far off to lock.
    pub fn target_out_of_range(&self) -> bool {
        self.events.iter().any(|e| self.is_live_target(e))
    }

    pub fn slowdown(&mut self) {
        if self.phase != Phase::Driving {
            return;
        }
        if self.slowdown_charges <= 0 {
            self.cue(cues::DENIED);
            self.say("No slowdown pulse charges left.");
            return;
        }
        let ready_in = self.slowdown_ready_in();
        if ready_in > 0.0 {
            self.cue(cues::DENIED);
            self.say(format!(
                "Slowdown pulse recharging. {} remaining.",
                seconds(ready_in)
            ));
            return;
        }
        self.slowdown_charges -= 1;
        self.last_slowdown = self.t;
        self.slowdown_end = self.t + SLOWDOWN_DURATION;
        self.reduce_stress(10);
        self.cue(cues::SLOWDOWN);
        self.say(format!(
            "Slowdown pulse active. Slowdown charges {}.",
            self.slowdown_charges
        ));
    }

    pub fn priority_override(&mut self) {
        if self.phase != Phase::Driving {
            return;
        }
        if self.override_charges <= 0 {
            self.cue(cues::DENIED);
            self.say("No priority override charges left.");
            return;
        }
        if self.override_active() {
            self.cue(cues::DENIED);
            self.say("Priority override already active.");
            return;
        }
        self.override_charges -= 1;
        self.override_end = self.t + OVERRIDE_DURATION;
        self.reduce_stress(8);
        self.cue(cues::OVERRIDE);
        self.say(format!(
            "Priority override active. Charges {}.",
            self.override_charges
        ));
    }

    // -- between sectors ---------------------------------------------------------

    /// Up to three upgrades to choose from, none that would do nothing.
    pub fn upgrade_options(&mut self) -> Vec<Upgrade> {
        let mut pool: Vec<Upgrade> = Upgrade::ALL
            .into_iter()
            .chain(self.earned.iter().copied())
            .filter(|u| !u.is_spent(&self.loadout))
            .collect();
        self.rng.shuffle(&mut pool);
        // A patch is always worth offering to a damaged skimmer.
        if self.hull < self.loadout.max_hull / 2
            && !pool[..3.min(pool.len())].contains(&Upgrade::Patch)
        {
            if let Some(at) = pool.iter().position(|u| *u == Upgrade::Patch) {
                pool.swap(0, at);
            }
        }
        pool.truncate(3);
        pool
    }

    pub fn apply_upgrade(&mut self, u: Upgrade) {
        let l = &mut self.loadout;
        match u {
            Upgrade::Shield => l.shields += 1,
            Upgrade::BoostCoils => l.boost_cooldown = (l.boost_cooldown - 1.5).max(2.0),
            Upgrade::HeavyRounds => l.heavy_rounds = true,
            Upgrade::Patch => self.hull = (self.hull + 30).min(l.max_hull),
            Upgrade::PulseCell => self.slowdown_charges += 1,
            Upgrade::OverrideCell => self.override_charges += 1,
            Upgrade::NearMiss => l.near_miss = true,
            Upgrade::Damper => l.stress_damp = (l.stress_damp + 0.25).min(0.5),
            Upgrade::Magnet => l.magnet = true,
            Upgrade::HardLock => l.hard_lock = true,
            Upgrade::ScrapMesh => l.scrap_mesh = true,
            Upgrade::DeadMan => l.dead_man = true,
            Upgrade::HotChain => l.hot_chain = true,
        }
        self.upgrades.push(u);
    }

    /// The routes on offer for the next sector, from those `unlocked`.
    pub fn route_options(&mut self, unlocked: &[Route]) -> Vec<Route> {
        self.route_options_with(unlocked, None)
    }

    /// Of the `unlocked` choosable routes, the one with the fewest clears
    /// counting this run's: `lifetime` gives the saved count. Ties are
    /// drawn at random.
    pub fn least_cleared(
        &mut self,
        unlocked: &[Route],
        lifetime: impl Fn(Route) -> u32,
    ) -> Option<Route> {
        let clears = |r: Route| {
            lifetime(r)
                + self
                    .sectors
                    .iter()
                    .take(self.sectors_cleared)
                    .filter(|s| s.route == r)
                    .count() as u32
        };
        let open: Vec<(Route, u32)> = Route::CHOOSABLE
            .into_iter()
            .filter(|r| unlocked.contains(r))
            .map(|r| (r, clears(r)))
            .collect();
        let min = open.iter().map(|(_, n)| *n).min()?;
        let tied: Vec<Route> = open
            .into_iter()
            .filter(|(_, n)| *n == min)
            .map(|(r, _)| r)
            .collect();
        Some(*self.rng.pick(&tied))
    }

    /// As [`Run::route_options`], always including `nudge` when it is open:
    /// the pilot's least-cleared route, so every strand gets heard.
    pub fn route_options_with(&mut self, unlocked: &[Route], nudge: Option<Route>) -> Vec<Route> {
        // A fixed route's next sector is already built.
        if self.mode.has_fixed_route() {
            return self
                .sectors
                .get(self.sectors_cleared)
                .map(|s| vec![s.route])
                .unwrap_or_default();
        }
        if let Some(total) = self.total_sectors() {
            if self.sectors.len() + 1 >= total {
                return vec![Route::Extraction];
            }
        }
        let mut pool: Vec<Route> = Route::CHOOSABLE
            .into_iter()
            .filter(|r| unlocked.contains(r))
            .collect();
        self.rng.shuffle(&mut pool);
        if let Some(n) = nudge {
            if let Some(at) = pool.iter().position(|r| *r == n) {
                if at >= 3 {
                    let slot = self.rng.below(3) as usize;
                    pool.swap(slot, at);
                }
            }
        }
        pool.truncate(3);
        pool
    }

    /// Leaves the intermission along `route`.
    pub fn choose_route(&mut self, route: Route) {
        if self.phase != Phase::Intermission {
            return;
        }
        if !(self.mode.has_fixed_route() && self.sectors.len() > self.sectors_cleared) {
            self.add_sector(route);
        }
        self.phase = Phase::Driving;
    }

    // -- what the player can ask -------------------------------------------------

    /// The status key, in layers. First: lane, hull and stress, the
    /// numbers that end a run. Second: what is ready to use. Third: where
    /// the run stands.
    pub fn status_layers(&self) -> Vec<String> {
        let first = format!(
            "{} lane. Hull {} of {}. Stress {} of {MAX_STRESS}, {}.",
            sentence(Lane::from_index(self.lane).name()),
            self.hull,
            self.loadout.max_hull,
            self.stress,
            self.pressure.text().to_lowercase()
        );
        let mut kit = Vec::new();
        kit.push(if self.twist == Some(Twist::NoBoost) {
            "Boost offline.".to_string()
        } else if self.boost_active() {
            "Boost active.".to_string()
        } else if self.boost_ready_in() <= 0.0 {
            "Boost ready.".to_string()
        } else {
            format!("Boost in {}.", seconds(self.boost_ready_in()))
        });
        kit.push(if self.weapon_ready_in() <= 0.0 {
            "Rail gun ready.".to_string()
        } else {
            format!("Rail gun in {}.", seconds(self.weapon_ready_in()))
        });
        kit.push(if self.slowdown_active() {
            "Slowdown pulse active.".to_string()
        } else if self.slowdown_charges <= 0 {
            "Slowdown pulse spent.".to_string()
        } else if self.slowdown_ready_in() <= 0.0 {
            format!("Slowdown pulse ready, {} left.", self.slowdown_charges)
        } else {
            format!(
                "Slowdown pulse in {}, {} left.",
                seconds(self.slowdown_ready_in()),
                self.slowdown_charges
            )
        });
        kit.push(if self.override_active() {
            "Priority override active.".to_string()
        } else {
            format!("Priority overrides {}.", self.override_charges)
        });
        if self.loadout.shields > 0 {
            kit.push(format!("Shields {}.", self.loadout.shields));
        }
        let total = self.total_sectors();
        let number = self.current_sector.map_or(1, |i| i + 1);
        let mut run = vec![match total {
            Some(total) => format!("Sector {} of {total}.", number.min(total)),
            None => format!("Sector {number}."),
        }];
        match self.length() {
            Some(length) => run.push(format!(
                "{} remaining.",
                seconds((length - self.t).max(0.0))
            )),
            None => {
                if let Some(sector) = self.sector() {
                    run.push(format!(
                        "{} to the sector's end.",
                        seconds((sector.end - self.t).max(0.0))
                    ));
                }
            }
        }
        run.push(format!(
            "Score {}, multiplier {}.",
            self.score, self.multiplier
        ));
        run.push(self.event_rule.clone());
        vec![
            first,
            parlor_core::status::sentences(kit),
            parlor_core::status::sentences(run),
        ]
    }

    /// Every layer of the status key at once, for the screen and tests.
    pub fn status(&self) -> String {
        parlor_core::status::sentences(self.status_layers())
    }

    /// The next arrival and what to do about it, naming the player's
    /// controls as given.
    pub fn hint(&self, keys: &Keys) -> String {
        let next = self
            .events
            .iter()
            .filter(|e| e.announced && !e.resolved)
            .min_by(|a, b| a.resolve.total_cmp(&b.resolve));
        let Some(e) = next else {
            let upcoming = self.events.iter().any(|e| !e.announced);
            return if upcoming {
                "Corridor clear for now.".to_string()
            } else if let Some(sector) = self.sector() {
                format!(
                    "Corridor clear. Sector ends in {}.",
                    seconds((sector.end - self.t).max(0.0))
                )
            } else {
                "Corridor clear.".to_string()
            };
        };
        let when = seconds_short(e.resolve - self.t);
        let here = e.lane.index() == Some(self.lane);
        let name = if self.convoy && e.kind == Kind::Priority {
            "Evacuation train".to_string()
        } else {
            sentence(self.kind_name(e.kind))
        };
        let lane = match e.lane {
            Lane::All => "across all lanes".to_string(),
            l if here => format!("in your lane, {}", l.name()),
            l => format!("in the {} lane", l.name()),
        };
        let toward = |l: Lane| -> String {
            let target = l.index().unwrap_or(1) as i32;
            let dir = if target < self.lane as i32 {
                &keys.left
            } else {
                &keys.right
            };
            let steps = (target - self.lane as i32).unsigned_abs();
            if steps == 2 {
                format!("press {dir} twice")
            } else {
                format!("press {dir}")
            }
        };
        let boost_ok = self.twist != Some(Twist::NoBoost);
        let advice = match (e.kind, here) {
            (Kind::Debris, true) | (Kind::HeavyBlocker, true) if !boost_ok => format!(
                "Shift out with {} or {}. Boost is offline.",
                keys.left, keys.right
            ),
            (Kind::Drone, true) | (Kind::Unstable, true) if !boost_ok => format!(
                "Shift out, or fire the rail gun with {} inside its last second.",
                keys.fire
            ),
            (Kind::Blockade, _) => format!(
                "You cannot dodge it. Press {} just before it hits.",
                keys.boost
            ),
            (Kind::Debris, true) => format!(
                "Shift out with {} or {}, or boost through it with {}.",
                keys.left, keys.right, keys.boost
            ),
            (Kind::Drone, true) | (Kind::Unstable, true) => format!(
                "Shift out, fire the rail gun with {} inside its last second, or boost through with {}.",
                keys.fire, keys.boost
            ),
            (Kind::Drone, false) | (Kind::Unstable, false) => format!(
                "Stay out of its lane, or fire the rail gun with {} as it closes in, for points.",
                keys.fire
            ),
            (Kind::HeavyBlocker, true) => {
                format!("Shift out, or boost with {} to force it open.", keys.boost)
            }
            (Kind::Debris, false) | (Kind::HeavyBlocker, false) => {
                "You are clear. Stay out of its lane.".to_string()
            }
            (Kind::Flow, true) | (Kind::Repair, true) | (Kind::Salvage, true) => {
                "You are in its lane. Hold steady to meet it.".to_string()
            }
            (Kind::Flow, false) | (Kind::Repair, false) | (Kind::Salvage, false) => {
                format!("Join its lane: {}.", toward(e.lane))
            }
            (Kind::FastPasser, true) => "You are in its lane. Hold steady.".to_string(),
            (Kind::FastPasser, false) => format!(
                "Join its lane: {}, or use the slowdown pulse with {}.",
                toward(e.lane),
                keys.slowdown
            ),
            (Kind::Priority, true) => "You are in its lane. Hold steady to clear it.".to_string(),
            (Kind::Priority, false) => format!(
                "Join its lane: {}, or use priority override with {}.",
                toward(e.lane),
                keys.priority
            ),
        };
        format!("{name} {lane}, {when}. {advice}")
    }
}

/// The player's control names, for hints.
#[derive(Clone, Debug, Default)]
pub struct Keys {
    pub left: String,
    pub right: String,
    pub boost: String,
    pub fire: String,
    pub slowdown: String,
    pub priority: String,
}

/// "3 seconds", rounding up as the original did.
pub fn seconds(s: f64) -> String {
    let n = (s - 1e-9).ceil().max(0.0) as i64;
    if n == 1 {
        "1 second".to_string()
    } else {
        format!("{n} seconds")
    }
}

/// "in 1.4 seconds" or "now".
fn seconds_short(s: f64) -> String {
    if s <= 0.05 {
        "now".to_string()
    } else {
        format!("in {:.1} seconds", s)
    }
}

/// The first letter in capitals.
pub fn sentence(text: &str) -> String {
    let mut c = text.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
