//! Sound keys, and where the sky sits around the listener: the dome is
//! under you, the horizon ahead, the far left lane to your left.

use hearken::{Bus, Ctx, Vec3};
use parlor_core::Parlor;

use crate::rules::lanes::HORIZON;
use crate::rules::Kind;

pub const TITLE_MUSIC: &str = "dustfall/music/title";
pub const PAUSE_MUSIC: &str = "dustfall/music/pause";
/// Played once as a night ends held.
pub const HELD: &str = "dustfall/music/held";
pub const RELIEF_MUSIC: &str = "dustfall/music/relief";
pub const FALLEN_MUSIC: &str = "dustfall/music/fallen";

/// A wave is called, and a wave is down with more to come.
pub const WAVE: &str = "dustfall/wave/coming";
pub const WAVE_CLEAR: &str = "dustfall/wave/clear";

/// A raider comes over the horizon, or is dropped.
pub const ARRIVE: &str = "dustfall/raider/arrive";
/// A raider leaves over the edge.
pub const LEAVE: &str = "dustfall/raider/leave";
/// A weaving raider slips a lane.
pub const WEAVE: &str = "dustfall/raider/weave";

pub const GUN_RAIL: &str = "dustfall/gun/rail";
pub const GUN_FIRE: &str = "dustfall/gun/fire";
pub const GUN_HIT: &str = "dustfall/gun/hit";
pub const GUN_DOWN: &str = "dustfall/gun/down";
pub const MISSILE: &str = "dustfall/gun/missile";
/// The lock tone: a raider is in the gun's lane. Pulses faster as it
/// nears.
pub const LOCK: &str = "dustfall/gun/lock";

pub const CHARGE_WIND: &str = "dustfall/charge/wind";
pub const CHARGE_PASS: &str = "dustfall/charge/pass";
pub const CHARGE_HIT: &str = "dustfall/charge/hit";

pub const BREACH_SHIELD: &str = "dustfall/breach/shield";
pub const BREACH_DOME: &str = "dustfall/breach/dome";
pub const BREACH_BUILDING: &str = "dustfall/breach/building";

pub const MINE: &str = "dustfall/lane/mine";
pub const FLAK: &str = "dustfall/lane/flak";

/// The dust storm's wind, looped at the lane it covers.
pub const STORM_WIND: &str = "dustfall/storm/wind";

pub const BUY: &str = "dustfall/colony/buy";
pub const CANT: &str = "dustfall/colony/cant";
pub const READY: &str = "dustfall/colony/ready";

/// The scan, played where the raider is.
pub const SCAN: &str = "dustfall/ui/scan";

/// A raider's engine, looped at its place in the sky, by kind.
pub fn engine(kind: Kind) -> &'static str {
    match kind {
        Kind::Drifter => "dustfall/engine/drifter",
        Kind::Skitter => "dustfall/engine/skitter",
        Kind::Hulk => "dustfall/engine/hulk",
        Kind::Sapper => "dustfall/engine/sapper",
        Kind::Mule => "dustfall/engine/mule",
        Kind::Mothership => "dustfall/engine/mothership",
        Kind::Carrier => "dustfall/engine/carrier",
    }
}

/// Metres between lanes, and from the dome to the horizon.
const LANE_GAP: f32 = 1.5;
const DEPTH: f32 = 7.0;

/// Where a raider sits: the centre lane dead ahead, the horizon seven
/// metres out, the dome just past your feet.
pub fn sky_pos(lane: u8, height: u32) -> Vec3 {
    let out = 0.6 + DEPTH * height as f32 / HORIZON as f32;
    Vec3::new(lane_x(lane), out, 0.0)
}

/// Where the gun sits in `lane`: right under the dome's edge.
pub fn lane_pos(lane: u8) -> Vec3 {
    Vec3::new(lane_x(lane), 0.5, 0.0)
}

fn lane_x(lane: u8) -> f32 {
    (lane as f32 - crate::rules::CENTRE as f32) * LANE_GAP
}

/// Plays `key` where a raider is.
pub fn at(ctx: &mut Ctx<Parlor>, key: &'static str, lane: u8, height: u32) {
    ctx.audio.play_at(key, sky_pos(lane, height), Bus::EFFECTS);
}

/// Plays `key` under lane `lane`.
pub fn under(ctx: &mut Ctx<Parlor>, key: &'static str, lane: u8) {
    ctx.audio.play_at(key, lane_pos(lane), Bus::EFFECTS);
}

/// The listener stands in the dome, facing the sky.
pub fn listen_from_the_dome(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_listener(Vec3::ZERO, 0.0);
}

/// The title's music.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
    listen_from_the_dome(ctx);
}

/// Quiet for the watch: no title music.
pub fn on_watch(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    listen_from_the_dome(ctx);
}
