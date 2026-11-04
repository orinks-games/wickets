//! The rules of a shift in the signal box, with nothing spoken or played.
//!
//! A [`Shift`] issues calls one at a time. Each call opens a time window;
//! the matching answer inside it passes a train and scores, a wrong answer
//! or no answer derails the run. Every few trains the window shrinks by a
//! set step, never below its floor. The shift runs on a fixed substep
//! clock, so the same play gives the same result at any frame rate.

use hearken::text::sentence_start;
use parlor_core::{FixedStep, ParlorAction, Rng};

/// Substeps a second of the shift's clock.
pub const STEP_HZ: f64 = 120.0;
/// The window at the start of a shift, in milliseconds.
pub const START_WINDOW: u32 = 3000;
/// How much the window shrinks each time, in milliseconds.
pub const WINDOW_STEP: u32 = 200;
/// The window shrinks after every this many trains.
pub const SHRINK_EVERY: u32 = 5;
/// The shortest window, in milliseconds.
pub const FLOOR: u32 = 1000;
/// A streak bonus comes after every this many trains.
pub const STREAK_EVERY: u32 = 10;
/// The streak bonus, in points.
pub const STREAK_BONUS: u32 = 5;
/// Seconds from opening the line to the first call.
pub const LEAD_IN: f64 = 1.5;
/// Ticks in one window: the tick track's tempo follows the window.
pub const TICKS_PER_WINDOW: f64 = 4.0;

/// One of the dispatcher's calls, each answered by its own action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Call {
    PointsLeft,
    PointsRight,
    SignalUp,
    SignalDown,
    RingBack,
}

impl Call {
    pub const ALL: [Call; 5] = [
        Call::PointsLeft,
        Call::PointsRight,
        Call::SignalUp,
        Call::SignalDown,
        Call::RingBack,
    ];

    /// The call as the dispatcher says it: "Points left".
    pub fn word(self) -> &'static str {
        match self {
            Call::PointsLeft => "Points left",
            Call::PointsRight => "Points right",
            Call::SignalUp => "Signal up",
            Call::SignalDown => "Signal down",
            Call::RingBack => "Ring back",
        }
    }

    /// What the answer does, after a key name: "throws the points left".
    pub fn deed(self) -> &'static str {
        match self {
            Call::PointsLeft => "throws the points left",
            Call::PointsRight => "throws the points right",
            Call::SignalUp => "raises the signal",
            Call::SignalDown => "lowers the signal",
            Call::RingBack => "rings the bell back",
        }
    }

    /// The call's sound, in plain words.
    pub fn sound_words(self) -> &'static str {
        match self {
            Call::PointsLeft => "a low iron clank on your left",
            Call::PointsRight => "a slightly higher iron clank on your right",
            Call::SignalUp => "a rising ratchet",
            Call::SignalDown => "a falling ratchet",
            Call::RingBack => "two strokes of a bell",
        }
    }

    /// The action that answers the call.
    pub fn action(self) -> ParlorAction {
        match self {
            Call::PointsLeft => ParlorAction::Left,
            Call::PointsRight => ParlorAction::Right,
            Call::SignalUp => ParlorAction::Up,
            Call::SignalDown => ParlorAction::Down,
            Call::RingBack => ParlorAction::Primary,
        }
    }

    /// The call an action answers, if it answers one.
    pub fn for_action(action: ParlorAction) -> Option<Call> {
        Call::ALL.into_iter().find(|c| c.action() == action)
    }
}

/// How a shift's pace runs. Times are whole milliseconds, so shrinking
/// never drifts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    pub start: u32,
    pub step: u32,
    pub every: u32,
    pub floor: u32,
}

impl Pace {
    /// The night shift's pace.
    pub const SHIFT: Pace = Pace {
        start: START_WINDOW,
        step: WINDOW_STEP,
        every: SHRINK_EVERY,
        floor: FLOOR,
    };
}

/// What kind of run the box is working.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Endless, with random calls.
    Endless,
    /// A fixed list of calls; clearing it ends the run.
    Timetable(Vec<Call>),
    /// Endless, with no clock and no derailing.
    Practice,
}

/// Why a run derailed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Miss {
    /// The wrong answer to `call`.
    Wrong { call: Call, got: Call },
    /// No answer to `call` inside the window.
    Late { call: Call },
}

impl Miss {
    pub fn call(self) -> Call {
        match self {
            Miss::Wrong { call, .. } | Miss::Late { call } => call,
        }
    }

    /// What went wrong, in a sentence.
    pub fn words(self) -> String {
        match self {
            Miss::Wrong { call, got } => format!(
                "The call was {}, and you {}.",
                call.word().to_lowercase(),
                past(got)
            ),
            Miss::Late { call } => {
                format!("Too late for {}.", call.word().to_lowercase())
            }
        }
    }
}

/// "threw the points left".
fn past(call: Call) -> &'static str {
    match call {
        Call::PointsLeft => "threw the points left",
        Call::PointsRight => "threw the points right",
        Call::SignalUp => "raised the signal",
        Call::SignalDown => "lowered the signal",
        Call::RingBack => "rang the bell back",
    }
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Derailed(Miss),
    /// Every train on the timetable passed.
    Cleared,
}

/// Where the shift is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// The line is not open yet.
    Waiting,
    /// The line is open; the first call comes when `left` runs out.
    LeadIn {
        left: f64,
    },
    /// A call is out and its window is running.
    Calling {
        call: Call,
        elapsed: f64,
    },
    /// A train is passing; the next call comes when `left` runs out.
    Passing {
        left: f64,
    },
    Over,
}

/// What happened, for the screen to voice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ev {
    /// A new call is out.
    Call(Call),
    /// The right answer: a train passed. `train` counts from 1.
    Passed {
        train: u32,
        points: u32,
    },
    /// Another `STREAK_EVERY` trains in a row.
    Streak {
        trains: u32,
        bonus: u32,
    },
    /// The window shrank, to this many milliseconds.
    Faster {
        window: u32,
    },
    /// The window reached its floor.
    TopSpeed,
    /// A beat of the tick track.
    Tick,
    /// An answer with no call out.
    NoCall,
    /// In practice, the wrong answer: the call stays out.
    Wrong {
        call: Call,
        got: Call,
    },
    Derailed(Miss),
    Cleared,
}

/// One run in the signal box.
#[derive(Clone, Debug)]
pub struct Shift {
    pub line: Line,
    pub pace: Pace,
    rng: Rng,
    clock: FixedStep,
    pub phase: Phase,
    /// The window now, in milliseconds.
    pub window: u32,
    /// Trains passed.
    pub passed: u32,
    pub score: u32,
    /// The last call out, for Repeat.
    pub last_call: Option<Call>,
    pub outcome: Option<Outcome>,
    /// Seconds since the line opened.
    pub time: f64,
    /// Seconds to the next tick.
    tick_left: f64,
    /// Calls answered wrongly in practice.
    pub practice_misses: u32,
    /// Every this many trains, a special worth more: the steam special.
    pub special_every: Option<u32>,
    /// Specials passed.
    pub specials: u32,
}

impl Shift {
    pub fn new(line: Line, pace: Pace, seed: u64) -> Shift {
        Shift {
            line,
            pace,
            rng: Rng::seeded(seed),
            clock: FixedStep::hz(STEP_HZ),
            phase: Phase::Waiting,
            window: pace.start,
            passed: 0,
            score: 0,
            last_call: None,
            outcome: None,
            time: 0.0,
            tick_left: 0.0,
            practice_misses: 0,
            special_every: None,
            specials: 0,
        }
    }

    /// A steam gala shift: endless, with every
    /// [`crate::events::SPECIAL_EVERY`]th train the special.
    pub fn steam_gala(seed: u64) -> Shift {
        let mut shift = Shift::endless(seed);
        shift.special_every = Some(crate::events::SPECIAL_EVERY);
        shift
    }

    /// Whether train `train`, counting from 1, is a special.
    pub fn is_special(&self, train: u32) -> bool {
        self.special_every
            .is_some_and(|n| n > 0 && train.is_multiple_of(n))
    }

    /// An endless night shift.
    pub fn endless(seed: u64) -> Shift {
        Shift::new(Line::Endless, Pace::SHIFT, seed)
    }

    /// The practice room: calls with no clock.
    pub fn practice(seed: u64) -> Shift {
        Shift::new(Line::Practice, Pace::SHIFT, seed)
    }

    pub fn timed(&self) -> bool {
        self.line != Line::Practice
    }

    /// The timetable's length, if the run has one.
    pub fn trains(&self) -> Option<u32> {
        match &self.line {
            Line::Timetable(calls) => Some(calls.len() as u32),
            _ => None,
        }
    }

    pub fn started(&self) -> bool {
        self.phase != Phase::Waiting
    }

    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }

    /// The call out now, if any.
    pub fn current(&self) -> Option<Call> {
        match self.phase {
            Phase::Calling { call, .. } => Some(call),
            _ => None,
        }
    }

    /// The window in seconds.
    pub fn window_secs(&self) -> f64 {
        self.window as f64 / 1000.0
    }

    /// The pause between a train passing and the next call: half the
    /// window, between 0.6 and 1.5 seconds.
    pub fn gap(&self) -> f64 {
        (self.window_secs() * 0.5).clamp(0.6, 1.5)
    }

    fn tick_period(&self) -> f64 {
        self.window_secs() / TICKS_PER_WINDOW
    }

    /// Opens the line. The first call follows after [`LEAD_IN`].
    pub fn start(&mut self) {
        if self.phase == Phase::Waiting {
            self.phase = Phase::LeadIn { left: LEAD_IN };
            self.tick_left = 0.0;
        }
    }

    /// The next call: the timetable's, or a fresh one from the seed.
    fn next_call(&mut self) -> Call {
        match &self.line {
            Line::Timetable(calls) => calls[self.passed as usize % calls.len().max(1)],
            _ => *self.rng.pick(&Call::ALL),
        }
    }

    fn issue(&mut self, evs: &mut Vec<Ev>) {
        let call = self.next_call();
        self.last_call = Some(call);
        self.phase = Phase::Calling { call, elapsed: 0.0 };
        evs.push(Ev::Call(call));
    }

    /// Gives the call out now a fresh window, as after a pause.
    pub fn recall(&mut self) -> Option<Call> {
        if let Phase::Calling { call, .. } = self.phase {
            self.phase = Phase::Calling { call, elapsed: 0.0 };
            return Some(call);
        }
        None
    }

    fn end(&mut self, outcome: Outcome, evs: &mut Vec<Ev>) {
        self.outcome = Some(outcome);
        self.phase = Phase::Over;
        evs.push(match outcome {
            Outcome::Derailed(miss) => Ev::Derailed(miss),
            Outcome::Cleared => Ev::Cleared,
        });
    }

    /// The player answers with `call`'s action.
    pub fn answer(&mut self, got: Call) -> Vec<Ev> {
        let mut evs = Vec::new();
        let Phase::Calling { call, .. } = self.phase else {
            if !self.finished() {
                evs.push(Ev::NoCall);
            }
            return evs;
        };
        if got != call {
            if self.timed() {
                self.end(Outcome::Derailed(Miss::Wrong { call, got }), &mut evs);
            } else {
                self.practice_misses += 1;
                evs.push(Ev::Wrong { call, got });
            }
            return evs;
        }
        self.passed += 1;
        let points = if self.is_special(self.passed) {
            self.specials += 1;
            crate::events::SPECIAL_POINTS
        } else {
            1
        };
        self.score += points;
        evs.push(Ev::Passed {
            train: self.passed,
            points,
        });
        if self.timed() && self.passed.is_multiple_of(STREAK_EVERY) {
            self.score += STREAK_BONUS;
            evs.push(Ev::Streak {
                trains: self.passed,
                bonus: STREAK_BONUS,
            });
        }
        if self.trains() == Some(self.passed) {
            self.end(Outcome::Cleared, &mut evs);
            return evs;
        }
        if self.timed()
            && self.passed.is_multiple_of(self.pace.every.max(1))
            && self.window > self.pace.floor
        {
            self.window = self
                .window
                .saturating_sub(self.pace.step)
                .max(self.pace.floor);
            evs.push(Ev::Faster {
                window: self.window,
            });
            if self.window == self.pace.floor {
                evs.push(Ev::TopSpeed);
            }
        }
        self.phase = Phase::Passing { left: self.gap() };
        evs
    }

    /// Time passing, in frames of any length.
    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        let mut evs = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |step| self.substep(step, &mut evs));
        self.clock = clock;
        evs
    }

    fn substep(&mut self, step: f64, evs: &mut Vec<Ev>) {
        if matches!(self.phase, Phase::Waiting | Phase::Over) {
            return;
        }
        self.time += step;
        if self.timed() {
            self.tick_left -= step;
            if self.tick_left <= 1e-9 {
                self.tick_left += self.tick_period();
                evs.push(Ev::Tick);
            }
        }
        match self.phase {
            Phase::LeadIn { left } | Phase::Passing { left } => {
                let left = left - step;
                if left <= 1e-9 {
                    self.issue(evs);
                } else if matches!(self.phase, Phase::LeadIn { .. }) {
                    self.phase = Phase::LeadIn { left };
                } else {
                    self.phase = Phase::Passing { left };
                }
            }
            Phase::Calling { call, elapsed } => {
                let elapsed = elapsed + step;
                if self.timed() && elapsed > self.window_secs() + 1e-9 {
                    self.end(Outcome::Derailed(Miss::Late { call }), evs);
                } else {
                    self.phase = Phase::Calling { call, elapsed };
                }
            }
            Phase::Waiting | Phase::Over => {}
        }
    }
}

/// A timetable of `n` calls from `rng`.
pub fn timetable(rng: &mut Rng, n: usize) -> Vec<Call> {
    (0..n).map(|_| *rng.pick(&Call::ALL)).collect()
}

/// The window in words: "about 3 seconds", "about 1.4 seconds".
pub fn window_words(ms: u32) -> String {
    let tenths = (ms + 50) / 100;
    match tenths {
        10 => "about 1 second".to_string(),
        t if t % 10 == 0 => format!("about {} seconds", t / 10),
        t => format!("about {}.{} seconds", t / 10, t % 10),
    }
}

/// "1 train", "34 trains".
pub fn trains_words(n: u32) -> String {
    match n {
        1 => "1 train".to_string(),
        n => format!("{n} trains"),
    }
}

/// "1 point", "12 points".
pub fn points_words(n: u32) -> String {
    match n {
        1 => "1 point".to_string(),
        n => format!("{n} points"),
    }
}

/// "Derailed after 34 trains."
pub fn derailed_words(passed: u32) -> String {
    match passed {
        0 => "Derailed on the first train.".to_string(),
        n => format!("Derailed after {}.", trains_words(n)),
    }
}

/// The key and what it does: "The Left arrow throws the points left."
pub fn answer_words(key: &str, call: Call) -> String {
    format!("{} {}.", sentence_start(key), call.deed())
}

thread_local! {
    static FIXED_SEED: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// Pins the seed new shifts get, for tests. `None` goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new shift: the pinned one, or one from the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}
