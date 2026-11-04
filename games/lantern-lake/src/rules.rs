//! The lake itself, in plain numbers: the canoe, the drifting lanterns,
//! the creatures and the discoveries. Nothing here touches audio or
//! speech; the lake leaves [`Note`]s for the screen to play and say, so
//! tests can drive a whole night with `update(dt)`.

use parlor_core::{Period, Rng};

use crate::cues;

/// How fast a held Left or Right turns the canoe, degrees a second.
pub const TURN_RATE: f32 = 70.0;
/// How far one tap of Left or Right turns the canoe, degrees.
pub const TAP_TURN: f32 = 15.0;
/// How long a turn key is held before it keeps turning by itself.
pub const TURN_HOLD: f32 = 0.45;
/// The speed one paddle stroke adds, metres a second.
pub const STROKE_PUSH: f32 = 1.4;
/// The canoe's top speed.
pub const MAX_SPEED: f32 = 4.0;
/// The slowest backward drift a back-paddle gives.
pub const MIN_SPEED: f32 = -1.2;
/// The time between paddle strokes.
pub const STROKE_GAP: f32 = 0.55;
/// How quickly the canoe glides to a stop (per second).
pub const DRAG: f32 = 0.45;
/// How near a lantern must be to gather it.
pub const GATHER_RADIUS: f32 = 2.5;
/// The island's own size: the canoe bumps against it.
pub const ISLAND_SIZE: f32 = 3.5;
/// How many voices the song can grow by.
pub const SONG_LAYERS: usize = 10;

/// A point on the lake, in metres: x east, y north, the lake's centre at
/// zero.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct P2 {
    pub x: f32,
    pub y: f32,
}

impl P2 {
    pub const ZERO: P2 = P2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> P2 {
        P2 { x, y }
    }

    pub fn len(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn dist(self, other: P2) -> f32 {
        (self - other).len()
    }

    pub fn scale(self, k: f32) -> P2 {
        P2::new(self.x * k, self.y * k)
    }

    /// The point turned `angle` radians clockwise about the centre.
    pub fn rotated(self, angle: f32) -> P2 {
        let (s, c) = angle.sin_cos();
        P2::new(self.x * c + self.y * s, -self.x * s + self.y * c)
    }

    /// The point `radius` from the centre in compass direction `deg`.
    pub fn polar(deg: f32, radius: f32) -> P2 {
        heading_vec(deg).scale(radius)
    }
}

impl std::ops::Add for P2 {
    type Output = P2;
    fn add(self, o: P2) -> P2 {
        P2::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for P2 {
    type Output = P2;
    fn sub(self, o: P2) -> P2 {
        P2::new(self.x - o.x, self.y - o.y)
    }
}

/// The unit step for a compass heading (degrees clockwise from north).
pub fn heading_vec(deg: f32) -> P2 {
    let (s, c) = deg.to_radians().sin_cos();
    P2::new(s, c)
}

/// The compass bearing from `from` to `to`, degrees clockwise from north.
pub fn bearing(from: P2, to: P2) -> f32 {
    let d = to - from;
    d.x.atan2(d.y).to_degrees().rem_euclid(360.0)
}

/// An angle folded into -180..180.
pub fn wrap(deg: f32) -> f32 {
    let a = deg.rem_euclid(360.0);
    if a > 180.0 {
        a - 360.0
    } else {
        a
    }
}

/// Where something is relative to the way you face: `rel` is degrees,
/// negative to the left. "straight ahead", "ahead and to your right, at 2
/// o'clock"...
pub fn direction_words(rel: f32) -> String {
    let rel = wrap(rel);
    let a = rel.abs();
    let side = if rel < 0.0 { "left" } else { "right" };
    let clock = ((rel.rem_euclid(360.0) / 30.0).round() as u32) % 12;
    // Just off straight ahead or right behind still rounds to 12 or 6:
    // name the hour on the side it is.
    let clock = match (clock, rel < 0.0) {
        (0, false) if a > 12.0 && a < 90.0 => 1,
        (0, true) if a > 12.0 && a < 90.0 => 11,
        (0, _) => 12,
        (6, false) if a < 168.0 => 5,
        (6, true) if a < 168.0 => 7,
        (c, _) => c,
    };
    let words = if a <= 12.0 {
        return "straight ahead".to_string();
    } else if a <= 60.0 {
        format!("ahead and to your {side}")
    } else if a <= 120.0 {
        format!("to your {side}")
    } else if a < 168.0 {
        format!("behind you to the {side}")
    } else {
        return "right behind you".to_string();
    };
    format!("{words}, at {clock} o'clock")
}

/// How far something is, in words.
pub fn distance_words(d: f32) -> &'static str {
    if d < 5.0 {
        "very close"
    } else if d < 12.0 {
        "close"
    } else if d < 25.0 {
        "a little way off"
    } else {
        "far across the water"
    }
}

/// The compass point nearest a heading: "north", "south-east"...
pub fn compass(deg: f32) -> &'static str {
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
    POINTS[((deg.rem_euclid(360.0) / 45.0).round() as usize) % 8]
}

/// Left or right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn sign(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }
}

/// Tonight's sky.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Rain,
    Mist,
    Breeze,
}

impl Weather {
    pub const ALL: [Weather; 4] = [
        Weather::Clear,
        Weather::Rain,
        Weather::Mist,
        Weather::Breeze,
    ];

    /// "a clear night"
    pub fn name(self) -> &'static str {
        match self {
            Weather::Clear => "a clear night",
            Weather::Rain => "light rain",
            Weather::Mist => "mist on the water",
            Weather::Breeze => "a soft breeze",
        }
    }

    /// The sentence that opens a night.
    pub fn describe(self) -> &'static str {
        match self {
            Weather::Clear => "The night is clear and still.",
            Weather::Rain => "A light rain patters on the water.",
            Weather::Mist => "Mist lies on the water, and far sounds are hushed.",
            Weather::Breeze => "A soft breeze is blowing, and the lanterns drift a little faster.",
        }
    }

    /// The weather's own quiet bed, if it has one.
    pub fn bed(self) -> Option<&'static str> {
        match self {
            Weather::Clear => None,
            Weather::Rain => Some(cues::RAIN),
            Weather::Mist => Some(cues::MIST),
            Weather::Breeze => Some(cues::BREEZE),
        }
    }

    /// How far sounds carry: mist hushes the far shore.
    pub fn carry(self) -> f32 {
        match self {
            Weather::Mist => 0.55,
            _ => 1.0,
        }
    }
}

/// The creatures that may be out tonight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Creature {
    Frogs,
    Crickets,
    Loon,
    Fish,
}

impl Creature {
    pub const ALL: [Creature; 4] = [
        Creature::Frogs,
        Creature::Crickets,
        Creature::Loon,
        Creature::Fish,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Creature::Frogs => "frogs",
            Creature::Crickets => "crickets",
            Creature::Loon => "a loon",
            Creature::Fish => "jumping fish",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Creature::Frogs => cues::FROGS,
            Creature::Crickets => cues::CRICKETS,
            Creature::Loon => cues::LOON,
            Creature::Fish => cues::FISH,
        }
    }

    /// Frogs and crickets sing all night from one spot on the shore; the
    /// loon and the fish call now and then.
    pub fn loops(self) -> bool {
        matches!(self, Creature::Frogs | Creature::Crickets)
    }
}

/// The small discoveries a lake may hold, remembered in the journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    Dock,
    Island,
    Fireflies,
    Stream,
    LilyPads,
    Buoy,
    Beaver,
    Willow,
}

impl Feature {
    pub const ALL: [Feature; 8] = [
        Feature::Dock,
        Feature::Island,
        Feature::Fireflies,
        Feature::Stream,
        Feature::LilyPads,
        Feature::Buoy,
        Feature::Beaver,
        Feature::Willow,
    ];

    /// The journal's key.
    pub fn id(self) -> &'static str {
        match self {
            Feature::Dock => "dock",
            Feature::Island => "island",
            Feature::Fireflies => "fireflies",
            Feature::Stream => "stream",
            Feature::LilyPads => "lily_pads",
            Feature::Buoy => "buoy",
            Feature::Beaver => "beaver",
            Feature::Willow => "willow",
        }
    }

    /// "the old dock"
    pub fn name(self) -> &'static str {
        match self {
            Feature::Dock => "the old dock",
            Feature::Island => "the owl's island",
            Feature::Fireflies => "the firefly meadow",
            Feature::Stream => "the little stream",
            Feature::LilyPads => "the lily pads",
            Feature::Buoy => "the bell buoy",
            Feature::Beaver => "the beaver lodge",
            Feature::Willow => "the weeping willow",
        }
    }

    /// What finding it is like.
    pub fn found_text(self) -> &'static str {
        match self {
            Feature::Dock => "You drift up to an old wooden dock. Its boards creak, and water slaps softly against the posts.",
            Feature::Island => "Your canoe nudges a small rocky island. High in a pine, an owl watches you and hoots.",
            Feature::Fireflies => "Fireflies rise from a meadow on the shore, hundreds of them, blinking over the water.",
            Feature::Stream => "A little stream tumbles into the lake here, bubbling over stones.",
            Feature::LilyPads => "You glide into a patch of lily pads. A bullfrog grumbles and plops into the water.",
            Feature::Buoy => "An old bell buoy rocks gently, and its bell rings low across the lake.",
            Feature::Beaver => "A beaver lodge of sticks and mud. Something inside gnaws, then slaps its tail on the water.",
            Feature::Willow => "A weeping willow trails its branches in the water, whispering as you pass beneath.",
        }
    }

    /// What it sounds like from afar, for looking around.
    pub fn heard(self) -> &'static str {
        match self {
            Feature::Dock => "wood creaking",
            Feature::Island => "water lapping on rocks",
            Feature::Fireflies => "a faint shimmer",
            Feature::Stream => "running water",
            Feature::LilyPads => "a bullfrog",
            Feature::Buoy => "a low bell",
            Feature::Beaver => "gnawing",
            Feature::Willow => "leaves whispering",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Feature::Dock => cues::DOCK,
            Feature::Island => cues::ISLAND,
            Feature::Fireflies => cues::FIREFLIES,
            Feature::Stream => cues::STREAM,
            Feature::LilyPads => cues::LILY_PADS,
            Feature::Buoy => cues::BUOY,
            Feature::Beaver => cues::BEAVER,
            Feature::Willow => cues::WILLOW,
        }
    }

    /// Whether it sits on the shore (else out on the water).
    pub fn on_shore(self) -> bool {
        !matches!(self, Feature::Island | Feature::LilyPads | Feature::Buoy)
    }

    /// How near the canoe must come to find it.
    pub fn reach(self) -> f32 {
        match self {
            Feature::Island => ISLAND_SIZE + 2.5,
            Feature::Fireflies | Feature::Willow => 6.0,
            _ => 4.5,
        }
    }
}

/// The kind of outing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The small practice pond, with every sound explained.
    Pond,
    /// One of the nights, numbered from 1.
    Night(u32),
    /// An endless lake where new lanterns keep appearing.
    Free,
    /// A challenge, the same lake for everyone this period.
    Challenge(Period),
    /// Lantern Festival night, open only around the festival.
    Festival,
}

impl Mode {
    pub fn name(self) -> String {
        match self {
            Mode::Pond => "The practice pond".to_string(),
            Mode::Night(n) => format!("Night {n}"),
            Mode::Free => "Free paddle".to_string(),
            Mode::Challenge(p) => format!("{} challenge", p.label()),
            Mode::Festival => crate::events::FESTIVAL.to_string(),
        }
    }
}

/// A challenge's special rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Twist {
    Mist,
    Rain,
    StillWater,
    Breeze,
    Starry,
    Constellation,
    Festival,
}

impl Twist {
    /// The daily twists, one picked each day.
    pub const DAILY: [Twist; 5] = [
        Twist::Mist,
        Twist::Rain,
        Twist::StillWater,
        Twist::Breeze,
        Twist::Starry,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::Mist => "Misty night",
            Twist::Rain => "Rainy night",
            Twist::StillWater => "Still water",
            Twist::Breeze => "Breezy night",
            Twist::Starry => "Starry night",
            Twist::Constellation => "Lantern constellation",
            Twist::Festival => "Lantern festival",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Twist::Mist => "Mist hushes the far side of the lake: listen closely, the lanterns sound softer until you are near.",
            Twist::Rain => "Light rain falls the whole night, pattering around you.",
            Twist::StillWater => "Not a breath of wind: the lanterns do not drift at all.",
            Twist::Breeze => "A breeze pushes the lanterns along twice as fast as usual.",
            Twist::Starry => "4 places hide around the lake.",
            Twist::Constellation => "Only one lantern is lit at a time. Gather them in order: each one you gather lights the next, and it rings a brighter chime.",
            Twist::Festival => "A great lake with every creature out, and a slow current that carries you and the lanterns clockwise around it.",
        }
    }
}

/// Everything that decides what a night is like.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub mode: Mode,
    pub radius: f32,
    pub lanterns: usize,
    /// How fast lanterns drift, metres a second.
    pub drift: f32,
    pub weather: Weather,
    pub creatures: Vec<Creature>,
    pub features: Vec<Feature>,
    /// Lanterns must be gathered in order; only the next one is lit.
    pub ordered: bool,
    /// A current turning the whole lake, radians a second clockwise.
    pub current: f32,
    /// New lanterns keep appearing (free paddle).
    pub respawn: bool,
    pub twist: Option<Twist>,
}

fn pick_creatures(rng: &mut Rng, min: usize) -> Vec<Creature> {
    let mut all = Creature::ALL.to_vec();
    rng.shuffle(&mut all);
    let n = rng.range(min as i32, 4) as usize;
    let mut out: Vec<Creature> = all.into_iter().take(n).collect();
    // Keep the shore's order steady, so the same night says the same thing.
    out.sort_by_key(|c| Creature::ALL.iter().position(|a| a == c));
    out
}

fn pick_features(rng: &mut Rng, n: usize) -> Vec<Feature> {
    let mut all = Feature::ALL.to_vec();
    rng.shuffle(&mut all);
    all.into_iter().take(n).collect()
}

impl Plan {
    /// Night `n`: each a little bigger, with more lanterns drifting a
    /// little faster. The first is always clear and gentle.
    pub fn night(n: u32, rng: &mut Rng) -> Plan {
        let n = n.max(1);
        let weather = if n == 1 {
            Weather::Clear
        } else {
            *rng.pick(&Weather::ALL)
        };
        let drift = (0.1 + 0.04 * n as f32).min(0.45);
        Plan {
            mode: Mode::Night(n),
            radius: (20.0 + 3.0 * n as f32).min(44.0),
            lanterns: (3 + n as usize).min(SONG_LAYERS),
            drift: if weather == Weather::Breeze {
                drift * 1.5
            } else {
                drift
            },
            weather,
            creatures: pick_creatures(rng, 2),
            features: pick_features(rng, (1 + n as usize / 2).min(3)),
            ordered: false,
            current: 0.0,
            respawn: false,
            twist: None,
        }
    }

    /// The practice pond: small and still, two lanterns, one dock.
    pub fn pond() -> Plan {
        Plan {
            mode: Mode::Pond,
            radius: 14.0,
            lanterns: 2,
            drift: 0.0,
            weather: Weather::Clear,
            creatures: vec![Creature::Frogs],
            features: vec![Feature::Dock],
            ordered: false,
            current: 0.0,
            respawn: false,
            twist: None,
        }
    }

    /// Free paddle: a wide lake with every creature out, where a new
    /// lantern appears whenever you gather one.
    pub fn free(rng: &mut Rng) -> Plan {
        Plan {
            mode: Mode::Free,
            radius: 40.0,
            lanterns: 5,
            drift: 0.25,
            weather: *rng.pick(&Weather::ALL),
            creatures: Creature::ALL.to_vec(),
            features: pick_features(rng, 3),
            ordered: false,
            current: 0.0,
            respawn: true,
            twist: None,
        }
    }

    /// Lantern Festival night: a wide, clear lake with twice as many
    /// lanterns as the song has voices, and every creature out.
    pub fn festival(rng: &mut Rng) -> Plan {
        Plan {
            mode: Mode::Festival,
            radius: 44.0,
            lanterns: crate::events::FESTIVAL_LANTERNS,
            drift: 0.2,
            weather: Weather::Clear,
            creatures: Creature::ALL.to_vec(),
            features: pick_features(rng, 3),
            ordered: false,
            current: 0.0,
            respawn: false,
            twist: None,
        }
    }

    /// The challenge of `period`, built from its shared seed.
    pub fn challenge(period: Period, rng: &mut Rng) -> Plan {
        match period {
            Period::Daily => {
                let twist = *rng.pick(&Twist::DAILY);
                let weather = match twist {
                    Twist::Mist => Weather::Mist,
                    Twist::Rain => Weather::Rain,
                    Twist::Breeze => Weather::Breeze,
                    _ => Weather::Clear,
                };
                let drift = match twist {
                    Twist::StillWater => 0.0,
                    Twist::Breeze => 0.5,
                    _ => 0.2,
                };
                let features = if twist == Twist::Starry { 4 } else { 2 };
                Plan {
                    mode: Mode::Challenge(period),
                    radius: 28.0,
                    lanterns: 5,
                    drift,
                    weather,
                    creatures: pick_creatures(rng, 2),
                    features: pick_features(rng, features),
                    ordered: false,
                    current: 0.0,
                    respawn: false,
                    twist: Some(twist),
                }
            }
            Period::Weekly => Plan {
                mode: Mode::Challenge(period),
                radius: 36.0,
                lanterns: 8,
                drift: 0.12,
                weather: *rng.pick(&Weather::ALL),
                creatures: pick_creatures(rng, 2),
                features: pick_features(rng, 2),
                ordered: true,
                current: 0.0,
                respawn: false,
                twist: Some(Twist::Constellation),
            },
            Period::Monthly => Plan {
                mode: Mode::Challenge(period),
                radius: 44.0,
                lanterns: SONG_LAYERS,
                drift: 0.2,
                weather: Weather::Clear,
                creatures: Creature::ALL.to_vec(),
                features: pick_features(rng, 4),
                ordered: false,
                current: 0.02,
                respawn: false,
                twist: Some(Twist::Festival),
            },
        }
    }

    /// "5 lanterns, 2 places to discover"
    pub fn summary(&self) -> String {
        let places = match self.features.len() {
            1 => "1 place to discover".to_string(),
            n => format!("{n} places to discover"),
        };
        let lanterns = if self.respawn {
            "lanterns that never run out".to_string()
        } else {
            format!("{} lanterns", self.lanterns)
        };
        format!("{lanterns}, {places}")
    }

    /// "frogs, crickets and a loon"
    pub fn creature_text(&self) -> String {
        let mut names: Vec<&str> = self.creatures.iter().map(|c| c.name()).collect();
        if self.features.contains(&Feature::Island) {
            names.push("an owl");
        }
        match names.len() {
            0 => "nothing stirring".to_string(),
            1 => names[0].to_string(),
            n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
        }
    }
}

/// One floating lantern.
#[derive(Clone, Debug, PartialEq)]
pub struct Lantern {
    pub pos: P2,
    pub vel: P2,
    /// Its place in a constellation, from 1.
    pub order: usize,
    pub gathered: bool,
    /// Which chime it rings, 1 to 3.
    pub voice: usize,
    pub chime_in: f32,
}

impl Lantern {
    pub fn key(&self) -> &'static str {
        cues::CHIMES[(self.voice + 2) % 3]
    }
}

/// A discovery placed on tonight's lake.
#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    pub feature: Feature,
    pub pos: P2,
    pub found: bool,
}

/// A creature placed tonight, with when it next calls.
#[derive(Clone, Debug, PartialEq)]
pub struct Critter {
    pub creature: Creature,
    pub pos: P2,
    pub call_in: f32,
}

/// The keys the lake names in what it says.
#[derive(Clone, Debug, PartialEq)]
pub struct Keys {
    pub left: String,
    pub right: String,
    pub paddle: String,
    pub back: String,
    pub look: String,
    pub home: String,
    pub status: String,
    pub hint: String,
}

impl Default for Keys {
    fn default() -> Keys {
        Keys {
            left: "the Left arrow".into(),
            right: "the Right arrow".into(),
            paddle: "the Up arrow".into(),
            back: "the Down arrow".into(),
            look: "Space".into(),
            home: "Tab".into(),
            status: "S".into(),
            hint: "H".into(),
        }
    }
}

/// Which movement keys are held this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Held {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

/// What the lake asks the screen to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// Speak, interrupting.
    Say(String),
    /// Speak after what is already being said.
    Queue(String),
    /// A sound from the canoe itself.
    Cue(&'static str),
    /// A sound from a place on the lake, at a volume.
    At(&'static str, P2, f32),
    /// A paddle stroke on one side of the canoe.
    Stroke { side: Side, back: bool },
    /// A new voice for the song: 1 to [`SONG_LAYERS`].
    Layer(usize),
    /// The last lantern is in: the song is whole.
    Complete,
}

/// The instrument each song layer adds, for speech.
pub const LAYER_NAMES: [&str; SONG_LAYERS] = [
    "A handpan",
    "A low bass",
    "A glassy pad",
    "A kalimba",
    "A music box",
    "A harp",
    "A flute",
    "A vibraphone",
    "A celesta",
    "A soft choir",
];

/// On Lantern Festival night, temple bells take the choir's place.
pub const TEMPLE_BELLS: &str = "Temple bells";

/// Seconds between the wolf's howls on a full moon night.
pub const WOLF_GAP: f32 = 45.0;

/// One night on the lake.
pub struct Lake {
    pub plan: Plan,
    rng: Rng,
    pub pos: P2,
    /// Degrees clockwise from north.
    pub heading: f32,
    /// Metres a second, negative when backing up.
    pub speed: f32,
    pub lanterns: Vec<Lantern>,
    pub gathered: usize,
    pub spots: Vec<Spot>,
    pub island: Option<P2>,
    pub critters: Vec<Critter>,
    /// Where the wind in the reeds comes from.
    pub reeds: P2,
    pub keys: Keys,
    pub last: String,
    pub complete: bool,
    pub strokes: u32,
    pub time: f64,
    /// Discoveries made since the screen last asked.
    pub new_finds: Vec<Feature>,
    notes: Vec<Note>,
    stroke_cd: f32,
    stroke_queued: Option<bool>,
    next_side: Side,
    turn_held: f32,
    at_shore: bool,
    bumped: bool,
    asleep_near: Option<usize>,
    /// Tonight's full moon by name, on an ordinary night or a free paddle.
    pub moon: Option<&'static str>,
    wolf_in: f32,
    wolf_named: bool,
}

/// A random point inside a circle of `radius`.
fn in_disk(rng: &mut Rng, radius: f32) -> P2 {
    let r = (rng.unit() as f32).sqrt() * radius;
    P2::polar(rng.unit() as f32 * 360.0, r)
}

impl Lake {
    /// A new night from its plan. The canoe starts near the south shore,
    /// facing north across the lake.
    pub fn new(plan: Plan, mut rng: Rng) -> Lake {
        let radius = plan.radius;
        let start = P2::new(0.0, -(radius - 3.0));
        // The island first, so lanterns keep clear of it.
        let island = plan.features.contains(&Feature::Island).then(|| {
            let mut best = P2::new(0.0, radius * 0.3);
            for _ in 0..100 {
                let p = in_disk(&mut rng, radius * 0.5);
                if p.dist(start) > 10.0 {
                    best = p;
                    break;
                }
            }
            best
        });
        let mut taken: Vec<P2> = island.into_iter().collect();
        let mut spots = Vec::new();
        let mut shore_angles: Vec<f32> = vec![180.0];
        for &feature in &plan.features {
            let pos = if feature == Feature::Island {
                island.unwrap_or(P2::ZERO)
            } else if feature.on_shore() {
                let mut angle = rng.unit() as f32 * 360.0;
                for _ in 0..60 {
                    if shore_angles.iter().all(|a| wrap(angle - a).abs() >= 40.0) {
                        break;
                    }
                    angle = rng.unit() as f32 * 360.0;
                }
                shore_angles.push(angle);
                P2::polar(angle, radius - 2.0)
            } else {
                let mut p = in_disk(&mut rng, radius - 6.0);
                for _ in 0..100 {
                    if p.dist(start) > 8.0 && taken.iter().all(|t| t.dist(p) > 8.0) {
                        break;
                    }
                    p = in_disk(&mut rng, radius - 6.0);
                }
                p
            };
            taken.push(pos);
            spots.push(Spot {
                feature,
                pos,
                found: false,
            });
        }
        let critters = plan
            .creatures
            .iter()
            .enumerate()
            .map(|(i, &creature)| {
                let angle = rng.unit() as f32 * 360.0;
                Critter {
                    creature,
                    pos: P2::polar(angle, radius + 1.0),
                    call_in: 6.0 + 5.0 * i as f32 + rng.unit() as f32 * 10.0,
                }
            })
            .collect();
        let reeds = P2::polar(rng.unit() as f32 * 360.0, radius + 1.0);
        let mut lake = Lake {
            plan,
            rng,
            pos: start,
            heading: 0.0,
            speed: 0.0,
            lanterns: Vec::new(),
            gathered: 0,
            spots,
            island,
            critters,
            reeds,
            keys: Keys::default(),
            last: String::new(),
            complete: false,
            strokes: 0,
            time: 0.0,
            new_finds: Vec::new(),
            notes: Vec::new(),
            stroke_cd: 0.0,
            stroke_queued: None,
            next_side: Side::Right,
            turn_held: 0.0,
            at_shore: false,
            bumped: false,
            asleep_near: None,
            moon: None,
            wolf_in: 14.0,
            wolf_named: false,
        };
        if lake.plan.mode == Mode::Pond {
            // The pond's two lanterns are placed to teach: one ahead, one
            // off to the right.
            lake.push_lantern_at(P2::new(0.0, -2.0));
            lake.push_lantern_at(P2::new(7.0, 4.0));
        } else {
            for _ in 0..lake.plan.lanterns {
                lake.spawn_lantern();
            }
        }
        if lake.plan.ordered {
            // A constellation runs round the lake from the start.
            let mut idx: Vec<usize> = (0..lake.lanterns.len()).collect();
            idx.sort_by(|&a, &b| {
                let ka = bearing(P2::ZERO, lake.lanterns[a].pos);
                let kb = bearing(P2::ZERO, lake.lanterns[b].pos);
                ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
            });
            for (order, i) in idx.into_iter().enumerate() {
                lake.lanterns[i].order = order + 1;
            }
        }
        lake
    }

    fn push_lantern_at(&mut self, pos: P2) {
        let n = self.lanterns.len();
        let dir = self.rng.unit() as f32 * 360.0;
        self.lanterns.push(Lantern {
            pos,
            vel: heading_vec(dir).scale(self.plan.drift),
            order: n + 1,
            gathered: false,
            voice: n % 3 + 1,
            chime_in: 0.6 + 0.7 * (n % 5) as f32,
        });
    }

    fn spawn_lantern(&mut self) {
        let radius = self.plan.radius - 5.0;
        let mut best = in_disk(&mut self.rng, radius);
        for _ in 0..200 {
            let p = in_disk(&mut self.rng, radius);
            let clear = p.dist(self.pos) >= 8.0
                && self
                    .lanterns
                    .iter()
                    .filter(|l| !l.gathered)
                    .all(|l| l.pos.dist(p) >= 5.0)
                && self.island.is_none_or(|i| i.dist(p) >= ISLAND_SIZE + 4.0);
            best = p;
            if clear {
                break;
            }
        }
        self.push_lantern_at(best);
    }

    /// A full moon over the lake: a wolf howls now and then from far off.
    pub fn set_moon(&mut self, moon: Option<&'static str>) {
        self.moon = moon;
    }

    /// Where the wolf howls from: well beyond the north east shore.
    pub fn wolf_pos(&self) -> P2 {
        P2::polar(45.0, self.plan.radius + 8.0)
    }

    /// The voice that joins the song at `layer`, for speech.
    pub fn layer_name(&self, layer: usize) -> &'static str {
        if self.plan.mode == Mode::Festival && layer == SONG_LAYERS {
            TEMPLE_BELLS
        } else {
            LAYER_NAMES[layer.clamp(1, SONG_LAYERS) - 1]
        }
    }

    fn howl(&mut self, dt: f32) {
        let Some(moon) = self.moon else {
            return;
        };
        self.wolf_in -= dt;
        if self.wolf_in > 0.0 {
            return;
        }
        self.wolf_in = WOLF_GAP;
        let at = self.wolf_pos();
        self.notes.push(Note::At(cues::WOLF, at, 0.5));
        if !self.wolf_named {
            self.wolf_named = true;
            self.notes.push(Note::Queue(format!(
                "A wolf howls far off, under {moon}. Nothing to find."
            )));
        }
    }

    /// The notes left since the last call.
    pub fn drain_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        self.notes.push(Note::Say(text));
    }

    /// Says `text`, or queues it when something was already said this
    /// step.
    fn tell(&mut self, spoke: &mut bool, text: impl Into<String>) {
        if *spoke {
            self.queue(text);
        } else {
            self.say(text);
        }
        *spoke = true;
    }

    fn queue(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        self.notes.push(Note::Queue(text));
    }

    /// The lanterns still on the water.
    pub fn remaining(&self) -> usize {
        self.lanterns.iter().filter(|l| !l.gathered).count()
    }

    /// The lantern number that is lit next in a constellation.
    pub fn next_order(&self) -> usize {
        self.lanterns
            .iter()
            .filter(|l| !l.gathered)
            .map(|l| l.order)
            .min()
            .unwrap_or(0)
    }

    /// Whether a lantern is lit and ringing.
    pub fn lit(&self, i: usize) -> bool {
        let l = &self.lanterns[i];
        !l.gathered && (!self.plan.ordered || l.order == self.next_order())
    }

    /// The lantern to head for: the lit one in a constellation, else the
    /// nearest.
    pub fn target(&self) -> Option<usize> {
        (0..self.lanterns.len())
            .filter(|&i| self.lit(i))
            .min_by(|&a, &b| {
                let da = self.lanterns[a].pos.dist(self.pos);
                let db = self.lanterns[b].pos.dist(self.pos);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Discoveries found tonight.
    pub fn found(&self) -> usize {
        self.spots.iter().filter(|s| s.found).count()
    }

    /// Tonight's gentle score: a lantern is 1 and a discovery 2.
    pub fn score(&self) -> u64 {
        self.gathered as u64 + 2 * self.found() as u64
    }

    /// Where something is from the canoe, in words.
    pub fn where_is(&self, p: P2) -> String {
        let rel = bearing(self.pos, p) - self.heading;
        format!(
            "{}, {}",
            direction_words(rel),
            distance_words(self.pos.dist(p))
        )
    }

    /// How a lantern's sound comes back to the canoe: its chime rings
    /// faster as you near it.
    pub fn chime_gap(d: f32) -> f32 {
        if d < 8.0 {
            1.2
        } else if d < 20.0 {
            2.4
        } else {
            3.6
        }
    }

    /// Tap Left or Right: a small turn now, and holding keeps turning.
    pub fn turn(&mut self, side: Side) {
        self.heading = (self.heading + side.sign() * TAP_TURN).rem_euclid(360.0);
        self.turn_held = 0.0;
        self.notes.push(Note::Cue(cues::TURN));
    }

    /// One paddle stroke forward, or queued for when the last one ends.
    pub fn paddle(&mut self) {
        if self.stroke_cd > 0.0 {
            self.stroke_queued = Some(false);
        } else {
            self.stroke(false);
        }
    }

    /// One stroke backward: slows the canoe, then backs it up.
    pub fn back_paddle(&mut self) {
        if self.stroke_cd > 0.0 {
            self.stroke_queued = Some(true);
        } else {
            self.stroke(true);
        }
    }

    fn stroke(&mut self, back: bool) {
        self.stroke_cd = STROKE_GAP;
        self.stroke_queued = None;
        self.strokes += 1;
        if back {
            self.speed = (self.speed - STROKE_PUSH).max(MIN_SPEED);
        } else {
            self.speed = (self.speed + STROKE_PUSH).min(MAX_SPEED);
        }
        let side = self.next_side;
        self.next_side = match side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        };
        self.notes.push(Note::Stroke { side, back });
    }

    /// Every lit lantern rings once now, nearest first, and the lake is
    /// described.
    pub fn look(&mut self) {
        let mut text = format!("You face {}.", compass(self.heading));
        let shore = self.plan.radius - self.pos.len();
        let shore_dir = if self.pos.len() < 0.5 {
            "all around you".to_string()
        } else {
            direction_words(bearing(P2::ZERO, self.pos) - self.heading)
        };
        text.push_str(&format!(
            " The nearest shore is {shore_dir}, {}.",
            distance_words(shore)
        ));
        let carry = 30.0 * self.plan.weather.carry();
        let mut heard = Vec::new();
        for s in &self.spots {
            if s.pos.dist(self.pos) < carry {
                let what = if s.found {
                    s.feature.name()
                } else {
                    s.feature.heard()
                };
                heard.push(format!(
                    "{what} {}",
                    direction_words(bearing(self.pos, s.pos) - self.heading)
                ));
            }
        }
        for c in &self.critters {
            if c.creature.loops() {
                heard.push(format!(
                    "{} {}",
                    c.creature.name(),
                    direction_words(bearing(self.pos, c.pos) - self.heading)
                ));
            }
        }
        if !heard.is_empty() {
            text.push_str(&format!(" You hear {}.", heard.join("; ")));
        }
        let lit = self.lit_count();
        match lit {
            0 => {}
            1 => text.push_str(" Listen: the lit lantern rings."),
            _ => text.push_str(" Listen: each lantern rings once, nearest first."),
        }
        // The nearest ring first, half a second apart.
        let mut order: Vec<usize> = (0..self.lanterns.len()).filter(|&i| self.lit(i)).collect();
        order.sort_by(|&a, &b| {
            let da = self.lanterns[a].pos.dist(self.pos);
            let db = self.lanterns[b].pos.dist(self.pos);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        for (k, i) in order.into_iter().enumerate() {
            self.lanterns[i].chime_in = 0.4 + 0.6 * k as f32;
        }
        self.say(text);
    }

    fn lit_count(&self) -> usize {
        (0..self.lanterns.len()).filter(|&i| self.lit(i)).count()
    }

    /// The S key: lanterns, where the nearest is, and the night.
    /// The status key, in layers. First: lanterns gathered and where the
    /// next one is. A second press: which way you face and move, the
    /// weather, and the night's discoveries.
    pub fn status_layers(&self) -> Vec<String> {
        let mut first = vec![if self.plan.respawn {
            format!("{} lanterns gathered tonight.", self.gathered)
        } else {
            format!(
                "{} of {} lanterns gathered.",
                self.gathered, self.plan.lanterns
            )
        }];
        match self.target() {
            Some(i) => {
                let l = &self.lanterns[i];
                let which = if self.plan.ordered {
                    format!("Lantern {} is lit", l.order)
                } else {
                    "The nearest lantern is".to_string()
                };
                first.push(format!("{which} {}.", self.where_is(l.pos)));
            }
            None => first.push("The song is whole.".to_string()),
        }
        let moving = if self.speed > 2.5 {
            "gliding along"
        } else if self.speed > 0.3 {
            "drifting forward"
        } else if self.speed < -0.3 {
            "drifting backward"
        } else {
            "still"
        };
        let mut more = vec![format!(
            "You face {}, {moving}. {}.",
            compass(self.heading),
            sentence(self.plan.weather.name())
        )];
        if let Some(m) = self.moon {
            more.push(format!(
                "{} is full: a wolf howls far off now and then.",
                sentence(m)
            ));
        }
        let found = self.found();
        if !self.spots.is_empty() {
            more.push(format!(
                "Discoveries tonight: {found} of {}.",
                self.spots.len()
            ));
        }
        vec![
            parlor_core::status::sentences(first),
            parlor_core::status::sentences(more),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        parlor_core::status::sentences(self.status_layers())
    }

    /// The H key: which way to turn for the next lantern, then a ring of
    /// its chime.
    pub fn hint(&mut self) -> String {
        let text = match self.target() {
            Some(i) => {
                let l = self.lanterns[i].pos;
                let rel = wrap(bearing(self.pos, l) - self.heading);
                let turn = if rel.abs() <= 12.0 {
                    format!("It is straight ahead: paddle with {}.", self.keys.paddle)
                } else if rel.abs() >= 150.0 {
                    format!(
                        "It is behind you: hold {} or {} to turn around.",
                        self.keys.left, self.keys.right
                    )
                } else {
                    let (key, amount) = if rel < 0.0 {
                        (&self.keys.left, -rel)
                    } else {
                        (&self.keys.right, rel)
                    };
                    let how = if amount < 45.0 { "a little" } else { "well" };
                    format!("Turn {how} with {key} until its chime is straight ahead, then paddle.")
                };
                self.lanterns[i].chime_in = 0.3;
                let near = distance_words(self.pos.dist(l));
                format!("{turn} The lantern is {near}.")
            }
            None => format!("Every lantern in. {} heads home.", self.keys.home),
        };
        self.last = text.clone();
        text
    }

    /// The opening words of the night, said at push-off. The page just
    /// before it named the night, so this does not say it again.
    pub fn intro(&self) -> String {
        let challenge = matches!(self.plan.mode, Mode::Challenge(_));
        let weather = if challenge && self.plan.weather != Weather::Clear {
            String::new()
        } else {
            format!("{} ", self.plan.weather.describe())
        };
        let lanterns = if self.plan.ordered {
            "Lantern 1 is ringing. ".to_string()
        } else if matches!(self.plan.mode, Mode::Night(_)) {
            format!("{} lanterns drift on the water. ", self.plan.lanterns)
        } else {
            String::new()
        };
        let moon = match self.moon {
            Some(m) => format!("{} is full tonight. ", sentence(m)),
            None => String::new(),
        };
        format!(
            "{weather}{moon}Out tonight: {}. {lanterns}Turn with {} and {}, paddle with {}. {} looks around.",
            self.plan.creature_text(),
            self.keys.left,
            self.keys.right,
            self.keys.paddle,
            sentence(&self.keys.look),
        )
    }

    /// One step of the night.
    pub fn update(&mut self, dt: f32, held: Held) {
        if dt <= 0.0 {
            return;
        }
        self.time += dt as f64;
        self.steer(dt, held);
        self.drift(dt);
        self.move_canoe(dt, held);
        self.chime(dt);
        self.gather();
        self.discover();
        self.calls(dt);
        self.howl(dt);
    }

    fn steer(&mut self, dt: f32, held: Held) {
        let dir = match (held.left, held.right) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };
        if dir == 0.0 {
            self.turn_held = 0.0;
            return;
        }
        self.turn_held += dt;
        if self.turn_held > TURN_HOLD {
            self.heading = (self.heading + dir * TURN_RATE * dt).rem_euclid(360.0);
        }
    }

    fn drift(&mut self, dt: f32) {
        let limit = self.plan.radius - 4.0;
        let current = self.plan.current * dt;
        for l in self.lanterns.iter_mut().filter(|l| !l.gathered) {
            l.pos = l.pos + l.vel.scale(dt);
            if current != 0.0 {
                l.pos = l.pos.rotated(current);
            }
            let r = l.pos.len();
            if r > limit {
                // Turn back toward open water.
                let n = l.pos.scale(1.0 / r);
                let dot = l.vel.x * n.x + l.vel.y * n.y;
                if dot > 0.0 {
                    l.vel = l.vel - n.scale(2.0 * dot);
                }
                l.pos = n.scale(limit);
            }
            if let Some(i) = self.island {
                let d = l.pos.dist(i);
                if d < ISLAND_SIZE + 1.0 && d > 0.0 {
                    let n = (l.pos - i).scale(1.0 / d);
                    l.pos = i + n.scale(ISLAND_SIZE + 1.0);
                    l.vel = n.scale(l.vel.len());
                }
            }
        }
        if current != 0.0 {
            self.pos = self.pos.rotated(current);
        }
    }

    fn move_canoe(&mut self, dt: f32, held: Held) {
        self.stroke_cd -= dt;
        if self.stroke_cd <= 0.0 {
            if let Some(back) = self.stroke_queued {
                self.stroke(back);
            } else if held.up && !held.down {
                self.stroke(false);
            } else if held.down && !held.up {
                self.stroke(true);
            }
        }
        self.speed *= (-DRAG * dt).exp();
        if self.speed.abs() < 0.02 {
            self.speed = 0.0;
        }
        self.pos = self.pos + heading_vec(self.heading).scale(self.speed * dt);
        let r = self.pos.len();
        let edge = self.plan.radius - 1.0;
        if r > edge {
            self.pos = self.pos.scale(edge / r);
            self.speed = 0.0;
            if !self.at_shore {
                self.at_shore = true;
                self.notes.push(Note::Cue(cues::REEDS));
                // Once the song is whole the night is ending: the rustle
                // is enough, and no steering advice follows it.
                if !self.complete {
                    self.say(format!(
                        "Reeds: the shore. Turn with {} or {}.",
                        self.keys.left, self.keys.right
                    ));
                }
            }
        } else if r < edge - 2.0 {
            self.at_shore = false;
        }
        if let Some(i) = self.island {
            let d = self.pos.dist(i);
            if d < ISLAND_SIZE {
                let n = if d > 0.0 {
                    (self.pos - i).scale(1.0 / d)
                } else {
                    P2::new(0.0, -1.0)
                };
                self.pos = i + n.scale(ISLAND_SIZE);
                self.speed = 0.0;
                if !self.bumped {
                    self.bumped = true;
                    self.notes.push(Note::Cue(cues::BUMP));
                    self.queue("Bumped the island.");
                }
            } else if d > ISLAND_SIZE + 2.0 {
                self.bumped = false;
            }
        }
    }

    fn chime(&mut self, dt: f32) {
        let carry = self.plan.weather.carry();
        let ordered = self.plan.ordered;
        for i in 0..self.lanterns.len() {
            if !self.lit(i) {
                continue;
            }
            let pos = self.pos;
            let l = &mut self.lanterns[i];
            l.chime_in -= dt;
            if l.chime_in <= 0.0 {
                let d = l.pos.dist(pos);
                l.chime_in += Lake::chime_gap(d);
                if l.chime_in <= 0.0 {
                    l.chime_in = Lake::chime_gap(d);
                }
                let key = if ordered { cues::BEACON } else { l.key() };
                // Mist hushes only what is far off.
                let vol = if d > 12.0 { carry } else { 1.0 };
                let at = l.pos;
                self.notes.push(Note::At(key, at, vol));
            }
        }
    }

    fn gather(&mut self) {
        let mut near_asleep = None;
        // Two lanterns met in one moment: the second is said after the
        // first instead of cutting it off.
        let mut spoke = false;
        for i in 0..self.lanterns.len() {
            let l = &self.lanterns[i];
            if l.gathered || l.pos.dist(self.pos) > GATHER_RADIUS {
                continue;
            }
            if !self.lit(i) {
                near_asleep = Some(i);
                if self.asleep_near != Some(i) {
                    let (order, at) = (l.order, l.pos);
                    let next = self.next_order();
                    self.notes.push(Note::At(cues::ASLEEP, at, 1.0));
                    self.tell(
                        &mut spoke,
                        format!("Sleeping lantern {order}. Lantern {next} is lit."),
                    );
                }
                continue;
            }
            self.lanterns[i].gathered = true;
            self.gathered += 1;
            let at = self.lanterns[i].pos;
            self.notes.push(Note::At(cues::GATHER, at, 1.0));
            // On festival night a voice joins with every second lantern.
            let voice = if self.plan.mode == Mode::Festival {
                self.gathered.is_multiple_of(2).then_some(self.gathered / 2)
            } else {
                Some(self.gathered)
            };
            let joins = match voice {
                Some(layer) if layer <= SONG_LAYERS => {
                    self.notes.push(Note::Layer(layer));
                    format!(" {} joins the song.", self.layer_name(layer))
                }
                None => " One more, and a new voice joins the song.".to_string(),
                Some(_) => " The song glows a little brighter.".to_string(),
            };
            if self.plan.respawn {
                self.tell(
                    &mut spoke,
                    format!("Lantern gathered, {} tonight.{joins}", self.gathered),
                );
                self.spawn_lantern();
            } else if self.remaining() == 0 {
                self.complete = true;
                self.notes.push(Note::Complete);
                self.tell(
                    &mut spoke,
                    format!(
                        "The last lantern. {} of {}.{joins} The song is whole.",
                        self.gathered, self.plan.lanterns
                    ),
                );
            } else {
                let left = self.remaining();
                let rest = if self.plan.ordered {
                    format!(" Lantern {} is lit now.", self.next_order())
                } else if left == 1 {
                    " One more drifts out there.".to_string()
                } else {
                    format!(" {left} more drift out there.")
                };
                self.tell(
                    &mut spoke,
                    format!(
                        "Lantern gathered, {} of {}.{joins}{rest}",
                        self.gathered, self.plan.lanterns
                    ),
                );
            }
        }
        self.asleep_near = near_asleep;
    }

    fn discover(&mut self) {
        for i in 0..self.spots.len() {
            let s = &self.spots[i];
            if s.found || s.pos.dist(self.pos) > s.feature.reach() {
                continue;
            }
            let (feature, at) = (s.feature, s.pos);
            self.spots[i].found = true;
            self.new_finds.push(feature);
            self.notes.push(Note::At(cues::FOUND, at, 1.0));
            if feature == Feature::Island {
                self.notes.push(Note::At(cues::OWL, at, 1.0));
            }
            self.queue(format!("A discovery. {}", feature.found_text()));
        }
    }

    fn calls(&mut self, dt: f32) {
        let carry = self.plan.weather.carry();
        let radius = self.plan.radius;
        for k in 0..self.critters.len() {
            let c = &mut self.critters[k];
            if c.creature.loops() {
                continue;
            }
            c.call_in -= dt;
            if c.call_in > 0.0 {
                continue;
            }
            let creature = c.creature;
            let (gap, pos) = match creature {
                Creature::Loon => {
                    let p = P2::polar(self.rng.unit() as f32 * 360.0, radius + 2.0);
                    (25.0 + self.rng.unit() as f32 * 20.0, p)
                }
                _ => {
                    let p = in_disk(&mut self.rng, radius - 3.0);
                    (12.0 + self.rng.unit() as f32 * 14.0, p)
                }
            };
            self.critters[k].call_in = gap;
            self.critters[k].pos = pos;
            self.notes.push(Note::At(creature.key(), pos, 0.8 * carry));
        }
        // The owl lives on the island, found or not.
        if let Some(i) = self.island {
            let t = self.time;
            let before = t - dt as f64;
            if (t / 20.0).floor() != (before / 20.0).floor() {
                self.notes.push(Note::At(cues::OWL, i, 0.8 * carry));
            }
        }
    }
}

/// "a clear night" as the start of a sentence.
pub fn sentence(text: &str) -> String {
    let mut c = text.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
