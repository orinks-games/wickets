//! The stream itself: the boat, the current, what floats down, and the
//! score. No sound or speech here, so every rule can be tested alone.
//!
//! Time runs on the music's beat. Everything that floats down arrives at
//! the boat on a beat, and is heard [`LEAD_BEATS`] beats before. Steering
//! is smooth, not in lanes: the boat drifts across the stream while
//! Left or Right is held.

use parlor_core::Rng;

/// Beats a minute.
pub const BPM: f64 = 96.0;
/// One beat, in seconds.
pub const BEAT: f64 = 60.0 / BPM;
/// Beats in a bar.
pub const BAR_BEATS: u32 = 4;
/// From the middle of the stream to either bank, in metres.
pub const HALF: f64 = 4.0;
/// The boat bumps the bank when its middle is this close to it.
pub const BANK_MARGIN: f64 = 0.4;
/// The bank's lapping is said to be close inside this distance.
pub const BANK_CLOSE: f64 = 1.2;
/// How far ahead a floating thing is first heard, in beats.
pub const LEAD_BEATS: u32 = 6;
/// A mill wheel is heard further ahead, so there is time to change sides.
pub const MILL_LEAD_BEATS: u32 = 8;
/// How long the boat stays shaken after a bump, in seconds.
pub const BUMP_SHAKE: f64 = 0.8;
/// How close to the current's middle counts as riding it, in metres.
pub const CURRENT_HALF: f64 = 1.0;
/// Catches between each step of the multiplier.
pub const COMBO_STEP: u32 = 10;
/// The highest multiplier.
pub const MAX_MULT: u32 = 5;
/// Bumps an endless boat takes before it sinks.
pub const SOGGY_SINKS: u32 = 5;
/// Catches that dry one bump out of an endless boat.
pub const DRY_EVERY: u32 = 20;
/// Points for each beat spent riding the current, before the multiplier.
pub const FLOW_POINTS: u64 = 2;
/// Points for going through a gate or a waterfall, before the multiplier.
pub const GATE_POINTS: u64 = 40;
/// The share of things caught that earns a second star, and a third.
pub const TWO_STARS: f64 = 0.6;
pub const THREE_STARS: f64 = 0.85;
/// Metres the boat travels in one beat, for endless distance.
pub const METRES_PER_BEAT: f64 = 2.0;
/// Seconds the river mouth rings before the results.
pub const MOUTH_TAIL: f64 = 2.5;

/// The four streams of the first season, in the order they open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stream {
    GardenBrook,
    RainGutter,
    MillRace,
    MarketCanal,
}

impl Stream {
    pub const ALL: [Stream; 4] = [
        Stream::GardenBrook,
        Stream::RainGutter,
        Stream::MillRace,
        Stream::MarketCanal,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stream::GardenBrook => "Garden brook",
            Stream::RainGutter => "Rain gutter",
            Stream::MillRace => "Mill race",
            Stream::MarketCanal => "Market canal",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Stream::GardenBrook => "garden",
            Stream::RainGutter => "gutter",
            Stream::MillRace => "mill",
            Stream::MarketCanal => "canal",
        }
    }

    pub fn from_id(id: &str) -> Option<Stream> {
        Stream::ALL.into_iter().find(|s| s.id() == id)
    }

    pub fn index(self) -> usize {
        Stream::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    /// Stars, over every stream, that open this one.
    pub fn stars_to_open(self) -> u32 {
        match self {
            Stream::GardenBrook => 0,
            Stream::RainGutter => 2,
            Stream::MillRace => 4,
            Stream::MarketCanal => 7,
        }
    }

    /// Bars from the launch to the river mouth.
    pub fn bars(self) -> u32 {
        match self {
            Stream::GardenBrook => 32,
            Stream::RainGutter => 36,
            Stream::MillRace => 40,
            Stream::MarketCanal => 44,
        }
    }

    /// The stream's own twist, for its row and its opening line.
    pub fn twist(self) -> &'static str {
        match self {
            Stream::GardenBrook => "a gentle brook, with a few stones and gates",
            Stream::RainGutter => "waterfalls to steer through",
            Stream::MillRace => "mill wheels that block one side",
            Stream::MarketCanal => "ducks that paddle across your path",
        }
    }

    /// What the stream adds to the mix of hazards.
    pub fn hazard(self) -> Option<Kind> {
        match self {
            Stream::GardenBrook => None,
            Stream::RainGutter => Some(Kind::Falls),
            Stream::MillRace => Some(Kind::Mill),
            Stream::MarketCanal => Some(Kind::Duck),
        }
    }
}

/// Something that floats down to the boat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A petal: the common catch.
    Petal,
    /// A leaf: a common catch, a lower note.
    Leaf,
    /// A scrap of paper: folds into new boats.
    Scrap,
    /// A lost trinket for the shelf: rare.
    Trinket,
    /// A floating lantern, on lantern float days.
    Lantern,
    /// A stone: steer round it.
    Stone,
    /// A gate between two stones: go through the middle for points.
    Gate,
    /// A waterfall between rocks: go through the rush of water.
    Falls,
    /// A mill wheel turning on one side: be on the other side.
    Mill,
    /// A duck paddling across.
    Duck,
}

impl Kind {
    pub const ALL: [Kind; 10] = [
        Kind::Petal,
        Kind::Leaf,
        Kind::Scrap,
        Kind::Trinket,
        Kind::Lantern,
        Kind::Stone,
        Kind::Gate,
        Kind::Falls,
        Kind::Mill,
        Kind::Duck,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Kind::Petal => "petal",
            Kind::Leaf => "leaf",
            Kind::Scrap => "scrap",
            Kind::Trinket => "trinket",
            Kind::Lantern => "lantern",
            Kind::Stone => "stone",
            Kind::Gate => "gate",
            Kind::Falls => "falls",
            Kind::Mill => "mill",
            Kind::Duck => "duck",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Petal => "petal",
            Kind::Leaf => "leaf",
            Kind::Scrap => "paper scrap",
            Kind::Trinket => "trinket",
            Kind::Lantern => "lantern",
            Kind::Stone => "stone",
            Kind::Gate => "gate",
            Kind::Falls => "waterfall",
            Kind::Mill => "mill wheel",
            Kind::Duck => "duck",
        }
    }

    /// Something to catch, as against something to steer round or through.
    pub fn is_catch(self) -> bool {
        matches!(
            self,
            Kind::Petal | Kind::Leaf | Kind::Scrap | Kind::Trinket | Kind::Lantern
        )
    }

    /// Points for a catch, before the multiplier.
    pub fn points(self) -> u64 {
        match self {
            Kind::Petal | Kind::Leaf | Kind::Scrap => 10,
            Kind::Lantern => 50,
            Kind::Trinket => 100,
            _ => 0,
        }
    }
}

/// The boats you can fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Boat {
    Paper,
    Newspaper,
    Leaf,
}

impl Boat {
    pub const ALL: [Boat; 3] = [Boat::Paper, Boat::Newspaper, Boat::Leaf];

    pub fn id(self) -> &'static str {
        match self {
            Boat::Paper => "paper",
            Boat::Newspaper => "newspaper",
            Boat::Leaf => "leaf",
        }
    }

    pub fn from_id(id: &str) -> Option<Boat> {
        Boat::ALL.into_iter().find(|b| b.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Boat::Paper => "Paper boat",
            Boat::Newspaper => "Newspaper boat",
            Boat::Leaf => "Leaf boat",
        }
    }

    /// What it is like to steer, in a few words.
    pub fn handling(self) -> &'static str {
        match self {
            Boat::Paper => "steady, the boat you start with",
            Boat::Newspaper => "wide, catches more, turns slowly",
            Boat::Leaf => "light and quick, and a bump costs only half your combo",
        }
    }

    /// Paper scraps it takes to fold.
    pub fn scraps(self) -> u32 {
        match self {
            Boat::Paper => 0,
            Boat::Newspaper => 25,
            Boat::Leaf => 50,
        }
    }

    /// Top sideways speed, metres a second.
    pub fn steer(self) -> f64 {
        match self {
            Boat::Paper => 3.2,
            Boat::Newspaper => 2.6,
            Boat::Leaf => 3.9,
        }
    }

    /// How far from the boat's middle a thing is still caught, metres.
    pub fn reach(self) -> f64 {
        match self {
            Boat::Paper => 0.8,
            Boat::Newspaper => 1.1,
            Boat::Leaf => 0.7,
        }
    }

    /// How close counts as a clean catch, metres, before the setting.
    pub fn clean(self) -> f64 {
        match self {
            Boat::Paper => 0.3,
            Boat::Newspaper => 0.4,
            Boat::Leaf => 0.3,
        }
    }

    /// The combo a bump leaves.
    pub fn after_bump(self, combo: u32) -> u32 {
        match self {
            Boat::Leaf => combo / 2,
            _ => 0,
        }
    }
}

/// How wide a clean catch is, in Settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CleanWidth {
    Narrow,
    #[default]
    Normal,
    Wide,
}

impl CleanWidth {
    pub const ALL: [CleanWidth; 3] = [CleanWidth::Narrow, CleanWidth::Normal, CleanWidth::Wide];

    pub fn scale(self) -> f64 {
        match self {
            CleanWidth::Narrow => 0.7,
            CleanWidth::Normal => 1.0,
            CleanWidth::Wide => 1.5,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            CleanWidth::Narrow => "narrow",
            CleanWidth::Normal => "normal",
            CleanWidth::Wide => "wide",
        }
    }

    pub fn id(self) -> u8 {
        match self {
            CleanWidth::Narrow => 0,
            CleanWidth::Normal => 1,
            CleanWidth::Wide => 2,
        }
    }

    pub fn from_id(id: u8) -> CleanWidth {
        match id {
            0 => CleanWidth::Narrow,
            2 => CleanWidth::Wide,
            _ => CleanWidth::Normal,
        }
    }
}

/// How a run is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// One of the streams, for stars and records.
    Stream(Stream),
    /// A stream with no end, built from every stream you have opened.
    Endless,
    /// No score and no bumps, slow water and the whole band.
    Drift,
    /// The tutorial's slow water.
    Practice,
    /// The challenges.
    Daily,
    Weekly,
    Monthly,
}

impl Mode {
    pub fn period(self) -> Option<parlor_core::Period> {
        match self {
            Mode::Daily => Some(parlor_core::Period::Daily),
            Mode::Weekly => Some(parlor_core::Period::Weekly),
            Mode::Monthly => Some(parlor_core::Period::Monthly),
            _ => None,
        }
    }

    pub fn for_period(period: parlor_core::Period) -> Mode {
        match period {
            parlor_core::Period::Daily => Mode::Daily,
            parlor_core::Period::Weekly => Mode::Weekly,
            parlor_core::Period::Monthly => Mode::Monthly,
        }
    }

    /// Whether the run keeps a score.
    pub fn scored(self) -> bool {
        !matches!(self, Mode::Drift | Mode::Practice)
    }

    /// Whether the banks and hazards cost anything.
    pub fn bumps_count(self) -> bool {
        !matches!(self, Mode::Drift | Mode::Practice)
    }

    /// The score board this mode keeps, if any.
    pub fn board(self) -> Option<&'static str> {
        match self {
            Mode::Stream(s) => Some(s.id()),
            Mode::Endless => Some("endless"),
            _ => None,
        }
    }

    pub fn label(self) -> String {
        match self {
            Mode::Stream(s) => s.name().to_string(),
            Mode::Endless => "The endless river".to_string(),
            Mode::Drift => "Drift".to_string(),
            Mode::Practice => "Practice water".to_string(),
            Mode::Daily => "Today's stream".to_string(),
            Mode::Weekly => "Boat of the week".to_string(),
            Mode::Monthly => "The long run".to_string(),
        }
    }
}

/// What a run is built from: its mode, which twists it uses, its length
/// and its calendar touches.
#[derive(Clone, Debug, PartialEq)]
pub struct Course {
    pub mode: Mode,
    /// Bars to the river mouth; `None` runs until the boat sinks.
    pub bars: Option<u32>,
    /// The hazards this course mixes in, besides stones and gates.
    pub hazards: Vec<Kind>,
    /// The stream whose trinkets float here.
    pub trinkets_from: Option<Stream>,
    /// Lanterns float down (lantern float day).
    pub lanterns: bool,
    /// More ducks, worth points to pass (duck parade day).
    pub parade: bool,
    /// How hard it starts, 0 gentle to 1 busy.
    pub start_level: f64,
    /// How hard it ends.
    pub end_level: f64,
}

impl Course {
    pub fn stream(s: Stream) -> Course {
        let start = 0.1 + 0.12 * s.index() as f64;
        Course {
            mode: Mode::Stream(s),
            bars: Some(s.bars()),
            hazards: s.hazard().into_iter().collect(),
            trinkets_from: Some(s),
            lanterns: false,
            parade: false,
            start_level: start,
            end_level: (start + 0.5).min(1.0),
        }
    }

    pub fn endless(open: &[Stream]) -> Course {
        Course {
            mode: Mode::Endless,
            bars: None,
            hazards: open.iter().filter_map(|s| s.hazard()).collect(),
            trinkets_from: None,
            lanterns: false,
            parade: false,
            start_level: 0.2,
            end_level: 1.0,
        }
    }

    pub fn drift() -> Course {
        Course {
            mode: Mode::Drift,
            bars: Some(24),
            hazards: Vec::new(),
            trinkets_from: None,
            lanterns: false,
            parade: false,
            start_level: 0.0,
            end_level: 0.15,
        }
    }

    pub fn practice() -> Course {
        Course {
            mode: Mode::Practice,
            bars: None,
            hazards: Vec::new(),
            trinkets_from: None,
            lanterns: false,
            parade: false,
            start_level: 0.0,
            end_level: 0.1,
        }
    }

    pub fn challenge(mode: Mode) -> Course {
        let all = vec![Kind::Falls, Kind::Mill, Kind::Duck];
        let (bars, hazards, start, end) = match mode {
            Mode::Daily => (32, all, 0.3, 0.7),
            Mode::Weekly => (40, vec![Kind::Mill, Kind::Duck], 0.35, 0.8),
            _ => (56, all, 0.3, 1.0),
        };
        Course {
            mode,
            bars: Some(bars),
            hazards,
            trinkets_from: None,
            lanterns: false,
            parade: false,
            start_level: start,
            end_level: end,
        }
    }

    /// How hard bar `bar` is, 0 to 1.
    pub fn level_at(&self, bar: u32) -> f64 {
        match self.bars {
            Some(total) if total > 1 => {
                let t = bar as f64 / (total - 1) as f64;
                self.start_level + (self.end_level - self.start_level) * t.clamp(0.0, 1.0)
            }
            Some(_) => self.start_level,
            // Endless climbs over its first 64 bars, then stays at its busiest.
            None => {
                let t = (bar as f64 / 64.0).min(1.0);
                self.start_level + (self.end_level - self.start_level) * t
            }
        }
    }
}

/// One thing on its way down the stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Thing {
    pub id: u32,
    pub kind: Kind,
    /// Across the stream at arrival: minus is left, metres from the middle.
    pub x: f64,
    /// The beat it reaches the boat.
    pub arrive: u32,
    /// The beat it is first heard.
    pub heard: u32,
    /// For a duck: where it starts, so it paddles from `from` to `x`.
    pub from: f64,
    /// For a mill wheel: the side it blocks, -1 left or 1 right.
    pub side: i8,
    /// For a trinket: which one.
    pub trinket: Option<&'static str>,
}

impl Thing {
    /// Where it is across the stream at `beat_pos` (beats from the start).
    pub fn x_at(&self, beat_pos: f64) -> f64 {
        if self.kind != Kind::Duck {
            return self.x;
        }
        let span = (self.arrive - self.heard).max(1) as f64;
        let t = ((beat_pos - self.heard as f64) / span).clamp(0.0, 1.0);
        self.from + (self.x - self.from) * t
    }

    /// Beats left until it arrives, from `beat_pos`.
    pub fn beats_left(&self, beat_pos: f64) -> f64 {
        self.arrive as f64 - beat_pos
    }
}

/// What happened, for the screen to sound and say.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The boat is on the water and the stream begins.
    Go,
    /// A thing is first heard.
    Heard { id: u32, kind: Kind },
    /// The first of its kind on this course, for a first-time explanation.
    Introduce(Kind),
    /// Caught.
    Caught {
        id: u32,
        kind: Kind,
        x: f64,
        clean: bool,
        points: u64,
        combo: u32,
        trinket: Option<&'static str>,
    },
    /// Floated past out of reach.
    Missed { id: u32, kind: Kind, x: f64 },
    /// Through a gate or a waterfall.
    Through { kind: Kind, x: f64, points: u64 },
    /// Past a stone, a mill wheel or a duck without touching it.
    Passed { kind: Kind, x: f64, points: u64 },
    /// A gate the boat went round: no harm, no points.
    Around { x: f64 },
    /// Hit a bank (`hazard` None) or a hazard.
    Bump {
        hazard: Option<Kind>,
        x: f64,
        lost_combo: u32,
        soggy: u32,
    },
    /// The multiplier went up.
    Multiplier(u32),
    /// The score passed the old best.
    NewBest,
    /// The boat started riding the current, or left it.
    Current(bool),
    /// A dry spell took one bump off an endless boat.
    Dried { soggy: u32 },
    /// The end of the course: the river mouth.
    Mouth,
    /// The endless boat sank.
    Sank,
}

/// The run's stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Waiting for the opening line to be heard, this many seconds left.
    LeadIn(f64),
    Playing,
    /// Past the mouth, ringing out this many seconds.
    Ending(f64),
    Over,
}

/// A run down a stream.
pub struct Run {
    pub course: Course,
    pub boat: Boat,
    pub clean_width: CleanWidth,
    pub phase: Phase,
    rng: Rng,
    /// Seconds of play since Go.
    pub time: f64,
    /// Beats handled so far: every arrival up to here is settled.
    pub beat: u32,
    pub x: f64,
    pub vx: f64,
    /// Which way the player is steering: -1, 0 or 1.
    pub steer: i8,
    pub shake: f64,
    pub things: Vec<Thing>,
    /// Planned things not yet heard, in order of `heard`.
    plan: Vec<Thing>,
    /// Bars planned so far.
    planned_bars: u32,
    next_id: u32,
    /// The current's middle drifts on its own path.
    current_phase: f64,
    pub riding: bool,
    pub score: u64,
    pub combo: u32,
    pub best_combo: u32,
    pub caught: u32,
    pub clean: u32,
    /// Things to catch that reached the boat, caught or not.
    pub offered: u32,
    pub bumps: u32,
    pub soggy: u32,
    pub flow_beats: u32,
    pub gates_through: u32,
    pub gates_total: u32,
    pub falls_through: u32,
    pub falls_total: u32,
    pub mills_passed: u32,
    pub mills_total: u32,
    pub ducks_bumped: u32,
    pub ducks_total: u32,
    pub scraps: u32,
    pub trinkets: Vec<&'static str>,
    /// Trinkets already on the shelf, so new ones come first.
    pub shelf: Vec<String>,
    /// Kinds met before, so only new ones are explained.
    pub met: Vec<Kind>,
    /// Whether to raise `Introduce` events at all.
    pub introduce: bool,
    /// The best to beat, for the new best line.
    pub best: Option<u64>,
    said_best: bool,
    events: Vec<Event>,
}

impl Run {
    pub fn new(course: Course, boat: Boat, rng: Rng) -> Run {
        let mut run = Run {
            course,
            boat,
            clean_width: CleanWidth::Normal,
            phase: Phase::LeadIn(3.0),
            rng,
            time: 0.0,
            beat: 0,
            x: 0.0,
            vx: 0.0,
            steer: 0,
            shake: 0.0,
            things: Vec::new(),
            plan: Vec::new(),
            planned_bars: 0,
            next_id: 1,
            current_phase: 0.0,
            riding: false,
            score: 0,
            combo: 0,
            best_combo: 0,
            caught: 0,
            clean: 0,
            offered: 0,
            bumps: 0,
            soggy: 0,
            flow_beats: 0,
            gates_through: 0,
            gates_total: 0,
            falls_through: 0,
            falls_total: 0,
            mills_passed: 0,
            mills_total: 0,
            ducks_bumped: 0,
            ducks_total: 0,
            scraps: 0,
            trinkets: Vec::new(),
            shelf: Vec::new(),
            met: Vec::new(),
            introduce: false,
            best: None,
            said_best: false,
            events: Vec::new(),
        };
        run.current_phase = run.rng.unit() * std::f64::consts::TAU;
        run.plan_ahead();
        run
    }

    /// The total beats to the mouth, if the course has one.
    pub fn total_beats(&self) -> Option<u32> {
        self.course.bars.map(|b| b * BAR_BEATS)
    }

    /// Beats from Go, with the fraction of the beat in progress.
    pub fn beat_pos(&self) -> f64 {
        self.time / BEAT
    }

    /// The bar in progress.
    pub fn bar(&self) -> u32 {
        (self.beat_pos() / BAR_BEATS as f64) as u32
    }

    pub fn multiplier(&self) -> u32 {
        (1 + self.combo / COMBO_STEP).min(MAX_MULT)
    }

    /// How close a clean catch has to be.
    pub fn clean_reach(&self) -> f64 {
        self.boat.clean() * self.clean_width.scale()
    }

    /// How far the current's middle swings from the middle of the stream.
    fn current_swing(&self) -> f64 {
        let level = self.course.level_at(self.bar());
        1.0 + 1.6 * level
    }

    /// The current's middle at `beat_pos`.
    pub fn current_at(&self, beat_pos: f64) -> f64 {
        let swing = self.current_swing();
        // Two slow waves, so it never repeats exactly.
        let a = (beat_pos / 23.0 + self.current_phase).sin();
        let b = (beat_pos / 9.0 + self.current_phase * 1.7).sin();
        ((a * 0.75 + b * 0.25) * swing).clamp(-(HALF - 1.0), HALF - 1.0)
    }

    /// The current's middle now.
    pub fn current(&self) -> f64 {
        self.current_at(self.beat_pos())
    }

    pub fn is_playing(&self) -> bool {
        self.phase == Phase::Playing
    }

    pub fn is_over(&self) -> bool {
        self.phase == Phase::Over
    }

    /// Whether the boat reached the mouth (as against sinking).
    pub fn reached_mouth(&self) -> bool {
        self.course.bars.is_some() && self.soggy < SOGGY_SINKS
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Starts now instead of waiting for the opening line.
    pub fn start_now(&mut self) -> bool {
        if matches!(self.phase, Phase::LeadIn(_)) {
            self.phase = Phase::Playing;
            self.events.push(Event::Go);
            true
        } else {
            false
        }
    }

    /// Sets the opening wait.
    pub fn set_lead_in(&mut self, secs: f64) {
        if matches!(self.phase, Phase::LeadIn(_)) {
            self.phase = Phase::LeadIn(secs);
        }
    }

    /// Stars this run earns: one for the mouth, more for the share caught.
    pub fn stars(&self) -> u32 {
        if !self.reached_mouth() || self.phase != Phase::Over {
            return 0;
        }
        let share = self.catch_share();
        if share >= THREE_STARS {
            3
        } else if share >= TWO_STARS {
            2
        } else {
            1
        }
    }

    /// The share of things caught, 0 to 1.
    pub fn catch_share(&self) -> f64 {
        if self.offered == 0 {
            0.0
        } else {
            self.caught as f64 / self.offered as f64
        }
    }

    /// Endless distance, metres.
    pub fn metres(&self) -> u64 {
        (self.beat as f64 * METRES_PER_BEAT) as u64
    }

    /// How far down the course, 0 to 1, if it has an end.
    pub fn progress(&self) -> Option<f64> {
        self.total_beats()
            .map(|t| (self.beat_pos() / t.max(1) as f64).clamp(0.0, 1.0))
    }

    /// The nearest bank's side (-1 left, 1 right) and its distance from
    /// the boat's middle, metres.
    pub fn nearest_bank(&self) -> (i8, f64) {
        if self.x < 0.0 {
            (-1, self.x + HALF)
        } else {
            (1, HALF - self.x)
        }
    }

    // --- planning -------------------------------------------------------

    fn id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Plans bars until the plan reaches a few bars past what can be heard.
    fn plan_ahead(&mut self) {
        let want = self.bar() + 4;
        while self.planned_bars < want {
            if let Some(total) = self.course.bars {
                if self.planned_bars >= total {
                    break;
                }
            }
            let bar = self.planned_bars;
            self.plan_bar(bar);
            self.planned_bars += 1;
        }
        self.plan.sort_by_key(|t| (t.heard, t.id));
    }

    fn push_thing(&mut self, kind: Kind, x: f64, arrive: u32) -> u32 {
        let lead = if kind == Kind::Mill {
            MILL_LEAD_BEATS
        } else {
            LEAD_BEATS
        };
        let id = self.id();
        self.plan.push(Thing {
            id,
            kind,
            x: x.clamp(-(HALF - 0.8), HALF - 0.8),
            arrive,
            heard: arrive.saturating_sub(lead),
            from: x,
            side: 0,
            trinket: None,
        });
        id
    }

    fn last_mut(&mut self) -> &mut Thing {
        self.plan.last_mut().expect("just pushed")
    }

    /// Where the path of catches was last placed, and on which beat.
    fn last_catch(&self) -> Option<(f64, u32)> {
        self.plan
            .iter()
            .filter(|t| t.kind.is_catch())
            .max_by_key(|t| t.arrive)
            .map(|t| (t.x, t.arrive))
    }

    fn plan_bar(&mut self, bar: u32) {
        let level = self.course.level_at(bar);
        let first = bar * BAR_BEATS;
        // The first bar is quiet: the boat settles on the water.
        if bar == 0 {
            return;
        }
        let last_bar = self.course.bars.map(|b| bar + 1 >= b).unwrap_or(false);
        if last_bar {
            // The last bar is the run into the mouth: one easy petal.
            let x = self.current_at(first as f64);
            self.push_thing(Kind::Petal, x, first);
            return;
        }
        // Which beats carry a catch.
        let beats: &[u32] = if level < 0.2 {
            &[0]
        } else if level < 0.45 {
            if bar.is_multiple_of(2) {
                &[0, 2]
            } else {
                &[0]
            }
        } else if level < 0.75 {
            &[0, 2]
        } else if bar.is_multiple_of(2) {
            &[0, 1, 2]
        } else {
            &[0, 2, 3]
        };
        // Every few bars a hazard takes a beat of its own.
        let hazard_every = if level < 0.3 {
            4
        } else if level < 0.7 {
            3
        } else {
            2
        };
        let hazard_beat = if bar >= 2 && bar % hazard_every == 1 {
            Some(first + 3)
        } else {
            None
        };
        let (mut px, mut pb) = self.last_catch().unwrap_or((0.0, first.saturating_sub(2)));
        let steer = self.boat.steer().min(Boat::Newspaper.steer());
        for &b in beats {
            let beat = first + b;
            if Some(beat) == hazard_beat {
                continue;
            }
            // Keep each catch within reach of the last one: half the
            // distance the slowest boat can cross in the time between.
            let gap = (beat.saturating_sub(pb)).max(1) as f64 * BEAT;
            let reach = (steer * gap * 0.5).min(2.5 + 2.5 * level);
            let pull = self.current_at(beat as f64);
            let wander = (self.rng.unit() * 2.0 - 1.0) * reach;
            // Things drift with the current: most of them sit near it.
            let x = (px + wander) * 0.5 + pull * 0.5;
            let x = x.clamp(px - reach, px + reach);
            let kind = self.catch_kind(level);
            let id = self.push_thing(kind, x, beat);
            if kind == Kind::Trinket {
                let name = self.pick_trinket();
                if let Some(t) = self.plan.iter_mut().find(|t| t.id == id) {
                    t.trinket = name;
                    if name.is_none() {
                        t.kind = Kind::Petal;
                    }
                }
            }
            px = x;
            pb = beat;
        }
        if let Some(beat) = hazard_beat {
            self.plan_hazard(beat, level, px);
        }
        // A gate now and then, between catches, for points.
        if bar % 6 == 3 && level > 0.05 {
            let x = self.current_at((first + 1) as f64);
            if !beats.contains(&1) {
                self.push_thing(Kind::Gate, x, first + 1);
            }
        }
    }

    fn catch_kind(&mut self, level: f64) -> Kind {
        let roll = self.rng.unit();
        let trinket_chance = if self.course.trinkets_from.is_some() {
            0.025
        } else {
            0.0
        };
        if roll < trinket_chance {
            return Kind::Trinket;
        }
        if self.course.lanterns && roll < trinket_chance + 0.05 {
            return Kind::Lantern;
        }
        let _ = level;
        let r = self.rng.unit();
        if r < 0.12 {
            Kind::Scrap
        } else if r < 0.56 {
            Kind::Petal
        } else {
            Kind::Leaf
        }
    }

    /// A trinket from this course's stream, new ones first.
    fn pick_trinket(&mut self) -> Option<&'static str> {
        let stream = self.course.trinkets_from?;
        let all: Vec<&'static crate::shelf::Trinket> = crate::shelf::TRINKETS
            .iter()
            .filter(|t| t.stream == stream)
            .collect();
        let planned: Vec<&'static str> = self
            .plan
            .iter()
            .chain(self.things.iter())
            .filter_map(|t| t.trinket)
            .chain(self.trinkets.iter().copied())
            .collect();
        let fresh: Vec<&&crate::shelf::Trinket> = all
            .iter()
            .filter(|t| !self.shelf.iter().any(|s| s == t.id) && !planned.contains(&t.id))
            .collect();
        if let Some(t) = fresh.first() {
            return Some(t.id);
        }
        let any: Vec<&&crate::shelf::Trinket> =
            all.iter().filter(|t| !planned.contains(&t.id)).collect();
        if any.is_empty() {
            None
        } else {
            let i = self.rng.below(any.len() as u32) as usize;
            Some(any[i].id)
        }
    }

    fn plan_hazard(&mut self, beat: u32, level: f64, path: f64) {
        let mut choices: Vec<Kind> = vec![Kind::Stone];
        for &h in &self.course.hazards {
            // A stream's own twist comes up twice as often as stones.
            choices.push(h);
            choices.push(h);
        }
        if self.course.parade {
            choices.push(Kind::Duck);
            choices.push(Kind::Duck);
        }
        let kind = *self.rng.pick(&choices);
        match kind {
            Kind::Stone => {
                // A stone sits off the path, on whichever side has more room.
                let side = if path > 0.0 { -1.0 } else { 1.0 };
                let x = path + side * (1.6 + self.rng.unit() * 0.8);
                self.push_thing(Kind::Stone, x, beat);
            }
            Kind::Falls => {
                let x = self.current_at(beat as f64);
                self.push_thing(Kind::Falls, x, beat);
            }
            Kind::Mill => {
                let side: i8 = if self.rng.chance(0.5) { -1 } else { 1 };
                self.push_thing(Kind::Mill, side as f64 * 2.4, beat);
                self.last_mut().side = side;
            }
            Kind::Duck => {
                let from_left = self.rng.chance(0.5);
                let from = if from_left { -(HALF - 0.5) } else { HALF - 0.5 };
                let travel = 2.0 + 2.5 * level;
                let to = from + if from_left { travel } else { -travel };
                self.push_thing(Kind::Duck, to, beat);
                let t = self.last_mut();
                t.from = from;
            }
            _ => {}
        }
    }

    // --- play -------------------------------------------------------------

    /// Hears what has come into earshot.
    fn hear(&mut self) {
        let now = self.beat_pos().floor() as u32;
        while let Some(first) = self.plan.first() {
            if first.heard > now {
                break;
            }
            let t = self.plan.remove(0);
            if self.introduce && !self.met.contains(&t.kind) {
                self.met.push(t.kind);
                self.events.push(Event::Introduce(t.kind));
            }
            self.events.push(Event::Heard {
                id: t.id,
                kind: t.kind,
            });
            match t.kind {
                Kind::Gate => self.gates_total += 1,
                Kind::Falls => self.falls_total += 1,
                Kind::Mill => self.mills_total += 1,
                Kind::Duck => self.ducks_total += 1,
                _ => {}
            }
            self.things.push(t);
        }
    }

    fn add_points(&mut self, base: u64) -> u64 {
        if !self.course.mode.scored() {
            return 0;
        }
        let points = base * self.multiplier() as u64;
        self.score += points;
        if let Some(best) = self.best {
            if !self.said_best && best > 0 && self.score > best {
                self.said_best = true;
                self.events.push(Event::NewBest);
            }
        }
        points
    }

    fn bump(&mut self, hazard: Option<Kind>, x: f64) {
        if !self.course.mode.bumps_count() {
            self.shake = BUMP_SHAKE * 0.5;
            self.events.push(Event::Bump {
                hazard,
                x,
                lost_combo: 0,
                soggy: 0,
            });
            return;
        }
        let before = self.combo;
        self.combo = self.boat.after_bump(self.combo);
        self.bumps += 1;
        self.shake = BUMP_SHAKE;
        if self.course.mode == Mode::Endless {
            self.soggy += 1;
        }
        self.events.push(Event::Bump {
            hazard,
            x,
            lost_combo: before - self.combo,
            soggy: self.soggy,
        });
        if self.course.mode == Mode::Endless && self.soggy >= SOGGY_SINKS {
            self.events.push(Event::Sank);
            self.phase = Phase::Over;
        }
    }

    /// Settles everything arriving on `beat`.
    fn arrive(&mut self, beat: u32) {
        let x = self.x;
        let arriving: Vec<Thing> = self
            .things
            .iter()
            .filter(|t| t.arrive == beat)
            .cloned()
            .collect();
        self.things.retain(|t| t.arrive != beat);
        for t in arriving {
            if self.phase == Phase::Over {
                break;
            }
            let tx = t.x_at(beat as f64);
            let dx = (tx - x).abs();
            match t.kind {
                k if k.is_catch() => {
                    self.offered += 1;
                    if dx <= self.boat.reach() {
                        let clean = dx <= self.clean_reach();
                        self.combo += 1;
                        self.best_combo = self.best_combo.max(self.combo);
                        self.caught += 1;
                        let before = self.multiplier();
                        let base = k.points() * if clean { 2 } else { 1 };
                        if clean {
                            self.clean += 1;
                        }
                        let points = self.add_points(base);
                        if k == Kind::Scrap {
                            self.scraps += 1;
                        }
                        if let Some(name) = t.trinket {
                            self.trinkets.push(name);
                        }
                        self.events.push(Event::Caught {
                            id: t.id,
                            kind: k,
                            x: tx,
                            clean,
                            points,
                            combo: self.combo,
                            trinket: t.trinket,
                        });
                        if self.multiplier() > before {
                            self.events.push(Event::Multiplier(self.multiplier()));
                        }
                        if self.course.mode == Mode::Endless
                            && self.soggy > 0
                            && self.caught.is_multiple_of(DRY_EVERY)
                        {
                            self.soggy -= 1;
                            self.events.push(Event::Dried { soggy: self.soggy });
                        }
                    } else {
                        self.events.push(Event::Missed {
                            id: t.id,
                            kind: k,
                            x: tx,
                        });
                    }
                }
                Kind::Stone => {
                    if dx <= 0.7 {
                        self.bump(Some(Kind::Stone), tx);
                    } else {
                        self.events.push(Event::Passed {
                            kind: Kind::Stone,
                            x: tx,
                            points: 0,
                        });
                    }
                }
                Kind::Gate => {
                    if dx <= 0.9 {
                        self.gates_through += 1;
                        let points = self.add_points(GATE_POINTS);
                        self.events.push(Event::Through {
                            kind: Kind::Gate,
                            x: tx,
                            points,
                        });
                    } else {
                        self.events.push(Event::Around { x: tx });
                    }
                }
                Kind::Falls => {
                    if dx <= 1.2 {
                        self.falls_through += 1;
                        let points = self.add_points(GATE_POINTS);
                        self.events.push(Event::Through {
                            kind: Kind::Falls,
                            x: tx,
                            points,
                        });
                    } else {
                        self.bump(Some(Kind::Falls), tx);
                    }
                }
                Kind::Mill => {
                    let blocked = (t.side < 0 && x < 0.3) || (t.side > 0 && x > -0.3);
                    if blocked {
                        self.bump(Some(Kind::Mill), tx);
                    } else {
                        self.mills_passed += 1;
                        self.events.push(Event::Passed {
                            kind: Kind::Mill,
                            x: tx,
                            points: 0,
                        });
                    }
                }
                Kind::Duck => {
                    if dx <= 0.9 {
                        self.ducks_bumped += 1;
                        self.bump(Some(Kind::Duck), tx);
                    } else {
                        let points = if self.course.parade {
                            self.add_points(GATE_POINTS / 2)
                        } else {
                            0
                        };
                        self.events.push(Event::Passed {
                            kind: Kind::Duck,
                            x: tx,
                            points,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// One beat of riding the current, or not.
    fn flow(&mut self, beat: u32) {
        let riding = (self.x - self.current_at(beat as f64)).abs() <= CURRENT_HALF;
        if riding != self.riding {
            self.riding = riding;
            self.events.push(Event::Current(riding));
        }
        if riding {
            self.flow_beats += 1;
            self.add_points(FLOW_POINTS);
        }
    }

    /// Moves the boat by the steering for `dt` seconds.
    fn steer_boat(&mut self, dt: f64) {
        let top = self.boat.steer() * if self.shake > 0.0 { 0.5 } else { 1.0 };
        let want = self.steer as f64 * top;
        // The boat answers quickly but not at once, and drifts to a stop.
        let rate = if self.steer == 0 { 6.0 } else { 10.0 };
        let k = (rate * dt).min(1.0);
        self.vx += (want - self.vx) * k;
        self.x += self.vx * dt;
        let edge = HALF - BANK_MARGIN;
        if self.x.abs() > edge {
            let side = self.x.signum();
            self.x = side * (edge - 0.3);
            self.vx = -self.vx * 0.3;
            if self.shake <= 0.0 {
                self.bump(None, side * HALF);
            }
        }
        self.shake = (self.shake - dt).max(0.0);
    }

    /// Advances the run by `dt` seconds.
    pub fn update(&mut self, dt: f64) {
        match self.phase {
            Phase::LeadIn(left) => {
                let left = left - dt;
                if left <= 0.0 {
                    self.phase = Phase::Playing;
                    self.events.push(Event::Go);
                } else {
                    self.phase = Phase::LeadIn(left);
                }
                return;
            }
            Phase::Ending(left) => {
                let left = left - dt;
                self.phase = if left <= 0.0 {
                    Phase::Over
                } else {
                    Phase::Ending(left)
                };
                return;
            }
            Phase::Over => return,
            Phase::Playing => {}
        }
        self.time += dt;
        self.steer_boat(dt);
        if self.phase == Phase::Over {
            return;
        }
        self.plan_ahead();
        self.hear();
        let now = self.beat_pos().floor() as u32;
        while self.beat < now && self.phase == Phase::Playing {
            self.beat += 1;
            let beat = self.beat;
            self.arrive(beat);
            self.flow(beat);
            if let Some(total) = self.total_beats() {
                if beat >= total {
                    self.events.push(Event::Mouth);
                    self.phase = Phase::Ending(MOUTH_TAIL);
                }
            }
        }
    }

    // --- words ----------------------------------------------------------

    /// "on your left", "just right", "straight ahead".
    pub fn side_words(&self, x: f64) -> &'static str {
        let dx = x - self.x;
        if dx.abs() <= self.clean_reach() {
            "straight ahead"
        } else if dx.abs() <= self.boat.reach() {
            if dx < 0.0 {
                "a touch left"
            } else {
                "a touch right"
            }
        } else if dx.abs() <= 2.5 {
            if dx < 0.0 {
                "to your left"
            } else {
                "to your right"
            }
        } else if dx < 0.0 {
            "far left"
        } else {
            "far right"
        }
    }

    /// The next thing to arrive, if any is in earshot.
    pub fn next_thing(&self) -> Option<&Thing> {
        self.things.iter().min_by_key(|t| (t.arrive, t.id))
    }

    /// The first layer of S: the combo and anything pressing, then a
    /// second layer with the score and how far down the stream.
    pub fn status_layers(&self) -> Vec<String> {
        let mut first = Vec::new();
        if self.course.mode == Mode::Endless && self.soggy > 0 {
            first.push(format!("Soggy, {} of {}.", self.soggy, SOGGY_SINKS));
        }
        let (side, d) = self.nearest_bank();
        if d <= BANK_CLOSE {
            first.push(format!(
                "Bank close on your {}.",
                if side < 0 { "left" } else { "right" }
            ));
        }
        if self.course.mode.scored() {
            first.push(format!(
                "Combo {}, times {}.",
                self.combo,
                self.multiplier()
            ));
        } else {
            first.push(format!("{} caught.", self.caught));
        }
        let mut second = Vec::new();
        if self.course.mode.scored() {
            second.push(format!("Score {}.", self.score));
        }
        match self.progress() {
            Some(p) => second.push(format!("{}, {}.", self.course.mode.label(), way_down(p))),
            None => second.push(format!("{} metres down the river.", self.metres())),
        }
        if let Some(best) = self.best {
            second.push(format!("Best {best}."));
        }
        second.push(if self.riding {
            "Riding the current.".to_string()
        } else {
            format!("The current is {}.", self.side_words(self.current()))
        });
        vec![first.join(" "), second.join(" ")]
    }

    /// H: where the current is and what is coming.
    pub fn hint(&self) -> String {
        if let Phase::LeadIn(_) = self.phase {
            return "The boat is on the water. The stream starts in a moment.".to_string();
        }
        let current = if self.riding {
            "You are in the current.".to_string()
        } else {
            format!("Current {}.", self.side_words(self.current()))
        };
        let next = match self.next_thing() {
            None => "Nothing coming yet.".to_string(),
            Some(t) => {
                let x = t.x_at(self.beat_pos());
                match t.kind {
                    Kind::Mill => format!(
                        "Mill wheel on the {} side: keep {}.",
                        if t.side < 0 { "left" } else { "right" },
                        if t.side < 0 { "right" } else { "left" }
                    ),
                    Kind::Stone => format!("Stone {}: steer clear.", self.side_words(x)),
                    Kind::Duck => format!("Duck {}: steer clear.", self.side_words(x)),
                    Kind::Falls => format!("Waterfall {}: aim for the rush.", self.side_words(x)),
                    Kind::Gate => format!("Gate {}.", self.side_words(x)),
                    k => format!("{} {}.", sentence_case(k.name()), self.side_words(x)),
                }
            }
        };
        format!("{current} {next}")
    }
}

/// "a quarter of the way down", "nearly at the mouth".
pub fn way_down(p: f64) -> &'static str {
    if p < 0.1 {
        "just setting off"
    } else if p < 0.35 {
        "a quarter of the way down"
    } else if p < 0.6 {
        "halfway down"
    } else if p < 0.85 {
        "three quarters of the way down"
    } else {
        "nearly at the mouth"
    }
}

/// "Paper scrap" from "paper scrap".
pub fn sentence_case(s: &str) -> String {
    hearken::text::sentence_start(s)
}

/// "1 star", "3 stars".
pub fn stars_phrase(n: u32) -> String {
    if n == 1 {
        "1 star".to_string()
    } else {
        format!("{n} stars")
    }
}
