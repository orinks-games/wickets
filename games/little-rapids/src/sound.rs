//! Sound keys, and where things sit around the boat.

use hearken::{Ctx, Vec3};
use parlor_core::Parlor;

use crate::rules::{Kind, HALF};

pub const TITLE_MUSIC: &str = "little-rapids/music/title";
pub const MOUTH_MUSIC: &str = "little-rapids/music/mouth";
pub const SANK_MUSIC: &str = "little-rapids/music/sank";

/// The water under the boat.
pub const WATER: &str = "little-rapids/ambience/water";
/// Lapping at a bank, one loop on each side.
pub const BANK: &str = "little-rapids/loop/bank";
/// The current's hiss.
pub const CURRENT: &str = "little-rapids/loop/current";

pub const LAUNCH: &str = "little-rapids/move/launch";
/// A paddle stroke, two takes so a long pull does not repeat itself.
pub const PADDLE: [&str; 2] = ["little-rapids/move/paddle_1", "little-rapids/move/paddle_2"];
/// Seconds between strokes while you keep paddling.
pub const STROKE_SECS: f64 = 0.5;
pub const BUMP_BANK: &str = "little-rapids/bump/bank";
pub const BUMP_STONE: &str = "little-rapids/bump/stone";
pub const BUMP_MILL: &str = "little-rapids/bump/mill";
pub const BUMP_DUCK: &str = "little-rapids/bump/duck";
pub const BUMP_FALLS: &str = "little-rapids/bump/falls";
pub const CLEAN: &str = "little-rapids/catch/clean";
pub const SCRAP: &str = "little-rapids/catch/scrap";
pub const TRINKET: &str = "little-rapids/catch/trinket";
pub const GATE: &str = "little-rapids/through/gate";
pub const FALLS: &str = "little-rapids/through/falls";
pub const MULTIPLIER: &str = "little-rapids/score/multiplier";
pub const NEW_BEST: &str = "little-rapids/score/best";
pub const RIBBON: &str = "little-rapids/score/ribbon";
pub const MOUTH: &str = "little-rapids/move/mouth";
pub const SANK: &str = "little-rapids/move/sank";
pub const MUSIC_BOX: &str = "little-rapids/shelf/music_box";
pub const FOLD: &str = "little-rapids/shelf/fold";
pub const CURRENT_IN: &str = "little-rapids/move/current_in";
pub const CURRENT_OUT: &str = "little-rapids/move/current_out";

/// The loop a thing makes as it floats toward you.
pub fn floating(kind: Kind) -> &'static str {
    match kind {
        Kind::Petal => "little-rapids/float/petal",
        Kind::Leaf => "little-rapids/float/leaf",
        Kind::Scrap => "little-rapids/float/scrap",
        Kind::Trinket => "little-rapids/float/trinket",
        Kind::Lantern => "little-rapids/float/lantern",
        Kind::Stone => "little-rapids/float/stone",
        Kind::Gate => "little-rapids/float/gate",
        Kind::Falls => "little-rapids/float/falls",
        Kind::Mill => "little-rapids/float/mill",
        Kind::Duck => "little-rapids/float/duck",
    }
}

/// Metres ahead per beat still to go, so a thing's distance is heard.
pub const AHEAD_PER_BEAT: f32 = 1.4;

/// Where a thing sits relative to the stream, `beats_left` ahead.
pub fn thing_pos(x: f64, beats_left: f64) -> Vec3 {
    Vec3::new(
        x as f32,
        (beats_left.max(0.0) as f32) * AHEAD_PER_BEAT + 0.5,
        0.0,
    )
}

/// The floating loop's pitch: low far off, rising as it comes.
pub fn float_pitch(beats_left: f64, lead: f64) -> f32 {
    let near = 1.0 - (beats_left / lead.max(1.0)).clamp(0.0, 1.0);
    0.85 + 0.3 * near as f32
}

/// The floating loop's volume: soft far off, full as it arrives.
pub fn float_volume(beats_left: f64, lead: f64) -> f32 {
    let near = 1.0 - (beats_left / lead.max(1.0)).clamp(0.0, 1.0);
    0.35 + 0.65 * near as f32
}

/// A bank's lapping volume for the boat at `x`.
pub fn bank_volume(x: f64, side: f64) -> f32 {
    let d = (side * HALF - x).abs();
    (1.0 - d / 4.0).clamp(0.08, 1.0) as f32
}

/// The listener rides in the boat, facing downstream.
pub fn listen_from(ctx: &mut Ctx<Parlor>, x: f64) {
    ctx.audio.set_listener(Vec3::new(x as f32, 0.0, 0.0), 0.0);
}

/// The title's music, and no water.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
    ctx.audio.set_listener(Vec3::ZERO, 0.0);
}

/// The stream's water, and no title music.
pub fn on_the_water(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(WATER));
}
