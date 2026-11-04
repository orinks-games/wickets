//! An evening in the meadow as a plain simulation: walking, fireflies
//! drifting and blinking, catching them in time with their blink, and
//! setting a jar free. The screen turns its notes into speech and sound.
//!
//! Time runs on a [`FixedStep`], so an evening plays the same at any
//! frame rate, and every random choice comes from a seeded [`Rng`].

use std::collections::BTreeSet;

use parlor_core::{FixedStep, Rng, Season};

use crate::meadow::{Plan, P2};
use crate::species::{chord_of, Chord, Pattern, Species};

/// Metres one step covers.
pub const STEP: f64 = 1.0;
/// How close a firefly must be to catch, metres.
pub const REACH: f64 = 2.0;
/// How early a press may come before a flash and still count, seconds.
pub const EARLY_GRACE: f64 = 0.12;
/// How late after a flash ends a press may come and still count.
pub const LATE_GRACE: f64 = 0.2;
/// The shortest time between steps while an arrow is held down.
pub const HELD_STEP: f64 = 0.22;
/// Simulation substeps a second.
pub const HZ: f64 = 60.0;
/// How fast fireflies drift, metres a second.
pub const DRIFT: f64 = 0.3;
/// Seconds before a caught firefly is replaced, where they are.
pub const RESPAWN: f64 = 5.0;
/// How long the meadow stays lit after a jar is set free.
pub const LIT_TIME: f64 = 10.0;
/// A challenge warns this many seconds before its time is up.
pub const WARN_AT: f64 = 30.0;
/// A walking direction: north is ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    North,
    South,
    West,
    East,
}

impl Dir {
    fn delta(self) -> P2 {
        match self {
            Dir::North => (0.0, STEP),
            Dir::South => (0.0, -STEP),
            Dir::West => (-STEP, 0.0),
            Dir::East => (STEP, 0.0),
        }
    }

    /// The meadow's edge this way: "north", for "the north edge".
    pub fn edge_words(self) -> &'static str {
        match self {
            Dir::North => "north",
            Dir::South => "south",
            Dir::West => "west",
            Dir::East => "east",
        }
    }
}

/// A firefly in the meadow.
#[derive(Clone, Debug, PartialEq)]
pub struct Fly {
    pub id: u32,
    pub species: Species,
    pub pos: P2,
    target: P2,
    /// Seconds into its blink cycle.
    pub phase: f64,
    /// Flashes of this burst answered so far.
    pub answered: u8,
    answered_flash: Option<u8>,
}

impl Fly {
    pub fn pattern(&self) -> Pattern {
        self.species.pattern()
    }

    /// Whether it is lit right now.
    pub fn lit(&self) -> bool {
        self.pattern().lit(self.phase).is_some()
    }
}

/// Something the screen should voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// A firefly lit up: its tone, from where it is.
    Glow {
        species: Species,
        pos: P2,
    },
    /// A firefly came within reach.
    Near {
        species: Species,
    },
    /// A blink ended with only some of its flashes answered.
    Missed {
        species: Species,
    },
    /// The first firefly of a late evening has come out.
    Appeared {
        species: Species,
    },
    Firework {
        pos: P2,
    },
    Owl {
        pos: P2,
    },
    Leaves {
        pos: P2,
    },
    /// A challenge's time is nearly up.
    Warn,
    /// A challenge's time is up: the evening is over.
    TimeUp,
}

/// What a press of the catch key did.
#[derive(Clone, Debug, PartialEq)]
pub enum Catch {
    Caught {
        species: Species,
        in_jar: usize,
        full: bool,
    },
    /// One flash of several answered; `left` to go.
    Answered {
        species: Species,
        left: u8,
    },
    /// That flash was already answered.
    Already,
    /// The firefly in reach is dark just now.
    Early {
        species: Species,
    },
    /// No firefly within reach; the nearest one, if any.
    Nothing {
        nearest: Option<P2>,
    },
    Full,
    Over,
}

/// A jar set free.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub jar: Vec<Species>,
    pub chord: Option<Chord>,
    pub full: bool,
}

/// What a step did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walk {
    Stepped,
    /// The edge of the meadow is this way.
    Edge(Dir),
    /// A held key's step came too soon after the last.
    Wait,
}

/// One evening in the meadow.
pub struct Evening {
    pub plan: Plan,
    rng: Rng,
    clock: FixedStep,
    /// Seconds since the evening began.
    pub time: f64,
    pub pos: P2,
    pub flies: Vec<Fly>,
    pub jar: Vec<Species>,
    pub caught: u32,
    pub species_tonight: BTreeSet<Species>,
    pub chords_tonight: Vec<Chord>,
    pub releases: u32,
    /// Seconds the meadow stays lit.
    pub lit: f64,
    pending: Vec<f64>,
    next_id: u32,
    next_firework: f64,
    next_critter: f64,
    last_step: f64,
    near: Option<u32>,
    appeared: bool,
    warned: bool,
    pub over: bool,
    notes: Vec<Note>,
}

impl Evening {
    /// An evening from `plan`.
    pub fn new(plan: Plan, seed: u64) -> Evening {
        let mut rng = Rng::seeded(seed);
        let pending = vec![plan.delay; plan.count];
        let next_firework = 6.0 + rng.unit() * 8.0;
        let next_critter = 15.0 + rng.unit() * 15.0;
        let mut evening = Evening {
            pos: plan.place.start(),
            plan,
            rng,
            clock: FixedStep::hz(HZ),
            time: 0.0,
            flies: Vec::new(),
            jar: Vec::new(),
            caught: 0,
            species_tonight: BTreeSet::new(),
            chords_tonight: Vec::new(),
            releases: 0,
            lit: 0.0,
            pending,
            next_id: 0,
            next_firework,
            next_critter,
            last_step: -1.0,
            near: None,
            appeared: false,
            warned: false,
            over: false,
            notes: Vec::new(),
        };
        evening.spawn_due();
        evening
    }

    fn take_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn random_spot(&mut self) -> P2 {
        let (w, d) = self.plan.size;
        (
            1.0 + self.rng.unit() * (w - 2.0),
            1.0 + self.rng.unit() * (d - 2.0),
        )
    }

    /// A spot at least `away` metres from the walker, when the meadow allows.
    fn far_spot(&mut self, away: f64) -> P2 {
        let mut spot = self.random_spot();
        for _ in 0..20 {
            if dist(spot, self.pos) >= away {
                break;
            }
            spot = self.random_spot();
        }
        spot
    }

    fn spawn_due(&mut self) {
        let now = self.time;
        let due = self.pending.iter().filter(|&&t| t <= now + 1e-9).count();
        if due == 0 {
            return;
        }
        self.pending.retain(|&t| t > now + 1e-9);
        for _ in 0..due {
            let Some(species) = self.plan.pick_species(&mut self.rng) else {
                continue;
            };
            let pos = if self.plan.practice {
                // Just out of reach, ahead if there is room, else behind.
                let d = self.plan.size.1;
                let y = if self.pos.1 + 2.5 <= d - 0.5 {
                    self.pos.1 + 2.5
                } else {
                    self.pos.1 - 2.5
                };
                (self.pos.0, y)
            } else {
                self.far_spot(4.0)
            };
            let target = if self.plan.practice {
                pos
            } else {
                self.random_spot()
            };
            let id = self.take_id();
            let phase = self.rng.unit() * species.pattern().cycle();
            self.flies.push(Fly {
                id,
                species,
                pos,
                target,
                phase,
                answered: 0,
                answered_flash: None,
            });
            if self.plan.delay > 0.0 && !self.appeared {
                self.appeared = true;
                self.notes.push(Note::Appeared { species });
            }
        }
    }

    /// Takes the notes gathered since the last call.
    pub fn drain_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    /// Runs the evening for `dt` seconds of frame time.
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
        self.lit = (self.lit - step).max(0.0);
        self.spawn_due();
        let lit_meadow = self.lit > 0.0;
        let mut notes = Vec::new();
        let practice = self.plan.practice;
        for i in 0..self.flies.len() {
            let fly = &mut self.flies[i];
            if !practice {
                let (dx, dy) = (fly.target.0 - fly.pos.0, fly.target.1 - fly.pos.1);
                let d = (dx * dx + dy * dy).sqrt();
                if d < 0.2 {
                    fly.target = (-1.0, -1.0);
                } else {
                    let m = (DRIFT * step).min(d);
                    fly.pos = (fly.pos.0 + dx / d * m, fly.pos.1 + dy / d * m);
                }
            }
            let pattern = fly.pattern();
            let before = fly.phase;
            let burst_end = pattern.burst() + LATE_GRACE;
            // A lit meadow blinks faster: the darkness runs at double speed.
            let speed = if lit_meadow && before > pattern.burst() {
                2.0
            } else {
                1.0
            };
            let mut after = before + step * speed;
            if before < burst_end && after >= burst_end {
                if fly.answered > 0 && fly.answered < pattern.flashes {
                    notes.push(Note::Missed {
                        species: fly.species,
                    });
                }
                fly.answered = 0;
                fly.answered_flash = None;
            }
            if after >= pattern.cycle() {
                after -= pattern.cycle();
                fly.answered = 0;
                fly.answered_flash = None;
            }
            fly.phase = after;
            let was = pattern.lit(before);
            let now = pattern.lit(after);
            if now.is_some() && (now != was || after < before) {
                notes.push(Note::Glow {
                    species: fly.species,
                    pos: fly.pos,
                });
            }
        }
        // Wanderers that reached their spot pick another.
        for i in 0..self.flies.len() {
            if self.flies[i].target == (-1.0, -1.0) {
                let spot = self.random_spot();
                self.flies[i].target = spot;
            }
        }
        self.notes.extend(notes);
        self.check_near();
        self.critters();
        if let Some(limit) = self.plan.clock {
            if !self.warned && self.time >= limit - WARN_AT {
                self.warned = true;
                self.notes.push(Note::Warn);
            }
            if self.time >= limit {
                self.over = true;
                self.notes.push(Note::TimeUp);
            }
        }
    }

    fn critters(&mut self) {
        let (w, d) = self.plan.size;
        if self.plan.fireworks && self.time >= self.next_firework {
            self.next_firework = self.time + 12.0 + self.rng.unit() * 16.0;
            let x = self.rng.unit() * w;
            self.notes.push(Note::Firework { pos: (x, d + 25.0) });
        }
        if self.time >= self.next_critter && !self.plan.practice {
            self.next_critter = self.time + 20.0 + self.rng.unit() * 25.0;
            let x = if self.rng.chance(0.5) { -3.0 } else { w + 3.0 };
            let y = self.rng.unit() * d;
            let note = match self.plan.season {
                Season::Autumn => Note::Leaves { pos: (x, y) },
                _ => Note::Owl { pos: (x, y) },
            };
            self.notes.push(note);
        }
    }

    /// The firefly nearest the walker, and how far it is.
    pub fn nearest(&self) -> Option<(&Fly, f64)> {
        self.flies
            .iter()
            .map(|f| (f, dist(f.pos, self.pos)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// The firefly a press of the catch key aims at: of those in reach,
    /// one glowing now if any, else the nearest.
    fn target(&self) -> Option<usize> {
        let in_reach: Vec<usize> = (0..self.flies.len())
            .filter(|&i| dist(self.flies[i].pos, self.pos) <= REACH)
            .collect();
        let glowing = in_reach.iter().copied().find(|&i| {
            let f = &self.flies[i];
            f.pattern().near(f.phase, EARLY_GRACE, LATE_GRACE).is_some()
        });
        glowing.or_else(|| {
            in_reach.into_iter().min_by(|&a, &b| {
                dist(self.flies[a].pos, self.pos).total_cmp(&dist(self.flies[b].pos, self.pos))
            })
        })
    }

    /// Whether a firefly is within reach.
    pub fn in_reach(&self) -> bool {
        self.target().is_some()
    }

    fn check_near(&mut self) {
        let now = self.target().map(|i| self.flies[i].id);
        if now.is_some() && now != self.near {
            if let Some(f) = self.flies.iter().find(|f| Some(f.id) == now) {
                self.notes.push(Note::Near {
                    species: f.species,
                });
            }
        }
        self.near = now;
    }

    /// One step `dir`. `held` marks a key's auto-repeat, which steps only
    /// every [`HELD_STEP`] seconds.
    pub fn walk(&mut self, dir: Dir, held: bool) -> Walk {
        if held && self.time - self.last_step < HELD_STEP {
            return Walk::Wait;
        }
        let (dx, dy) = dir.delta();
        let (w, d) = self.plan.size;
        let next = (self.pos.0 + dx, self.pos.1 + dy);
        if next.0 < 0.0 || next.0 > w || next.1 < 0.0 || next.1 > d {
            return Walk::Edge(dir);
        }
        self.pos = next;
        self.last_step = self.time;
        self.check_near();
        Walk::Stepped
    }

    /// A press of the catch key.
    pub fn catch(&mut self) -> Catch {
        if self.over {
            return Catch::Over;
        }
        let Some(i) = self.target() else {
            return Catch::Nothing {
                nearest: self.nearest().map(|(f, _)| f.pos),
            };
        };
        let fly = &self.flies[i];
        let pattern = fly.pattern();
        let Some(flash) = pattern.near(fly.phase, EARLY_GRACE, LATE_GRACE) else {
            return Catch::Early {
                species: fly.species,
            };
        };
        if self.jar.len() >= self.plan.jar {
            return Catch::Full;
        }
        if fly.answered_flash == Some(flash) {
            return Catch::Already;
        }
        let fly = &mut self.flies[i];
        fly.answered += 1;
        fly.answered_flash = Some(flash);
        if fly.answered < pattern.flashes {
            return Catch::Answered {
                species: fly.species,
                left: pattern.flashes - fly.answered,
            };
        }
        let fly = self.flies.remove(i);
        self.near = None;
        self.jar.push(fly.species);
        self.caught += 1;
        self.species_tonight.insert(fly.species);
        if self.plan.respawn {
            self.pending.push(self.time + RESPAWN);
        }
        Catch::Caught {
            species: fly.species,
            in_jar: self.jar.len(),
            full: self.jar.len() >= self.plan.jar,
        }
    }

    /// Sets the jar free, if anything is in it.
    pub fn release(&mut self) -> Option<Release> {
        if self.jar.is_empty() || self.over {
            return None;
        }
        let jar = std::mem::take(&mut self.jar);
        let chord = chord_of(&jar);
        if let Some(c) = chord {
            if !self.chords_tonight.contains(&c) {
                self.chords_tonight.push(c);
            }
        }
        self.releases += 1;
        self.lit = LIT_TIME;
        let full = jar.len() >= self.plan.jar;
        Some(Release { jar, chord, full })
    }

    /// Fireflies still out.
    pub fn live(&self) -> usize {
        self.flies.len()
    }

    /// Seconds left on a challenge's clock.
    pub fn time_left(&self) -> Option<f64> {
        self.plan.clock.map(|c| (c - self.time).max(0.0))
    }

    /// A challenge's score: 10 a firefly, 25 a species, 40 a named chord.
    pub fn score(&self) -> u64 {
        self.caught as u64 * 10
            + self.species_tonight.len() as u64 * 25
            + self.chords_tonight.len() as u64 * 40
    }

    /// Ends the evening early: the walker heads home.
    pub fn head_home(&mut self) {
        self.over = true;
    }
}

pub fn dist(a: P2, b: P2) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Where `to` is from `from`, in words, with north ahead:
/// "4 metres ahead and to the left", or "right here".
pub fn direction_words(from: P2, to: P2) -> String {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let d = (dx * dx + dy * dy).sqrt();
    if d < 0.75 {
        return "right here".to_string();
    }
    let ahead = if dy > d * 0.38 {
        Some("ahead")
    } else if dy < -d * 0.38 {
        Some("behind you")
    } else {
        None
    };
    let side = if dx > d * 0.38 {
        Some("to the right")
    } else if dx < -d * 0.38 {
        Some("to the left")
    } else {
        None
    };
    let way = match (ahead, side) {
        (Some(a), Some(s)) => format!("{a} and {s}"),
        (Some(a), None) => a.to_string(),
        (None, Some(s)) => s.to_string(),
        (None, None) => "close by".to_string(),
    };
    format!("{} {way}", metres(d))
}

/// "1 metre", "4 metres"; "3 feet", "13 feet" with US units.
pub fn metres(d: f64) -> String {
    parlor_core::units::count(parlor_core::units::from_metres(d).round().max(1.0) as i64)
}

/// "45 seconds", "3 minutes", "3 minutes 20 seconds".
pub fn duration_words(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    let (m, s) = (total / 60, total % 60);
    let mins = match m {
        1 => "1 minute".to_string(),
        n => format!("{n} minutes"),
    };
    let secs = match s {
        1 => "1 second".to_string(),
        n => format!("{n} seconds"),
    };
    match (m, s) {
        (0, _) => secs,
        (_, 0) => mins,
        _ => format!("{mins} {secs}"),
    }
}

/// A seed for real play, from the clock.
pub fn run_seed() -> u64 {
    Rng::from_time().next_u64()
}
