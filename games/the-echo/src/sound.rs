//! Sound keys, the two held tones of a pipe, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

pub const PIPE: &str = "the-echo/pipe";
pub const NOTE: &str = "the-echo/note";
pub const SLIDE: &str = "the-echo/slide";
pub const IN_TUNE: &str = "the-echo/in-tune";
pub const NOT_YET: &str = "the-echo/not-yet";
pub const HINT: &str = "the-echo/hint";
pub const RANK_TUNED: &str = "the-echo/rank-tuned";
pub const AMBIENCE: &str = "the-echo/cellar-ambience";
pub const TITLE_MUSIC: &str = "the-echo/music/title";

/// How loud each of the two held tones plays.
pub const TONE_VOLUME: f32 = 0.45;
/// How long a pitch change glides, in milliseconds.
pub const GLIDE_MS: u32 = 60;

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

/// Plays `key` once at `pitch`.
pub fn play_pitched(ctx: &mut Ctx<Parlor>, key: &str, pitch: f32) -> Sound {
    let s = ctx.audio.play(key, Bus::EFFECTS);
    ctx.audio.set_pitch(s, pitch);
    s
}

/// Starts a held tone at `pitch`.
pub fn hold(ctx: &mut Ctx<Parlor>, pitch: f32) -> Sound {
    let s = ctx.audio.start_loop(PIPE, None, Bus::EFFECTS);
    ctx.audio.set_pitch(s, pitch);
    ctx.audio.set_volume(s, TONE_VOLUME);
    s
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The quiet of the cellar, music off.
pub fn cellar_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(AMBIENCE));
}

/// Every sound, with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            PIPE,
            "Pipe: a steady, soft organ tone. While you tune, two of these play together, the pipe and its true note. Out of tune, they wobble; in tune, the wobble goes still.",
        ),
        (
            NOTE,
            "Echo note: one short organ note. The Echo plays these back as a tune once a rank is tuned.",
        ),
        (
            SLIDE,
            "Slide: a tiny brass tick. The pipe moved up or down a step.",
        ),
        (
            IN_TUNE,
            "In tune: a clear bell. You set the pipe, and it holds its note.",
        ),
        (
            HINT,
            "Hint: a soft felt knock. H told you which way the pipe is out.",
        ),
        (
            NOT_YET,
            "Not yet: a dull thud. The pipe still wobbles, so it did not set.",
        ),
        (
            RANK_TUNED,
            "Rank tuned: rising tubular bells over an organ chord. Every pipe in the rank is in tune.",
        ),
    ]
}
