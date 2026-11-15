//! Sound keys and where they play.
//!
//! You are the listener, skating forward. Ahead is up the pond, left and
//! right are the banks. A lantern is heard ahead in its lane, off to the
//! side by how far it is from you, and comes closer until it passes.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::{PatchKind, StallKind, BANK};

pub const TITLE_MUSIC: &str = "snowfall-skate/music/title";
pub const PAUSE_MUSIC: &str = "snowfall-skate/music/pause";
/// The pond at night, looped: a hush, the village far off.
pub const POND: &str = "snowfall-skate/bed/pond";
/// The snow coming down, looped. Its volume follows how deep it lies.
pub const SNOWFALL: &str = "snowfall-skate/bed/snowfall";
/// The frozen river at the fair, looped: a crowd along the banks.
pub const RIVER: &str = "snowfall-skate/bed/river";
/// Your blades on clear ice, looped.
pub const BLADES: &str = "snowfall-skate/skate/blades";
/// Your blades in settled snow, looped: softer.
pub const BLADES_SOFT: &str = "snowfall-skate/skate/blades_soft";
/// Your partner's blades, looped, a little ahead.
pub const PARTNER: &str = "snowfall-skate/skate/partner";
/// A lean taken up: a carve of the blade.
pub const CARVE: &str = "snowfall-skate/skate/carve";
/// The push-off.
pub const START: &str = "snowfall-skate/skate/push";
/// A short press: one stride.
pub const STRIDE: &str = "snowfall-skate/skate/stride";
/// A lantern burning ahead, looped.
pub const FLAME: &str = "snowfall-skate/lantern/flame";
/// A lantern taken in the wrong order.
pub const WRONG: &str = "snowfall-skate/lantern/wrong";
/// A lantern gone by.
pub const PASSED: &str = "snowfall-skate/lantern/passed";
/// A drift ahead, looped.
pub const DRIFT: &str = "snowfall-skate/patch/drift";
/// Thin ice ahead, looped.
pub const THIN: &str = "snowfall-skate/patch/thin";
/// Into a drift.
pub const DRIFTED: &str = "snowfall-skate/patch/drifted";
/// The ice cracks under you.
pub const CRACK: &str = "snowfall-skate/patch/crack";
/// A spin winding, looped while the key is held.
pub const SPIN_WIND: &str = "snowfall-skate/spin/wind";
pub const SPIN_READY: &str = "snowfall-skate/spin/ready";
pub const SPIN_LAND: &str = "snowfall-skate/spin/land";
pub const WOBBLE: &str = "snowfall-skate/spin/wobble";
/// The snow has covered the ice.
pub const SNOWED_OVER: &str = "snowfall-skate/end/snowed";
/// A course skated to its end.
pub const DONE: &str = "snowfall-skate/end/done";
pub const WON: &str = "snowfall-skate/end/won";
pub const LOST: &str = "snowfall-skate/end/lost";

/// The note a lantern rings as it lights.
pub fn light_key(note: usize) -> String {
    format!("snowfall-skate/lantern/light{}", note.min(7))
}

/// The note a lantern rings as it is taken.
pub fn take_key(note: usize) -> String {
    format!("snowfall-skate/lantern/take{}", note.min(7))
}

/// A drift or thin ice ahead, looped.
pub fn patch_key(kind: PatchKind) -> &'static str {
    match kind {
        PatchKind::Drift => DRIFT,
        PatchKind::Thin => THIN,
    }
}

/// A stall at the fair, looped as you pass.
pub fn stall_key(kind: StallKind) -> &'static str {
    match kind {
        StallKind::Chestnuts => "snowfall-skate/stall/chestnuts",
        StallKind::Cider => "snowfall-skate/stall/cider",
        StallKind::Band => "snowfall-skate/stall/band",
        StallKind::Gingerbread => "snowfall-skate/stall/gingerbread",
    }
}

/// How far ahead a lantern is when it lights.
pub const AHEAD_FROM: f32 = 9.0;
/// How far to the side the whole width of the pond sounds.
pub const SIDE: f32 = 6.0;
/// Your own blades, just below.
pub const UNDER: Vec3 = Vec3::new(0.0, 0.2, -0.8);

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, at: Vec3) -> Sound {
    ctx.audio.play_at(key, at, Bus::EFFECTS)
}

/// Where something at `x` on the ice sounds from you at `you`, `ahead`
/// metres up the pond.
pub fn place(x: f64, you: f64, ahead: f32) -> Vec3 {
    let side = ((x - you) / BANK) as f32 * SIDE;
    Vec3::new(side, ahead.max(-2.0), 0.0)
}

/// How far ahead something passing at `at` is at `now`, heard from `from`.
pub fn ahead(at: i64, from: i64, now: i64) -> f32 {
    let lead = (at - from).max(1) as f32;
    ((at - now) as f32 / lead).clamp(-0.25, 1.0) * AHEAD_FROM
}

/// A sound that names a lane, for the sound check and hints.
pub fn lane_place(lane: i32) -> Vec3 {
    place(crate::rules::lane_x(lane), 0.0, AHEAD_FROM * 0.6)
}

/// Where a stall on `side` sounds, as you pass it at `at`.
pub fn stall_place(side: i32, at: i64, now: i64) -> Vec3 {
    let ahead = ((at - now) as f32 / 3000.0).clamp(-0.7, 1.0) * 10.0;
    Vec3::new(side as f32 * 7.0, ahead, 0.0)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// How loud the snowfall is with `clear` of the ice still clear.
pub fn snowfall_volume(clear: f64) -> f32 {
    (0.2 + 0.8 * (1.0 - clear / 100.0)).clamp(0.0, 1.0) as f32
}

/// How loud your crisp blades are against the soft ones.
pub fn crisp_volume(clear: f64) -> f32 {
    (clear / 100.0).clamp(0.15, 1.0) as f32
}
