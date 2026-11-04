//! Sound keys, where the hall's sounds sit, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::Side;

pub const STEP_A: &str = "lamplighter/step-a";
pub const STEP_B: &str = "lamplighter/step-b";
pub const LAMP_OUT: &str = "lamplighter/lamp-out";
pub const LAMP_BURNING: &str = "lamplighter/lamp-burning";
pub const LIGHT: &str = "lamplighter/light";
pub const BARE_WALL: &str = "lamplighter/bare-wall";
pub const ALREADY_BURNING: &str = "lamplighter/already-burning";
pub const LOBBY_DOOR: &str = "lamplighter/lobby-door";
pub const FAR_WALL: &str = "lamplighter/far-wall";
pub const TAPER_LOW: &str = "lamplighter/taper-low";
pub const TAPER_OUT: &str = "lamplighter/taper-out";
pub const NEW_TAPER: &str = "lamplighter/new-taper";
pub const DRAUGHT: &str = "lamplighter/draught";
pub const HALL_LIT: &str = "lamplighter/hall-lit";
pub const AMBIENCE: &str = "lamplighter/hall-ambience";
pub const TITLE_MUSIC: &str = "lamplighter/music/title";

/// How far a wall sits from the middle of the hall, in metres.
pub const WALL: f32 = 1.5;
/// How long a step is, in metres.
pub const STEP: f32 = 1.2;
/// How loud a burning lamp crackles as you pass it.
pub const BURNING_VOLUME: f32 = 0.5;

/// Where an alcove sits from the player: `ahead` steps on, on `side`.
pub fn place(ahead: i32, side: Side) -> Vec3 {
    let x = match side {
        Side::Left => -WALL,
        Side::Right => WALL,
    };
    Vec3::new(x, ahead as f32 * STEP, 0.0)
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, ahead: i32, side: Side) -> Sound {
    ctx.audio.play_at(key, place(ahead, side), Bus::EFFECTS)
}

pub fn play_at_vol(ctx: &mut Ctx<Parlor>, key: &str, ahead: i32, side: Side, volume: f32) {
    let s = play_at(ctx, key, ahead, side);
    ctx.audio.set_volume(s, volume);
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The quiet of the hall, music off.
pub fn hall_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(AMBIENCE));
}

/// Every sound, with its name and meaning, for the sound check. The
/// sounds that sit on a wall play on the left.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            LAMP_OUT,
            "Lamp out: a thin whistle of draught across empty glass. It pulses from the wall where a lamp needs lighting, louder as you come near.",
        ),
        (
            LAMP_BURNING,
            "Lamp burning: the small flutter of a candle flame. You hear it from the wall as you pass a lamp that is lit.",
        ),
        (
            LIGHT,
            "Lighting: a match strike flaring into a glass ting. The lamp you reached for is lit.",
        ),
        (
            BARE_WALL,
            "Bare wall: one dull knock. There is no lamp on that wall at this step.",
        ),
        (
            ALREADY_BURNING,
            "Already burning: a soft felt knock. That lamp is lit already.",
        ),
        (
            STEP_A,
            "Footstep: a soft step on wooden boards, one step along the hall.",
        ),
        (
            LOBBY_DOOR,
            "Lobby door: a door creaking on its hinges. You are at the lobby end of the hall.",
        ),
        (
            FAR_WALL,
            "Far wall: two hollow knocks. You are at the far end of the hall.",
        ),
        (
            TAPER_LOW,
            "Taper low: a low, muted bell. The taper has 20 seconds left.",
        ),
        (
            TAPER_OUT,
            "Taper out: a short puff. The taper burned down, and the taper bonus is gone.",
        ),
        (
            NEW_TAPER,
            "Fresh taper: a quick match. Mabel lit you a new taper, so you can carry on.",
        ),
        (
            DRAUGHT,
            "Draught: a long gust through a doorway. It blew a lamp out somewhere in the hall.",
        ),
        (
            HALL_LIT,
            "Hall lit: a rising run of glass. Every lamp is burning, and the round is over.",
        ),
    ]
}
