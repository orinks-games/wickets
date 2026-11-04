//! The world a ride runs in: the course, the rider, the cars from behind
//! and the clock, stepped in fixed 120 Hz substeps so the same inputs give
//! the same ride at any frame rate.
//!
//! Every hazard warns by time to reach the rider at the rider's speed, so
//! a two-second warning is two seconds on any bike. Traffic comes from its
//! own seeded stream, keyed by clock time, so nothing the player does
//! changes which cars come.

use parlor_core::{FixedStep, Rng};

use crate::bike::{Bike, Input, RideEvent, Rider, RoadAt, G, RIDER_POWER_W, SPRINT_POWER_W};
use crate::course::{
    Course, HazardKind, BIKE_LANE_LINE, CAR_LANE_X, CENTRE_LINE, CURB_M, DOOR_EDGE, DOOR_OPENS,
    MIN_WARN_M, PARKED_EDGE, RAMP_M, RAMP_WINDOW, SAFE_X, START_X, TURN_LEAD, TURN_SPEED,
    TURN_WARN_M,
};

/// The substep, seconds.
pub const SUBSTEP: f64 = 1.0 / 120.0;
/// A car's length and half its width, metres.
pub const CAR_LEN: f64 = 4.5;
pub const CAR_HALF: f64 = 0.9;
/// A bike's length, metres.
pub const BIKE_LEN: f64 = 1.7;
/// Room a car leaves when it passes a rider, metres.
pub const PASS_ROOM: f64 = 1.5;
/// The furthest left a passing car's centre goes, over the centre line.
pub const CAR_MAX_X: f64 = 7.6;
/// A car stuck behind for this many seconds honks.
pub const HONK_AFTER: f64 = 4.0;
/// A car behind is announced this many seconds before it reaches you.
pub const CAR_LEAD: f64 = 3.0;
/// The act-now cue sounds this many seconds before contact.
pub const ACT_NOW: f64 = 0.6;
/// For a pothole it sounds later, so a hop pressed on the beep is still in
/// the air at the hole: the road bike's hop lasts under 0.3 seconds.
pub const ACT_NOW_HOP: f64 = 0.45;
/// Half the width that meets a hazard: the whole bike for a door, the
/// tyre for anything on the ground.
pub const BODY_HALF: f64 = 0.3;
pub const TYRE_HALF: f64 = 0.05;
/// The finish is announced this many seconds or metres ahead.
pub const FINISH_LEAD: f64 = 8.0;
pub const FINISH_WARN_M: f64 = 100.0;

/// Why a rider came off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cause {
    Door,
    Curb,
    Car,
    /// An oncoming car in the traffic lane.
    Oncoming,
}

/// Something that happened in a substep, for the screen to sound and say.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldEvent {
    /// A hazard's warning cue starts: for a door, the handle's click.
    Warn(usize),
    DoorOpen(usize),
    /// Hop, brake or swerve now: this hazard is in your line.
    ActNow(usize),
    /// The rider's front wheel reached a hazard's line, after any event
    /// that contact caused.
    Passed(usize),
    /// Hopped clean over a hazard in your line.
    Cleared(usize),
    Pothole {
        hazard: usize,
        flat: bool,
    },
    CurbStrike {
        hazard: usize,
        crash: bool,
    },
    Jump(usize),
    Crash(Cause),
    TurnAhead(usize),
    Turned(usize),
    /// Too fast, or in the wrong part of the road, at a junction.
    TurnWide(usize),
    CarBehind(usize),
    Horn(usize),
    CarPassed {
        car: usize,
        thanked: bool,
    },
    FinishAhead,
    Finished,
    /// Across the bike lane line: true into the traffic lane.
    Line(bool),
    /// Into or out of the door zone beside the parked cars.
    DoorZone(bool),
    /// A leaf pile's rustle starts: it is coming up in the bike lane.
    LeavesAhead(usize),
    /// The rider's front wheel reached a leaf pile: `crunched` when the
    /// rider rode through it.
    Leaves {
        pile: usize,
        crunched: bool,
    },
    /// An oncoming car is heard ahead.
    OncomingAhead(usize),
    /// Get out of the traffic lane now: an oncoming car is in your line.
    OncomingNow(usize),
    OncomingPassed(usize),
    /// A parcel's cue starts: it is coming up.
    ParcelAhead(usize),
    /// Scooped up a parcel.
    Parcel(usize),
    /// Reached a parcel too fast to grab it.
    ParcelTooFast(usize),
    /// The last crash an endless ride allows: the ride is over.
    OutOfCrashes,
    Ride(RideEvent),
}

/// A car coming toward the rider in the traffic lane, overtaking on the
/// wrong side of the road.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oncoming {
    pub id: usize,
    /// Its front bumper, the end nearest the rider, metres along the route.
    pub s: f64,
    pub x: f64,
    /// Its speed toward the rider.
    pub v: f64,
    pub warned: bool,
    pub act_now: bool,
    pub hit: bool,
    pub passed: bool,
}

impl Oncoming {
    /// Metres between its front and the rider's front wheel: positive
    /// while it is ahead.
    pub fn gap(&self, rider: &Rider) -> f64 {
        self.s - rider.s
    }

    /// Seconds until it meets the rider, at both speeds.
    pub fn secs(&self, rider: &Rider) -> f64 {
        self.gap(rider) / (self.v + rider.v).max(0.1)
    }

    pub fn alongside(&self, rider: &Rider) -> bool {
        self.s < rider.s && self.s + CAR_LEN > rider.s - BIKE_LEN
    }

    /// Whether a rider at `x` is in its path.
    pub fn in_path(&self, x: f64) -> bool {
        (self.x - x).abs() < CAR_HALF + BODY_HALF
    }
}

/// A parcel's progress through the ride.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ParcelState {
    pub warned: bool,
    pub passed: bool,
    pub taken: bool,
}

/// Half the width that scoops up a parcel, metres.
pub const PARCEL_HALF: f64 = 0.5;
/// A parcel's cue sounds this many seconds ahead, at the rider's speed.
pub const PARCEL_LEAD: f64 = 2.5;
/// A parcel can be grabbed at this speed or under: 20 kilometres an hour.
pub const GRAB_SPEED: f64 = 20.0 / 3.6;
/// An oncoming car is heard this many seconds before it meets you.
pub const ONCOMING_LEAD: f64 = 3.0;
/// Oncoming cars appear this far ahead, metres.
pub const ONCOMING_SPAWN: f64 = 160.0;
/// How many crashes end an endless ride.
pub const ENDLESS_CRASHES: u32 = 3;

/// A leaf pile in the bike lane, in leaf season. Not a hazard: riding
/// through one is a crunch and a bonus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeafPile {
    /// Metres along the route.
    pub s: f64,
    /// The middle of the pile across the road.
    pub x: f64,
    pub warned: bool,
    pub passed: bool,
}

/// Half the width of a leaf pile, metres.
pub const LEAF_HALF: f64 = 0.6;
/// A leaf pile rustles this many seconds ahead, at the rider's speed.
pub const LEAF_LEAD: f64 = 2.5;

/// A car coming up from behind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Car {
    pub id: usize,
    /// Its front bumper, metres along the route.
    pub s: f64,
    pub x: f64,
    pub v: f64,
    pub target_v: f64,
    pub target_x: f64,
    pub stuck_for: f64,
    pub honk_in: f64,
    pub honked: bool,
    pub warned: bool,
    pub passed: bool,
}

impl Car {
    /// Metres from its front to the rear of the rider's bike: positive
    /// while it is behind.
    pub fn gap(&self, rider: &Rider) -> f64 {
        rider.s - BIKE_LEN - self.s
    }

    pub fn alongside(&self, rider: &Rider) -> bool {
        self.s > rider.s - BIKE_LEN && self.s - CAR_LEN < rider.s
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CarPlan {
    at: f64,
    speed: f64,
}

/// A hazard's progress through the ride.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct HazardState {
    pub warned: bool,
    pub open: bool,
    pub act_now: bool,
    pub passed: bool,
}

/// What the ride earned and cost.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub crashes: u32,
    pub doors: u32,
    pub flats: u32,
    pub jumps: u32,
    /// Seconds in the air off ramps.
    pub airtime: f64,
    /// Cars that thanked you for pulling over.
    pub tips: u32,
    pub wide_turns: u32,
    pub turns: u32,
    pub curb_strikes: u32,
    /// Leaf piles ridden through.
    pub leaves: u32,
    /// Dollars earned from parcels scooped up.
    pub cash: u32,
    /// Parcels passed too fast to grab.
    pub missed_parcels: u32,
    /// Oncoming cars that went by.
    pub oncoming: u32,
}

/// Where the next thing on the road is, and how soon the rider meets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// Metres ahead.
    pub metres: f64,
    /// Seconds at the rider's speed, or `None` when stopped.
    pub secs: Option<f64>,
    /// Whether the rider's line meets it.
    pub in_line: bool,
}

/// The world of one ride.
pub struct World {
    pub course: Course,
    pub bike: Bike,
    pub rider: Rider,
    pub cars: Vec<Car>,
    /// Seconds since the start.
    pub clock: f64,
    pub hazards: Vec<HazardState>,
    pub stats: Stats,
    /// The clock when the rider crossed the finish line.
    pub finished: Option<f64>,
    /// Whether the rider is in the door zone, and on which side of the
    /// bike lane line.
    pub in_door_zone: bool,
    pub in_traffic_lane: bool,
    /// Leaf piles in the bike lane, in leaf season; none otherwise.
    pub leaves: Vec<LeafPile>,
    pub oncoming: Vec<Oncoming>,
    pub parcels: Vec<ParcelState>,
    /// Crashes that end the ride, for an endless ride.
    pub crash_limit: Option<u32>,
    oncoming_plans: Vec<CarPlan>,
    next_oncoming: usize,
    first_parcel: usize,
    step: FixedStep,
    plans: Vec<CarPlan>,
    next_plan: usize,
    next_car: usize,
    first_hazard: usize,
    turn_warned: Vec<bool>,
    turn_done: Vec<bool>,
    finish_warned: bool,
    jumping: bool,
}

impl World {
    pub const SUBSTEP: f64 = SUBSTEP;

    /// A ride on `course` on `bike`, with cars from the `traffic_seed` stream.
    pub fn new(course: Course, bike: Bike, traffic_seed: u64) -> World {
        let mut rng = Rng::seeded(traffic_seed);
        let mut plans = Vec::new();
        if course.traffic && course.car_gap > 0.0 {
            let mut t = 4.0 + rng.unit() * 6.0;
            while t < 1800.0 {
                plans.push(CarPlan {
                    at: t,
                    speed: 10.5 + rng.unit() * 2.5,
                });
                t += course.car_gap * (0.5 + rng.unit());
            }
        }
        let mut oncoming_plans = Vec::new();
        if course.oncoming_gap > 0.0 {
            let mut rng = Rng::seeded(traffic_seed ^ 0x6f6e_636f);
            let mut t = 12.0 + rng.unit() * 8.0;
            while t < 3600.0 {
                oncoming_plans.push(CarPlan {
                    at: t,
                    speed: 8.0 + rng.unit() * 4.0,
                });
                // They come twice as often by five minutes in.
                let squeeze = (1.0 - t / 600.0).max(0.5);
                t += course.oncoming_gap * squeeze * (0.6 + 0.8 * rng.unit());
            }
        }
        let parcels = vec![ParcelState::default(); course.parcels.len()];
        let hazards = vec![HazardState::default(); course.hazards.len()];
        let turns = course.junctions.len();
        let rider = Rider::new(START_X);
        World {
            course,
            bike,
            rider,
            cars: Vec::new(),
            clock: 0.0,
            hazards,
            stats: Stats::default(),
            finished: None,
            in_door_zone: rider.x - BODY_HALF < DOOR_EDGE,
            in_traffic_lane: rider.x > BIKE_LANE_LINE,
            leaves: Vec::new(),
            oncoming: Vec::new(),
            parcels,
            crash_limit: None,
            oncoming_plans,
            next_oncoming: 0,
            first_parcel: 0,
            step: FixedStep::new(SUBSTEP),
            plans,
            next_plan: 0,
            next_car: 0,
            first_hazard: 0,
            turn_warned: vec![false; turns],
            turn_done: vec![false; turns],
            finish_warned: false,
            jumping: false,
        }
    }

    /// The same world with leaf piles at these spots, metres along the
    /// route and across it.
    pub fn with_leaves(mut self, piles: &[(f64, f64)]) -> World {
        self.leaves = piles
            .iter()
            .map(|&(s, x)| LeafPile {
                s,
                x,
                warned: false,
                passed: false,
            })
            .collect();
        self
    }

    /// The next leaf pile not yet reached.
    pub fn next_leaves(&self) -> Option<usize> {
        self.leaves.iter().position(|p| !p.passed)
    }

    /// The same world, over after `n` crashes.
    pub fn with_crash_limit(mut self, n: u32) -> World {
        self.crash_limit = Some(n);
        self
    }

    /// Crashes left before the ride ends, for an endless ride.
    pub fn crashes_left(&self) -> Option<u32> {
        self.crash_limit
            .map(|n| n.saturating_sub(self.stats.crashes))
    }

    /// The road edges and grip under the rider.
    pub fn road(&self) -> RoadAt {
        let tyre = self.bike.spec().tyre;
        let surface = self.course.surface;
        RoadAt {
            mu: tyre.mu(surface),
            crr_scale: tyre.crr_scale(surface),
            x_min: PARKED_EDGE + BODY_HALF,
            x_max: CENTRE_LINE - BODY_HALF,
        }
    }

    /// Runs a frame of `dt` seconds in fixed substeps.
    pub fn update(&mut self, input: Input, dt: f64) -> Vec<WorldEvent> {
        let mut events = Vec::new();
        let mut hop = input.hop;
        let mut step = self.step;
        step.advance(dt, |_| {
            let input = Input { hop, ..input };
            hop = false;
            events.extend(self.substep(input));
        });
        self.step = step;
        events
    }

    /// How far toward the next substep the stored time is, 0 to 1.
    pub fn alpha(&self) -> f64 {
        self.step.alpha()
    }

    /// One substep.
    pub fn substep(&mut self, input: Input) -> Vec<WorldEvent> {
        let dt = SUBSTEP;
        let mut events = Vec::new();
        if self.finished.is_some() {
            return events;
        }
        let spec = *self.bike.spec();
        let road = self.road();
        let prev_s = self.rider.s;
        let (power, sprint) = if spec.engine_w > 0.0 {
            (spec.engine_w, 0.0)
        } else {
            (RIDER_POWER_W, SPRINT_POWER_W)
        };
        for ev in self.rider.update(&spec, &road, input, power, sprint, dt) {
            if let RideEvent::Land { air, .. } = ev {
                if self.jumping {
                    self.stats.airtime += air;
                    self.jumping = false;
                }
            }
            events.push(WorldEvent::Ride(ev));
        }
        self.hazards_step(prev_s, &mut events);
        self.turns_step(prev_s, &mut events);
        self.cars_step(dt, &mut events);
        self.lanes_step(&mut events);
        self.leaves_step(prev_s, &mut events);
        self.oncoming_step(dt, &mut events);
        self.parcels_step(prev_s, &mut events);
        self.clock += dt;
        if self.finished.is_some() {
            return events;
        }
        let length = self.course.length;
        let r = &self.rider;
        if !self.finish_warned && length - r.s <= FINISH_WARN_M.max(r.v * FINISH_LEAD) {
            self.finish_warned = true;
            events.push(WorldEvent::FinishAhead);
        }
        if r.s >= length {
            self.finished = Some(self.clock);
            events.push(WorldEvent::Finished);
        }
        events
    }

    fn crash(&mut self, cause: Cause, events: &mut Vec<WorldEvent>) {
        self.rider.crash();
        self.rider.x = SAFE_X;
        self.jumping = false;
        self.stats.crashes += 1;
        if cause == Cause::Door {
            self.stats.doors += 1;
        }
        events.push(WorldEvent::Crash(cause));
        if self.crash_limit.is_some_and(|n| self.stats.crashes >= n) {
            self.finished = Some(self.clock);
            events.push(WorldEvent::OutOfCrashes);
        }
    }

    fn oncoming_step(&mut self, dt: f64, events: &mut Vec<WorldEvent>) {
        while self.next_oncoming < self.oncoming_plans.len()
            && self.oncoming_plans[self.next_oncoming].at <= self.clock
        {
            let plan = self.oncoming_plans[self.next_oncoming];
            self.next_oncoming += 1;
            // Nothing new comes at you while you are down.
            if self.rider.down() || self.finished.is_some() {
                continue;
            }
            self.oncoming.push(Oncoming {
                id: self.next_car,
                s: self.rider.s + ONCOMING_SPAWN,
                x: CAR_LANE_X,
                v: plan.speed,
                warned: false,
                act_now: false,
                hit: false,
                passed: false,
            });
            self.next_car += 1;
        }
        let mut crash = false;
        for car in &mut self.oncoming {
            car.s -= car.v * dt;
            let r = &self.rider;
            if car.passed {
                continue;
            }
            let gap = car.gap(r);
            if !car.warned && gap > 0.0 && (gap <= 25.0 || car.secs(r) <= ONCOMING_LEAD) {
                car.warned = true;
                events.push(WorldEvent::OncomingAhead(car.id));
            }
            if !car.act_now && gap > 0.0 && car.secs(r) <= ACT_NOW && car.in_path(r.x) && !r.down()
            {
                car.act_now = true;
                events.push(WorldEvent::OncomingNow(car.id));
            }
            if car.alongside(r) {
                if !car.hit && !r.down() && car.in_path(r.x) {
                    car.hit = true;
                    crash = true;
                }
            } else if gap < 0.0 {
                car.passed = true;
                self.stats.oncoming += 1;
                events.push(WorldEvent::OncomingPassed(car.id));
            }
        }
        let rider_s = self.rider.s;
        self.oncoming.retain(|c| rider_s - c.s < 60.0);
        if crash && self.finished.is_none() {
            self.crash(Cause::Oncoming, events);
        }
    }

    fn parcels_step(&mut self, prev_s: f64, events: &mut Vec<WorldEvent>) {
        let r = self.rider;
        let mut i = self.first_parcel;
        while i < self.parcels.len() {
            let c = self.course.parcels[i];
            let d = c.s - r.s;
            if d > MIN_WARN_M.max(r.v * PARCEL_LEAD) {
                break;
            }
            let state = &mut self.parcels[i];
            if !state.warned && d > 0.0 {
                state.warned = true;
                events.push(WorldEvent::ParcelAhead(i));
            }
            if !state.passed && prev_s < c.s && r.s >= c.s {
                state.passed = true;
                if !r.down() && !r.airborne() && (r.x - c.x).abs() < PARCEL_HALF {
                    if r.v <= GRAB_SPEED {
                        state.taken = true;
                        self.stats.cash += c.value;
                        events.push(WorldEvent::Parcel(i));
                    } else {
                        self.stats.missed_parcels += 1;
                        events.push(WorldEvent::ParcelTooFast(i));
                    }
                }
            }
            i += 1;
        }
        while self.first_parcel < self.parcels.len() && self.parcels[self.first_parcel].passed {
            self.first_parcel += 1;
        }
    }

    /// The nearest oncoming car that has been heard and not yet gone by.
    pub fn oncoming_ahead(&self) -> Option<&Oncoming> {
        self.oncoming
            .iter()
            .filter(|c| c.warned && !c.passed)
            .min_by(|a, b| a.s.total_cmp(&b.s))
    }

    /// Whether an oncoming car is on the road ahead, heard yet or not:
    /// its engine sounds from the moment it comes.
    pub fn oncoming_near(&self) -> bool {
        self.oncoming.iter().any(|c| !c.passed)
    }

    /// The next parcel not yet reached.
    pub fn next_parcel(&self) -> Option<usize> {
        (self.first_parcel..self.parcels.len()).find(|&i| !self.parcels[i].passed)
    }

    fn hazards_step(&mut self, prev_s: f64, events: &mut Vec<WorldEvent>) {
        let spec = *self.bike.spec();
        let mut i = self.first_hazard;
        while i < self.course.hazards.len() {
            let h = self.course.hazards[i];
            let r = self.rider;
            let d = h.s - r.s;
            let reach = MIN_WARN_M.max(r.v * h.kind.lead());
            if d > reach && self.hazards[i].warned {
                i += 1;
                continue;
            }
            if d > reach {
                break;
            }
            let state = &mut self.hazards[i];
            if !state.warned && d > 0.0 {
                state.warned = true;
                events.push(WorldEvent::Warn(i));
            }
            if h.kind == HazardKind::Door
                && state.warned
                && !state.open
                && d > 0.0
                && (d <= r.v * DOOR_OPENS || d <= 1.5)
            {
                state.open = true;
                events.push(WorldEvent::DoorOpen(i));
            }
            let half = if h.kind == HazardKind::Door {
                BODY_HALF
            } else {
                TYRE_HALF
            };
            let act_now = if h.kind == HazardKind::Pothole {
                ACT_NOW_HOP
            } else {
                ACT_NOW
            };
            if !state.act_now
                && d > 0.0
                && r.v > 0.5
                && d / r.v <= act_now
                && h.covers(r.x, half)
                && !r.down()
                && !(r.airborne() && h.kind != HazardKind::Door)
            {
                state.act_now = true;
                events.push(WorldEvent::ActNow(i));
            }
            if !state.passed && prev_s < h.s && r.s >= h.s {
                state.passed = true;
                // Contact first, so Passed comes after its outcome
                // (Cleared, Pothole, a crash).
                self.contact(i, &spec, events);
                events.push(WorldEvent::Passed(i));
            }
            i += 1;
        }
        while self.first_hazard < self.hazards.len() && self.hazards[self.first_hazard].passed {
            self.first_hazard += 1;
        }
    }

    /// The rider's front wheel meets hazard `i`'s line.
    fn contact(&mut self, i: usize, spec: &crate::bike::BikeSpec, events: &mut Vec<WorldEvent>) {
        let h = self.course.hazards[i];
        let r = self.rider;
        if r.down() {
            return;
        }
        match h.kind {
            HazardKind::Door => {
                if self.hazards[i].open && h.covers(r.x, BODY_HALF) {
                    self.crash(Cause::Door, events);
                }
            }
            HazardKind::Pothole => {
                if !h.covers(r.x, TYRE_HALF) {
                    return;
                }
                // Any air at all clears a pothole: the road bike's hop is
                // short, and a lifted wheel skips the hole.
                if r.airborne() {
                    events.push(WorldEvent::Cleared(i));
                } else if spec.tyre.wide() {
                    self.rider.v *= 0.75;
                    events.push(WorldEvent::Pothole {
                        hazard: i,
                        flat: false,
                    });
                } else {
                    self.rider.v *= 0.85;
                    if !self.rider.flat {
                        self.rider.flat = true;
                        self.stats.flats += 1;
                    }
                    events.push(WorldEvent::Pothole {
                        hazard: i,
                        flat: true,
                    });
                }
            }
            HazardKind::Curb => {
                if !h.covers(r.x, TYRE_HALF) {
                    return;
                }
                if r.z >= CURB_M + 0.02 {
                    events.push(WorldEvent::Cleared(i));
                    return;
                }
                self.stats.curb_strikes += 1;
                if spec.tyre.wide() {
                    self.rider.v *= 0.6;
                    events.push(WorldEvent::CurbStrike {
                        hazard: i,
                        crash: false,
                    });
                } else {
                    events.push(WorldEvent::CurbStrike {
                        hazard: i,
                        crash: true,
                    });
                    self.crash(Cause::Curb, events);
                }
            }
            HazardKind::Ramp => {
                if !h.covers(r.x, TYRE_HALF) {
                    return;
                }
                if r.airborne() || r.since_hop <= RAMP_WINDOW {
                    let launch = (2.0 * G * (spec.hop_m + RAMP_M)).sqrt();
                    self.rider.vz = launch;
                    self.rider.z = self.rider.z.max(1e-6);
                    self.rider.air_for = 0.0;
                    self.jumping = true;
                    self.stats.jumps += 1;
                    events.push(WorldEvent::Jump(i));
                }
            }
        }
    }

    fn turns_step(&mut self, prev_s: f64, events: &mut Vec<WorldEvent>) {
        for (i, j) in self.course.junctions.iter().enumerate() {
            if self.turn_done[i] {
                continue;
            }
            let r = &mut self.rider;
            let d = j.s - r.s;
            if !self.turn_warned[i] && d > 0.0 && d <= TURN_WARN_M.max(r.v * TURN_LEAD) {
                self.turn_warned[i] = true;
                events.push(WorldEvent::TurnAhead(i));
            }
            if prev_s < j.s && r.s >= j.s {
                self.turn_done[i] = true;
                let (lo, hi) = j.turn.band();
                if r.x >= lo && r.x <= hi && r.v <= TURN_SPEED + 0.05 {
                    self.stats.turns += 1;
                    events.push(WorldEvent::Turned(i));
                } else {
                    r.v = (r.v * 0.5).min(3.0);
                    self.stats.wide_turns += 1;
                    events.push(WorldEvent::TurnWide(i));
                }
            }
        }
    }

    fn cars_step(&mut self, dt: f64, events: &mut Vec<WorldEvent>) {
        while self.next_plan < self.plans.len() && self.plans[self.next_plan].at <= self.clock {
            let plan = self.plans[self.next_plan];
            self.next_plan += 1;
            self.cars.push(Car {
                id: self.next_car,
                s: self.rider.s - BIKE_LEN - 70.0,
                x: CAR_LANE_X,
                v: plan.speed,
                target_v: plan.speed,
                target_x: CAR_LANE_X,
                stuck_for: 0.0,
                honk_in: 0.0,
                honked: false,
                warned: false,
                passed: false,
            });
            self.next_car += 1;
        }
        let mut crashed = false;
        let mut tips = 0;
        for car in &mut self.cars {
            let r = &self.rider;
            let gap = car.gap(r);
            let pass_x = CAR_LANE_X.max(r.x + BODY_HALF + PASS_ROOM + CAR_HALF);
            let can_pass = pass_x <= CAR_MAX_X;
            let mut want_v = car.target_v;
            if !car.passed {
                if gap > 0.0 {
                    if can_pass {
                        car.stuck_for = 0.0;
                        if gap < 40.0 {
                            car.target_x = pass_x;
                        }
                    } else {
                        if gap < 10.0 {
                            want_v = car.target_v.min(r.v);
                        }
                        if gap < 4.0 {
                            want_v = r.v * 0.9;
                        }
                        if gap < 12.0 {
                            car.stuck_for += dt;
                        }
                        car.honk_in -= dt;
                        if car.stuck_for >= HONK_AFTER && car.honk_in <= 0.0 {
                            car.honked = true;
                            car.honk_in = 6.0;
                            events.push(WorldEvent::Horn(car.id));
                        }
                    }
                    let closing = car.v - r.v;
                    if !car.warned && (gap <= 25.0 || (closing > 0.0 && gap / closing <= CAR_LEAD))
                    {
                        car.warned = true;
                        events.push(WorldEvent::CarBehind(car.id));
                    }
                } else if car.alongside(r) {
                    if !r.down() && (car.x - r.x).abs() < CAR_HALF + BODY_HALF {
                        crashed = true;
                    }
                } else {
                    car.passed = true;
                    car.target_x = CAR_LANE_X;
                    tips += car.honked as u32;
                    events.push(WorldEvent::CarPassed {
                        car: car.id,
                        thanked: car.honked,
                    });
                }
            }
            let dv = want_v - car.v;
            car.v += dv.clamp(-4.0 * dt, 3.0 * dt);
            car.v = car.v.max(0.0);
            let dx = car.target_x - car.x;
            car.x += dx.clamp(-1.5 * dt, 1.5 * dt);
            car.s += car.v * dt;
        }
        self.stats.tips += tips;
        let rider_s = self.rider.s;
        self.cars
            .retain(|c| !(c.passed && c.s - rider_s > 150.0) && rider_s - c.s < 250.0);
        if crashed {
            self.crash(Cause::Car, events);
        }
    }

    /// Leaf piles: a rustle once every hazard before the pile is passed,
    /// so it never sounds over a hazard's warning, then a crunch.
    fn leaves_step(&mut self, prev_s: f64, events: &mut Vec<WorldEvent>) {
        let r = self.rider;
        let hazard_before =
            |s: f64, w: &World| w.next_hazard().is_some_and(|i| w.course.hazards[i].s < s);
        for i in 0..self.leaves.len() {
            let p = self.leaves[i];
            if p.passed {
                continue;
            }
            let d = p.s - r.s;
            if !p.warned
                && d > 0.0
                && d <= MIN_WARN_M.max(r.v * LEAF_LEAD)
                && !hazard_before(p.s, self)
            {
                self.leaves[i].warned = true;
                events.push(WorldEvent::LeavesAhead(i));
            }
            if prev_s < p.s && r.s >= p.s {
                self.leaves[i].passed = true;
                let crunched = !r.down() && !r.airborne() && (r.x - p.x).abs() < LEAF_HALF;
                if crunched {
                    self.stats.leaves += 1;
                }
                events.push(WorldEvent::Leaves { pile: i, crunched });
            }
        }
    }

    fn lanes_step(&mut self, events: &mut Vec<WorldEvent>) {
        let x = self.rider.x;
        let zone = x - BODY_HALF < DOOR_EDGE;
        if zone != self.in_door_zone {
            self.in_door_zone = zone;
            events.push(WorldEvent::DoorZone(zone));
        }
        let traffic = x > BIKE_LANE_LINE;
        if traffic != self.in_traffic_lane {
            self.in_traffic_lane = traffic;
            events.push(WorldEvent::Line(traffic));
        }
    }

    /// The first hazard not yet reached.
    pub fn next_hazard(&self) -> Option<usize> {
        (self.first_hazard..self.course.hazards.len()).find(|&i| !self.hazards[i].passed)
    }

    /// The next junction not yet reached.
    pub fn next_turn(&self) -> Option<usize> {
        (0..self.course.junctions.len()).find(|&i| !self.turn_done[i])
    }

    /// Where hazard `i` is from the rider, and whether the rider's line meets it.
    pub fn predict_contact(&self, i: usize) -> Contact {
        let h = self.course.hazards[i];
        let r = &self.rider;
        let metres = h.s - r.s;
        let half = if h.kind == HazardKind::Door {
            BODY_HALF
        } else {
            TYRE_HALF
        };
        Contact {
            metres,
            secs: (r.v > 0.1).then(|| metres / r.v),
            in_line: h.covers(r.x, half),
        }
    }

    /// The closest car behind that has been announced and not yet passed.
    pub fn car_behind(&self) -> Option<&Car> {
        self.cars
            .iter()
            .filter(|c| !c.passed && c.warned)
            .min_by(|a, b| a.gap(&self.rider).total_cmp(&b.gap(&self.rider)))
    }

    /// A hash of everything that moves, for determinism checks.
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv::new();
        let r = &self.rider;
        for v in [
            self.clock,
            r.s,
            r.x,
            r.v,
            r.z,
            r.vz,
            r.stamina,
            r.down_for,
            self.stats.airtime,
        ] {
            h.f(v);
        }
        h.u(r.flat as u64);
        for c in &self.cars {
            h.u(c.id as u64);
            h.f(c.s);
            h.f(c.x);
            h.f(c.v);
            h.u(c.honked as u64);
        }
        for s in &self.hazards {
            h.u(s.warned as u64 | (s.open as u64) << 1 | (s.passed as u64) << 2);
        }
        for c in &self.oncoming {
            h.u(c.id as u64);
            h.f(c.s);
        }
        h.u(self.stats.crashes as u64);
        h.u(self.stats.tips as u64);
        h.u(self.stats.cash as u64);
        h.0
    }
}

struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn u(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn f(&mut self, v: f64) {
        self.u(v.to_bits());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::{Hazard, Junction, Mix, Turn, COURSES, FAIR_LEAD};

    /// Holds the rider at `x` and speed `v` for a whole course, and returns
    /// every hazard's warning and contact times.
    fn cruise(course: Course, v: f64, x: f64) -> Vec<(HazardKind, Option<f64>, Option<f64>)> {
        let n = course.hazards.len();
        let mut world = World::new(course, Bike::Road, 7);
        world.rider.x = x;
        let mut warn = vec![None; n];
        let mut reach = vec![None; n];
        let mut guard = 0;
        while world.finished.is_none() && guard < 2_000_000 {
            world.rider.v = v;
            world.rider.flat = false;
            world.rider.down_for = 0.0;
            world.rider.x = x;
            for ev in world.substep(Input::default()) {
                match ev {
                    WorldEvent::Warn(i) => warn[i] = Some(world.clock),
                    WorldEvent::Passed(i) => reach[i] = Some(world.clock),
                    _ => {}
                }
            }
            guard += 1;
        }
        let kinds: Vec<HazardKind> = world.course.hazards.iter().map(|h| h.kind).collect();
        kinds
            .into_iter()
            .zip(warn.into_iter().zip(reach))
            .map(|(k, (w, r))| (k, w, r))
            .collect()
    }

    #[test]
    fn every_hazard_warns_1_5_s_ahead() {
        let mut speeds: Vec<f64> = (3..=10).map(|v| v as f64).collect();
        speeds.push(10.4);
        for seed in 0..50u64 {
            let course = Course::generate("Fair", 1000 + seed, 700.0, Mix::DOWNTOWN);
            for &v in &speeds {
                for (i, (kind, warn, reach)) in
                    cruise(course.clone(), v, 6.0).into_iter().enumerate()
                {
                    let warn = warn
                        .unwrap_or_else(|| panic!("seed {seed} {v}: {kind:?} {i} never warned"));
                    let reach = reach.expect("every hazard is reached");
                    assert!(
                        reach - warn >= FAIR_LEAD,
                        "seed {seed} at {v}: {kind:?} {i} warned {:.2} seconds ahead",
                        reach - warn
                    );
                }
            }
        }
    }

    #[test]
    fn door_warning_lead_is_time_based() {
        let course = Course::empty("Doors", 200.0).with(Hazard {
            kind: HazardKind::Door,
            s: 100.0,
            x0: PARKED_EDGE,
            x1: DOOR_EDGE,
        });
        for v in [4.0, 10.0] {
            let times = cruise(course.clone(), v, 6.0);
            let (_, warn, reach) = times[0];
            let lead = reach.unwrap() - warn.unwrap();
            assert!(
                (lead - crate::course::DOOR_LEAD).abs() <= 0.1,
                "at {v}: lead {lead}"
            );
        }
    }

    #[test]
    fn door_opens_after_its_click_and_before_the_rider() {
        let course = Course::empty("Doors", 200.0).with(Hazard {
            kind: HazardKind::Door,
            s: 100.0,
            x0: PARKED_EDGE,
            x1: DOOR_EDGE,
        });
        let mut world = World::new(course, Bike::Road, 1);
        let mut order = Vec::new();
        while world.finished.is_none() {
            world.rider.v = 7.0;
            for ev in world.substep(Input::default()) {
                match ev {
                    WorldEvent::Warn(_) => order.push("click"),
                    WorldEvent::DoorOpen(_) => order.push("open"),
                    WorldEvent::Crash(Cause::Door) => order.push("crash"),
                    _ => {}
                }
            }
        }
        assert_eq!(order, ["click", "open", "crash"]);
    }

    /// Rides at a steady 5 metres a second toward a curb across the bike
    /// lane, hopping 1.5 metres before it.
    fn approach_curb(bike: Bike) -> Vec<WorldEvent> {
        let course = Course::empty("Curb", 100.0).with(Hazard {
            kind: HazardKind::Curb,
            s: 40.0,
            x0: PARKED_EDGE,
            x1: BIKE_LANE_LINE,
        });
        let mut world = World::new(course, bike, 1);
        world.rider.v = 5.0;
        let mut events = Vec::new();
        let mut hopped = false;
        while world.rider.s < 45.0 {
            let hop = !hopped && world.rider.s >= 38.5;
            hopped |= hop;
            world.rider.v = world.rider.v.max(5.0);
            let input = Input {
                hop,
                ..Input::default()
            };
            events.extend(world.substep(input).into_iter().filter(|e| {
                matches!(
                    e,
                    WorldEvent::CurbStrike { .. } | WorldEvent::Cleared(_) | WorldEvent::Crash(_)
                )
            }));
            if world.rider.down() {
                break;
            }
        }
        events
    }

    #[test]
    fn bmx_hop_clears_curb_road_bike_strikes() {
        let road = approach_curb(Bike::Road);
        assert_eq!(
            road,
            [
                WorldEvent::CurbStrike {
                    hazard: 0,
                    crash: true
                },
                WorldEvent::Crash(Cause::Curb)
            ]
        );
        let bmx = approach_curb(Bike::Bmx);
        assert!(
            bmx.iter()
                .all(|e| !matches!(e, WorldEvent::CurbStrike { .. })),
            "{bmx:?}"
        );
        assert_eq!(bmx, [WorldEvent::Cleared(0)]);
    }

    /// A seeded input log: a fresh random input every half second.
    fn input_log(seed: u64, secs: f64) -> Vec<Input> {
        let mut rng = Rng::seeded(seed);
        let mut log = Vec::new();
        let mut current = Input::default();
        for i in 0..(secs / SUBSTEP) as usize {
            if i % 60 == 0 {
                current = Input {
                    pedal: rng.chance(0.8),
                    brake: rng.chance(0.1),
                    steer: [-1.0, 0.0, 0.0, 1.0][rng.below(4) as usize],
                    hop: false,
                    sprint: rng.chance(0.2),
                };
            }
            log.push(Input {
                hop: i % 60 == 0 && rng.chance(0.2),
                ..current
            });
        }
        log
    }

    fn play(traffic_seed: u64) -> (u64, World) {
        let course = Course::generate("Long", 77, 3000.0, Mix::DOWNTOWN);
        let mut world = World::new(course, Bike::Road, traffic_seed);
        for input in input_log(5, 120.0) {
            world.substep(input);
        }
        (world.state_hash(), world)
    }

    #[test]
    fn same_seed_same_state_hash() {
        let (a, world) = play(11);
        let (b, _) = play(11);
        assert_eq!(a, b);
        assert!(world.rider.s > 200.0, "rode {}", world.rider.s);
        let (c, _) = play(12);
        assert_ne!(a, c);
    }

    #[test]
    fn the_world_runs_the_same_at_any_frame_rate() {
        parlor_core::testkit::same_at_frame_rates(|dt| {
            let course = Course::downtown(&COURSES[0]);
            let mut world = World::new(course, Bike::Bmx, 3);
            let frames = (30.0 / dt).round() as usize;
            for f in 0..frames {
                let input = Input {
                    pedal: true,
                    steer: if (f as f64 * dt) % 6.0 < 1.0 {
                        1.0
                    } else {
                        0.0
                    },
                    ..Input::default()
                };
                world.update(input, dt);
            }
            world.state_hash()
        });
    }

    #[test]
    fn a_car_passes_with_room_and_honks_when_it_has_none() {
        let course = Course::empty("Cars", 3000.0);
        let mut world = World::new(
            Course {
                traffic: true,
                car_gap: 20.0,
                ..course
            },
            Bike::Road,
            4,
        );
        let mut passed = 0;
        for _ in 0..(40.0 / SUBSTEP) as usize {
            world.rider.v = 5.0;
            world.rider.x = 2.8;
            for ev in world.substep(Input::default()) {
                if let WorldEvent::CarPassed { thanked, .. } = ev {
                    assert!(!thanked);
                    passed += 1;
                }
                assert!(!matches!(ev, WorldEvent::Crash(_) | WorldEvent::Horn(_)));
            }
        }
        assert!(passed >= 1);
        // Out in the middle of the lane, a car cannot pass and honks; pulling
        // over lets it by with a thank-you toot.
        let mut world = World::new(
            Course {
                traffic: true,
                car_gap: 20.0,
                ..Course::empty("Cars", 3000.0)
            },
            Bike::Road,
            4,
        );
        world.rider.x = 6.0;
        let mut honked = false;
        let mut thanked = false;
        for _ in 0..(60.0 / SUBSTEP) as usize {
            world.rider.v = 5.0;
            let target = if honked { 2.8 } else { 6.0 };
            let step = (target - world.rider.x).clamp(-1.2 * SUBSTEP, 1.2 * SUBSTEP);
            world.rider.x += step;
            for ev in world.substep(Input::default()) {
                match ev {
                    WorldEvent::Horn(_) => honked = true,
                    WorldEvent::CarPassed { thanked: true, .. } => thanked = true,
                    WorldEvent::Crash(_) => panic!("crashed"),
                    _ => {}
                }
            }
        }
        assert!(honked);
        assert!(thanked);
        assert!(world.stats.tips >= 1);
    }

    #[test]
    fn swerving_into_a_passing_car_is_a_crash() {
        let course = Course {
            traffic: true,
            car_gap: 1000.0,
            ..Course::empty("Cars", 3000.0)
        };
        let mut world = World::new(course, Bike::Road, 4);
        let mut crashed = false;
        for _ in 0..(40.0 / SUBSTEP) as usize {
            world.rider.v = 4.0;
            let alongside = world.cars.iter().any(|c| c.alongside(&world.rider));
            if alongside {
                world.rider.x = (world.rider.x + 0.05).min(6.7);
            }
            for ev in world.substep(Input::default()) {
                crashed |= ev == WorldEvent::Crash(Cause::Car);
            }
            if crashed {
                break;
            }
        }
        assert!(crashed);
    }

    #[test]
    fn junctions_turn_slow_and_on_the_correct_side() {
        let course = Course::empty("Turns", 300.0)
            .with_turn(Junction {
                s: 100.0,
                turn: Turn::Right,
            })
            .with_turn(Junction {
                s: 200.0,
                turn: Turn::Left,
            });
        for (v, x, want) in [
            (5.0, 2.8, ["turned", "wide"]),
            (5.0, 6.0, ["wide", "turned"]),
            (8.0, 2.8, ["wide", "wide"]),
        ] {
            let mut world = World::new(course.clone(), Bike::Road, 1);
            let mut got = Vec::new();
            let mut ahead = 0;
            while world.finished.is_none() {
                world.rider.v = v;
                world.rider.x = x;
                for ev in world.substep(Input::default()) {
                    match ev {
                        WorldEvent::Turned(_) => got.push("turned"),
                        WorldEvent::TurnWide(_) => got.push("wide"),
                        WorldEvent::TurnAhead(_) => ahead += 1,
                        _ => {}
                    }
                }
            }
            assert_eq!(got, want, "at {v} and {x}");
            assert_eq!(ahead, 2);
        }
    }

    #[test]
    fn a_hop_at_a_ramp_lip_jumps_and_earns_airtime() {
        let course = Course::empty("Ramp", 100.0).with(Hazard {
            kind: HazardKind::Ramp,
            s: 40.0,
            x0: 2.3,
            x1: 3.3,
        });
        for bike in Bike::ALL {
            let mut world = World::new(course.clone(), bike, 1);
            let mut hopped = false;
            let mut jumped = false;
            while world.finished.is_none() {
                world.rider.v = world.rider.v.max(6.0);
                let hop = !hopped && world.rider.s >= 39.0;
                hopped |= hop;
                for ev in world.substep(Input {
                    hop,
                    ..Input::default()
                }) {
                    jumped |= matches!(ev, WorldEvent::Jump(0));
                }
            }
            assert!(jumped, "{bike:?}");
            assert!(
                world.stats.airtime > 0.5,
                "{bike:?}: {}",
                world.stats.airtime
            );
        }
    }

    #[test]
    fn a_pothole_flats_a_road_bike_and_jolts_a_bmx() {
        let course = Course::empty("Hole", 100.0).with(Hazard {
            kind: HazardKind::Pothole,
            s: 40.0,
            x0: 2.45,
            x1: 3.15,
        });
        for (bike, flat) in [(Bike::Road, true), (Bike::Bmx, false)] {
            let mut world = World::new(course.clone(), bike, 1);
            let mut hit = None;
            while world.finished.is_none() {
                world.rider.v = world.rider.v.max(4.0);
                for ev in world.substep(Input::default()) {
                    if let WorldEvent::Pothole { flat, .. } = ev {
                        hit = Some(flat);
                    }
                }
            }
            assert_eq!(hit, Some(flat));
            assert_eq!(world.rider.flat, flat);
        }
    }

    #[test]
    fn act_now_comes_only_for_hazards_in_your_line() {
        let course = Course::empty("Hole", 100.0).with(Hazard {
            kind: HazardKind::Pothole,
            s: 40.0,
            x0: 2.45,
            x1: 3.15,
        });
        for (x, want) in [(2.8, true), (5.0, false)] {
            let mut world = World::new(course.clone(), Bike::Bmx, 1);
            world.rider.x = x;
            let mut act = false;
            while world.finished.is_none() {
                world.rider.v = 5.0;
                for ev in world.substep(Input::default()) {
                    act |= ev == WorldEvent::ActNow(0);
                }
            }
            assert_eq!(act, want);
        }
    }
}
