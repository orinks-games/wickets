//! Sound keys, where they play, and the sound check's catalog.
//!
//! The tunes are music, on `Bus::MUSIC`: the game starts each part on its
//! beat. The caller and the dance cues are effects. The beds are quiet
//! ambience under everything.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::tunes::{Place, Step};

pub const TITLE_MUSIC: &str = "clover-jig/music/title";
pub const COUNT: &str = "clover-jig/count";
pub const SPOT_ON: &str = "clover-jig/judge/spot-on";
pub const CLOSE: &str = "clover-jig/judge/close";
pub const STUMBLE: &str = "clover-jig/judge/stumble";
pub const SHAMROCK: &str = "clover-jig/shamrock";
pub const LUCKY: &str = "clover-jig/lucky";
pub const CLOVER_CALL: &str = "clover-jig/clover-call";
pub const CLOVER: &str = "clover-jig/clover";
pub const CLAP: &str = "clover-jig/clap";
pub const APPLAUSE: &str = "clover-jig/applause";
pub const BED_KITCHEN: &str = "clover-jig/bed/kitchen";
pub const BED_OUTDOORS: &str = "clover-jig/bed/outdoors";
pub const BED_SQUARE: &str = "clover-jig/bed/square";
pub const BED_HARBOUR: &str = "clover-jig/bed/harbour";
pub const BED_HALL: &str = "clover-jig/bed/hall";

/// How loud the caller plays, over the music.
pub const CALL_VOLUME: f32 = 1.0;
/// How loud a tune plays, under the caller and the steps.
pub const TUNE_VOLUME: f32 = 0.7;
/// How loud a stray step plays: a step with no call near it.
pub const STRAY_VOLUME: f32 = 0.45;

/// The caller's word for a step.
pub fn call_key(step: Step) -> &'static str {
    match step {
        Step::Left => "clover-jig/call/left",
        Step::Right => "clover-jig/call/right",
        Step::Hop => "clover-jig/call/hop",
        Step::Turn => "clover-jig/call/turn",
        Step::Forward => "clover-jig/call/forward",
        Step::Back => "clover-jig/call/back",
    }
}

/// The sound of a step landing.
pub fn step_key(step: Step) -> &'static str {
    match step {
        Step::Left => "clover-jig/step/left",
        Step::Right => "clover-jig/step/right",
        Step::Hop => "clover-jig/step/hop",
        Step::Turn => "clover-jig/step/turn",
        Step::Forward => "clover-jig/step/forward",
        Step::Back => "clover-jig/step/back",
    }
}

/// Where a step lands around the listener.
pub fn step_place(step: Step) -> Vec3 {
    match step {
        Step::Left => Vec3::new(-2.0, 0.5, 0.0),
        Step::Right => Vec3::new(2.0, 0.5, 0.0),
        Step::Forward => Vec3::new(0.0, 2.0, 0.0),
        Step::Back => Vec3::new(0.0, -1.5, 0.0),
        Step::Hop | Step::Turn => Vec3::new(0.0, 0.5, 0.0),
    }
}

pub fn bed_key(place: Place) -> &'static str {
    match place {
        Place::Kitchen => BED_KITCHEN,
        Place::Outdoors => BED_OUTDOORS,
        Place::Square => BED_SQUARE,
        Place::Harbour => BED_HARBOUR,
        Place::Hall => BED_HALL,
    }
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_vol(ctx: &mut Ctx<Parlor>, key: &str, volume: f32) -> Sound {
    let s = ctx.audio.play(key, Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

/// The caller calls a step.
pub fn play_call(ctx: &mut Ctx<Parlor>, step: Step) -> Sound {
    play_vol(ctx, call_key(step), CALL_VOLUME)
}

/// A step lands, in its place.
pub fn play_step(ctx: &mut Ctx<Parlor>, step: Step, volume: f32) -> Sound {
    let s = ctx
        .audio
        .play_at(step_key(step), step_place(step), Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

/// A part of a tune, on the music bus.
pub fn play_part(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    let s = ctx.audio.play(key, Bus::MUSIC);
    ctx.audio.set_volume(s, TUNE_VOLUME);
    s
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The place's bed, the title music off: the tune plays from the game.
pub fn place_bed(ctx: &mut Ctx<Parlor>, place: Place) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(bed_key(place)));
}

/// Every cue but the calls and steps, with its name and meaning, for the
/// sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            COUNT,
            "Count: a deep bodhran beat. Four of them count you in before each tune, and three after a pause.",
        ),
        (
            SPOT_ON,
            "Spot on: a small bright chime with your step. It landed right on the beat.",
        ),
        (
            CLOSE,
            "Close: a soft dull tick with your step. It landed in time, but a little early or late.",
        ),
        (
            STUMBLE,
            "Stumble: a scuff and a thud. A step was missed, or the wrong step was danced.",
        ),
        (
            CLAP,
            "Claps: the crowd claps three times. Another 8 steps in a row, for a bonus.",
        ),
        (
            APPLAUSE,
            "Applause: the end of a tune.",
        ),
        (
            SHAMROCK,
            "Shamrock: two high chimes with a call, in Saint Patrick's week. Dance that step good or better for a luck point.",
        ),
        (
            LUCKY,
            "Luck point: a quick music box run up. You caught a shamrock.",
        ),
        (
            CLOVER_CALL,
            "Clover call: three sparkling notes with a call, once a night at the ceili. Dance that step to find the four-leaf clover.",
        ),
        (
            CLOVER,
            "Four-leaf clover: a harp sweep and a shimmer. You found it.",
        ),
    ]
}

/// Whether a catalog key is a bed or music, played on a music bus.
pub fn is_music(key: &str) -> bool {
    key == TITLE_MUSIC || key.starts_with("clover-jig/bed/")
}
