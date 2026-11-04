//! The Long Hall and its lamps: the rules, with no sound or speech.
//!
//! The hall is a row of steps from the lobby door to the far wall. Each
//! step has an alcove on the left wall and one on the right, and an alcove
//! holds a lamp or is bare. Some lamps are out. The player walks the hall
//! and reaches to a wall to light a lamp that is out. In a timed round a
//! taper burns down as they go; when it runs out Mabel hands them a fresh
//! one, so nothing is lost but the taper bonus.

use std::cell::Cell;

use parlor_core::Rng;

/// Points for each lamp lit.
pub const LAMP_POINTS: u32 = 10;
/// Seconds left on the taper when it warns that it is low.
pub const TAPER_LOW: f64 = 20.0;
/// A draught comes after this many lamps lit, while the plan has one left.
pub const DRAUGHT_EVERY: u32 = 4;

/// A wall of the hall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::Left, Side::Right];

    pub fn word(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }
}

/// What an alcove holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alcove {
    Bare,
    Out,
    Burning,
}

/// The shape of a hall: how long, how many lamps out, the taper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// Steps from the lobby door to the far wall.
    pub length: usize,
    /// Lamps out at the start.
    pub out: usize,
    /// Seconds the taper burns, or `None` for no taper.
    pub taper: Option<f64>,
    /// Draughts that blow a lamp out during the round.
    pub draughts: u32,
}

/// The evening round.
pub const ROUND: Layout = Layout {
    length: 16,
    out: 8,
    taper: Some(120.0),
    draughts: 1,
};

/// The practice hall: short, and no taper.
pub const PRACTICE: Layout = Layout {
    length: 8,
    out: 4,
    taper: None,
    draughts: 0,
};

/// Something that happened, for the screen to voice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ev {
    /// Walked to `to`, counted from 0 at the lobby door.
    Step { to: usize },
    /// Down at the first step: the lobby door is there.
    LobbyDoor,
    /// Up at the last step: the far wall.
    FarWall,
    /// A lamp lit.
    Lit { side: Side },
    /// Reached to a wall with no lamp.
    Bare { side: Side },
    /// Reached to a lamp already burning.
    AlreadyBurning { side: Side },
    /// The taper is low.
    TaperLow,
    /// The taper went out and Mabel handed over a fresh one.
    TaperOut,
    /// A draught blew a lamp out.
    Draught { step: usize, side: Side },
    /// Every lamp burning: the round is over.
    HallLit { bonus: u32 },
    /// A key pressed before the taper is lit.
    NotStarted,
}

/// One hall, from the lobby door to the far wall, as it stands.
#[derive(Clone, Debug)]
pub struct Hall {
    pub layout: Layout,
    steps: Vec<[Alcove; 2]>,
    /// The player's step, from 0 at the lobby door.
    pub pos: usize,
    /// Seconds left on the taper, in a timed round.
    pub taper: Option<f64>,
    /// Still on the first taper, so the bonus stands.
    pub first_taper: bool,
    /// Fresh tapers Mabel has handed over.
    pub tapers: u32,
    /// Lamps lit this run.
    pub lit: u32,
    pub score: u32,
    /// The taper bonus, once the hall is lit.
    pub bonus: u32,
    /// Seconds since the taper was lit.
    pub elapsed: f64,
    draughts: u32,
    started: bool,
    low_warned: bool,
    done: bool,
    rng: Rng,
}

impl Hall {
    /// Lamps lit again after a draught blew them out.
    pub fn relit(&self) -> u32 {
        self.layout.draughts.saturating_sub(self.draughts)
    }

    /// A hall of `layout`, its lamps placed by `seed`.
    pub fn new(layout: Layout, seed: u64) -> Hall {
        let mut rng = Rng::seeded(seed);
        let length = layout.length.max(2);
        let mut steps = vec![[Alcove::Bare; 2]; length];
        let mut lamps = Vec::new();
        for (i, step) in steps.iter_mut().enumerate() {
            for side in Side::BOTH {
                if rng.chance(0.55) {
                    step[side.index()] = Alcove::Burning;
                    lamps.push((i, side));
                }
            }
        }
        let want = layout.out.clamp(1, length * 2);
        while lamps.len() < want {
            let i = rng.below(length as u32) as usize;
            let side = if rng.chance(0.5) {
                Side::Left
            } else {
                Side::Right
            };
            if steps[i][side.index()] == Alcove::Bare {
                steps[i][side.index()] = Alcove::Burning;
                lamps.push((i, side));
            }
        }
        rng.shuffle(&mut lamps);
        for &(i, side) in lamps.iter().take(want) {
            steps[i][side.index()] = Alcove::Out;
        }
        Hall {
            layout,
            steps,
            pos: 0,
            taper: layout.taper,
            first_taper: true,
            tapers: 0,
            lit: 0,
            score: 0,
            bonus: 0,
            elapsed: 0.0,
            draughts: layout.draughts,
            started: false,
            low_warned: false,
            done: false,
            rng,
        }
    }

    pub fn length(&self) -> usize {
        self.steps.len()
    }

    pub fn timed(&self) -> bool {
        self.layout.taper.is_some()
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn finished(&self) -> bool {
        self.done
    }

    /// Lights the taper: the round begins.
    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn alcove(&self, step: usize, side: Side) -> Alcove {
        self.steps
            .get(step)
            .map(|s| s[side.index()])
            .unwrap_or(Alcove::Bare)
    }

    /// The two alcoves at the player's step, left then right.
    pub fn here(&self) -> [Alcove; 2] {
        self.steps[self.pos]
    }

    /// Lamps still out.
    pub fn out(&self) -> usize {
        self.steps
            .iter()
            .flatten()
            .filter(|a| **a == Alcove::Out)
            .count()
    }

    /// Every lamp that is out, as step and side.
    pub fn out_lamps(&self) -> Vec<(usize, Side)> {
        let mut v = Vec::new();
        for (i, s) in self.steps.iter().enumerate() {
            for side in Side::BOTH {
                if s[side.index()] == Alcove::Out {
                    v.push((i, side));
                }
            }
        }
        v
    }

    /// The lamp out nearest the player: steps away, ahead positive, and
    /// its side. Ties go to the one ahead, then the left.
    pub fn nearest_out(&self) -> Option<(i32, Side)> {
        self.out_lamps()
            .into_iter()
            .map(|(i, side)| (i as i32 - self.pos as i32, side))
            .min_by_key(|(d, side)| (d.abs(), *d < 0, side.index()))
    }

    /// Up walks toward the far wall, Down toward the lobby door.
    pub fn walk(&mut self, ahead: bool) -> Vec<Ev> {
        if self.done {
            return Vec::new();
        }
        if !self.started {
            return vec![Ev::NotStarted];
        }
        if ahead {
            if self.pos + 1 >= self.length() {
                return vec![Ev::FarWall];
            }
            self.pos += 1;
        } else {
            if self.pos == 0 {
                return vec![Ev::LobbyDoor];
            }
            self.pos -= 1;
        }
        vec![Ev::Step { to: self.pos }]
    }

    /// Reaching to a wall: lights a lamp that is out.
    pub fn reach(&mut self, side: Side) -> Vec<Ev> {
        if self.done {
            return Vec::new();
        }
        if !self.started {
            return vec![Ev::NotStarted];
        }
        let cell = &mut self.steps[self.pos][side.index()];
        match *cell {
            Alcove::Bare => vec![Ev::Bare { side }],
            Alcove::Burning => vec![Ev::AlreadyBurning { side }],
            Alcove::Out => {
                *cell = Alcove::Burning;
                self.lit += 1;
                self.score += LAMP_POINTS;
                let mut evs = vec![Ev::Lit { side }];
                if self.out() == 0 {
                    self.finish(&mut evs);
                } else if self.lit.is_multiple_of(DRAUGHT_EVERY) && self.draughts > 0 {
                    if let Some(ev) = self.draught() {
                        evs.push(ev);
                    }
                }
                evs
            }
        }
    }

    /// A draught blows out a burning lamp at least two steps away.
    fn draught(&mut self) -> Option<Ev> {
        let pos = self.pos;
        let burning: Vec<(usize, Side)> = self
            .steps
            .iter()
            .enumerate()
            .filter(|(i, _)| i.abs_diff(pos) >= 2)
            .flat_map(|(i, s)| {
                Side::BOTH
                    .into_iter()
                    .filter(move |side| s[side.index()] == Alcove::Burning)
                    .map(move |side| (i, side))
            })
            .collect();
        if burning.is_empty() {
            return None;
        }
        self.draughts -= 1;
        let (step, side) = *self.rng.pick(&burning);
        self.steps[step][side.index()] = Alcove::Out;
        Some(Ev::Draught { step, side })
    }

    fn finish(&mut self, evs: &mut Vec<Ev>) {
        self.done = true;
        self.bonus = match (self.first_taper, self.taper) {
            (true, Some(left)) => left.max(0.0) as u32,
            _ => 0,
        };
        self.score += self.bonus;
        evs.push(Ev::HallLit { bonus: self.bonus });
    }

    /// Time passing: the taper burns down.
    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        if !self.started || self.done {
            return Vec::new();
        }
        self.elapsed += dt;
        let Some(left) = self.taper.as_mut() else {
            return Vec::new();
        };
        *left -= dt;
        let mut evs = Vec::new();
        if *left <= 0.0 {
            *left = self.layout.taper.unwrap_or(0.0);
            self.first_taper = false;
            self.tapers += 1;
            self.low_warned = false;
            evs.push(Ev::TaperOut);
        } else if *left <= TAPER_LOW && !self.low_warned && self.first_taper {
            self.low_warned = true;
            evs.push(Ev::TaperLow);
        }
        evs
    }

    /// Whole seconds left on the taper.
    pub fn taper_secs(&self) -> Option<u32> {
        self.taper.map(|t| t.max(0.0).ceil() as u32)
    }
}

/// "1 lamp", "3 lamps".
pub fn lamps_words(n: usize) -> String {
    match n {
        1 => "1 lamp".to_string(),
        n => format!("{n} lamps"),
    }
}

/// "1 point", "40 points".
pub fn points_words(n: u32) -> String {
    match n {
        1 => "1 point".to_string(),
        n => format!("{n} points"),
    }
}

/// "45 seconds", "1 minute", "2 minutes 5 seconds".
pub fn time_words(secs: u32) -> String {
    let (m, s) = (secs / 60, secs % 60);
    let sec = match s {
        1 => "1 second".to_string(),
        s => format!("{s} seconds"),
    };
    let min = match m {
        1 => "1 minute".to_string(),
        m => format!("{m} minutes"),
    };
    match (m, s) {
        (0, _) => sec,
        (_, 0) => min,
        _ => format!("{min} {sec}"),
    }
}

/// Where a lamp is from the player, "3 steps ahead on your left", or
/// "here on your left".
pub fn where_words(ahead: i32, side: Side) -> String {
    let away = match ahead {
        0 => "here".to_string(),
        1 => "1 step ahead".to_string(),
        -1 => "1 step back".to_string(),
        d if d > 0 => format!("{d} steps ahead"),
        d => format!("{} steps back", -d),
    };
    format!("{away} on your {}", side.word())
}

/// What an alcove holds, in a few words.
pub fn alcove_words(a: Alcove) -> &'static str {
    match a {
        Alcove::Bare => "bare wall",
        Alcove::Out => "lamp out",
        Alcove::Burning => "lamp burning",
    }
}

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins every new round's seed, for tests.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new round: the pinned one, or one from the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}
