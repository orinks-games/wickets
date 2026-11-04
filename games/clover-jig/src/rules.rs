//! The dance as plain data: the calls of a tune, the beat clock, and how a
//! step is judged. No screens or sounds here, so tests can drive it.
//!
//! Time runs in whole milliseconds on a [`FixedStep`] clock, so a dance plays
//! out the same at any frame rate. A tune starts with a count of
//! [`LEAD_BEATS`] bodhran beats, then the music: [`PARTS`] parts of
//! [`PART_BEATS`] beats. The caller calls each step one beat before it, and
//! the step is judged by how far from its beat the key lands.

use parlor_core::{FixedStep, Rng};

use crate::tunes::{Level, Step, Tune};

/// Bodhran beats before the music starts.
pub const LEAD_BEATS: i64 = 4;
/// Beats in one part, A or B: four bars of two beats.
pub const PART_BEATS: u32 = 8;
/// Parts played: A A B B, twice.
pub const PARTS: u32 = 8;
/// Beats in a whole tune.
pub const TUNE_BEATS: u32 = PART_BEATS * PARTS;
/// Bars in a whole tune.
pub const TUNE_BARS: u32 = TUNE_BEATS / 2;
/// The first beat a step can fall on, so the first call comes after the
/// music has started.
pub const FIRST_STEP_BEAT: u32 = 2;
/// Milliseconds after the last beat before the tune counts as over, for
/// the last notes to ring.
pub const END_MS: i64 = 900;

/// A step this close to its beat is spot on.
pub const SPOT_ON_MS: i64 = 70;
/// This close is good.
pub const GOOD_MS: i64 = 140;
/// This close is close; further is a miss.
pub const WINDOW_MS: i64 = 220;

/// Beats counted in again after a pause.
pub const COUNT_IN_BEATS: i64 = 3;
/// Every this many hits in a row, the crowd claps and adds a bonus.
pub const STREAK_EVERY: u32 = 8;
pub const STREAK_BONUS: u32 = 4;
/// Points for catching the four-leaf clover.
pub const CLOVER_BONUS: u32 = 25;
/// Milliseconds between two tunes of a set.
pub const BETWEEN_MS: i64 = 4500;

/// Something extra on a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Luck {
    None,
    /// A shamrock, in Saint Patrick's week: a chime with the call. Hit the
    /// step good or better for a luck point.
    Shamrock,
    /// The four-leaf clover of ceili night: hit the step to find it.
    Clover,
}

/// One called step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Call {
    /// The beat the step falls on, from 0 at the start of the music.
    pub beat: u32,
    pub step: Step,
    pub luck: Luck,
}

/// How a step landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Judgment {
    SpotOn,
    Good,
    Close,
    /// No step in the window.
    Missed,
    /// The wrong step, the one given.
    Wrong(Step),
    /// In practice, a step danced while the tune waited for it.
    Waited,
}

impl Judgment {
    pub fn points(self) -> u32 {
        match self {
            Judgment::SpotOn => 3,
            Judgment::Good => 2,
            Judgment::Close | Judgment::Waited => 1,
            Judgment::Missed | Judgment::Wrong(_) => 0,
        }
    }

    /// Whether the step was danced, at any timing.
    pub fn hit(self) -> bool {
        self.points() > 0
    }

    /// The one word the game says for it, the first time it happens.
    pub fn word(self) -> &'static str {
        match self {
            Judgment::SpotOn => "Spot on.",
            Judgment::Good => "Good.",
            Judgment::Close => "Close.",
            Judgment::Missed => "Missed.",
            Judgment::Wrong(_) => "Wrong step.",
            Judgment::Waited => "Danced.",
        }
    }
}

/// How a step `offset_ms` from its beat is judged, or `None` outside the
/// window. Early and late count the same.
pub fn judge(offset_ms: i64) -> Option<Judgment> {
    match offset_ms.abs() {
        d if d <= SPOT_ON_MS => Some(Judgment::SpotOn),
        d if d <= GOOD_MS => Some(Judgment::Good),
        d if d <= WINDOW_MS => Some(Judgment::Close),
        _ => None,
    }
}

/// Stars for a tune, from the share of the best points it earned: 1 star
/// from half, 2 from three quarters, 3 from nine tenths.
pub fn stars(points: u32, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    let share = points as f64 / max as f64;
    match share {
        s if s >= 0.9 => 3,
        s if s >= 0.75 => 2,
        s if s >= 0.5 => 1,
        _ => 0,
    }
}

/// "1 star" or "3 stars", or "no stars".
pub fn stars_words(n: u32) -> String {
    match n {
        0 => "no stars".to_string(),
        1 => "1 star".to_string(),
        n => format!("{n} stars"),
    }
}

/// "1 point" or "40 points".
pub fn points_words(n: u32) -> String {
    match n {
        1 => "1 point".to_string(),
        n => format!("{n} points"),
    }
}

/// "1 step" or "24 steps".
pub fn steps_words(n: u32) -> String {
    match n {
        1 => "1 step".to_string(),
        n => format!("{n} steps"),
    }
}

/// "1 luck point" or "3 luck points".
pub fn luck_words(n: u32) -> String {
    match n {
        1 => "1 luck point".to_string(),
        n => format!("{n} luck points"),
    }
}

/// "1 beat" or "2 beats".
pub fn beats_words(n: u32) -> String {
    match n {
        1 => "1 beat".to_string(),
        n => format!("{n} beats"),
    }
}

/// The calls of a tune, called at `level`.
pub fn calls(level: &Level, rng: &mut Rng) -> Vec<Call> {
    let mut out: Vec<Call> = Vec::new();
    for beat in FIRST_STEP_BEAT..TUNE_BEATS {
        let p = if beat % 2 == 0 {
            level.downbeat
        } else {
            level.offbeat
        };
        if !rng.chance(p) {
            continue;
        }
        let prev = out.last().map(|c| c.step);
        let step = next_step(level.steps, prev, rng);
        out.push(Call {
            beat,
            step,
            luck: Luck::None,
        });
    }
    if out.is_empty() {
        out.push(Call {
            beat: FIRST_STEP_BEAT,
            step: level.steps[0],
            luck: Luck::None,
        });
    }
    out
}

/// A step that follows `prev` the way a dancer would: left answers right,
/// forward is often followed by back, and a turn never comes twice.
fn next_step(steps: &[Step], prev: Option<Step>, rng: &mut Rng) -> Step {
    let has = |s: Step| steps.contains(&s);
    match prev {
        Some(Step::Left) if has(Step::Right) && rng.chance(0.5) => return Step::Right,
        Some(Step::Right) if has(Step::Left) && rng.chance(0.5) => return Step::Left,
        Some(Step::Forward) if has(Step::Back) && rng.chance(0.6) => return Step::Back,
        _ => {}
    }
    loop {
        let s = *rng.pick(steps);
        if Some(s) == prev && matches!(s, Step::Turn | Step::Hop) && steps.len() > 1 {
            continue;
        }
        return s;
    }
}

/// Marks some calls with shamrocks: about one in ten, never fewer than 2
/// or more than 6.
pub fn hide_shamrocks(calls: &mut [Call], rng: &mut Rng) {
    let mut marked: Vec<usize> = (0..calls.len()).filter(|_| rng.chance(0.1)).collect();
    while marked.len() < 2.min(calls.len()) {
        let i = rng.below(calls.len() as u32) as usize;
        if !marked.contains(&i) {
            marked.push(i);
        }
    }
    marked.truncate(6);
    for i in marked {
        calls[i].luck = Luck::Shamrock;
    }
}

/// Puts the four-leaf clover on one call in the second half of the tune.
pub fn hide_clover(calls: &mut [Call], rng: &mut Rng) {
    let later: Vec<usize> = (0..calls.len())
        .filter(|&i| calls[i].beat >= TUNE_BEATS / 2 && calls[i].luck == Luck::None)
        .collect();
    let pool = if later.is_empty() {
        (0..calls.len()).collect()
    } else {
        later
    };
    if pool.is_empty() {
        return;
    }
    let i = pool[rng.below(pool.len() as u32) as usize];
    calls[i].luck = Luck::Clover;
}

/// One tune as it will be danced.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub tune: &'static Tune,
    pub calls: Vec<Call>,
}

impl Plan {
    pub fn new(tune: &'static Tune, level: &Level, rng: &mut Rng) -> Plan {
        Plan {
            tune,
            calls: calls(level, rng),
        }
    }

    /// The most points the calls can earn, without streak bonuses.
    pub fn max_points(&self) -> u32 {
        self.calls.len() as u32 * Judgment::SpotOn.points()
    }

    pub fn shamrocks(&self) -> usize {
        self.calls
            .iter()
            .filter(|c| c.luck == Luck::Shamrock)
            .count()
    }

    pub fn has_clover(&self) -> bool {
        self.calls.iter().any(|c| c.luck == Luck::Clover)
    }
}

/// What happened in a millisecond, or after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A count beat: before the music (1 to 4), or after a pause.
    Count(u32),
    /// A part of the tune starts: its number from 0, and whether it is B.
    Part { index: u32, b: bool },
    /// The caller calls a step, one beat ahead.
    Call {
        index: usize,
        step: Step,
        luck: Luck,
    },
    /// A step landed in its window.
    Judged {
        index: usize,
        step: Step,
        judgment: Judgment,
    },
    /// A step's window closed with no step.
    Missed { index: usize, step: Step },
    /// The wrong step for a call. In practice the call keeps waiting.
    Wrong { index: usize, want: Step, got: Step },
    /// A step with no call near it.
    Stray(Step),
    /// Another [`STREAK_EVERY`] hits in a row.
    Streak { run: u32, bonus: u32 },
    /// A shamrock caught: the night's luck points so far.
    Shamrock { luck: u32 },
    /// The four-leaf clover found.
    Clover,
    /// In practice, the tune waits on a step's beat.
    Wait { index: usize, step: Step },
    /// The tune goes on after a wait or a count-in.
    Go,
    /// The tune is over.
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hold {
    /// Practice: waiting for this call's step.
    Wait(usize),
    /// Counting back in after a pause: milliseconds into the count, below
    /// 0 for a silent wait before it.
    CountIn(i64),
}

/// The tallies of a dance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub spot_on: u32,
    pub good: u32,
    pub close: u32,
    pub missed: u32,
    pub waited: u32,
}

impl Tally {
    pub fn hits(&self) -> u32 {
        self.spot_on + self.good + self.close + self.waited
    }

    pub fn add(&mut self, other: &Tally) {
        self.spot_on += other.spot_on;
        self.good += other.good;
        self.close += other.close;
        self.missed += other.missed;
        self.waited += other.waited;
    }

    /// "12 spot on, 8 good, 3 close, 2 missed".
    pub fn words(&self) -> String {
        let mut parts = vec![
            format!("{} spot on", self.spot_on),
            format!("{} good", self.good),
            format!("{} close", self.close),
        ];
        if self.waited > 0 {
            parts.push(format!("{} with the tune waiting", self.waited));
        }
        parts.push(format!("{} missed", self.missed));
        parts.join(", ")
    }
}

/// One tune being danced.
#[derive(Clone, Debug)]
pub struct Dance {
    pub plan: Plan,
    /// The tune waits on each step's beat until it is danced.
    pub practice: bool,
    now: i64,
    judged: Vec<Option<Judgment>>,
    next_call: usize,
    first_open: usize,
    next_count: i64,
    next_part: u32,
    hold: Option<Hold>,
    ended: bool,
    /// The last call made.
    pub last_call: Option<usize>,
    pub streak: u32,
    pub best_streak: u32,
    pub bonus: u32,
    pub luck: u32,
    pub clover: bool,
}

impl Dance {
    pub fn new(plan: Plan, practice: bool) -> Dance {
        let n = plan.calls.len();
        Dance {
            plan,
            practice,
            now: 0,
            judged: vec![None; n],
            next_call: 0,
            first_open: 0,
            next_count: 0,
            next_part: 0,
            hold: None,
            ended: false,
            last_call: None,
            streak: 0,
            best_streak: 0,
            bonus: 0,
            luck: 0,
            clover: false,
        }
    }

    fn beat_ms(&self) -> f64 {
        self.plan.tune.beat_ms()
    }

    /// The time of `beat` (0 is the first beat of the music) in
    /// milliseconds from the start of the count.
    pub fn time_of(&self, beat: i64) -> i64 {
        ((beat + LEAD_BEATS) as f64 * self.beat_ms()).round() as i64
    }

    /// Milliseconds since the count began.
    pub fn now(&self) -> i64 {
        self.now
    }

    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Whether the tune is waiting on a step, in practice.
    pub fn waiting(&self) -> Option<usize> {
        match self.hold {
            Some(Hold::Wait(i)) => Some(i),
            _ => None,
        }
    }

    pub fn counting_in(&self) -> bool {
        matches!(self.hold, Some(Hold::CountIn(_)))
    }

    /// Whether the music has started.
    pub fn music_started(&self) -> bool {
        self.next_part > 0
    }

    pub fn judgment(&self, index: usize) -> Option<Judgment> {
        self.judged[index]
    }

    /// The current bar, 1 to 32, or 0 during the count.
    pub fn bar(&self) -> u32 {
        let since = self.now - self.time_of(0);
        if since < 0 {
            return 0;
        }
        let beat = (since as f64 / self.beat_ms()) as u32;
        (beat / 2 + 1).min(TUNE_BARS)
    }

    /// The next call still to be danced, if any.
    pub fn next_open(&self) -> Option<usize> {
        (self.first_open..self.plan.calls.len()).find(|&i| self.judged[i].is_none())
    }

    /// Whole beats from now to call `index`'s beat, at least 0.
    pub fn beats_until(&self, index: usize) -> u32 {
        let t = self.time_of(self.plan.calls[index].beat as i64);
        ((t - self.now).max(0) as f64 / self.beat_ms()).round() as u32
    }

    /// Points so far, with streak and clover bonuses.
    pub fn score(&self) -> u32 {
        self.points() + self.bonus
    }

    /// Points so far, without bonuses.
    pub fn points(&self) -> u32 {
        self.judged.iter().flatten().map(|j| j.points()).sum()
    }

    pub fn tally(&self) -> Tally {
        let mut t = Tally::default();
        for j in self.judged.iter().flatten() {
            match j {
                Judgment::SpotOn => t.spot_on += 1,
                Judgment::Good => t.good += 1,
                Judgment::Close => t.close += 1,
                Judgment::Waited => t.waited += 1,
                Judgment::Missed | Judgment::Wrong(_) => t.missed += 1,
            }
        }
        t
    }

    pub fn stars(&self) -> u32 {
        stars(self.points(), self.plan.max_points())
    }

    /// Holds the tune for a count of [`COUNT_IN_BEATS`] beats, after a
    /// pause. A practice wait stays as it is.
    pub fn count_in(&mut self) {
        if self.ended || self.waiting().is_some() || !self.music_started() {
            return;
        }
        self.hold = Some(Hold::CountIn(0));
    }

    /// Like [`Dance::count_in`], with `wait_ms` of silence before the
    /// count, so a line can be said first.
    pub fn count_in_after(&mut self, wait_ms: i64) {
        self.count_in();
        if self.counting_in() {
            self.hold = Some(Hold::CountIn(-wait_ms.max(0)));
        }
    }

    /// One millisecond.
    pub fn tick(&mut self, out: &mut Vec<Ev>) {
        if self.ended {
            return;
        }
        match self.hold {
            Some(Hold::Wait(_)) => return,
            Some(Hold::CountIn(ms)) => {
                for k in 0..COUNT_IN_BEATS {
                    if ms == (k as f64 * self.beat_ms()).round() as i64 {
                        out.push(Ev::Count(k as u32 + 1));
                    }
                }
                if ms >= (COUNT_IN_BEATS as f64 * self.beat_ms()).round() as i64 {
                    self.hold = None;
                    out.push(Ev::Go);
                } else {
                    self.hold = Some(Hold::CountIn(ms + 1));
                    return;
                }
            }
            None => {}
        }
        self.now += 1;
        let now = self.now;
        while self.next_count < LEAD_BEATS && self.time_of(self.next_count - LEAD_BEATS) <= now {
            self.next_count += 1;
            out.push(Ev::Count(self.next_count as u32));
        }
        while self.next_part < PARTS && self.time_of((self.next_part * PART_BEATS) as i64) <= now {
            let index = self.next_part;
            self.next_part += 1;
            out.push(Ev::Part {
                index,
                b: index % 4 >= 2,
            });
        }
        while self.next_call < self.plan.calls.len() {
            let c = self.plan.calls[self.next_call];
            if self.time_of(c.beat as i64 - 1) > now {
                break;
            }
            let index = self.next_call;
            self.next_call += 1;
            self.last_call = Some(index);
            out.push(Ev::Call {
                index,
                step: c.step,
                luck: c.luck,
            });
        }
        let mut i = self.first_open;
        while i < self.plan.calls.len() {
            let c = self.plan.calls[i];
            let beat_at = self.time_of(c.beat as i64);
            if beat_at > now {
                break;
            }
            if self.judged[i].is_none() {
                if self.practice {
                    self.hold = Some(Hold::Wait(i));
                    out.push(Ev::Wait {
                        index: i,
                        step: c.step,
                    });
                    return;
                }
                if beat_at + WINDOW_MS < now {
                    self.judged[i] = Some(Judgment::Missed);
                    self.streak = 0;
                    out.push(Ev::Missed {
                        index: i,
                        step: c.step,
                    });
                }
            }
            i += 1;
        }
        while self.first_open < self.plan.calls.len() && self.judged[self.first_open].is_some() {
            self.first_open += 1;
        }
        if self.first_open >= self.plan.calls.len()
            && now >= self.time_of(TUNE_BEATS as i64) + END_MS
        {
            self.ended = true;
            out.push(Ev::Ended);
        }
    }

    /// The player dances `step` now.
    pub fn press(&mut self, step: Step) -> Vec<Ev> {
        let mut out = Vec::new();
        if self.ended || self.counting_in() {
            out.push(Ev::Stray(step));
            return out;
        }
        if let Some(index) = self.waiting() {
            let want = self.plan.calls[index].step;
            if want == step {
                self.land(index, step, Judgment::Waited, &mut out);
                self.hold = None;
                out.push(Ev::Go);
            } else {
                out.push(Ev::Wrong {
                    index,
                    want,
                    got: step,
                });
            }
            return out;
        }
        let near = (self.first_open..self.plan.calls.len())
            .filter(|&i| self.judged[i].is_none())
            .map(|i| (i, self.now - self.time_of(self.plan.calls[i].beat as i64)))
            .take_while(|&(_, off)| off >= -WINDOW_MS)
            .filter(|&(_, off)| off.abs() <= WINDOW_MS)
            .min_by_key(|&(_, off)| off.abs());
        let Some((index, offset)) = near else {
            out.push(Ev::Stray(step));
            return out;
        };
        let want = self.plan.calls[index].step;
        if want != step {
            if self.practice {
                out.push(Ev::Wrong {
                    index,
                    want,
                    got: step,
                });
                return out;
            }
            self.judged[index] = Some(Judgment::Wrong(step));
            self.streak = 0;
            out.push(Ev::Wrong {
                index,
                want,
                got: step,
            });
            return out;
        }
        let judgment = judge(offset).unwrap_or(Judgment::Close);
        self.land(index, step, judgment, &mut out);
        out
    }

    fn land(&mut self, index: usize, step: Step, judgment: Judgment, out: &mut Vec<Ev>) {
        self.judged[index] = Some(judgment);
        out.push(Ev::Judged {
            index,
            step,
            judgment,
        });
        self.streak += 1;
        self.best_streak = self.best_streak.max(self.streak);
        if self.streak.is_multiple_of(STREAK_EVERY) {
            self.bonus += STREAK_BONUS;
            out.push(Ev::Streak {
                run: self.streak,
                bonus: STREAK_BONUS,
            });
        }
        match self.plan.calls[index].luck {
            Luck::Shamrock if judgment.points() >= Judgment::Good.points() => {
                self.luck += 1;
                out.push(Ev::Shamrock { luck: self.luck });
            }
            Luck::Clover => {
                self.clover = true;
                self.bonus += CLOVER_BONUS;
                out.push(Ev::Clover);
            }
            _ => {}
        }
    }
}

/// A finished tune.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuneResult {
    pub tune: &'static Tune,
    pub score: u32,
    pub points: u32,
    pub max: u32,
    pub stars: u32,
    pub tally: Tally,
    pub best_streak: u32,
    pub luck: u32,
    pub shamrocks: u32,
    pub clover: bool,
}

impl TuneResult {
    pub fn of(dance: &Dance) -> TuneResult {
        TuneResult {
            tune: dance.plan.tune,
            score: dance.score(),
            points: dance.points(),
            max: dance.plan.max_points(),
            stars: dance.stars(),
            tally: dance.tally(),
            best_streak: dance.best_streak,
            luck: dance.luck,
            shamrocks: dance.plan.shamrocks() as u32,
            clover: dance.clover,
        }
    }
}

/// What a run of tunes did in a millisecond.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunEv {
    Dance(Ev),
    /// Tune `index` is over.
    TuneDone(usize),
    /// Tune `index` starts its count.
    Next(usize),
    /// Every tune is over.
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Before the first key: the player can hear each step.
    Ready,
    Dancing,
    /// The gap between two tunes.
    Between(i64),
    Done,
}

/// A run: one or more tunes danced in a row.
#[derive(Clone, Debug)]
pub struct Run {
    pub plans: Vec<Plan>,
    pub at: usize,
    pub dance: Dance,
    pub phase: Phase,
    pub results: Vec<TuneResult>,
    clock: FixedStep,
}

impl Run {
    pub fn new(plans: Vec<Plan>, practice: bool) -> Run {
        assert!(!plans.is_empty(), "a run needs a tune");
        let dance = Dance::new(plans[0].clone(), practice);
        Run {
            plans,
            at: 0,
            dance,
            phase: Phase::Ready,
            results: Vec::new(),
            clock: FixedStep::hz(1000.0),
        }
    }

    pub fn started(&self) -> bool {
        self.phase != Phase::Ready
    }

    pub fn finished(&self) -> bool {
        self.phase == Phase::Done
    }

    pub fn practice(&self) -> bool {
        self.dance.practice
    }

    pub fn start(&mut self) {
        if self.phase == Phase::Ready {
            self.phase = Phase::Dancing;
        }
    }

    /// The whole score so far: finished tunes and the one being danced.
    pub fn score(&self) -> u32 {
        let done: u32 = self.results.iter().map(|r| r.score).sum();
        match self.phase {
            Phase::Dancing => done + self.dance.score(),
            _ => done,
        }
    }

    pub fn luck(&self) -> u32 {
        let done: u32 = self.results.iter().map(|r| r.luck).sum();
        match self.phase {
            Phase::Dancing => done + self.dance.luck,
            _ => done,
        }
    }

    pub fn clover(&self) -> bool {
        self.results.iter().any(|r| r.clover) || self.dance.clover
    }

    pub fn tally(&self) -> Tally {
        let mut t = Tally::default();
        for r in &self.results {
            t.add(&r.tally);
        }
        t
    }

    pub fn best_streak(&self) -> u32 {
        self.results
            .iter()
            .map(|r| r.best_streak)
            .chain([self.dance.best_streak])
            .max()
            .unwrap_or(0)
    }

    pub fn update(&mut self, dt: f64) -> Vec<RunEv> {
        let mut out = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |_| self.tick(&mut out));
        self.clock = clock;
        out
    }

    fn tick(&mut self, out: &mut Vec<RunEv>) {
        match self.phase {
            Phase::Ready | Phase::Done => {}
            Phase::Dancing => {
                let mut evs = Vec::new();
                self.dance.tick(&mut evs);
                let ended = evs.contains(&Ev::Ended);
                out.extend(evs.into_iter().map(RunEv::Dance));
                if ended {
                    self.results.push(TuneResult::of(&self.dance));
                    out.push(RunEv::TuneDone(self.at));
                    if self.at + 1 < self.plans.len() {
                        self.phase = Phase::Between(BETWEEN_MS);
                    } else {
                        self.phase = Phase::Done;
                        out.push(RunEv::Finished);
                    }
                }
            }
            Phase::Between(left) => {
                if left > 1 {
                    self.phase = Phase::Between(left - 1);
                } else {
                    self.at += 1;
                    let practice = self.dance.practice;
                    self.dance = Dance::new(self.plans[self.at].clone(), practice);
                    self.phase = Phase::Dancing;
                    out.push(RunEv::Next(self.at));
                }
            }
        }
    }

    /// The player dances `step`.
    pub fn press(&mut self, step: Step) -> Vec<Ev> {
        match self.phase {
            Phase::Dancing => self.dance.press(step),
            _ => vec![Ev::Stray(step)],
        }
    }

    /// Counts back in after a pause.
    pub fn count_in(&mut self) {
        if self.phase == Phase::Dancing {
            self.dance.count_in();
        }
    }

    /// Counts back in after `wait_ms` of silence.
    pub fn count_in_after(&mut self, wait_ms: i64) {
        if self.phase == Phase::Dancing {
            self.dance.count_in_after(wait_ms);
        }
    }
}
