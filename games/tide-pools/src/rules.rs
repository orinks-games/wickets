//! The rules of an outing, with no sound or speech: a strip of rock pools,
//! what lives in each, the player's place and lean, shy creatures that
//! hide and creep back out, and the tide that ends the trip.
//!
//! Everything advances by `update(dt)` and by the player's moves, each
//! returning the [`Ev`]s the screen voices, so tests can drive it exactly.

use std::cell::Cell;

use parlor_core::{Period, Rng};

pub use crate::events::Event;

/// The pools a king tide opens at each end of the shore.
pub const KING_TIDE_EXTRA: usize = 2;

/// How long after a creature's sound a gather still catches it, in seconds.
pub const GATHER_WINDOW: f64 = 3.5;

/// Metres between neighbouring pools, as the screen places them.
pub const POOL_SPACING: f32 = 3.0;

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins the seed of every new outing on this thread, for tests. `None`
/// goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A fresh random source: the pinned seed in tests, else the clock.
pub fn fresh_rng() -> Rng {
    match FIXED_SEED.with(|s| s.get()) {
        Some(seed) => Rng::seeded(seed),
        None => Rng::from_time(),
    }
}

/// Everything that can live in a pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Species {
    Crab,
    Periwinkle,
    Starfish,
    Anemone,
    Goby,
    Hermit,
    Shrimp,
    Mussel,
    Cowrie,
    Seahorse,
    Octopus,
    Jelly,
    SandDollar,
    SeaSlug,
}

impl Species {
    pub const ALL: [Species; 14] = [
        Species::Crab,
        Species::Periwinkle,
        Species::Starfish,
        Species::Anemone,
        Species::Goby,
        Species::Hermit,
        Species::Shrimp,
        Species::Mussel,
        Species::Cowrie,
        Species::Seahorse,
        Species::Octopus,
        Species::Jelly,
        Species::SandDollar,
        Species::SeaSlug,
    ];

    pub const COMMON: [Species; 8] = [
        Species::Crab,
        Species::Periwinkle,
        Species::Starfish,
        Species::Anemone,
        Species::Goby,
        Species::Hermit,
        Species::Shrimp,
        Species::Mussel,
    ];

    pub const RARE: [Species; 6] = [
        Species::Cowrie,
        Species::Seahorse,
        Species::Octopus,
        Species::Jelly,
        Species::SandDollar,
        Species::SeaSlug,
    ];

    /// A stable id for saved data and sound keys.
    pub fn id(self) -> &'static str {
        match self {
            Species::Crab => "crab",
            Species::Periwinkle => "periwinkle",
            Species::Starfish => "starfish",
            Species::Anemone => "anemone",
            Species::Goby => "goby",
            Species::Hermit => "hermit",
            Species::Shrimp => "shrimp",
            Species::Mussel => "mussel",
            Species::Cowrie => "cowrie",
            Species::Seahorse => "seahorse",
            Species::Octopus => "octopus",
            Species::Jelly => "jelly",
            Species::SandDollar => "sanddollar",
            Species::SeaSlug => "seaslug",
        }
    }

    pub fn from_id(id: &str) -> Option<Species> {
        Species::ALL.into_iter().find(|s| s.id() == id)
    }

    /// "shore crab".
    pub fn name(self) -> &'static str {
        match self {
            Species::Crab => "shore crab",
            Species::Periwinkle => "periwinkle shell",
            Species::Starfish => "starfish",
            Species::Anemone => "sea anemone",
            Species::Goby => "little goby fish",
            Species::Hermit => "hermit crab",
            Species::Shrimp => "rock shrimp",
            Species::Mussel => "mussel",
            Species::Cowrie => "cowrie shell",
            Species::Seahorse => "seahorse",
            Species::Octopus => "baby octopus",
            Species::Jelly => "moon jellyfish",
            Species::SandDollar => "sand dollar",
            Species::SeaSlug => "sea slug",
        }
    }

    /// "shore crabs", for counts above one.
    pub fn plural(self) -> String {
        match self {
            Species::Starfish => "starfish".to_string(),
            Species::Goby => "little goby fish".to_string(),
            Species::Octopus => "baby octopuses".to_string(),
            Species::Jelly => "moon jellyfish".to_string(),
            other => format!("{}s", other.name()),
        }
    }

    /// "1 shore crab", "3 starfish".
    pub fn count_words(self, n: u32) -> String {
        if n == 1 {
            format!("1 {}", self.name())
        } else {
            format!("{n} {}", self.plural())
        }
    }

    /// What its sound is like, in words.
    pub fn sound_words(self) -> &'static str {
        match self {
            Species::Crab => "a quick skitter of clicking claws",
            Species::Periwinkle => "a little shell rolling and clattering over pebbles",
            Species::Starfish => "a slow, soft two-note twinkle",
            Species::Anemone => "a soft, low, wet pop",
            Species::Goby => "a bright little splash, a tail flicking",
            Species::Hermit => "three wooden taps of a borrowed shell on rock",
            Species::Shrimp => "two sharp snaps",
            Species::Mussel => "a string of small bubbles",
            Species::Cowrie => "a glossy shell rolling with a glassy ring",
            Species::Seahorse => "a tiny, breathy rising whistle",
            Species::Octopus => "a low gurgle and a squirt of ink",
            Species::Jelly => "a slow, soft pulse, like a breath underwater",
            Species::SandDollar => "a hollow tap and two low, round notes",
            Species::SeaSlug => "a slow, wobbling squelch",
        }
    }

    /// How long it must hear nothing of you before it comes back out, in
    /// seconds, before the beach's own scale. `None` is not shy at all.
    pub fn shy(self) -> Option<f64> {
        match self {
            Species::Crab => Some(3.0),
            Species::Goby => Some(4.0),
            Species::Hermit => Some(3.0),
            Species::Shrimp => Some(3.5),
            Species::Seahorse => Some(4.5),
            Species::Octopus => Some(5.0),
            _ => None,
        }
    }

    pub fn rare(self) -> bool {
        Species::RARE.contains(&self)
    }

    /// Points for gathering one.
    pub fn points(self) -> u64 {
        match self {
            Species::Periwinkle | Species::Anemone | Species::Mussel => 5,
            Species::Starfish => 10,
            Species::Crab | Species::Hermit | Species::Shrimp => 15,
            Species::Goby => 20,
            _ => 50,
        }
    }

    /// For a rare find, when it can turn up, as a phrase: "on a spring tide".
    pub fn when(self) -> Option<&'static str> {
        match self {
            Species::Cowrie => Some("on a spring tide, the lowest of the month"),
            Species::Seahorse => Some("on a sunny day"),
            Species::Octopus => Some("at dusk"),
            Species::Jelly => Some("when it is breezy and the wind blows them in"),
            Species::SandDollar => Some("in fog"),
            Species::SeaSlug => Some("in drizzle"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Sunny,
    Breezy,
    Foggy,
    Drizzle,
}

impl Weather {
    pub const ALL: [Weather; 4] = [
        Weather::Sunny,
        Weather::Breezy,
        Weather::Foggy,
        Weather::Drizzle,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Weather::Sunny => "sunny",
            Weather::Breezy => "breezy",
            Weather::Foggy => "foggy",
            Weather::Drizzle => "drizzly",
        }
    }

    /// The word for a night trip: a sunny sky is a clear one at night.
    pub fn night_word(self) -> &'static str {
        match self {
            Weather::Sunny => "clear",
            w => w.word(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tide {
    /// A small tide: the water does not go far out, so the trip is short.
    Neap,
    Ordinary,
    /// The lowest tide of the month: a longer trip and cowries.
    Spring,
}

impl Tide {
    pub fn words(self) -> &'static str {
        match self {
            Tide::Neap => "a small neap tide",
            Tide::Ordinary => "an ordinary low tide",
            Tide::Spring => "a spring tide, the lowest of the month",
        }
    }

    /// Seconds the tide adds to or takes from a trip.
    pub fn extra(self) -> f64 {
        match self {
            Tide::Neap => -30.0,
            Tide::Ordinary => 0.0,
            Tide::Spring => 45.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeOfDay {
    Morning,
    Afternoon,
    Dusk,
    /// Only on the moonlit pools, under a full moon.
    Night,
}

impl TimeOfDay {
    pub fn word(self) -> &'static str {
        match self {
            TimeOfDay::Morning => "morning",
            TimeOfDay::Afternoon => "afternoon",
            TimeOfDay::Dusk => "evening at dusk",
            TimeOfDay::Night => "night",
        }
    }
}

/// The weather, tide and time of one outing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conditions {
    pub weather: Weather,
    pub tide: Tide,
    pub time: TimeOfDay,
}

impl Conditions {
    pub fn roll(rng: &mut Rng) -> Conditions {
        let weather = *rng.pick(&Weather::ALL);
        let tide = match rng.below(10) {
            0..=1 => Tide::Neap,
            2..=7 => Tide::Ordinary,
            _ => Tide::Spring,
        };
        let time = *rng.pick(&[TimeOfDay::Morning, TimeOfDay::Afternoon, TimeOfDay::Dusk]);
        Conditions {
            weather,
            tide,
            time,
        }
    }

    /// "A sunny afternoon, on an ordinary low tide."
    pub fn describe(&self) -> String {
        if self.time == TimeOfDay::Night {
            return format!(
                "A {} night under the full moon, on {}.",
                self.weather.night_word(),
                self.tide.words()
            );
        }
        format!(
            "A {} {}, on {}.",
            self.weather.word(),
            self.time.word(),
            self.tide.words()
        )
    }

    /// "sunny afternoon", for the aquarium's notes.
    pub fn short(&self) -> String {
        if self.time == TimeOfDay::Night {
            return format!("{} moonlit night", self.weather.night_word());
        }
        format!("{} {}", self.weather.word(), self.time.word())
    }

    /// Whether a rare species can turn up today.
    pub fn allows(&self, species: Species) -> bool {
        match species {
            Species::Cowrie => self.tide == Tide::Spring,
            Species::Seahorse => self.weather == Weather::Sunny && self.time != TimeOfDay::Night,
            Species::Octopus => matches!(self.time, TimeOfDay::Dusk | TimeOfDay::Night),
            Species::Jelly => self.weather == Weather::Breezy || self.time == TimeOfDay::Night,
            Species::SandDollar => self.weather == Weather::Foggy,
            Species::SeaSlug => self.weather == Weather::Drizzle,
            _ => true,
        }
    }

    /// The rare species these conditions allow.
    pub fn rares(&self) -> Vec<Species> {
        Species::RARE
            .into_iter()
            .filter(|s| self.allows(*s))
            .collect()
    }

    /// Conditions that let `species` appear, otherwise rolled.
    pub fn allowing(species: Species, rng: &mut Rng) -> Conditions {
        let mut c = Conditions::roll(rng);
        match species {
            Species::Cowrie => c.tide = Tide::Spring,
            Species::Seahorse => c.weather = Weather::Sunny,
            Species::Octopus => c.time = TimeOfDay::Dusk,
            Species::Jelly => c.weather = Weather::Breezy,
            Species::SandDollar => c.weather = Weather::Foggy,
            Species::SeaSlug => c.weather = Weather::Drizzle,
            _ => {}
        }
        c
    }
}

/// The three stretches of shore, each opening as the aquarium grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Cove,
    Point,
    Reef,
}

impl Place {
    pub const ALL: [Place; 3] = [Place::Cove, Place::Point, Place::Reef];

    pub fn name(self) -> &'static str {
        match self {
            Place::Cove => "Sandy Cove",
            Place::Point => "Rocky Point",
            Place::Reef => "the Far Reef",
        }
    }

    /// The best-score board.
    pub fn board(self) -> &'static str {
        match self {
            Place::Cove => "cove",
            Place::Point => "point",
            Place::Reef => "reef",
        }
    }

    pub fn pools(self) -> usize {
        match self {
            Place::Cove => 6,
            Place::Point => 9,
            Place::Reef => 12,
        }
    }

    /// The trip's length on an ordinary tide, in seconds.
    pub fn tide_secs(self) -> f64 {
        match self {
            Place::Cove => 180.0,
            Place::Point => 240.0,
            Place::Reef => 300.0,
        }
    }

    pub fn max_per_pool(self) -> i32 {
        match self {
            Place::Cove | Place::Point => 2,
            Place::Reef => 3,
        }
    }

    /// How much longer shy creatures stay hidden here.
    pub fn shy_scale(self) -> f64 {
        match self {
            Place::Cove => 0.7,
            Place::Point => 1.0,
            Place::Reef => 1.3,
        }
    }

    /// The chance each rare the day allows is out in the pools.
    pub fn rare_chance(self) -> f64 {
        match self {
            Place::Cove => 0.35,
            Place::Point => 0.5,
            Place::Reef => 0.65,
        }
    }

    /// The common creatures living here.
    pub fn commons(self) -> &'static [Species] {
        match self {
            Place::Cove => &[
                Species::Crab,
                Species::Periwinkle,
                Species::Starfish,
                Species::Anemone,
                Species::Mussel,
            ],
            Place::Point => &[
                Species::Crab,
                Species::Periwinkle,
                Species::Starfish,
                Species::Anemone,
                Species::Mussel,
                Species::Goby,
                Species::Hermit,
            ],
            Place::Reef => &Species::COMMON,
        }
    }

    /// Kinds in the aquarium needed to open this place.
    pub fn unlock_kinds(self) -> usize {
        match self {
            Place::Cove => 0,
            Place::Point => 4,
            Place::Reef => 7,
        }
    }

    /// One sentence about the place.
    pub fn blurb(self) -> String {
        format!(
            "{} pools, about {} minutes before the tide returns. {}",
            self.pools(),
            (self.tide_secs() / 60.0).round() as u32,
            match self {
                Place::Cove => "A gentle sandy bay with easy creatures, and shy ones that come out quickly.",
                Place::Point => "Rockier pools with gobies and hermit crabs, and a better chance of rare finds.",
                Place::Reef => "A long reef with every common creature, the shyest ones, and the best chance of rare finds.",
            }
        )
    }
}

/// A rule that changes a challenge outing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    None,
    /// Shy creatures come out twice as fast.
    StillWater,
    /// Every pool holds at least one creature.
    Crowded,
    /// A shorter trip, every find worth double.
    QuickTide,
    /// Only the pool in front of you can be heard.
    Hushed,
    /// The weekly: one rare find is out there.
    RareHunt(Species),
    /// The monthly: the great low tide, when anything can turn up.
    GreatLowTide,
}

impl Modifier {
    pub const DAILY: [Modifier; 4] = [
        Modifier::StillWater,
        Modifier::Crowded,
        Modifier::QuickTide,
        Modifier::Hushed,
    ];

    pub fn name(self) -> String {
        match self {
            Modifier::None => "No twist".to_string(),
            Modifier::StillWater => "Still water".to_string(),
            Modifier::Crowded => "A busy shore".to_string(),
            Modifier::QuickTide => "A quick tide".to_string(),
            Modifier::Hushed => "Hushed".to_string(),
            Modifier::RareHunt(s) => format!("The {} hunt", s.name()),
            Modifier::GreatLowTide => "The Great Low Tide".to_string(),
        }
    }

    pub fn describe(self) -> String {
        match self {
            Modifier::None => String::new(),
            Modifier::StillWater => "Still water: shy creatures come back out twice as fast.".to_string(),
            Modifier::Crowded => "A busy shore: every pool holds at least one creature.".to_string(),
            Modifier::QuickTide => "A quick tide: the trip is half a minute shorter, and every find is worth double.".to_string(),
            Modifier::Hushed => "Hushed: the pools either side are silent, wherever you stand.".to_string(),
            Modifier::RareHunt(s) => format!(
                "Somewhere along the reef is one {}: {}. Finding it is worth 300 points.",
                s.name(),
                s.sound_words()
            ),
            Modifier::GreatLowTide => "The Great Low Tide: the sea draws back further than on any other day this month. Every rare creature can turn up, whatever the weather, and each different kind you gather is worth 40 points more. Gather 8 different kinds to meet the goal.".to_string(),
        }
    }
}

/// What kind of outing this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Beach(Place),
    Practice,
    Challenge(Period),
    /// A calendar event's trip, to a shore.
    Event(Event, Place),
}

/// Everything an outing is generated from.
#[derive(Clone, Debug)]
pub struct Plan {
    pub mode: Mode,
    pub place: Place,
    pub pools: usize,
    pub tide_secs: f64,
    pub conditions: Conditions,
    pub shy_scale: f64,
    pub commons: Vec<Species>,
    /// Placed at least once.
    pub guaranteed: Vec<Species>,
    /// The daily's three species.
    pub targets: Vec<Species>,
    pub modifier: Modifier,
    pub rare_chance: f64,
    /// Rares turn up whatever the conditions (the monthly).
    pub any_rare: bool,
}

impl Plan {
    /// An ordinary trip to a place, conditions rolled.
    pub fn beach(place: Place, rng: &mut Rng) -> Plan {
        let conditions = Conditions::roll(rng);
        Plan {
            mode: Mode::Beach(place),
            place,
            pools: place.pools(),
            tide_secs: place.tide_secs() + conditions.tide.extra(),
            conditions,
            shy_scale: place.shy_scale(),
            commons: place.commons().to_vec(),
            guaranteed: Vec::new(),
            targets: Vec::new(),
            modifier: Modifier::None,
            rare_chance: place.rare_chance(),
            any_rare: false,
        }
    }

    /// The practice pools: three pools, laid out the same every time.
    pub fn practice() -> Plan {
        Plan {
            mode: Mode::Practice,
            place: Place::Cove,
            pools: 3,
            tide_secs: 600.0,
            conditions: Conditions {
                weather: Weather::Sunny,
                tide: Tide::Ordinary,
                time: TimeOfDay::Morning,
            },
            shy_scale: 0.7,
            commons: Vec::new(),
            guaranteed: Vec::new(),
            targets: Vec::new(),
            modifier: Modifier::None,
            rare_chance: 0.0,
            any_rare: false,
        }
    }

    /// A calendar event's trip to `place`: the shore's own creatures,
    /// changed by the day. A king tide adds two pools at each end on a
    /// spring tide; moonlit pools are a night trip.
    pub fn event(event: Event, place: Place, rng: &mut Rng) -> Plan {
        let mut plan = Plan::beach(place, rng);
        plan.mode = Mode::Event(event, place);
        match event {
            Event::KingTide => {
                plan.conditions.tide = Tide::Spring;
                plan.pools = place.pools() + 2 * KING_TIDE_EXTRA;
                plan.tide_secs = place.tide_secs() + Tide::Spring.extra() + 60.0;
            }
            Event::Moonlit => plan.conditions.time = TimeOfDay::Night,
        }
        plan
    }

    /// The challenge of `period`, from its shared random source.
    pub fn challenge(period: Period, rng: &mut Rng) -> Plan {
        match period {
            Period::Daily => {
                let mut pool = Place::Point.commons().to_vec();
                rng.shuffle(&mut pool);
                let targets: Vec<Species> = pool.into_iter().take(3).collect();
                let modifier = *rng.pick(&Modifier::DAILY);
                let mut conditions = Conditions::roll(rng);
                conditions.time = TimeOfDay::Dusk;
                conditions.tide = Tide::Ordinary;
                let mut tide_secs = 150.0;
                if modifier == Modifier::QuickTide {
                    tide_secs -= 30.0;
                }
                Plan {
                    mode: Mode::Challenge(period),
                    place: Place::Point,
                    pools: 7,
                    tide_secs,
                    conditions,
                    shy_scale: if modifier == Modifier::StillWater {
                        0.5
                    } else {
                        1.0
                    },
                    commons: Place::Point.commons().to_vec(),
                    guaranteed: targets.clone(),
                    targets,
                    modifier,
                    rare_chance: 0.3,
                    any_rare: false,
                }
            }
            Period::Weekly => {
                let rare = *rng.pick(&Species::RARE);
                let conditions = Conditions::allowing(rare, rng);
                Plan {
                    mode: Mode::Challenge(period),
                    place: Place::Reef,
                    pools: 12,
                    tide_secs: 300.0,
                    conditions,
                    shy_scale: 1.3,
                    commons: Place::Reef.commons().to_vec(),
                    guaranteed: vec![rare],
                    targets: Vec::new(),
                    modifier: Modifier::RareHunt(rare),
                    rare_chance: 0.0,
                    any_rare: false,
                }
            }
            Period::Monthly => {
                let mut conditions = Conditions::roll(rng);
                conditions.tide = Tide::Spring;
                let mut rares = Species::RARE.to_vec();
                rng.shuffle(&mut rares);
                Plan {
                    mode: Mode::Challenge(period),
                    place: Place::Reef,
                    pools: 15,
                    tide_secs: 420.0,
                    conditions,
                    shy_scale: 1.2,
                    commons: Place::Reef.commons().to_vec(),
                    guaranteed: rares.into_iter().take(2).collect(),
                    targets: Vec::new(),
                    modifier: Modifier::GreatLowTide,
                    rare_chance: 0.45,
                    any_rare: true,
                }
            }
        }
    }

    /// "7 pools, about 2 minutes. Tonight's three: ..."
    pub fn summary(&self) -> String {
        let mut text = format!(
            "{} pools, about {} before the tide returns.",
            self.pools,
            minutes_words(self.tide_secs)
        );
        if !self.targets.is_empty() {
            text.push_str(&format!(
                " Tonight's three species: {}.",
                list_words(&self.targets.iter().map(|s| s.name()).collect::<Vec<_>>())
            ));
        }
        text
    }
}

/// "3 minutes", "1 minute", "2 and a half minutes", "40 seconds".
pub fn minutes_words(secs: f64) -> String {
    let secs = secs.max(0.0);
    if secs < 60.0 {
        let s = ((secs / 10.0).round() * 10.0).max(10.0) as u32;
        return format!("{s} seconds");
    }
    let halves = (secs / 30.0).round() as u32;
    let whole = halves / 2;
    let half = halves % 2 == 1;
    match (whole, half) {
        (1, false) => "1 minute".to_string(),
        (1, true) => "a minute and a half".to_string(),
        (n, false) => format!("{n} minutes"),
        (n, true) => format!("{n} and a half minutes"),
    }
}

/// "a, b and c".
pub fn list_words(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// One creature in a pool.
#[derive(Clone, Debug, PartialEq)]
pub struct Find {
    pub species: Species,
    /// Seconds until it comes back out; zero when it is out.
    pub hiding: f64,
    /// Seconds until its next sound.
    pub call_in: f64,
    pub gathered: bool,
}

impl Find {
    pub fn out(&self) -> bool {
        !self.gathered && self.hiding <= 0.0
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pool {
    pub finds: Vec<Find>,
}

impl Pool {
    pub fn left(&self) -> usize {
        self.finds.iter().filter(|f| !f.gathered).count()
    }
}

/// How close to the pool you are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lean {
    /// Standing back on the rocks: you hear two pools either side.
    Back,
    /// At the water's edge: you hear the next pool either side.
    Edge,
    /// Leaning in close: only this pool, and you can reach in.
    In,
}

impl Lean {
    pub fn words(self) -> &'static str {
        match self {
            Lean::Back => "standing back on the rocks",
            Lean::Edge => "at the water's edge",
            Lean::In => "leaning in close",
        }
    }

    /// How many pools either side you can hear.
    pub fn range(self) -> i32 {
        match self {
            Lean::Back => 2,
            Lean::Edge => 1,
            Lean::In => 0,
        }
    }

    /// How far ahead the pools are, in metres, as the screen places them.
    pub fn distance(self) -> f32 {
        match self {
            Lean::Back => 3.0,
            Lean::Edge => 1.6,
            Lean::In => 0.6,
        }
    }
}

/// What happened, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// Walked to another pool.
    Arrive {
        pool: usize,
        of: usize,
        /// Everything that lived here is gathered.
        cleared: bool,
    },
    /// No more rocks that way.
    Edge { left: bool },
    /// Changed lean, or tried to go past the nearest or furthest.
    Lean { lean: Lean, changed: bool },
    /// Leaning in over a pool with nothing left in it.
    OnlyWater,
    /// A creature made its sound, `offset` pools from you.
    Call {
        species: Species,
        offset: i32,
        lean: Lean,
    },
    /// Shy creatures darted out of sight as you leaned in.
    Hide { count: usize },
    /// A shy creature crept back out in your pool.
    Emerge { species: Species },
    /// Reached for one that is still hiding: it tucks away again.
    TooSoon,
    /// Reached in without having heard anything just now.
    NotYet,
    /// Reached into a pool with nothing left.
    EmptyPool,
    /// Reached for the water from too far back.
    OutOfReach,
    Gathered {
        species: Species,
        points: u64,
        target: bool,
    },
    /// Half the time is gone: the water starts back.
    TideTurned,
    /// Seconds left.
    TideWarning(u32),
    /// The trip is over: the tide is in.
    TideIn,
    /// The trip is over: nothing left to find.
    AllGathered,
}

/// One outing to the pools.
pub struct Outing {
    pub plan: Plan,
    pub pools: Vec<Pool>,
    pub at: usize,
    pub lean: Lean,
    pub tide_left: f64,
    pub elapsed: f64,
    /// Everything gathered, in order.
    pub gathered: Vec<Species>,
    /// Points from finds, before end-of-trip bonuses.
    pub points: u64,
    pub hints: u32,
    pub finished: bool,
    /// The find in this pool heard last, and how long ago.
    heard: Option<(usize, f64)>,
    /// The species of the last sound heard from this pool, for naming it.
    pub last_heard: Option<Species>,
    rng: Rng,
    turned: bool,
    warned_60: bool,
    warned_20: bool,
}

impl Outing {
    pub fn new(plan: Plan, mut rng: Rng) -> Outing {
        let pools = if plan.mode == Mode::Practice {
            practice_pools()
        } else {
            generate(&plan, &mut rng)
        };
        let mut outing = Outing {
            tide_left: plan.tide_secs,
            plan,
            pools,
            at: 0,
            lean: Lean::Edge,
            elapsed: 0.0,
            gathered: Vec::new(),
            points: 0,
            hints: 0,
            finished: false,
            heard: None,
            last_heard: None,
            rng,
            turned: false,
            warned_60: false,
            warned_20: false,
        };
        for p in 0..outing.pools.len() {
            for f in 0..outing.pools[p].finds.len() {
                let t = 0.4 + outing.rng.unit() * 2.6;
                outing.pools[p].finds[f].call_in = t;
            }
        }
        outing
    }

    pub fn pool(&self) -> &Pool {
        &self.pools[self.at]
    }

    /// Creatures still out on the beach.
    pub fn remaining(&self) -> usize {
        self.pools.iter().map(Pool::left).sum()
    }

    pub fn total(&self) -> usize {
        self.pools.iter().map(|p| p.finds.len()).sum()
    }

    /// Hearing range either side, after the modifier.
    pub fn range(&self) -> i32 {
        if self.plan.modifier == Modifier::Hushed {
            0
        } else {
            self.lean.range()
        }
    }

    /// Left (-1) or right (+1) one pool.
    pub fn walk(&mut self, dir: i32) -> Vec<Ev> {
        if self.finished {
            return Vec::new();
        }
        let to = self.at as i32 + dir;
        if to < 0 || to >= self.pools.len() as i32 {
            return vec![Ev::Edge { left: dir < 0 }];
        }
        self.at = to as usize;
        self.heard = None;
        self.last_heard = None;
        if self.lean == Lean::In {
            self.lean = Lean::Edge;
        }
        let pool = self.pool();
        vec![Ev::Arrive {
            pool: self.at,
            of: self.pools.len(),
            cleared: !pool.finds.is_empty() && pool.left() == 0,
        }]
    }

    /// Up: closer to the water.
    pub fn lean_in(&mut self) -> Vec<Ev> {
        if self.finished {
            return Vec::new();
        }
        match self.lean {
            Lean::In => vec![Ev::Lean {
                lean: Lean::In,
                changed: false,
            }],
            Lean::Back => {
                self.lean = Lean::Edge;
                vec![Ev::Lean {
                    lean: Lean::Edge,
                    changed: true,
                }]
            }
            Lean::Edge => {
                self.lean = Lean::In;
                let mut evs = vec![Ev::Lean {
                    lean: Lean::In,
                    changed: true,
                }];
                let scale = self.plan.shy_scale;
                let mut count = 0;
                for f in self.pools[self.at].finds.iter_mut() {
                    if f.gathered {
                        continue;
                    }
                    if let Some(base) = f.species.shy() {
                        f.hiding = base * scale;
                        count += 1;
                    }
                }
                if count > 0 {
                    self.heard = None;
                    evs.push(Ev::Hide { count });
                } else if self.pool().left() == 0 {
                    evs.push(Ev::OnlyWater);
                }
                evs
            }
        }
    }

    /// Down: further from the water.
    pub fn step_back(&mut self) -> Vec<Ev> {
        if self.finished {
            return Vec::new();
        }
        let (lean, changed) = match self.lean {
            Lean::In => (Lean::Edge, true),
            Lean::Edge => (Lean::Back, true),
            Lean::Back => (Lean::Back, false),
        };
        self.lean = lean;
        vec![Ev::Lean { lean, changed }]
    }

    /// Space: reach in for what you just heard.
    pub fn gather(&mut self) -> Vec<Ev> {
        if self.finished {
            return Vec::new();
        }
        if self.lean != Lean::In {
            return vec![Ev::OutOfReach];
        }
        if let Some((i, _)) = self.heard.take() {
            let find = &mut self.pools[self.at].finds[i];
            if find.out() {
                find.gathered = true;
                let species = find.species;
                let mut points = species.points();
                if self.plan.modifier == Modifier::QuickTide {
                    points *= 2;
                }
                let target =
                    self.plan.targets.contains(&species) && !self.gathered.contains(&species);
                self.points += points;
                self.gathered.push(species);
                let mut evs = vec![Ev::Gathered {
                    species,
                    points,
                    target,
                }];
                if self.remaining() == 0 {
                    self.finished = true;
                    evs.push(Ev::AllGathered);
                }
                return evs;
            }
        }
        let pool = &mut self.pools[self.at];
        let mut hidden = false;
        let scale = self.plan.shy_scale;
        for f in pool.finds.iter_mut() {
            if !f.gathered && f.hiding > 0.0 {
                hidden = true;
                let full = f.species.shy().unwrap_or(1.0) * scale;
                f.hiding = f.hiding.max(full * 0.5);
            }
        }
        if hidden {
            vec![Ev::TooSoon]
        } else if pool.left() > 0 {
            vec![Ev::NotYet]
        } else {
            vec![Ev::EmptyPool]
        }
    }

    /// The nearest pool with something left, as an offset from here
    /// (nearer first, left before right on a tie).
    pub fn nearest(&self) -> Option<i32> {
        let n = self.pools.len() as i32;
        let here = self.at as i32;
        (0..n).flat_map(|d| [-d, d]).find(|&off| {
            let p = here + off;
            p >= 0 && p < n && self.pools[p as usize].left() > 0
        })
    }

    /// Whether something in this pool is hiding right now.
    pub fn hiding_here(&self) -> bool {
        self.pool()
            .finds
            .iter()
            .any(|f| !f.gathered && f.hiding > 0.0)
    }

    pub fn note_hint(&mut self) {
        self.hints += 1;
    }

    /// The trip's score: finds, plus the challenge's bonus.
    pub fn score(&self) -> u64 {
        self.points + self.bonus()
    }

    /// The challenge's end-of-trip bonus.
    pub fn bonus(&self) -> u64 {
        match self.plan.modifier {
            Modifier::RareHunt(s) if self.gathered.contains(&s) => 300,
            Modifier::GreatLowTide => 40 * self.kinds().len() as u64,
            _ => {
                100 * self
                    .plan
                    .targets
                    .iter()
                    .filter(|t| self.gathered.contains(t))
                    .count() as u64
            }
        }
    }

    /// The different kinds gathered, in the order first found.
    pub fn kinds(&self) -> Vec<Species> {
        let mut out = Vec::new();
        for s in &self.gathered {
            if !out.contains(s) {
                out.push(*s);
            }
        }
        out
    }

    /// Whether the challenge's goal was met (every target, the rare, or
    /// eight kinds at the great low tide).
    pub fn goal_met(&self) -> bool {
        match self.plan.modifier {
            Modifier::RareHunt(s) => self.gathered.contains(&s),
            Modifier::GreatLowTide => self.kinds().len() >= 8,
            _ => {
                !self.plan.targets.is_empty()
                    && self.plan.targets.iter().all(|t| self.gathered.contains(t))
            }
        }
    }

    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        let mut evs = Vec::new();
        if self.finished {
            return evs;
        }
        self.elapsed += dt;
        if let Some((_, age)) = self.heard.as_mut() {
            *age += dt;
            if *age > GATHER_WINDOW {
                self.heard = None;
            }
        }
        let range = self.range();
        let here = self.at as i32;
        let leaning_here = self.lean == Lean::In;
        for p in 0..self.pools.len() {
            let offset = p as i32 - here;
            for f in 0..self.pools[p].finds.len() {
                let find = &mut self.pools[p].finds[f];
                if find.gathered {
                    continue;
                }
                if find.hiding > 0.0 {
                    find.hiding -= dt;
                    if find.hiding <= 0.0 {
                        find.hiding = 0.0;
                        find.call_in = 0.8;
                        if offset == 0 {
                            evs.push(Ev::Emerge {
                                species: find.species,
                            });
                        }
                    }
                    continue;
                }
                find.call_in -= dt;
                if find.call_in <= 0.0 {
                    let quick = offset == 0 && leaning_here;
                    let (lo, span) = if quick { (1.6, 1.6) } else { (2.4, 2.4) };
                    find.call_in = lo + self.rng.unit() * span;
                    if offset.abs() <= range {
                        let species = find.species;
                        evs.push(Ev::Call {
                            species,
                            offset,
                            lean: self.lean,
                        });
                        if offset == 0 {
                            self.heard = Some((f, 0.0));
                            self.last_heard = Some(species);
                        }
                    }
                }
            }
        }
        // The practice pools have no tide: the trip lasts until everything
        // is gathered or the player leaves.
        if self.plan.mode == Mode::Practice {
            return evs;
        }
        let total = self.plan.tide_secs;
        self.tide_left -= dt;
        if !self.turned && self.tide_left <= total / 2.0 {
            self.turned = true;
            evs.push(Ev::TideTurned);
        }
        if total > 120.0 && !self.warned_60 && self.tide_left <= 60.0 {
            self.warned_60 = true;
            evs.push(Ev::TideWarning(60));
        }
        if total > 60.0 && !self.warned_20 && self.tide_left <= 20.0 {
            self.warned_20 = true;
            evs.push(Ev::TideWarning(20));
        }
        if self.tide_left <= 0.0 {
            self.tide_left = 0.0;
            self.finished = true;
            self.heard = None;
            evs.push(Ev::TideIn);
        }
        evs
    }

    /// How far in the tide has come, 0 at the start to 1 when it is in.
    pub fn tide_in(&self) -> f64 {
        (1.0 - self.tide_left / self.plan.tide_secs).clamp(0.0, 1.0)
    }
}

fn find(species: Species) -> Find {
    Find {
        species,
        hiding: 0.0,
        call_in: 1.0,
        gathered: false,
    }
}

/// The practice pools: an anemone, a shy crab, and a periwinkle with a
/// starfish.
fn practice_pools() -> Vec<Pool> {
    vec![
        Pool {
            finds: vec![find(Species::Anemone)],
        },
        Pool {
            finds: vec![find(Species::Crab)],
        },
        Pool {
            finds: vec![find(Species::Periwinkle), find(Species::Starfish)],
        },
    ]
}

/// Lays out a beach from a plan.
pub fn generate(plan: &Plan, rng: &mut Rng) -> Vec<Pool> {
    let n = plan.pools.max(1);
    let mut pools = vec![Pool::default(); n];
    let max = plan.place.max_per_pool();
    let min = if plan.modifier == Modifier::Crowded {
        1
    } else {
        0
    };
    for pool in pools.iter_mut() {
        let count = rng.range(min, max);
        for _ in 0..count {
            if plan.commons.is_empty() {
                break;
            }
            pool.finds.push(find(*rng.pick(&plan.commons)));
        }
    }
    let mut extra: Vec<Species> = plan.guaranteed.clone();
    for rare in Species::RARE {
        if extra.contains(&rare) {
            continue;
        }
        if (plan.any_rare || plan.conditions.allows(rare)) && rng.chance(plan.rare_chance) {
            extra.push(rare);
        }
    }
    for s in extra {
        let already = pools.iter().any(|p| p.finds.iter().any(|f| f.species == s));
        if plan.targets.contains(&s) && already {
            continue;
        }
        // Rares and the weekly's find never sit in the first pool.
        let lo = if s.rare() && n > 2 { 2 } else { 0 };
        let p = lo + rng.below((n - lo) as u32) as usize;
        pools[p].finds.push(find(s));
    }
    // A king tide's extra pools, out where the sea has gone, are never bare.
    if let Mode::Event(Event::KingTide, _) = plan.mode {
        let ends: Vec<usize> = (0..KING_TIDE_EXTRA.min(n))
            .chain(n.saturating_sub(KING_TIDE_EXTRA)..n)
            .collect();
        for p in ends {
            if pools[p].finds.is_empty() && !plan.commons.is_empty() {
                pools[p].finds.push(find(*rng.pick(&plan.commons)));
            }
        }
    }
    let total: usize = pools.iter().map(|p| p.finds.len()).sum();
    let want = n.div_ceil(2);
    for _ in total..want {
        if plan.commons.is_empty() {
            break;
        }
        let p = rng.below(n as u32) as usize;
        pools[p].finds.push(find(*rng.pick(&plan.commons)));
    }
    pools
}
