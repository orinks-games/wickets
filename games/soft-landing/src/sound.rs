//! Sound keys, how loud the loops sit, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

pub const ENGINE: &str = "soft-landing/engine";
pub const DESCENT_TONE: &str = "soft-landing/descent-tone";
pub const PAD_BEACON: &str = "soft-landing/pad-beacon";
pub const FUEL_LOW: &str = "soft-landing/fuel-low";
pub const TOUCHDOWN: &str = "soft-landing/touchdown";
pub const CRASH: &str = "soft-landing/crash";
pub const RADIO_CLICK: &str = "soft-landing/radio-click";
pub const CRICKETS: &str = "soft-landing/event/crickets";
pub const QUINDAR: &str = "soft-landing/event/quindar";
pub const AMBIENCE: &str = "soft-landing/space-ambience";
pub const TITLE_MUSIC: &str = "soft-landing/music/title";

/// The engine loop, under speech.
pub const ENGINE_VOLUME: f32 = 0.6;
/// The descent tone, quieter still: it runs the whole flight.
pub const TONE_VOLUME: f32 = 0.35;
/// Crickets behind the radio click, below it.
pub const CRICKETS_VOLUME: f32 = 0.3;
/// The beacon, a soft ping.
pub const BEACON_VOLUME: f32 = 0.5;

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_vol(ctx: &mut Ctx<Parlor>, key: &str, volume: f32) -> Sound {
    let s = ctx.audio.play(key, Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

/// Starts a loop on the effects bus at `volume`.
pub fn start_loop(ctx: &mut Ctx<Parlor>, key: &str, volume: f32) -> Sound {
    let s = ctx.audio.start_loop(key, None, Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The quiet hum of space, music off.
pub fn space_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(AMBIENCE));
}

/// Every sound but the descent tone, with its name and meaning, for the
/// sound check. The tone comes first there, slow and fast.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            ENGINE,
            "Engine: a low roar while you hold the engine key. It slows your fall and burns fuel.",
        ),
        (
            PAD_BEACON,
            "Pad beacon: a soft ping from the pad below you. It quickens as you get closer.",
        ),
        (
            RADIO_CLICK,
            "Radio click: mission control is about to give a call-out.",
        ),
        (
            QUINDAR,
            "Quindar tone: a short high beep on the Tranquility pad. It stands for the radio click: a call-out follows.",
        ),
        (
            FUEL_LOW,
            "Fuel alarm: two soft beeps. Fuel is down to 25 percent, then again at 10 percent.",
        ),
        (
            TOUCHDOWN,
            "Touchdown: a thud and a creak of the legs. You landed safely.",
        ),
        (
            CRASH,
            "Crash: a heavy crunch. You touched down faster than the safe speed.",
        ),
    ]
}
