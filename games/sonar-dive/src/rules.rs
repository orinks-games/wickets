//! The sea and the sub, as plain data driven by `dt`, so tests can play a
//! dive without a screen.
//!
//! The sea is a flat, top-down disc around the boat. Positions are metres:
//! `x` east, `y` north. Headings are degrees clockwise from north, the way
//! the audio listener faces. The boat's bell is at the origin.

use parlor_core::Rng;

/// How fast sonar travels out and back, metres of range per second: an
/// echo from 50 metres returns a second after the ping.
pub const ECHO_SPEED: f32 = 50.0;
/// Seconds between pings.
pub const PING_COOLDOWN: f64 = 1.2;
/// Degrees a held turn key turns per second.
pub const TURN_RATE: f32 = 90.0;
/// Speed in metres per second for each throttle step, reverse to full.
pub const SPEEDS: [f32; 5] = [-1.5, 0.0, 2.0, 4.0, 6.0];
pub const MIN_THROTTLE: i32 = -1;
pub const MAX_THROTTLE: i32 = 3;
/// How close a mine must be before it ticks.
pub const MINE_HEARING: f32 = 16.0;
/// Touching distance for a mine.
pub const MINE_BLAST: f32 = 2.5;
/// Pick-up distance for a chest.
pub const CHEST_REACH: f32 = 3.0;
/// How near a wreck you must stop to salvage it.
pub const WRECK_REACH: f32 = 6.0;
/// Seconds of stillness a wreck takes to salvage.
pub const SALVAGE_SECONDS: f64 = 3.0;
/// The galleon is bigger and takes longer.
pub const GALLEON_SECONDS: f64 = 6.0;
/// How close a whale must pass to share its air.
pub const WHALE_REACH: f32 = 7.0;
/// Seconds of air a whale gives.
pub const WHALE_AIR: f64 = 25.0;
/// Seconds of air a scrape costs.
pub const SCRAPE_AIR: f64 = 3.0;
/// The bell surfaces you inside this distance, once you have left it.
pub const BELL_REACH: f32 = 5.0;
/// How far from the bell counts as having left it.
pub const BELL_LEAVE: f32 = 12.0;
/// Seconds between bell rings.
pub const BELL_EVERY: f64 = 5.0;
/// Buoys beep this often.
pub const BUOY_EVERY: f64 = 2.5;
/// At most this many buoys are out at once.
pub const MAX_BUOYS: usize = 3;
/// At most this many echoes return from one ping, nearest first.
pub const MAX_ECHOES: usize = 10;
/// What a glow pearl is worth, before the depth's value.
pub const PEARL_LOOT: u32 = 150;
/// What the galleon holds.
pub const GALLEON_LOOT: u32 = 1000;

/// A point or a direction on the sea.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const ZERO: V2 = V2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> V2 {
        V2 { x, y }
    }

    /// A unit vector pointing along `heading` degrees.
    pub fn from_heading(heading: f32) -> V2 {
        let (s, c) = heading.to_radians().sin_cos();
        V2::new(s, c)
    }

    pub fn len(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn dist(self, other: V2) -> f32 {
        (self - other).len()
    }

    pub fn scale(self, k: f32) -> V2 {
        V2::new(self.x * k, self.y * k)
    }

    /// The same direction with length one; zero stays zero.
    pub fn unit(self) -> V2 {
        let l = self.len();
        if l < 1e-6 {
            V2::ZERO
        } else {
            self.scale(1.0 / l)
        }
    }

    /// Degrees clockwise from north this direction points.
    pub fn heading(self) -> f32 {
        self.x.atan2(self.y).to_degrees().rem_euclid(360.0)
    }
}

impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2::new(self.x - o.x, self.y - o.y)
    }
}

/// Wraps a heading into `0..360`.
pub fn wrap(deg: f32) -> f32 {
    deg.rem_euclid(360.0)
}

/// Where `to` lies from `from` for a sub facing `heading`, as a clock hour:
/// 12 straight ahead, 3 to the right, 6 behind, 9 to the left.
pub fn clock(heading: f32, from: V2, to: V2) -> u32 {
    clock_of(wrap((to - from).heading() - heading))
}

/// A relative bearing in degrees as a clock hour.
pub fn clock_of(relative: f32) -> u32 {
    let h = (wrap(relative) / 30.0).round() as u32 % 12;
    if h == 0 {
        12
    } else {
        h
    }
}

/// "north", "north-east"... for a heading.
pub fn compass(heading: f32) -> &'static str {
    const POINTS: [&str; 8] = [
        "north",
        "north-east",
        "east",
        "south-east",
        "south",
        "south-west",
        "west",
        "north-west",
    ];
    POINTS[((wrap(heading) + 22.5) / 45.0) as usize % 8]
}

/// What the throttle is set to, as the sub's telegraph says it.
pub fn throttle_name(throttle: i32) -> &'static str {
    match throttle {
        i32::MIN..=-1 => "slow astern",
        0 => "all stop",
        1 => "slow ahead",
        2 => "half ahead",
        _ => "full ahead",
    }
}

/// Things in the sea.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A sunken ship with treasure in its hold: stop beside it to salvage.
    Wreck,
    /// A chest on the seabed: swim into it.
    Chest,
    /// A mine: ticks when you are near, explodes on contact.
    Mine,
    /// A rock outcrop: scrapes and stops you.
    Rock,
    /// A friendly whale: pass close and it shares air.
    Whale,
    /// A current: pushes you while you are inside it.
    Current,
    /// The monthly event's treasure galleon.
    Galleon,
    /// The rock wall at the edge of the dive site.
    Wall,
    /// The glow dive's glow pearl: swim into it, like a chest.
    Pearl,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Wreck => "wreck",
            Kind::Chest => "treasure chest",
            Kind::Mine => "mine",
            Kind::Rock => "rock",
            Kind::Whale => "whale",
            Kind::Current => "current",
            Kind::Galleon => "galleon",
            Kind::Wall => "rock wall",
            Kind::Pearl => "glow pearl",
        }
    }
}

/// One thing in the sea.
#[derive(Clone, Debug)]
pub struct Object {
    pub kind: Kind,
    pub pos: V2,
    /// Its size: echoes return from its edge.
    pub radius: f32,
    /// Coins it still holds (wrecks, chests, the galleon).
    pub loot: u32,
    /// False once a chest is taken or a mine has gone off.
    pub active: bool,
    /// A current's push, metres per second.
    pub flow: V2,
    /// A whale circles this point.
    pub center: V2,
    pub orbit: f32,
    pub angle: f32,
    /// A mine has been announced; a whale has given its air.
    pub noted: bool,
    pub song_in: f64,
}

impl Object {
    fn new(kind: Kind, pos: V2, radius: f32) -> Object {
        Object {
            kind,
            pos,
            radius,
            loot: 0,
            active: true,
            flow: V2::ZERO,
            center: pos,
            orbit: 0.0,
            angle: 0.0,
            noted: false,
            song_in: 0.0,
        }
    }

    /// Whether a ping hears it.
    pub fn echoes(&self) -> bool {
        match self.kind {
            Kind::Chest | Kind::Pearl | Kind::Mine => self.active,
            _ => true,
        }
    }

    /// A wreck or galleon whose hold is empty.
    pub fn emptied(&self) -> bool {
        matches!(self.kind, Kind::Wreck | Kind::Galleon) && self.loot == 0
    }

    /// Treasure still to take here.
    pub fn treasure(&self) -> bool {
        matches!(
            self.kind,
            Kind::Wreck | Kind::Chest | Kind::Galleon | Kind::Pearl
        ) && self.active
            && self.loot > 0
    }
}

/// A rule that changes a challenge run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    None,
    /// Sonar reaches half as far.
    Murky,
    /// Treasure is worth double, and there are four more mines.
    Rich,
    /// Currents are stronger and there are two more.
    Rip,
    /// The tank holds 60 percent of its usual air.
    ThinAir,
}

impl Modifier {
    pub const TWISTS: [Modifier; 4] = [
        Modifier::Murky,
        Modifier::Rich,
        Modifier::Rip,
        Modifier::ThinAir,
    ];

    /// Picks a twist from a challenge seed.
    pub fn from_seed(seed: u64) -> Modifier {
        Modifier::TWISTS[(seed % Modifier::TWISTS.len() as u64) as usize]
    }

    pub fn name(self) -> &'static str {
        match self {
            Modifier::None => "no twist",
            Modifier::Murky => "Murky water",
            Modifier::Rich => "Rich wrecks",
            Modifier::Rip => "Rip currents",
            Modifier::ThinAir => "Thin air",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Modifier::None => "",
            Modifier::Murky => "sonar reaches only half as far",
            Modifier::Rich => "treasure is worth double, but four more mines are laid",
            Modifier::Rip => "currents are much stronger and there are two more of them",
            Modifier::ThinAir => "your tank holds only 60 percent of its usual air",
        }
    }
}

/// How an expedition is set up.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// The first dive's number: deeper numbers are darker and more crowded.
    pub start_dive: u32,
    /// The most dives the expedition allows; `None` for no limit.
    pub max_dives: Option<u32>,
    /// Hull points at the start.
    pub hull: u32,
    /// Whether surfacing repairs one hull point.
    pub repair: bool,
    pub modifier: Modifier,
    /// No air drain and harmless mines, for learning.
    pub practice: bool,
    /// Lays the monthly event's galleon.
    pub galleon: bool,
    /// Lays a glow pearl on every dive: the glow dive.
    pub pearl: bool,
}

impl Config {
    /// The standard expedition from dive 1.
    pub fn expedition() -> Config {
        Config {
            start_dive: 1,
            max_dives: None,
            hull: 3,
            repair: true,
            modifier: Modifier::None,
            practice: false,
            galleon: false,
            pearl: false,
        }
    }

    /// The glow dive: an expedition with a glow pearl on every dive.
    pub fn glow() -> Config {
        Config {
            pearl: true,
            ..Config::expedition()
        }
    }

    /// The unlockable deep start: begin at dive 4.
    pub fn deep_start() -> Config {
        Config {
            start_dive: 4,
            ..Config::expedition()
        }
    }

    /// A practice dive: no air drain, mines do no harm.
    pub fn practice() -> Config {
        Config {
            practice: true,
            ..Config::expedition()
        }
    }

    /// One dive, from dive 1 to 3, with a twist.
    pub fn daily(seed: u64) -> Config {
        Config {
            start_dive: 1 + ((seed >> 8) % 3) as u32,
            max_dives: Some(1),
            modifier: Modifier::from_seed(seed),
            ..Config::expedition()
        }
    }

    /// Up to three dives from dive 3, no repairs, with a twist.
    pub fn weekly(seed: u64) -> Config {
        Config {
            start_dive: 3,
            max_dives: Some(3),
            repair: false,
            modifier: Modifier::from_seed(seed >> 3),
            ..Config::expedition()
        }
    }

    /// The sunken galleon: two dives from dive 4, one hull point.
    pub fn monthly() -> Config {
        Config {
            start_dive: 4,
            max_dives: Some(2),
            hull: 1,
            repair: false,
            galleon: true,
            ..Config::expedition()
        }
    }
}

/// How a dive's site is laid out, from its number and the config.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub number: u32,
    pub depth_m: u32,
    pub radius: f32,
    pub chests: usize,
    pub wrecks: usize,
    pub mines: usize,
    pub rocks: usize,
    pub whales: usize,
    pub currents: usize,
    pub current_speed: f32,
    pub oxygen: f64,
    pub sonar_range: f32,
    /// Treasure value multiplier, in percent.
    pub value: u32,
}

impl Layout {
    pub fn for_dive(number: u32, config: &Config) -> Layout {
        let n = number.max(1);
        let m = config.modifier;
        let rip = m == Modifier::Rip;
        Layout {
            number: n,
            depth_m: 30 * n,
            radius: (45.0 + 8.0 * n as f32).min(100.0),
            chests: (3 + n as usize / 2).min(7),
            wrecks: (1 + n as usize / 2).min(4),
            mines: (2 + 2 * n as usize).min(16) + if m == Modifier::Rich { 4 } else { 0 },
            rocks: (3 + n as usize).min(9),
            whales: 1,
            currents: if n >= 2 { (n as usize / 2).min(4) } else { 0 } + if rip { 2 } else { 0 },
            current_speed: (1.2 + 0.3 * n as f32).min(3.2) * if rip { 1.6 } else { 1.0 },
            oxygen: (150.0 - 10.0 * (n as f64 - 1.0)).max(80.0)
                * if m == Modifier::ThinAir { 0.6 } else { 1.0 },
            sonar_range: (75.0 - 3.0 * n as f32).max(48.0)
                * if m == Modifier::Murky { 0.5 } else { 1.0 },
            value: (100 + 25 * (n - 1)) * if m == Modifier::Rich { 2 } else { 1 },
        }
    }
}

/// How a dive ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiveEnd {
    Surfaced,
    Drowned,
    Wrecked,
}

/// The air warnings, each given once a dive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirWarning {
    Half,
    Quarter,
    Low,
}

/// One echo a ping heard, for the spoken readout and hints.
#[derive(Clone, Debug, PartialEq)]
pub struct Return {
    pub kind: Kind,
    pub dist: f32,
    pub clock: u32,
    pub empty: bool,
}

/// What happened during an update, for the screen to sound and say.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Ping,
    PingNotReady,
    Echo {
        kind: Kind,
        pos: V2,
        empty: bool,
    },
    /// The last echo of a ping is back.
    PingDone {
        returns: Vec<Return>,
    },
    Chest {
        value: u32,
        carried: u32,
    },
    /// A glow pearl collected.
    Pearl {
        value: u32,
        carried: u32,
    },
    SalvageStart {
        galleon: bool,
    },
    SalvageClank {
        pos: V2,
    },
    SalvageDone {
        value: u32,
        carried: u32,
        galleon: bool,
    },
    SalvageBroken,
    MineNear {
        pos: V2,
    },
    MineTick {
        pos: V2,
    },
    MineHit {
        pos: V2,
        hull: u32,
    },
    Scrape {
        pos: V2,
        wall: bool,
    },
    WhaleSong {
        pos: V2,
    },
    WhaleAir {
        secs: f64,
    },
    CurrentIn {
        flow: V2,
    },
    CurrentOut,
    Air(AirWarning),
    Bell,
    BuoyBeep {
        pos: V2,
        n: u32,
    },
    End(DiveEnd),
}

#[derive(Clone, Debug)]
pub struct Buoy {
    pub n: u32,
    pub pos: V2,
    pub beep_in: f64,
}

#[derive(Clone, Debug)]
struct PendingEcho {
    at: f64,
    kind: Kind,
    pos: V2,
    empty: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Salvage {
    pub index: usize,
    pub progress: f64,
    clank_in: f64,
}

/// One dive: from leaving the bell to surfacing at it, or not.
#[derive(Clone, Debug)]
pub struct Dive {
    pub layout: Layout,
    pub practice: bool,
    /// Run by the tutorial: the bell never surfaces the sub, so the lesson
    /// ends before the dive is summed up.
    pub lesson: bool,
    pub objects: Vec<Object>,
    pub pos: V2,
    pub heading: f32,
    pub throttle: i32,
    pub oxygen: f64,
    pub max_oxygen: f64,
    pub carried: u32,
    pub hull: u32,
    pub max_hull: u32,
    pub time: f64,
    pub buoys: Vec<Buoy>,
    pub salvage: Option<Salvage>,
    /// The current the sub is in, by object index.
    pub in_current: Option<usize>,
    pub left_bell: bool,
    pub outcome: Option<DiveEnd>,
    /// What the last ping heard.
    pub returns: Vec<Return>,
    /// Pings sent this dive.
    pub pings: u32,
    echoes: Vec<PendingEcho>,
    ping_pending: bool,
    ping_ready_at: f64,
    bell_in: f64,
    mine_tick_in: f64,
    scrape_cool: f64,
    warned: [bool; 3],
    next_buoy: u32,
}

impl Dive {
    /// A dive site laid out from `rng`.
    pub fn generate(layout: Layout, config: &Config, hull: u32, rng: &mut Rng) -> Dive {
        let mut objects: Vec<Object> = Vec::new();
        let r = layout.radius;
        let value = |base: u32| base * layout.value / 100;
        if config.galleon {
            let a = rng.unit() as f32 * 360.0;
            let mut g = Object::new(Kind::Galleon, V2::from_heading(a).scale(r * 0.8), 4.0);
            g.loot = GALLEON_LOOT;
            let center = g.pos;
            objects.push(g);
            for i in 0..6 {
                let b = a + 30.0 + 60.0 * i as f32;
                let d = 10.0 + rng.unit() as f32 * 2.0;
                objects.push(Object::new(
                    Kind::Mine,
                    center + V2::from_heading(b).scale(d),
                    0.6,
                ));
            }
        }
        for _ in 0..layout.wrecks {
            if let Some(p) = spot(rng, r, 18.0, &objects, 10.0) {
                let mut o = Object::new(Kind::Wreck, p, 3.0);
                o.loot = value(120 + rng.below(81));
                objects.push(o);
            }
        }
        for _ in 0..layout.chests {
            if let Some(p) = spot(rng, r, 14.0, &objects, 7.0) {
                let mut o = Object::new(Kind::Chest, p, 0.8);
                o.loot = value(40 + rng.below(41));
                objects.push(o);
            }
        }
        // Half the mines guard wrecks; the rest drift free.
        let wrecks: Vec<V2> = objects
            .iter()
            .filter(|o| o.kind == Kind::Wreck)
            .map(|o| o.pos)
            .collect();
        for i in 0..layout.mines {
            let p = if i % 2 == 0 && !wrecks.is_empty() {
                let w = *rng.pick(&wrecks);
                (0..20).find_map(|_| {
                    let p = w + V2::from_heading(rng.unit() as f32 * 360.0)
                        .scale(9.0 + rng.unit() as f32 * 4.0);
                    clear(p, r, 14.0, &objects, 7.0).then_some(p)
                })
            } else {
                spot(rng, r, 14.0, &objects, 7.0)
            };
            if let Some(p) = p {
                objects.push(Object::new(Kind::Mine, p, 0.6));
            }
        }
        for _ in 0..layout.rocks {
            let size = 2.0 + rng.unit() as f32 * 2.0;
            if let Some(p) = spot(rng, r, 14.0, &objects, 6.0 + size) {
                objects.push(Object::new(Kind::Rock, p, size));
            }
        }
        // Currents keep clear of wrecks and the galleon: you cannot salvage
        // while a current pushes you, so one over a hold would seal it.
        let holds: Vec<Object> = objects
            .iter()
            .filter(|o| matches!(o.kind, Kind::Wreck | Kind::Galleon))
            .cloned()
            .collect();
        for _ in 0..layout.currents {
            let size = 10.0 + rng.unit() as f32 * 5.0;
            if let Some(p) = spot(rng, r - 6.0, 16.0 + size, &holds, size + WRECK_REACH) {
                let mut o = Object::new(Kind::Current, p, size);
                o.flow = V2::from_heading(rng.unit() as f32 * 360.0).scale(layout.current_speed);
                objects.push(o);
            }
        }
        for i in 0..layout.whales {
            if let Some(p) = spot(rng, r - 15.0, 20.0, &[], 0.0) {
                let mut o = Object::new(Kind::Whale, p, 3.0);
                o.center = p;
                o.orbit = 8.0;
                o.angle = rng.unit() as f32 * 360.0;
                o.pos = p + V2::from_heading(o.angle).scale(o.orbit);
                o.song_in = 2.0 + i as f64 * 1.3;
                objects.push(o);
            }
        }
        // Laid last, so a sea without one is the same as ever.
        if config.pearl {
            if let Some(p) = spot(rng, r, 20.0, &objects, 8.0) {
                let mut o = Object::new(Kind::Pearl, p, 0.6);
                o.loot = value(PEARL_LOOT);
                objects.push(o);
            }
        }
        Dive::with_objects(layout, config, hull, objects)
    }

    /// A dive site with exactly these objects, for tests and set pieces.
    pub fn with_objects(layout: Layout, config: &Config, hull: u32, objects: Vec<Object>) -> Dive {
        Dive {
            practice: config.practice,
            oxygen: layout.oxygen,
            max_oxygen: layout.oxygen,
            layout,
            objects,
            pos: V2::ZERO,
            heading: 0.0,
            throttle: 0,
            carried: 0,
            hull,
            max_hull: config.hull,
            time: 0.0,
            buoys: Vec::new(),
            salvage: None,
            in_current: None,
            left_bell: false,
            lesson: false,
            outcome: None,
            returns: Vec::new(),
            pings: 0,
            echoes: Vec::new(),
            ping_pending: false,
            ping_ready_at: 0.0,
            bell_in: 0.5,
            mine_tick_in: 0.0,
            scrape_cool: 0.0,
            warned: [false; 3],
            next_buoy: 1,
        }
    }

    /// Adds one object, for tests.
    pub fn place(&mut self, kind: Kind, pos: V2, loot: u32) -> usize {
        let radius = match kind {
            Kind::Wreck => 3.0,
            Kind::Galleon => 4.0,
            Kind::Chest => 0.8,
            Kind::Pearl => 0.6,
            Kind::Mine => 0.6,
            Kind::Rock => 2.5,
            Kind::Whale => 3.0,
            Kind::Current => 10.0,
            Kind::Wall => 0.0,
        };
        let mut o = Object::new(kind, pos, radius);
        o.loot = loot;
        if kind == Kind::Whale {
            o.orbit = 0.0;
            o.song_in = 9.0;
        }
        self.objects.push(o);
        self.objects.len() - 1
    }

    /// The speed the throttle asks for.
    pub fn speed(&self) -> f32 {
        SPEEDS[(self.throttle - MIN_THROTTLE) as usize]
    }

    /// One throttle step up; `None` at full ahead already.
    pub fn throttle_up(&mut self) -> Option<i32> {
        (self.throttle < MAX_THROTTLE).then(|| {
            self.throttle += 1;
            self.throttle
        })
    }

    /// One throttle step down; `None` at slow astern already.
    pub fn throttle_down(&mut self) -> Option<i32> {
        (self.throttle > MIN_THROTTLE).then(|| {
            self.throttle -= 1;
            self.throttle
        })
    }

    /// Whether echoes of the last ping are still on their way back.
    pub fn echoes_due(&self) -> bool {
        !self.echoes.is_empty()
    }

    /// Sends a ping: the echoes come back over the next updates.
    pub fn ping(&mut self) -> Event {
        if self.outcome.is_some() || self.time < self.ping_ready_at {
            return Event::PingNotReady;
        }
        self.ping_ready_at = self.time + PING_COOLDOWN;
        self.pings += 1;
        let range = self.layout.sonar_range;
        let mut found: Vec<(f32, Kind, V2, bool)> = self
            .objects
            .iter()
            .filter(|o| o.echoes())
            .map(|o| {
                let d = (o.pos.dist(self.pos) - o.radius).max(0.5);
                (d, o.kind, o.pos, o.emptied())
            })
            .filter(|(d, ..)| *d <= range)
            .collect();
        let edge = self.layout.radius - self.pos.len();
        // Exactly at the bell the wall is equally far all round, so it has
        // no one direction to answer from: leave it out rather than fake one.
        if edge <= range && self.pos.len() >= 1e-3 {
            let out = self.pos.unit();
            found.push((
                edge.max(0.5),
                Kind::Wall,
                out.scale(self.layout.radius),
                false,
            ));
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        found.truncate(MAX_ECHOES);
        self.returns = found
            .iter()
            .map(|&(dist, kind, pos, empty)| Return {
                kind,
                dist,
                clock: clock(self.heading, self.pos, pos),
                empty,
            })
            .collect();
        self.echoes = found
            .into_iter()
            .map(|(d, kind, pos, empty)| PendingEcho {
                at: self.time + (d / ECHO_SPEED) as f64,
                kind,
                pos,
                empty,
            })
            .collect();
        self.ping_pending = true;
        Event::Ping
    }

    /// Whether a ping's echoes are still on their way back: nothing else
    /// should sound over them.
    pub fn echoes_pending(&self) -> bool {
        self.ping_pending || !self.echoes.is_empty()
    }

    /// Whether a ping is ready.
    pub fn ping_ready(&self) -> bool {
        self.time >= self.ping_ready_at
    }

    /// Drops a buoy here; the oldest goes when there are too many. Gives
    /// the new buoy's number.
    pub fn drop_buoy(&mut self) -> u32 {
        let n = self.next_buoy;
        self.next_buoy += 1;
        self.buoys.push(Buoy {
            n,
            pos: self.pos,
            beep_in: BUOY_EVERY,
        });
        if self.buoys.len() > MAX_BUOYS {
            self.buoys.remove(0);
        }
        n
    }

    pub fn bell_distance(&self) -> f32 {
        self.pos.len()
    }

    pub fn bell_clock(&self) -> u32 {
        clock(self.heading, self.pos, V2::ZERO)
    }

    /// Seconds of air a straight run home at half ahead would take, with a
    /// margin.
    pub fn air_home(&self) -> f64 {
        (self.bell_distance() / SPEEDS[3]) as f64 * 1.3 + 8.0
    }

    /// The nearest treasure still to take, by object index.
    pub fn nearest_treasure(&self) -> Option<usize> {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.treasure())
            .min_by(|a, b| a.1.pos.dist(self.pos).total_cmp(&b.1.pos.dist(self.pos)))
            .map(|(i, _)| i)
    }

    /// The nearest live mine and how far it is.
    pub fn nearest_mine(&self) -> Option<(usize, f32)> {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.kind == Kind::Mine && o.active)
            .map(|(i, o)| (i, o.pos.dist(self.pos)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Treasure left on this site, in coins.
    pub fn treasure_left(&self) -> u32 {
        self.objects
            .iter()
            .filter(|o| o.treasure())
            .map(|o| o.loot)
            .sum()
    }

    /// Advances the dive by `dt` seconds with the turn keys at `turn`
    /// (-1 left, 1 right, 0 none).
    pub fn update(&mut self, dt: f64, turn: i32) -> Vec<Event> {
        let mut ev = Vec::new();
        if self.outcome.is_some() || dt <= 0.0 {
            return ev;
        }
        self.time += dt;
        let fdt = dt as f32;
        self.heading = wrap(self.heading + turn.signum() as f32 * TURN_RATE * fdt);

        // Currents.
        let here = self
            .objects
            .iter()
            .position(|o| o.kind == Kind::Current && o.pos.dist(self.pos) < o.radius);
        if here != self.in_current {
            match here {
                Some(i) => ev.push(Event::CurrentIn {
                    flow: self.objects[i].flow,
                }),
                None => ev.push(Event::CurrentOut),
            }
            self.in_current = here;
        }
        let flow = here.map_or(V2::ZERO, |i| self.objects[i].flow);
        let own = V2::from_heading(self.heading).scale(self.speed());
        let v = own + flow;
        self.pos = self.pos + v.scale(fdt);

        // Walls and rocks. The engine stops only when the sub itself is
        // driving into the obstacle: a current pinning you against rock
        // must not undo the throttle you are using to get away.
        let into = |normal: V2| own.x * normal.x + own.y * normal.y > 0.0;
        self.scrape_cool -= dt;
        let mut scraped = None;
        let mut stall = false;
        if self.pos.len() > self.layout.radius {
            let out = self.pos.unit();
            stall |= into(out);
            self.pos = out.scale(self.layout.radius - 0.5);
            scraped = Some((out.scale(self.layout.radius), true));
        }
        for o in self.objects.iter().filter(|o| o.kind == Kind::Rock) {
            let d = o.pos.dist(self.pos);
            if d < o.radius + 1.0 {
                let away = (self.pos - o.pos).unit();
                let away = if away == V2::ZERO {
                    V2::from_heading(self.heading + 180.0)
                } else {
                    away
                };
                stall |= into(away.scale(-1.0));
                self.pos = o.pos + away.scale(o.radius + 1.05);
                scraped = Some((o.pos, false));
            }
        }
        if let Some((pos, wall)) = scraped {
            if stall {
                self.throttle = 0;
            }
            if self.scrape_cool <= 0.0 {
                self.scrape_cool = 1.5;
                if !self.practice {
                    self.oxygen -= SCRAPE_AIR;
                }
                ev.push(Event::Scrape { pos, wall });
            }
        }

        // Air.
        if !self.practice {
            self.oxygen -= dt * (1.0 + 0.15 * self.throttle.max(0) as f64);
        }
        let frac = self.oxygen / self.max_oxygen;
        let warnings = [
            (frac <= 0.5, AirWarning::Half),
            (frac <= 0.25, AirWarning::Quarter),
            (self.oxygen <= 15.0, AirWarning::Low),
        ];
        for (i, (due, w)) in warnings.into_iter().enumerate() {
            if due && !self.warned[i] {
                self.warned[i] = true;
                // A sharp drop gives only the most urgent warning.
                if !warnings[i + 1..].iter().any(|(d, _)| *d) {
                    ev.push(Event::Air(w));
                }
            }
        }
        if self.oxygen <= 0.0 {
            self.oxygen = 0.0;
            return self.end(ev, DiveEnd::Drowned);
        }

        // Mines.
        let mut hit = None;
        for o in self
            .objects
            .iter_mut()
            .filter(|o| o.kind == Kind::Mine && o.active)
        {
            if o.pos.dist(self.pos) < MINE_BLAST {
                o.active = false;
                hit = Some(o.pos);
                break;
            }
        }
        if let Some(pos) = hit {
            if !self.practice {
                self.hull = self.hull.saturating_sub(1);
            }
            ev.push(Event::MineHit {
                pos,
                hull: self.hull,
            });
            if self.hull == 0 {
                return self.end(ev, DiveEnd::Wrecked);
            }
        }
        match self.nearest_mine() {
            Some((i, d)) if d < MINE_HEARING => {
                if !self.objects[i].noted {
                    self.objects[i].noted = true;
                    ev.push(Event::MineNear {
                        pos: self.objects[i].pos,
                    });
                }
                self.mine_tick_in -= dt;
                if self.mine_tick_in <= 0.0 {
                    self.mine_tick_in = 0.12 + 0.9 * (d / MINE_HEARING) as f64;
                    ev.push(Event::MineTick {
                        pos: self.objects[i].pos,
                    });
                }
            }
            _ => self.mine_tick_in = 0.0,
        }

        // Chests.
        for o in self
            .objects
            .iter_mut()
            .filter(|o| matches!(o.kind, Kind::Chest | Kind::Pearl) && o.active)
        {
            if o.pos.dist(self.pos) < CHEST_REACH {
                o.active = false;
                self.carried += o.loot;
                let (value, carried) = (o.loot, self.carried);
                ev.push(if o.kind == Kind::Pearl {
                    Event::Pearl { value, carried }
                } else {
                    Event::Chest { value, carried }
                });
                o.loot = 0;
            }
        }

        // Salvage.
        let still = self.throttle == 0 && self.in_current.is_none();
        let wreck = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| matches!(o.kind, Kind::Wreck | Kind::Galleon) && o.loot > 0)
            .find(|(_, o)| o.pos.dist(self.pos) < WRECK_REACH + o.radius - 3.0)
            .map(|(i, _)| i);
        match (wreck, still) {
            (Some(i), true) => {
                let galleon = self.objects[i].kind == Kind::Galleon;
                let need = if galleon {
                    GALLEON_SECONDS
                } else {
                    SALVAGE_SECONDS
                };
                let s = self.salvage.get_or_insert_with(|| {
                    ev.push(Event::SalvageStart { galleon });
                    Salvage {
                        index: i,
                        progress: 0.0,
                        clank_in: 0.6,
                    }
                });
                s.progress += dt;
                s.clank_in -= dt;
                if s.clank_in <= 0.0 {
                    s.clank_in = 0.6;
                    ev.push(Event::SalvageClank {
                        pos: self.objects[i].pos,
                    });
                }
                if s.progress >= need {
                    self.salvage = None;
                    let value = std::mem::take(&mut self.objects[i].loot);
                    self.carried += value;
                    ev.push(Event::SalvageDone {
                        value,
                        carried: self.carried,
                        galleon,
                    });
                }
            }
            _ => {
                if self.salvage.take().is_some() {
                    ev.push(Event::SalvageBroken);
                }
            }
        }

        // Whales.
        for o in self.objects.iter_mut().filter(|o| o.kind == Kind::Whale) {
            if o.orbit > 0.0 {
                o.angle = wrap(o.angle + (1.2 / o.orbit).to_degrees() * fdt);
                o.pos = o.center + V2::from_heading(o.angle).scale(o.orbit);
            }
            o.song_in -= dt;
            if o.song_in <= 0.0 {
                o.song_in = 9.0;
                ev.push(Event::WhaleSong { pos: o.pos });
            }
            // A whale keeps its air until you need some: passing it with a
            // nearly full tank does not spend its gift. In practice the air
            // never runs low, so it just greets you once.
            if !o.noted && o.pos.dist(self.pos) < WHALE_REACH {
                if self.practice {
                    o.noted = true;
                    ev.push(Event::WhaleAir { secs: 0.0 });
                } else if self.oxygen <= self.max_oxygen - 5.0 {
                    o.noted = true;
                    let before = self.oxygen;
                    self.oxygen = (self.oxygen + WHALE_AIR).min(self.max_oxygen);
                    ev.push(Event::WhaleAir {
                        secs: self.oxygen - before,
                    });
                }
            }
        }

        // The bell and buoys.
        self.bell_in -= dt;
        if self.bell_in <= 0.0 {
            self.bell_in = BELL_EVERY;
            ev.push(Event::Bell);
        }
        for b in &mut self.buoys {
            b.beep_in -= dt;
            if b.beep_in <= 0.0 {
                b.beep_in = BUOY_EVERY;
                ev.push(Event::BuoyBeep { pos: b.pos, n: b.n });
            }
        }

        // Echoes coming home.
        let now = self.time;
        let (due, later): (Vec<_>, Vec<_>) = self.echoes.drain(..).partition(|e| e.at <= now);
        self.echoes = later;
        for e in due {
            ev.push(Event::Echo {
                kind: e.kind,
                pos: e.pos,
                empty: e.empty,
            });
        }
        if self.ping_pending && self.echoes.is_empty() {
            self.ping_pending = false;
            ev.push(Event::PingDone {
                returns: self.returns.clone(),
            });
        }

        // Home.
        let home = self.bell_distance();
        if home > BELL_LEAVE {
            self.left_bell = true;
        }
        if self.left_bell && home < BELL_REACH && !self.lesson {
            return self.end(ev, DiveEnd::Surfaced);
        }
        ev
    }

    fn end(&mut self, mut ev: Vec<Event>, end: DiveEnd) -> Vec<Event> {
        self.outcome = Some(end);
        self.salvage = None;
        self.echoes.clear();
        ev.push(Event::End(end));
        ev
    }
}

/// A random clear point on the site, or `None` after many tries.
fn spot(rng: &mut Rng, radius: f32, from_bell: f32, others: &[Object], gap: f32) -> Option<V2> {
    (0..60).find_map(|_| {
        let a = rng.unit() as f32 * 360.0;
        let d = from_bell + (radius - 6.0 - from_bell).max(1.0) * (rng.unit() as f32).sqrt();
        let p = V2::from_heading(a).scale(d);
        clear(p, radius, from_bell, others, gap).then_some(p)
    })
}

fn clear(p: V2, radius: f32, from_bell: f32, others: &[Object], gap: f32) -> bool {
    p.len() >= from_bell
        && p.len() <= radius - 4.0
        && others
            .iter()
            .filter(|o| o.kind != Kind::Current)
            .all(|o| o.pos.dist(p) >= gap + o.radius)
}

/// What one whole expedition has done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpeditionEnd {
    /// The player chose to stop, or used every dive allowed.
    Banked,
    Drowned,
    Wrecked,
}

/// A run of dives from the boat, banking treasure each time you surface.
pub struct Expedition {
    pub config: Config,
    pub dive: Dive,
    /// Dives finished at the bell.
    pub surfaced: u32,
    pub banked: u32,
    /// What the last dive brought up.
    pub last_haul: u32,
    pub end: Option<ExpeditionEnd>,
    rng: Rng,
}

impl Expedition {
    pub fn new(config: Config, mut rng: Rng) -> Expedition {
        let layout = Layout::for_dive(config.start_dive, &config);
        let dive = Dive::generate(layout, &config, config.hull, &mut rng);
        Expedition {
            config,
            dive,
            surfaced: 0,
            banked: 0,
            last_haul: 0,
            end: None,
            rng,
        }
    }

    /// The current dive's number.
    pub fn dive_number(&self) -> u32 {
        self.dive.layout.number
    }

    /// How many dives this expedition has begun.
    pub fn dives_begun(&self) -> u32 {
        self.dive_number() + 1 - self.config.start_dive
    }

    /// Settles a finished dive: banks the haul on surfacing, or ends the
    /// expedition and loses it.
    pub fn settle(&mut self) {
        match self.dive.outcome {
            Some(DiveEnd::Surfaced) => {
                self.last_haul = self.dive.carried;
                self.banked += self.dive.carried;
                self.dive.carried = 0;
                self.surfaced += 1;
                if self.config.repair {
                    self.dive.hull = (self.dive.hull + 1).min(self.config.hull);
                }
                if !self.can_continue() {
                    self.end = Some(ExpeditionEnd::Banked);
                }
            }
            Some(DiveEnd::Drowned) => self.end = Some(ExpeditionEnd::Drowned),
            Some(DiveEnd::Wrecked) => self.end = Some(ExpeditionEnd::Wrecked),
            None => {}
        }
    }

    /// Whether another dive is allowed.
    pub fn can_continue(&self) -> bool {
        self.config
            .max_dives
            .is_none_or(|max| self.dives_begun() < max)
    }

    /// Starts the next, deeper dive.
    pub fn next_dive(&mut self) {
        let layout = Layout::for_dive(self.dive_number() + 1, &self.config);
        let hull = self.dive.hull;
        self.dive = Dive::generate(layout, &self.config, hull, &mut self.rng);
    }

    /// Stops after surfacing, keeping the haul.
    pub fn stop(&mut self) {
        self.end = Some(ExpeditionEnd::Banked);
    }

    /// The score: everything banked.
    pub fn score(&self) -> u32 {
        self.banked
    }
}
