//! Midsummer night as a plain simulation: a swarm of synchronous
//! fireflies round a bonfire flashes together, six beats at a time, and
//! some drift out of step. A press in time with a flash brings one
//! straggler back into the chorus.

use parlor_core::{FixedStep, Rng};

use crate::meadow::P2;

/// Seconds between flashes.
pub const BEAT: f64 = 0.6;
/// Flashes in each burst.
pub const FLASHES: u64 = 6;
/// Beats in a round: the burst, then the dark.
pub const ROUND_BEATS: u64 = 10;
/// Rounds in the show.
pub const ROUNDS: u64 = 12;
/// Fireflies in the swarm.
pub const SWARM: usize = 12;
/// Seconds before the first flash.
pub const LEAD_IN: f64 = 3.0;
/// How late a straggler flashes after the chorus.
pub const LAG: f64 = 0.22;
/// How far from a flash a press may land and still be in step.
pub const WINDOW: f64 = 0.16;

/// Something the screen should voice.
#[derive(Clone, Debug, PartialEq)]
pub enum ShowNote {
    /// A round begins; `drifted` fireflies fell out of step.
    Round { round: u64, drifted: usize },
    /// The chorus flashes; `in_step` fireflies flash together.
    Flash { in_step: usize },
    /// A straggler flashes late, from where it is.
    Straggle { pos: P2 },
    /// The show is over.
    Over,
}

/// What a press did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// In time with a flash; whether a straggler rejoined, and how many
    /// are still out of step.
    OnBeat {
        rejoined: bool,
        left: usize,
    },
    /// Between flashes.
    OffBeat,
    /// That flash was already answered.
    Already,
    /// Before the show has begun.
    NotYet,
    Over,
}

pub struct Show {
    rng: Rng,
    clock: FixedStep,
    pub time: f64,
    /// Where each firefly of the swarm is, round the bonfire at 0, 0.
    pub positions: Vec<P2>,
    pub out_of_step: Vec<bool>,
    /// The next beat to play, counting every beat of the show.
    next_beat: u64,
    pending: Vec<(f64, P2)>,
    answered: Option<u64>,
    pub in_step_flashes: u64,
    pub all_flashes: u64,
    pub on_beat: u32,
    pub off_beat: u32,
    pub rejoined: u32,
    pub over: bool,
    notes: Vec<ShowNote>,
}

impl Show {
    pub fn new(seed: u64) -> Show {
        let mut rng = Rng::seeded(seed);
        let positions = (0..SWARM)
            .map(|i| {
                let angle = i as f64 / SWARM as f64 * std::f64::consts::TAU + rng.unit() * 0.3;
                let r = 3.0 + rng.unit() * 5.0;
                (angle.cos() * r, angle.sin() * r + 2.0)
            })
            .collect();
        Show {
            rng,
            clock: FixedStep::hz(120.0),
            time: 0.0,
            positions,
            out_of_step: vec![false; SWARM],
            next_beat: 0,
            pending: Vec::new(),
            answered: None,
            in_step_flashes: 0,
            all_flashes: 0,
            on_beat: 0,
            off_beat: 0,
            rejoined: 0,
            over: false,
            notes: Vec::new(),
        }
    }

    /// When beat `b` falls.
    pub fn beat_time(b: u64) -> f64 {
        LEAD_IN + b as f64 * BEAT
    }

    /// Whether beat `b` is a flash, not a dark beat.
    pub fn is_flash(b: u64) -> bool {
        b % ROUND_BEATS < FLASHES
    }

    /// The round being played, from 0.
    pub fn round(&self) -> u64 {
        (self.next_beat.saturating_sub(1) / ROUND_BEATS).min(ROUNDS - 1)
    }

    pub fn stragglers(&self) -> usize {
        self.out_of_step.iter().filter(|&&o| o).count()
    }

    /// Stragglers on one side of the fire: `left` true for the west.
    pub fn stragglers_on(&self, left: bool) -> usize {
        self.positions
            .iter()
            .zip(&self.out_of_step)
            .filter(|(p, &o)| o && (p.0 < 0.0) == left)
            .count()
    }

    /// How in step the meadow was, percent of all flashes.
    pub fn harmony(&self) -> u32 {
        (self.in_step_flashes * 100)
            .checked_div(self.all_flashes)
            .map_or(100, |h| h as u32)
    }

    pub fn drain_notes(&mut self) -> Vec<ShowNote> {
        std::mem::take(&mut self.notes)
    }

    pub fn update(&mut self, dt: f64) {
        if self.over {
            return;
        }
        let mut clock = self.clock;
        clock.advance(dt, |step| self.tick(step));
        self.clock = clock;
    }

    fn tick(&mut self, step: f64) {
        if self.over {
            return;
        }
        self.time += step;
        let total = ROUNDS * ROUND_BEATS;
        while self.next_beat < total && Self::beat_time(self.next_beat) <= self.time + 1e-9 {
            let b = self.next_beat;
            self.next_beat += 1;
            self.beat(b);
        }
        let now = self.time;
        let mut due = Vec::new();
        self.pending.retain(|&(t, pos)| {
            if t <= now + 1e-9 {
                due.push(pos);
                false
            } else {
                true
            }
        });
        self.notes
            .extend(due.into_iter().map(|pos| ShowNote::Straggle { pos }));
        if self.next_beat >= total && self.time >= Self::beat_time(total) + 1.0 {
            self.over = true;
            self.notes.push(ShowNote::Over);
        }
    }

    fn beat(&mut self, b: u64) {
        let k = b % ROUND_BEATS;
        if k == 0 {
            let round = b / ROUND_BEATS;
            let want = if round == 0 {
                2
            } else {
                1 + self.rng.below(3) as usize
            };
            let mut drifted = 0;
            for _ in 0..want {
                let in_step: Vec<usize> = (0..SWARM).filter(|&i| !self.out_of_step[i]).collect();
                if in_step.len() <= SWARM / 2 {
                    break;
                }
                let i = *self.rng.pick(&in_step);
                self.out_of_step[i] = true;
                drifted += 1;
            }
            self.notes.push(ShowNote::Round { round, drifted });
        }
        if k < FLASHES {
            let in_step = SWARM - self.stragglers();
            self.in_step_flashes += in_step as u64;
            self.all_flashes += SWARM as u64;
            self.notes.push(ShowNote::Flash { in_step });
            let late = self.time + LAG;
            for i in 0..SWARM {
                if self.out_of_step[i] {
                    self.pending.push((late, self.positions[i]));
                }
            }
        }
    }

    /// A press of the main action.
    pub fn press(&mut self) -> Press {
        if self.over {
            return Press::Over;
        }
        if self.time < LEAD_IN - WINDOW {
            return Press::NotYet;
        }
        let b = ((self.time - LEAD_IN) / BEAT).round().max(0.0) as u64;
        let on = b < ROUNDS * ROUND_BEATS
            && Self::is_flash(b)
            && (self.time - Self::beat_time(b)).abs() <= WINDOW;
        if !on {
            self.off_beat += 1;
            return Press::OffBeat;
        }
        if self.answered == Some(b) {
            return Press::Already;
        }
        self.answered = Some(b);
        self.on_beat += 1;
        let straggler = (0..SWARM).find(|&i| self.out_of_step[i]);
        if let Some(i) = straggler {
            self.out_of_step[i] = false;
            self.rejoined += 1;
        }
        Press::OnBeat {
            rejoined: straggler.is_some(),
            left: self.stragglers(),
        }
    }
}
