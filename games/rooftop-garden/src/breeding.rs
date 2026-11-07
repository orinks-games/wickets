//! Seed varieties, crosses, keepsakes and the neighbours' requests.
//!
//! Every plant carries [`Traits`]: the instrument its part plays on (its
//! tone), how thirsty it is, how fast it grows and whether it can give a
//! double harvest. A plain seed has the plain traits. When you harvest a
//! plant in your own garden that bloomed beside a different plant, the two
//! may cross, and the harvest leaves a packet of new seeds that mixes
//! their traits, with a little chance of something new. Each mix of traits
//! has its own name, so every species has 54 varieties, 324 in all.
//!
//! What you find is kept in the [`Collection`], saved apart from the
//! garden so starting the garden over never loses it. New varieties,
//! heirlooms and filled requests each earn a star, and stars unlock
//! [`Keepsake`]s for the rooftop.

use parlor_core::Rng;
use serde::{Deserialize, Serialize};

use crate::rules::Species;

/// How many tones each species can play on.
pub const TONES: u8 = 3;
/// Harvests of one variety before it breeds true: an heirloom.
pub const HEIRLOOM_AT: u32 = 5;
/// The most requests open at once.
pub const MAX_REQUESTS: usize = 2;
/// Requests open at once with the swap board.
pub const SWAP_REQUESTS: usize = 3;
/// How likely two different plants side by side are to cross at a harvest.
pub const CROSS_CHANCE: f64 = 0.3;
/// How likely a plant with no different neighbour is to throw a new
/// variety by itself.
pub const SPORT_CHANCE: f64 = 0.08;
/// How likely each trait is to change in a cross.
pub const MUTATE: f64 = 0.2;
/// How likely a double-harvest plant is to give two at once.
pub const DOUBLE_CHANCE: f64 = 0.5;
/// What the beehive adds to both chances.
pub const HIVE_BONUS: f64 = 0.15;
/// Waterings the rain barrel holds after a shower.
pub const BARREL_FILL: u32 = 3;

/// A plant's inherited traits. The plain seed is tone 0, steady thirst,
/// steady pace, single harvests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Traits {
    /// Which instrument its part plays on: 0 is its own, 1 and 2 others.
    pub tone: u8,
    /// 0 thirsty, 1 steady, 2 hardy.
    pub thirst: u8,
    /// 0 slow, 1 steady, 2 quick.
    pub pace: u8,
    /// Whether a harvest can give two at once.
    pub double: bool,
}

impl Default for Traits {
    fn default() -> Traits {
        Traits::PLAIN
    }
}

impl Traits {
    pub const PLAIN: Traits = Traits {
        tone: 0,
        thirst: 1,
        pace: 1,
        double: false,
    };

    /// Every set of traits, in a fixed order: 54.
    pub fn all() -> Vec<Traits> {
        let mut out = Vec::new();
        for tone in 0..TONES {
            for thirst in 0..3 {
                for pace in 0..3 {
                    for double in [false, true] {
                        out.push(Traits {
                            tone,
                            thirst,
                            pace,
                            double,
                        });
                    }
                }
            }
        }
        out
    }

    /// Mends traits from a damaged save.
    pub fn repair(&mut self) {
        self.tone = self.tone.min(TONES - 1);
        self.thirst = self.thirst.min(2);
        self.pace = self.pace.min(2);
    }

    /// How fast its soil dries, against a plain seed.
    pub fn thirst_rate(self) -> f32 {
        match self.thirst {
            0 => 1.3,
            2 => 0.7,
            _ => 1.0,
        }
    }

    /// How fast it grows, against a plain seed.
    pub fn growth_rate(self) -> f32 {
        match self.pace {
            0 => 0.85,
            2 => 1.2,
            _ => 1.0,
        }
    }

    /// How often aphids land on it: slow plants are tougher.
    pub fn pest_rate(self) -> f32 {
        if self.pace == 0 {
            0.6
        } else {
            1.0
        }
    }

    /// Its growing traits in a few words: "hardy, quick, double harvests".
    /// Empty for steady, single-harvest plants.
    pub fn habits(self) -> Vec<&'static str> {
        let mut out = Vec::new();
        match self.thirst {
            0 => out.push("thirsty"),
            2 => out.push("hardy"),
            _ => {}
        }
        match self.pace {
            0 => out.push("slow, few aphids"),
            2 => out.push("quick"),
            _ => {}
        }
        if self.double {
            out.push("double harvests");
        }
        out
    }
}

/// First words of a variety's name, by thirst and pace. A steady, steady
/// plant has none.
const FIRST: [&str; 9] = [
    "Willow", "Brook", "Spring", "Sleepy", "", "Swift", "Stone", "Copper", "Desert",
];
/// Second words, by tone and double harvests. Tone 0, single harvests has
/// none.
const SECOND: [&str; 6] = ["", "Bounty", "Moon", "Crown", "Star", "Jewel"];

/// A species with its traits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Variety {
    pub species: Species,
    #[serde(default)]
    pub traits: Traits,
}

impl Variety {
    pub fn plain(species: Species) -> Variety {
        Variety {
            species,
            traits: Traits::PLAIN,
        }
    }

    pub fn is_plain(self) -> bool {
        self.traits == Traits::PLAIN
    }

    /// Every variety of every species: 324.
    pub fn count() -> usize {
        Species::ALL.len() * Traits::all().len()
    }

    /// "Copper Moon tomato", or "tomato" for a plain seed.
    pub fn name(self) -> String {
        let t = self.traits;
        let first = FIRST[(t.thirst * 3 + t.pace) as usize];
        let second = SECOND[(t.tone * 2 + t.double as u8) as usize];
        [first, second, self.species.name()]
            .into_iter()
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The instrument it plays on: "a marimba".
    pub fn tone(self) -> &'static str {
        tone_name(self.species, self.traits.tone)
    }

    /// Its traits for a list: "hardy, quick, on a marimba".
    pub fn describe(self) -> String {
        let mut parts: Vec<String> = self
            .traits
            .habits()
            .into_iter()
            .map(String::from)
            .collect();
        parts.push(format!("on {}", self.tone()));
        parts.join(", ")
    }
}

impl From<Species> for Variety {
    fn from(species: Species) -> Variety {
        Variety::plain(species)
    }
}

/// The instrument a species' part plays on, with its article.
pub fn tone_name(species: Species, tone: u8) -> &'static str {
    let names: [&str; 3] = match species {
        Species::Tomato => ["a kalimba", "a marimba", "a music box"],
        Species::Squash => ["an upright bass", "a finger bass", "a tuba"],
        Species::Bean => ["plucked strings", "a harp", "a banjo"],
        Species::Sunflower => ["a flute", "a clarinet", "an ocarina"],
        Species::Lavender => ["a glockenspiel", "a celesta", "a vibraphone"],
        Species::Mint => ["a glass pad", "a string pad", "a choir"],
    };
    names[tone.min(TONES - 1) as usize]
}

/// Picks one of `n` values other than `now`.
fn other(rng: &mut Rng, now: u8, n: u8) -> u8 {
    (now + 1 + rng.below(n as u32 - 1) as u8) % n
}

fn mutate(rng: &mut Rng, t: &mut Traits, which: u32) {
    match which {
        0 => t.tone = other(rng, t.tone, TONES),
        1 => t.thirst = other(rng, t.thirst, 3),
        2 => t.pace = other(rng, t.pace, 3),
        _ => t.double = !t.double,
    }
}

/// The seeds a cross of `mother` with `father` leaves: each trait from
/// one parent or the other, and now and then a new one. With no father,
/// a sport: the mother with one trait changed.
pub fn cross(rng: &mut Rng, mother: Variety, father: Option<Traits>) -> Variety {
    let mut t = mother.traits;
    match father {
        Some(f) => {
            if rng.chance(0.5) {
                t.tone = f.tone;
            }
            if rng.chance(0.5) {
                t.thirst = f.thirst;
            }
            if rng.chance(0.5) {
                t.pace = f.pace;
            }
            if rng.chance(0.5) {
                t.double = f.double;
            }
            for which in 0..4 {
                if rng.chance(MUTATE) {
                    mutate(rng, &mut t, which);
                }
            }
        }
        None => {
            let which = rng.below(4);
            mutate(rng, &mut t, which);
        }
    }
    Variety {
        species: mother.species,
        traits: t,
    }
}

/// Things for the rooftop, unlocked by stars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Keepsake {
    BirdBath,
    RainBarrel,
    Beehive,
    Trellis,
    CompostBin,
    WindChimes,
    StringLights,
    ColdFrame,
    SwapBoard,
    PottingBench,
}

impl Keepsake {
    pub const ALL: [Keepsake; 10] = [
        Keepsake::BirdBath,
        Keepsake::RainBarrel,
        Keepsake::Beehive,
        Keepsake::Trellis,
        Keepsake::CompostBin,
        Keepsake::WindChimes,
        Keepsake::StringLights,
        Keepsake::ColdFrame,
        Keepsake::SwapBoard,
        Keepsake::PottingBench,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Keepsake::BirdBath => "bird bath",
            Keepsake::RainBarrel => "rain barrel",
            Keepsake::Beehive => "beehive",
            Keepsake::Trellis => "trellis",
            Keepsake::CompostBin => "compost bin",
            Keepsake::WindChimes => "wind chimes",
            Keepsake::StringLights => "string lights",
            Keepsake::ColdFrame => "cold frame",
            Keepsake::SwapBoard => "swap board",
            Keepsake::PottingBench => "potting bench",
        }
    }

    /// Stars that unlock it.
    pub fn stars(self) -> u32 {
        match self {
            Keepsake::BirdBath => 3,
            Keepsake::RainBarrel => 8,
            Keepsake::Beehive => 15,
            Keepsake::Trellis => 25,
            Keepsake::CompostBin => 40,
            Keepsake::WindChimes => 60,
            Keepsake::StringLights => 85,
            Keepsake::ColdFrame => 110,
            Keepsake::SwapBoard => 140,
            Keepsake::PottingBench => 180,
        }
    }

    /// What it does, in one sentence.
    pub fn about(self) -> &'static str {
        match self {
            Keepsake::BirdBath => "Sparrows visit and eat some aphids before they settle.",
            Keepsake::RainBarrel => "Catches rain, then waters the next 3 plants that get thirsty.",
            Keepsake::Beehive => "Bees carry pollen, so plants cross more often.",
            Keepsake::Trellis => "Plants in bloom get leggy half as often.",
            Keepsake::CompostBin => "New seeds sprout twice as fast.",
            Keepsake::WindChimes => {
                "On breezy days, pollen blows further: plants can cross from a corner or two planters away."
            }
            Keepsake::StringLights => "Lights come on each evening, and plants grow faster under them.",
            Keepsake::ColdFrame => "C sets it over a planter. The soil under it never dries out.",
            Keepsake::SwapBoard => "Neighbours can ask for 3 things at once, and every filled request brings gift seeds.",
            Keepsake::PottingBench => {
                "Once a garden day, use P to select two different varieties in bloom or ripe and try for new seeds."
            }
        }
    }
}

/// What a neighbour wants from a harvest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Want {
    Any,
    Hardy,
    Quick,
    Double,
    Tone(u8),
}

impl Want {
    pub fn fits(self, t: Traits) -> bool {
        match self {
            Want::Any => true,
            Want::Hardy => t.thirst == 2,
            Want::Quick => t.pace == 2,
            Want::Double => t.double,
            Want::Tone(n) => t.tone == n,
        }
    }
}

/// The neighbours who ask for things.
pub const NEIGHBOURS: [&str; 8] = [
    "Mrs Patel", "Mr Okafor", "Rosa", "Old Sam", "Ines", "Dev", "Mrs Moreau", "Tomas",
];

/// One neighbour's request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub who: u8,
    pub species: Species,
    pub want: Want,
}

impl Request {
    pub fn who(self) -> &'static str {
        NEIGHBOURS[self.who as usize % NEIGHBOURS.len()]
    }

    /// "Rosa wants beans from a hardy plant."
    pub fn describe(self) -> String {
        let what = match self.species {
            Species::Tomato => "a tomato",
            Species::Squash => "a squash",
            Species::Bean => "beans",
            Species::Sunflower => "a sunflower",
            Species::Lavender => "lavender",
            Species::Mint => "mint",
        };
        let from = match self.want {
            Want::Any => String::new(),
            Want::Hardy => " from a hardy plant".into(),
            Want::Quick => " from a quick plant".into(),
            Want::Double => " from a double harvest plant".into(),
            Want::Tone(n) => format!(" from a plant on {}", tone_name(self.species, n)),
        };
        format!("{} wants {what}{from}.", self.who())
    }

    pub fn fits(self, v: Variety) -> bool {
        v.species == self.species && self.want.fits(v.traits)
    }
}

/// One variety found, and how often it has been harvested since.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Found {
    pub variety: Variety,
    #[serde(default)]
    pub harvests: u32,
}

impl Found {
    pub fn heirloom(&self) -> bool {
        self.harvests >= HEIRLOOM_AT
    }
}

/// Everything your gardens have found, kept apart from the garden.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    /// New varieties, in the order found. Plain seeds are not listed.
    #[serde(default)]
    pub found: Vec<Found>,
    #[serde(default)]
    pub keepsakes: Vec<Keepsake>,
    #[serde(default)]
    pub requests: Vec<Request>,
    /// Requests filled.
    #[serde(default)]
    pub thanks: u32,
    /// Rain barrel waterings left.
    #[serde(default)]
    pub barrel: u32,
}

impl Collection {
    pub fn knows(&self, v: Variety) -> bool {
        v.is_plain() || self.found.iter().any(|f| f.variety == v)
    }

    pub fn heirlooms(&self) -> usize {
        self.found.iter().filter(|f| f.heirloom()).count()
    }

    /// One for each new variety, heirloom and filled request.
    pub fn stars(&self) -> u32 {
        (self.found.len() + self.heirlooms()) as u32 + self.thanks
    }

    pub fn has(&self, k: Keepsake) -> bool {
        self.keepsakes.contains(&k)
    }

    /// Requests open at once.
    pub fn max_requests(&self) -> usize {
        if self.has(Keepsake::SwapBoard) {
            SWAP_REQUESTS
        } else {
            MAX_REQUESTS
        }
    }

    /// The next keepsake still to earn.
    pub fn next_keepsake(&self) -> Option<Keepsake> {
        Keepsake::ALL.into_iter().find(|k| !self.has(*k))
    }

    /// The varieties of `species` you can plant: the plain seed, then the
    /// ones found, in their fixed order.
    pub fn varieties(&self, species: Species) -> Vec<Variety> {
        let mut out: Vec<Variety> = self
            .found
            .iter()
            .map(|f| f.variety)
            .filter(|v| v.species == species)
            .collect();
        out.sort_by_key(|v| v.traits);
        out.insert(0, Variety::plain(species));
        out
    }

    /// Mends a damaged save.
    pub fn repair(&mut self) {
        for f in self.found.iter_mut() {
            f.variety.traits.repair();
        }
        let mut seen = Vec::new();
        self.found.retain(|f| {
            let keep = !f.variety.is_plain() && !seen.contains(&f.variety);
            seen.push(f.variety);
            keep
        });
        self.requests.truncate(self.max_requests());
        self.barrel = self.barrel.min(BARREL_FILL);
    }

    /// A new request for one of `seeds`, from a neighbour not already
    /// asking. It asks for a trait you have, or now and then one to find.
    pub fn new_request(&self, rng: &mut Rng, seeds: &[Species]) -> Option<Request> {
        if seeds.is_empty() || self.requests.len() >= self.max_requests() {
            return None;
        }
        let free: Vec<u8> = (0..NEIGHBOURS.len() as u8)
            .filter(|n| !self.requests.iter().any(|r| r.who == *n))
            .collect();
        let who = *rng.pick(&free);
        let species = *rng.pick(seeds);
        let have: Vec<Want> = self
            .varieties(species)
            .iter()
            .flat_map(|v| {
                let t = v.traits;
                let mut w = Vec::new();
                if t.thirst == 2 {
                    w.push(Want::Hardy);
                }
                if t.pace == 2 {
                    w.push(Want::Quick);
                }
                if t.double {
                    w.push(Want::Double);
                }
                if t.tone > 0 {
                    w.push(Want::Tone(t.tone));
                }
                w
            })
            .collect();
        let want = if !have.is_empty() && rng.chance(0.6) {
            *rng.pick(&have)
        } else if self.found.len() >= 3 && rng.chance(0.5) {
            *rng.pick(&[
                Want::Hardy,
                Want::Quick,
                Want::Double,
                Want::Tone(1),
                Want::Tone(2),
            ])
        } else {
            Want::Any
        };
        Some(Request { who, species, want })
    }

    /// A variety of one of `seeds` not found yet, as a neighbour's gift.
    pub fn gift(&self, rng: &mut Rng, seeds: &[Species]) -> Option<Variety> {
        let all = Traits::all();
        for _ in 0..60 {
            let v = Variety {
                species: *rng.pick(seeds),
                traits: *rng.pick(&all),
            };
            if !self.knows(v) {
                return Some(v);
            }
        }
        None
    }
}
