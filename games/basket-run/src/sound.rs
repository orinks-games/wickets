//! Sound keys and where things sit in the meadow.

use hearken::{Ctx, Vec3};
use parlor_core::Parlor;

use crate::rules::Kind;

pub const TITLE_MUSIC: &str = "basket-run/music/title";
pub const ROUND_CLEAR: &str = "basket-run/music/round_clear";
pub const WIN: &str = "basket-run/music/win";
pub const LOSE: &str = "basket-run/music/lose";
pub const MEADOW: &str = "basket-run/ambience/meadow";
/// The meadow's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("basket-run");
/// Pan pipes: a festival pattern begins to fall.
pub const PATTERN_CALL: &str = "basket-run/festival/call";
/// A cascade of bells: a festival pattern landed cleanly.
pub const PATTERN_DONE: &str = "basket-run/festival/done";
/// A low wooden flop: a festival pattern was broken.
pub const PATTERN_BROKEN: &str = "basket-run/festival/broken";
/// A cuckoo call: the daily's twist is announced.
pub const TWIST: &str = "basket-run/challenge/twist";

pub const CATCH_EGG: &str = "basket-run/catch/egg";
pub const CATCH_GOLDEN: &str = "basket-run/catch/golden";
pub const CRACK_EGG: &str = "basket-run/crack/egg";
pub const CRACK_GOLDEN: &str = "basket-run/crack/golden";
pub const SPLAT: &str = "basket-run/mud/splat";
pub const MUD_THUD: &str = "basket-run/mud/thud";
pub const STING: &str = "basket-run/bee/sting";
pub const BEE_AWAY: &str = "basket-run/bee/away";
pub const FULL: &str = "basket-run/basket/full";
pub const BANK: &str = "basket-run/basket/bank";
pub const EMPTY: &str = "basket-run/basket/empty";
pub const SPILL: &str = "basket-run/basket/spill";
pub const BEACON: &str = "basket-run/cart/beacon";
pub const STEP: &str = "basket-run/move/step";
pub const EDGE: &str = "basket-run/move/edge";
pub const AT_CART: &str = "basket-run/move/cart";
pub const READY: &str = "basket-run/round/ready";
pub const WARNING: &str = "basket-run/round/warning";
pub const TICK: &str = "basket-run/round/tick";
pub const BELL: &str = "basket-run/round/bell";
pub const STREAK: &str = "basket-run/streak";
pub const LIFE_LOST: &str = "basket-run/life/lost";

/// Metres between lanes.
pub const LANE_SPACING: f32 = 2.0;
/// How far ahead of the player things fall, so a lane's pan stays gentle.
pub const AHEAD: f32 = 3.0;
/// How high a drop starts.
pub const HEIGHT: f32 = 4.0;

/// The falling loop of each kind.
pub fn falling(kind: Kind) -> &'static str {
    match kind {
        Kind::Egg => "basket-run/fall/egg",
        Kind::Golden => "basket-run/fall/golden",
        Kind::Mud => "basket-run/fall/mud",
        Kind::Bee => "basket-run/fall/bee",
    }
}

/// A lane's centre, across the meadow.
pub fn lane_x(lane: usize, lanes: usize) -> f32 {
    (lane as f32 - (lanes.max(1) - 1) as f32 / 2.0) * LANE_SPACING
}

/// Where something in `lane` sits at `progress` through its fall.
pub fn drop_pos(lane: usize, lanes: usize, progress: f64) -> Vec3 {
    Vec3::new(
        lane_x(lane, lanes),
        AHEAD,
        HEIGHT * (1.0 - progress.clamp(0.0, 1.0) as f32),
    )
}

/// Where something lands in `lane`.
pub fn ground(lane: usize, lanes: usize) -> Vec3 {
    drop_pos(lane, lanes, 1.0)
}

/// The falling loop's pitch: high as it starts, low as it lands.
pub fn fall_pitch(progress: f64) -> f32 {
    1.35 - 0.6 * progress.clamp(0.0, 1.0) as f32
}

/// The falling loop's volume: soft high up, full as it lands.
pub fn fall_volume(progress: f64) -> f32 {
    0.45 + 0.55 * progress.clamp(0.0, 1.0) as f32
}

/// Moves the listener to the player's lane.
pub fn listen_from(ctx: &mut Ctx<Parlor>, lane: usize, lanes: usize) {
    ctx.audio
        .set_listener(Vec3::new(lane_x(lane, lanes), 0.0, 0.0), 0.0);
}

/// The title's music, and no meadow.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
    ctx.audio.set_listener(Vec3::ZERO, 0.0);
}

/// The meadow's ambience, and no music.
pub fn meadow(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(MEADOW));
}
