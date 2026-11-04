//! A meteor watch, as plain rules: shooting stars at seeded times, each
//! streaking across the sky for a second or so, and wishes made while one
//! passes. Driven by `update(dt)` on a fixed step.

use parlor_core::{FixedStep, Rng};

use crate::almanac::Strength;

/// Seconds after a meteor fades during which a wish still counts.
pub const GRACE: f64 = 0.4;

/// One shooting star crossing the sky.
#[derive(Clone, Debug, PartialEq)]
pub struct Meteor {
    /// Seconds since it appeared.
    pub t: f64,
    /// Seconds it takes to cross.
    pub dur: f64,
    /// Where it starts and ends, left to right from -1 to 1.
    pub from: f32,
    pub to: f32,
    /// A fireball: brighter, slower and louder.
    pub bright: bool,
    pub wished: bool,
}

impl Meteor {
    /// Where it is now, left to right.
    pub fn pan(&self) -> f32 {
        let p = (self.t / self.dur).clamp(0.0, 1.0) as f32;
        self.from + (self.to - self.from) * p
    }

    /// Whether it is still crossing, or just faded.
    pub fn catchable(&self) -> bool {
        self.t <= self.dur + GRACE
    }
}

/// What the watch asks the screen to do.
#[derive(Clone, Debug, PartialEq)]
pub enum WatchCue {
    /// A shooting star appears at `pan`.
    Start { pan: f32, bright: bool },
    /// It has moved to `pan`.
    Move { pan: f32 },
    /// It has gone, wished on or not.
    Gone { wished: bool },
}

/// What a wish did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wish {
    /// A wish made, on a fireball or not.
    Made { bright: bool },
    /// This shooting star has had its wish.
    Already,
    /// No shooting star is passing.
    NoMeteor,
}

/// A night's meteor watch.
pub struct MeteorWatch {
    pub strength: Strength,
    pub current: Option<Meteor>,
    pub wishes: u32,
    pub seen: u32,
    pub fireballs: u32,
    /// Seconds to the next shooting star.
    pub until_next: f64,
    clock: FixedStep,
    rng: Rng,
}

impl MeteorWatch {
    pub fn new(strength: Strength, seed: u64) -> MeteorWatch {
        let mut watch = MeteorWatch {
            strength,
            current: None,
            wishes: 0,
            seen: 0,
            fireballs: 0,
            until_next: 0.0,
            clock: FixedStep::hz(60.0),
            rng: Rng::seeded(seed),
        };
        // The first comes soon, so the player hears what to listen for.
        watch.until_next = 2.5 + watch.rng.unit() * 1.5;
        watch
    }

    /// Seconds between shooting stars: closer on the peak night.
    fn gap(&mut self) -> f64 {
        match self.strength {
            Strength::Peak => 3.0 + self.rng.unit() * 6.0,
            Strength::RunUp { .. } => 12.0 + self.rng.unit() * 10.0,
        }
    }

    /// Space: a wish on the shooting star passing now.
    pub fn wish(&mut self) -> Wish {
        match self.current.as_mut() {
            Some(m) if m.catchable() && !m.wished => {
                m.wished = true;
                self.wishes += 1;
                Wish::Made { bright: m.bright }
            }
            Some(m) if m.catchable() => Wish::Already,
            _ => Wish::NoMeteor,
        }
    }

    /// Whether a shooting star is crossing now.
    pub fn passing(&self) -> bool {
        self.current.as_ref().is_some_and(|m| m.t <= m.dur)
    }

    pub fn update(&mut self, dt: f64) -> Vec<WatchCue> {
        let mut out = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |step| self.tick(step, &mut out));
        self.clock = clock;
        out
    }

    fn tick(&mut self, step: f64, out: &mut Vec<WatchCue>) {
        if let Some(m) = self.current.as_mut() {
            m.t += step;
            if m.t <= m.dur {
                // Every tenth of a second is enough to sweep the sound.
                let before = ((m.t - step) * 10.0).floor();
                if (m.t * 10.0).floor() > before {
                    out.push(WatchCue::Move { pan: m.pan() });
                }
            }
            if !m.catchable() {
                out.push(WatchCue::Gone { wished: m.wished });
                self.current = None;
            }
            return;
        }
        self.until_next -= step;
        if self.until_next > 0.0 {
            return;
        }
        let bright = self.rng.chance(0.12);
        let from = (self.rng.unit() * 1.6 - 0.8) as f32;
        let span = (0.6 + self.rng.unit() * 0.8) as f32;
        let to = if from > 0.0 { from - span } else { from + span };
        let dur = if bright {
            1.8 + self.rng.unit() * 0.6
        } else {
            0.9 + self.rng.unit() * 0.6
        };
        self.seen += 1;
        if bright {
            self.fireballs += 1;
        }
        out.push(WatchCue::Start { pan: from, bright });
        self.current = Some(Meteor {
            t: 0.0,
            dur,
            from,
            to: to.clamp(-1.0, 1.0),
            bright,
            wished: false,
        });
        self.until_next = self.gap();
    }
}
