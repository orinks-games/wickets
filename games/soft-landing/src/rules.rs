//! The rules of a landing, as plain structs with no screen or sound: the
//! moons, the lander's fall and burn on fixed substeps, call-outs, fuel
//! warnings, touchdown, the score and the hint.
//!
//! Altitude is in metres above the pad and velocity in metres a second,
//! up positive, so a falling lander has a negative velocity and its
//! descent speed is `-velocity`. Fuel is counted in seconds of burn.

use parlor_core::{FixedStep, Rng};

/// Substeps a second. Every frame rate runs the same substeps.
pub const STEP_HZ: f64 = 120.0;

/// Points for any landing, before the fuel and softness bonuses.
pub const LANDED_POINTS: u32 = 200;
/// The most the fuel bonus pays, for a full tank.
pub const FUEL_POINTS: f64 = 300.0;
/// The most the softness bonus pays, for a touchdown at no speed.
pub const SOFT_POINTS: f64 = 500.0;

/// The call-out intervals the player can choose, in metres, least often
/// first.
pub const CALL_EVERY: [u32; 3] = [100, 50, 10];
/// The call-out interval on a fresh save.
pub const DEFAULT_CALL_EVERY: u32 = 50;
/// The practice pad's call-out interval.
pub const PRACTICE_CALL_EVERY: u32 = 10;
/// With US units, each interval is this many feet for every metre in
/// `CALL_EVERY`: 300, 150 and 30 feet, close to the metric marks.
pub const FEET_PER_CALL_METRE: u32 = 3;
const METRES_PER_FOOT: f64 = 0.3048;

/// The call-out interval as the player hears it: `every` metres, or
/// three times as many feet with US units.
pub fn shown_call_every(every: u32, feet: bool) -> u32 {
    if feet {
        every * FEET_PER_CALL_METRE
    } else {
        every
    }
}

/// How far over the safe speed still lands and still counts as safe:
/// anything spoken as the safe speed lands, and Too fast waits for more.
pub const LAND_TOLERANCE: f64 = 0.05;

/// The shortest time between two altitude call-outs, so a fast fall at
/// a short interval does not crowd speech.
pub const CALL_GAP: f64 = 1.0;

/// The fuel left, in percent, at which the low fuel warning sounds.
pub const FUEL_WARNINGS: [u32; 2] = [25, 10];

/// The descent speed a little under the safe speed counts as safe again,
/// so the spoken threshold does not flicker at the line.
pub const BAND_MARGIN: f64 = 0.1;

/// How many survey moons there are.
pub const MOONS: u32 = 3;

/// One moon and its descent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moon {
    pub name: &'static str,
    /// Gravity, in metres a second squared.
    pub gravity: f64,
    /// Start height above the pad, in metres.
    pub height: f64,
    /// Descent speed at the start, in metres a second.
    pub drop: f64,
    /// The main engine's push, in metres a second squared.
    pub thrust: f64,
    /// The fuel load, in seconds of burn.
    pub fuel: f64,
    /// The fastest touchdown that still lands, in metres a second.
    pub safe: f64,
}

/// The survey moons, in order.
pub const SURVEY: [Moon; MOONS as usize] = [
    Moon {
        name: "Thistle",
        gravity: 1.2,
        height: 300.0,
        drop: 5.0,
        thrust: 3.0,
        fuel: 30.0,
        safe: 3.0,
    },
    Moon {
        name: "Marrow",
        gravity: 1.8,
        height: 450.0,
        drop: 10.0,
        thrust: 4.0,
        fuel: 30.0,
        safe: 3.0,
    },
    Moon {
        name: "Ember",
        gravity: 2.6,
        height: 600.0,
        drop: 15.0,
        thrust: 5.0,
        fuel: 26.0,
        safe: 2.5,
    },
];

/// Survey moon `n`, counting from 1.
pub fn survey_moon(n: u32) -> Moon {
    SURVEY[(n.clamp(1, MOONS) - 1) as usize]
}

/// Names for challenge moons, which TTS reads cleanly.
const NAMES: [&str; 8] = [
    "Saffron", "Quill", "Harrow", "Wren", "Tallow", "Brindle", "Juniper", "Corrie",
];

impl Moon {
    /// The engine's push left after gravity, in metres a second squared.
    pub fn net(&self) -> f64 {
        self.thrust - self.gravity
    }

    /// "light gravity", "medium gravity", "heavy gravity".
    pub fn gravity_words(&self) -> &'static str {
        if self.gravity < 1.5 {
            "light gravity"
        } else if self.gravity < 2.3 {
            "medium gravity"
        } else {
            "heavy gravity"
        }
    }

    /// The burn, in seconds, of the best single burn from the start: fall
    /// freely, then hold the engine down to the safe speed at the pad.
    pub fn needed_burn(&self) -> f64 {
        let a = self.net();
        let vs = self.safe;
        let h = ((self.drop * self.drop + 2.0 * self.gravity * self.height - vs * vs)
            / (2.0 * (a + self.gravity)))
            .clamp(0.0, self.height);
        let v = (vs * vs + 2.0 * a * h).sqrt();
        ((v - vs) / a).max(0.0)
    }

    /// A seeded moon for a challenge. `margin` is how many times the best
    /// burn the tank holds; `hard` makes it heavier, higher and stricter.
    pub fn seeded(rng: &mut Rng, margin: f64, hard: bool) -> Moon {
        let name = *rng.pick(&NAMES);
        let (g_lo, g_hi) = if hard { (2.4, 3.2) } else { (1.0, 2.6) };
        let gravity = round1(g_lo + (g_hi - g_lo) * rng.unit());
        let thrust = round1(gravity * (1.8 + 0.5 * rng.unit()));
        let height = if hard {
            rng.range(55, 75) as f64 * 10.0
        } else {
            rng.range(30, 60) as f64 * 10.0
        };
        let drop = if hard {
            rng.range(10, 16) as f64
        } else {
            rng.range(3, 12) as f64
        };
        let safe = if hard { 2.5 } else { 3.0 };
        let mut moon = Moon {
            name,
            gravity,
            height,
            drop,
            thrust,
            fuel: 0.0,
            safe,
        };
        moon.fuel = (moon.needed_burn() * margin).ceil();
        moon
    }
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// How the descent speed sits against the safe speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Safe,
    TooFast,
}

/// How a flight ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Down at this speed, no faster than the safe speed.
    Landed { speed: f64 },
    /// Down too fast, at this speed.
    Crashed { speed: f64 },
}

impl Outcome {
    pub fn landed(self) -> bool {
        matches!(self, Outcome::Landed { .. })
    }

    pub fn speed(self) -> f64 {
        match self {
            Outcome::Landed { speed } | Outcome::Crashed { speed } => speed,
        }
    }
}

/// What a flight's update heard happen, for the screen to voice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ev {
    EngineOn,
    EngineOff,
    /// The tank ran dry while the engine burned.
    FuelOut,
    /// The fuel fell to this percent.
    FuelLow(u32),
    /// The descent speed went over the safe speed.
    TooFast,
    /// The descent speed came back under it.
    SafeSpeed,
    /// The lander passed an altitude mark, in feet when `Flight::feet` is
    /// set, falling at this speed in metres a second.
    CallOut {
        altitude: u32,
        speed: f64,
    },
    /// The pad beacon pinged.
    Beacon,
    Touchdown(Outcome),
}

/// One descent onto one moon.
#[derive(Clone, Debug)]
pub struct Flight {
    pub moon: Moon,
    pub altitude: f64,
    /// Metres a second, up positive.
    pub velocity: f64,
    /// Seconds of burn left.
    pub fuel: f64,
    /// The tank's size, for the percentage and the score.
    pub capacity: f64,
    /// The practice pad: fuel never drains.
    pub unlimited: bool,
    /// Seconds of flight since the descent began.
    pub time: f64,
    /// The engine is burning now.
    pub engine: bool,
    pub band: Option<Band>,
    /// Metres between altitude call-outs (see [`shown_call_every`] for feet).
    pub call_every: u32,
    /// Call-outs mark and name feet, not metres, for US units.
    pub feet: bool,
    /// The next altitude mark to call.
    marker: f64,
    since_call: f64,
    warned: [bool; 2],
    beacon_in: f64,
    started: bool,
    pub outcome: Option<Outcome>,
    clock: FixedStep,
}

impl Flight {
    /// A flight onto `moon` with its own full tank.
    pub fn new(moon: Moon) -> Flight {
        Flight::with_fuel(moon, moon.fuel, moon.fuel)
    }

    /// A flight with `fuel` left in a tank of `capacity`, as the weekly
    /// challenge carries one load across three moons.
    pub fn with_fuel(moon: Moon, fuel: f64, capacity: f64) -> Flight {
        let mut f = Flight {
            moon,
            altitude: moon.height,
            velocity: -moon.drop,
            fuel: fuel.max(0.0),
            capacity: capacity.max(1.0),
            unlimited: false,
            time: 0.0,
            engine: false,
            band: None,
            call_every: DEFAULT_CALL_EVERY,
            feet: parlor_core::units::us(),
            marker: 0.0,
            since_call: CALL_GAP,
            warned: [false; 2],
            beacon_in: 0.0,
            started: false,
            outcome: None,
            clock: FixedStep::hz(STEP_HZ),
        };
        f.reset_marker();
        f
    }

    /// A practice flight: the first moon with fuel that never drains.
    pub fn practice() -> Flight {
        let mut f = Flight::new(SURVEY[0]);
        f.unlimited = true;
        f.call_every = PRACTICE_CALL_EVERY;
        f.reset_marker();
        f
    }

    /// The fastest gentle descent. "Too fast", the tone and the hint all
    /// go by it.
    pub fn limit(&self) -> f64 {
        self.moon.safe
    }

    /// Descent speed in metres a second; negative while climbing.
    pub fn speed(&self) -> f64 {
        -self.velocity
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }

    /// Fuel left, in whole percent of the tank.
    pub fn fuel_percent(&self) -> u32 {
        if self.unlimited {
            return 100;
        }
        ((self.fuel / self.capacity) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u32
    }

    /// Lets the lander fall.
    pub fn start(&mut self) {
        self.started = true;
    }

    /// Changes the call-out interval and finds the next mark below.
    pub fn set_call_every(&mut self, metres: u32) {
        self.call_every = metres.max(1);
        self.reset_marker();
    }

    /// Metres between call-out marks.
    fn mark_gap(&self) -> f64 {
        if self.feet {
            f64::from(shown_call_every(self.call_every, true)) * METRES_PER_FOOT
        } else {
            f64::from(self.call_every)
        }
    }

    /// Whether call-outs name feet or metres, and re-finds the next mark.
    pub fn set_feet(&mut self, feet: bool) {
        self.feet = feet;
        self.reset_marker();
    }

    /// The highest mark strictly below the lander.
    fn reset_marker(&mut self) {
        let every = self.mark_gap();
        let below = ((self.altitude - 1e-6) / every).floor() * every;
        self.marker = below.max(0.0);
    }

    fn band_for(&self, speed: f64) -> Band {
        match self.band {
            Some(Band::TooFast) if speed > self.limit() - BAND_MARGIN => Band::TooFast,
            _ if speed > self.limit() + LAND_TOLERANCE => Band::TooFast,
            _ => Band::Safe,
        }
    }

    /// Runs `dt` seconds of frame time, with the engine key held or not.
    pub fn update(&mut self, dt: f64, held: bool) -> Vec<Ev> {
        let mut evs = Vec::new();
        if !self.started || self.finished() {
            return evs;
        }
        let mut clock = self.clock;
        clock.advance(dt, |step| {
            if self.outcome.is_none() {
                self.substep(step, held, &mut evs);
            }
        });
        self.clock = clock;
        evs
    }

    fn substep(&mut self, step: f64, held: bool, evs: &mut Vec<Ev>) {
        let moon = self.moon;
        let burning = held && (self.unlimited || self.fuel > 0.0);
        if burning != self.engine {
            self.engine = burning;
            evs.push(if burning { Ev::EngineOn } else { Ev::EngineOff });
        }
        let accel = if burning {
            moon.thrust - moon.gravity
        } else {
            -moon.gravity
        };
        // Constant push over a substep, so this is exact.
        self.altitude += self.velocity * step + 0.5 * accel * step * step;
        self.velocity += accel * step;
        self.time += step;
        self.since_call += step;
        if burning && !self.unlimited {
            let before = self.fuel / self.capacity * 100.0;
            self.fuel = (self.fuel - step).max(0.0);
            let after = self.fuel / self.capacity * 100.0;
            for (i, level) in FUEL_WARNINGS.into_iter().enumerate() {
                if !self.warned[i] && before > level as f64 && after <= level as f64 {
                    self.warned[i] = true;
                    evs.push(Ev::FuelLow(level));
                }
            }
            if self.fuel <= 0.0 {
                self.engine = false;
                evs.push(Ev::FuelOut);
            }
        }
        if self.altitude <= 0.0 {
            self.altitude = 0.0;
            let speed = self.speed().max(0.0);
            let outcome = if speed <= moon.safe + LAND_TOLERANCE {
                Outcome::Landed { speed }
            } else {
                Outcome::Crashed { speed }
            };
            self.velocity = 0.0;
            if self.engine {
                self.engine = false;
                evs.push(Ev::EngineOff);
            }
            self.outcome = Some(outcome);
            evs.push(Ev::Touchdown(outcome));
            return;
        }
        let band = self.band_for(self.speed());
        if self.band != Some(band) {
            self.band = Some(band);
            evs.push(match band {
                Band::Safe => Ev::SafeSpeed,
                Band::TooFast => Ev::TooFast,
            });
        }
        let every = self.mark_gap();
        if self.altitude > self.marker + every {
            self.reset_marker();
        } else if self.marker > 0.0 && self.altitude <= self.marker {
            if self.since_call >= CALL_GAP {
                self.since_call = 0.0;
                evs.push(Ev::CallOut {
                    altitude: if self.feet {
                        (self.marker / METRES_PER_FOOT).round() as u32
                    } else {
                        self.marker.round() as u32
                    },
                    speed: self.speed(),
                });
            }
            self.reset_marker();
        }
        self.beacon_in -= step;
        if self.beacon_in <= 0.0 {
            self.beacon_in = beacon_gap(self.altitude, moon.height);
            evs.push(Ev::Beacon);
        }
    }

    /// The score this flight would earn if it ended now in `outcome`.
    pub fn score(&self, outcome: Outcome) -> u32 {
        match outcome {
            Outcome::Crashed { .. } => 0,
            Outcome::Landed { speed } => {
                landing_points(&self.moon, speed) + fuel_points(self.fuel, self.capacity)
            }
        }
    }
}

/// Seconds between beacon pings: quick near the pad, slow high above it.
pub fn beacon_gap(altitude: f64, height: f64) -> f64 {
    0.4 + 1.6 * (altitude / height.max(1.0)).clamp(0.0, 1.0)
}

/// The landing points without the fuel: the landing itself and the
/// softness bonus.
pub fn landing_points(moon: &Moon, speed: f64) -> u32 {
    let soft = ((moon.safe - speed) / moon.safe).clamp(0.0, 1.0) * SOFT_POINTS;
    LANDED_POINTS + soft.round() as u32
}

/// The fuel bonus for `fuel` left in a tank of `capacity`.
pub fn fuel_points(fuel: f64, capacity: f64) -> u32 {
    ((fuel / capacity.max(1.0)).clamp(0.0, 1.0) * FUEL_POINTS).round() as u32
}

/// What the hint advises.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Advice {
    /// Not falling yet.
    Ready,
    /// Going up: let go.
    Climbing,
    /// Speed is safe already.
    Steady,
    /// Fall freely for about this many seconds, then burn.
    Coast { secs: u32 },
    /// Burn now and hold for about this many seconds.
    Burn { secs: u32 },
    /// No fuel left to burn.
    NoFuel,
}

/// What to do now, for the Hint key. A burn is never advised for longer
/// than the fuel left.
pub fn advice(f: &Flight) -> Advice {
    if !f.started {
        return Advice::Ready;
    }
    let moon = &f.moon;
    let limit = f.limit();
    let s = f.speed();
    if s < -0.2 {
        return Advice::Climbing;
    }
    if s <= limit * 0.85 {
        return Advice::Steady;
    }
    let fuel = if f.unlimited { f64::INFINITY } else { f.fuel };
    if fuel <= 0.0 {
        return Advice::NoFuel;
    }
    let a = moon.net();
    let target = limit * 0.7;
    let g = moon.gravity;
    // The height to start one burn from, with a margin.
    let burn_at = ((s * s + 2.0 * g * f.altitude - target * target) / (2.0 * (a + g))).max(0.0);
    let start = burn_at * 1.25 + 5.0;
    if f.altitude > start + 5.0 {
        // Time to fall from here to `start`: 0.5 g t² + s t = drop.
        let drop = f.altitude - start;
        let t = (-s + (s * s + 2.0 * g * drop).sqrt()) / g;
        let secs = t.floor().max(1.0) as u32;
        return Advice::Coast { secs };
    }
    if fuel < 1.0 {
        // Under a second of fuel: burn the last of it.
        return Advice::Burn { secs: 0 };
    }
    let need = (s - target) / a;
    let secs = need.floor().max(1.0).min(fuel.floor());
    Advice::Burn { secs: secs as u32 }
}

/// The next call-out interval, more often (`more`) or less often, or
/// `None` at the end of the list.
pub fn next_call_every(now: u32, more: bool) -> Option<u32> {
    let at = CALL_EVERY.iter().position(|e| *e == now).unwrap_or(1);
    if more {
        CALL_EVERY.get(at + 1).copied()
    } else {
        at.checked_sub(1).map(|i| CALL_EVERY[i])
    }
}

/// "1 second", "3 seconds".
pub fn seconds_words(n: u32) -> String {
    if n == 1 {
        "1 second".to_string()
    } else {
        format!("{n} seconds")
    }
}

/// A speed in words with one decimal: "2.4 metres a second", or feet a
/// second with US units.
pub fn speed_words(speed: f64) -> String {
    let v = round1(parlor_core::units::from_metres(speed.max(0.0)));
    let unit = parlor_core::units::plural();
    if v == 1.0 {
        let one = if parlor_core::units::us() {
            "foot"
        } else {
            "metre"
        };
        format!("1 {one} a second")
    } else if v.fract() == 0.0 {
        format!("{v:.0} {unit} a second")
    } else {
        format!("{v:.1} {unit} a second")
    }
}

/// How the fall sounds in words, for Status.
pub fn fall_words(f: &Flight) -> &'static str {
    let s = f.speed();
    let safe = f.limit();
    if s < -0.2 {
        "climbing"
    } else if s <= safe {
        "slow and steady"
    } else if s <= safe * 2.0 {
        "a little fast"
    } else {
        "falling fast"
    }
}

/// The descent tone's pitch for a descent speed: low and slow is safe,
/// high and fast is danger.
pub fn tone_pitch(speed: f64, safe: f64) -> f32 {
    (0.6 + 0.4 * (speed.max(0.0) / safe.max(0.1))).clamp(0.5, 2.2) as f32
}
