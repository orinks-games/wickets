//! A dynamic score: music that follows play.
//!
//! A game's score is a set of stems, loops of the same length rendered from
//! one genny piece (see `sound-specs/<id>.json`, `score/*`). They all start
//! together, so they stay on the beat, and each fades in and out with how
//! intense play is: a quiet base at the calm end, more voices as the run
//! heats up. A stem can also follow a flag of its own instead, such as a
//! danger layer that plays only while something threatens the player.
//!
//! The game owns a [`Score`], starts it as play begins, calls
//! [`Score::follow`] each frame with what play is doing and the frame's
//! `dt`, and stops it when
//! play ends. Pause and resume need nothing: Hearken's pause menu pauses
//! every sound and resumes them together.

use crate::Parlor;
use hearken::{Bus, Ctx, Sound};

/// How a stem's volume follows play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rise {
    /// Always at full volume.
    Always,
    /// Silent at or below intensity `from`, full at `full` and above, in
    /// between on a straight line. Intensity runs from 0 to 1.
    Between(f32, f32),
    /// Full while the game's flag `n` is set, silent otherwise.
    Flag(usize),
}

/// One stem of a score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stem {
    /// The loop's sound key.
    pub key: &'static str,
    /// Its volume at full.
    pub volume: f32,
    pub rise: Rise,
}

impl Stem {
    pub const fn always(key: &'static str, volume: f32) -> Stem {
        Stem {
            key,
            volume,
            rise: Rise::Always,
        }
    }

    pub const fn between(key: &'static str, volume: f32, from: f32, full: f32) -> Stem {
        Stem {
            key,
            volume,
            rise: Rise::Between(from, full),
        }
    }

    pub const fn flag(key: &'static str, volume: f32, flag: usize) -> Stem {
        Stem {
            key,
            volume,
            rise: Rise::Flag(flag),
        }
    }

    /// This stem's volume for `mood`.
    pub fn volume_for(&self, mood: Mood) -> f32 {
        let level = match self.rise {
            Rise::Always => 1.0,
            Rise::Between(from, full) => {
                if mood.intensity <= from {
                    0.0
                } else if mood.intensity >= full || full <= from {
                    1.0
                } else {
                    (mood.intensity - from) / (full - from)
                }
            }
            Rise::Flag(n) => {
                if mood.flags & (1 << n) != 0 {
                    1.0
                } else {
                    0.0
                }
            }
        };
        self.volume * level
    }
}

/// What play is doing, as the score hears it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mood {
    /// 0 calm to 1 at its most intense.
    pub intensity: f32,
    /// Bit `n` set raises [`Rise::Flag`]`(n)`.
    pub flags: u32,
}

impl Mood {
    pub fn new(intensity: f32) -> Mood {
        Mood {
            intensity: intensity.clamp(0.0, 1.0),
            flags: 0,
        }
    }

    /// The same mood with flag `n` set when `on`.
    pub fn with_flag(mut self, n: usize, on: bool) -> Mood {
        if on {
            self.flags |= 1 << n;
        }
        self
    }
}

/// The flag that raises a score's `hurry` stem: time pressure, such as a
/// round's last seconds.
pub const HURRY: usize = 0;

/// The flag that raises a score's `danger` stem: a threat to the player.
pub const DANGER: usize = 1;

/// The six stems every game's score is rendered as, under
/// `<game>/score/`: `base` always plays, `pulse`, `melody` and `lift` come
/// in one after another as intensity climbs, and `hurry` and `danger`
/// follow [`HURRY`] and [`DANGER`].
#[macro_export]
macro_rules! score_stems {
    ($game:literal) => {
        [
            $crate::Stem::always(concat!($game, "/score/base"), 1.0),
            $crate::Stem::between(concat!($game, "/score/pulse"), 1.0, 0.1, 0.35),
            $crate::Stem::between(concat!($game, "/score/melody"), 1.0, 0.35, 0.6),
            $crate::Stem::between(concat!($game, "/score/lift"), 1.0, 0.65, 0.95),
            $crate::Stem::flag(concat!($game, "/score/hurry"), 1.0, $crate::score::HURRY),
            $crate::Stem::flag(concat!($game, "/score/danger"), 1.0, $crate::score::DANGER),
        ]
    };
}

/// How long a stem takes to swell from silence to full, or fade back, in
/// seconds: slow enough to sound like the band answering, not a switch.
pub const RAMP_S: f32 = 1.5;

/// How long the score takes to fade out when play ends.
pub const STOP_MS: u32 = 600;

/// A score that is playing.
#[derive(Debug, Default)]
pub struct Score {
    stems: Vec<(Stem, Sound, f32)>,
    mood: Mood,
}

impl Score {
    /// A score that plays nothing, until [`Score::start`].
    pub fn new() -> Score {
        Score::default()
    }

    /// Starts every stem together on the music bus, at `mood`'s volumes,
    /// stopping whatever music was playing. Already playing, it starts over.
    pub fn start(&mut self, ctx: &mut Ctx<Parlor>, stems: &[Stem], mood: Mood) {
        self.stop(ctx);
        ctx.audio.play_music(None, false);
        self.mood = mood;
        for stem in stems {
            let sound = ctx.audio.start_loop(stem.key, None, Bus::MUSIC);
            let volume = stem.volume_for(mood);
            ctx.audio.set_volume(sound, volume);
            self.stems.push((*stem, sound, volume));
        }
    }

    /// Follows play: each stem eases toward its volume for `mood`, by
    /// at most its full volume per [`RAMP_S`] seconds, over `dt` seconds.
    pub fn follow(&mut self, ctx: &mut Ctx<Parlor>, mood: Mood, dt: f64) {
        self.mood = mood;
        for (stem, sound, volume) in &mut self.stems {
            let want = stem.volume_for(mood);
            if (want - *volume).abs() < 1e-4 {
                continue;
            }
            let step = stem.volume * (dt as f32 / RAMP_S).max(0.0);
            *volume = if want > *volume {
                (*volume + step).min(want)
            } else {
                (*volume - step).max(want)
            };
            ctx.audio.set_volume(*sound, *volume);
        }
    }

    /// Jumps every stem straight to its volume for `mood`, for a change
    /// play makes all at once (a new round, the run's end).
    pub fn jump(&mut self, ctx: &mut Ctx<Parlor>, mood: Mood) {
        self.mood = mood;
        for (stem, sound, volume) in &mut self.stems {
            *volume = stem.volume_for(mood);
            ctx.audio.set_volume(*sound, *volume);
        }
    }

    /// Fades every stem out.
    pub fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        for (_, sound, _) in self.stems.drain(..) {
            ctx.audio.stop(sound, STOP_MS);
        }
    }

    pub fn is_playing(&self) -> bool {
        !self.stems.is_empty()
    }

    /// The mood the score last followed.
    pub fn mood(&self) -> Mood {
        self.mood
    }

    /// Each stem's key and its volume now, for tests.
    pub fn levels(&self) -> Vec<(&'static str, f32)> {
        self.stems.iter().map(|(s, _, v)| (s.key, *v)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stem_rises_between_its_bounds_and_follows_its_flag() {
        let s = Stem::between("x", 0.5, 0.2, 0.6);
        assert_eq!(s.volume_for(Mood::new(0.1)), 0.0);
        assert!((s.volume_for(Mood::new(0.4)) - 0.25).abs() < 1e-6);
        assert_eq!(s.volume_for(Mood::new(0.9)), 0.5);
        let f = Stem::flag("y", 0.4, 2);
        assert_eq!(f.volume_for(Mood::new(1.0)), 0.0);
        assert_eq!(f.volume_for(Mood::new(0.0).with_flag(2, true)), 0.4);
        assert_eq!(Stem::always("z", 0.3).volume_for(Mood::default()), 0.3);
    }
}
