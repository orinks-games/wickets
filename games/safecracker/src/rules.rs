//! The rules on their own: the dial, the wheels, the guard and the clock.
//!
//! Nothing here speaks or plays a sound. [`Run`] takes the player's moves
//! and the passing of time, and answers with [`Ev`]s that the screen turns
//! into speech and sound, so tests can drive a whole safe by hand.

use std::cell::Cell;

use parlor_core::{Period, Rng};

pub use crate::events::UNLUCKY_NUMBER;

/// Numbers on the dial: 0 to 99.
pub const DIAL: u32 = 100;
/// How far a spin (Up or Down) turns the dial.
pub const SPIN: u32 = 10;
/// Wrong sets before the relocker fires and the safe jams for good.
pub const STRIKES: u32 = 3;
/// Noises the guard hears before he catches you: the first only makes him
/// suspicious.
pub const SUSPICION: u32 = 2;
/// The safes of the campaign.
pub const SAFES: u32 = 10;
/// Seconds a hint costs in time attack.
pub const HINT_COST: f64 = 5.0;
/// The safe that opens time attack once cracked.
pub const ATTACK_OPENS: u32 = 3;
/// How far from the dial a new wheel's number always sits, so there is
/// always some turning to do.
pub const MIN_GAP: u32 = 12;
/// Seconds left at which the time attack clock warns.
pub const WARNINGS: [u32; 3] = [60, 30, 10];

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins every new run's seed, for tests. `None` goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// The seed a new run uses: the pinned one, else the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}

/// A way to turn the dial. Right counts up, Left counts down.
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

    pub fn opposite(self) -> Turn {
        match self {
            Turn::Left => Turn::Right,
            Turn::Right => Turn::Left,
        }
    }

    /// One step this way from `n`, wrapping round the dial.
    pub fn step(self, n: u32) -> u32 {
        match self {
            Turn::Right => (n + 1) % DIAL,
            Turn::Left => (n + DIAL - 1) % DIAL,
        }
    }

    /// How many steps this way from `from` to `to`.
    pub fn distance(self, from: u32, to: u32) -> u32 {
        match self {
            Turn::Right => (to + DIAL - from) % DIAL,
            Turn::Left => (from + DIAL - to) % DIAL,
        }
    }
}

/// The shortest distance round the dial between two numbers.
pub fn gap(a: u32, b: u32) -> u32 {
    let d = Turn::Right.distance(a, b);
    d.min(DIAL - d)
}

/// The kind of click a number makes as the dial reaches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detent {
    /// An ordinary number: a soft click.
    Plain,
    /// Every tenth number (10, 20 ... 90): a firmer double click.
    Ten,
    /// Zero, at the top of the dial: a deep notch.
    Zero,
}

impl Detent {
    pub fn of(n: u32) -> Detent {
        if n == 0 {
            Detent::Zero
        } else if n.is_multiple_of(10) {
            Detent::Ten
        } else {
            Detent::Plain
        }
    }
}

/// How a guard visits a safe's room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuardSpec {
    /// Seconds between visits, least and most.
    pub gap: (f64, f64),
    /// Seconds of footsteps coming before they stop at the door.
    pub approach: f64,
    /// Seconds he stands listening, least and most.
    pub near: (f64, f64),
    /// Seconds of footsteps walking away.
    pub leave: f64,
}

/// One safe: how hard it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SafeSpec {
    /// 1 to [`SAFES`]; 0 is the practice safe.
    pub number: u32,
    pub name: &'static str,
    pub wheels: usize,
    /// False gates on each wheel: hollow rattles at wrong numbers.
    pub decoys: usize,
    /// How loud the true clunk is, 1.0 the loudest.
    pub clunk: f32,
    /// Wheels alternate: the first is found turning right, the next left.
    pub alternate: bool,
    pub guard: Option<GuardSpec>,
    /// Speak the guard's comings and goings, not just sound them.
    pub call_guard: bool,
    /// Building noises to either side, seconds apart (least, most).
    pub noises: Option<(f64, f64)>,
    pub hints: u32,
    /// Seconds under which a crack earns a time bonus.
    pub par: f64,
    /// Seconds on the time attack clock.
    pub limit: f64,
    /// What is new on this safe, said as it begins.
    pub news: &'static str,
}

const fn guard(gap: (f64, f64), approach: f64, near: (f64, f64)) -> Option<GuardSpec> {
    Some(GuardSpec {
        gap,
        approach,
        near,
        leave: 4.0,
    })
}

/// The practice safe: one wheel at a time, forever, with nothing to lose.
pub const PRACTICE: SafeSpec = SafeSpec {
    number: 0,
    name: "the practice safe",
    wheels: 1,
    decoys: 0,
    clunk: 1.0,
    alternate: false,
    guard: None,
    call_guard: false,
    noises: None,
    hints: u32::MAX,
    par: 0.0,
    limit: 0.0,
    news: "",
};

/// Safe `n` of the campaign, 1 to [`SAFES`]; anything else is the practice safe.
pub fn safe_spec(n: u32) -> SafeSpec {
    let base = SafeSpec {
        number: n,
        name: "",
        wheels: 2,
        decoys: 0,
        clunk: 1.0,
        alternate: false,
        guard: None,
        call_guard: false,
        noises: None,
        hints: 3,
        par: 60.0,
        limit: 60.0,
        news: "",
    };
    match n {
        1 => SafeSpec {
            name: "the office lockbox",
            news: "A good loud clunk on every wheel. Take your time: nothing is chasing you.",
            ..base
        },
        2 => SafeSpec {
            name: "the hotel strongbox",
            clunk: 0.85,
            noises: Some((10.0, 18.0)),
            limit: 55.0,
            news: "The hotel's old pipes knock and its floors creak, off to your left and right. They are only the building. The tumbler always clunks from the dial, right in front of you.",
            ..base
        },
        3 => SafeSpec {
            name: "the pawnbroker's safe",
            wheels: 3,
            decoys: 1,
            clunk: 0.8,
            noises: Some((12.0, 20.0)),
            par: 90.0,
            limit: 75.0,
            news: "Each has a false gate: a hollow rattle at a wrong number. Only the deep clunk is the true tumbler. Crack this safe to open time attack.",
            ..base
        },
        4 => SafeSpec {
            name: "the jeweller's safe",
            wheels: 3,
            decoys: 1,
            clunk: 0.75,
            alternate: true,
            noises: Some((12.0, 20.0)),
            par: 100.0,
            limit: 80.0,
            news: "A real combination dial: the direction rule. The tumbler only clunks when you turn the right way, and you must reach the number turning that way before you set it.",
            ..base
        },
        5 => SafeSpec {
            name: "the bank manager's safe",
            wheels: 3,
            decoys: 1,
            clunk: 0.7,
            alternate: true,
            guard: guard((25.0, 40.0), 7.0, (5.0, 7.0)),
            call_guard: true,
            noises: Some((14.0, 22.0)),
            hints: 2,
            par: 120.0,
            limit: 100.0,
            news: "A night guard walks the corridor. You will hear his footsteps coming for several seconds. When they stop outside the door, keep still: do not turn the dial or set a wheel until you hear him walk away. The first noise makes him suspicious, and the second gets you caught. On this safe, the game tells you when he comes and goes.",
            ..base
        },
        6 => SafeSpec {
            name: "the casino cage",
            wheels: 4,
            decoys: 2,
            clunk: 0.65,
            alternate: true,
            guard: guard((22.0, 35.0), 6.0, (5.0, 7.0)),
            noises: Some((12.0, 20.0)),
            hints: 2,
            par: 150.0,
            limit: 115.0,
            news: "Two false gates on each wheel. From now on the guard comes unannounced: listen for his footsteps, and press {status} if you are unsure where he is.",
            ..base
        },
        7 => SafeSpec {
            name: "the museum strongroom",
            wheels: 4,
            decoys: 2,
            clunk: 0.6,
            alternate: true,
            guard: guard((18.0, 30.0), 6.0, (5.0, 8.0)),
            noises: Some((10.0, 18.0)),
            hints: 2,
            par: 170.0,
            limit: 130.0,
            news: "The tumblers here are well oiled: the clunk is quieter. Listen closely.",
            ..base
        },
        8 => SafeSpec {
            name: "the embassy safe",
            wheels: 5,
            decoys: 2,
            clunk: 0.55,
            alternate: true,
            guard: guard((18.0, 28.0), 6.0, (5.0, 8.0)),
            noises: Some((10.0, 16.0)),
            hints: 2,
            par: 200.0,
            limit: 155.0,
            news: "Keep listening for the guard.",
            ..base
        },
        9 => SafeSpec {
            name: "the private bank vault",
            wheels: 5,
            decoys: 3,
            clunk: 0.5,
            alternate: true,
            guard: guard((15.0, 25.0), 5.0, (6.0, 8.0)),
            noises: Some((8.0, 14.0)),
            hints: 1,
            par: 220.0,
            limit: 165.0,
            news: "Three false gates a wheel, a quieter clunk, and a guard who walks by more often. One hint only.",
            ..base
        },
        10 => SafeSpec {
            name: "the Crown vault",
            wheels: 5,
            decoys: 3,
            clunk: 0.45,
            alternate: true,
            guard: guard((14.0, 22.0), 5.0, (6.0, 9.0)),
            noises: Some((8.0, 14.0)),
            hints: 1,
            par: 240.0,
            limit: 180.0,
            news: "The last and finest safe in the city. Everything you have learned, all at once.",
            ..base
        },
        _ => PRACTICE,
    }
}

/// Friday the 13th's bonus safe: four wheels, one of them always 13.
pub const UNLUCKY: SafeSpec = SafeSpec {
    number: 0,
    name: "the unlucky safe",
    wheels: 4,
    decoys: 1,
    clunk: 0.8,
    alternate: false,
    guard: None,
    call_guard: false,
    noises: Some((12.0, 20.0)),
    hints: 3,
    par: 120.0,
    limit: 160.0,
    news: "One of its four wheels is always 13, and each has a false gate. A black cat sleeps on top. She yowls softly just after the clunk at 13, so you know that wheel when you find it.",
};

/// The new moon's bonus safe, in the dark: three wheels, and the dial's
/// tens click like any other number.
pub const BLACKOUT: SafeSpec = SafeSpec {
    number: 0,
    name: "the blackout safe",
    wheels: 3,
    decoys: 1,
    clunk: 0.8,
    alternate: false,
    guard: None,
    call_guard: false,
    noises: None,
    hints: 3,
    par: 90.0,
    limit: 120.0,
    news: "The power is cut and the room hum is gone: you hear rain on the windows. In the dark the tens give the plain click, so count from the deep zero notch, or press {secondary} for the number. Each wheel has a false gate.",
};

/// What kind of play this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// One safe of the campaign, at your own pace.
    Safe(u32),
    /// One cracked safe again, against the clock.
    TimeAttack(u32),
    /// Endless single wheels with nothing to lose.
    Practice,
    /// A safe of a daily, weekly or monthly challenge job, its spec built
    /// by [`crate::challenge::Plan`]; `timed` when the job runs on a clock.
    Challenge { period: Period, timed: bool },
    /// Friday the 13th's bonus safe.
    Unlucky,
    /// The new moon's bonus safe, in the dark.
    Blackout,
}

impl Mode {
    pub fn number(self) -> u32 {
        match self {
            Mode::Safe(n) | Mode::TimeAttack(n) => n,
            Mode::Practice | Mode::Challenge { .. } | Mode::Unlucky | Mode::Blackout => 0,
        }
    }

    /// A bonus safe that is on the title only on its nights.
    pub fn is_event(self) -> bool {
        matches!(self, Mode::Unlucky | Mode::Blackout)
    }

    pub fn timed(self) -> bool {
        matches!(
            self,
            Mode::TimeAttack(_) | Mode::Challenge { timed: true, .. }
        )
    }

    /// The score board this mode records to.
    pub fn board(self) -> String {
        match self {
            Mode::Safe(n) => format!("safe {n}"),
            Mode::TimeAttack(n) => format!("time attack {n}"),
            Mode::Practice => "practice".to_string(),
            Mode::Challenge { .. } => "challenge".to_string(),
            Mode::Unlucky => "unlucky".to_string(),
            Mode::Blackout => "blackout".to_string(),
        }
    }

    /// The campaign's spec for this mode. A challenge's safes are built by
    /// its plan and started with [`Run::with_spec`]; on its own a challenge
    /// mode falls back to the practice safe.
    pub fn spec(self) -> SafeSpec {
        match self {
            Mode::Practice | Mode::Challenge { .. } => PRACTICE,
            Mode::Unlucky => UNLUCKY,
            Mode::Blackout => BLACKOUT,
            m => safe_spec(m.number()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardPhase {
    /// Somewhere else in the building.
    Away,
    /// Footsteps coming down the corridor.
    Coming,
    /// Standing at the door, listening.
    Near,
    /// Footsteps walking away.
    Leaving,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guard {
    pub phase: GuardPhase,
    /// Seconds left in this phase.
    pub left: f64,
    /// This phase's whole length.
    pub length: f64,
}

impl Guard {
    /// How far through the phase he is, 0 to 1.
    pub fn progress(&self) -> f64 {
        if self.length <= 0.0 {
            1.0
        } else {
            (1.0 - self.left / self.length).clamp(0.0, 1.0)
        }
    }
}

/// How a safe ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Opened,
    /// Three wrong sets: the relocker fired.
    Jammed,
    /// The guard heard you twice.
    Caught,
    /// The time attack clock ran out.
    TimeUp,
}

impl Outcome {
    pub fn won(self) -> bool {
        self == Outcome::Opened
    }
}

/// Why a set failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrong {
    /// The dial was not on the wheel's number.
    Number,
    /// On the number, but reached turning the wrong way.
    Direction,
}

/// Building noises that are not the safe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Noise {
    Pipe,
    Creak,
    Clock,
}

impl Noise {
    pub const ALL: [Noise; 3] = [Noise::Pipe, Noise::Creak, Noise::Clock];
}

/// How near a hint says you are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Near {
    On,
    VeryClose,
    Close,
    Some,
    Far,
}

impl Near {
    pub fn of(d: u32) -> Near {
        match d {
            0 => Near::On,
            1..=2 => Near::VeryClose,
            3..=6 => Near::Close,
            7..=20 => Near::Some,
            _ => Near::Far,
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Near::On => "on the number",
            Near::VeryClose => "very close, one or two numbers away",
            Near::Close => "close, within six numbers",
            Near::Some => "a little way off, within twenty numbers",
            Near::Far => "far off, more than twenty numbers away",
        }
    }
}

/// A hint, ready to be put into words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    pub near: Near,
    /// The way to turn to reach the number.
    pub turn: Turn,
    /// The direction rule wants the number reached turning `turn`, and it
    /// lies just behind you: go back past it, then come forward again.
    pub passed: bool,
    /// On the number, but reached the wrong way.
    pub wrong_way: bool,
    /// Hints left after this one, `None` when unlimited.
    pub left: Option<u32>,
}

impl Hint {
    pub fn words(&self) -> String {
        let main = if self.near == Near::On {
            if self.wrong_way {
                format!(
                    "The dial is on the number, but you reached it turning {}. Turn a little {}, then come back turning {}.",
                    self.turn.opposite().word(),
                    self.turn.opposite().word(),
                    self.turn.word()
                )
            } else {
                "The dial is on the number. Set it.".to_string()
            }
        } else if self.passed {
            format!(
                "You have just passed it: it is {} behind you. Turn back {} past it, then come forward turning {}.",
                self.near.words(),
                self.turn.opposite().word(),
                self.turn.word()
            )
        } else {
            format!(
                "The number is {}, turning {}.",
                self.near.words(),
                self.turn.word()
            )
        };
        match self.left {
            Some(0) => format!("{main} That was your last hint."),
            Some(1) => format!("{main} 1 hint left."),
            Some(n) => format!("{main} {n} hints left."),
            None => main,
        }
    }
}

/// What happened, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// The dial moved one number.
    Click {
        number: u32,
        detent: Detent,
    },
    /// The dial spun ten numbers.
    Spin {
        to: u32,
    },
    /// The true tumbler: the dial passed or reached the wheel's number,
    /// turning the right way.
    Clunk,
    /// A false gate's rattle.
    FalseGate,
    /// A wheel set; `next` is the wheel now being found, 0-based.
    Set {
        wheel: usize,
        of: usize,
    },
    /// Practice: a wheel set at `number`, and a fresh one waiting.
    PracticeSet {
        number: u32,
        sets: u32,
    },
    /// A wrong set: the wheel springs back.
    Wrong {
        why: Wrong,
        strikes: u32,
    },
    /// Every wheel set: the handle turns.
    Opened {
        score: u64,
    },
    Hint(Hint),
    NoHints,
    GuardComing,
    GuardNear,
    GuardLeaving,
    GuardGone,
    /// You made a noise with the guard at the door.
    Heard {
        suspicion: u32,
    },
    TimeWarning(u32),
    /// A building noise, `x` metres to the side (negative is left).
    Noise {
        kind: Noise,
        x: f32,
    },
    Ended(Outcome),
}

/// One safe being cracked.
#[derive(Clone, Debug)]
pub struct Run {
    pub mode: Mode,
    pub spec: SafeSpec,
    rng: Rng,
    /// The number under the index mark.
    pub dial: u32,
    /// The wheel being found, 0-based.
    pub wheel: usize,
    /// This wheel's number.
    target: u32,
    /// This wheel's false gates.
    decoys: Vec<u32>,
    /// The way the dial last moved.
    pub last_turn: Option<Turn>,
    pub strikes: u32,
    pub suspicion: u32,
    pub hints_left: u32,
    pub hints_used: u32,
    /// Seconds spent on this safe.
    pub time: f64,
    /// Seconds left on the time attack clock.
    pub time_left: f64,
    /// Whether the clock runs. The screen holds it until the first turn,
    /// so the intro costs no time.
    pub clock_running: bool,
    warned: Vec<u32>,
    pub guard: Option<Guard>,
    pub guard_visits: u32,
    noise_in: f64,
    pub outcome: Option<Outcome>,
    pub score: u64,
    /// Practice: wheels set so far.
    pub practice_sets: u32,
    /// The unlucky safe: the wheel, 0-based, whose number is 13.
    pub unlucky_wheel: Option<usize>,
}

impl Run {
    pub fn new(mode: Mode, seed: u64) -> Run {
        Run::with_spec(mode, mode.spec(), seed)
    }

    /// A run of a safe built elsewhere, such as a challenge's.
    pub fn with_spec(mode: Mode, spec: SafeSpec, seed: u64) -> Run {
        let mut rng = Rng::seeded(seed);
        let dial = rng.below(DIAL);
        let guard = spec.guard.map(|g| {
            let wait = between(&mut rng, g.gap) * 0.6;
            Guard {
                phase: GuardPhase::Away,
                left: wait,
                length: wait,
            }
        });
        let noise_in = spec.noises.map(|n| between(&mut rng, n)).unwrap_or(0.0);
        let unlucky_wheel = (mode == Mode::Unlucky).then(|| rng.below(spec.wheels as u32) as usize);
        let mut dial = dial;
        if unlucky_wheel == Some(0) && gap(dial, UNLUCKY_NUMBER) < MIN_GAP {
            dial = (UNLUCKY_NUMBER + DIAL / 2) % DIAL;
        }
        let mut run = Run {
            mode,
            spec,
            rng,
            dial,
            wheel: 0,
            target: 0,
            decoys: Vec::new(),
            last_turn: None,
            strikes: 0,
            suspicion: 0,
            hints_left: spec.hints,
            hints_used: 0,
            time: 0.0,
            time_left: if mode.timed() { spec.limit } else { 0.0 },
            clock_running: true,
            warned: Vec::new(),
            guard,
            guard_visits: 0,
            noise_in,
            outcome: None,
            score: 0,
            practice_sets: 0,
            unlucky_wheel,
        };
        run.new_wheel();
        run
    }

    /// Chooses the current wheel's number and false gates.
    fn new_wheel(&mut self) {
        let unlucky = self.unlucky_wheel == Some(self.wheel);
        // The wheel before the unlucky one leaves the dial well clear of 13.
        let before_unlucky = self.unlucky_wheel == Some(self.wheel + 1);
        loop {
            if unlucky {
                self.target = UNLUCKY_NUMBER;
                break;
            }
            let t = self.rng.below(DIAL);
            let clear = self.unlucky_wheel.is_none()
                || (t != UNLUCKY_NUMBER && (!before_unlucky || gap(t, UNLUCKY_NUMBER) >= MIN_GAP));
            if gap(t, self.dial) >= MIN_GAP && clear {
                self.target = t;
                break;
            }
        }
        self.decoys.clear();
        while self.decoys.len() < self.spec.decoys {
            let d = self.rng.below(DIAL);
            if gap(d, self.target) >= 4
                && gap(d, self.dial) >= 2
                && self.decoys.iter().all(|&o| gap(o, d) >= 3)
            {
                self.decoys.push(d);
            }
        }
        self.last_turn = None;
    }

    /// Starts the clock at `secs` left, as when a job's one clock carries
    /// over from the safe before; warnings already passed stay quiet.
    pub fn carry_clock(&mut self, secs: f64) {
        self.time_left = secs.max(0.0);
        for w in WARNINGS {
            if self.time_left <= w as f64 && !self.warned.contains(&w) {
                self.warned.push(w);
            }
        }
    }

    /// This wheel's number. The screen never says it; tests and hints use it.
    pub fn target(&self) -> u32 {
        self.target
    }

    pub fn decoys(&self) -> &[u32] {
        &self.decoys
    }

    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn practice(&self) -> bool {
        self.mode == Mode::Practice
    }

    /// The way the current wheel must be found, when the direction rule holds.
    pub fn required(&self) -> Option<Turn> {
        self.spec.alternate.then(|| required_turn(self.wheel))
    }

    /// The guard is at the door, listening.
    pub fn guard_listening(&self) -> bool {
        self.guard.is_some_and(|g| g.phase == GuardPhase::Near)
    }

    fn turn_ok(&self, turn: Turn) -> bool {
        self.required().is_none_or(|r| r == turn)
    }

    /// A noise with the guard at the door.
    fn noise(&mut self, evs: &mut Vec<Ev>) {
        if !self.guard_listening() {
            return;
        }
        self.suspicion += 1;
        evs.push(Ev::Heard {
            suspicion: self.suspicion,
        });
        if self.suspicion >= SUSPICION {
            self.end(Outcome::Caught, evs);
        }
    }

    fn end(&mut self, outcome: Outcome, evs: &mut Vec<Ev>) {
        if self.outcome.is_none() {
            self.outcome = Some(outcome);
            evs.push(Ev::Ended(outcome));
        }
    }

    /// Turns the dial one number.
    pub fn turn(&mut self, turn: Turn) -> Vec<Ev> {
        let mut evs = Vec::new();
        if self.finished() {
            return evs;
        }
        self.noise(&mut evs);
        if self.finished() {
            return evs;
        }
        self.dial = turn.step(self.dial);
        self.last_turn = Some(turn);
        evs.push(Ev::Click {
            number: self.dial,
            detent: Detent::of(self.dial),
        });
        if self.dial == self.target && self.turn_ok(turn) {
            evs.push(Ev::Clunk);
        }
        if self.decoys.contains(&self.dial) {
            evs.push(Ev::FalseGate);
        }
        evs
    }

    /// Spins the dial ten numbers; a tumbler or false gate passed on the
    /// way still sounds, once.
    pub fn spin(&mut self, turn: Turn) -> Vec<Ev> {
        let mut evs = Vec::new();
        if self.finished() {
            return evs;
        }
        self.noise(&mut evs);
        if self.finished() {
            return evs;
        }
        let mut clunk = false;
        let mut gate = false;
        for _ in 0..SPIN {
            self.dial = turn.step(self.dial);
            clunk |= self.dial == self.target && self.turn_ok(turn);
            gate |= self.decoys.contains(&self.dial);
        }
        self.last_turn = Some(turn);
        evs.push(Ev::Spin { to: self.dial });
        if clunk {
            evs.push(Ev::Clunk);
        }
        if gate {
            evs.push(Ev::FalseGate);
        }
        evs
    }

    /// Sets the number under the mark on the current wheel.
    pub fn set(&mut self) -> Vec<Ev> {
        let mut evs = Vec::new();
        if self.finished() {
            return evs;
        }
        self.noise(&mut evs);
        if self.finished() {
            return evs;
        }
        let why = if self.dial != self.target {
            Some(Wrong::Number)
        } else if self.required().is_some() && self.last_turn != self.required() {
            Some(Wrong::Direction)
        } else {
            None
        };
        if let Some(why) = why {
            if !self.practice() {
                self.strikes += 1;
            }
            evs.push(Ev::Wrong {
                why,
                strikes: self.strikes,
            });
            if self.strikes >= STRIKES {
                self.end(Outcome::Jammed, &mut evs);
            }
            return evs;
        }
        if self.practice() {
            self.practice_sets += 1;
            evs.push(Ev::PracticeSet {
                number: self.target,
                sets: self.practice_sets,
            });
            self.new_wheel();
            return evs;
        }
        self.wheel += 1;
        if self.wheel >= self.spec.wheels {
            self.score = self.crack_score();
            evs.push(Ev::Opened { score: self.score });
            self.end(Outcome::Opened, &mut evs);
        } else {
            evs.push(Ev::Set {
                wheel: self.wheel,
                of: self.spec.wheels,
            });
            self.new_wheel();
        }
        evs
    }

    /// The score for opening the safe now.
    pub fn crack_score(&self) -> u64 {
        let wheels = self.spec.wheels as f64 * 100.0;
        let bonus = if self.mode.timed() {
            self.time_left.max(0.0) * 3.0
        } else {
            (self.spec.par - self.time).max(0.0) * 2.0
        };
        let cost = self.strikes as f64 * 30.0 + self.hints_used as f64 * 20.0;
        (wheels + bonus - cost).round().max(10.0) as u64
    }

    /// A hint: how far the number is, in words.
    pub fn hint(&mut self) -> Vec<Ev> {
        if self.finished() {
            return Vec::new();
        }
        if self.hints_left == 0 {
            return vec![Ev::NoHints];
        }
        let unlimited = self.spec.hints == u32::MAX;
        if !unlimited {
            self.hints_left -= 1;
            self.hints_used += 1;
            if self.mode.timed() {
                self.time_left -= HINT_COST;
            }
        }
        let left = (!unlimited).then_some(self.hints_left);
        let hint = match self.required() {
            Some(turn) => {
                let ahead = turn.distance(self.dial, self.target);
                let behind = DIAL - ahead;
                if ahead == 0 {
                    Hint {
                        near: Near::On,
                        turn,
                        passed: false,
                        wrong_way: self.last_turn != Some(turn),
                        left,
                    }
                } else if behind < ahead {
                    Hint {
                        near: Near::of(behind),
                        turn,
                        passed: true,
                        wrong_way: false,
                        left,
                    }
                } else {
                    Hint {
                        near: Near::of(ahead),
                        turn,
                        passed: false,
                        wrong_way: false,
                        left,
                    }
                }
            }
            None => {
                let right = Turn::Right.distance(self.dial, self.target);
                let left_d = Turn::Left.distance(self.dial, self.target);
                let (turn, d) = if right <= left_d {
                    (Turn::Right, right)
                } else {
                    (Turn::Left, left_d)
                };
                Hint {
                    near: Near::of(d),
                    turn,
                    passed: false,
                    wrong_way: false,
                    left,
                }
            }
        };
        let mut evs = vec![Ev::Hint(hint)];
        if self.mode.timed() && self.time_left <= 0.0 {
            self.time_left = 0.0;
            self.end(Outcome::TimeUp, &mut evs);
        }
        evs
    }

    /// Time passing: the clock, the guard and the building.
    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        let mut evs = Vec::new();
        if self.finished() {
            return evs;
        }
        if self.clock_running {
            self.time += dt;
        }
        if self.clock_running && self.mode.timed() && !self.guard_listening() {
            self.time_left -= dt;
            for w in WARNINGS {
                if self.time_left <= w as f64
                    && self.spec.limit > w as f64
                    && !self.warned.contains(&w)
                {
                    self.warned.push(w);
                    if self.time_left > 0.0 {
                        evs.push(Ev::TimeWarning(w));
                    }
                }
            }
            if self.time_left <= 0.0 {
                self.time_left = 0.0;
                self.end(Outcome::TimeUp, &mut evs);
                return evs;
            }
        }
        self.update_guard(dt, &mut evs);
        if let Some(range) = self.spec.noises {
            self.noise_in -= dt;
            if self.noise_in <= 0.0 {
                self.noise_in = between(&mut self.rng, range);
                let kind = *self.rng.pick(&Noise::ALL);
                let side = if self.rng.chance(0.5) { -1.0 } else { 1.0 };
                let x = side * (2.5 + self.rng.unit() as f32 * 2.0);
                evs.push(Ev::Noise { kind, x });
            }
        }
        evs
    }

    fn update_guard(&mut self, dt: f64, evs: &mut Vec<Ev>) {
        let (Some(mut g), Some(spec)) = (self.guard, self.spec.guard) else {
            return;
        };
        g.left -= dt;
        if g.left <= 0.0 {
            let (phase, length, ev) = match g.phase {
                GuardPhase::Away => (GuardPhase::Coming, spec.approach, Ev::GuardComing),
                GuardPhase::Coming => (
                    GuardPhase::Near,
                    between(&mut self.rng, spec.near),
                    Ev::GuardNear,
                ),
                GuardPhase::Near => (GuardPhase::Leaving, spec.leave, Ev::GuardLeaving),
                GuardPhase::Leaving => {
                    self.guard_visits += 1;
                    (
                        GuardPhase::Away,
                        between(&mut self.rng, spec.gap),
                        Ev::GuardGone,
                    )
                }
            };
            g = Guard {
                phase,
                left: length,
                length,
            };
            evs.push(ev);
        }
        self.guard = Some(g);
    }

    /// Seconds until the guard next stands at the door, if he is coming
    /// or away; for tests and status.
    pub fn guard_phase(&self) -> Option<GuardPhase> {
        self.guard.map(|g| g.phase)
    }
}

/// The way wheel `i` (0-based) is found under the direction rule: right,
/// left, right...
pub fn required_turn(i: usize) -> Turn {
    if i.is_multiple_of(2) {
        Turn::Right
    } else {
        Turn::Left
    }
}

fn between(rng: &mut Rng, (lo, hi): (f64, f64)) -> f64 {
    lo + rng.unit() * (hi - lo).max(0.0)
}

/// "1 minute 5 seconds", "42 seconds".
pub fn time_words(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    let (m, s) = (s / 60, s % 60);
    let sec = if s == 1 { "second" } else { "seconds" };
    match m {
        0 => format!("{s} {sec}"),
        _ => {
            let min = if m == 1 { "minute" } else { "minutes" };
            if s == 0 {
                format!("{m} {min}")
            } else {
                format!("{m} {min} {s} {sec}")
            }
        }
    }
}

/// "1:05.3", for best-time tables.
pub fn time_short(secs: f64) -> String {
    let t = (secs.max(0.0) * 10.0).round() as u64;
    let (m, rest) = (t / 600, t % 600);
    format!("{m}:{:02}.{}", rest / 10, rest % 10)
}

/// "one", "two" ... for wheel counts in speech.
pub fn count_word(n: usize) -> &'static str {
    match n {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        _ => "several",
    }
}

/// The direction sequence of a safe, spoken: "right, left, right".
pub fn sequence_words(spec: &SafeSpec) -> String {
    (0..spec.wheels)
        .map(|i| required_turn(i).word())
        .collect::<Vec<_>>()
        .join(", ")
}
