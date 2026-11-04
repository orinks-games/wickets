//! Sound keys, where things sit in the sky, and the music for each time of
//! year.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::{Kind, Range};
use crate::valentine::{self, Gate};

pub const TITLE_MUSIC: &str = "cupids-quiver/music/title";
/// The title in the Valentine's season, with roses in the sky.
pub const ROSES_MUSIC: &str = "cupids-quiver/music/roses";
/// The title on Valentine's Day itself.
pub const VALENTINE_MUSIC: &str = "cupids-quiver/music/valentine";
pub const WIN: &str = "cupids-quiver/music/win";
pub const EVENING_ENDS: &str = "cupids-quiver/music/evening_ends";

pub const ROOFTOP: &str = "cupids-quiver/ambience/rooftop";
pub const ROSE_GARDEN: &str = "cupids-quiver/ambience/roses";
pub const NIGHT: &str = "cupids-quiver/ambience/night";

pub const HUM: &str = "cupids-quiver/target/heart";
pub const ROSE: &str = "cupids-quiver/target/rose";
pub const GOLDEN: &str = "cupids-quiver/target/golden";
pub const LETTER: &str = "cupids-quiver/target/letter";

pub const CREAK: &str = "cupids-quiver/bow/creak";
pub const FULL: &str = "cupids-quiver/bow/full";
pub const LOOSE: &str = "cupids-quiver/bow/loose";
pub const SLACK: &str = "cupids-quiver/bow/slack";
pub const TICK: &str = "cupids-quiver/bow/tick";
pub const EDGE: &str = "cupids-quiver/bow/edge";
pub const EMPTY: &str = "cupids-quiver/bow/empty";

pub const POP: &str = "cupids-quiver/hit/pop";
pub const CHIME: &str = "cupids-quiver/hit/chime";
pub const SWEETHEART: &str = "cupids-quiver/hit/sweetheart";
pub const ROSE_HIT: &str = "cupids-quiver/hit/rose";
pub const GOLDEN_HIT: &str = "cupids-quiver/hit/golden";
pub const LETTER_HIT: &str = "cupids-quiver/hit/letter";
pub const MISS: &str = "cupids-quiver/hit/miss";

pub const SIGHT: &str = "cupids-quiver/cue/sight";
pub const ROUND_START: &str = "cupids-quiver/cue/round_start";
pub const ROUND_END: &str = "cupids-quiver/cue/round_end";
pub const HINT: &str = "cupids-quiver/cue/hint";
pub const TURN: &str = "cupids-quiver/cue/turn";
pub const NOPE: &str = "cupids-quiver/cue/nope";
pub const KEEPSAKE: &str = "cupids-quiver/cue/keepsake";

/// How high the sky sits above the bow, in metres.
pub const HEIGHT: f32 = 2.0;

/// A target's drifting sound.
pub fn target_loop(kind: Kind) -> &'static str {
    match kind {
        Kind::Heart(_) => HUM,
        Kind::Rose => ROSE,
        Kind::Golden => GOLDEN,
        Kind::Letter => LETTER,
    }
}

/// A target's pitch: a heart's colour sets its note.
pub fn target_pitch(kind: Kind) -> f32 {
    match kind {
        Kind::Heart(c) => c.pitch(),
        _ => 1.0,
    }
}

/// A target's volume: far ones are turned up a little, so distance
/// softens them without losing them.
pub fn range_volume(range: Range) -> f32 {
    match range {
        Range::Near => 0.6,
        Range::Middle => 0.85,
        Range::Far => 1.0,
    }
}

/// Where something at `angle` and `range` sits.
pub fn sky_pos(angle: f64, range: Range) -> Vec3 {
    let (s, c) = angle.to_radians().sin_cos();
    let d = range.metres();
    Vec3::new(s as f32 * d, c as f32 * d, HEIGHT)
}

/// The bow faces its aim, so a heart in line sounds straight ahead.
pub fn face(ctx: &mut Ctx<Parlor>, aim: f64) {
    ctx.audio.set_listener(Vec3::ZERO, aim as f32);
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

/// The title's music for the time of year.
pub fn title_music_key(gate: Gate) -> &'static str {
    match gate {
        Gate::Day => VALENTINE_MUSIC,
        Gate::Season { .. } => ROSES_MUSIC,
        Gate::Off { .. } => TITLE_MUSIC,
    }
}

/// The sky's bed for the time of year, or Valentine's night's.
pub fn bed_key(gate: Gate, night: bool) -> &'static str {
    if night {
        return NIGHT;
    }
    match gate {
        Gate::Day | Gate::Season { .. } => ROSE_GARDEN,
        Gate::Off { .. } => ROOFTOP,
    }
}

/// The title's music, from today's date, and no bed.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    let gate = valentine::gate(parlor_core::challenges::today(ctx));
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(title_music_key(gate)), true);
    ctx.audio.set_listener(Vec3::ZERO, 0.0);
}

/// The sky's bed, and no music.
pub fn sky_bed(ctx: &mut Ctx<Parlor>, night: bool) {
    let gate = valentine::gate(parlor_core::challenges::today(ctx));
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(bed_key(gate, night)));
}

/// Every cue with its name and meaning, for the sound check. The first
/// part, before the colon, is the row's label.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            ROSE,
            "Rose: a soft music box tune with a rustle of petals. A rose drifting in the Valentine's season, worth 25 more than a heart.",
        ),
        (
            GOLDEN,
            "Golden heart: a shimmer of tiny bells. Only on Valentine's night, worth 100.",
        ),
        (
            LETTER,
            "Love note: a slow flutter of paper. A folded note drifting slowly. Hit it to keep the note.",
        ),
        (
            CREAK,
            "String creak: the bow drawing, rising as the string comes back.",
        ),
        (
            FULL,
            "Full draw: a soft wooden click. Let go now to shoot.",
        ),
        (LOOSE, "Arrow loosed: a twang and a whoosh."),
        (
            SLACK,
            "Slack string: a dull flop. Let go too soon, so the arrow stays in your hand.",
        ),
        (TICK, "Aim tick: the bow turns one step."),
        (EDGE, "Edge knock: the bow is turned as far as it goes."),
        (EMPTY, "Empty quiver: a hollow tap of wood."),
        (POP, "Pop: a paper heart bursts."),
        (
            SWEETHEART,
            "Sweetheart chord: rising chimes that end in a chord. Both hearts of a pair, one after the other.",
        ),
        (ROSE_HIT, "Rose caught: a rustle and a bright harp run."),
        (
            GOLDEN_HIT,
            "Golden heart caught: a burst of bells, high and bright.",
        ),
        (
            LETTER_HIT,
            "Note caught: paper unfolding, and a soft chord. A love note to keep.",
        ),
        (
            MISS,
            "Miss: a far tap on roof tiles. The arrow met nothing.",
        ),
        (
            SIGHT,
            "Sights ping: a small glassy ping. Let go now and the arrow meets a target.",
        ),
        (ROUND_START, "Rooftop bell: a round begins."),
        (ROUND_END, "Quiver spent: three falling notes. The round is over."),
        (NOPE, "Dull tap: nothing to do."),
    ]
}
