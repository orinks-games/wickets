//! The rules of a crossing, in plain structs driven by `dt`.
//!
//! Your balance is `lean`, from -1 (falling off to the left) to +1
//! (falling off to the right). The wire is an inverted pendulum: a lean
//! grows by itself unless you shift your weight back, small nudges and the
//! wire's sway push you about, gusts push hard from one side after a
//! spoken, sounded warning, and pigeons bounce the wire when they land.
//! Steps are taken one at a time; a step while leaning past the safe line
//! wobbles you further the way you lean.

use hearken::text::sentence_start;
use parlor_core::{Period, Rng};

use crate::cues;

/// Steps are safe inside this lean.
pub const SAFE_LEAN: f64 = 0.3;
/// Inside this lean you are balanced: the steady chime sounds.
pub const STEADY_LEAN: f64 = 0.15;
/// Past this lean the danger alarm sounds and is spoken.
pub const DANGER_LEAN: f64 = 0.6;
/// How much one weight shift changes your sway.
pub const SHIFT: f64 = 0.3;
/// How fast sway dies away by itself, per second.
pub const DAMPING: f64 = 0.9;
/// The shortest time between two steps.
pub const STEP_COOLDOWN: f64 = 0.45;
/// No gust is ever warned less than this far ahead, in seconds.
pub const MIN_WARNING: f64 = 2.0;
/// The first gust of a run comes with a spoken explanation: it is warned
/// at least this far ahead, so the explanation is heard before the riser.
pub const TEACH_WARNING: f64 = 8.0;
/// The rising whoosh's length: it climbs to the moment the gust hits.
pub const RISER: f64 = 2.0;
/// How long a gust pushes.
pub const BLOW_TIME: f64 = 1.2;
/// How long the drop to the net and the climb back take.
pub const FALL_TIME: f64 = 3.0;
/// The pause on a rooftop after a crossing.
pub const CROSSED_TIME: f64 = 3.5;
/// The calm at the start of a wire and after a fall: nothing pushes you.
pub const CALM_TIME: f64 = 2.5;
/// How often the far rooftop's beacon chimes.
pub const BEACON_EVERY: f64 = 3.0;
/// Crouching scales the wire's pull and a gust's push by these.
pub const CROUCH_PULL: f64 = 0.5;
pub const CROUCH_GUST: f64 = 0.4;
/// The number of rooftop crossings in the skyline.
pub const SKYLINE_LEN: u32 = 10;

/// A side of the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// -1 for left, +1 for right: the direction of a lean.
    pub fn sign(self) -> f64 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// The side a lean of `x` is toward.
    pub fn of(x: f64) -> Side {
        if x < 0.0 {
            Side::Left
        } else {
            Side::Right
        }
    }

    fn random(rng: &mut Rng) -> Side {
        if rng.chance(0.5) {
            Side::Left
        } else {
            Side::Right
        }
    }
}

/// How the wind blows on a wire.
#[derive(Clone, Debug, PartialEq)]
pub struct Gusts {
    /// Seconds into the wire before the first warning.
    pub first: f64,
    /// Seconds between one gust ending and the next warning.
    pub gap: (f64, f64),
    /// How hard a gust pushes.
    pub strength: f64,
    /// Seconds from the spoken warning to the push.
    pub warning: f64,
}

/// How often pigeons land.
#[derive(Clone, Debug, PartialEq)]
pub struct Pigeons {
    pub first: f64,
    pub gap: (f64, f64),
}

/// One wire between two rooftops.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    /// 1 to 10 on the skyline; 0 for the training wire and challenges'
    /// special spans.
    pub number: u32,
    pub from: &'static str,
    pub to: &'static str,
    pub floors: u32,
    pub steps: u32,
    /// How fast a lean grows by itself.
    pub instability: f64,
    /// The size of the small random nudges.
    pub drift: f64,
    /// How hard the wire sways at mid-span.
    pub sway: f64,
    /// Seconds for one sway back and forth.
    pub sway_period: f64,
    pub gusts: Option<Gusts>,
    pub pigeons: Option<Pigeons>,
}

impl Level {
    /// "from the bakery to the laundromat".
    pub fn route(&self) -> String {
        format!("from {} to {}", self.from, self.to)
    }

    /// The par time: finish under it for a time bonus.
    pub fn par(&self) -> f64 {
        self.steps as f64 * 1.3 + 6.0
    }

    /// The board name of this wire's best time.
    pub fn time_board(&self) -> String {
        format!("crossing {}", self.number)
    }
}

/// The rooftops of the skyline, in order: crossing n goes from the n-th to
/// the next.
pub const ROOFTOPS: [&str; 11] = [
    "the bakery",
    "the laundromat",
    "the bookshop",
    "the old cinema",
    "the Grand Hotel",
    "the clock tower",
    "the bank",
    "the office block",
    "the department store",
    "the glass tower",
    "the Skyline Spire",
];

/// How high each skyline wire is strung.
pub const FLOORS: [u32; 10] = [3, 5, 7, 10, 14, 18, 23, 28, 34, 40];

/// Skyline crossing `n` (1 to 10); `night` is the harder night skyline.
pub fn skyline_level(n: u32, night: bool) -> Level {
    let n = n.clamp(1, SKYLINE_LEN);
    let d = (n - 1) as f64 / (SKYLINE_LEN - 1) as f64;
    let steps = 12 + 4 * (n - 1);
    let mut level = Level {
        number: n,
        from: ROOFTOPS[n as usize - 1],
        to: ROOFTOPS[n as usize],
        floors: FLOORS[n as usize - 1],
        steps,
        instability: 0.6 + 0.8 * d,
        drift: 0.06 + 0.1 * d,
        sway: 0.04 + 0.006 * steps as f64,
        sway_period: 4.0 - 1.0 * d,
        gusts: Some(Gusts {
            first: 1.5 + 1.5 * d,
            gap: (9.0 - 3.0 * d, 14.0 - 5.0 * d),
            strength: 0.6 + 0.6 * d,
            warning: 3.0 - 0.8 * d,
        }),
        pigeons: (n >= 2).then_some(Pigeons {
            first: 9.0 - 3.0 * d,
            gap: (15.0 - 5.0 * d, 22.0 - 7.0 * d),
        }),
    };
    if night {
        level.instability *= 1.2;
        level.drift *= 1.25;
        if let Some(g) = level.gusts.as_mut() {
            g.strength *= 1.15;
            g.warning = (g.warning - 0.3).max(MIN_WARNING);
            g.gap = (g.gap.0 * 0.85, g.gap.1 * 0.85);
        }
        if let Some(p) = level.pigeons.as_mut() {
            p.gap = (p.gap.0 * 0.85, p.gap.1 * 0.85);
        }
    }
    level
}

/// The low practice wire.
pub fn training_level() -> Level {
    Level {
        number: 0,
        from: "the garden shed",
        to: "the garage roof",
        floors: 1,
        steps: 14,
        instability: 0.3,
        drift: 0.03,
        sway: 0.02,
        sway_period: 4.0,
        gusts: Some(Gusts {
            first: 0.5,
            gap: (40.0, 50.0),
            strength: 0.5,
            warning: 3.5,
        }),
        pigeons: Some(Pigeons {
            first: 0.5,
            gap: (40.0, 50.0),
        }),
    }
}

/// The monthly event's one immense wire.
pub fn grand_span() -> Level {
    Level {
        number: 0,
        from: "the Northgate tower",
        to: "the Southgate tower",
        floors: 60,
        steps: 60,
        instability: 1.25,
        drift: 0.13,
        sway: 0.4,
        sway_period: 3.5,
        gusts: Some(Gusts {
            first: 6.0,
            gap: (8.0, 11.0),
            strength: 1.15,
            warning: 2.6,
        }),
        pigeons: Some(Pigeons {
            first: 12.0,
            gap: (14.0, 20.0),
        }),
    }
}

/// A rule laid over a challenge's wires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    None,
    /// Gusts come twice as often.
    Gale,
    /// Pigeons land three times as often.
    Flock,
    /// The wire sways twice as hard.
    Swaying,
    /// Every step slips a little to one side.
    Slick,
    /// The wind changes side with every gust (the monthly Grand Span).
    Alternating,
}

impl Modifier {
    /// The modifiers a daily or weekly challenge picks from.
    pub const DAILY: [Modifier; 4] = [
        Modifier::Gale,
        Modifier::Flock,
        Modifier::Swaying,
        Modifier::Slick,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Modifier::None => "No twist",
            Modifier::Gale => "Gale day",
            Modifier::Flock => "Pigeon parade",
            Modifier::Swaying => "Loose wire",
            Modifier::Slick => "Slick wire",
            Modifier::Alternating => "Changing winds",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Modifier::None => "No twist: the standard skyline wires.",
            Modifier::Gale => {
                "Gale day: gusts come twice as often. Every one is still warned in time."
            }
            Modifier::Flock => "Pigeon parade: pigeons land three times as often.",
            Modifier::Swaying => "Loose wire: the wire sways twice as hard at mid-span.",
            Modifier::Slick => {
                "Slick wire: every step slips you a little to one side, so settle after each one."
            }
            Modifier::Alternating => {
                "Changing winds: each gust comes from the opposite side to the last."
            }
        }
    }

    /// Lays the modifier over a wire.
    pub fn apply(self, level: &mut Level) {
        match self {
            Modifier::Gale => {
                if let Some(g) = level.gusts.as_mut() {
                    g.gap = (g.gap.0 * 0.5, g.gap.1 * 0.5);
                    g.first *= 0.7;
                }
            }
            Modifier::Flock => {
                let p = level.pigeons.get_or_insert(Pigeons {
                    first: 6.0,
                    gap: (15.0, 20.0),
                });
                p.gap = (p.gap.0 / 3.0, p.gap.1 / 3.0);
                p.first *= 0.5;
            }
            Modifier::Swaying => level.sway *= 2.0,
            Modifier::None | Modifier::Slick | Modifier::Alternating => {}
        }
    }
}

/// What kind of run this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The low practice wire: explained as it goes, no lives lost.
    Training,
    /// All ten crossings with three lives.
    Skyline,
    /// The skyline at night: harder, unlocked by crossing the skyline.
    Night,
    /// One skyline crossing, for its best time.
    Single(u32),
    /// A challenge.
    Challenge(Period),
}

impl Mode {
    /// The best-score board this mode saves to.
    pub fn board(self) -> String {
        match self {
            Mode::Training => "training".into(),
            Mode::Skyline => "skyline".into(),
            Mode::Night => "night".into(),
            Mode::Single(n) => format!("single {n}"),
            Mode::Challenge(p) => format!("challenge {}", p.label()),
        }
    }

    pub fn name(self) -> String {
        match self {
            Mode::Training => "Practice wire".into(),
            Mode::Skyline => "Skyline run".into(),
            Mode::Night => "Night skyline".into(),
            Mode::Single(n) => format!("Crossing {n}"),
            Mode::Challenge(p) => format!("{} challenge", p.label()),
        }
    }
}

/// The wires, lives and rule of one run.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub mode: Mode,
    pub levels: Vec<Level>,
    pub lives: u32,
    pub modifier: Modifier,
}

impl Plan {
    /// The plan of a non-challenge mode.
    pub fn of(mode: Mode) -> Plan {
        let (levels, lives) = match mode {
            Mode::Training => (vec![training_level()], 3),
            Mode::Skyline => (
                (1..=SKYLINE_LEN).map(|n| skyline_level(n, false)).collect(),
                3,
            ),
            Mode::Night => (
                (1..=SKYLINE_LEN).map(|n| skyline_level(n, true)).collect(),
                3,
            ),
            Mode::Single(n) => (vec![skyline_level(n, false)], 3),
            Mode::Challenge(p) => return Plan::challenge(p, &mut Rng::seeded(0)),
        };
        Plan {
            mode,
            levels,
            lives,
            modifier: Modifier::None,
        }
    }

    /// A challenge's plan, built from the challenge's own random source so
    /// every player gets the same one.
    pub fn challenge(period: Period, rng: &mut Rng) -> Plan {
        let (mut levels, lives, modifier) = match period {
            Period::Daily => {
                let n = 3 + rng.below(4);
                let m = *rng.pick(&Modifier::DAILY);
                (vec![skyline_level(n, false)], 3, m)
            }
            Period::Weekly => {
                let a = 5 + rng.below(2);
                let b = 7 + rng.below(2);
                let c = 9 + rng.below(2);
                let m = *rng.pick(&Modifier::DAILY);
                (
                    vec![
                        skyline_level(a, false),
                        skyline_level(b, false),
                        skyline_level(c, false),
                    ],
                    2,
                    m,
                )
            }
            Period::Monthly => (vec![grand_span()], 1, Modifier::Alternating),
        };
        for level in &mut levels {
            modifier.apply(level);
        }
        Plan {
            mode: Mode::Challenge(period),
            levels,
            lives,
            modifier,
        }
    }

    /// One sentence on what the run is: the wires and the lives.
    pub fn summary(&self) -> String {
        let lives = match self.lives {
            1 => "one life".to_string(),
            n => format!("{n} lives"),
        };
        match self.levels.as_slice() {
            [one] => format!(
                "One crossing {}, {} up, {}, with {lives}.",
                one.route(),
                plural(one.floors, "floor"),
                plural(one.steps, "step")
            ),
            many => format!(
                "{} crossings, {} to {}, with {lives}.",
                many.len(),
                many[0].from,
                many[many.len() - 1].to
            ),
        }
    }
}

/// A gust on its way or blowing.
#[derive(Clone, Debug, PartialEq)]
pub struct Gust {
    /// The side the wind comes from: it pushes you toward the other.
    pub side: Side,
    /// Seconds until it hits; zero once it blows.
    pub warn_left: f64,
    /// Seconds of push left once it hits.
    pub blow_left: f64,
    pub strength: f64,
    riser: bool,
}

impl Gust {
    pub fn blowing(&self) -> bool {
        self.warn_left <= 0.0
    }

    /// The push on your lean while it blows: away from the wind.
    pub fn force(&self) -> f64 {
        if self.blowing() {
            -self.side.sign() * self.strength
        } else {
            0.0
        }
    }
}

/// A pigeon sitting on the wire ahead.
#[derive(Clone, Debug, PartialEq)]
pub struct Pigeon {
    /// The step it sits on: stepping onto it startles it.
    pub at: u32,
    /// The side of the wire it perches toward.
    pub side: Side,
    /// Seconds until it flies off by itself.
    pub stay: f64,
}

/// Where a run is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Walking,
    /// Dropped to the net; climbing back when `left` runs out.
    Falling {
        left: f64,
    },
    /// On the far rooftop; the next wire starts when `left` runs out.
    Crossed {
        left: f64,
    },
    Over {
        won: bool,
    },
}

/// What the screen should play or say.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    Say(String),
    Queue(String),
    /// A sound from the middle.
    Cue(&'static str),
    /// A sound to one side: `x` metres right (negative is left).
    At(&'static str, f32),
    /// A sound `y` metres straight ahead.
    Ahead(&'static str, f32),
    /// A street sound far below, `x` metres to the side.
    Below(&'static str, f32),
    /// Music: a stinger.
    Music(&'static str),
    /// A new wire began: the street and wind change with its height.
    NewWire,
}

/// The controls by name, for spoken instructions.
#[derive(Clone, Debug, PartialEq)]
pub struct Keys {
    pub left: String,
    pub right: String,
    pub step: String,
    pub shoo: String,
    pub crouch: String,
    pub stand: String,
    pub status: String,
    pub hint: String,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            left: "Left".into(),
            right: "Right".into(),
            step: "Space".into(),
            shoo: "Tab".into(),
            crouch: "Down".into(),
            stand: "Up".into(),
            status: "S".into(),
            hint: "H".into(),
        }
    }
}

/// How a lean sounds in words.
pub fn lean_words(lean: f64) -> String {
    let a = lean.abs();
    let side = Side::of(lean).name();
    if a < STEADY_LEAN {
        "balanced".to_string()
    } else if a < 0.35 {
        format!("leaning slightly {side}")
    } else if a < DANGER_LEAN {
        format!("leaning {side}")
    } else if a < 0.8 {
        format!("leaning hard {side}")
    } else {
        format!("about to fall {side}")
    }
}

fn plural(n: u32, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

/// "1 life", "3 lives".
pub fn lives(n: u32) -> String {
    if n == 1 {
        "1 life".to_string()
    } else {
        format!("{n} lives")
    }
}

/// One run: a plan of wires walked in order.
#[derive(Clone, Debug)]
pub struct Run {
    pub mode: Mode,
    pub modifier: Modifier,
    pub levels: Vec<Level>,
    /// The wire being walked.
    pub index: usize,
    pub lives: u32,
    pub score: u64,
    /// Falls this run, and on this wire.
    pub falls: u32,
    pub wire_falls: u32,
    pub phase: Phase,
    /// -1 is falling off left, +1 falling off right.
    pub lean: f64,
    /// How fast the lean is changing.
    pub vel: f64,
    /// Steps taken on this wire.
    pub pos: u32,
    pub crouched: bool,
    /// Seconds on this wire, and in the whole run.
    pub t: f64,
    pub total_t: f64,
    pub gust: Option<Gust>,
    pub pigeon: Option<Pigeon>,
    /// Crossings finished, with their times: (crossing number, seconds).
    pub times: Vec<(u32, f64)>,
    pub crossed: u32,
    /// The player's controls by name, set by the screen.
    pub keys: Keys,
    /// The last prompt, for the repeat key.
    pub last: String,
    step_cool: f64,
    calm: f64,
    next_nudge: f64,
    next_gust: f64,
    next_pigeon: f64,
    next_creak: f64,
    next_beacon: f64,
    next_street: f64,
    zone: u8,
    wind_side: Side,
    taught: u8,
    rng: Rng,
    notes: Vec<Note>,
}

const TAUGHT_GUST: u8 = 1;
const TAUGHT_PIGEON: u8 = 2;
const TAUGHT_DANGER: u8 = 4;
const TAUGHT_STEADY: u8 = 8;
const TAUGHT_CROUCH: u8 = 16;
const TAUGHT_SHOO: u8 = 32;

impl Run {
    pub fn new(plan: Plan, rng: Rng) -> Run {
        let mut run = Run {
            mode: plan.mode,
            modifier: plan.modifier,
            levels: plan.levels,
            index: 0,
            lives: plan.lives,
            score: 0,
            falls: 0,
            wire_falls: 0,
            phase: Phase::Walking,
            lean: 0.0,
            vel: 0.0,
            pos: 0,
            crouched: false,
            t: 0.0,
            total_t: 0.0,
            gust: None,
            pigeon: None,
            times: Vec::new(),
            crossed: 0,
            keys: Keys::default(),
            last: String::new(),
            step_cool: 0.0,
            calm: 0.0,
            next_nudge: 0.0,
            next_gust: 0.0,
            next_pigeon: 0.0,
            next_creak: 0.0,
            next_beacon: 0.0,
            next_street: 0.0,
            zone: 0,
            wind_side: Side::Left,
            taught: 0,
            rng,
            notes: Vec::new(),
        };
        run.start_wire();
        run
    }

    /// A run of `mode` (not a challenge) seeded with `rng`.
    pub fn of(mode: Mode, rng: Rng) -> Run {
        Run::new(Plan::of(mode), rng)
    }

    pub fn level(&self) -> &Level {
        &self.levels[self.index.min(self.levels.len() - 1)]
    }

    pub fn training(&self) -> bool {
        self.mode == Mode::Training
    }

    pub fn drain_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    /// Whether the run has ended.
    pub fn over(&self) -> Option<bool> {
        match self.phase {
            Phase::Over { won } => Some(won),
            _ => None,
        }
    }

    fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        self.notes.push(Note::Say(text));
    }

    fn queue(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last = format!("{} {text}", self.last).trim().to_string();
        self.notes.push(Note::Queue(text));
    }

    fn range(&mut self, (lo, hi): (f64, f64)) -> f64 {
        lo + (hi - lo) * self.rng.unit()
    }

    /// Sets up the wire at `index`.
    fn start_wire(&mut self) {
        let level = self.level().clone();
        self.lean = 0.0;
        self.vel = 0.0;
        self.pos = 0;
        self.crouched = false;
        self.t = 0.0;
        self.wire_falls = 0;
        self.gust = None;
        self.pigeon = None;
        self.step_cool = 0.0;
        self.calm = CALM_TIME;
        self.zone = 0;
        self.next_nudge = 0.8;
        self.next_gust = level.gusts.as_ref().map_or(f64::INFINITY, |g| g.first);
        self.next_pigeon = level.pigeons.as_ref().map_or(f64::INFINITY, |p| p.first);
        self.next_creak = 1.0;
        self.next_beacon = 1.0;
        self.next_street = 2.0 + 3.0 * self.rng.unit();
        self.wind_side = Side::random(&mut self.rng);
        self.phase = Phase::Walking;
        self.notes.push(Note::NewWire);
        self.notes.push(Note::Cue(cues::START));
        let intro = self.intro();
        if self.index == 0 {
            self.say(intro);
        } else {
            // Queued, so the last wire's arrival report is heard in full.
            self.queue(intro);
        }
    }

    /// What the player hears as a wire begins.
    pub fn intro(&self) -> String {
        let level = self.level();
        let head = match self.mode {
            Mode::Training => "The practice wire".to_string(),
            Mode::Skyline | Mode::Night => {
                format!("Crossing {} of {}", self.index + 1, self.levels.len())
            }
            Mode::Single(n) => format!("Crossing {n}"),
            Mode::Challenge(Period::Monthly) => "The Grand Span".to_string(),
            Mode::Challenge(_) => {
                format!("Challenge wire {} of {}", self.index + 1, self.levels.len())
            }
        };
        let mut text = format!(
            "{head}: {}. {} up, {}.",
            level.route(),
            plural(level.floors, "floor"),
            plural(level.steps, "step")
        );
        if level.pigeons.is_some() && self.mode != Mode::Training {
            text.push_str(" Pigeons roost on this wire.");
        }
        if self.training() {
            text.push_str(&format!(
                " The tone is your balance: centred and low when you are steady. {} and {} shift your weight. {} takes a step.",
                sentence_start(&self.keys.left),
                self.keys.right,
                sentence_start(&self.keys.step)
            ));
        } else {
            text.push_str(&format!(
                " Balanced. {} steps.",
                sentence_start(&self.keys.step)
            ));
        }
        text
    }

    // -- controls ----------------------------------------------------------

    /// Shifts your weight toward `side`.
    pub fn shift(&mut self, side: Side) {
        if self.phase != Phase::Walking {
            self.not_now();
            return;
        }
        self.vel += side.sign() * SHIFT;
        self.notes
            .push(Note::At(cues::SHIFT, (side.sign() * 1.5) as f32));
    }

    /// Takes one step forward.
    pub fn step(&mut self) {
        if self.phase != Phase::Walking {
            self.not_now();
            return;
        }
        if self.crouched {
            self.notes.push(Note::Cue(cues::REFUSED));
            let stand = self.keys.stand.clone();
            self.say(format!(
                "You are crouching. {} stands up to walk.",
                sentence_start(&stand)
            ));
            return;
        }
        if self.step_cool > 0.0 {
            self.notes.push(Note::Cue(cues::REFUSED));
            return;
        }
        if self.shoo_lesson() {
            let pigeon_next = self.pigeon.as_ref().is_some_and(|p| p.at == self.pos + 1);
            if pigeon_next {
                // The practice wire's pigeon waits to be shooed.
                self.notes.push(Note::Cue(cues::REFUSED));
                let shoo = self.keys.shoo.clone();
                self.say(format!(
                    "Pigeon in the way. {} shoos it.",
                    sentence_start(&shoo)
                ));
                return;
            }
            if self.pigeon.is_none() && self.pos + 2 >= self.level().steps {
                self.notes.push(Note::Cue(cues::REFUSED));
                self.say("Wait: a pigeon is coming.");
                return;
            }
        }
        self.step_cool = STEP_COOLDOWN;
        if let Some(p) = self.pigeon.clone() {
            if p.at == self.pos + 1 {
                // It bursts up in your face and knocks you away from it.
                self.pigeon = None;
                self.vel += p.side.other().sign() * 0.55;
                self.notes
                    .push(Note::At(cues::FLAP, (p.side.sign() * 1.0) as f32));
                self.notes.push(Note::Cue(cues::WOBBLE));
                let away = p.side.other().name();
                self.say(format!("Into the pigeon! Knocked {away}."));
                self.schedule_pigeon();
                return;
            }
        }
        let number = self.level().number.max(1) as u64;
        self.pos += 1;
        self.score += 10 * number;
        let a = self.lean.abs();
        if a > SAFE_LEAN {
            self.vel += self.lean.signum() * (0.25 + 0.6 * a);
            self.notes
                .push(Note::At(cues::WOBBLE, (self.lean * 3.0) as f32));
            let side = Side::of(self.lean).name();
            self.say(format!("Wobble! You stepped leaning {side}."));
        } else {
            self.notes.push(Note::Cue(cues::STEP));
        }
        // No foot lands quite square: every step jolts the wire a little.
        let jolt = 0.06 + 1.2 * self.level().drift;
        let side = Side::random(&mut self.rng);
        self.vel += side.sign() * jolt * (0.5 + 0.5 * self.rng.unit());
        if self.modifier == Modifier::Slick {
            let side = Side::random(&mut self.rng);
            self.vel += side.sign() * 0.14;
            self.notes
                .push(Note::At(cues::SLIP, (side.sign() * 1.5) as f32));
        }
        let steps = self.level().steps;
        if self.pos >= steps {
            self.arrive();
            return;
        }
        let left = steps - self.pos;
        if self.pos == steps / 2 && steps >= 8 {
            self.queue("Halfway.");
        } else if left == 3 {
            self.queue("3 steps to the rooftop.");
        } else if left == 1 {
            self.queue("One more step.");
        }
    }

    /// Crouches low: steadier, and gusts push less, but no steps.
    pub fn crouch(&mut self) {
        if self.phase != Phase::Walking {
            self.not_now();
            return;
        }
        if self.crouched {
            let stand = self.keys.stand.clone();
            self.say(format!(
                "Already crouching. {} stands.",
                sentence_start(&stand)
            ));
            return;
        }
        self.crouched = true;
        self.notes.push(Note::Cue(cues::CROUCH));
        if self.taught & TAUGHT_CROUCH == 0 {
            self.taught |= TAUGHT_CROUCH;
            let stand = self.keys.stand.clone();
            self.say(format!(
                "Crouching: steadier in wind. {} stands.",
                sentence_start(&stand)
            ));
        } else {
            // Short, so it does not talk over the wind for long.
            self.say("Crouching.");
        }
    }

    pub fn stand(&mut self) {
        if self.phase != Phase::Walking {
            self.not_now();
            return;
        }
        if !self.crouched {
            self.say("Already standing.");
            return;
        }
        self.crouched = false;
        self.notes.push(Note::Cue(cues::STAND));
        self.say("Standing.");
    }

    /// Shoos a pigeon within two steps.
    pub fn shoo(&mut self) {
        if self.phase != Phase::Walking {
            self.not_now();
            return;
        }
        self.notes.push(Note::Cue(cues::SHOO));
        match self.pigeon.clone() {
            Some(p) if p.at <= self.pos + 2 => {
                self.pigeon = None;
                self.vel += p.side.sign() * 0.15;
                self.notes
                    .push(Note::At(cues::FLAP, (p.side.sign() * 1.0) as f32));
                self.taught |= TAUGHT_SHOO;
                self.say("Shooed.");
                self.schedule_pigeon();
            }
            Some(p) => {
                let d = p.at - self.pos;
                self.say(format!("Too far to shoo: {} ahead.", plural(d, "step")));
            }
            None => self.say("No pigeon to shoo."),
        }
    }

    /// Answers a wire key off the wire. Not kept for R or the results.
    fn not_now(&mut self) {
        let text = match self.phase {
            Phase::Falling { .. } => "Climbing back up.",
            Phase::Crossed { .. } if self.index + 1 < self.levels.len() => {
                "On the roof. Next wire soon."
            }
            Phase::Crossed { .. } => "On the last roof.",
            _ => return,
        };
        self.notes.push(Note::Say(text.to_string()));
    }

    /// The practice wire holds its pigeon until the player has shooed one.
    fn shoo_lesson(&self) -> bool {
        self.mode == Mode::Training && self.taught & TAUGHT_SHOO == 0
    }

    fn schedule_pigeon(&mut self) {
        if let Some(p) = self.level().pigeons.clone() {
            self.next_pigeon = self.range(p.gap);
        }
    }

    // -- time --------------------------------------------------------------

    pub fn update(&mut self, dt: f64) {
        if dt <= 0.0 {
            return;
        }
        let mut left = dt;
        while left > 0.0 {
            let h = left.min(0.02);
            self.tick(h);
            left -= h;
        }
    }

    fn tick(&mut self, dt: f64) {
        match self.phase {
            Phase::Over { .. } => {}
            Phase::Falling { left } => {
                self.t += dt;
                self.total_t += dt;
                let left = left - dt;
                if left > 0.0 {
                    self.phase = Phase::Falling { left };
                } else {
                    self.back_on_wire();
                }
            }
            Phase::Crossed { left } => {
                let left = left - dt;
                if left > 0.0 {
                    self.phase = Phase::Crossed { left };
                } else if self.index + 1 < self.levels.len() {
                    self.index += 1;
                    self.start_wire();
                } else {
                    self.phase = Phase::Over { won: true };
                    self.notes.push(Note::Music(cues::MUSIC_VICTORY));
                    // The arrival was heard; the results open with the
                    // final words, said once there.
                    self.last = self.over_text(true);
                }
            }
            Phase::Walking => self.walk(dt),
        }
    }

    fn walk(&mut self, dt: f64) {
        self.t += dt;
        self.total_t += dt;
        self.step_cool = (self.step_cool - dt).max(0.0);
        let level = self.level().clone();
        let calm = self.calm > 0.0;
        self.calm = (self.calm - dt).max(0.0);

        if !calm {
            // Small random nudges.
            self.next_nudge -= dt;
            if self.next_nudge <= 0.0 {
                let side = Side::random(&mut self.rng);
                let size = level.drift * (0.5 + 0.5 * self.rng.unit());
                self.vel += side.sign() * size;
                self.next_nudge = self.range((0.6, 1.6));
            }
            self.gusts(dt, &level);
            self.pigeons(dt, &level);
        }

        // The wire pulls a lean further; sway, gusts and damping act on it.
        let mid = (std::f64::consts::PI * self.pos as f64 / level.steps.max(1) as f64).sin();
        let sway = if calm {
            0.0
        } else {
            level.sway
                * (0.3 + 0.7 * mid)
                * (std::f64::consts::TAU * self.t / level.sway_period).sin()
        };
        let pull = if calm { 0.0 } else { level.instability };
        let (pull_k, gust_k) = if self.crouched {
            (CROUCH_PULL, CROUCH_GUST)
        } else {
            (1.0, 1.0)
        };
        let gust = self.gust.as_ref().map_or(0.0, |g| g.force());
        let acc = pull * pull_k * self.lean + sway * pull_k + gust * gust_k;
        self.vel += acc * dt;
        self.vel *= (-DAMPING * dt).exp();
        if calm {
            self.vel *= (-3.0 * dt).exp();
        }
        self.vel = self.vel.clamp(-2.0, 2.0);
        self.lean += self.vel * dt;

        // Creaks of the wire: quicker as you sway harder.
        self.next_creak -= dt * (0.6 + self.vel.abs() * 2.5 + sway.abs() * 4.0);
        if self.next_creak <= 0.0 {
            let key = if self.rng.chance(0.5) {
                cues::CREAK_1
            } else {
                cues::CREAK_2
            };
            self.notes.push(Note::At(key, (self.lean * 3.0) as f32));
            self.next_creak = self.range((1.2, 2.2));
        }
        // The far rooftop's beacon, straight ahead.
        self.next_beacon -= dt;
        if self.next_beacon <= 0.0 {
            let ahead = (level.steps - self.pos.min(level.steps)) as f32;
            self.notes
                .push(Note::Ahead(cues::BEACON, ahead.clamp(1.0, 20.0)));
            self.next_beacon = BEACON_EVERY;
        }
        // The street far below.
        self.next_street -= dt;
        if self.next_street <= 0.0 {
            let key = if self.rng.chance(0.75) {
                cues::HORN
            } else {
                cues::SIREN
            };
            let x = (self.rng.unit() * 16.0 - 8.0) as f32;
            self.notes.push(Note::Below(key, x));
            self.next_street = self.range((6.0, 14.0));
        }

        self.zones();
        if self.lean.abs() >= 1.0 {
            self.fall();
        }
    }

    fn gusts(&mut self, dt: f64, level: &Level) {
        let Some(g) = level.gusts.as_ref() else {
            return;
        };
        match self.gust.as_mut() {
            None => {
                // The training wire's gust waits until you are under way,
                // past the tutorial's first 3 steps.
                if self.mode == Mode::Training && self.pos < 4 {
                    return;
                }
                self.next_gust -= dt;
                if self.next_gust <= 0.0 {
                    let side = match self.modifier {
                        Modifier::Alternating => {
                            let side = self.wind_side;
                            self.wind_side = side.other();
                            side
                        }
                        _ => Side::random(&mut self.rng),
                    };
                    let strength = g.strength * (0.8 + 0.4 * self.rng.unit());
                    let teach = self.taught & TAUGHT_GUST == 0
                        && (self.training() || self.mode == Mode::Single(1) || self.index == 0);
                    let mut warning = g.warning.max(MIN_WARNING);
                    if teach {
                        warning = warning.max(TEACH_WARNING);
                    }
                    self.gust = Some(Gust {
                        side,
                        warn_left: warning,
                        blow_left: BLOW_TIME,
                        strength,
                        riser: false,
                    });
                    self.notes
                        .push(Note::At(cues::GUST_ALERT, (side.sign() * 4.0) as f32));
                    let mut text = format!("Gust from the {}!", side.name());
                    if self.taught & TAUGHT_GUST == 0 {
                        self.taught |= TAUGHT_GUST;
                        if teach {
                            text.push_str(&format!(
                                " Lean into it with {}, or crouch with {}.",
                                if side == Side::Left {
                                    &self.keys.left
                                } else {
                                    &self.keys.right
                                },
                                self.keys.crouch
                            ));
                        }
                    }
                    self.say(text);
                }
            }
            Some(gust) => {
                if gust.warn_left > 0.0 {
                    gust.warn_left -= dt;
                    if !gust.riser && gust.warn_left <= RISER {
                        gust.riser = true;
                        let x = (gust.side.sign() * 4.0) as f32;
                        self.notes.push(Note::At(cues::GUST_WARNING, x));
                    }
                    if gust.warn_left <= 0.0 {
                        gust.warn_left = 0.0;
                        let x = (gust.side.sign() * 4.0) as f32;
                        self.notes.push(Note::At(cues::GUST_BLAST, x));
                    }
                } else {
                    gust.blow_left -= dt;
                    if gust.blow_left <= 0.0 {
                        self.gust = None;
                        self.next_gust = self.range(g.gap);
                    }
                }
            }
        }
    }

    fn pigeons(&mut self, dt: f64, level: &Level) {
        if level.pigeons.is_none() {
            return;
        }
        match self.pigeon.as_mut() {
            None => {
                // The training wire's pigeon follows its gust.
                if self.mode == Mode::Training && (self.pos < 4 || self.taught & TAUGHT_GUST == 0) {
                    return;
                }
                // No landing talks over a gust's warning: it waits for calm air.
                if self.gust.is_some() {
                    self.next_pigeon = self.next_pigeon.max(0.5);
                    return;
                }
                self.next_pigeon -= dt;
                if self.next_pigeon <= 0.0 {
                    let mut at = self.pos + 2 + self.rng.below(2);
                    if self.shoo_lesson() {
                        // Always before the far roof, so it must be shooed.
                        at = at.min(level.steps - 1);
                    }
                    if at >= level.steps || at <= self.pos {
                        self.next_pigeon = 4.0;
                        return;
                    }
                    let side = Side::random(&mut self.rng);
                    let stay = self.range((7.0, 11.0));
                    self.pigeon = Some(Pigeon { at, side, stay });
                    self.vel += side.sign() * 0.12;
                    let d = at - self.pos;
                    self.notes
                        .push(Note::At(cues::PIGEON_LAND, (side.sign() * 1.0) as f32));
                    let mut text = format!(
                        "A pigeon lands {} ahead, on your {}.",
                        plural(d, "step"),
                        side.name()
                    );
                    if self.taught & TAUGHT_PIGEON == 0 {
                        self.taught |= TAUGHT_PIGEON;
                        text.push_str(&format!(
                            " {} shoos it from 2 steps.",
                            sentence_start(&self.keys.shoo)
                        ));
                    }
                    self.say(text);
                }
            }
            Some(p) => {
                if self.mode != Mode::Training || self.taught & TAUGHT_SHOO != 0 {
                    p.stay -= dt;
                }
                if p.stay <= 0.0 {
                    let side = p.side;
                    self.pigeon = None;
                    self.vel += side.sign() * 0.08;
                    self.notes
                        .push(Note::At(cues::FLAP, (side.sign() * 1.0) as f32));
                    // Queued: it never cuts off a gust warning.
                    self.queue("The pigeon flies off.");
                    self.schedule_pigeon();
                }
            }
        }
    }

    /// Speaks the balance zones as you cross them.
    fn zones(&mut self) {
        let a = self.lean.abs();
        let zone = if a >= DANGER_LEAN {
            3
        } else if a >= 0.35 {
            2
        } else if a >= STEADY_LEAN {
            1
        } else {
            0
        };
        // A little give, so hovering on a line does not chatter.
        let settled = match (self.zone, zone) {
            (3, 2) if a > DANGER_LEAN - 0.08 => 3,
            (z, 0) if z >= 1 && a > STEADY_LEAN - 0.04 => z.min(1),
            _ => zone,
        };
        if settled == 3 && self.zone < 3 {
            let side = Side::of(self.lean);
            self.notes
                .push(Note::At(cues::DANGER, (side.sign() * 3.0) as f32));
            let fix = if side == Side::Left {
                self.keys.right.clone()
            } else {
                self.keys.left.clone()
            };
            let mut text = format!("Leaning hard {}!", side.name());
            if self.taught & TAUGHT_DANGER == 0 {
                self.taught |= TAUGHT_DANGER;
                text.push_str(&format!(
                    " Near falling. {} shifts back.",
                    sentence_start(&fix)
                ));
            }
            self.say(text);
        }
        if settled == 0 && self.zone >= 2 {
            self.notes.push(Note::Cue(cues::STEADY));
            if self.taught & TAUGHT_STEADY == 0 && self.training() {
                self.taught |= TAUGHT_STEADY;
                let step = self.keys.step.clone();
                self.say(format!("Balanced: safe to step with {step}."));
            }
        }
        self.zone = settled;
    }

    fn fall(&mut self) {
        let side = Side::of(self.lean);
        self.falls += 1;
        self.wire_falls += 1;
        if !self.training() {
            self.lives = self.lives.saturating_sub(1);
        }
        self.gust = None;
        self.crouched = false;
        self.phase = Phase::Falling { left: FALL_TIME };
        self.notes
            .push(Note::At(cues::FALL, (side.sign() * 3.0) as f32));
        self.notes.push(Note::Cue(cues::NET));
        let lives = if self.training() {
            "No lives lost on the practice wire.".to_string()
        } else if self.lives == 0 {
            "No lives left.".to_string()
        } else {
            format!("{} left.", lives(self.lives))
        };
        self.say(format!("Fell {}, into the net. {lives}", side.name()));
        if self.lives == 0 && !self.training() {
            // The run ends in the net: no climb back. The results open
            // with how it ended, after the fall is heard.
            self.phase = Phase::Over { won: false };
            self.notes.push(Note::Music(cues::MUSIC_LOSE));
            self.last = self.over_text(false);
        }
    }

    fn back_on_wire(&mut self) {
        self.lean = 0.0;
        self.vel = 0.0;
        self.calm = CALM_TIME;
        self.zone = 0;
        self.phase = Phase::Walking;
        self.step_cool = 0.0;
        if let Some(g) = self.level().gusts.clone() {
            self.next_gust = self.next_gust.max(g.gap.0 * 0.5);
        }
        self.notes.push(Note::Cue(cues::CLIMB));
        let steps = self.level().steps;
        let step = self.keys.step.clone();
        // Queued, so the fall's lives count is heard in full first.
        self.queue(format!(
            "Back on, step {} of {steps}. {} steps.",
            self.pos,
            sentence_start(&step)
        ));
    }

    fn arrive(&mut self) {
        let level = self.level().clone();
        let n = level.number.max(1) as u64;
        let time_bonus = ((level.par() - self.t).max(0.0) * 5.0) as u64;
        let clean = if self.wire_falls == 0 { 50 * n } else { 0 };
        let bonus = 100 * n + time_bonus + clean;
        self.score += bonus;
        self.crossed += 1;
        self.times.push((level.number, self.t));
        self.gust = None;
        self.pigeon = None;
        self.crouched = false;
        self.phase = Phase::Crossed { left: CROSSED_TIME };
        self.notes.push(Note::Cue(cues::ARRIVE));
        if self.index + 1 < self.levels.len() {
            self.notes.push(Note::Music(cues::MUSIC_CROSSED));
        }
        let clean_text = if clean > 0 { " No falls." } else { "" };
        self.say(format!(
            "Across. You step onto {}. {:.1} seconds, par {:.0}.{clean_text} Bonus {bonus}. Score {}.",
            level.to,
            self.t,
            level.par(),
            self.score
        ));
    }

    fn over_text(&self, won: bool) -> String {
        if won {
            match self.mode {
                Mode::Skyline => format!(
                    "You crossed the whole skyline to {}. Final score {}.",
                    ROOFTOPS[SKYLINE_LEN as usize], self.score
                ),
                Mode::Night => format!("Night skyline complete. Final score {}.", self.score),
                Mode::Training => "Practice wire done. You are ready for the skyline.".to_string(),
                _ => format!("Done. Final score {}.", self.score),
            }
        } else {
            let wire = match self.mode {
                Mode::Single(n) => format!("crossing {n}"),
                Mode::Challenge(_) if self.levels.len() == 1 => "the challenge wire".to_string(),
                Mode::Challenge(_) => {
                    format!("challenge wire {} of {}", self.index + 1, self.levels.len())
                }
                _ if self.levels.len() == 1 => "this wire".to_string(),
                _ => format!("crossing {} of {}", self.index + 1, self.levels.len()),
            };
            format!("Out of lives on {wire}. Final score {}.", self.score)
        }
    }

    // -- speech ------------------------------------------------------------

    /// The status key's report.
    pub fn status_layers(&self) -> Vec<String> {
        let level = self.level();
        let mut first = vec![match self.phase {
            Phase::Falling { .. } => "In the safety net, climbing back.".to_string(),
            Phase::Crossed { .. } => format!("On the rooftop of {}.", level.to),
            Phase::Over { won: true } => "Run complete.".to_string(),
            Phase::Over { won: false } => "Run over.".to_string(),
            Phase::Walking => format!(
                "Step {} of {}, {}{}.",
                self.pos,
                level.steps,
                lean_words(self.lean),
                if self.crouched { ", crouching" } else { "" }
            ),
        }];
        if self.phase == Phase::Walking {
            if let Some(g) = &self.gust {
                if g.blowing() {
                    first.push(format!("Wind blowing from the {}.", g.side.name()));
                } else {
                    first.push(format!(
                        "Gust from the {} in {}.",
                        g.side.name(),
                        plural(g.warn_left.ceil() as u32, "second")
                    ));
                }
            }
            if let Some(p) = &self.pigeon {
                first.push(format!(
                    "Pigeon {} ahead on the {}.",
                    plural(p.at - self.pos, "step"),
                    p.side.name()
                ));
            }
        }
        let mut more = Vec::new();
        if !self.training() {
            more.push(format!("{}.", sentence_start(&lives(self.lives))));
        }
        more.push(format!("Score {}.", self.score));
        more.push(format!(
            "{} on this wire.",
            plural(self.t.round() as u32, "second")
        ));
        more.push(format!(
            "Wire {} of {}, {} up.",
            self.index + 1,
            self.levels.len(),
            plural(level.floors, "floor")
        ));
        vec![
            parlor_core::status::sentences(first),
            parlor_core::status::sentences(more),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        parlor_core::status::sentences(self.status_layers())
    }

    /// The hint key's advice for this moment.
    pub fn hint(&self) -> String {
        self.hint_now()
    }

    fn hint_now(&self) -> String {
        let k = &self.keys;
        match self.phase {
            Phase::Falling { .. } => return "Wait: climbing back on.".into(),
            Phase::Crossed { .. } if self.index + 1 < self.levels.len() => {
                return "Rest: the next wire starts by itself.".into()
            }
            Phase::Crossed { .. } => return "Last wire crossed. Results next.".into(),
            Phase::Over { .. } => return "The run is over.".into(),
            Phase::Walking => {}
        }
        if let Some(g) = &self.gust {
            let into = if g.side == Side::Left {
                &k.left
            } else {
                &k.right
            };
            if g.blowing() {
                return format!("Wind pushing you {}: press {into}.", g.side.other().name());
            }
            return format!(
                "Wind from the {} in {}. Lean into it with {into}, or crouch with {}.",
                g.side.name(),
                plural(g.warn_left.ceil() as u32, "second"),
                k.crouch
            );
        }
        let a = self.lean.abs();
        let drifting = self.lean + self.vel * 0.5;
        if a >= STEADY_LEAN || drifting.abs() > 0.3 {
            let side = Side::of(drifting);
            let fix = if side == Side::Left {
                &k.right
            } else {
                &k.left
            };
            return format!(
                "{} and drifting {}. Press {fix}, then wait for the chime.",
                sentence_start(&lean_words(self.lean)),
                side.name()
            );
        }
        if let Some(p) = &self.pigeon {
            if p.at <= self.pos + 2 {
                return format!(
                    "Pigeon {} ahead. Shoo it with {}.",
                    plural(p.at - self.pos, "step"),
                    k.shoo
                );
            }
        }
        if self.crouched {
            return format!(
                "All calm. {} stands so you can walk.",
                sentence_start(&k.stand)
            );
        }
        format!("Balanced. {} steps forward.", sentence_start(&k.step))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lean_words_cover_both_sides() {
        assert_eq!(lean_words(0.0), "balanced");
        assert_eq!(lean_words(-0.2), "leaning slightly left");
        assert_eq!(lean_words(0.5), "leaning right");
        assert_eq!(lean_words(-0.7), "leaning hard left");
        assert_eq!(lean_words(0.9), "about to fall right");
    }
}
