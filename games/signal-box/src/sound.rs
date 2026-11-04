//! Sound keys, where the calls sit, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::Call;

pub const POINTS_LEFT: &str = "signal-box/call/points-left";
pub const POINTS_RIGHT: &str = "signal-box/call/points-right";
pub const SIGNAL_UP: &str = "signal-box/call/signal-up";
pub const SIGNAL_DOWN: &str = "signal-box/call/signal-down";
pub const RING_BACK: &str = "signal-box/call/ring-back";
pub const TICK: &str = "signal-box/tick";
pub const TRAIN_PASS: &str = "signal-box/train-pass";
pub const BRAKE: &str = "signal-box/brake-screech";
pub const STREAK: &str = "signal-box/streak-chime";
pub const AMBIENCE: &str = "signal-box/box-ambience";
pub const TITLE_MUSIC: &str = "signal-box/music/title";
/// The thirteenth train on Friday the 13th, in place of the rush.
pub const GHOST_TRAIN: &str = "signal-box/event/ghost-train";
/// The steam special, in place of the rush.
pub const SPECIAL: &str = "signal-box/event/moonlight-special";

/// How loud the tick track plays, under speech and the calls.
pub const TICK_VOLUME: f32 = 0.35;

/// A call's earcon.
pub fn call_key(call: Call) -> &'static str {
    match call {
        Call::PointsLeft => POINTS_LEFT,
        Call::PointsRight => POINTS_RIGHT,
        Call::SignalUp => SIGNAL_UP,
        Call::SignalDown => SIGNAL_DOWN,
        Call::RingBack => RING_BACK,
    }
}

/// Where a call's earcon sits: the points off to their side, the signal
/// and the bell ahead.
pub fn call_place(call: Call) -> Vec3 {
    match call {
        Call::PointsLeft => Vec3::new(-3.0, 1.0, 0.0),
        Call::PointsRight => Vec3::new(3.0, 1.0, 0.0),
        _ => Vec3::new(0.0, 2.0, 0.0),
    }
}

/// Plays a call's earcon in its place.
pub fn play_call(ctx: &mut Ctx<Parlor>, call: Call) -> Sound {
    ctx.audio
        .play_at(call_key(call), call_place(call), Bus::EFFECTS)
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_vol(ctx: &mut Ctx<Parlor>, key: &str, volume: f32) -> Sound {
    let s = ctx.audio.play(key, Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The night outside the box, music off.
pub fn box_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(AMBIENCE));
}

/// Every sound but the calls, with its name and meaning, for the sound
/// check. The calls come first there, from [`Call::ALL`].
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            TICK,
            "Tick: a dry, quiet tap that keeps the tempo. It quickens as the window shrinks.",
        ),
        (
            TRAIN_PASS,
            "Train passing: a rush from left to right. Your answer was right and the train is through.",
        ),
        (
            SPECIAL,
            "Steam special: steam chuffs and a two-note whistle as the train rushes by. At the steam gala, every fifth train is the special, worth 3 points.",
        ),
        (
            GHOST_TRAIN,
            "Ghost train: the rush with a long, falling, ghostly whistle. On Friday the 13th, the thirteenth train of the night shift went through.",
        ),
        (
            STREAK,
            "Streak chime: a bright rising run. Another 10 trains in a row, for a bonus of 5 points.",
        ),
        (
            BRAKE,
            "Brake screech: a long squeal and a crash. A wrong or late answer derailed the train, and the shift is over.",
        ),
    ]
}
