//! A session on lane nine, with no sound or screen: where the player
//! stands and aims, the swing, the ball's roll, the pinfall and the
//! scoring, driven by `dt` on fixed substeps.

use parlor_core::{FixedStep, Rng};

use crate::rules::{
    pinfall, pinfall_live, pull, Game, Line, Rack, AIM_MAX, CENTRE, STAND_MAX, STAND_MIN,
    SWING_END, SWING_TOP,
};

/// Substeps a second. The swing is timed in whole substeps, so a release
/// lands on the same substep at any frame rate.
pub const RATE: f64 = 120.0;
/// Seconds from the release to the ball reaching the head pin.
pub const ROLL_TIME: f64 = 2.4;
/// Seconds from the ball reaching the pins to the spoken result, while
/// the pins fall.
pub const CLATTER: f64 = 0.7;
/// Seconds of the pinsetter's sweep, until the ball is back.
pub const SWEEP: f64 = 1.2;

/// What the session is: whole games, or single balls at set leaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A series of this many ten-frame games.
    Games(u32),
    /// One ball at each leave; a leave is made when every pin falls.
    Leaves(Vec<Rack>),
}

/// Where the session is in the roll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Choosing the board and the aim.
    Ready,
    /// The swing, `steps` substeps after it began.
    Swing { steps: u32 },
    /// The ball on its way, `t` seconds after the release.
    Rolling { t: f64 },
    /// The pins falling, `t` seconds after the ball reached them.
    Clatter { t: f64 },
    /// The pinsetter sweeping and the ball coming back.
    Sweep { t: f64 },
    /// Nothing left to bowl.
    Over,
}

/// How a roll turned out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Strike,
    Spare,
    /// The pins left standing are split.
    Split,
    /// Not every pin fell and the frame goes on or ends open.
    Open,
}

/// One finished roll, as the result line reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roll {
    /// The game, from 1, in a series.
    pub game: usize,
    /// The frame (1 to 10) and the roll in it, or the leave and 1.
    pub frame: usize,
    pub roll: usize,
    /// The pins standing before the ball.
    pub before: Rack,
    pub knocked: Rack,
    /// The pins still standing after it.
    pub standing: Rack,
    pub mark: Mark,
    pub gutter: bool,
    /// The release, seconds off the top of the swing.
    pub error: f64,
    /// Strikes in a row, this one included.
    pub streak: u32,
}

/// What happened in an update or on a key, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// The player stepped to this board.
    Step(i32),
    /// The player is at the edge of the approach.
    StepEdge(i32),
    /// The aim turned to this many steps.
    Aim(i32),
    /// The aim is as far as it turns.
    AimEdge(i32),
    /// The swing began.
    Swing,
    /// The ball left the hand, `error` seconds off the top of the swing,
    /// pulled `pull` boards.
    Released { error: f64, pull: f64 },
    /// The ball dropped into the gutter.
    Gutter,
    /// The ball reached the pins, knocking `down` of them.
    Impact { down: u8 },
    /// The roll is scored.
    Result(Roll),
    /// The pinsetter is clearing the deck for a full rack.
    NewRack,
    /// The ball is back: ready to roll.
    Ready,
    /// Game `game` (from 1) of the series is over, scoring `score`.
    GameOver { game: usize, score: u32 },
    /// The whole session is over.
    Over,
    /// The key does nothing now; the reason.
    Busy(Busy),
}

/// Why a key does nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Busy {
    Rolling,
    Sweeping,
    Swinging,
    Over,
}

/// A ball on its way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Throw {
    pub line: Line,
    pub error: f64,
    pub knocked: Rack,
    gutter_called: bool,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub kind: Kind,
    /// The game in progress, in a series.
    pub game: Game,
    /// Games finished, in order.
    pub games: Vec<Game>,
    /// The leave in progress, from 0, in a leaves session.
    pub leave: usize,
    /// Leaves made so far.
    pub made: u32,
    /// The pins standing now.
    pub rack: Rack,
    pub stand: i32,
    pub aim: i32,
    pub phase: Phase,
    pub throw: Option<Throw>,
    /// The last scored roll.
    pub last: Option<Roll>,
    /// Strikes in a row now, and the most in a row this session.
    pub streak: u32,
    pub best_streak: u32,
    pub strikes: u32,
    pub spares: u32,
    rng: Rng,
    clock: FixedStep,
    events: Vec<Ev>,
}

impl Session {
    pub fn new(kind: Kind, seed: u64) -> Session {
        let rack = match &kind {
            Kind::Games(_) => Rack::FULL,
            Kind::Leaves(leaves) => leaves.first().copied().unwrap_or(Rack::FULL),
        };
        Session {
            kind,
            game: Game::new(),
            games: Vec::new(),
            leave: 0,
            made: 0,
            rack,
            stand: CENTRE,
            aim: 0,
            phase: Phase::Ready,
            throw: None,
            last: None,
            streak: 0,
            best_streak: 0,
            strikes: 0,
            spares: 0,
            rng: Rng::seeded(seed),
            clock: FixedStep::hz(RATE),
            events: Vec::new(),
        }
    }

    /// One full game.
    pub fn single(seed: u64) -> Session {
        Session::new(Kind::Games(1), seed)
    }

    pub fn ready(&self) -> bool {
        self.phase == Phase::Ready
    }

    pub fn over(&self) -> bool {
        self.phase == Phase::Over
    }

    /// The games in the series, or 0 for leaves.
    pub fn series(&self) -> u32 {
        match self.kind {
            Kind::Games(n) => n,
            Kind::Leaves(_) => 0,
        }
    }

    /// The leaves to shoot, for a leaves session.
    pub fn leaves(&self) -> Option<&[Rack]> {
        match &self.kind {
            Kind::Leaves(l) => Some(l),
            Kind::Games(_) => None,
        }
    }

    /// The game number now, from 1.
    pub fn game_number(&self) -> usize {
        self.games.len() + 1
    }

    /// The frame and roll of the next ball, or the leave and 1.
    pub fn position(&self) -> Option<(usize, usize)> {
        match &self.kind {
            Kind::Games(_) => self.game.next(),
            Kind::Leaves(l) => (self.leave < l.len()).then_some((self.leave + 1, 1)),
        }
    }

    /// Every game's score so far, the one in progress last.
    pub fn series_total(&self) -> u32 {
        self.games.iter().map(Game::score).sum::<u32>() + self.game.score()
    }

    /// The board the ball would cross at the pins with a clean release.
    pub fn target(&self) -> i32 {
        self.stand + self.aim
    }

    fn emit(&mut self, ev: Ev) {
        self.events.push(ev);
    }

    fn take(&mut self) -> Vec<Ev> {
        std::mem::take(&mut self.events)
    }

    fn busy(&self) -> Option<Busy> {
        match self.phase {
            Phase::Ready => None,
            Phase::Swing { .. } => Some(Busy::Swinging),
            Phase::Rolling { .. } | Phase::Clatter { .. } => Some(Busy::Rolling),
            Phase::Sweep { .. } => Some(Busy::Sweeping),
            Phase::Over => Some(Busy::Over),
        }
    }

    /// Steps `by` boards along the approach: negative to the left.
    pub fn step(&mut self, by: i32) -> Vec<Ev> {
        match self.busy() {
            Some(b @ (Busy::Swinging | Busy::Over)) => self.emit(Ev::Busy(b)),
            _ => {
                let to = (self.stand + by).clamp(STAND_MIN, STAND_MAX);
                if to == self.stand {
                    self.emit(Ev::StepEdge(to));
                } else {
                    self.stand = to;
                    self.emit(Ev::Step(to));
                }
            }
        }
        self.take()
    }

    /// Turns the aim `by` steps: negative to the left.
    pub fn turn(&mut self, by: i32) -> Vec<Ev> {
        match self.busy() {
            Some(b @ (Busy::Swinging | Busy::Over)) => self.emit(Ev::Busy(b)),
            _ => {
                let to = (self.aim + by).clamp(-AIM_MAX, AIM_MAX);
                if to == self.aim {
                    self.emit(Ev::AimEdge(to));
                } else {
                    self.aim = to;
                    self.emit(Ev::Aim(to));
                }
            }
        }
        self.take()
    }

    /// The main action: start the swing, or let go of the ball.
    pub fn primary(&mut self) -> Vec<Ev> {
        match self.phase {
            Phase::Ready => {
                self.phase = Phase::Swing { steps: 0 };
                self.clock.reset();
                self.emit(Ev::Swing);
            }
            Phase::Swing { steps } => self.release(steps),
            _ => {
                let b = self.busy().unwrap_or(Busy::Rolling);
                self.emit(Ev::Busy(b));
            }
        }
        self.take()
    }

    /// Stops a swing without letting go, as when the game pauses. Returns
    /// whether a swing was running.
    pub fn cancel_swing(&mut self) -> bool {
        if matches!(self.phase, Phase::Swing { .. }) {
            self.phase = Phase::Ready;
            true
        } else {
            false
        }
    }

    /// How far through the swing, in seconds, while it runs.
    pub fn swing_time(&self) -> Option<f64> {
        match self.phase {
            Phase::Swing { steps } => Some(steps as f64 / RATE),
            _ => None,
        }
    }

    /// The ball's progress down the lane, from 0 to 1, while it rolls.
    pub fn ball_progress(&self) -> Option<f64> {
        match self.phase {
            Phase::Rolling { t } => Some((t / ROLL_TIME).min(1.0)),
            _ => None,
        }
    }

    /// The board under the ball, while it rolls.
    pub fn ball_board(&self) -> Option<f64> {
        let p = self.ball_progress()?;
        self.throw.map(|t| t.line.board_at(p))
    }

    fn release(&mut self, steps: u32) {
        let error = steps as f64 / RATE - SWING_TOP;
        let pulled = pull(error, &mut self.rng);
        let line = Line::new(self.stand, self.aim, pulled);
        let knocked = pinfall_live(self.rack, &line, &mut self.rng);
        self.throw = Some(Throw {
            line,
            error,
            knocked,
            gutter_called: false,
        });
        self.phase = Phase::Rolling { t: 0.0 };
        self.emit(Ev::Released {
            error,
            pull: pulled,
        });
    }

    /// Runs the session for `dt` seconds.
    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        let mut clock = self.clock;
        clock.advance(dt, |step| self.substep(step));
        self.clock = clock;
        self.take()
    }

    fn substep(&mut self, step: f64) {
        match self.phase {
            Phase::Ready | Phase::Over => {}
            Phase::Swing { steps } => {
                let steps = steps + 1;
                self.phase = Phase::Swing { steps };
                if steps as f64 / RATE >= SWING_END - 1e-9 {
                    self.release(steps);
                }
            }
            Phase::Rolling { t } => {
                let t = t + step;
                let mut throw = self.throw.expect("a ball in play");
                if let Some(edge) = throw.line.gutter_at() {
                    if !throw.gutter_called && t / ROLL_TIME >= edge {
                        throw.gutter_called = true;
                        self.throw = Some(throw);
                        self.emit(Ev::Gutter);
                    }
                }
                if t >= ROLL_TIME - 1e-9 {
                    self.phase = Phase::Clatter { t: 0.0 };
                    self.emit(Ev::Impact {
                        down: throw.knocked.count(),
                    });
                } else {
                    self.phase = Phase::Rolling { t };
                }
            }
            Phase::Clatter { t } => {
                let t = t + step;
                if t >= CLATTER - 1e-9 {
                    self.score_roll();
                } else {
                    self.phase = Phase::Clatter { t };
                }
            }
            Phase::Sweep { t } => {
                let t = t + step;
                if t >= SWEEP - 1e-9 {
                    self.phase = Phase::Ready;
                    self.emit(Ev::Ready);
                } else {
                    self.phase = Phase::Sweep { t };
                }
            }
        }
    }

    fn score_roll(&mut self) {
        let throw = self.throw.take().expect("a ball in play");
        let before = self.rack;
        let standing = before.without(throw.knocked);
        // Only the first ball at a newly set rack can strike or split.
        let fresh = match &self.kind {
            Kind::Games(_) => self.game.new_rack(),
            Kind::Leaves(_) => false,
        };
        let cleared = standing.is_empty();
        let mark = if cleared && fresh {
            Mark::Strike
        } else if cleared {
            Mark::Spare
        } else if fresh && standing.is_split() {
            Mark::Split
        } else {
            Mark::Open
        };
        match mark {
            Mark::Strike => {
                self.streak += 1;
                self.strikes += 1;
                self.best_streak = self.best_streak.max(self.streak);
            }
            Mark::Spare => {
                self.streak = 0;
                self.spares += 1;
            }
            _ => self.streak = 0,
        }
        let (game, frame, roll) = match &self.kind {
            Kind::Games(_) => {
                let (f, r) = self.game.next().expect("a roll to score");
                (self.game_number(), f, r)
            }
            Kind::Leaves(_) => (1, self.leave + 1, 1),
        };
        let result = Roll {
            game,
            frame,
            roll,
            before,
            knocked: throw.knocked,
            standing,
            mark,
            gutter: throw.line.gutter(),
            error: throw.error,
            streak: self.streak,
        };
        self.last = Some(result);
        self.emit(Ev::Result(result));
        match self.kind.clone() {
            Kind::Games(series) => {
                self.game.roll(throw.knocked.count());
                if self.game.is_over() {
                    let score = self.game.score();
                    let done = std::mem::take(&mut self.game);
                    self.games.push(done);
                    self.emit(Ev::GameOver {
                        game: self.games.len(),
                        score,
                    });
                    if self.games.len() as u32 >= series {
                        self.phase = Phase::Over;
                        self.emit(Ev::Over);
                        return;
                    }
                    self.rack = Rack::FULL;
                } else if self.game.fresh_rack() {
                    self.rack = Rack::FULL;
                } else {
                    self.rack = standing;
                }
            }
            Kind::Leaves(leaves) => {
                if cleared {
                    self.made += 1;
                }
                self.leave += 1;
                if self.leave >= leaves.len() {
                    self.phase = Phase::Over;
                    self.emit(Ev::Over);
                    return;
                }
                self.rack = leaves[self.leave];
            }
        }
        if self.rack == Rack::FULL || self.leaves().is_some() {
            self.emit(Ev::NewRack);
        }
        self.phase = Phase::Sweep { t: 0.0 };
    }

    /// The board and aim that knock down the most pins standing now with
    /// a clean release, nearest the player's own board first.
    pub fn suggestion(&self) -> (i32, i32) {
        best_line(self.rack, self.stand)
    }
}

/// The pocket lines for a full rack, angled in from either side of the
/// head pin. Each strikes with a clean ball on the clean model, and
/// carries best on the live lane.
pub const POCKET_LEFT: (i32, i32) = (12, 5);
pub const POCKET_RIGHT: (i32, i32) = (28, -5);

/// The board and aim that knock down the most of `rack` with a clean
/// release, preferring to stay near `stand` and to aim straight. A full
/// rack takes the pocket line on the player's side of the lane.
pub fn best_line(rack: Rack, stand: i32) -> (i32, i32) {
    if rack == Rack::FULL {
        return if stand <= CENTRE {
            POCKET_LEFT
        } else {
            POCKET_RIGHT
        };
    }
    let mut best = (stand, 0);
    let mut best_key = (0u8, i32::MIN);
    for s in STAND_MIN..=STAND_MAX {
        for a in -AIM_MAX..=AIM_MAX {
            let line = Line::new(s, a, 0.0);
            let down = pinfall(rack, &line).count();
            // A line with room either side is safer than one on a knife
            // edge: count how many of the neighbouring lines do as well.
            let steady = [-1.0, 1.0]
                .iter()
                .filter(|&&d| pinfall(rack, &Line::new(s, a, d)).count() == down)
                .count() as i32;
            let key = (down, steady * 100 - (s - stand).abs() * 3 - a.abs());
            if key > best_key {
                best_key = key;
                best = (s, a);
            }
        }
    }
    best
}

/// `n` leaves for spare shooting, each what a first ball from a seeded
/// line left on a full rack: never a strike and never all ten standing.
/// Each one can be made with a clean ball, and none comes up twice.
pub fn seeded_leaves(rng: &mut Rng, n: usize) -> Vec<Rack> {
    let mut leaves: Vec<Rack> = Vec::with_capacity(n);
    let mut tries = 0;
    while leaves.len() < n {
        tries += 1;
        let stand = rng.range(12, 28);
        let aim = rng.range(-4, 4);
        let pulled = (rng.unit() - 0.5) * 8.0;
        let line = Line::new(stand, aim, pulled);
        let left = Rack::FULL.without(pinfall(Rack::FULL, &line));
        if left.is_empty() || left == Rack::FULL {
            continue;
        }
        let (s, a) = best_line(left, CENTRE);
        if pinfall(left, &Line::new(s, a, 0.0)) != left {
            continue;
        }
        // Repeats are allowed only if the seed runs short of new leaves.
        if leaves.contains(&left) && tries < 5000 {
            continue;
        }
        leaves.push(left);
    }
    leaves
}
