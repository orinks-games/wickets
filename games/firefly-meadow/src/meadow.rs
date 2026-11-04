//! The places to walk, and the plan an evening is built from: its size,
//! season, fireflies, landmarks and what the calendar adds.

use parlor_core::{Date, Period, Rng, Season};
use serde::{Deserialize, Serialize};

use crate::almanac;
use crate::sound;
use crate::species::Species;

/// A point in the meadow, in metres: x runs east, y runs north.
pub type P2 = (f64, f64);

/// A walking place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Place {
    Home,
    Orchard,
    Creek,
    Hayfield,
    /// The tiny practice meadow.
    Practice,
}

impl Place {
    /// The places the title's Meadow row steps through, in unlock order.
    pub const WALKS: [Place; 4] = [Place::Home, Place::Orchard, Place::Creek, Place::Hayfield];

    pub fn name(self) -> &'static str {
        match self {
            Place::Home => "the home meadow",
            Place::Orchard => "the orchard",
            Place::Creek => "the creek bank",
            Place::Hayfield => "the hayfield",
            Place::Practice => "the practice patch",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Place::Home => "home",
            Place::Orchard => "orchard",
            Place::Creek => "creek",
            Place::Hayfield => "hayfield",
            Place::Practice => "practice",
        }
    }

    /// Fireflies caught, in all, that open it.
    pub fn opens_at(self) -> u32 {
        match self {
            Place::Home | Place::Practice => 0,
            Place::Orchard => 8,
            Place::Creek => 20,
            Place::Hayfield => 35,
        }
    }

    /// Width and depth, metres.
    pub fn size(self) -> (f64, f64) {
        match self {
            Place::Home => (20.0, 20.0),
            Place::Orchard => (22.0, 18.0),
            Place::Creek => (16.0, 26.0),
            Place::Hayfield => (28.0, 28.0),
            Place::Practice => (8.0, 8.0),
        }
    }

    /// Fireflies out at once at the height of summer.
    pub fn summer_count(self) -> usize {
        match self {
            Place::Home => 6,
            Place::Orchard => 7,
            Place::Creek => 7,
            Place::Hayfield => 9,
            Place::Practice => 1,
        }
    }

    /// The species that live here, with how often each appears.
    pub fn species(self) -> &'static [(Species, u32)] {
        match self {
            Place::Home => &[
                (Species::Dipper, 5),
                (Species::TwinSpark, 4),
                (Species::SlowEmber, 3),
            ],
            Place::Orchard => &[
                (Species::Dipper, 4),
                (Species::OrchardLantern, 4),
                (Species::TripleTwinkle, 3),
                (Species::BlueGhost, 1),
            ],
            Place::Creek => &[
                (Species::CreekGlimmer, 5),
                (Species::TwinSpark, 3),
                (Species::Dipper, 2),
                (Species::BlueGhost, 1),
            ],
            Place::Hayfield => &[
                (Species::HayFlicker, 5),
                (Species::TripleTwinkle, 3),
                (Species::Dipper, 3),
                (Species::SlowEmber, 2),
            ],
            Place::Practice => &[(Species::Dipper, 1)],
        }
    }

    /// Where the walk starts: the middle of the south edge.
    pub fn start(self) -> P2 {
        let (w, _) = self.size();
        (w / 2.0, 1.0)
    }

    /// The fixed sounds that mark the place, and where they are.
    pub fn landmarks(self) -> Vec<(&'static str, P2, &'static str)> {
        let (w, d) = self.size();
        match self {
            Place::Home => vec![
                (sound::FROGS, (w + 2.0, d * 0.6), "the frog pond, east"),
                (sound::CREEK, (w / 2.0, d + 3.0), "the creek, north"),
            ],
            Place::Orchard => vec![
                (sound::TREES, (w * 0.3, d * 0.6), "the old apple tree, west"),
                (
                    sound::TREES,
                    (w * 0.75, d * 0.75),
                    "the young apple trees, north east",
                ),
                (
                    sound::CREEK,
                    (w / 2.0, d + 8.0),
                    "the creek, far to the north",
                ),
            ],
            Place::Creek => vec![
                (
                    sound::CREEK,
                    (-1.0, d * 0.5),
                    "the creek, along the west side",
                ),
                (sound::FROGS, (-1.0, d * 0.85), "the frogs, north west"),
            ],
            Place::Hayfield => vec![
                (sound::HAY, (w * 0.5, d * 0.5), "the hay, all round"),
                (sound::FROGS, (-3.0, d * 0.2), "the ditch frogs, south west"),
            ],
            Place::Practice => Vec::new(),
        }
    }
}

/// Everything an evening is built from.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub place: Place,
    pub season: Season,
    pub size: P2,
    /// Fireflies out at once.
    pub count: usize,
    pub respawn: bool,
    /// The species, with weights.
    pub species: Vec<(Species, u32)>,
    /// Seconds before the first firefly shows.
    pub delay: f64,
    /// Distant fireworks on the horizon.
    pub fireworks: bool,
    /// Whether today is midsummer, so the evening can say so.
    pub midsummer: bool,
    /// How many fireflies the jar holds.
    pub jar: usize,
    /// A challenge's time limit, seconds.
    pub clock: Option<f64>,
    /// Places the first firefly just ahead, for practice.
    pub practice: bool,
    /// The date the evening is on.
    pub date: Date,
}

/// The jar's size.
pub const JAR: usize = 6;

impl Plan {
    /// A walk in `place` on the real `date`.
    pub fn on(place: Place, date: Date) -> Plan {
        let wild = almanac::wild(date, place.summer_count());
        let species = match &wild.only {
            Some(only) => only.iter().map(|&s| (s, 1)).collect(),
            None => place.species().to_vec(),
        };
        Plan {
            place,
            season: almanac::season(date),
            size: place.size(),
            count: wild.count,
            respawn: wild.respawn,
            species,
            delay: wild.delay,
            fireworks: almanac::fireworks(date) && almanac::season(date) == Season::Summer,
            midsummer: almanac::is_midsummer(date),
            jar: JAR,
            clock: None,
            practice: false,
            date,
        }
    }

    /// The practice patch: always summer, one Dipper just ahead.
    pub fn practice(date: Date) -> Plan {
        Plan {
            season: Season::Summer,
            count: 1,
            respawn: true,
            species: vec![(Species::Dipper, 1)],
            delay: 0.0,
            fireworks: false,
            midsummer: false,
            practice: true,
            ..Plan::on(Place::Practice, date)
        }
    }

    /// A challenge: a summer evening from the almanac, the same for every
    /// player, whatever the season outside.
    pub fn challenge(period: Period, rng: &mut Rng, date: Date) -> Plan {
        let place = match period {
            Period::Monthly => Place::Hayfield,
            _ => *rng.pick(&Place::WALKS),
        };
        let (clock, jar) = match period {
            Period::Daily => (240.0, JAR),
            Period::Weekly => (360.0, 4),
            Period::Monthly => (480.0, JAR),
        };
        Plan {
            season: Season::Summer,
            count: place.summer_count(),
            respawn: true,
            species: place.species().to_vec(),
            delay: 0.0,
            fireworks: false,
            midsummer: false,
            jar,
            clock: Some(clock),
            ..Plan::on(place, date)
        }
    }

    /// "the home meadow, a summer evening".
    pub fn title(&self) -> String {
        format!(
            "{}, {}",
            self.place.name(),
            almanac::evening_words(self.season)
        )
    }

    /// Picks a species by weight.
    pub fn pick_species(&self, rng: &mut Rng) -> Option<Species> {
        let total: u32 = self.species.iter().map(|(_, w)| w).sum();
        if total == 0 {
            return None;
        }
        let mut n = rng.below(total);
        for &(s, w) in &self.species {
            if n < w {
                return Some(s);
            }
            n -= w;
        }
        self.species.last().map(|(s, _)| *s)
    }
}
