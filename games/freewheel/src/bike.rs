//! The ride model: what each bike is, what the player asks of it, and how
//! a rider moves. Plain structs and pure functions with no audio or `Ctx`,
//! so tests drive them headless with a fixed `dt`.
//!
//! Per substep: pedal power through a force cap, against rolling
//! resistance and air drag, with brakes that build over a moment. Classic
//! steering slides sideways at the bike's own rate. A hop lifts the rider
//! by the bike's hop height and gravity brings it back down.

use serde::{Deserialize, Serialize};

/// Gravity, metres a second squared.
pub const G: f64 = 9.81;
/// Air density, kilograms a cubic metre.
pub const RHO: f64 = 1.2;
/// The player's sustained pedal power, watts.
pub const RIDER_POWER_W: f64 = 250.0;
/// Extra power while sprinting, watts.
pub const SPRINT_POWER_W: f64 = 350.0;
/// Seconds for the brakes to reach full force.
pub const BRAKE_RAMP_S: f64 = 0.3;
/// Share of the tyre's grip the brakes can use.
pub const BRAKE_SHARE: f64 = 0.85;
/// Stamina a second of sprinting uses, out of 1.
pub const SPRINT_DRAIN: f64 = 1.0 / 8.0;
/// Stamina a second of not sprinting gives back.
pub const SPRINT_REFILL: f64 = 1.0 / 20.0;
/// Seconds on the ground after a crash.
pub const DOWN_S: f64 = 3.0;
/// Seconds standing still that swap a flat tube.
pub const TUBE_SWAP_S: f64 = 3.0;
/// Below this speed the bike cannot hop or steer.
pub const MIN_MOVING: f64 = 1.0;
/// Below this speed the bike counts as stopped.
pub const STOPPED: f64 = 0.05;

/// A tyre: its grip and how it meets rough ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tyre {
    /// 25 millimetre road slicks.
    Slick25,
    /// 50 millimetre BMX tyres.
    Bmx50,
    /// 55 millimetre knobbly mountain bike tyres.
    Knobby55,
    /// 40 millimetre e-bike tyres with a little tread.
    Hybrid40,
    /// A moped's small, fat tyres.
    Scooter,
}

/// What the road is made of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Surface {
    /// Dry asphalt.
    #[default]
    Dry,
    /// Wet asphalt after rain: less grip.
    Wet,
    /// Old cobbles: rough, and slow on thin tyres.
    Cobbles,
}

impl Tyre {
    /// Wide tyres roll through potholes and survive a curb strike.
    pub fn wide(self) -> bool {
        matches!(self, Tyre::Bmx50 | Tyre::Knobby55 | Tyre::Scooter)
    }

    /// Grip on dry asphalt.
    pub fn mu_dry(self) -> f64 {
        0.8
    }

    /// Grip on this surface.
    pub fn mu(self, surface: Surface) -> f64 {
        match surface {
            Surface::Dry => self.mu_dry(),
            Surface::Wet => match self {
                Tyre::Slick25 => 0.45,
                Tyre::Bmx50 | Tyre::Hybrid40 => 0.55,
                Tyre::Knobby55 | Tyre::Scooter => 0.6,
            },
            Surface::Cobbles => 0.7,
        }
    }

    /// How much harder this tyre rolls on this surface than on asphalt.
    pub fn crr_scale(self, surface: Surface) -> f64 {
        match surface {
            Surface::Dry | Surface::Wet => 1.0,
            Surface::Cobbles => match self {
                Tyre::Slick25 => 8.0,
                Tyre::Hybrid40 => 3.0,
                Tyre::Bmx50 | Tyre::Scooter => 1.6,
                Tyre::Knobby55 => 1.2,
            },
        }
    }
}

/// One kind of bike.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BikeSpec {
    pub id: &'static str,
    /// Rider and bike, kilograms.
    pub mass_kg: f64,
    /// Drag area, square metres.
    pub cda_m2: f64,
    /// Rolling resistance on smooth asphalt.
    pub crr: f64,
    /// Pedal force cap at low speed, newtons.
    pub max_force_n: f64,
    /// Top-gear cadence limit: power fades to 0 by 1.1 times this.
    pub spin_out_ms: f64,
    /// Hop height, metres.
    pub hop_m: f64,
    /// Classic steering's sideways speed, metres a second.
    pub steer_rate_ms: f64,
    pub tyre: Tyre,
    /// Handlebar width, metres.
    pub width_m: f64,
    /// An e-bike's motor: watts added while pedalling, up to its cut-off
    /// speed in metres a second.
    pub assist_w: f64,
    pub assist_cut_ms: f64,
    /// A moped's engine, watts: it replaces pedalling, and there is no sprint.
    pub engine_w: f64,
}

/// The road bike: fastest on the flat, a low hop.
pub const ROAD_BIKE: BikeSpec = BikeSpec {
    id: "road",
    mass_kg: 80.0,
    cda_m2: 0.30,
    crr: 0.0045,
    max_force_n: 120.0,
    spin_out_ms: 16.0,
    hop_m: 0.10,
    steer_rate_ms: 1.2,
    tyre: Tyre::Slick25,
    width_m: 0.6,
    assist_w: 0.0,
    assist_cut_ms: 0.0,
    engine_w: 0.0,
};

/// The BMX: quick off the line, spins out near 8.5 metres a second, a big
/// hop and the tightest steering.
pub const BMX: BikeSpec = BikeSpec {
    id: "bmx",
    mass_kg: 74.0,
    cda_m2: 0.40,
    crr: 0.008,
    max_force_n: 150.0,
    spin_out_ms: 8.4,
    hop_m: 0.40,
    steer_rate_ms: 1.5,
    tyre: Tyre::Bmx50,
    width_m: 0.6,
    assist_w: 0.0,
    assist_cut_ms: 0.0,
    engine_w: 0.0,
};

/// The mountain bike: knobbly tyres that shrug off cobbles, potholes and
/// wet roads, and a hop that clears a curb.
pub const MOUNTAIN_BIKE: BikeSpec = BikeSpec {
    id: "mtb",
    mass_kg: 84.0,
    cda_m2: 0.40,
    crr: 0.010,
    max_force_n: 160.0,
    spin_out_ms: 11.0,
    hop_m: 0.30,
    steer_rate_ms: 1.4,
    tyre: Tyre::Knobby55,
    width_m: 0.7,
    assist_w: 0.0,
    assist_cut_ms: 0.0,
    engine_w: 0.0,
};

/// The e-bike: heavy, with a motor that helps up to 25 kilometres an hour.
pub const E_BIKE: BikeSpec = BikeSpec {
    id: "ebike",
    mass_kg: 100.0,
    cda_m2: 0.40,
    crr: 0.006,
    max_force_n: 220.0,
    spin_out_ms: 12.0,
    hop_m: 0.08,
    steer_rate_ms: 1.2,
    tyre: Tyre::Hybrid40,
    width_m: 0.65,
    assist_w: 250.0,
    assist_cut_ms: 25.0 / 3.6,
    engine_w: 0.0,
};

/// The moped, after Cyclepath's motorbikes: an engine instead of pedals,
/// 50 kilometres an hour, and no hop at all.
pub const MOPED: BikeSpec = BikeSpec {
    id: "moped",
    mass_kg: 150.0,
    cda_m2: 0.55,
    crr: 0.010,
    max_force_n: 500.0,
    spin_out_ms: 13.9,
    hop_m: 0.0,
    steer_rate_ms: 1.6,
    tyre: Tyre::Scooter,
    width_m: 0.7,
    assist_w: 0.0,
    assist_cut_ms: 0.0,
    engine_w: 1800.0,
};

/// The bikes the player can ride.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Bike {
    #[default]
    Road,
    Bmx,
    Mountain,
    Electric,
    Moped,
}

impl Bike {
    pub const ALL: [Bike; 5] = [
        Bike::Road,
        Bike::Bmx,
        Bike::Mountain,
        Bike::Electric,
        Bike::Moped,
    ];
    /// The bikes every rider starts with, which the challenges use.
    pub const STARTERS: [Bike; 2] = [Bike::Road, Bike::Bmx];

    pub fn spec(self) -> &'static BikeSpec {
        match self {
            Bike::Road => &ROAD_BIKE,
            Bike::Bmx => &BMX,
            Bike::Mountain => &MOUNTAIN_BIKE,
            Bike::Electric => &E_BIKE,
            Bike::Moped => &MOPED,
        }
    }

    /// Whether its hop rises high enough to clear a curb.
    pub fn clears_curbs(self) -> bool {
        self.spec().hop_m >= crate::course::CURB_M + 0.02
    }

    /// Dollars it costs in the bike shop; 0 for a starter.
    pub fn price(self) -> u32 {
        match self {
            Bike::Road | Bike::Bmx => 0,
            Bike::Mountain => 150,
            Bike::Electric => 400,
            Bike::Moped => 1000,
        }
    }

    /// Whether it has an engine instead of pedals.
    pub fn motor(self) -> bool {
        self.spec().engine_w > 0.0
    }

    pub fn id(self) -> &'static str {
        self.spec().id
    }

    /// "road bike", "BMX".
    pub fn name(self) -> &'static str {
        match self {
            Bike::Road => "road bike",
            Bike::Bmx => "BMX",
            Bike::Mountain => "mountain bike",
            Bike::Electric => "e-bike",
            Bike::Moped => "moped",
        }
    }

    /// How it rides, in a sentence.
    pub fn about(self) -> String {
        match self {
            Bike::Road => format!(
                "The fastest on the flat, up to {}. Its hop is too low to clear a curb.",
                parlor_core::units::per_hour(37.0)
            ),
            Bike::Bmx => format!(
                "Quick off the line and tops out near {}. Its big hop clears curbs.",
                parlor_core::units::per_hour(31.0)
            ),
            Bike::Mountain => {
                "Knobbly tyres for cobbles, potholes and rain. Its hop clears curbs.".to_string()
            }
            Bike::Electric => format!(
                "A motor helps you up to {}. Its hop cannot clear a curb, and potholes flatten its tyres.",
                parlor_core::units::per_hour(25.0)
            ),
            Bike::Moped => format!(
                "An engine, not pedals: up to {}. It cannot hop or sprint.",
                parlor_core::units::per_hour(50.0)
            ),
        }
    }
}

/// What the player, or later an AI rider, asks of the bike this substep.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub pedal: bool,
    pub brake: bool,
    /// -1 steers right, toward the curb; 1 steers left, toward the centre line.
    pub steer: f32,
    /// A fresh hop press.
    pub hop: bool,
    pub sprint: bool,
}

/// The road under the rider: grip, rolling resistance scale and the edges
/// the bike can reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadAt {
    pub mu: f64,
    pub crr_scale: f64,
    /// The lowest and highest `x` the rider's centre can reach.
    pub x_min: f64,
    pub x_max: f64,
}

impl RoadAt {
    /// Flat, dry asphalt with no edges in reach.
    pub fn open() -> RoadAt {
        RoadAt {
            mu: 0.8,
            crr_scale: 1.0,
            x_min: f64::NEG_INFINITY,
            x_max: f64::INFINITY,
        }
    }
}

/// What a rider's own update can report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RideEvent {
    Hop,
    /// Back on the ground after `air` seconds in the air.
    Land {
        air: f64,
        hard: bool,
    },
    /// Up again after a crash.
    Up,
    /// A flat tube swapped while standing still.
    TubeSwapped,
    /// Stamina ran out mid-sprint.
    Spent,
    /// Steering met the edge of the road.
    Edge,
}

/// One rider on the road.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rider {
    /// Metres along the route.
    pub s: f64,
    /// Metres from the right-hand curb.
    pub x: f64,
    /// Speed along the route, metres a second.
    pub v: f64,
    /// Height above the road, and its upward speed.
    pub z: f64,
    pub vz: f64,
    /// Sprint stamina, 0 to 1.
    pub stamina: f64,
    /// How far the brakes have built, 0 to 1.
    pub brake_ramp: f64,
    /// Seconds left on the ground after a crash.
    pub down_for: f64,
    pub flat: bool,
    /// Seconds stood still, for swapping a flat.
    pub still_for: f64,
    /// Seconds in the air on this hop or jump.
    pub air_for: f64,
    /// Seconds since the last hop press, for the ramp's take-off window.
    pub since_hop: f64,
    /// Whether the last substep pedalled, sprinted and braked, for sound.
    pub pedalling: bool,
    pub sprinting: bool,
    pub braking: bool,
    /// Whether steering was pressed against an edge.
    pub at_edge: bool,
}

impl Rider {
    pub fn new(x: f64) -> Rider {
        Rider {
            s: 0.0,
            x,
            v: 0.0,
            z: 0.0,
            vz: 0.0,
            stamina: 1.0,
            brake_ramp: 0.0,
            down_for: 0.0,
            flat: false,
            still_for: 0.0,
            air_for: 0.0,
            since_hop: f64::INFINITY,
            pedalling: false,
            sprinting: false,
            braking: false,
            at_edge: false,
        }
    }

    pub fn airborne(&self) -> bool {
        self.z > 0.0
    }

    pub fn down(&self) -> bool {
        self.down_for > 0.0
    }

    /// Knocked off: stopped, on the ground for a while.
    pub fn crash(&mut self) {
        self.v = 0.0;
        self.z = 0.0;
        self.vz = 0.0;
        self.brake_ramp = 0.0;
        self.down_for = DOWN_S;
    }

    /// One substep of `dt` seconds with `power_w` of sustained pedal power
    /// and `sprint_w` more while sprinting.
    pub fn update(
        &mut self,
        spec: &BikeSpec,
        road: &RoadAt,
        input: Input,
        power_w: f64,
        sprint_w: f64,
        dt: f64,
    ) -> Vec<RideEvent> {
        let mut events = Vec::new();
        self.since_hop += dt;
        if self.down() {
            self.down_for -= dt;
            self.pedalling = false;
            self.sprinting = false;
            self.braking = false;
            if self.down_for <= 0.0 {
                self.down_for = 0.0;
                events.push(RideEvent::Up);
            }
            return events;
        }
        let input = if self.airborne() {
            Input {
                pedal: false,
                brake: false,
                hop: false,
                ..input
            }
        } else {
            input
        };

        // Sprint stamina.
        let sprinting = input.sprint && input.pedal && self.stamina > 0.0 && spec.engine_w == 0.0;
        if sprinting {
            self.stamina = (self.stamina - SPRINT_DRAIN * dt).max(0.0);
            if self.stamina == 0.0 {
                events.push(RideEvent::Spent);
            }
        } else if !input.sprint {
            self.stamina = (self.stamina + SPRINT_REFILL * dt).min(1.0);
        }

        // Along the route.
        let v = self.v;
        let fade = spin_fade(spec, v);
        let assist = if v < spec.assist_cut_ms {
            spec.assist_w
        } else {
            0.0
        };
        let power = if input.pedal {
            (power_w + assist + if sprinting { sprint_w } else { 0.0 }) * fade
        } else {
            0.0
        };
        let drive = (spec.max_force_n * road.mu / 0.8).min(power / v.max(0.5));
        let crr = spec.crr * road.crr_scale * if self.flat { 4.0 } else { 1.0 };
        let resist = crr * spec.mass_kg * G + 0.5 * RHO * spec.cda_m2 * v * v;
        self.brake_ramp = if input.brake {
            (self.brake_ramp + dt / BRAKE_RAMP_S).min(1.0)
        } else {
            0.0
        };
        let brake = if self.airborne() {
            0.0
        } else {
            self.brake_ramp * road.mu * spec.mass_kg * G * BRAKE_SHARE
        };
        let mut accel = drive / spec.mass_kg;
        if v > 0.0 || drive > crr * spec.mass_kg * G {
            accel -= (resist + brake) / spec.mass_kg;
        }
        self.v = (v + accel * dt).max(0.0);
        if self.v < STOPPED && !input.pedal {
            self.v = 0.0;
        }
        self.s += self.v * dt;
        self.pedalling = input.pedal && power > 0.0;
        self.sprinting = sprinting;
        self.braking = input.brake && self.v > 0.0;

        // Across the route: classic steering.
        self.at_edge = false;
        if input.steer != 0.0 && self.v >= MIN_MOVING {
            let x = self.x + input.steer as f64 * spec.steer_rate_ms * dt;
            let clamped = x.clamp(road.x_min, road.x_max);
            if clamped != x {
                self.at_edge = true;
                events.push(RideEvent::Edge);
            }
            self.x = clamped;
        }

        // Up and down.
        if input.hop {
            self.since_hop = 0.0;
            if !self.airborne() && self.v >= MIN_MOVING && spec.hop_m > 0.0 {
                self.vz = (2.0 * G * spec.hop_m).sqrt();
                self.air_for = 0.0;
                events.push(RideEvent::Hop);
            }
        }
        if self.airborne() || self.vz > 0.0 {
            self.vz -= G * dt;
            self.z += self.vz * dt;
            self.air_for += dt;
            if self.z <= 0.0 {
                let hard = self.vz < -4.0;
                self.z = 0.0;
                self.vz = 0.0;
                if hard {
                    self.v *= 0.8;
                }
                events.push(RideEvent::Land {
                    air: self.air_for,
                    hard,
                });
            }
        }

        // A flat tube is swapped by standing still for a moment.
        if self.v == 0.0 {
            self.still_for += dt;
            if self.flat && self.still_for >= TUBE_SWAP_S {
                self.flat = false;
                events.push(RideEvent::TubeSwapped);
            }
        } else {
            self.still_for = 0.0;
        }
        events
    }
}

/// How much pedal power the gears still turn into speed: 1 up to the
/// spin-out speed, falling to 0 at 1.1 times it.
pub fn spin_fade(spec: &BikeSpec, v: f64) -> f64 {
    if v <= spec.spin_out_ms {
        1.0
    } else {
        (1.0 - (v - spec.spin_out_ms) / (0.1 * spec.spin_out_ms)).max(0.0)
    }
}

/// How far the bike runs from speed `v` under full brakes on a road with
/// grip `mu`, by the same substeps the ride uses.
pub fn stopping_distance(spec: &BikeSpec, v: f64, mu: f64) -> f64 {
    let road = RoadAt {
        mu,
        ..RoadAt::open()
    };
    let mut rider = Rider::new(0.0);
    rider.v = v;
    let input = Input {
        brake: true,
        ..Input::default()
    };
    let dt = crate::world::SUBSTEP;
    let mut guard = 0;
    while rider.v > 0.0 && guard < 100_000 {
        rider.update(spec, &road, input, 0.0, 0.0, dt);
        guard += 1;
    }
    rider.s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::SUBSTEP;

    fn ride_for(spec: &BikeSpec, secs: f64, input: Input, mut each: impl FnMut(f64, &Rider)) {
        let road = RoadAt::open();
        let mut rider = Rider::new(3.0);
        let steps = (secs / SUBSTEP).round() as usize;
        for i in 1..=steps {
            rider.update(spec, &road, input, RIDER_POWER_W, SPRINT_POWER_W, SUBSTEP);
            each(i as f64 * SUBSTEP, &rider);
        }
    }

    fn pedal() -> Input {
        Input {
            pedal: true,
            ..Input::default()
        }
    }

    fn time_to(spec: &BikeSpec, speed: f64) -> f64 {
        let mut at = f64::INFINITY;
        ride_for(spec, 60.0, pedal(), |t, r| {
            if r.v >= speed && at.is_infinite() {
                at = t;
            }
        });
        at
    }

    #[test]
    fn road_bike_reaches_8_ms_within_16_s() {
        let t = time_to(&ROAD_BIKE, 8.0);
        assert!(t <= 16.0, "took {t} seconds");
    }

    #[test]
    fn road_bike_top_speed_10_4() {
        let mut v = 0.0;
        ride_for(&ROAD_BIKE, 60.0, pedal(), |_, r| v = r.v);
        assert!((v - 10.4).abs() <= 0.2, "settled at {v}");
    }

    #[test]
    fn bmx_is_quicker_to_4_ms_and_never_passes_9() {
        assert!(time_to(&BMX, 4.0) < time_to(&ROAD_BIKE, 4.0));
        let mut top: f64 = 0.0;
        ride_for(&BMX, 90.0, pedal(), |_, r| top = top.max(r.v));
        assert!(top <= 9.0, "reached {top}");
        assert!(top > 8.0, "only reached {top}");
    }

    #[test]
    fn dry_braking_from_8_ms_within_6_m() {
        let d = stopping_distance(&ROAD_BIKE, 8.0, Tyre::Slick25.mu_dry());
        assert!(d <= 6.0, "stopped in {d} metres");
        assert!(d > 3.0, "stopped in only {d} metres");
    }

    #[test]
    fn coasting_slows_monotonically_and_never_goes_negative() {
        let road = RoadAt::open();
        let mut rider = Rider::new(3.0);
        rider.v = 8.0;
        let mut last = rider.v;
        for _ in 0..(120.0 / SUBSTEP) as usize {
            rider.update(
                &ROAD_BIKE,
                &road,
                Input::default(),
                RIDER_POWER_W,
                0.0,
                SUBSTEP,
            );
            assert!(rider.v <= last);
            assert!(rider.v >= 0.0);
            last = rider.v;
        }
        assert!(last < 8.0);
    }

    #[test]
    fn classic_steering_moves_steer_rate_times_t() {
        for spec in [&ROAD_BIKE, &BMX] {
            let road = RoadAt::open();
            let mut rider = Rider::new(4.0);
            rider.v = 6.0;
            let input = Input {
                steer: 1.0,
                pedal: true,
                ..Input::default()
            };
            let steps = 240;
            for _ in 0..steps {
                rider.update(spec, &road, input, RIDER_POWER_W, 0.0, SUBSTEP);
            }
            let t = steps as f64 * SUBSTEP;
            let moved = rider.x - 4.0;
            assert!(
                (moved - spec.steer_rate_ms * t).abs() < 1e-9,
                "{} moved {moved}",
                spec.id
            );
        }
    }

    #[test]
    fn steering_stops_at_the_edge() {
        let road = RoadAt {
            x_min: 2.3,
            x_max: 6.7,
            ..RoadAt::open()
        };
        let mut rider = Rider::new(2.5);
        rider.v = 5.0;
        let mut edge = false;
        for _ in 0..240 {
            let ev = rider.update(
                &ROAD_BIKE,
                &road,
                Input {
                    steer: -1.0,
                    ..Input::default()
                },
                0.0,
                0.0,
                SUBSTEP,
            );
            edge |= ev.contains(&RideEvent::Edge);
        }
        assert_eq!(rider.x, 2.3);
        assert!(edge);
    }

    #[test]
    fn a_hop_rises_to_the_bike_hop_height() {
        for spec in [&ROAD_BIKE, &BMX] {
            let road = RoadAt::open();
            let mut rider = Rider::new(3.0);
            rider.v = 5.0;
            let mut top: f64 = 0.0;
            let mut landed = false;
            for i in 0..240 {
                let input = Input {
                    hop: i == 0,
                    ..Input::default()
                };
                let ev = rider.update(spec, &road, input, 0.0, 0.0, SUBSTEP);
                top = top.max(rider.z);
                landed |= ev
                    .iter()
                    .any(|e| matches!(e, RideEvent::Land { hard: false, .. }));
            }
            assert!((top - spec.hop_m).abs() < 0.02, "{} rose {top}", spec.id);
            assert!(landed);
        }
    }

    #[test]
    fn sprint_drains_in_8_seconds_and_refills() {
        let mut spent = false;
        let mut stamina = 1.0;
        ride_for(
            &ROAD_BIKE,
            8.5,
            Input {
                pedal: true,
                sprint: true,
                ..Input::default()
            },
            |_, r| {
                stamina = r.stamina;
            },
        );
        assert_eq!(stamina, 0.0);
        let road = RoadAt::open();
        let mut rider = Rider::new(3.0);
        rider.stamina = 0.0;
        for _ in 0..(10.0 / SUBSTEP) as usize {
            let ev = rider.update(&ROAD_BIKE, &road, pedal(), RIDER_POWER_W, 0.0, SUBSTEP);
            spent |= ev.contains(&RideEvent::Spent);
        }
        assert!(!spent);
        assert!((rider.stamina - 0.5).abs() < 0.01);
    }

    #[test]
    fn a_crash_keeps_the_rider_down_3_seconds() {
        let road = RoadAt::open();
        let mut rider = Rider::new(3.0);
        rider.v = 7.0;
        rider.crash();
        let mut up_at = None;
        for i in 1..=600 {
            let ev = rider.update(&ROAD_BIKE, &road, pedal(), RIDER_POWER_W, 0.0, SUBSTEP);
            if ev.contains(&RideEvent::Up) {
                up_at = Some(i as f64 * SUBSTEP);
                break;
            }
            assert_eq!(rider.v, 0.0);
        }
        let up = up_at.expect("got back up");
        assert!((up - DOWN_S).abs() < 0.02);
    }

    fn time_to_on(spec: &BikeSpec, power: f64, speed: f64) -> f64 {
        let road = RoadAt::open();
        let mut rider = Rider::new(3.0);
        for i in 1..=(60.0 / SUBSTEP) as usize {
            rider.update(spec, &road, pedal(), power, 0.0, SUBSTEP);
            if rider.v >= speed {
                return i as f64 * SUBSTEP;
            }
        }
        f64::INFINITY
    }

    fn top_speed(spec: &BikeSpec, road: &RoadAt, power: f64) -> f64 {
        let mut rider = Rider::new(3.0);
        for _ in 0..(90.0 / SUBSTEP) as usize {
            rider.update(spec, road, pedal(), power, 0.0, SUBSTEP);
        }
        rider.v
    }

    #[test]
    fn e_bike_gets_to_6_ms_first_and_its_motor_stops_at_25_kmh() {
        let e = time_to_on(&E_BIKE, RIDER_POWER_W, 6.0);
        let road = time_to_on(&ROAD_BIKE, RIDER_POWER_W, 6.0);
        assert!(e < road, "e-bike {e}, road bike {road}");
        // Above the cut-off only the rider pedals, so it settles below the road bike.
        let top = top_speed(&E_BIKE, &RoadAt::open(), RIDER_POWER_W);
        assert!(top > E_BIKE.assist_cut_ms, "{top}");
        assert!(top < 10.0, "{top}");
    }

    #[test]
    fn the_moped_reaches_50_kmh_and_cannot_hop() {
        let top = top_speed(&MOPED, &RoadAt::open(), MOPED.engine_w);
        assert!((top - 13.9).abs() < 0.5, "settled at {top}");
        let t = time_to_on(&MOPED, MOPED.engine_w, 10.0);
        assert!(t < 6.0, "took {t}");
        let mut rider = Rider::new(3.0);
        rider.v = 8.0;
        let ev = rider.update(
            &MOPED,
            &RoadAt::open(),
            Input {
                hop: true,
                ..Input::default()
            },
            0.0,
            0.0,
            SUBSTEP,
        );
        assert!(!ev.contains(&RideEvent::Hop));
        assert!(!rider.airborne());
    }

    #[test]
    fn wet_braking_takes_half_as_far_again_on_slicks() {
        let dry = stopping_distance(&ROAD_BIKE, 8.0, Tyre::Slick25.mu(Surface::Dry));
        let wet = stopping_distance(&ROAD_BIKE, 8.0, Tyre::Slick25.mu(Surface::Wet));
        assert!(wet >= 1.5 * dry, "dry {dry}, wet {wet}");
    }

    #[test]
    fn cobbles_slow_a_road_bike_below_a_mountain_bike() {
        let cobbles = |t: Tyre| RoadAt {
            mu: t.mu(Surface::Cobbles),
            crr_scale: t.crr_scale(Surface::Cobbles),
            ..RoadAt::open()
        };
        let road = top_speed(&ROAD_BIKE, &cobbles(Tyre::Slick25), RIDER_POWER_W);
        let mtb = top_speed(&MOUNTAIN_BIKE, &cobbles(Tyre::Knobby55), RIDER_POWER_W);
        assert!(road < mtb, "road {road}, mountain {mtb}");
        // On asphalt the road bike is still the faster.
        let road_flat = top_speed(&ROAD_BIKE, &RoadAt::open(), RIDER_POWER_W);
        let mtb_flat = top_speed(&MOUNTAIN_BIKE, &RoadAt::open(), RIDER_POWER_W);
        assert!(road_flat > mtb_flat);
    }

    #[test]
    fn a_flat_is_swapped_standing_still() {
        let road = RoadAt::open();
        let mut rider = Rider::new(3.0);
        rider.flat = true;
        let mut swapped = false;
        for _ in 0..(TUBE_SWAP_S / SUBSTEP) as usize + 2 {
            let ev = rider.update(&ROAD_BIKE, &road, Input::default(), 0.0, 0.0, SUBSTEP);
            swapped |= ev.contains(&RideEvent::TubeSwapped);
        }
        assert!(swapped);
        assert!(!rider.flat);
    }
}
