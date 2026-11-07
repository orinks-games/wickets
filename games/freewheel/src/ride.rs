//! Riding: the play screen for time trials, free rides, training lessons
//! and challenges. It reads the held keys into an `Input`, steps the
//! world, and turns what happened into sound placed around the rider and
//! short spoken lines.

use std::cell::RefCell;
use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{
    units, Challenge, Challenges, Pages, Parlor, ParlorAction, Period, Rng, ScoreResult,
};

use crate::bike::{Bike, Input, RideEvent, MIN_MOVING, TUBE_SWAP_S};
use crate::challenge::{self, Plan};
use crate::clock;
use crate::course::{
    Course, CourseInfo, District, HazardKind, BIKE_LANE_LINE, CENTRE_LINE, COURSES, PARKED_EDGE,
    TURN_SPEED,
};
use crate::events;
use crate::progress::{
    self, distance_words, dollar_words, metres_words, near_words, secs_words, span_words,
    speed_words, time_words, CallOuts, Choices, Progress, Saved,
};
use crate::rides::CHALLENGES_NAME;
use crate::sound::{self, place};
use crate::training;
use crate::world::{Cause, World, WorldEvent, BODY_HALF, ENDLESS_CRASHES, TYRE_HALF};
use crate::{HELP, TITLE_NAME};

/// The ride screen's `State::name`.
pub const RIDE_NAME: &str = "freewheel_ride";
/// Seconds of countdown before a timed ride starts.
pub const COUNTDOWN: f64 = 3.0;
/// A free ride's length, metres.
pub const FREE_LENGTH: f64 = 3000.0;

/// Which ride this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A Downtown time trial, by its place in `COURSES`.
    TimeTrial(usize),
    FreeRide,
    /// A training lesson, from 1.
    Training(u32),
    Challenge(Period),
    /// The autumn leaf season ride, with leaf piles.
    LeafSeason,
    /// Car-free Sunday's ride on streets closed to cars.
    Sunday,
    /// Ride on through a district until the third crash.
    Endless(District),
    /// A job from the job board, in this district.
    Delivery(District),
    /// Against the clock through a district: drops feed the clock, and
    /// the ride ends when it runs out.
    Clock(District),
}

impl Mode {
    /// An event ride's name and board, if this is one.
    pub fn event(self) -> Option<(&'static str, &'static str)> {
        match self {
            Mode::LeafSeason => Some((events::LEAF_SEASON, events::LEAF_BOARD)),
            Mode::Sunday => Some((events::SUNDAY, events::SUNDAY_BOARD)),
            _ => None,
        }
    }
}

/// How long a leaf pile muffles the freewheel, seconds.
pub const MUFFLE_S: f64 = 0.5;
/// A hazard or oncoming car this close, in metres, leads the status key.
pub const DANGER_AHEAD: f64 = 60.0;
/// A hazard or oncoming car this many seconds away comes first in the
/// hint, even before a flat tyre.
pub const URGENT_S: f64 = 2.0;
/// The freewheel's loudness while muffled.
pub const MUFFLED: f32 = 0.35;

thread_local! {
    /// Cues already named aloud this session.
    static TAUGHT: RefCell<HashSet<&'static str>> = RefCell::new(HashSet::new());
}

/// Whether this is the first time this session that `cue` is named, and
/// notes that it now has been.
fn first_time(cue: &'static str) -> bool {
    TAUGHT.with(|t| t.borrow_mut().insert(cue))
}

/// Forgets which cues have been named, as a new session would.
pub fn forget_cues() {
    TAUGHT.with(|t| t.borrow_mut().clear());
}

struct Loops {
    roll: Sound,
    freehub: Sound,
    pedal: Sound,
    wind: Sound,
    edge: Sound,
    door_zone: Sound,
    cobbles: Sound,
}

impl Default for Loops {
    fn default() -> Self {
        Loops {
            roll: Sound::NONE,
            freehub: Sound::NONE,
            pedal: Sound::NONE,
            wind: Sound::NONE,
            edge: Sound::NONE,
            door_zone: Sound::NONE,
            cobbles: Sound::NONE,
        }
    }
}

/// A training lesson's progress.
#[derive(Default)]
struct Lesson {
    fast: bool,
    /// Moving in the brake lesson before reaching its speed.
    rolling: bool,
    coasted: f64,
    out: bool,
    cleared: bool,
    missed: u32,
    failed: bool,
}

pub struct Ride {
    pub world: World,
    pub mode: Mode,
    challenge: Option<Challenge>,
    par: f64,
    countdown: f64,
    count_said: i32,
    started: bool,
    hop: bool,
    call_outs: CallOuts,
    last_line: String,
    loops: Loops,
    cars: Vec<(usize, Sound)>,
    oncoming: Vec<(usize, Sound)>,
    ticks: Vec<f64>,
    lesson: Lesson,
    finished: bool,
    restart_in: Option<f64>,
    beacon_in: Option<f64>,
    jumping: bool,
    was_at_edge: bool,
    best_air: f64,
    /// Seconds the freewheel stays muffled after a leaf pile.
    muffle: f64,
    /// Inside the tutorial: a passed lesson stops here, and a restart
    /// starts the street over in place.
    inside: bool,
    /// The training lesson was passed.
    pub passed: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// An endless ride's parcel money is in the bank.
    banked: bool,
    /// The delivery this ride makes, from the job board.
    job: Option<crate::jobs::Job>,
    /// Seconds to the low clock's next tick.
    tick_in: f64,
}

impl Ride {
    fn new(world: World, mode: Mode, par: f64) -> Ride {
        let n = world.course.hazards.len();
        let countdown = match mode {
            Mode::Training(_) => 0.0,
            _ => COUNTDOWN,
        };
        Ride {
            status_key: Default::default(),
            world,
            mode,
            challenge: None,
            par,
            countdown,
            count_said: COUNTDOWN as i32 + 1,
            started: false,
            hop: false,
            call_outs: CallOuts::default(),
            last_line: String::new(),
            loops: Loops::default(),
            cars: Vec::new(),
            oncoming: Vec::new(),
            ticks: vec![0.0; n],
            lesson: Lesson::default(),
            finished: false,
            restart_in: None,
            beacon_in: None,
            jumping: false,
            was_at_edge: false,
            best_air: 0.0,
            muffle: 0.0,
            inside: false,
            passed: false,
            banked: false,
            job: None,
            tick_in: 0.0,
        }
    }

    /// A delivery from the job board.
    pub fn delivery(job: crate::jobs::Job, bike: Bike) -> Ride {
        let world = World::new(job.course(), bike, job.seed ^ 0x006a_6f62);
        let mut ride = Ride::new(world, Mode::Delivery(job.district), job.deadline());
        ride.job = Some(job);
        ride
    }

    /// An endless ride through `district`, until the third crash.
    pub fn endless(bike: Bike, district: District, seed: u64) -> Ride {
        let course = Course::endless(district, seed);
        let world = World::new(course, bike, seed ^ 0x656e_646c).with_crash_limit(ENDLESS_CRASHES);
        Ride::new(world, Mode::Endless(district), 100.0)
    }

    /// A ride against the clock through `district`: drops feed the clock,
    /// and it ends when the clock runs out.
    pub fn against_clock(bike: Bike, district: District, seed: u64) -> Ride {
        let course = clock::course(district, seed);
        let world = World::new(course, bike, seed ^ 0x636c_6b72).with_drop_clock();
        Ride::new(world, Mode::Clock(district), 100.0)
    }

    /// "Parcel" in the endless ride; "Drop" against the clock, where each
    /// one is a delivery made.
    fn parcel_word(&self) -> &'static str {
        if matches!(self.mode, Mode::Clock(_)) {
            "Drop"
        } else {
            "Parcel"
        }
    }

    /// "Hold the Up arrow to pedal.", or on the moped, "for throttle".
    fn go_words(&self, ctx: &Ctx<Parlor>) -> String {
        let up = Self::key(ctx, ParlorAction::Up);
        if self.world.bike.motor() {
            format!("Hold {up} for throttle.")
        } else {
            format!("Hold {up} to pedal.")
        }
    }

    /// This leaf season's ride, with its leaf piles.
    pub fn leaf_season(bike: Bike, today: parlor_core::Date) -> Ride {
        let (course, piles) = events::leaf_course(today);
        let traffic = course.seed ^ 0x6c66;
        let par = events::par(course.length);
        let world = World::new(course, bike, traffic).with_leaves(&piles);
        Ride::new(world, Mode::LeafSeason, par)
    }

    /// Today's Car-free Sunday ride on `course`, with no cars.
    pub fn sunday(bike: Bike, course: Course) -> Ride {
        let par = events::par(course.length);
        Ride::new(World::new(course, bike, 1), Mode::Sunday, par)
    }

    /// A Downtown time trial. Its traffic is the same every time, so a
    /// course can be learned.
    pub fn time_trial(course: usize, bike: Bike) -> Ride {
        let info = &COURSES[course.min(COURSES.len() - 1)];
        let world = World::new(Course::downtown(info), bike, info.seed ^ 0x7472_6166);
        Ride::new(world, Mode::TimeTrial(course), info.par)
    }

    /// A free ride on a fresh stretch of Downtown, `FREE_LENGTH` long.
    pub fn free_ride(bike: Bike, seed: u64) -> Ride {
        let course = Course::generate("Free ride", seed, FREE_LENGTH, crate::course::Mix::DOWNTOWN);
        Ride::new(
            World::new(course, bike, seed ^ 0x6672),
            Mode::FreeRide,
            400.0,
        )
    }

    /// Training lesson `n`, from 1.
    pub fn training(n: u32, bike: Bike) -> Ride {
        Ride::new(
            World::new(training::course(n), bike, 1),
            Mode::Training(n),
            60.0,
        )
    }

    /// Training lesson `n` inside the tutorial, which moves on itself once
    /// it is passed.
    pub fn for_lesson(n: u32, bike: Bike) -> Ride {
        Ride {
            status_key: Default::default(),
            inside: true,
            ..Ride::training(n, bike)
        }
    }

    /// The same lesson from the start of its street, in place.
    fn start_over(&mut self, ctx: &mut Ctx<Parlor>, n: u32) {
        let bike = self.world.bike;
        self.stop_all(ctx);
        *self = Ride::for_lesson(n, bike);
        self.enter(ctx);
    }

    /// This period's challenge ride.
    pub fn challenge(ch: Challenge) -> Ride {
        let plan = Plan::for_challenge(&ch);
        let world = World::new(plan.course, plan.bike, plan.traffic_seed);
        let mut ride = Ride::new(world, Mode::Challenge(ch.period), plan.par);
        ride.challenge = Some(ch);
        ride
    }

    /// Any course, for tests and demonstrations: a time trial of course 0's
    /// records, on this course.
    pub fn custom(course: Course, bike: Bike, mode: Mode) -> Ride {
        Ride::new(World::new(course, bike, 1), mode, 100.0)
    }

    /// Any world, for tests: a ride of `mode` on it, worth 100,000 ride
    /// points at `par` seconds.
    pub fn from_world(world: World, mode: Mode, par: f64) -> Ride {
        Ride::new(world, mode, par)
    }

    /// Whether the ride has started: the countdown is over.
    pub fn rolling(&self) -> bool {
        self.countdown <= 0.0
    }

    fn leave(&self) -> Leave {
        match self.mode {
            Mode::Challenge(_) => Leave::Challenge(CHALLENGES_NAME),
            _ => Leave::Title(TITLE_NAME),
        }
    }

    // -- speech ----------------------------------------------------------

    /// Says `line` now, for R to repeat.
    fn tell(&mut self, ctx: &mut Ctx<Parlor>, line: String) {
        ctx.say(line.clone());
        self.last_line = line;
    }

    /// Names a cue by the call-out setting: the long line the first time
    /// this session, the short one after that when call-outs are on for
    /// every cue.
    fn call(&mut self, ctx: &mut Ctx<Parlor>, cue: &'static str, long: String, short: String) {
        let first = first_time(cue);
        let line = match (self.call_outs, first) {
            (CallOuts::Off, _) => return,
            (_, true) => long,
            (CallOuts::All, false) => short,
            (CallOuts::FirstTime, false) => return,
        };
        self.tell(ctx, line);
    }

    /// `call`, but waiting for any line already speaking: for the leaf
    /// piles, which never cut in on a hazard's warning.
    fn call_queued(
        &mut self,
        ctx: &mut Ctx<Parlor>,
        cue: &'static str,
        long: String,
        short: String,
    ) {
        let first = first_time(cue);
        let line = match (self.call_outs, first) {
            (CallOuts::Off, _) => return,
            (_, true) => long,
            (CallOuts::All, false) => short,
            (CallOuts::FirstTime, false) => return,
        };
        ctx.say_queued(line.clone());
        self.last_line = line;
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// Where a spot `x` across the road is from the rider, in words.
    fn side_words(&self, x0: f64, x1: f64) -> String {
        self.side_words_for(x0, x1, BODY_HALF)
    }

    /// Where hazard `i` is from the rider, measured the way the world
    /// measures contact with it: the tyre for a pothole, curb or ramp,
    /// the body for a door. "In your line" then means the act-now beep
    /// and the hit will come.
    fn hazard_side_words(&self, i: usize) -> String {
        let h = self.world.course.hazards[i];
        let half = if h.kind == HazardKind::Door {
            BODY_HALF
        } else {
            TYRE_HALF
        };
        self.side_words_for(h.x0, h.x1, half)
    }

    fn side_words_for(&self, x0: f64, x1: f64, half: f64) -> String {
        let r = &self.world.rider;
        if r.x + half > x0 && r.x - half < x1 {
            "in your line".to_string()
        } else if x0 > r.x {
            format!("{} to your left", near_words(x0 - r.x - half))
        } else {
            format!("{} to your right", near_words(r.x - half - x1))
        }
    }

    fn start_line(&self, ctx: &Ctx<Parlor>) -> String {
        let bike = self.world.bike.name();
        match self.mode {
            Mode::TimeTrial(c) => format!(
                "Time trial, {}, on the {bike}. {}",
                COURSES[c].name,
                COURSES[c].about()
            ),
            Mode::FreeRide => format!(
                "Free ride on the {bike}, {}, no clock. {}",
                units::route(FREE_LENGTH),
                self.go_words(ctx)
            ),
            Mode::Delivery(d) => match &self.job {
                Some(job) => format!(
                    "Delivery, {}, {}, on the {bike}. {}, deadline {}.",
                    job.title(),
                    d.name(),
                    units::route(job.length),
                    span_words(job.deadline())
                ),
                None => format!("Delivery in {}, on the {bike}.", d.name()),
            },
            Mode::Endless(d) => format!(
                "Endless ride, {}, on the {bike}. {ENDLESS_CRASHES} crashes end it.",
                d.name()
            ),
            Mode::Clock(d) => format!(
                "{}, {}, on the {bike}. {}. Drops add time.",
                clock::NAME,
                d.name(),
                sentence_start(&secs_words(clock::START_S))
            ),
            Mode::Training(n) => training::goal(ctx, n),
            Mode::LeafSeason => format!(
                "{}, on the {bike}, {}. Ride through leaf piles for {} points each.",
                events::LEAF_SEASON,
                units::route(self.world.course.length),
                events::LEAF_POINTS
            ),
            Mode::Sunday => format!(
                "{}, on the {bike}, {}. No cars today.",
                events::SUNDAY,
                units::route(self.world.course.length)
            ),
            Mode::Challenge(p) => {
                let plan = self.challenge.as_ref().map(Plan::for_challenge);
                format!(
                    "{}: {}. {}.",
                    p.label(),
                    challenge::name(p),
                    sentence_start(&plan.map(|p| p.tag()).unwrap_or_default())
                )
            }
        }
    }

    /// The status key, in layers. First, most urgent first: a hazard or
    /// oncoming car ahead, a flat tyre, then your speed and lane. A second
    /// press: what else is ahead, the gap to the road's edge, the clock,
    /// leaf piles and your breath.
    pub fn status_layers(&self) -> Vec<String> {
        let w = &self.world;
        let r = &w.rider;
        let mut first = Vec::new();
        if !self.rolling() {
            first.push(format!(
                "Starting in {}.",
                secs_words(self.countdown.ceil())
            ));
        }
        let (danger, ahead) = self.ahead();
        if danger {
            first.push(format!("{}.", sentence_start(&ahead)));
        }
        if r.flat {
            first.push("Flat tyre.".to_string());
        }
        let speed = if r.down() {
            format!("Down, up in {}", secs_words(r.down_for))
        } else if r.v == 0.0 {
            "Stopped".to_string()
        } else {
            speed_words(r.v)
        };
        let (lane, gap) = if r.x < BIKE_LANE_LINE {
            let lane = if w.in_door_zone {
                "bike lane, in the door zone"
            } else {
                "bike lane"
            };
            (
                lane,
                format!(
                    "{} from the parked cars",
                    near_words(r.x - BODY_HALF - PARKED_EDGE)
                ),
            )
        } else {
            (
                "traffic lane",
                format!(
                    "{} from the centre line",
                    near_words(CENTRE_LINE - BODY_HALF - r.x)
                ),
            )
        };
        first.push(match &w.drop_clock {
            Some(c) => format!(
                "{} left. {speed}, {lane}.",
                sentence_start(&secs_words(c.left.ceil()))
            ),
            None => format!("{speed}, {lane}."),
        });
        let mut more = Vec::new();
        if !danger {
            more.push(ahead);
        }
        more.push(gap);
        match self.mode {
            Mode::TimeTrial(_)
            | Mode::Challenge(_)
            | Mode::LeafSeason
            | Mode::Sunday => {
                more.push(time_words(w.clock));
            }
            Mode::Delivery(_) => {
                more.push(time_words(w.clock));
                if let Some(job) = &self.job {
                    let left = job.deadline() - w.clock;
                    more.push(if left >= 0.0 {
                        format!("{} to the deadline", time_words(left))
                    } else {
                        "late, half pay".to_string()
                    });
                }
            }
            Mode::Clock(_) => {
                if let Some(c) = &w.drop_clock {
                    more.push(clock::drop_words(c.drops));
                }
                more.push(dollar_words(w.stats.cash));
            }
            Mode::Endless(_) => {
                more.push(distance_words(r.s));
                more.push(dollar_words(w.stats.cash));
                if let Some(left) = w.crashes_left() {
                    more.push(match left {
                        1 => "1 crash left".to_string(),
                        n => format!("{n} crashes left"),
                    });
                }
            }
            Mode::FreeRide | Mode::Training(_) => {}
        }
        if self.mode == Mode::LeafSeason {
            more.push(format!(
                "leaf piles {} of {}",
                w.stats.leaves,
                w.leaves.len()
            ));
        }
        // An engine has no sprint, so no stamina to report.
        if self.world.bike.spec().engine_w == 0.0 {
            more.push(if r.stamina < 0.995 {
                format!("stamina {} percent", (r.stamina * 100.0).round() as u32)
            } else {
                "full stamina".to_string()
            });
        }
        vec![
            first.join(" "),
            format!("{}.", sentence_start(&more.join(", "))),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }

    /// The nearest thing ahead, "door in 7 metres", and whether it is a
    /// danger: a hazard or an oncoming car, rather than a turn or the
    /// finish.
    fn ahead(&self) -> (bool, String) {
        let w = &self.world;
        let r = &w.rider;
        // A danger in the rider's line leads, the sooner of a car and a
        // hazard, as in the hint; only then the nearest thing ahead.
        let car_line = self.car_in_line_secs();
        let hazard_line = self.hazard_in_line_secs();
        let in_line = match (car_line, hazard_line) {
            (Some(c), h) if h.is_none_or(|h| c <= h) => w
                .oncoming_ahead()
                .map(|car| (car.gap(r).max(0.0), "oncoming car".to_string())),
            (_, Some(_)) => w.next_hazard().map(|i| {
                let h = w.course.hazards[i];
                (h.s - r.s, h.kind.word().to_string())
            }),
            _ => None,
        };
        if let Some((m, word)) = in_line.filter(|(m, _)| *m <= DANGER_AHEAD) {
            return (true, format!("{word} in {}", metres_words(m)));
        }
        let mut best = (w.course.length - r.s, "finish".to_string(), false);
        if let Mode::Endless(_) | Mode::Clock(_) = self.mode {
            best = (f64::INFINITY, String::new(), false);
        }
        if let Some(car) = w.oncoming_ahead() {
            best = (car.gap(r).max(0.0), "oncoming car".to_string(), true);
        }
        if let Some(i) = w.next_hazard() {
            let h = w.course.hazards[i];
            if h.s - r.s < best.0 {
                best = (h.s - r.s, h.kind.word().to_string(), true);
            }
        }
        if let Some(i) = w.next_turn() {
            let j = w.course.junctions[i];
            if j.s - r.s < best.0 {
                best = (j.s - r.s, format!("{} turn", j.turn.word()), false);
            }
        }
        if best.0.is_infinite() {
            return (false, "clear road ahead".to_string());
        }
        let danger = best.2 && best.0 <= DANGER_AHEAD;
        (danger, format!("{} in {}", best.1, metres_words(best.0)))
    }

    /// Seconds until an oncoming car whose path the rider is in meets
    /// them.
    fn car_in_line_secs(&self) -> Option<f64> {
        let w = &self.world;
        let r = &w.rider;
        w.oncoming_ahead()
            .filter(|c| c.in_path(r.x))
            .map(|c| c.secs(r))
    }

    /// Seconds until the next hazard, when the rider's line meets it:
    /// `INFINITY` while stopped.
    fn hazard_in_line_secs(&self) -> Option<f64> {
        let w = &self.world;
        let c = w.predict_contact(w.next_hazard()?);
        c.in_line.then(|| c.secs.unwrap_or(f64::INFINITY))
    }

    /// Whether an oncoming car or a hazard in the rider's line meets
    /// them within [`URGENT_S`]: then the hint speaks of it before a
    /// flat tyre. A danger the rider is clear of does not count.
    fn danger_soon(&self) -> bool {
        self.car_in_line_secs().is_some_and(|s| s <= URGENT_S)
            || self.hazard_in_line_secs().is_some_and(|s| s <= URGENT_S)
    }

    /// The hint's line for the next hazard once it is near or heard, and
    /// whether the rider's line meets it.
    fn hazard_hint(&self, ctx: &Ctx<Parlor>) -> Option<(bool, String)> {
        let w = &self.world;
        let r = &w.rider;
        let p = Self::key(ctx, ParlorAction::Primary);
        let i = w.next_hazard()?;
        let h = w.course.hazards[i];
        let c = w.predict_contact(i);
        if c.metres > 60.0 && !w.hazards[i].warned {
            return None;
        }
        let when = match c.secs {
            Some(s) => format!("in {}", secs_words(s)),
            None => format!("{} ahead", metres_words(c.metres)),
        };
        let side = self.hazard_side_words(i);
        let away = if h.x() > r.x { "right" } else { "left" };
        let line = match (h.kind, c.in_line) {
            (HazardKind::Door, true) => {
                format!("Car door {when}, in your line. Steer left or brake.")
            }
            (HazardKind::Door, false) => format!("Car door {when}. You are clear."),
            (HazardKind::Pothole, true) if w.bike.spec().hop_m == 0.0 => {
                format!("Pothole in your line {when}. Steer {away}.")
            }
            (HazardKind::Pothole, true) => {
                format!("Pothole in your line {when}. Steer {away} or hop with {p}.")
            }
            (HazardKind::Pothole, false) => format!("Pothole {side}, {when}. Hold your line."),
            // The way around is the traffic lane: not while a car comes
            // down it.
            (HazardKind::Curb, true) if !w.bike.clears_curbs() && w.oncoming_near() => {
                let down = Self::key(ctx, ParlorAction::Down);
                format!("Curb {when}, oncoming car in the traffic lane. Brake with {down}.")
            }
            // Nor while a car from behind is about to pass.
            (HazardKind::Curb, true)
                if !w.bike.clears_curbs()
                    && w.car_behind().is_some_and(|car| car.gap(&w.rider) < 15.0) =>
            {
                let down = Self::key(ctx, ParlorAction::Down);
                format!("Curb {when}, car behind. Brake with {down}, let it pass, then go around.")
            }
            (HazardKind::Curb, true) if !w.bike.clears_curbs() => {
                format!("Curb {when}. Steer left into the traffic lane.")
            }
            (HazardKind::Curb, true) => format!("Curb {when}. Hop with {p} just before it."),
            (HazardKind::Curb, false) => format!("Curb {when}. You are clear."),
            // A bike with no hop just rolls over a ramp.
            (HazardKind::Ramp, true) if w.bike.spec().hop_m == 0.0 => {
                format!("Ramp in your line {when}. Ride over it.")
            }
            (HazardKind::Ramp, false) if w.bike.spec().hop_m == 0.0 => {
                format!("Ramp {side}, {when}. You are clear.")
            }
            (HazardKind::Ramp, true) => {
                format!("Ramp in your line {when}. Jump with {p} at its lip.")
            }
            (HazardKind::Ramp, false) => {
                format!("Ramp {side}, {when}. Jump with {p} at its lip, or pass it.")
            }
        };
        Some((c.in_line, line))
    }

    /// The hint: what is coming and what to do about it.
    pub fn hint(&self, ctx: &Ctx<Parlor>) -> String {
        let w = &self.world;
        let r = &w.rider;
        let up = Self::key(ctx, ParlorAction::Up);
        let down = Self::key(ctx, ParlorAction::Down);
        if !self.rolling() {
            return format!("Wait for go. {}", self.go_words(ctx));
        }
        if r.down() {
            return format!("You are down. Back up in {}.", secs_words(r.down_for));
        }
        if r.flat && !self.danger_soon() {
            return format!(
                "Flat tyre. Hold {down} to stop, then wait {} to swap the tube.",
                secs_words(TUBE_SWAP_S)
            );
        }
        // The danger in the rider's line first, the sooner of a car and
        // a hazard; only then one they are clear of.
        let hazard = self.hazard_hint(ctx);
        let car_line = self.car_in_line_secs();
        let hazard_line = hazard
            .as_ref()
            .filter(|(in_line, _)| *in_line)
            .and_then(|_| self.hazard_in_line_secs());
        match (car_line, hazard_line) {
            (Some(c), h) if h.is_none_or(|h| c <= h) => {
                return format!(
                    "Oncoming car in {}, in your line. Steer right.",
                    secs_words(c)
                );
            }
            (_, Some(_)) => return hazard.map(|(_, line)| line).unwrap_or_default(),
            _ => {}
        }
        if let Some(car) = w.oncoming_ahead() {
            return format!(
                "Oncoming car in {}. You are clear.",
                secs_words(car.secs(r))
            );
        }
        if let Some(car) = w.car_behind() {
            if car.stuck_for > 1.0 {
                return "Car waiting behind. Move right into the bike lane.".to_string();
            }
        }
        if let Some((_, line)) = hazard {
            return line;
        }
        if let Some(i) = w.next_turn() {
            let j = w.course.junctions[i];
            let d = j.s - r.s;
            if d <= 150.0 {
                let (lo, hi) = j.turn.band();
                let placed = if r.x >= lo && r.x <= hi {
                    "You are on the right side."
                } else if j.turn == crate::course::Turn::Left {
                    "Steer left to get there."
                } else {
                    "Steer right to get there."
                };
                let speed = if r.v > TURN_SPEED {
                    format!(" Brake with {down} before the line.")
                } else {
                    String::new()
                };
                return format!(
                    "{} turn in {}. Be in {}, under {}. {placed}{speed}",
                    sentence_start(j.turn.word()),
                    metres_words(d),
                    j.turn.place(),
                    speed_words(TURN_SPEED)
                );
            }
        }
        if let Some(i) = w.next_leaves() {
            let p = w.leaves[i];
            let d = p.s - r.s;
            if d <= 60.0 {
                let side =
                    self.side_words(p.x - crate::world::LEAF_HALF, p.x + crate::world::LEAF_HALF);
                return format!(
                    "Leaf pile {side}, {} ahead, {} points.",
                    metres_words(d),
                    events::LEAF_POINTS
                );
            }
        }
        if let Some(i) = w.next_parcel() {
            let c = w.course.parcels[i];
            let d = c.s - r.s;
            if d <= 50.0 {
                let side = self.side_words(c.x - 0.1, c.x + 0.1);
                let verb = if matches!(self.mode, Mode::Clock(_)) {
                    "Make it"
                } else {
                    "Grab it"
                };
                return format!(
                    "{} {side}, {} ahead. {verb} under {}.",
                    self.parcel_word(),
                    metres_words(d),
                    speed_words(crate::world::GRAB_SPEED)
                );
            }
        }
        if w.bike.motor() {
            return format!("Clear road. Hold {up} for throttle.");
        }
        format!(
            "Clear road. Hold {up} to pedal, {} to sprint.",
            Self::key(ctx, ParlorAction::Secondary)
        )
    }

    // -- events ------------------------------------------------------------

    fn hazard_pos(&self, i: usize) -> Vec3 {
        let h = self.world.course.hazards[i];
        let r = &self.world.rider;
        place(r.x - h.x(), h.s - r.s)
    }

    fn on_event(&mut self, ctx: &mut Ctx<Parlor>, ev: WorldEvent) {
        let p = Self::key(ctx, ParlorAction::Primary);
        match ev {
            WorldEvent::Warn(i) => {
                let h = self.world.course.hazards[i];
                let pos = self.hazard_pos(i);
                let side = self.hazard_side_words(i);
                self.ticks[i] = 0.5;
                match h.kind {
                    HazardKind::Door => {
                        sound::play_at(ctx, sound::DOOR_CLICK, pos);
                        self.call(
                            ctx,
                            "door",
                            format!("Car door about to open, {side}. Move left."),
                            format!("Door, {side}."),
                        );
                    }
                    HazardKind::Pothole => {
                        sound::play_at(ctx, sound::MARKER, pos);
                        self.call(
                            ctx,
                            "pothole",
                            format!("That tick is a pothole, {side}."),
                            format!("Pothole, {side}."),
                        );
                    }
                    HazardKind::Curb => {
                        sound::play_at(ctx, sound::CURB, pos);
                        let advice = if !self.world.bike.clears_curbs() {
                            "Ride around it in the traffic lane.".to_string()
                        } else {
                            format!("Hop it with {p}, or ride around.")
                        };
                        self.call(
                            ctx,
                            "curb",
                            format!("Curb across the bike lane, {side}. {advice}"),
                            format!("Curb, {side}."),
                        );
                    }
                    HazardKind::Ramp => {
                        sound::play_at(ctx, sound::RAMP, pos);
                        self.call(
                            ctx,
                            "ramp",
                            if self.world.bike.spec().hop_m == 0.0 {
                                format!("Ramp, {side}. This bike rolls over it.")
                            } else {
                                format!("Ramp, {side}. Jump with {p} at its lip.")
                            },
                            format!("Ramp, {side}."),
                        );
                    }
                }
            }
            WorldEvent::DoorOpen(i) => {
                let pos = self.hazard_pos(i);
                sound::play_at(ctx, sound::DOOR_OPEN, pos);
                self.call(
                    ctx,
                    "door_open",
                    "Door opening.".to_string(),
                    "Door open.".to_string(),
                );
            }
            WorldEvent::ActNow(_) => {
                sound::play(ctx, sound::ACT_NOW);
                self.call(
                    ctx,
                    "act_now",
                    "Now. In your line.".to_string(),
                    "Now.".to_string(),
                );
            }
            WorldEvent::Cleared(_) => {
                self.call(
                    ctx,
                    "cleared",
                    "Clean hop.".to_string(),
                    "Cleared.".to_string(),
                );
                if let Mode::Training(5) = self.mode {
                    self.lesson.cleared = true;
                }
            }
            WorldEvent::Pothole { flat: true, .. } => {
                sound::play(ctx, sound::FLAT);
                // The hop lesson starts over anyway: no tube swap to explain.
                let training = matches!(self.mode, Mode::Training(_));
                let line = if !training && first_time("flat") {
                    format!(
                        "Flat tyre. Stop and wait {} to swap the tube.",
                        secs_words(TUBE_SWAP_S)
                    )
                } else {
                    "Pothole. Flat tyre.".to_string()
                };
                self.tell(ctx, line);
            }
            WorldEvent::Pothole { flat: false, .. } => {
                sound::play(ctx, sound::BUMP);
                self.call(
                    ctx,
                    "bump",
                    "Pothole. Rolled through.".to_string(),
                    "Pothole.".to_string(),
                );
            }
            WorldEvent::CurbStrike { crash, .. } => {
                sound::play(ctx, sound::CURB_STRIKE);
                if !crash {
                    let tip = if self.world.bike.clears_curbs() {
                        "Curb strike. Hop earlier."
                    } else {
                        "Curb strike. Ride around curbs."
                    };
                    self.tell(ctx, tip.to_string());
                }
            }
            WorldEvent::Jump(_) => {
                sound::play(ctx, sound::HOP);
                self.jumping = true;
                self.call(ctx, "jump", "Jump.".to_string(), "Jump.".to_string());
            }
            WorldEvent::Crash(cause) => {
                sound::play(ctx, sound::CRASH);
                self.jumping = false;
                let why = match cause {
                    Cause::Door => "Doored.",
                    Cause::Curb => "Curb strike. This bike cannot hop curbs.",
                    Cause::Car => "Hit by a car.",
                    Cause::Oncoming => "Hit an oncoming car.",
                };
                let lost = self.world.drop_clock.map(|c| c.lost);
                let line = match self.world.crash_limit {
                    _ if lost.is_some() => {
                        format!("{why} Minus {} seconds.", clock::CRASH_S as i64)
                    }
                    Some(n) if self.world.stats.crashes >= n => {
                        format!("{why} Crash {n} of {n}.")
                    }
                    Some(n) => format!("{why} Crash {} of {n}.", self.world.stats.crashes),
                    None => format!("{why} Down for 3 seconds."),
                };
                self.tell(ctx, line);
                if let (Mode::Training(4), Cause::Door) = (self.mode, cause) {
                    self.lesson.failed = true;
                }
            }
            WorldEvent::Ride(RideEvent::Up) => {
                sound::play(ctx, sound::UP);
                if self.lesson.failed {
                    self.restart_in = Some(0.5);
                    self.tell(ctx, "Back up. The lesson starts again.".to_string());
                } else {
                    let go = self.go_words(ctx);
                    self.tell(ctx, format!("Back up. {go}"));
                }
            }
            WorldEvent::Ride(RideEvent::Hop) => {
                sound::play(ctx, sound::HOP);
            }
            WorldEvent::Ride(RideEvent::Land { air, hard }) => {
                sound::play(
                    ctx,
                    if hard {
                        sound::LAND_HARD
                    } else {
                        sound::LAND_CLEAN
                    },
                );
                if std::mem::take(&mut self.jumping) {
                    self.best_air = self.best_air.max(air);
                    self.tell(ctx, format!("Landed, {:.1} seconds.", air));
                }
            }
            WorldEvent::Ride(RideEvent::Spent) => {
                sound::play(ctx, sound::SPENT);
                self.call(
                    ctx,
                    "spent",
                    "Out of breath. Ride easy to recover.".to_string(),
                    "Out of breath.".to_string(),
                );
            }
            WorldEvent::Ride(RideEvent::TubeSwapped) => {
                sound::play(ctx, sound::TUBE);
                let go = self.go_words(ctx);
                self.tell(ctx, format!("Tube swapped. {go}"));
            }
            WorldEvent::Ride(RideEvent::Edge) => {}
            WorldEvent::TurnAhead(i) => {
                let j = self.world.course.junctions[i];
                let d = j.s - self.world.rider.s;
                let (key, side) = match j.turn {
                    crate::course::Turn::Left => (sound::TURN_LEFT, 3.0),
                    crate::course::Turn::Right => (sound::TURN_RIGHT, -3.0),
                };
                sound::play_at(ctx, key, place(-side, 6.0));
                let head = format!(
                    "{} turn in {}.",
                    sentence_start(j.turn.word()),
                    metres_words(d)
                );
                self.call(
                    ctx,
                    "turn",
                    format!(
                        "{head} Be in {}, under {}.",
                        j.turn.place(),
                        speed_words(TURN_SPEED)
                    ),
                    head,
                );
            }
            WorldEvent::Turned(i) => {
                sound::play(ctx, sound::TURNED);
                let word = self.world.course.junctions[i].turn.word();
                self.call(
                    ctx,
                    "turned",
                    format!("Turned {word}."),
                    format!("Turned {word}."),
                );
            }
            WorldEvent::TurnWide(i) => {
                sound::play(ctx, sound::SKID);
                let j = self.world.course.junctions[i];
                let (lo, hi) = j.turn.band();
                let x = self.world.rider.x;
                let line = if x >= lo && x <= hi {
                    format!(
                        "Ran wide, too fast. Under {} for turns.",
                        speed_words(TURN_SPEED)
                    )
                } else {
                    format!(
                        "Ran wide, wrong side for a {} turn. Be in {}.",
                        j.turn.word(),
                        j.turn.place()
                    )
                };
                self.tell(ctx, line);
            }
            WorldEvent::CarBehind(_) => {
                self.call(
                    ctx,
                    "car",
                    "Car behind. Hold your line.".to_string(),
                    "Car behind.".to_string(),
                );
            }
            WorldEvent::Horn(_) => {
                sound::play_at(ctx, sound::HORN, place(-1.0, -6.0));
                self.call(
                    ctx,
                    "horn",
                    "Horn. Move right to let it pass.".to_string(),
                    "Car waiting behind.".to_string(),
                );
            }
            WorldEvent::CarPassed { thanked: true, .. } => {
                sound::play_at(ctx, sound::TOOT, place(-2.0, 6.0));
                self.call(
                    ctx,
                    "toot",
                    "Thank-you toot. Tip.".to_string(),
                    "Tip.".to_string(),
                );
            }
            WorldEvent::CarPassed { .. } => {}
            WorldEvent::FinishAhead => {
                sound::play_at(ctx, sound::BEACON, place(0.0, 8.0));
                self.beacon_in = Some(1.5);
                let d = self.world.course.length - self.world.rider.s;
                self.call(
                    ctx,
                    "finish",
                    format!("Finish bell, {} ahead.", metres_words(d)),
                    "Finish ahead.".to_string(),
                );
            }
            WorldEvent::Finished => {}
            WorldEvent::Line(into) => {
                sound::play(ctx, sound::LINE);
                if into {
                    self.call(
                        ctx,
                        "line_out",
                        "Lane line. Traffic lane, cars pass here.".to_string(),
                        "Traffic lane.".to_string(),
                    );
                } else {
                    self.call(
                        ctx,
                        "line_in",
                        "Bike lane.".to_string(),
                        "Bike lane.".to_string(),
                    );
                }
            }
            WorldEvent::DoorZone(true) => {
                self.call(
                    ctx,
                    "door_zone",
                    "Door zone. Doors can open here.".to_string(),
                    "Door zone.".to_string(),
                );
            }
            WorldEvent::DoorZone(false) => {}
            WorldEvent::LeavesAhead(i) => {
                let p = self.world.leaves[i];
                let r = &self.world.rider;
                sound::play_at(ctx, sound::LEAVES, place(r.x - p.x, p.s - r.s));
                let side =
                    self.side_words(p.x - crate::world::LEAF_HALF, p.x + crate::world::LEAF_HALF);
                self.call_queued(
                    ctx,
                    "leaves",
                    format!(
                        "Leaf pile, {side}. Ride through for {} points.",
                        events::LEAF_POINTS
                    ),
                    format!("Leaves, {side}."),
                );
            }
            WorldEvent::Leaves { crunched, .. } => {
                if crunched {
                    sound::play(ctx, sound::CRUNCH);
                    self.muffle = MUFFLE_S;
                    if self.loops.freehub != Sound::NONE {
                        ctx.audio.set_volume(self.loops.freehub, MUFFLED);
                    }
                    self.call_queued(
                        ctx,
                        "crunch",
                        format!("Leaves, {} points.", events::LEAF_POINTS),
                        "Leaves.".to_string(),
                    );
                }
            }
            WorldEvent::OncomingAhead(id) => {
                let r = self.world.rider;
                if let Some(car) = self.world.oncoming.iter().find(|c| c.id == id) {
                    let pos = place(r.x - car.x, car.gap(&r));
                    sound::play_at(ctx, sound::ONCOMING_HORN, pos);
                }
                self.call(
                    ctx,
                    "oncoming",
                    "Horn: oncoming car in the traffic lane. Steer right into the bike lane."
                        .to_string(),
                    "Oncoming car.".to_string(),
                );
            }
            WorldEvent::OncomingNow(_) => {
                sound::play(ctx, sound::ACT_NOW);
                self.call(
                    ctx,
                    "act_now",
                    "Double beep: it is in your line. Act now.".to_string(),
                    "Now.".to_string(),
                );
            }
            WorldEvent::OncomingPassed(_) => {}
            WorldEvent::ParcelAhead(i) => {
                let c = self.world.course.parcels[i];
                let r = &self.world.rider;
                let s = sound::play_at(ctx, sound::PARCEL_CALL, place(r.x - c.x, c.s - r.s));
                ctx.audio.set_volume(s, 0.7);
                let side = self.side_words(c.x - 0.1, c.x + 0.1);
                let word = self.parcel_word();
                // Its own cue, so the endless ride's lesson does not stand
                // for this one.
                let cue = if word == "Drop" { "drop" } else { "parcel" };
                self.call_queued(
                    ctx,
                    cue,
                    format!(
                        "{word}, {side}: that doorbell. Ride over it under {}.",
                        speed_words(crate::world::GRAB_SPEED)
                    ),
                    format!("{word}, {side}."),
                );
            }
            WorldEvent::Parcel(_) if matches!(self.mode, Mode::Clock(_)) => {
                // The time it added is said with the TimeAdded that follows.
                sound::play(ctx, sound::PARCEL_GRAB);
            }
            WorldEvent::Parcel(i) => {
                sound::play(ctx, sound::PARCEL_GRAB);
                let value = self.world.course.parcels[i].value;
                self.call(
                    ctx,
                    "parcel_grab",
                    format!("Parcel grabbed, {}.", dollar_words(value)),
                    format!("{} dollars.", value),
                );
            }
            WorldEvent::ParcelTooFast(_) => {
                sound::play(ctx, sound::NOPE);
                let line = if matches!(self.mode, Mode::Clock(_)) {
                    "Too fast. Drop missed."
                } else {
                    "Too fast to grab."
                };
                self.tell(ctx, line.to_string());
            }
            WorldEvent::OutOfCrashes => {}
            WorldEvent::TimeAdded { secs, quick } => {
                let n = secs.round() as i64;
                let left = self.world.drop_clock.map_or(0.0, |c| c.left).ceil() as i64;
                let line = match (n, quick) {
                    (0, _) => format!("Drop made. The clock is full, {left} seconds."),
                    (n, true) => format!("Plus {n}, quick drop. {left} left."),
                    (n, false) => format!("Plus {n}. {left} left."),
                };
                self.tell(ctx, line);
            }
            WorldEvent::TimeLost(_) => {}
            WorldEvent::TimeCall(n) => {
                self.tell(ctx, format!("{} seconds left.", n as i64));
            }
            WorldEvent::TimeUp => {}
            WorldEvent::Passed(i) => {
                let h = self.world.course.hazards[i];
                let r = self.world.rider;
                let missed = matches!(self.mode, Mode::Training(5))
                    && h.kind == HazardKind::Pothole
                    && !self.lesson.cleared
                    && h.covers(r.x, TYRE_HALF);
                if missed {
                    self.lesson.missed += 1;
                    let line = if self.lesson.missed >= 2 {
                        self.restart_in = Some(1.0);
                        "Both missed. The lesson starts again.".to_string()
                    } else if r.since_hop < 1.0 {
                        "Too early. Hop on the double beep.".to_string()
                    } else {
                        format!("No hop. Press {p} on the double beep.")
                    };
                    // After the pothole's own line, not over it.
                    ctx.say_queued(line.clone());
                    self.last_line = line;
                }
            }
        }
    }

    // -- sound -----------------------------------------------------------

    fn ensure(ctx: &mut Ctx<Parlor>, s: &mut Sound, key: &str, pos: Option<Vec3>) {
        if *s == Sound::NONE {
            *s = ctx.audio.start_loop(key, pos, Bus::EFFECTS);
        } else if pos.is_some() {
            ctx.audio.set_position(*s, pos);
        }
    }

    fn hush(ctx: &mut Ctx<Parlor>, s: &mut Sound) {
        if *s != Sound::NONE {
            ctx.audio.stop(*s, 120);
            *s = Sound::NONE;
        }
    }

    fn stop_all(&mut self, ctx: &mut Ctx<Parlor>) {
        let l = &mut self.loops;
        for s in [
            &mut l.roll,
            &mut l.freehub,
            &mut l.pedal,
            &mut l.wind,
            &mut l.edge,
            &mut l.door_zone,
            &mut l.cobbles,
        ] {
            Self::hush(ctx, s);
        }
        for (_, mut s) in self.cars.drain(..).chain(self.oncoming.drain(..)) {
            Self::hush(ctx, &mut s);
        }
    }

    fn sound_frame(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        // A low clock ticks, faster in its last seconds.
        match self.world.drop_clock.and_then(|c| c.tick_gap()) {
            Some(gap) => {
                self.tick_in -= dt;
                if self.tick_in <= 0.0 {
                    sound::play(ctx, sound::CLOCK_TICK);
                    self.tick_in = gap;
                }
            }
            None => self.tick_in = 0.0,
        }
        let r = self.world.rider;
        let bike = self.world.bike;
        let speed = (r.v / 10.0).min(1.3) as f32;
        let l = &mut self.loops;
        if r.v > 0.3 && !r.airborne() {
            Self::ensure(ctx, &mut l.roll, sound::roll(bike), None);
            ctx.audio.set_volume(l.roll, 0.3 + 0.6 * speed.min(1.0));
            ctx.audio.set_pitch(l.roll, 0.7 + 0.6 * speed);
        } else {
            Self::hush(ctx, &mut l.roll);
        }
        self.muffle = (self.muffle - dt).max(0.0);
        if !r.pedalling && r.v > 0.5 && !r.down() {
            Self::ensure(ctx, &mut l.freehub, sound::freehub(bike), None);
            ctx.audio.set_pitch(l.freehub, 0.5 + speed);
            if self.mode == Mode::LeafSeason {
                let v = if self.muffle > 0.0 { MUFFLED } else { 1.0 };
                ctx.audio.set_volume(l.freehub, v);
            }
        } else {
            Self::hush(ctx, &mut l.freehub);
        }
        if r.pedalling {
            Self::ensure(ctx, &mut l.pedal, sound::pedal(bike), None);
            ctx.audio.set_pitch(l.pedal, 0.8 + 0.4 * speed);
        } else {
            Self::hush(ctx, &mut l.pedal);
        }
        if r.v > 3.0 {
            Self::ensure(ctx, &mut l.wind, sound::WIND, None);
            ctx.audio
                .set_volume(l.wind, 0.6 * speed.min(1.0) * speed.min(1.0));
        } else {
            Self::hush(ctx, &mut l.wind);
        }
        if r.at_edge {
            let right = if r.x < 4.5 { 1.0 } else { -1.0 };
            Self::ensure(ctx, &mut l.edge, sound::EDGE, Some(place(right, 0.5)));
        } else {
            Self::hush(ctx, &mut l.edge);
        }
        if self.world.in_door_zone && r.v > 0.5 {
            Self::ensure(
                ctx,
                &mut l.door_zone,
                sound::DOOR_ZONE,
                Some(place(1.2, 0.0)),
            );
            ctx.audio.set_volume(l.door_zone, 0.3);
        } else {
            Self::hush(ctx, &mut l.door_zone);
        }
        if r.at_edge && !self.was_at_edge {
            let (cue, line) = if r.x < 4.5 {
                ("edge_right", "Edge: parked cars on your right.")
            } else {
                ("edge_left", "Edge: centre line on your left.")
            };
            self.call(ctx, cue, line.to_string(), "Edge.".to_string());
        }
        self.was_at_edge = r.at_edge;

        // Cars, each a loop placed where it is.
        let present: Vec<usize> = self.world.cars.iter().map(|c| c.id).collect();
        let mut keep = Vec::new();
        for (id, mut s) in self.cars.drain(..) {
            if present.contains(&id) {
                keep.push((id, s));
            } else {
                Self::hush(ctx, &mut s);
            }
        }
        self.cars = keep;
        for car in &self.world.cars {
            let ahead = car.s - crate::world::CAR_LEN / 2.0 - r.s;
            let pos = place(r.x - car.x, ahead);
            match self.cars.iter_mut().find(|(id, _)| *id == car.id) {
                Some((_, s)) => ctx.audio.set_position(*s, Some(pos)),
                None => {
                    let s = ctx.audio.start_loop(sound::CAR, Some(pos), Bus::EFFECTS);
                    self.cars.push((car.id, s));
                }
            }
            if let Some((_, s)) = self.cars.iter().find(|(id, _)| *id == car.id) {
                ctx.audio
                    .set_volume(*s, sound::distance_volume(ahead) * 0.8);
            }
        }

        // Oncoming cars, each a loop ahead in the traffic lane.
        let present: Vec<usize> = self.world.oncoming.iter().map(|c| c.id).collect();
        let mut keep = Vec::new();
        for (id, mut s) in self.oncoming.drain(..) {
            if present.contains(&id) {
                keep.push((id, s));
            } else {
                Self::hush(ctx, &mut s);
            }
        }
        self.oncoming = keep;
        for car in &self.world.oncoming {
            let ahead = car.s + crate::world::CAR_LEN / 2.0 - r.s;
            let pos = place(r.x - car.x, ahead);
            match self.oncoming.iter_mut().find(|(id, _)| *id == car.id) {
                Some((_, s)) => ctx.audio.set_position(*s, Some(pos)),
                None => {
                    let s = ctx
                        .audio
                        .start_loop(sound::ONCOMING, Some(pos), Bus::EFFECTS);
                    self.oncoming.push((car.id, s));
                }
            }
            if let Some((_, s)) = self.oncoming.iter().find(|(id, _)| *id == car.id) {
                ctx.audio
                    .set_volume(*s, sound::distance_volume(ahead) * 0.8);
            }
        }

        // Cobbles rattle under the wheels in Old Town.
        let l = &mut self.loops;
        if self.world.course.surface == crate::bike::Surface::Cobbles && r.v > 0.3 && !r.airborne()
        {
            Self::ensure(ctx, &mut l.cobbles, sound::COBBLES, None);
            ctx.audio.set_volume(l.cobbles, 0.25 + 0.5 * speed.min(1.0));
            ctx.audio.set_pitch(l.cobbles, 0.8 + 0.4 * speed);
        } else {
            Self::hush(ctx, &mut l.cobbles);
        }

        // Hazard markers, ticking faster as they near.
        for i in 0..self.ticks.len() {
            let state = self.world.hazards[i];
            if !state.warned || state.passed {
                continue;
            }
            self.ticks[i] -= dt;
            if self.ticks[i] > 0.0 {
                continue;
            }
            let c = self.world.predict_contact(i);
            let secs = c.secs.unwrap_or(10.0);
            self.ticks[i] = 1.0 / sound::tick_rate(secs);
            let kind = self.world.course.hazards[i].kind;
            let pos = self.hazard_pos(i);
            let s = sound::play_at(ctx, sound::marker(kind), pos);
            ctx.audio.set_pitch(s, sound::tick_pitch(c.metres));
            ctx.audio.set_volume(s, sound::distance_volume(c.metres));
        }

        // The finish bell rings on until you cross the line.
        if let Some(t) = &mut self.beacon_in {
            *t -= dt;
            if *t <= 0.0 {
                *t = 1.5;
                let d = self.world.course.length - r.s;
                let s = sound::play_at(ctx, sound::BEACON, place(0.0, d));
                ctx.audio.set_volume(s, sound::distance_volume(d));
            }
        }
    }

    // -- training ----------------------------------------------------------

    fn lesson_step(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Mode::Training(n) = self.mode else {
            return;
        };
        let r = self.world.rider;
        let down = Self::key(ctx, ParlorAction::Down);
        let right = Self::key(ctx, ParlorAction::Right);
        let passed = match n {
            1 => {
                if !self.lesson.fast && r.v >= 5.5 {
                    self.lesson.fast = true;
                    self.tell(
                        ctx,
                        format!(
                            "{}. Now let go and coast: listen to the freewheel tick.",
                            units::per_hour(20.0)
                        ),
                    );
                }
                if self.lesson.fast && !r.pedalling && r.v > 1.0 {
                    self.lesson.coasted += dt;
                }
                self.lesson.coasted >= 2.0
            }
            2 => {
                if !self.lesson.fast && r.v >= training::BRAKE_FROM {
                    self.lesson.fast = true;
                    self.tell(ctx, format!("Now hold {down} to brake to a stop."));
                }
                if !self.lesson.fast {
                    if r.v > 1.0 {
                        self.lesson.rolling = true;
                    } else if self.lesson.rolling && r.v == 0.0 {
                        // Stopped short of the lesson's speed: say so.
                        self.lesson.rolling = false;
                        self.tell(ctx, "Faster first.".to_string());
                    }
                }
                self.lesson.fast && r.v == 0.0
            }
            3 => {
                if !self.lesson.out && r.x >= 4.3 {
                    self.lesson.out = true;
                    self.tell(
                        ctx,
                        format!("In the traffic lane. Now hold {right} to steer back into the bike lane."),
                    );
                }
                self.lesson.out && r.x <= 3.2
            }
            4 => r.s >= training::DOORS_DONE && self.world.stats.doors == 0 && !r.down(),
            _ => self.lesson.cleared,
        };
        if passed {
            self.finished = true;
            self.stop_all(ctx);
            sound::play(ctx, sound::LESSON);
            let mut progress = Progress::load(ctx);
            progress.lessons = progress.lessons.max(n);
            progress.store(ctx);
            if self.inside {
                self.passed = true;
                return;
            }
            let bike = self.world.bike;
            if n < training::LESSONS {
                ctx.say(format!("Lesson {n} passed."));
                let page = training::page(ctx, n + 1);
                ctx.replace(
                    Pages::briefing([page])
                        .then(move |ctx| ctx.replace(Ride::training(n + 1, bike))),
                );
            } else {
                progress.rides += 1;
                progress.store(ctx);
                let pages = training::done_pages(ctx);
                let results =
                    Results::new("Training complete", pages.clone(), Back::Title(TITLE_NAME))
                        .next(
                            format!("Next: time trial, {}", COURSES[0].name),
                            COURSES[0].about(),
                            move |ctx| ctx.replace(Ride::time_trial(0, bike)),
                        )
                        .again(
                            labels::PLAY_AGAIN,
                            "All five lessons again, from the first.",
                            move |ctx| ctx.replace(training::start(ctx, bike)),
                        );
                let mut results = Some(results);
                ctx.replace(Pages::new(pages).then(move |ctx| {
                    if let Some(r) = results.take() {
                        ctx.replace(r);
                    }
                }));
            }
        }
    }

    // -- the end -----------------------------------------------------------

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.finished = true;
        self.stop_all(ctx);
        sound::play(ctx, sound::FINISH);
        let w = &self.world;
        let time = w.finished.unwrap_or(w.clock);
        let stats = w.stats;
        let bike = w.bike;
        if let Mode::Training(n) = self.mode {
            self.tell(
                ctx,
                "End of the street. The lesson starts again.".to_string(),
            );
            if self.inside {
                self.start_over(ctx, n);
            } else {
                ctx.replace(Ride::training(n, bike));
            }
            return;
        }
        let mut progress = Progress::load(ctx);
        progress.rides += 1;
        progress.metres += w.course.length;
        progress.crashes += stats.crashes;
        progress.best_air = progress.best_air.max(self.best_air);
        progress.store(ctx);
        ctx.audio.set_bed(None);
        ctx.audio.play_music(Some(sound::RESULT_MUSIC), false);
        if let Mode::Endless(district) = self.mode {
            // The distance ridden, not the course, counts toward the total.
            progress.metres += w.rider.s - w.course.length;
            progress.store(ctx);
            let metres = w.rider.s;
            let cash = stats.cash;
            let grabbed = w.parcels.iter().filter(|p| p.taken).count();
            let wallet = self.bank(ctx);
            let board = progress::endless_board(district);
            let result = parlor_core::Records::record(ctx, crate::ID, &board, metres as u64);
            let record = match result {
                ScoreResult::First => "Your first endless ride here.".to_string(),
                ScoreResult::NewBest { previous } => {
                    format!("A new best, beating {}.", distance_words(previous as f64))
                }
                ScoreResult::Short { best } => {
                    format!("Your best is {}.", distance_words(best as f64))
                }
            };
            let pages = vec![
                format!(
                    "Ride over. {} in {}. {record}",
                    sentence_start(&distance_words(metres)),
                    district.name()
                ),
                format!(
                    "{} grabbed, {}. You have {} to spend in the bike shop.",
                    parcel_words(grabbed),
                    dollar_words(cash),
                    dollar_words(wallet)
                ),
            ];
            let summary = format!(
                "Endless ride: {}, {}",
                distance_words(metres),
                dollar_words(cash)
            );
            let results = Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
                labels::PLAY_AGAIN,
                "A new endless ride in the same district, on the same bike.",
                move |ctx| ctx.replace(Ride::endless(bike, district, Rng::from_time().next_u64())),
            );
            self.show(ctx, pages, results);
            return;
        }
        if let Mode::Clock(district) = self.mode {
            progress.metres += w.rider.s - w.course.length;
            progress.store(ctx);
            let c = w.drop_clock.unwrap_or_default();
            let cash = stats.cash;
            let wallet = self.bank(ctx);
            let result = parlor_core::Records::record(
                ctx,
                crate::ID,
                &clock::board(district),
                c.drops as u64,
            );
            let record = match result {
                ScoreResult::First => {
                    format!("Your first ride {} here.", clock::NAME.to_lowercase())
                }
                ScoreResult::NewBest { previous: 0 } => "A new best.".to_string(),
                ScoreResult::NewBest { previous } => {
                    format!("A new best, beating {previous}.")
                }
                ScoreResult::Short { best } => format!("Your best is {best}."),
            };
            let pages = vec![
                format!(
                    "Time's up. {} in {}, in {}. {record}",
                    sentence_start(&clock::drop_words(c.drops)),
                    district.name(),
                    span_words(time)
                ),
                format!(
                    "{} at lockers, {} quick. {} {} earned. You have {} to spend in the bike shop.",
                    c.lockers,
                    c.quick,
                    if c.lost > 0.0 {
                        format!("{} lost to crashes.", sentence_start(&secs_words(c.lost)))
                    } else {
                        "No crashes.".to_string()
                    },
                    sentence_start(&dollar_words(cash)),
                    dollar_words(wallet)
                ),
            ];
            let summary = format!(
                "{}: {}, {}",
                clock::NAME,
                clock::drop_words(c.drops),
                dollar_words(cash)
            );
            let results = Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
                labels::PLAY_AGAIN,
                "A new street in the same district, on the same bike.",
                move |ctx| {
                    ctx.replace(Ride::against_clock(
                        bike,
                        district,
                        Rng::from_time().next_u64(),
                    ))
                },
            );
            self.show(ctx, pages, results);
            return;
        }
        if let (Mode::Delivery(_), Some(job)) = (self.mode, self.job.clone()) {
            let paid = job.paid(time);
            let mut progress = Progress::load(ctx);
            progress.deliveries += 1;
            progress.bank(paid);
            progress.store(ctx);
            let deadline = job.deadline();
            let when = if time <= deadline {
                "on time".to_string()
            } else {
                format!("{} late. Half pay", time_words(time - deadline))
            };
            let pages = vec![format!(
                "Delivered to {} in {}, {when}. {} earned. You have {}.",
                job.to,
                time_words(time),
                sentence_start(&dollar_words(paid)),
                dollar_words(progress.cash)
            )];
            let summary = format!("Delivered, {}", dollar_words(paid));
            let results = Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).next(
                "Job board",
                "Pick your next delivery.",
                |ctx| ctx.replace(crate::jobs::JobBoard::new()),
            );
            self.show(ctx, pages, results);
            return;
        }
        let (points, breakdown) = ride_points(self.par, time, &stats);
        match self.mode {
            Mode::TimeTrial(c) => {
                let info = &COURSES[c];
                let board = progress::board(info.id, bike);
                let saved = progress::save_time(ctx, &board, time, points);
                let pages = trial_pages(info, bike, time, points, &breakdown, &saved);
                let summary = format!(
                    "{} in {}. {points} ride points",
                    info.name,
                    time_words(time)
                );
                let results = after_trial(c, bike, summary, pages.clone());
                self.show(ctx, pages, results);
            }
            Mode::FreeRide => {
                let pages = vec![format!(
                    "Free ride done: {} in {}. {}",
                    units::route(FREE_LENGTH),
                    time_words(time),
                    crash_words(stats.crashes)
                )];
                let summary = format!("Free ride done in {}", time_words(time));
                let results = Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
                    labels::PLAY_AGAIN,
                    format!(
                        "A new stretch of Downtown, {}, no clock.",
                        units::route(FREE_LENGTH)
                    ),
                    move |ctx| ctx.replace(Ride::free_ride(bike, Rng::from_time().next_u64())),
                );
                self.show(ctx, pages, results);
            }
            Mode::Challenge(period) => {
                let Some(ch) = self.challenge.clone() else {
                    return;
                };
                let result = Challenges::submit(ctx, &ch, points);
                let streak = Challenges::streak(ctx, crate::ID, period).max(1);
                let pages = vec![
                    format!(
                        "{} done in {}. {points} ride points. {}",
                        challenge::name(period),
                        time_words(time),
                        score_words(result)
                    ),
                    format!("Ride points: {}.", breakdown.join(", ")),
                    format!("A {streak} {} streak. {}", period.unit(), ch.resets()),
                ];
                let summary = format!("{} result: {points} ride points", ch.title());
                let results =
                    Results::new(summary, pages.clone(), Back::Challenges(CHALLENGES_NAME)).again(
                        labels::challenge_again(period),
                        "The same course straight away. Only your best result is kept.",
                        move |ctx| {
                            let ch = Challenge::current(ctx, crate::ID, period);
                            ctx.replace(Ride::challenge(ch))
                        },
                    );
                self.show(ctx, pages, results);
            }
            Mode::LeafSeason | Mode::Sunday => {
                let Some((name, stem)) = self.mode.event() else {
                    return;
                };
                let board = events::board(stem, bike);
                let saved = progress::save_time(ctx, &board, time, points);
                let record = match saved.previous {
                    None => format!("Your first {name}."),
                    Some(p) if saved.record => {
                        format!("A new best time, {} faster.", time_words(p - time))
                    }
                    Some(p) => format!(
                        "{} slower than your best of {}.",
                        sentence_start(&time_words(time - p)),
                        time_words(p)
                    ),
                };
                let mut first = format!(
                    "{name} done on the {} in {}. {record}",
                    bike.name(),
                    time_words(time)
                );
                if self.mode == Mode::LeafSeason {
                    first.push_str(&format!(
                        " Leaf piles: {} of {}.",
                        stats.leaves,
                        w.leaves.len()
                    ));
                }
                let pages = vec![
                    first,
                    format!(
                        "{points} ride points: {}. {}",
                        breakdown.join(", "),
                        score_words(saved.points)
                    ),
                ];
                let summary = format!("{name} in {}. {points} ride points", time_words(time));
                let mode = self.mode;
                let results = Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
                    labels::PLAY_AGAIN,
                    "The same course again, to beat your time.",
                    move |ctx| {
                        // An event ride starts again only while it is open.
                        let today = parlor_core::challenges::today(ctx);
                        let ride = match mode {
                            Mode::LeafSeason if events::leaf_open(today) => {
                                Some(Ride::leaf_season(bike, today))
                            }
                            Mode::Sunday => {
                                events::sunday_course(today).map(|c| Ride::sunday(bike, c))
                            }
                            _ => None,
                        };
                        match ride {
                            Some(ride) => ctx.replace(ride),
                            None => {
                                ctx.audio.play(sound::NOPE, Bus::UI);
                                ctx.say(events::RIDE_CLOSED);
                            }
                        }
                    },
                );
                self.show(ctx, pages, results);
            }
            Mode::Training(_) | Mode::Endless(_) | Mode::Delivery(_) | Mode::Clock(_) => {}
        }
    }

    /// Puts an endless ride's parcel money in the bank, once, and returns
    /// the dollars there now.
    fn bank(&mut self, ctx: &mut Ctx<Parlor>) -> u32 {
        let mut progress = Progress::load(ctx);
        if matches!(self.mode, Mode::Endless(_) | Mode::Clock(_)) && !self.banked {
            self.banked = true;
            progress.bank(self.world.stats.cash);
            progress.store(ctx);
        }
        progress.cash
    }

    fn show(&mut self, ctx: &mut Ctx<Parlor>, pages: Vec<String>, results: Results) {
        let mut results = Some(results);
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(r) = results.take() {
                ctx.replace(r);
            }
        }));
    }

    fn restart_lesson(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Mode::Training(n) = self.mode {
            if self.inside {
                self.start_over(ctx, n);
                return;
            }
            self.finished = true;
            self.stop_all(ctx);
            ctx.replace(Ride::training(n, self.world.bike));
        }
    }

    fn input(&self, ctx: &Ctx<Parlor>) -> Input {
        let held = |a| ctx.action_held(a);
        let mut steer = 0.0;
        if held(ParlorAction::Left) {
            steer += 1.0;
        }
        if held(ParlorAction::Right) {
            steer -= 1.0;
        }
        Input {
            pedal: held(ParlorAction::Up),
            brake: held(ParlorAction::Down),
            steer,
            hop: self.hop,
            sprint: held(ParlorAction::Secondary),
        }
    }

    /// A key that cannot act yet, while the countdown runs or the rider is down.
    fn not_yet(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        if !self.rolling() {
            ctx.say(format!(
                "Wait for go: {}.",
                secs_words(self.countdown.ceil())
            ));
            return true;
        }
        if self.world.rider.down() {
            ctx.say(format!(
                "You are down. Back up in {}.",
                secs_words(self.world.rider.down_for)
            ));
            return true;
        }
        false
    }
}

/// Ride points: 100,000 at par time, more for faster, with bonuses and a
/// cost for each crash. Also the parts, in words.
pub fn ride_points(par: f64, time: f64, stats: &crate::world::Stats) -> (u64, Vec<String>) {
    let base = (100_000.0 * par / time.max(1.0)).round() as i64;
    let mut parts = vec![format!("{base} for speed")];
    let mut total = base;
    if stats.crashes == 0 {
        total += 5_000;
        parts.push("5000 for a clean ride".to_string());
    }
    let air = (stats.airtime * 1000.0).round() as i64;
    if air > 0 {
        total += air;
        parts.push(format!("{air} for airtime"));
    }
    if stats.tips > 0 {
        let tips = stats.tips as i64 * 1000;
        total += tips;
        parts.push(format!("{tips} in tips"));
    }
    if stats.leaves > 0 {
        let leaves = stats.leaves as i64 * events::LEAF_POINTS as i64;
        total += leaves;
        parts.push(format!("{leaves} for leaf piles"));
    }
    if stats.crashes > 0 {
        let cost = stats.crashes as i64 * 3000;
        total -= cost;
        parts.push(format!(
            "{cost} lost for {}",
            crash_words(stats.crashes)
                .trim_end_matches('.')
                .to_lowercase()
        ));
    }
    (total.max(0) as u64, parts)
}

fn parcel_words(n: usize) -> String {
    match n {
        0 => "No parcels".to_string(),
        1 => "1 parcel".to_string(),
        n => format!("{n} parcels"),
    }
}

fn crash_words(n: u32) -> String {
    match n {
        0 => "No crashes.".to_string(),
        1 => "1 crash.".to_string(),
        n => format!("{n} crashes."),
    }
}

fn score_words(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result here.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// The pages after a time trial: the time and what it did to the record,
/// then the ride points.
pub fn trial_pages(
    info: &CourseInfo,
    bike: Bike,
    time: f64,
    points: u64,
    breakdown: &[String],
    saved: &Saved,
) -> Vec<String> {
    let record = match saved.previous {
        None => format!("Your first time on {} on the {}.", info.name, bike.name()),
        Some(p) if saved.record => format!("A new best time, {} faster.", time_words(p - time)),
        Some(p) => format!(
            "{} slower than your best of {}.",
            sentence_start(&time_words(time - p)),
            time_words(p)
        ),
    };
    vec![
        format!(
            "Finished {} on the {} in {}. {record}",
            info.name,
            bike.name(),
            time_words(time)
        ),
        format!(
            "{points} ride points: {}. {}",
            breakdown.join(", "),
            score_words(saved.points)
        ),
    ]
}

/// The results menu after a time trial: the next course, this one again,
/// the results again, and back to the title.
pub fn after_trial(course: usize, bike: Bike, summary: String, pages: Vec<String>) -> Results {
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if course + 1 < COURSES.len() {
        let next = &COURSES[course + 1];
        results = results.next(
            format!("Next course: {}", next.name),
            next.about(),
            move |ctx| {
                let mut choices = Choices::load(ctx);
                choices.course = course + 1;
                choices.store(ctx);
                ctx.replace(Ride::time_trial(course + 1, bike))
            },
        );
    }
    results.again(
        labels::PLAY_AGAIN,
        "The same course and bike, with the same traffic, to beat your time.",
        move |ctx| ctx.replace(Ride::time_trial(course, bike)),
    )
}

impl State<Parlor> for Ride {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        RIDE_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.started = true;
        self.call_outs = Choices::load(ctx).call_outs;
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        if self.mode == Mode::Sunday {
            sound::sunday_bed(ctx);
        } else if let Mode::Endless(d) | Mode::Delivery(d) | Mode::Clock(d) = self.mode {
            sound::district_bed(ctx, d);
        } else {
            sound::city_bed(ctx);
        }
        let line = self.start_line(ctx);
        if self.rolling() {
            self.tell(ctx, line);
        } else {
            ctx.say(line.clone());
            self.last_line = line;
            sound::play(ctx, sound::COUNT);
            ctx.say_queued("3.");
            self.count_said = 3;
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        // Parcel money from an endless ride, or one against the clock, is
        // kept, even when it is left early.
        self.bank(ctx);
        self.stop_all(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.finished {
            return;
        }
        // Settings, from the pause menu, may have changed the call-outs.
        self.call_outs = Choices::load(ctx).call_outs;
        let status = self.status_layers().remove(0);
        ctx.say(format!("Riding. {status}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.finished {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if !event.is_repeat() {
                let leave = self.leave();
                menus::pause(ctx, HELP, leave);
            }
            return;
        }
        if event.is_repeat() {
            return;
        }
        let Some(action) = action else {
            return;
        };
        let up = Self::key(ctx, ParlorAction::Up);
        match action {
            ParlorAction::Status => {
                let layers = self.status_layers();
                self.status_key.speak(ctx, layers);
            }
            ParlorAction::Repeat => {
                let line = if self.last_line.is_empty() {
                    self.start_line(ctx)
                } else {
                    self.last_line.clone()
                };
                ctx.say(line);
            }
            ParlorAction::Hint => {
                let h = self.hint(ctx);
                ctx.say(h.clone());
                self.last_line = h;
            }
            _ if self.not_yet(ctx) => {}
            ParlorAction::Up => {
                let bike = self.world.bike;
                Self::ensure(ctx, &mut self.loops.pedal, sound::pedal(bike), None);
            }
            ParlorAction::Down => {
                if self.world.rider.v > 0.5 {
                    sound::play(ctx, sound::BRAKE);
                } else {
                    let go = self.go_words(ctx);
                    ctx.say(format!("Stopped. {go}"));
                }
            }
            ParlorAction::Left | ParlorAction::Right => {
                if self.world.rider.v >= MIN_MOVING {
                    let side = if action == ParlorAction::Left {
                        -0.8
                    } else {
                        0.8
                    };
                    sound::play_at(ctx, sound::STEER, place(side, 0.5));
                } else {
                    let go = self.go_words(ctx);
                    ctx.say(format!("Too slow to steer. {go}"));
                }
            }
            ParlorAction::Primary => {
                let r = &self.world.rider;
                if self.world.bike.spec().hop_m == 0.0 {
                    sound::play(ctx, sound::NOPE);
                    ctx.say("The moped cannot hop.");
                } else if r.airborne() {
                    sound::play(ctx, sound::NOPE);
                    ctx.say("Already in the air.");
                } else if r.v < MIN_MOVING {
                    sound::play(ctx, sound::NOPE);
                    ctx.say("Too slow to hop.");
                } else {
                    // The hop sounds from the ride's next substep.
                    self.hop = true;
                }
            }
            ParlorAction::Secondary => {
                if self.world.bike.motor() {
                    sound::play(ctx, sound::NOPE);
                    ctx.say("No sprint on the moped.");
                } else if self.world.rider.stamina > 0.05 {
                    sound::play(ctx, sound::SPRINT);
                    self.call(
                        ctx,
                        "sprint",
                        format!("Sprint. Hold it with {up} until you run out of breath."),
                        "Sprint.".to_string(),
                    );
                } else {
                    sound::play(ctx, sound::SPENT);
                    ctx.say("Out of breath.");
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if !self.started || self.finished {
            return;
        }
        if let Some(t) = &mut self.restart_in {
            *t -= dt;
            if *t <= 0.0 {
                self.restart_lesson(ctx);
            }
            return;
        }
        if !self.rolling() {
            self.countdown -= dt;
            let n = self.countdown.ceil() as i32;
            if self.countdown <= 0.0 {
                self.countdown = 0.0;
                sound::play(ctx, sound::GO);
                let go = self.go_words(ctx);
                self.tell(ctx, format!("Go. {go}"));
            } else if n < self.count_said {
                self.count_said = n;
                sound::play(ctx, sound::COUNT);
                ctx.say(format!("{n}."));
            }
            return;
        }
        let input = self.input(ctx);
        // A hop press reaches the world's first substep, then is spent.
        let before = self.world.clock;
        let events = self.world.update(input, dt);
        if self.world.clock > before {
            self.hop = false;
        }
        for ev in events {
            self.on_event(ctx, ev);
            if self.finished {
                return;
            }
        }
        if self.finished {
            return;
        }
        self.sound_frame(ctx, dt);
        self.lesson_step(ctx, dt);
        if self.finished {
            return;
        }
        if self.world.finished.is_some() {
            self.finish(ctx);
        }
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| ctx.control_name(a);
        format!(
            "{} pedals and {} brakes. {} and {} steer. {} hops, {} sprints. {} says where you are, {} what to do next. Escape pauses.",
            sentence_start(&k(ParlorAction::Up)),
            k(ParlorAction::Down),
            sentence_start(&k(ParlorAction::Left)),
            k(ParlorAction::Right),
            sentence_start(&k(ParlorAction::Primary)),
            k(ParlorAction::Secondary),
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Hint)
        )
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_ride;
