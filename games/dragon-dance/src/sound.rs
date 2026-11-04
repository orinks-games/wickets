//! Sound keys, where things sit in the street, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::{challenges, Parlor};

use crate::festival;

pub const TITLE_MUSIC: &str = "dragon-dance/music/title";
/// The title in the run-up to New Year, with lion-dance drums and cymbals.
pub const FESTIVE_MUSIC: &str = "dragon-dance/music/festive";
/// A soft, beatless bed under the street, so the drum cues lead.
pub const STREET_BED: &str = "dragon-dance/music/street";
/// The Lantern walk's calm bed.
pub const LANTERN_BED: &str = "dragon-dance/music/lantern_walk";
pub const CLEAR: &str = "dragon-dance/music/clear";
pub const FINISH: &str = "dragon-dance/music/finish";
pub const MIDNIGHT: &str = "dragon-dance/music/midnight";

pub const CROWD: &str = "dragon-dance/ambience/crowd";
pub const NIGHT: &str = "dragon-dance/ambience/night";

pub const DRUM: &str = "dragon-dance/drum/beat";
pub const DRUM_DOWN: &str = "dragon-dance/drum/down";
pub const COUNT: &str = "dragon-dance/drum/count";
pub const LION: &str = "dragon-dance/drum/lion";
pub const ROLL: &str = "dragon-dance/drum/roll";
pub const GONG: &str = "dragon-dance/gong/strike";
pub const GREAT_GONG: &str = "dragon-dance/gong/great";
pub const REAR_PERFECT: &str = "dragon-dance/rear/perfect";
pub const REAR_GOOD: &str = "dragon-dance/rear/good";
pub const REAR_STRAY: &str = "dragon-dance/rear/stray";
pub const GONG_MISSED: &str = "dragon-dance/gong/missed";

pub const PEARL: &str = "dragon-dance/pearl/loop";
pub const PEARL_MOVE: &str = "dragon-dance/pearl/move";
pub const PEARL_HERE: &str = "dragon-dance/pearl/here";
pub const STEP: &str = "dragon-dance/move/step";
pub const EDGE: &str = "dragon-dance/move/edge";

pub const ENVELOPE: &str = "dragon-dance/envelope/loop";
pub const ENVELOPE_GET: &str = "dragon-dance/envelope/get";
pub const ENVELOPE_MISS: &str = "dragon-dance/envelope/miss";
pub const LANTERN: &str = "dragon-dance/lantern/loop";
pub const LANTERN_GET: &str = "dragon-dance/lantern/get";
pub const CRACKLE: &str = "dragon-dance/firecracker/loop";
pub const STUMBLE: &str = "dragon-dance/firecracker/stumble";
pub const BURST: &str = "dragon-dance/firecracker/burst";

pub const CHEER: &str = "dragon-dance/crowd/cheer";
pub const CLAP: &str = "dragon-dance/crowd/clap";
pub const COMBO: &str = "dragon-dance/crowd/combo";
pub const LANTERNS_UP: &str = "dragon-dance/crowd/lanterns";
pub const OFF_PEARL: &str = "dragon-dance/pearl/lost";
pub const COUNTDOWN: &str = "dragon-dance/countdown/tick";
pub const GREETING: &str = "dragon-dance/festival/greeting";
pub const KEEPSAKE: &str = "dragon-dance/festival/keepsake";
pub const NOPE: &str = "dragon-dance/cue/nope";

/// Metres between lanes.
pub const LANE_SPACING: f32 = 2.0;
/// How far ahead of the head the pearl bobs.
pub const PEARL_AHEAD: f32 = 4.0;
/// How far ahead a warned thing is first heard.
pub const FAR: f32 = 12.0;
/// How far out to the side the shops stand.
pub const SHOP_OUT: f32 = 4.0;

/// A lane's centre, across the street.
pub fn lane_x(lane: usize, lanes: usize) -> f32 {
    (lane as f32 - (lanes.max(1) - 1) as f32 / 2.0) * LANE_SPACING
}

/// Where the pearl bobs in `lane`.
pub fn pearl_pos(lane: usize, lanes: usize) -> Vec3 {
    Vec3::new(lane_x(lane, lanes), PEARL_AHEAD, 1.0)
}

/// Where a warned thing in `lane` is, `beats` from arriving.
pub fn coming_pos(lane: usize, lanes: usize, beats: f64) -> Vec3 {
    let ahead = (beats / crate::rules::WARN as f64).clamp(0.0, 1.0) as f32 * FAR;
    Vec3::new(lane_x(lane, lanes), ahead.max(0.5), 0.0)
}

/// Where something lands at the head's row, in `lane`.
pub fn at_lane(lane: usize, lanes: usize) -> Vec3 {
    Vec3::new(lane_x(lane, lanes), 0.5, 0.0)
}

/// Where a shop stands as the head passes it.
pub fn shop_pos(left: bool, lanes: usize) -> Vec3 {
    let edge = lane_x(lanes - 1, lanes) + SHOP_OUT;
    Vec3::new(if left { -edge } else { edge }, 0.0, 0.0)
}

/// Moves the listener to the head's lane.
pub fn listen_from(ctx: &mut Ctx<Parlor>, lane: usize, lanes: usize) {
    ctx.audio
        .set_listener(Vec3::new(lane_x(lane, lanes), 0.0, 0.0), 0.0);
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, pos: Vec3) -> Sound {
    ctx.audio.play_at(key, pos, Bus::EFFECTS)
}

/// The title's music: the festive tune in the run-up and on the festival
/// days, the usual one otherwise.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    let today = challenges::today(ctx);
    let key = match festival::gate(today) {
        festival::Gate::Off => TITLE_MUSIC,
        _ => FESTIVE_MUSIC,
    };
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(key), true);
    ctx.audio.set_listener(Vec3::ZERO, 0.0);
}

/// The street's beds: the crowd, and the soft music under the drums.
pub fn street_beds(ctx: &mut Ctx<Parlor>, calm: bool) {
    if calm {
        ctx.audio.play_music(Some(LANTERN_BED), true);
        ctx.audio.set_bed(Some(NIGHT));
    } else {
        ctx.audio.play_music(Some(STREET_BED), true);
        ctx.audio.set_bed(Some(CROWD));
    }
}

/// Every cue with its name and meaning, for the sound check: the name is
/// the part before the colon.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (DRUM, "Drum: a deep drum on every beat. The dragon dances on each one."),
        (DRUM_DOWN, "First beat: the drum with a cymbal, on the first beat of each bar."),
        (COUNT, "Count-in: wooden clicks before the street begins."),
        (ROLL, "Drum roll: a gong strikes in 2 beats. Get ready to rear up."),
        (GONG, "Gong: rear up as it strikes."),
        (GREAT_GONG, "Midnight gong: the great gong at the end of the New Year parade, worth double."),
        (REAR_PERFECT, "Perfect rear: a whoosh and a bright cymbal. You reared right on the gong."),
        (REAR_GOOD, "Good rear: a whoosh. You reared close to the gong."),
        (GONG_MISSED, "Missed gong: a dull wooden knock. The gong passed without a rear."),
        (REAR_STRAY, "Stray rear: a soft thud. You reared with no gong near, and the combo breaks."),
        (PEARL, "Pearl: a soft shimmer ahead of you, in the pearl's lane. Follow it."),
        (PEARL_MOVE, "Pearl moves: a bright chime from the pearl's new lane."),
        (PEARL_HERE, "On the pearl: a soft high ping as your head reaches the pearl's lane."),
        (OFF_PEARL, "Off the pearl: a falling tone. You were away from the pearl too long, and the combo breaks."),
        (STEP, "Step: the head moves one lane."),
        (EDGE, "Street edge: a wooden bump. You are in the last lane on that side."),
        (ENVELOPE, "Red envelope coming: a papery rustle with little bells, coming down its lane."),
        (ENVELOPE_GET, "Envelope caught: coins and a chime. The dragon grows one segment."),
        (ENVELOPE_MISS, "Envelope missed: a soft flutter as it drifts past."),
        (CRACKLE, "Firecrackers coming: a fizzing crackle in their lane. Keep your head out of it."),
        (BURST, "Firecrackers burst: a crackle in another lane. You are safe."),
        (STUMBLE, "Stumble: firecrackers right under you and a clatter. The tail stumbles and you lose points."),
        (LANTERN, "Lantern coming: a warm, low glow of chimes, on the Lantern walk."),
        (LANTERN_GET, "Lantern caught: a soft harp run."),
        (CHEER, "Crowd cheers: a shop's crowd cheers as you dance past."),
        (CLAP, "Polite clapping: a shop's crowd claps. Keep a combo going and they cheer."),
        (COMBO, "Combo cheer: a big cheer and a cymbal for every 16 beats on the pearl."),
        (COUNTDOWN, "Countdown: a ticking wood block, the beats before midnight."),
        (NOPE, "Dull tap: nothing to do there."),
    ]
}
