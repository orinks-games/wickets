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
//!
//! A game with a band style can play a song written for the run instead
//! ([`Score::start_song`]): Hearken's band plays it note by note with the
//! same six layers following the same mood, so nothing else about the
//! game changes. A tutorial, and one fresh run in ten, play the written
//! stems (see [`crate::band::SongPlan`]).
//!
//! The score also keeps the run's song (see [`crate::songs`]): the first
//! [`Score::start`] of a run begins the take and [`Score::stop`] ends it,
//! so every game with a score keeps its runs without a line of its own. A
//! start while the score plays (a new round) starts the stems over inside
//! the same song.

use crate::band::{Band, Feel, SongPlan};
use crate::Parlor;
use hearken::band::Role;
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
    /// The band, when the run plays a song written for it.
    band: Option<Playing>,
    /// The stems' keys, so a band's levels read as the stems' would.
    keys: Vec<&'static str>,
    mood: Mood,
    /// The game whose song the score is keeping, from its stems' keys.
    game: Option<&'static str>,
}

/// A band at work, behind a `Debug` of its own.
struct Playing(Box<Band>);

impl std::fmt::Debug for Playing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Band({})", self.0.song().describe())
    }
}

/// What the band hears from a mood: intensity, and the hurry and danger
/// flags.
pub fn feel(mood: Mood) -> Feel {
    Feel {
        intensity: mood.intensity,
        hurry: mood.flags & (1 << HURRY) != 0,
        danger: mood.flags & (1 << DANGER) != 0,
    }
}

/// The stem that stands for each of the band's layers, in the order of
/// [`score_stems!`]: base, pulse, melody, lift, hurry, danger.
const LAYERS: [Role; 6] = [
    Role::Base,
    Role::Pulse,
    Role::Melody,
    Role::Lift,
    Role::Hurry,
    Role::Danger,
];

impl Score {
    /// A score that plays nothing, until [`Score::start`].
    pub fn new() -> Score {
        Score::default()
    }

    /// Starts every stem together on the music bus, at `mood`'s volumes,
    /// stopping whatever music was playing. Already playing, it starts over.
    pub fn start(&mut self, ctx: &mut Ctx<Parlor>, stems: &[Stem], mood: Mood) {
        self.start_song(ctx, stems, mood, None);
    }

    /// Starts the run's music: the song `plan` writes, played by the band,
    /// or the written `stems` when there is no plan, a tutorial is open, or
    /// the plan draws the written score. Already playing, it starts over.
    pub fn start_song(&mut self, ctx: &mut Ctx<Parlor>, stems: &[Stem], mood: Mood, plan: Option<SongPlan>) {
        let song = plan
            .filter(|p| !p.plays_written_score() && !crate::songs::in_lesson(ctx))
            .and_then(|p| Some((p, p.song()?)));
        let Some((plan, song)) = song else {
            self.start_stems(ctx, stems, mood);
            return;
        };
        self.silence(ctx);
        ctx.audio.play_music(None, false);
        if let Some(game) = stems.first().and_then(|s| s.key.split('/').next()) {
            self.game = Some(game);
            crate::songs::begin(ctx);
        }
        self.mood = mood;
        self.keys = stems.iter().map(|s| s.key).collect();
        let mut band = Band::new(song, crate::band::bank().clone());
        band.set_speed(plan.speed);
        band.start_at(&mut ctx.audio, feel(mood), plan.section);
        self.band = Some(Playing(Box::new(band)));
    }

    /// Sets the band's own speed on top of its song's tempo, from the next
    /// bar (see [`SongPlan::speed`]); nothing for the written stems.
    pub fn set_speed(&mut self, speed: f32) {
        if let Some(band) = &mut self.band {
            band.0.set_speed(speed);
        }
    }

    /// The song the band is playing, in a line, for logs and tests: `None`
    /// while the written stems play.
    pub fn describe(&self) -> Option<String> {
        self.band.as_ref().map(|b| b.0.song().describe())
    }

    fn start_stems(&mut self, ctx: &mut Ctx<Parlor>, stems: &[Stem], mood: Mood) {
        self.silence(ctx);
        ctx.audio.play_music(None, false);
        if let Some(game) = stems.first().and_then(|s| s.key.split('/').next()) {
            self.game = Some(game);
            crate::songs::begin(ctx);
        }
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
        if let Some(band) = &mut self.band {
            band.0.follow(&mut ctx.audio, feel(mood), dt);
        }
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
        if let Some(band) = &mut self.band {
            band.0.jump(&mut ctx.audio, feel(mood));
        }
        for (stem, sound, volume) in &mut self.stems {
            *volume = stem.volume_for(mood);
            ctx.audio.set_volume(*sound, *volume);
        }
    }

    /// Fades every stem out, and keeps the run's song.
    pub fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        self.silence(ctx);
        if let Some(game) = self.game.take() {
            crate::songs::end(ctx, game);
        }
    }

    fn silence(&mut self, ctx: &mut Ctx<Parlor>) {
        for (_, sound, _) in self.stems.drain(..) {
            ctx.audio.stop(sound, STOP_MS);
        }
        if let Some(mut band) = self.band.take() {
            band.0.stop(&mut ctx.audio, STOP_MS);
        }
        self.keys.clear();
    }

    pub fn is_playing(&self) -> bool {
        !self.stems.is_empty() || self.band.is_some()
    }

    /// The mood the score last followed.
    pub fn mood(&self) -> Mood {
        self.mood
    }

    /// Each stem's key and its volume now, for tests. While the band
    /// plays, each stem's key stands for its layer, with the layer's level.
    pub fn levels(&self) -> Vec<(&'static str, f32)> {
        if let Some(band) = &self.band {
            let levels = band.0.levels();
            return self
                .keys
                .iter()
                .zip(LAYERS)
                .map(|(k, role)| {
                    let i = Role::ALL.iter().position(|r| *r == role).unwrap_or(0);
                    (*k, levels[i])
                })
                .collect();
        }
        self.stems.iter().map(|(s, _, v)| (s.key, *v)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::Base;
    use hearken::{Headless, State, TakeState};

    struct Run;
    impl State<Parlor> for Run {
        fn name(&self) -> &'static str {
            "tightrope_run"
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    #[test]
    fn the_score_keeps_the_runs_song_from_its_first_start_to_its_stop() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Run);
        h.frame(0.0);
        let stems = score_stems!("tightrope");
        let mut score = Score::new();
        score.start(&mut h.app.ctx, &stems, Mood::new(0.2));
        assert_eq!(h.app.ctx.take_state(), TakeState::Recording);
        h.frame(10.0);
        // A new round starts the stems over inside the same song.
        score.start(&mut h.app.ctx, &stems, Mood::new(0.5));
        assert_eq!(h.app.ctx.take_state(), TakeState::Recording);
        h.frame(10.0);
        score.stop(&mut h.app.ctx);
        assert_eq!(h.app.ctx.take_state(), TakeState::Idle);
        let kept = crate::songs::fresh(&h.app.ctx).expect("the song is kept");
        assert_eq!(kept.game, "tightrope");
        let entries = crate::songs::entries(&mut h.app.ctx, "tightrope");
        assert_eq!(entries.len(), 1);
        assert!((entries[0].length_s - 20.0).abs() < 0.1, "{}", entries[0].length_s);
    }

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
