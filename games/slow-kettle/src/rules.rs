//! The rules as plain structs: the kettle, the steep, how a cup is judged,
//! and who comes in each afternoon. No sound or speech here, so tests can
//! drive it all by `dt`.

use std::cell::Cell;
use std::collections::BTreeMap;

use parlor_core::sim::FixedStep;
use parlor_core::{Date, Period, Rng};

use crate::days::{self, coming, occasion_on, Occasion};
use crate::guests::{
    regulars_met, Grade, Regular, Taster, WalkIn, NOTES, REGULARS, TASTERS, WALK_INS,
};
use crate::teas::{Cup, Style, Taste, Tea, CUPS, TEAS};

/// Room temperature: where a fresh kettle starts, in degrees.
pub const ROOM: f64 = 20.0;
/// The boil.
pub const BOIL: f64 = 100.0;
/// How fast an unheated kettle cools, in degrees a second.
pub const COOL: f64 = 0.5;
/// Substeps a second for the kettle and the steep.
pub const STEP_HZ: f64 = 60.0;
/// How long a guest takes to come in, from the door bell to their words.
pub const ARRIVE_S: f64 = 2.0;
/// The longest steep: after this the leaves come out on their own.
pub const MAX_STEEP: f64 = 40.0;
/// Within this many degrees, the water is just right.
pub const HEAT_SLACK: f64 = 3.0;
/// Within this many seconds, the steep is just right.
pub const STEEP_SLACK: f64 = 1.0;

/// Every cup pays this, however it went.
pub const PAY: u32 = 2;
/// A lovely cup pays one more.
pub const LOVELY_EXTRA: u32 = 1;
/// The tip a perfect cup leaves.
pub const TIP: u32 = 3;

/// The kettle's song, by how hot it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Quiet,
    Rumble,
    Bubbles,
    Hiss,
    Rolling,
    Boil,
}

impl Stage {
    pub fn of(temp: f64) -> Stage {
        match temp {
            t if t >= BOIL - 0.05 => Stage::Boil,
            t if t >= 92.0 => Stage::Rolling,
            t if t >= 80.0 => Stage::Hiss,
            t if t >= 65.0 => Stage::Bubbles,
            t if t >= 40.0 => Stage::Rumble,
            _ => Stage::Quiet,
        }
    }

    /// "a soft hiss", as S and the call-outs say it.
    pub fn words(self) -> &'static str {
        match self {
            Stage::Quiet => "quiet",
            Stage::Rumble => "a low rumble",
            Stage::Bubbles => "small bubbles",
            Stage::Hiss => "a soft hiss",
            Stage::Rolling => "a rolling hiss",
            Stage::Boil => "the boil, with a gentle whistle",
        }
    }

    /// The call-out as the song reaches this stage: "Soft hiss, 80 degrees."
    pub fn call_out(self) -> &'static str {
        match self {
            Stage::Quiet => "Quiet.",
            Stage::Rumble => "Low rumble, about 40 degrees.",
            Stage::Bubbles => "Small bubbles, about 65 degrees.",
            Stage::Hiss => "Soft hiss, about 80 degrees.",
            Stage::Rolling => "Rolling hiss, about 92 degrees.",
            Stage::Boil => "The boil. The kettle whistles.",
        }
    }
}

/// The kettle: it heats while it is on and cools slowly when it is off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kettle {
    pub temp: f64,
    pub on: bool,
}

impl Default for Kettle {
    fn default() -> Self {
        Kettle {
            temp: ROOM,
            on: false,
        }
    }
}

impl Kettle {
    /// One substep. Heating is quick from cold and slows near the boil:
    /// from room temperature to the boil takes about 13 seconds.
    pub fn step(&mut self, dt: f64) {
        if self.on {
            let left = ((BOIL - self.temp) / (BOIL - ROOM)).clamp(0.0, 1.0);
            let rate = 4.0 + 5.0 * left;
            self.temp = (self.temp + rate * dt).min(BOIL);
        } else {
            self.temp = (self.temp - COOL * dt).max(ROOM);
        }
    }

    pub fn degrees(&self) -> u32 {
        self.temp.round() as u32
    }

    pub fn stage(&self) -> Stage {
        Stage::of(self.temp)
    }
}

/// How the steep tastes so far, by how long it has had against what the
/// tea wants: "still pale", "rich and ready".
pub fn strength_words(steeped: f64, ideal: f64) -> &'static str {
    let r = steeped / ideal.max(1.0);
    match r {
        r if r < 0.5 => "still pale",
        r if r < 0.85 => "taking colour",
        r if r <= 1.15 => "rich and ready",
        r if r < 1.6 => "getting strong",
        _ => "very strong",
    }
}

/// What went into a cup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brewed {
    pub tea: &'static Tea,
    pub cup: &'static Cup,
    /// The water, in degrees, as it was poured.
    pub heat: f64,
    /// The steep, in seconds.
    pub steep: f64,
    /// The taste asked for, or `None` when the guest named the tea.
    pub taste: Option<Taste>,
    /// The tea asked for by name, if any.
    pub wants: Option<&'static Tea>,
}

/// How a cup went, and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Verdict {
    /// Out of 100.
    pub points: u32,
    pub grade: Grade,
    /// Degrees over (positive) or under the tea's heat.
    pub heat_off: f64,
    /// Seconds over (positive) or under the tea's steep.
    pub steep_off: f64,
    /// The tea answered the taste asked for.
    pub matched: bool,
    /// The cup suited the taste too.
    pub cup_suits: bool,
}

/// What a cup lost points for most, if anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slip {
    WrongTaste,
    TooHot,
    TooCool,
    TooStrong,
    TooPale,
}

impl Slip {
    /// What the guest says about it.
    pub fn words(self) -> &'static str {
        match self {
            Slip::WrongTaste => "Not quite the cup I had in mind.",
            Slip::TooHot => "The water was a little hot for this tea.",
            Slip::TooCool => "It wanted hotter water.",
            Slip::TooStrong => "A little strong.",
            Slip::TooPale => "A little pale.",
        }
    }
}

fn heat_points(off: f64) -> f64 {
    let off = off.abs();
    if off <= HEAT_SLACK {
        30.0
    } else {
        (30.0 * (1.0 - (off - HEAT_SLACK) / 22.0)).max(0.0)
    }
}

fn steep_points(off: f64, ideal: f64) -> f64 {
    let off = off.abs();
    if off <= STEEP_SLACK {
        30.0
    } else {
        (30.0 * (1.0 - (off - STEEP_SLACK) / (ideal * 0.8).max(4.0))).max(0.0)
    }
}

/// Judges a cup. Tea that suits the taste is worth 35, water and steep 30
/// each, a cup that suits the taste 5. A perfect cup needs 90.
pub fn judge(b: &Brewed) -> Verdict {
    let heat_off = b.heat - b.tea.heat as f64;
    let steep_off = b.steep - b.tea.steep as f64;
    let matched = match (b.wants, b.taste) {
        (Some(w), _) => w.id == b.tea.id,
        (None, Some(t)) => b.tea.suits(t),
        (None, None) => true,
    };
    let cup_suits = b.taste.is_some_and(|t| b.cup.suits(t));
    let points = if matched { 35.0 } else { 15.0 }
        + heat_points(heat_off)
        + steep_points(steep_off, b.tea.steep as f64)
        + if cup_suits { 5.0 } else { 0.0 };
    let points = (points.round() as u32).min(100);
    let grade = match points {
        90.. => Grade::Perfect,
        72.. => Grade::Lovely,
        50.. => Grade::Pleasant,
        _ => Grade::Gentle,
    };
    Verdict {
        points,
        grade,
        heat_off,
        steep_off,
        matched,
        cup_suits,
    }
}

impl Verdict {
    /// The thing this cup lost most for, when it was not perfect.
    pub fn slip(&self, tea: &Tea) -> Option<Slip> {
        if self.grade == Grade::Perfect {
            return None;
        }
        let taste: f64 = if self.matched { 0.0 } else { 20.0 };
        let heat = 30.0 - heat_points(self.heat_off);
        let steep = 30.0 - steep_points(self.steep_off, tea.steep as f64);
        let worst = taste.max(heat).max(steep);
        if worst <= 0.0 {
            None
        } else if worst == taste {
            Some(Slip::WrongTaste)
        } else if worst == heat {
            Some(if self.heat_off > 0.0 {
                Slip::TooHot
            } else {
                Slip::TooCool
            })
        } else {
            Some(if self.steep_off > 0.0 {
                Slip::TooStrong
            } else {
                Slip::TooPale
            })
        }
    }

    /// Coins the cup pays, and the tip on top.
    pub fn coins(&self) -> (u32, u32) {
        match self.grade {
            Grade::Perfect => (PAY, TIP),
            Grade::Lovely => (PAY + LOVELY_EXTRA, 0),
            _ => (PAY, 0),
        }
    }
}

/// Where a cup is: from the guest coming in to the cup set down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The door bell and footsteps; the guest speaks when it ends.
    Arriving,
    /// Up and Down along the shelf, Left and Right through the cups.
    Choosing,
    /// Tea chosen, kettle cold: the main action puts it on.
    Ready,
    /// The kettle is singing: the main action pours.
    Heating,
    /// The leaves are in: the main action lifts them out.
    Steeping,
    /// A pot for several: the main action pours each cup.
    Pouring,
    /// The guest has their cup: the main action welcomes the next guest.
    Served,
}

/// Something the rules want heard, from `Service::update`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// The guest is in and speaks.
    Arrived,
    /// The kettle's song reached a new stage.
    Stage(Stage),
    /// A second of steeping has passed: the clock ticks.
    Tick(u32),
    /// The sand timer ran out: the tea is at its steep.
    Timer,
    /// The steep ran so long the leaves came out on their own.
    LongSteep,
}

/// One cup of tea being made, by `dt`.
#[derive(Clone, Debug, PartialEq)]
pub struct Service {
    pub phase: Phase,
    pub kettle: Kettle,
    pub tea: Option<&'static Tea>,
    /// The water's heat when it was poured.
    pub poured_at: f64,
    /// Seconds of steeping.
    pub steeped: f64,
    /// Cups this pot pours, and how many are poured.
    pub cups: u8,
    pub poured: u8,
    /// Whether the sand timer rings at the tea's steep.
    pub sand_timer: bool,
    wait: f64,
    clock: FixedStep,
    stage: Stage,
    ticks: u32,
    timer_rang: bool,
}

impl Service {
    pub fn new(sand_timer: bool) -> Service {
        Service {
            phase: Phase::Served,
            kettle: Kettle::default(),
            tea: None,
            poured_at: ROOM,
            steeped: 0.0,
            cups: 1,
            poured: 0,
            sand_timer,
            wait: 0.0,
            clock: FixedStep::hz(STEP_HZ),
            stage: Stage::Quiet,
            ticks: 0,
            timer_rang: false,
        }
    }

    /// A new guest comes in, with a fresh kettle.
    pub fn arrive(&mut self, cups: u8) {
        self.phase = Phase::Arriving;
        self.wait = ARRIVE_S;
        self.kettle = Kettle::default();
        self.stage = Stage::Quiet;
        self.tea = None;
        self.cups = cups.max(1);
        self.poured = 0;
        self.steeped = 0.0;
        self.ticks = 0;
        self.timer_rang = false;
    }

    /// Skips the rest of the guest's walk in. True if it was still going.
    pub fn skip_arrival(&mut self) -> bool {
        if self.phase == Phase::Arriving {
            self.phase = Phase::Choosing;
            self.wait = 0.0;
            true
        } else {
            false
        }
    }

    /// Straight to the shelf, as practice starts.
    pub fn to_shelf(&mut self) {
        self.phase = Phase::Choosing;
        self.kettle.on = false;
    }

    /// The tea goes in the pot.
    pub fn choose(&mut self, tea: &'static Tea) {
        self.tea = Some(tea);
        self.phase = Phase::Ready;
    }

    pub fn heat(&mut self) {
        self.kettle.on = true;
        self.stage = self.kettle.stage();
        self.phase = Phase::Heating;
    }

    /// Takes the kettle off and pours onto the leaves: the steep starts.
    pub fn pour(&mut self) -> f64 {
        self.kettle.on = false;
        self.poured_at = self.kettle.temp;
        self.steeped = 0.0;
        self.ticks = 0;
        self.timer_rang = false;
        self.phase = Phase::Steeping;
        self.poured_at
    }

    /// Lifts the leaves out. A pot for one is served at once; a pot for
    /// several waits to be poured.
    pub fn lift(&mut self) {
        self.phase = if self.cups > 1 {
            Phase::Pouring
        } else {
            Phase::Served
        };
    }

    /// Pours the next cup of a pot. Returns how many are poured.
    pub fn pour_cup(&mut self) -> u8 {
        self.poured = (self.poured + 1).min(self.cups);
        if self.poured >= self.cups {
            self.phase = Phase::Served;
        }
        self.poured
    }

    /// Seconds of the steep, as the clock has ticked them.
    pub fn seconds(&self) -> u32 {
        self.ticks
    }

    pub fn update(&mut self, dt: f64) -> Vec<Cue> {
        let mut cues = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |step| self.substep(step, &mut cues));
        self.clock = clock;
        cues
    }

    fn substep(&mut self, step: f64, cues: &mut Vec<Cue>) {
        self.kettle.step(step);
        match self.phase {
            Phase::Arriving => {
                self.wait -= step;
                if self.wait <= 1e-9 {
                    self.phase = Phase::Choosing;
                    cues.push(Cue::Arrived);
                }
            }
            Phase::Heating => {
                let stage = self.kettle.stage();
                if stage > self.stage {
                    self.stage = stage;
                    cues.push(Cue::Stage(stage));
                }
            }
            Phase::Steeping => {
                self.steeped += step;
                let whole = (self.steeped + 1e-9).floor() as u32;
                if whole > self.ticks {
                    self.ticks = whole;
                    cues.push(Cue::Tick(whole));
                }
                if let Some(tea) = self.tea {
                    if self.sand_timer
                        && !self.timer_rang
                        && self.steeped + 1e-9 >= tea.steep as f64
                    {
                        self.timer_rang = true;
                        cues.push(Cue::Timer);
                    }
                }
                if self.steeped >= MAX_STEEP {
                    self.lift();
                    cues.push(Cue::LongSteep);
                }
            }
            _ => {}
        }
    }
}

/// How the tea house is being played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The main game: an afternoon of 4 guests.
    Afternoon,
    /// A rainy evening of 7 guests, opened after 3 afternoons.
    Evening,
    /// Cups for yourself, as many as you like, with exact numbers.
    Practice,
    Challenge(Period),
}

/// Afternoons to play before the long evening opens.
pub const EVENING_AFTER: u32 = 3;

impl Mode {
    /// Guests in one sitting; 0 for practice, which never ends.
    pub fn guests(self) -> usize {
        match self {
            Mode::Afternoon => 4,
            Mode::Evening => 7,
            Mode::Practice => 0,
            Mode::Challenge(Period::Daily) => 3,
            Mode::Challenge(Period::Weekly) => 6,
            Mode::Challenge(Period::Monthly) => 5,
        }
    }

    /// The board its best coins are kept under.
    pub fn board(self) -> Option<&'static str> {
        match self {
            Mode::Afternoon => Some("afternoon"),
            Mode::Evening => Some("evening"),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Afternoon => "The afternoon",
            Mode::Evening => "Long evening",
            Mode::Practice => "Practice brewing",
            Mode::Challenge(_) => "Challenge",
        }
    }
}

/// Who is at the counter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Who {
    Regular(&'static Regular),
    WalkIn(&'static WalkIn),
    /// The day's special guest.
    Holiday(Occasion),
    Taster(&'static Taster),
    /// Practice: a cup for yourself.
    Myself,
}

/// One guest's visit, planned as the afternoon opens.
#[derive(Clone, Debug, PartialEq)]
pub struct Visit {
    pub who: Who,
    /// The taste asked for; `None` when a tea is asked for by name.
    pub taste: Option<Taste>,
    /// The tea asked for by name, if any.
    pub wants: Option<&'static Tea>,
    /// What the guest says they would like, without their name.
    pub asks: String,
    pub cups: u8,
    /// The coming holiday, for a regular to mention.
    pub hint: Option<Occasion>,
    /// The regular's first visit.
    pub first: bool,
    /// The year the occasion belongs to, for its keepsake.
    pub year: i32,
}

impl Visit {
    /// The guest's name, to start a line: "Mister Hale", "The walker".
    pub fn name(&self) -> String {
        match self.who {
            Who::Regular(r) => r.name.to_string(),
            Who::WalkIn(w) => w.short.to_string(),
            Who::Holiday(o) => holiday_name(o).to_string(),
            Who::Taster(t) => t.short.to_string(),
            Who::Myself => "You".to_string(),
        }
    }
}

/// The day's guest, by name.
pub fn holiday_name(o: Occasion) -> &'static str {
    match o {
        Occasion::LunarNewYear => "Mei",
        Occasion::Valentines => "Rosa",
        Occasion::SpringEquinox => "The gardener",
        Occasion::Midsummer => "The boy",
        Occasion::AutumnEquinox => "The farmer",
        Occasion::Halloween => "The stranger",
        Occasion::Thanksgiving => "The grandmother",
    }
}

/// What the tea house knows when it plans the guests.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Known {
    /// Afternoons and evenings played before this one.
    pub sessions: u32,
    /// The session each regular last came in, by id.
    pub last_seen: BTreeMap<String, u32>,
}

/// The guests of one sitting, in order. On a holiday the day's guest comes
/// last; in a holiday's run-up the first regular mentions it.
pub fn plan(known: &Known, today: Date, mode: Mode, rng: &mut Rng) -> Vec<Visit> {
    let n = mode.guests();
    let year = today.year;
    let mut visits = Vec::new();
    match mode {
        Mode::Practice => return visits,
        Mode::Challenge(Period::Monthly) => {
            let mut order: Vec<usize> = (0..TEAS.len()).collect();
            rng.shuffle(&mut order);
            for (i, taster) in TASTERS.iter().enumerate().take(n) {
                let tea = &TEAS[order[i]];
                visits.push(Visit {
                    who: Who::Taster(taster),
                    taste: None,
                    wants: Some(tea),
                    asks: format!("A cup of {}, please.", tea.name),
                    cups: 1,
                    hint: None,
                    first: false,
                    year,
                });
            }
            return visits;
        }
        Mode::Challenge(_) => {
            let mut order: Vec<usize> = (0..WALK_INS.len()).collect();
            rng.shuffle(&mut order);
            for &i in order.iter().take(n) {
                visits.push(walk_in(&WALK_INS[i], year));
            }
            return visits;
        }
        Mode::Afternoon | Mode::Evening => {}
    }
    let occasion = occasion_on(today);
    let slots = n;
    let slots = if occasion.is_some() { slots - 1 } else { slots };
    let mut met = regulars_met(known.sessions);
    // The least recently seen first; never seen is longest ago.
    rng.shuffle(&mut met);
    met.sort_by_key(|r| known.last_seen.get(r.id).map(|&s| s as i64).unwrap_or(-1));
    let want_regulars = if mode == Mode::Evening { 3 } else { 2 };
    let regulars: Vec<&'static Regular> = met.into_iter().take(want_regulars.min(slots)).collect();
    for r in &regulars {
        let ask = rng.pick(r.taste.asks());
        visits.push(Visit {
            who: Who::Regular(r),
            taste: Some(r.taste),
            wants: None,
            asks: ask.to_string(),
            cups: 1,
            hint: None,
            first: !known.last_seen.contains_key(r.id),
            year,
        });
    }
    let mut order: Vec<usize> = (0..WALK_INS.len()).collect();
    rng.shuffle(&mut order);
    for &i in order.iter().take(slots - visits.len()) {
        visits.push(walk_in(&WALK_INS[i], year));
    }
    rng.shuffle(&mut visits);
    if let Some((o, _)) = coming(today) {
        if let Some(v) = visits.iter_mut().find(|v| matches!(v.who, Who::Regular(_))) {
            v.hint = Some(o);
        }
    }
    if let Some(o) = occasion {
        visits.push(holiday_visit(o, known, today));
    }
    visits
}

fn walk_in(w: &'static WalkIn, year: i32) -> Visit {
    Visit {
        who: Who::WalkIn(w),
        taste: Some(w.taste),
        wants: None,
        asks: w.asks.to_string(),
        cups: 1,
        hint: None,
        first: false,
        year,
    }
}

/// The day's special guest.
pub fn holiday_visit(o: Occasion, _known: &Known, today: Date) -> Visit {
    let tea = o.tea();
    Visit {
        who: Who::Holiday(o),
        taste: None,
        wants: Some(tea),
        asks: format!("{} is on your shelf today.", tea.name),
        cups: o.cups(),
        hint: None,
        first: false,
        year: o.year(today),
    }
}

/// Practice: a cup for yourself, any tea you like.
pub fn practice_visit(year: i32) -> Visit {
    Visit {
        who: Who::Myself,
        taste: None,
        wants: None,
        asks: "A cup for yourself. Any tea you like.".to_string(),
        cups: 1,
        hint: None,
        first: false,
        year,
    }
}

/// A sitting in progress: its guests, and how their cups went.
#[derive(Clone, Debug, PartialEq)]
pub struct Sitting {
    pub mode: Mode,
    pub visits: Vec<Visit>,
    /// The guest at the counter, from 0.
    pub index: usize,
    pub verdicts: Vec<Verdict>,
    pub coins: u32,
    pub tips: u32,
    /// Cups poured: a pot for two counts two.
    pub cups: u32,
}

impl Sitting {
    pub fn new(mode: Mode, visits: Vec<Visit>) -> Sitting {
        Sitting {
            mode,
            visits,
            index: 0,
            verdicts: Vec::new(),
            coins: 0,
            tips: 0,
            cups: 0,
        }
    }

    pub fn visit(&self) -> &Visit {
        &self.visits[self.index.min(self.visits.len().saturating_sub(1))]
    }

    /// Whether this is the last guest of the sitting.
    pub fn last(&self) -> bool {
        self.mode != Mode::Practice && self.index + 1 >= self.visits.len()
    }

    /// Counts a served cup: its coins, once for each cup poured.
    pub fn serve(&mut self, verdict: Verdict, cups: u8) -> (u32, u32) {
        let (pay, tip) = verdict.coins();
        let (pay, tip) = (pay * cups as u32, tip * cups as u32);
        self.coins += pay + tip;
        self.tips += tip;
        self.cups += cups as u32;
        self.verdicts.push(verdict);
        (pay, tip)
    }

    /// On to the next guest. False when the sitting is over.
    pub fn advance(&mut self) -> bool {
        if self.mode == Mode::Practice {
            return true;
        }
        self.index += 1;
        self.index < self.visits.len()
    }

    pub fn perfect(&self) -> usize {
        self.verdicts
            .iter()
            .filter(|v| v.grade == Grade::Perfect)
            .count()
    }

    /// The challenge score: every cup's points added.
    pub fn points(&self) -> u64 {
        self.verdicts.iter().map(|v| v.points as u64).sum()
    }
}

/// A random pick from a list of lines, for variety.
pub fn line(rng: &mut Rng, lines: &'static [&'static str]) -> &'static str {
    rng.pick(lines)
}

/// A note for a perfect cup.
pub fn note(rng: &mut Rng) -> &'static str {
    rng.pick(&NOTES)
}

/// The teas a guest's taste would like, from `shelf`.
pub fn suggestions(shelf: &[&'static Tea], taste: Taste) -> Vec<&'static Tea> {
    shelf.iter().copied().filter(|t| t.suits(taste)).collect()
}

/// The shelf in a challenge: every year-round tea, the same for everyone.
pub fn challenge_shelf() -> Vec<&'static Tea> {
    TEAS.iter().collect()
}

/// Every cup, for a challenge.
pub fn all_cups() -> Vec<&'static Cup> {
    CUPS.iter().collect()
}

/// Whether a tea is iced.
pub fn iced(tea: &Tea) -> bool {
    tea.style == Style::Iced
}

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins the seed every sitting starts from, for tests.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A random source for a sitting: the pinned seed in tests, else the clock.
pub fn sitting_rng() -> Rng {
    FIXED_SEED
        .with(|s| s.get())
        .map(Rng::seeded)
        .unwrap_or_else(Rng::from_time)
}

/// Every regular, for the guest book.
pub fn regulars() -> &'static [Regular] {
    &REGULARS
}

/// The shelf on `today` for a house that owns `owned`.
pub fn shelf_for(owned: &dyn Fn(&str) -> bool, today: Date) -> Vec<&'static Tea> {
    days::shelf(owned, today)
}
