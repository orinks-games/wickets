//! The rules of Belfry, as plain structs with no screen or sound: the
//! secret peal, the answer to a rung row, hints, scoring, and the ringer
//! that spaces a row's bells and answer out in time.

use std::cell::Cell;

use parlor_core::{FixedStep, Rng};

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins every new peal's seed, for tests. `None` goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// The seed a new peal uses: the pinned one, else the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}

/// The shape of a peal: how many places, how many bells to choose from,
/// whether a bell may come twice, how many rows the tower allows, and
/// whether hints are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    pub places: usize,
    pub bells: u8,
    pub repeats: bool,
    pub rows: u32,
    pub hints: bool,
}

/// Four places, six bells, repeats allowed, ten rows.
pub const VILLAGE: Rules = Rules {
    places: 4,
    bells: 6,
    repeats: true,
    rows: 10,
    hints: true,
};

/// Village with no bell twice in the peal, for beginners.
pub const PLAIN: Rules = Rules {
    repeats: false,
    ..VILLAGE
};

/// Five places, eight bells, repeats allowed, twelve rows, no hints. The
/// monthly challenge rings it.
pub const CATHEDRAL: Rules = Rules {
    places: 5,
    bells: 8,
    repeats: true,
    rows: 12,
    hints: false,
};

impl Rules {
    /// "4 bells from 6, repeats allowed, 10 rows".
    pub fn describe(&self) -> String {
        format!(
            "{} places, {} bells to choose from, {}, {} rows",
            self.places,
            self.bells,
            if self.repeats {
                "a bell may come twice"
            } else {
                "no bell twice"
            },
            self.rows
        )
    }
}

/// Five places, six bells, repeats allowed, ten rows: the practice night
/// peal, where the tower captain calls one place.
pub const PRACTICE_NIGHT: Rules = Rules {
    places: 5,
    ..VILLAGE
};

/// A mode on the title, with its own records board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Village,
    Plain,
    /// Practice night, on the second Wednesday of every month only.
    PracticeNight,
}

impl Mode {
    /// The modes always on the title.
    pub const ALL: [Mode; 2] = [Mode::Village, Mode::Plain];
    /// The modes on the title only on their nights.
    pub const EVENTS: [Mode; 1] = [Mode::PracticeNight];

    pub fn rules(self) -> Rules {
        match self {
            Mode::Village => VILLAGE,
            Mode::Plain => PLAIN,
            Mode::PracticeNight => PRACTICE_NIGHT,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Village => "Village",
            Mode::Plain => "Plain",
            Mode::PracticeNight => crate::events::PRACTICE_NAME,
        }
    }

    /// The records board: `belfry/scores` key `village`, `plain`,
    /// `moonlit` is the practice night's board, from its old name.
    pub fn board(self) -> &'static str {
        match self {
            Mode::Village => "village",
            Mode::Plain => "plain",
            Mode::PracticeNight => "moonlit",
        }
    }

    /// The help for a fresh peal of this mode, on the results menu.
    pub fn again_help(self) -> String {
        match self {
            Mode::PracticeNight => "Another practice night peal.".to_string(),
            m => format!("A new {} peal.", m.name()),
        }
    }
}

/// A bell's pitch word, by its number and the size of the set. Bell 1, the
/// treble, is the highest; the last, the tenor, the lowest.
pub fn bell_name(bell: u8, bells: u8) -> &'static str {
    const SIX: [&str; 6] = [
        "the treble",
        "high",
        "middle high",
        "middle low",
        "low",
        "the tenor",
    ];
    const EIGHT: [&str; 8] = [
        "the treble",
        "high",
        "upper middle",
        "middle high",
        "middle low",
        "lower middle",
        "low",
        "the tenor",
    ];
    let i = bell.saturating_sub(1) as usize;
    if bells <= 6 {
        SIX.get(i).copied().unwrap_or("")
    } else {
        EIGHT.get(i).copied().unwrap_or("")
    }
}

/// "Bell 3, middle high".
pub fn bell_words(bell: u8, bells: u8) -> String {
    format!("Bell {bell}, {}", bell_name(bell, bells))
}

/// "1, 1, 4, 5".
pub fn row_words(row: &[u8]) -> String {
    row.iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The answer to a rung row: bells right and in their place, and right
/// bells in the wrong place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Answer {
    pub in_place: u32,
    pub misplaced: u32,
}

impl Answer {
    /// "2 in place, 1 misplaced", "None of these bells is in the peal".
    pub fn words(&self) -> String {
        match (self.in_place, self.misplaced) {
            (0, 0) => "None of these bells is in the peal".to_string(),
            (i, m) => {
                let place = match i {
                    0 => "none in place".to_string(),
                    i => format!("{i} in place"),
                };
                let mis = match m {
                    0 => "none misplaced".to_string(),
                    m => format!("{m} misplaced"),
                };
                hearken::text::sentence_start(&format!("{place}, {mis}"))
            }
        }
    }

    /// The words for a row of `places` bells: a row of 2 with no bell in
    /// the peal says "Neither bell is in the peal".
    pub fn words_for(&self, places: usize) -> String {
        if places == 2 && *self == Answer::default() {
            "Neither bell is in the peal".to_string()
        } else {
            self.words()
        }
    }
}

/// The answer rule. Each bell of the guess counts at most once: first the
/// bells in their right place, then, of the rest, as many of each bell as
/// the code still holds.
pub fn score_guess(code: &[u8], guess: &[u8]) -> Answer {
    let in_place = code.iter().zip(guess).filter(|(c, g)| c == g).count() as u32;
    let top = code.iter().chain(guess).copied().max().unwrap_or(0) as usize;
    let mut in_code = vec![0u32; top + 1];
    let mut in_guess = vec![0u32; top + 1];
    for &c in code {
        in_code[c as usize] += 1;
    }
    for &g in guess {
        in_guess[g as usize] += 1;
    }
    let shared: u32 = in_code.iter().zip(&in_guess).map(|(a, b)| *a.min(b)).sum();
    Answer {
        in_place,
        misplaced: shared - in_place,
    }
}

/// A secret peal for these rules, from `rng`.
pub fn make_code(rules: &Rules, rng: &mut Rng) -> Vec<u8> {
    if rules.repeats {
        (0..rules.places)
            .map(|_| 1 + rng.below(rules.bells as u32) as u8)
            .collect()
    } else {
        let mut all: Vec<u8> = (1..=rules.bells).collect();
        rng.shuffle(&mut all);
        all.truncate(rules.places);
        all
    }
}

/// A hint: whether one bell is in the peal. Always true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    pub bell: u8,
    pub inside: bool,
}

impl Hint {
    /// "Bell 5 is in the peal", "Bell 2 is not in the peal".
    pub fn words(&self) -> String {
        if self.inside {
            format!("Bell {} is in the peal", self.bell)
        } else {
            format!("Bell {} is not in the peal", self.bell)
        }
    }
}

/// One row of the tower's log: a rung row and its answer, or a hint, which
/// takes a row of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Row { bells: Vec<u8>, answer: Answer },
    Hint(Hint),
}

/// Why a ring or a hint did not happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The peal is solved or the rows are spent.
    Finished,
    /// Hints are off under these rules.
    HintsOff,
    /// A hint takes a row, and the last row is kept for ringing.
    HintNeedsRows,
    /// Every bell has had its hint.
    NothingToHint,
}

/// How a peal ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Solved,
    OutOfRows,
}

/// Points for solving, before rows left and hints.
pub const SOLVE_POINTS: u64 = 100;
/// Points for each row left unused.
pub const ROW_POINTS: u64 = 100;

/// One peal in the tower: the secret, the row being built, and the log.
#[derive(Clone, Debug)]
pub struct Peal {
    pub rules: Rules,
    pub code: Vec<u8>,
    /// The row being built, bell numbers from 1.
    pub row: Vec<u8>,
    /// The place the cursor is on, from 0.
    pub place: usize,
    pub log: Vec<Entry>,
    pub hints_used: u32,
    pub outcome: Option<Outcome>,
    /// Seconds spent on the peal.
    pub time: f64,
    rng: Rng,
}

impl Peal {
    pub fn new(rules: Rules, seed: u64) -> Peal {
        let mut rng = Rng::seeded(seed);
        let code = make_code(&rules, &mut rng);
        let row = (0..rules.places)
            .map(|i| 1 + (i as u8 % rules.bells))
            .collect();
        let mut peal = Peal {
            rules,
            code,
            row,
            place: 0,
            log: Vec::new(),
            hints_used: 0,
            outcome: None,
            time: 0.0,
            rng,
        };
        peal.unsolve_start(None);
        peal
    }

    /// Keeps the starting row from being the peal itself, which one ring
    /// would find untouched: two of its bells change places. `keep` is a
    /// place that stays, such as one the tower captain called.
    pub fn unsolve_start(&mut self, keep: Option<usize>) {
        if self.row != self.code {
            return;
        }
        let places: Vec<usize> = (0..self.row.len()).filter(|&i| Some(i) != keep).collect();
        for (n, &a) in places.iter().enumerate() {
            if let Some(&b) = places[n + 1..]
                .iter()
                .find(|&&b| self.row[b] != self.row[a])
            {
                self.row.swap(a, b);
                return;
            }
        }
    }

    pub fn rows_used(&self) -> u32 {
        self.log.len() as u32
    }

    pub fn rows_left(&self) -> u32 {
        self.rules.rows.saturating_sub(self.rows_used())
    }

    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn solved(&self) -> bool {
        self.outcome == Some(Outcome::Solved)
    }

    /// The bell in the current place.
    pub fn bell(&self) -> u8 {
        self.row[self.place]
    }

    /// Moves to the next place, or the one before, wrapping. Returns it.
    pub fn move_place(&mut self, forward: bool) -> usize {
        let n = self.rules.places;
        self.place = if forward {
            (self.place + 1) % n
        } else {
            (self.place + n - 1) % n
        };
        self.place
    }

    /// Changes the bell in the current place to the next number, or the
    /// one before, wrapping. Returns the new bell.
    pub fn change_bell(&mut self, up: bool) -> u8 {
        let n = self.rules.bells;
        let b = self.row[self.place];
        let b = if up { b % n + 1 } else { (b + n - 2) % n + 1 };
        self.row[self.place] = b;
        b
    }

    /// Rings the row: logs it with its answer, and ends the peal when it is
    /// solved or the rows are spent.
    pub fn ring(&mut self) -> Result<Answer, Refusal> {
        if self.finished() || self.rows_left() == 0 {
            return Err(Refusal::Finished);
        }
        let answer = score_guess(&self.code, &self.row);
        self.log.push(Entry::Row {
            bells: self.row.clone(),
            answer,
        });
        if answer.in_place as usize == self.rules.places {
            self.outcome = Some(Outcome::Solved);
        } else if self.rows_left() == 0 {
            self.outcome = Some(Outcome::OutOfRows);
        }
        Ok(answer)
    }

    /// Names one bell, not named before, that is or is not in the peal. It
    /// takes a row, and needs a row left after it for ringing.
    pub fn hint(&mut self) -> Result<Hint, Refusal> {
        if self.finished() {
            return Err(Refusal::Finished);
        }
        if !self.rules.hints {
            return Err(Refusal::HintsOff);
        }
        if self.rows_left() < 2 {
            return Err(Refusal::HintNeedsRows);
        }
        let named: Vec<u8> = self
            .log
            .iter()
            .filter_map(|e| match e {
                Entry::Hint(h) => Some(h.bell),
                _ => None,
            })
            .collect();
        let open: Vec<u8> = (1..=self.rules.bells)
            .filter(|b| !named.contains(b))
            .collect();
        if open.is_empty() {
            return Err(Refusal::NothingToHint);
        }
        // A bell no row has rung yet tells you something new.
        let untried: Vec<u8> = open
            .iter()
            .copied()
            .filter(|b| {
                !self.log.iter().any(|e| match e {
                    Entry::Row { bells, .. } => bells.contains(b),
                    _ => false,
                })
            })
            .collect();
        let bell = if untried.is_empty() {
            *self.rng.pick(&open)
        } else {
            *self.rng.pick(&untried)
        };
        let hint = Hint {
            bell,
            inside: self.code.contains(&bell),
        };
        self.log.push(Entry::Hint(hint));
        self.hints_used += 1;
        Ok(hint)
    }

    /// The last rung row's answer.
    pub fn last_answer(&self) -> Option<Answer> {
        self.log.iter().rev().find_map(|e| match e {
            Entry::Row { answer, .. } => Some(*answer),
            _ => None,
        })
    }

    /// Rows left times 100, plus 100 for solving, halved for each hint.
    /// Nothing for a peal not solved.
    pub fn score(&self) -> u64 {
        if !self.solved() {
            return 0;
        }
        let raw = self.rows_left() as u64 * ROW_POINTS + SOLVE_POINTS;
        raw >> self.hints_used.min(16)
    }

    /// Time passing while the peal is open.
    pub fn update(&mut self, dt: f64) {
        if !self.finished() && dt.is_finite() && dt > 0.0 {
            self.time += dt;
        }
    }

    /// "Row 2: 1, 1, 4, 5. 2 in place, 1 misplaced." for each log entry.
    pub fn log_lines(&self) -> Vec<String> {
        self.log
            .iter()
            .enumerate()
            .map(|(i, e)| match e {
                Entry::Row { bells, answer } => {
                    format!(
                        "Row {}: {}. {}.",
                        i + 1,
                        row_words(bells),
                        answer.words_for(self.rules.places)
                    )
                }
                Entry::Hint(h) => format!("Row {}, hint: {}.", i + 1, h.words()),
            })
            .collect()
    }
}

/// A sound the ringer asks for, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cue {
    Bell(u8),
    /// A bell right and in its place.
    Knock,
    /// A right bell in the wrong place.
    Tap,
    /// The spoken answer, once the sounds are done.
    Speak(String),
}

/// The ringer's clock rate: cues land on these steps at any frame rate.
pub const RINGER_HZ: f64 = 120.0;
/// Seconds between the bells of a row.
pub const BELL_GAP: f64 = 0.4;
/// Seconds between knocks and taps.
pub const KNOCK_GAP: f64 = 0.25;
/// The pause between the last bell and the first knock.
pub const ANSWER_PAUSE: f64 = 0.5;

/// Spaces sounds out in time on a fixed clock, so a row rings the same at
/// any frame rate.
#[derive(Clone, Debug)]
pub struct Ringer {
    clock: FixedStep,
    /// Cues waiting, by the step they are due on.
    queue: Vec<(u64, Cue)>,
    /// The step the next cue goes after.
    tail: u64,
}

impl Default for Ringer {
    fn default() -> Self {
        Ringer::new()
    }
}

impl Ringer {
    pub fn new() -> Ringer {
        Ringer {
            clock: FixedStep::hz(RINGER_HZ),
            queue: Vec::new(),
            tail: 0,
        }
    }

    fn steps(secs: f64) -> u64 {
        (secs * RINGER_HZ).round() as u64
    }

    /// Whether any cue is still waiting.
    pub fn busy(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Adds `cue` `gap` seconds after the last one queued, or after now.
    pub fn after(&mut self, gap: f64, cue: Cue) {
        let now = self.clock.steps();
        let base = if self.queue.is_empty() {
            now
        } else {
            self.tail
        };
        self.tail = base + Self::steps(gap);
        self.queue.push((self.tail, cue));
    }

    /// Queues a row: each bell, then after a pause the knocks and taps of
    /// its answer, if it has one, then `speak`.
    pub fn row(&mut self, bells: &[u8], answer: Option<Answer>, speak: Option<String>) {
        for (i, &b) in bells.iter().enumerate() {
            self.after(if i == 0 { 0.0 } else { BELL_GAP }, Cue::Bell(b));
        }
        let mut gap = ANSWER_PAUSE;
        if let Some(a) = answer {
            for _ in 0..a.in_place {
                self.after(gap, Cue::Knock);
                gap = KNOCK_GAP;
            }
            for _ in 0..a.misplaced {
                self.after(gap, Cue::Tap);
                gap = KNOCK_GAP;
            }
        }
        if let Some(text) = speak {
            self.after(gap, Cue::Speak(text));
        }
    }

    /// Queues an answer's knocks and taps alone, then `speak`.
    pub fn answer(&mut self, answer: Answer, speak: Option<String>) {
        let mut gap = 0.0;
        for _ in 0..answer.in_place {
            self.after(gap, Cue::Knock);
            gap = KNOCK_GAP;
        }
        for _ in 0..answer.misplaced {
            self.after(gap, Cue::Tap);
            gap = KNOCK_GAP;
        }
        if let Some(text) = speak {
            self.after(gap.max(KNOCK_GAP), Cue::Speak(text));
        }
    }

    /// The cues due now, without advancing time: those queued with no gap.
    pub fn due(&mut self) -> Vec<Cue> {
        let now = self.clock.steps();
        let mut out = Vec::new();
        self.queue.retain(|(at, cue)| {
            if *at <= now {
                out.push(cue.clone());
                false
            } else {
                true
            }
        });
        out
    }

    /// Time passing: the cues now due, in order.
    pub fn update(&mut self, dt: f64) -> Vec<Cue> {
        self.clock.advance(dt, |_| {});
        self.due()
    }

    /// Drops every waiting cue and returns them, so a key can cut a row
    /// short and still speak its answer.
    pub fn flush(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.queue)
            .into_iter()
            .map(|(_, c)| c)
            .collect()
    }
}

/// "1 point", "250 points".
pub fn points(n: u64) -> String {
    if n == 1 {
        "1 point".to_string()
    } else {
        format!("{n} points")
    }
}
