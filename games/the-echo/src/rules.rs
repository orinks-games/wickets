//! A rank of the Echo's pipes, and tuning it: the rules, with no sound or
//! speech.
//!
//! Each pipe should hold one note. It starts out of tune by a few steps of
//! 5 cents, sharp or flat. The game plays the pipe against its true note,
//! and the two wobble against each other: fast when the pipe is far out,
//! slower as it comes near, still when it is in tune. The player raises or
//! lowers the pipe and sets it once the wobble has gone. Setting too soon
//! costs a point or two, and nothing fails.

use std::cell::Cell;

use parlor_core::Rng;

/// Cents a press moves the pipe.
pub const STEP_CENTS: i32 = 5;
/// The farthest a pipe can be pushed, in steps either way.
pub const LIMIT: i32 = 24;
/// Points for a pipe set in tune at the first try with no hint.
pub const PIPE_POINTS: u32 = 10;
/// Points lost for each try at setting a pipe that was still out.
pub const TRY_COST: u32 = 2;
/// Points lost for each hint, where hints cost.
pub const HINT_COST: u32 = 3;
/// The least a pipe in tune scores.
pub const PIPE_FLOOR: u32 = 2;
/// Notes the Echo plays back once a rank is tuned.
pub const TUNE_LEN: usize = 8;

/// The notes a pipe can hold, in semitones from G4, low to high: G major
/// pentatonic from D4 to B5.
pub const NOTES: [i32; 10] = [-5, -3, 0, 2, 4, 7, 9, 12, 14, 16];

/// The shape of a rank.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// Pipes in the rank, up to the 10 notes.
    pub pipes: usize,
    /// How far out a pipe starts, in steps, at least and at most.
    pub near: i32,
    pub far: i32,
    /// Hints cost nothing.
    pub free_hints: bool,
}

/// The evening's tuning: one rank of 6 pipes.
pub const RANK: Layout = Layout {
    pipes: 6,
    near: 3,
    far: 10,
    free_hints: false,
};

/// The practice rank: 4 pipes, hints free.
pub const PRACTICE: Layout = Layout {
    pipes: 4,
    near: 2,
    far: 8,
    free_hints: true,
};

/// One pipe of a rank.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pipe {
    /// Its true note, in semitones from G4.
    pub note: i32,
    /// How far out it is, in steps: sharp above 0, flat below.
    pub off: i32,
    /// Tries at setting it while it was still out.
    pub tries: u32,
    pub hints: u32,
    pub tuned: bool,
    /// What it scored, once tuned.
    pub points: u32,
}

impl Pipe {
    /// The pipe's pitch as a ratio of G4.
    pub fn pitch(&self) -> f32 {
        ratio(self.note, self.off * STEP_CENTS)
    }

    /// The true note's pitch as a ratio of G4.
    pub fn true_pitch(&self) -> f32 {
        ratio(self.note, 0)
    }

    /// Cents out of tune, sharp above 0.
    pub fn cents(&self) -> i32 {
        self.off * STEP_CENTS
    }
}

/// `semis` semitones and `cents` cents above G4, as a pitch ratio.
pub fn ratio(semis: i32, cents: i32) -> f32 {
    2f32.powf((semis * 100 + cents) as f32 / 1200.0)
}

/// Something that happened, for the screen to voice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    /// The pipe moved, up or down.
    Moved {
        up: bool,
    },
    /// The pipe will go no further that way.
    Limit,
    /// Now listening to pipe `index`.
    Chose {
        index: usize,
    },
    /// No pipe further that way.
    End {
        first: bool,
    },
    /// The pipe was set in tune.
    InTune {
        index: usize,
        points: u32,
    },
    /// Set too soon: still out, `near` when one step away.
    NotYet {
        near: bool,
    },
    /// That pipe is already in tune.
    AlreadyTuned,
    /// Which way the pipe is out, or that it is in tune.
    Hint {
        sharp: bool,
        steps: i32,
    },
    HintInTune,
    /// Every pipe in tune.
    RankTuned,
}

/// A rank of pipes as it stands.
#[derive(Clone, Debug)]
pub struct Rank {
    pub layout: Layout,
    pub pipes: Vec<Pipe>,
    /// The pipe being tuned, from 0 at the lowest.
    pub cur: usize,
    pub score: u32,
    /// The tune the Echo plays back, as pipe indexes.
    pub tune: Vec<usize>,
    done: bool,
}

impl Rank {
    /// A rank of `layout`, its notes and offsets drawn from `seed`.
    pub fn new(layout: Layout, seed: u64) -> Rank {
        let mut rng = Rng::seeded(seed);
        let n = layout.pipes.clamp(1, NOTES.len());
        let mut notes = NOTES.to_vec();
        rng.shuffle(&mut notes);
        notes.truncate(n);
        notes.sort_unstable();
        let near = layout.near.max(1);
        let far = layout.far.max(near);
        let pipes = notes
            .into_iter()
            .map(|note| {
                let size = rng.range(near, far);
                let off = if rng.chance(0.5) { size } else { -size };
                Pipe {
                    note,
                    off,
                    tries: 0,
                    hints: 0,
                    tuned: false,
                    points: 0,
                }
            })
            .collect();
        let tune = (0..TUNE_LEN)
            .map(|_| rng.below(n as u32) as usize)
            .collect();
        Rank {
            layout,
            pipes,
            cur: 0,
            score: 0,
            tune,
            done: false,
        }
    }

    /// A rank with these notes and offsets, for the tutorial and tests.
    pub fn fixed(layout: Layout, pipes: &[(i32, i32)]) -> Rank {
        let pipes: Vec<Pipe> = pipes
            .iter()
            .map(|&(note, off)| Pipe {
                note,
                off,
                tries: 0,
                hints: 0,
                tuned: off == 0,
                points: 0,
            })
            .collect();
        let tune = (0..pipes.len()).collect();
        Rank {
            layout,
            pipes,
            cur: 0,
            score: 0,
            tune,
            done: false,
        }
    }

    pub fn finished(&self) -> bool {
        self.done
    }

    pub fn pipe(&self) -> &Pipe {
        &self.pipes[self.cur]
    }

    /// Pipes in tune.
    pub fn tuned(&self) -> usize {
        self.pipes.iter().filter(|p| p.tuned).count()
    }

    /// Pipes set in tune at the first try with no hint.
    pub fn perfect(&self) -> usize {
        self.pipes
            .iter()
            .filter(|p| p.tuned && p.tries == 0 && p.hints == 0)
            .count()
    }

    /// Up raises the pipe a step, Down lowers it.
    pub fn nudge(&mut self, up: bool) -> Vec<Ev> {
        if self.done {
            return Vec::new();
        }
        let p = &mut self.pipes[self.cur];
        if p.tuned {
            return vec![Ev::AlreadyTuned];
        }
        let to = p.off + if up { 1 } else { -1 };
        if to.abs() > LIMIT {
            return vec![Ev::Limit];
        }
        p.off = to;
        vec![Ev::Moved { up }]
    }

    /// Left and Right move to the next pipe down or up the rank.
    pub fn choose(&mut self, higher: bool) -> Vec<Ev> {
        if self.done {
            return Vec::new();
        }
        if higher {
            if self.cur + 1 >= self.pipes.len() {
                return vec![Ev::End { first: false }];
            }
            self.cur += 1;
        } else {
            if self.cur == 0 {
                return vec![Ev::End { first: true }];
            }
            self.cur -= 1;
        }
        vec![Ev::Chose { index: self.cur }]
    }

    /// Sets the pipe: in tune if its wobble has gone.
    pub fn set(&mut self) -> Vec<Ev> {
        if self.done {
            return Vec::new();
        }
        let free = self.layout.free_hints;
        let index = self.cur;
        let p = &mut self.pipes[index];
        if p.tuned {
            return vec![Ev::AlreadyTuned];
        }
        if p.off != 0 {
            p.tries += 1;
            return vec![Ev::NotYet {
                near: p.off.abs() == 1,
            }];
        }
        p.tuned = true;
        let hint_cost = if free { 0 } else { HINT_COST * p.hints };
        p.points = PIPE_POINTS
            .saturating_sub(TRY_COST * p.tries + hint_cost)
            .max(PIPE_FLOOR);
        self.score += p.points;
        let points = p.points;
        let mut evs = vec![Ev::InTune { index, points }];
        if let Some(next) = (index + 1..self.pipes.len())
            .chain(0..index)
            .find(|&i| !self.pipes[i].tuned)
        {
            self.cur = next;
            evs.push(Ev::Chose { index: next });
        } else {
            self.done = true;
            evs.push(Ev::RankTuned);
        }
        evs
    }

    /// Which way the pipe is out.
    pub fn hint(&mut self) -> Vec<Ev> {
        if self.done {
            return Vec::new();
        }
        let p = &mut self.pipes[self.cur];
        if p.tuned {
            return vec![Ev::AlreadyTuned];
        }
        // Confirming a pipe the still tone already says is in tune is free.
        if p.off == 0 {
            return vec![Ev::HintInTune];
        }
        p.hints += 1;
        vec![Ev::Hint {
            sharp: p.off > 0,
            steps: p.off.abs(),
        }]
    }
}

/// "1 pipe", "6 pipes".
pub fn pipes_words(n: usize) -> String {
    match n {
        1 => "1 pipe".to_string(),
        n => format!("{n} pipes"),
    }
}

/// "1 point", "40 points".
pub fn points_words(n: u32) -> String {
    match n {
        1 => "1 point".to_string(),
        n => format!("{n} points"),
    }
}

/// "1 step", "3 steps".
pub fn steps_words(n: i32) -> String {
    match n {
        1 => "1 step".to_string(),
        n => format!("{n} steps"),
    }
}

/// A note's name, from semitones above G4: "G4", "B5".
pub fn note_name(semis: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C sharp", "D", "D sharp", "E", "F", "F sharp", "G", "G sharp", "A", "A sharp", "B",
    ];
    let midi = 67 + semis;
    format!(
        "{} {}",
        NAMES[midi.rem_euclid(12) as usize],
        midi.div_euclid(12) - 1
    )
}

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins every new rank's seed, for tests.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new rank: the pinned one, or one from the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}
