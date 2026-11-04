//! Sound keys, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

/// The eight bells, treble first. Village and Plain ring the first six.
pub const BELLS: [&str; 8] = [
    "belfry/bell/1",
    "belfry/bell/2",
    "belfry/bell/3",
    "belfry/bell/4",
    "belfry/bell/5",
    "belfry/bell/6",
    "belfry/bell/7",
    "belfry/bell/8",
];
pub const KNOCK: &str = "belfry/answer/knock";
pub const TAP: &str = "belfry/answer/tap";
pub const ROPE: &str = "belfry/row/rope";
pub const SOLVED: &str = "belfry/peal/solved";
pub const LOST: &str = "belfry/peal/lost";
pub const NEXT_PEAL: &str = "belfry/peal/next";
pub const HINT: &str = "belfry/cue/hint";
pub const NOPE: &str = "belfry/cue/nope";
pub const TOWER: &str = "belfry/ambience/tower";
pub const TITLE_MUSIC: &str = "belfry/music/title";
/// The shimmer on the called place of a Practice night peal.
pub const MOON: &str = "belfry/moon/shimmer";

/// A bell's sound, by its number from 1.
pub fn bell(n: u8) -> &'static str {
    BELLS[(n.clamp(1, 8) - 1) as usize]
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The quiet tower bed on, music off.
pub fn tower_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(TOWER));
}

/// Every cue but the bells, with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            KNOCK,
            "Knock: a firm knock on the beam. One bell of your row is right and in its place.",
        ),
        (
            TAP,
            "Tap: a soft tap on the beam. One bell of your row is in the peal, in another place.",
        ),
        (ROPE, "Rope pull: your row is rung and the answer follows."),
        (
            SOLVED,
            "Peal found: every bell rings out three times. The peal is solved.",
        ),
        (
            LOST,
            "Out of rows: a single low bell. The tower's rows are spent and the peal is read out.",
        ),
        (
            NEXT_PEAL,
            "Next peal: a short rising run. In the weekly challenge, the next peal is waiting.",
        ),
        (NOPE, "Dull thud: nothing to do, or no hint to give."),
        (
            MOON,
            "Called place: a short, quiet glassy shimmer, just after a bell. On Practice night, the place the tower captain called, whose bell is already set.",
        ),
    ]
}
