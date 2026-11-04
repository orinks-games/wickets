//! The daily, weekly and monthly rides: the same course and bike for every
//! player in the period, built from the challenge's seed.

use parlor_core::{Challenge, Date, Period};

use crate::bike::Bike;
use crate::course::{Course, Mix};

/// What a challenge ride is.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub period: Period,
    pub bike: Bike,
    pub course: Course,
    pub traffic_seed: u64,
    /// Seconds worth 100,000 ride points.
    pub par: f64,
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Morning run",
        Period::Weekly => "Rush hour",
        Period::Monthly => "Cross-town ride",
    }
}

/// The daily bike: the road bike and the BMX take turns by weekday.
pub fn daily_bike(ch: &Challenge) -> Bike {
    let weekday = Date::parse(&ch.key).map(|d| d.weekday()).unwrap_or(0);
    Bike::STARTERS[(weekday % 2) as usize]
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        let mut rng = ch.rng();
        let seed = rng.next_u64();
        let traffic_seed = rng.next_u64();
        let pick = rng.below(2);
        let (length, mix, bike) = match ch.period {
            // The daily bike takes turns by the day.
            Period::Daily => (700.0, Mix::DOWNTOWN, daily_bike(ch)),
            // Rush hour: cars twice as often.
            Period::Weekly => (
                1200.0,
                Mix {
                    car_gap: 7.0,
                    ..Mix::DOWNTOWN
                },
                Bike::STARTERS[pick as usize],
            ),
            // Across town: more doors and ramps, three junctions.
            Period::Monthly => (
                2200.0,
                Mix {
                    weights: [5, 3, 2, 3],
                    junctions: 3,
                    ..Mix::DOWNTOWN
                },
                Bike::STARTERS[pick as usize],
            ),
        };
        Plan {
            period: ch.period,
            bike,
            course: Course::generate(name(ch.period), seed, length, mix),
            traffic_seed,
            par: length / 7.5,
        }
    }

    /// "700 metres on the BMX".
    pub fn tag(&self) -> String {
        let extra = match self.period {
            Period::Daily => "",
            Period::Weekly => ", in heavy traffic",
            Period::Monthly => ", with 3 turns",
        };
        format!(
            "{} on the {}{extra}",
            parlor_core::units::route(self.course.length),
            self.bike.name()
        )
    }

    /// The rule in a sentence or two.
    pub fn rule(&self) -> String {
        format!(
            "{} of Downtown on the {}. Ride points count: speed, clean riding, jumps and tips.",
            parlor_core::units::route(self.course.length),
            self.bike.name()
        )
    }
}
