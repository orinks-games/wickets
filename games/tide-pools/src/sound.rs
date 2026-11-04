//! Sound keys, what each one means, and where sounds sit.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::{Lean, Species, POOL_SPACING};

pub const HIDE: &str = "tide-pools/cue/hide";
pub const EMERGE: &str = "tide-pools/cue/emerge";
pub const GATHER: &str = "tide-pools/cue/gather";
pub const EMPTY: &str = "tide-pools/cue/empty";
pub const REACH: &str = "tide-pools/cue/reach";
pub const STEP_A: &str = "tide-pools/cue/step_a";
pub const STEP_B: &str = "tide-pools/cue/step_b";
pub const LEAN_IN: &str = "tide-pools/cue/lean_in";
pub const LEAN_BACK: &str = "tide-pools/cue/lean_back";
pub const EDGE: &str = "tide-pools/cue/edge";
pub const TIDE_TURN: &str = "tide-pools/cue/tide_turn";
pub const TIDE_WARN: &str = "tide-pools/cue/tide_warn";
pub const RARE: &str = "tide-pools/cue/rare";
pub const NEW: &str = "tide-pools/cue/new";
pub const TANK: &str = "tide-pools/cue/tank";
pub const GULL: &str = "tide-pools/atmos/gull";
pub const GUST: &str = "tide-pools/atmos/gust";
pub const FOGHORN: &str = "tide-pools/atmos/foghorn";
pub const WAVE_CRASH: &str = "tide-pools/atmos/wave_crash";
/// A curlew on the far sand, on a king tide.
pub const CURLEW: &str = "tide-pools/atmos/curlew";
/// A night heron, on the moonlit pools.
pub const HERON: &str = "tide-pools/atmos/heron";
pub const WAVES: &str = "tide-pools/ambience/waves";
pub const SHORE: &str = "tide-pools/ambience/shore";
pub const RAIN: &str = "tide-pools/ambience/rain";
pub const POOL: &str = "tide-pools/ambience/pool";
pub const AQUARIUM: &str = "tide-pools/ambience/aquarium";
/// The shore's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("tide-pools");
pub const TITLE_MUSIC: &str = "tide-pools/music/title";
pub const WIN: &str = "tide-pools/music/win";
pub const HOME: &str = "tide-pools/music/home";

/// A creature's own sound.
pub fn creature(s: Species) -> &'static str {
    match s {
        Species::Crab => "tide-pools/creatures/crab",
        Species::Periwinkle => "tide-pools/creatures/periwinkle",
        Species::Starfish => "tide-pools/creatures/starfish",
        Species::Anemone => "tide-pools/creatures/anemone",
        Species::Goby => "tide-pools/creatures/goby",
        Species::Hermit => "tide-pools/creatures/hermit",
        Species::Shrimp => "tide-pools/creatures/shrimp",
        Species::Mussel => "tide-pools/creatures/mussel",
        Species::Cowrie => "tide-pools/creatures/cowrie",
        Species::Seahorse => "tide-pools/creatures/seahorse",
        Species::Octopus => "tide-pools/creatures/octopus",
        Species::Jelly => "tide-pools/creatures/jelly",
        Species::SandDollar => "tide-pools/creatures/sanddollar",
        Species::SeaSlug => "tide-pools/creatures/seaslug",
    }
}

/// Every sound with its name and what it means, for the sound guide.
pub fn catalog() -> Vec<(&'static str, String, String)> {
    let mut out: Vec<(&'static str, String, String)> = Species::ALL
        .into_iter()
        .map(|s| {
            let shy = if s.shy().is_some() {
                " It is shy: it hides when you lean in, until you keep still."
            } else {
                ""
            };
            let rare = match s.when() {
                Some(w) => format!(" A rare find: it only turns up {w}."),
                None => String::new(),
            };
            let mut name = s.name().to_string();
            if let Some(first) = name.get_mut(0..1) {
                first.make_ascii_uppercase();
            }
            (
                creature(s),
                name,
                format!(
                    "The {}: {}.{shy}{rare} Worth {} points.",
                    s.name(),
                    s.sound_words(),
                    s.points()
                ),
            )
        })
        .collect();
    let cues: &[(&'static str, &str, &str)] = &[
        (HIDE, "Hiding", "A quick falling swish and a plop: something shy darted out of sight as you leaned in. Keep still until it creeps back out."),
        (EMERGE, "Creeping out", "Two soft rising notes: something shy has come back out. Listen for its sound, then reach in."),
        (GATHER, "Gathered", "A scoop of water and a rising chime: you gathered a creature for your aquarium."),
        (EMPTY, "Empty hands", "Water dripping from your fingers: nothing came up. Reach in just after you hear something."),
        (REACH, "Out of reach", "A hand patting wet rock: you are too far back to reach the water. Lean in first."),
        (STEP_A, "Footstep", "Walking over wet rock to the next pool."),
        (LEAN_IN, "Leaning in", "Kneeling closer to the water."),
        (LEAN_BACK, "Stepping back", "Rising and stepping back from the water."),
        (EDGE, "End of the rocks", "A soft thud into sand: there are no more pools that way."),
        (TIDE_TURN, "The tide turns", "One distant bell buoy: half the trip is gone and the sea starts back."),
        (TIDE_WARN, "The tide is near", "The bell buoy twice: the tide will soon cover the pools."),
        (WAVE_CRASH, "The tide comes in", "A big wave washing over the rocks: the trip is over."),
        (RARE, "A rare find", "A shimmer: what you gathered is rare."),
        (NEW, "New for the aquarium", "A little rising arpeggio: a kind you have never gathered before."),
        (WAVES, "Waves", "The sea, ahead of you beyond the pools. It grows louder and nearer as the tide comes in."),
        (POOL, "The pool in front of you", "The water of the pool you stand at, nearer as you lean in."),
    ];
    for &(key, name, meaning) in cues {
        out.push((key, name.to_string(), meaning.to_string()));
    }
    out
}

/// Where a pool `offset` pools along sits, from where you stand.
pub fn pool_at(offset: i32, lean: Lean) -> Vec3 {
    Vec3::new(offset as f32 * POOL_SPACING, lean.distance(), 0.0)
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, pos: Vec3) -> Sound {
    ctx.audio.play_at(key, pos, Bus::EFFECTS)
}

/// The title theme on, beds off.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}
