//! Sound keys, and where the grid sits around the listener: the dome is
//! under you, the far row ahead, the far left column to your left.

use hearken::{Bus, Ctx, Vec3};
use parlor_core::Parlor;

use crate::rules::{Kind, Sector};

pub const TITLE_MUSIC: &str = "dustfall/music/title";
pub const PAUSE_MUSIC: &str = "dustfall/music/pause";
/// Played once as a night ends held.
pub const HELD: &str = "dustfall/music/held";
pub const RELIEF_MUSIC: &str = "dustfall/music/relief";
pub const FALLEN_MUSIC: &str = "dustfall/music/fallen";

/// A raider enters the far row, or is dropped.
pub const ARRIVE: &str = "dustfall/raider/arrive";
/// A raider leaves over the edge.
pub const LEAVE: &str = "dustfall/raider/leave";

pub const GUN_RAIL: &str = "dustfall/gun/rail";
pub const GUN_FIRE: &str = "dustfall/gun/fire";
pub const GUN_HIT: &str = "dustfall/gun/hit";
pub const GUN_DOWN: &str = "dustfall/gun/down";
pub const MISSILE: &str = "dustfall/gun/missile";

pub const CHARGE_WIND: &str = "dustfall/charge/wind";
pub const CHARGE_PASS: &str = "dustfall/charge/pass";
pub const CHARGE_HIT: &str = "dustfall/charge/hit";

pub const BREACH_SHIELD: &str = "dustfall/breach/shield";
pub const BREACH_DOME: &str = "dustfall/breach/dome";
pub const BREACH_BUILDING: &str = "dustfall/breach/building";

pub const WALL_CRACK: &str = "dustfall/grid/wall_crack";
pub const WALL_DOWN: &str = "dustfall/grid/wall_down";
pub const MINE: &str = "dustfall/grid/mine";
pub const FLAK: &str = "dustfall/grid/flak";

/// The dust storm's wind, looped at the column it covers.
pub const STORM_WIND: &str = "dustfall/storm/wind";

pub const BUY: &str = "dustfall/colony/buy";
pub const CANT: &str = "dustfall/colony/cant";
pub const READY: &str = "dustfall/colony/ready";

/// The scan and the placing cursor, played where the sector is.
pub const SCAN: &str = "dustfall/ui/scan";
/// Your turn begins.
pub const TURN: &str = "dustfall/ui/turn";
/// Your turn ends.
pub const TURN_OVER: &str = "dustfall/ui/turn_over";

/// The sound a raider makes as it steps, by kind.
pub fn raider(kind: Kind) -> &'static str {
    match kind {
        Kind::Drifter => "dustfall/raider/drifter",
        Kind::Skitter => "dustfall/raider/skitter",
        Kind::Hulk => "dustfall/raider/hulk",
        Kind::Sapper => "dustfall/raider/sapper",
        Kind::Mule => "dustfall/raider/mule",
        Kind::Mothership => "dustfall/raider/mothership",
        Kind::Carrier => "dustfall/raider/carrier",
    }
}

/// Metres between columns, and per row ahead.
const COLUMN_GAP: f32 = 1.5;
const ROW_GAP: f32 = 1.2;

/// Where a sector sits: the centre column dead ahead, row 1 just past the
/// dome, row 6 at the horizon.
pub fn sector_pos(at: Sector) -> Vec3 {
    Vec3::new(column_x(at.col), at.row as f32 * ROW_GAP, 0.0)
}

/// Where the gun sits in column `col`: right under the dome's edge.
pub fn column_pos(col: u8) -> Vec3 {
    Vec3::new(column_x(col), 0.5, 0.0)
}

fn column_x(col: u8) -> f32 {
    (col as f32 - crate::rules::grid::CENTRE as f32) * COLUMN_GAP
}

/// Plays `key` at a sector.
pub fn at(ctx: &mut Ctx<Parlor>, key: &'static str, sector: Sector) {
    ctx.audio.play_at(key, sector_pos(sector), Bus::EFFECTS);
}

/// Plays `key` under column `col`.
pub fn under(ctx: &mut Ctx<Parlor>, key: &'static str, col: u8) {
    ctx.audio.play_at(key, column_pos(col), Bus::EFFECTS);
}

/// The listener stands in the dome, facing the grid.
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
