//! Courses: a route measured along (`s`) and across (`x`, metres from the
//! right-hand curb), its cross-section, and the hazards and junctions on
//! it. Courses come from a seed, so every player gets the same one.

use parlor_core::Rng;
use serde::{Deserialize, Serialize};

use crate::bike::Surface;

/// The Downtown cross-section, from the right-hand curb outward.
/// Parked cars fill the parking lane, from the curb to here.
pub const PARKED_EDGE: f64 = 2.0;
/// An opening car door reaches out to here: the door zone runs from the
/// parked cars to this line.
pub const DOOR_EDGE: f64 = 3.0;
/// The painted line between the bike lane and the traffic lane.
pub const BIKE_LANE_LINE: f64 = 3.5;
/// The centre line, beyond which is the oncoming lane.
pub const CENTRE_LINE: f64 = 7.0;
/// The middle of the bike lane, where rides start.
pub const START_X: f64 = 2.8;
/// Where a rider gets back up after a crash: the bike lane, clear of doors.
pub const SAFE_X: f64 = 3.35;
/// Where cars from behind drive, by their centre.
pub const CAR_LANE_X: f64 = 5.25;

/// The kinds of hazard on the road.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HazardKind {
    /// A parked car whose door opens into the door zone.
    Door,
    /// A pothole: a flat for a road bike, a jolt for a BMX.
    Pothole,
    /// A raised curb across the bike lane at a bus stop.
    Curb,
    /// A ramp in the bike lane: hop at its lip for a jump.
    Ramp,
}

impl HazardKind {
    /// Its name in a spoken line: "door", "pothole".
    pub fn word(self) -> &'static str {
        match self {
            HazardKind::Door => "door",
            HazardKind::Pothole => "pothole",
            HazardKind::Curb => "curb",
            HazardKind::Ramp => "ramp",
        }
    }

    /// Seconds before the rider reaches it that its warning sounds, at the
    /// rider's speed.
    pub fn lead(self) -> f64 {
        match self {
            HazardKind::Door => DOOR_LEAD,
            _ => MARKER_LEAD,
        }
    }

    /// Whether running into it is bad, as opposed to a ramp.
    pub fn harmful(self) -> bool {
        !matches!(self, HazardKind::Ramp)
    }
}

/// A door's handle clicks this many seconds before the rider reaches it.
pub const DOOR_LEAD: f64 = 2.0;
/// And the door swings open this many seconds before.
pub const DOOR_OPENS: f64 = 1.0;
/// Potholes, curbs and ramps warn this many seconds ahead.
pub const MARKER_LEAD: f64 = 2.5;
/// No warning comes closer than this many metres, so a slow rider still
/// hears a hazard well ahead.
pub const MIN_WARN_M: f64 = 6.0;
/// The fairness rule: every hazard warns at least this long before contact.
pub const FAIR_LEAD: f64 = 1.5;
/// Height of a curb, metres.
pub const CURB_M: f64 = 0.15;
/// Extra height a ramp gives a hop, metres.
pub const RAMP_M: f64 = 0.35;
/// A hop this close before a ramp's lip, in seconds, takes off from it.
pub const RAMP_WINDOW: f64 = 0.35;
/// Half the width of a pothole, metres.
pub const POTHOLE_HALF: f64 = 0.35;

/// One hazard: where it is along the route and the stretch across it
/// that it covers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hazard {
    pub kind: HazardKind,
    pub s: f64,
    pub x0: f64,
    pub x1: f64,
}

impl Hazard {
    /// The middle of its stretch across the road.
    pub fn x(&self) -> f64 {
        (self.x0 + self.x1) / 2.0
    }

    /// Whether a span centred on `x`, `half` wide each side, meets it.
    pub fn covers(&self, x: f64, half: f64) -> bool {
        x + half > self.x0 && x - half < self.x1
    }
}

/// A turn at a junction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Turn {
    Left,
    Right,
}

impl Turn {
    pub fn word(self) -> &'static str {
        match self {
            Turn::Left => "left",
            Turn::Right => "right",
        }
    }

    /// The stretch of the road to be in at the junction line: the third
    /// of the rideable road on the turn's side.
    pub fn band(self) -> (f64, f64) {
        let third = (CENTRE_LINE - PARKED_EDGE) / 3.0;
        match self {
            Turn::Right => (PARKED_EDGE, PARKED_EDGE + third),
            Turn::Left => (CENTRE_LINE - third, CENTRE_LINE),
        }
    }

    /// Where in the road that is, in words.
    pub fn place(self) -> &'static str {
        match self {
            Turn::Right => "the bike lane",
            Turn::Left => "the left of the traffic lane, near the centre line",
        }
    }
}

/// Take a junction turn at this speed or under, metres a second.
pub const TURN_SPEED: f64 = 6.0;
/// A junction turn is announced this many seconds ahead, or this many
/// metres, whichever comes first.
pub const TURN_LEAD: f64 = 6.0;
pub const TURN_WARN_M: f64 = 50.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Junction {
    pub s: f64,
    pub turn: Turn,
}

/// A parcel left out for collection, in an endless ride: `x` is where it
/// sits across the road.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParcelSpot {
    pub s: f64,
    pub x: f64,
    /// Dollars.
    pub value: u32,
}

/// A whole course.
#[derive(Clone, Debug, PartialEq)]
pub struct Course {
    pub name: String,
    pub seed: u64,
    pub length: f64,
    /// In order along the route.
    pub hazards: Vec<Hazard>,
    pub junctions: Vec<Junction>,
    /// Whether cars come from behind.
    pub traffic: bool,
    /// Seconds between cars, on average.
    pub car_gap: f64,
    /// Parcels to scoop up, in order along the route.
    pub parcels: Vec<ParcelSpot>,
    /// Seconds between oncoming cars at the start, on average; 0 for none.
    pub oncoming_gap: f64,
    pub surface: Surface,
}

/// The districts of Courier City, each a world for the endless ride.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum District {
    #[default]
    Downtown,
    Riverside,
    OldTown,
}

impl District {
    pub const ALL: [District; 3] = [District::Downtown, District::Riverside, District::OldTown];

    pub fn id(self) -> &'static str {
        match self {
            District::Downtown => "downtown",
            District::Riverside => "riverside",
            District::OldTown => "oldtown",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            District::Downtown => "Downtown",
            District::Riverside => "Riverside",
            District::OldTown => "Old Town",
        }
    }

    /// Dollars it costs in the bike shop; 0 for Downtown.
    pub fn price(self) -> u32 {
        match self {
            District::Downtown => 0,
            District::Riverside => 250,
            District::OldTown => 500,
        }
    }

    pub fn surface(self) -> Surface {
        match self {
            District::Downtown => Surface::Dry,
            District::Riverside => Surface::Wet,
            District::OldTown => Surface::Cobbles,
        }
    }

    /// What riding there is like, in a sentence.
    pub fn about(self) -> &'static str {
        match self {
            District::Downtown => "Dry streets, doors, potholes and ramps.",
            District::Riverside => {
                "Rain on the river road. Wet brakes need more room, and potholes are everywhere."
            }
            District::OldTown => "Cobbled lanes. Thin tyres crawl, and café doors open often.",
        }
    }

    /// The hazard mix at the start of its endless ride.
    pub fn mix(self) -> Mix {
        match self {
            District::Downtown => Mix {
                junctions: 0,
                ..Mix::DOWNTOWN
            },
            District::Riverside => Mix {
                gap: (35.0, 65.0),
                weights: [2, 6, 1, 2],
                junctions: 0,
                car_gap: 12.0,
            },
            District::OldTown => Mix {
                gap: (30.0, 60.0),
                weights: [6, 2, 2, 1],
                junctions: 0,
                car_gap: 18.0,
            },
        }
    }
}

/// How much of what a generated course holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {
    /// Metres between hazards, least and most.
    pub gap: (f64, f64),
    /// Relative weights of doors, potholes, curbs and ramps.
    pub weights: [u32; 4],
    pub junctions: u32,
    pub car_gap: f64,
}

impl Mix {
    pub const DOWNTOWN: Mix = Mix {
        gap: (35.0, 70.0),
        weights: [4, 3, 2, 2],
        junctions: 2,
        car_gap: 14.0,
    };
}

/// An endless ride's road runs this far: further than anyone rides.
pub const ENDLESS_M: f64 = 40_000.0;
/// Hazards in an endless ride never come closer than this, metres.
pub const MIN_ENDLESS_GAP: f64 = 22.0;
/// How often a gap between hazards holds a parcel.
pub const PARCEL_CHANCE: f64 = 0.6;
/// Where parcels sit and what each pays: in a shop doorway by the parked
/// cars, in the door zone, or at a parcel locker on the traffic island by
/// the centre line, out past the cars, which pays more.
pub const PARCEL_SPOTS: [(f64, u32); 2] = [(2.6, 3), (6.5, 6)];
/// Seconds between oncoming cars at the start of an endless ride.
pub const ONCOMING_GAP: f64 = 22.0;

/// The kinds in the order `Mix::weights` lists them.
const KINDS: [HazardKind; 4] = [
    HazardKind::Door,
    HazardKind::Pothole,
    HazardKind::Curb,
    HazardKind::Ramp,
];

/// How much closer together an endless ride's hazards are `s` metres in:
/// 1 at the start, down to half by 3 kilometres.
pub fn endless_squeeze(s: f64) -> f64 {
    (1.0 - s / 6000.0).max(0.5)
}

/// One of the Downtown time trial courses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CourseInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub seed: u64,
    pub length: f64,
    /// Par time, seconds: the time worth 100,000 ride points.
    pub par: f64,
    /// Where it runs, after its length: "from the station to the post office".
    pub way: &'static str,
}

impl CourseInfo {
    /// "600 metres from the station to the post office.", in the player's units.
    pub fn about(&self) -> String {
        format!("{} {}.", parlor_core::units::route(self.length), self.way)
    }
}

/// The three Downtown courses, easiest first.
pub const COURSES: [CourseInfo; 3] = [
    CourseInfo {
        id: "station",
        name: "Station Run",
        seed: 0x5743_4e31,
        length: 600.0,
        par: 80.0,
        way: "from the station to the post office",
    },
    CourseInfo {
        id: "market",
        name: "Market Street",
        seed: 0x4d4b_5432,
        length: 900.0,
        par: 120.0,
        way: "down Market Street, past the shops",
    },
    CourseInfo {
        id: "bank",
        name: "Bank Square",
        seed: 0x424e_4b33,
        length: 1300.0,
        par: 170.0,
        way: "through the business district to Bank Square",
    },
];

impl Course {
    /// A Downtown time trial course.
    pub fn downtown(info: &CourseInfo) -> Course {
        Course::generate(info.name, info.seed, info.length, Mix::DOWNTOWN)
    }

    /// A course from a seed: hazards spaced along it, never two in one
    /// place, and a junction turn or two in the gaps.
    pub fn generate(name: &str, seed: u64, length: f64, mix: Mix) -> Course {
        let mut rng = Rng::seeded(seed);
        let mut hazards = Vec::new();
        let mut junctions = Vec::new();
        let mut slots = Vec::new();
        let span = |rng: &mut Rng, lo: f64, hi: f64| lo + (hi - lo) * rng.unit();
        let mut s = 60.0 + span(&mut rng, 0.0, 20.0);
        while s < length - 50.0 {
            slots.push(s);
            s += span(&mut rng, mix.gap.0, mix.gap.1);
        }
        // Junctions take evenly spread slots, never the first.
        let mut junction_slots = Vec::new();
        if mix.junctions > 0 && slots.len() > 3 {
            let n = mix.junctions as usize;
            for j in 0..n {
                let at = 1 + (j + 1) * (slots.len() - 1) / (n + 1);
                if !junction_slots.contains(&at) {
                    junction_slots.push(at);
                }
            }
        }
        let total: u32 = mix.weights.iter().sum();
        for (i, &s) in slots.iter().enumerate() {
            if junction_slots.contains(&i) {
                let turn = if rng.chance(0.5) {
                    Turn::Left
                } else {
                    Turn::Right
                };
                junctions.push(Junction { s, turn });
                continue;
            }
            let mut pick = rng.below(total.max(1));
            let mut kind = HazardKind::Door;
            for (k, &w) in [
                HazardKind::Door,
                HazardKind::Pothole,
                HazardKind::Curb,
                HazardKind::Ramp,
            ]
            .iter()
            .zip(&mix.weights)
            {
                if pick < w {
                    kind = *k;
                    break;
                }
                pick -= w;
            }
            hazards.push(Course::place(kind, s, &mut rng));
        }
        Course {
            name: name.to_string(),
            seed,
            length,
            hazards,
            junctions,
            traffic: true,
            car_gap: mix.car_gap,
            parcels: Vec::new(),
            oncoming_gap: 0.0,
            surface: Surface::Dry,
        }
    }

    /// An endless ride through `district`: hazards that come closer
    /// together the further you go, parcels to scoop up between them, and
    /// oncoming cars.
    pub fn endless(district: District, seed: u64) -> Course {
        let mix = district.mix();
        let mut rng = Rng::seeded(seed);
        let total: u32 = mix.weights.iter().sum();
        let mut hazards = Vec::new();
        let mut parcels = Vec::new();
        let mut s = 60.0 + rng.unit() * 20.0;
        while s < ENDLESS_M - 50.0 {
            let mut pick = rng.below(total.max(1));
            let mut kind = HazardKind::Door;
            for (k, &w) in KINDS.iter().zip(&mix.weights) {
                if pick < w {
                    kind = *k;
                    break;
                }
                pick -= w;
            }
            hazards.push(Course::place(kind, s, &mut rng));
            let squeeze = endless_squeeze(s);
            let gap = (mix.gap.0 + (mix.gap.1 - mix.gap.0) * rng.unit()) * squeeze;
            let gap = gap.max(MIN_ENDLESS_GAP);
            // A parcel in some gaps, halfway between hazards.
            if rng.chance(PARCEL_CHANCE) && gap >= 24.0 {
                let (x, value) = PARCEL_SPOTS[rng.below(PARCEL_SPOTS.len() as u32) as usize];
                parcels.push(ParcelSpot {
                    s: s + gap / 2.0,
                    x,
                    value,
                });
            }
            s += gap;
        }
        Course {
            name: format!("Endless {}", district.name()),
            seed,
            length: ENDLESS_M,
            hazards,
            junctions: Vec::new(),
            traffic: true,
            car_gap: mix.car_gap,
            parcels,
            oncoming_gap: ONCOMING_GAP,
            surface: district.surface(),
        }
    }

    /// A hazard of `kind` at `s`, across the stretch of road it belongs in.
    pub fn place(kind: HazardKind, s: f64, rng: &mut Rng) -> Hazard {
        let (x0, x1) = match kind {
            HazardKind::Door => (PARKED_EDGE, DOOR_EDGE),
            HazardKind::Curb => (PARKED_EDGE, BIKE_LANE_LINE),
            HazardKind::Ramp => (2.3, 3.3),
            HazardKind::Pothole => {
                let x = 2.4 + rng.unit() * (6.6 - 2.4);
                (x - POTHOLE_HALF, x + POTHOLE_HALF)
            }
        };
        Hazard { kind, s, x0, x1 }
    }

    /// An empty straight with no traffic, for training.
    pub fn empty(name: &str, length: f64) -> Course {
        Course {
            name: name.to_string(),
            seed: 0,
            length,
            hazards: Vec::new(),
            junctions: Vec::new(),
            traffic: false,
            car_gap: 0.0,
            parcels: Vec::new(),
            oncoming_gap: 0.0,
            surface: Surface::Dry,
        }
    }

    /// Adds a hazard, keeping them in order.
    pub fn with(mut self, hazard: Hazard) -> Course {
        self.hazards.push(hazard);
        self.hazards.sort_by(|a, b| a.s.total_cmp(&b.s));
        self
    }

    /// Adds a junction, keeping them in order.
    pub fn with_turn(mut self, junction: Junction) -> Course {
        self.junctions.push(junction);
        self.junctions.sort_by(|a, b| a.s.total_cmp(&b.s));
        self
    }

    /// Without cars from behind.
    pub fn quiet(mut self) -> Course {
        self.traffic = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_course() {
        let a = Course::downtown(&COURSES[1]);
        let b = Course::downtown(&COURSES[1]);
        assert_eq!(a, b);
        let c = Course::generate("Other", 99, 900.0, Mix::DOWNTOWN);
        assert_ne!(a.hazards, c.hazards);
    }

    #[test]
    fn hazards_are_spaced_and_in_order() {
        for seed in 0..50 {
            let c = Course::generate("Test", seed, 1000.0, Mix::DOWNTOWN);
            let mut all: Vec<f64> = c.hazards.iter().map(|h| h.s).collect();
            all.extend(c.junctions.iter().map(|j| j.s));
            all.sort_by(f64::total_cmp);
            for pair in all.windows(2) {
                assert!(pair[1] - pair[0] >= Mix::DOWNTOWN.gap.0 - 1e-9);
            }
            assert!(c.hazards.windows(2).all(|p| p[0].s < p[1].s));
            assert!(all.iter().all(|s| (60.0..950.0).contains(s)));
        }
    }

    #[test]
    fn every_downtown_course_leaves_the_traffic_lane_open() {
        for info in &COURSES {
            let c = Course::downtown(info);
            assert!(!c.hazards.is_empty());
            assert_eq!(c.junctions.len(), 2, "{}", info.name);
            for h in &c.hazards {
                assert!(h.x0 >= PARKED_EDGE && h.x1 <= CENTRE_LINE);
                if h.kind != HazardKind::Pothole {
                    assert!(h.x1 <= BIKE_LANE_LINE);
                }
            }
        }
    }

    #[test]
    fn turn_bands_are_the_outer_thirds() {
        let (lo, hi) = Turn::Right.band();
        assert_eq!(lo, PARKED_EDGE);
        assert!(hi > BIKE_LANE_LINE - 0.1);
        let (lo, hi) = Turn::Left.band();
        assert_eq!(hi, CENTRE_LINE);
        assert!(lo > CAR_LANE_X);
    }
}
