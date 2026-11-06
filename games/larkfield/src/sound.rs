//! Sound keys, and where things sit along the lane.
//!
//! The lane runs left to right from the gate to the woods. You hear from
//! the place you stand: a settler chopping in the woods sounds far off to
//! the right while you are at the cottages.

use hearken::{Bus, Ctx, Vec3};
use parlor_core::Parlor;

use crate::rules::{Place, Weather};

pub const TITLE_MUSIC: &str = "larkfield/music/title";
pub const SUPPER_MUSIC: &str = "larkfield/music/supper";
pub const PAUSE_MUSIC: &str = "larkfield/music/pause";

pub const DAY: &str = "larkfield/ambience/day";
pub const RAIN: &str = "larkfield/ambience/rain";
pub const WINTER: &str = "larkfield/ambience/winter";
/// The river, a loop at the river.
pub const RIVER: &str = "larkfield/loop/river";

pub const STEP_LANE: &str = "larkfield/move/lane";
pub const STEP_IN: &str = "larkfield/move/inside";
pub const STEP_FIELD: &str = "larkfield/move/field";
pub const EDGE: &str = "larkfield/move/edge";

pub const BITE: &str = "larkfield/fish/bite";
pub const MORNING: &str = "larkfield/day/morning";
pub const SUPPER_BELL: &str = "larkfield/day/supper_bell";
pub const BUILT: &str = "larkfield/day/built";
pub const ARRIVED: &str = "larkfield/day/arrived";
pub const KEEPSAKE: &str = "larkfield/day/keepsake";
pub const GOAL: &str = "larkfield/day/goal";

/// Every work cue the hamlet can ask for, by id.
pub const WORK: [&str; 19] = [
    "water", "dig", "sow", "weed", "harvest", "cook", "hammer", "chop", "pick", "hens", "goat",
    "feed", "cast", "splash", "reel", "forage", "talk", "fire", "fox",
];

/// The sound key of a work cue.
pub fn work(id: &str) -> String {
    format!("larkfield/work/{id}")
}

/// Metres between places on the lane.
pub const PLACE_GAP: f32 = 8.0;

/// Where a place sits on the lane.
pub fn place_pos(p: Place) -> Vec3 {
    Vec3::new((p.index() as f32 - 3.0) * PLACE_GAP, 1.0, 0.0)
}

/// You hear from where you stand.
pub fn listen_at(ctx: &mut Ctx<Parlor>, p: Place) {
    let at = place_pos(p);
    ctx.audio.set_listener(Vec3::new(at.x, 0.0, 0.0), 0.0);
}

/// The day's bed for the weather and season.
pub fn bed_for(weather: Weather, winter: bool) -> &'static str {
    if weather.wet() {
        RAIN
    } else if winter {
        WINTER
    } else {
        DAY
    }
}

/// The title's music, and no hamlet.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
    ctx.audio.set_listener(Vec3::ZERO, 0.0);
}

/// Plays a work cue at its place, softer when the setting asks.
pub fn cue(ctx: &mut Ctx<Parlor>, id: &str, at: Place, volume: f32) {
    let s = ctx.audio.play_at(&work(id), place_pos(at), Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
}
