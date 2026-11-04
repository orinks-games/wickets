//! Every sound key the game plays, the music and beds, and the sound
//! check's catalog.

use hearken::{Bus, Ctx};
use parlor_core::{Parlor, Season};

use crate::species::Species;

pub const TITLE_MUSIC: &str = "firefly-meadow/music/title";
pub const HOME_MUSIC: &str = "firefly-meadow/music/home";
pub const KEEPSAKE_MUSIC: &str = "firefly-meadow/music/keepsake";

pub const BED_SUMMER: &str = "firefly-meadow/bed/summer";
pub const BED_SPRING: &str = "firefly-meadow/bed/spring";
pub const BED_AUTUMN: &str = "firefly-meadow/bed/autumn";
pub const LIT: &str = "firefly-meadow/bed/lit";

pub const FROGS: &str = "firefly-meadow/land/frogs";
pub const CREEK: &str = "firefly-meadow/land/creek";
pub const TREES: &str = "firefly-meadow/land/trees";
pub const HAY: &str = "firefly-meadow/land/hay";

pub const STEP_GRASS: &str = "firefly-meadow/walk/grass";
pub const EDGE: &str = "firefly-meadow/walk/edge";

pub const NEAR: &str = "firefly-meadow/catch/near";
pub const CATCH: &str = "firefly-meadow/catch/caught";
pub const ANSWER: &str = "firefly-meadow/catch/answer";
pub const EARLY: &str = "firefly-meadow/catch/early";
pub const MISSED: &str = "firefly-meadow/catch/missed";
pub const NEW_SPECIES: &str = "firefly-meadow/catch/new";
pub const FULL: &str = "firefly-meadow/catch/full";
pub const NOTHING: &str = "firefly-meadow/catch/nothing";
pub const RELEASE: &str = "firefly-meadow/catch/release";

pub const OWL: &str = "firefly-meadow/night/owl";
pub const LEAVES: &str = "firefly-meadow/night/leaves";
pub const FIREWORK: &str = "firefly-meadow/night/firework";

pub const BONFIRE: &str = "firefly-meadow/show/bonfire";
pub const CHORUS: &str = "firefly-meadow/show/chorus";
pub const IN_STEP: &str = "firefly-meadow/show/in_step";
pub const OFF_BEAT: &str = "firefly-meadow/show/off";

/// A species' blink: a soft, short tone at its note.
pub fn glow(species: Species) -> &'static str {
    match species {
        Species::Dipper => "firefly-meadow/glow/dipper",
        Species::TwinSpark => "firefly-meadow/glow/twin_spark",
        Species::SlowEmber => "firefly-meadow/glow/slow_ember",
        Species::TripleTwinkle => "firefly-meadow/glow/triple_twinkle",
        Species::OrchardLantern => "firefly-meadow/glow/orchard_lantern",
        Species::CreekGlimmer => "firefly-meadow/glow/creek_glimmer",
        Species::HayFlicker => "firefly-meadow/glow/hay_flicker",
        Species::BlueGhost => "firefly-meadow/glow/blue_ghost",
        Species::LateEmber => "firefly-meadow/glow/late_ember",
        Species::MidsummerChorus => "firefly-meadow/glow/midsummer_chorus",
    }
}

/// A species' voice in a released jar's chord: its note, held.
pub fn bloom(species: Species) -> &'static str {
    match species {
        Species::Dipper => "firefly-meadow/bloom/dipper",
        Species::TwinSpark => "firefly-meadow/bloom/twin_spark",
        Species::SlowEmber => "firefly-meadow/bloom/slow_ember",
        Species::TripleTwinkle => "firefly-meadow/bloom/triple_twinkle",
        Species::OrchardLantern => "firefly-meadow/bloom/orchard_lantern",
        Species::CreekGlimmer => "firefly-meadow/bloom/creek_glimmer",
        Species::HayFlicker => "firefly-meadow/bloom/hay_flicker",
        Species::BlueGhost => "firefly-meadow/bloom/blue_ghost",
        Species::LateEmber => "firefly-meadow/bloom/late_ember",
        Species::MidsummerChorus => "firefly-meadow/bloom/midsummer_chorus",
    }
}

/// The night's bed for a season.
pub fn bed(season: Season) -> &'static str {
    match season {
        Season::Summer => BED_SUMMER,
        Season::Spring => BED_SPRING,
        Season::Autumn => BED_AUTUMN,
        Season::Winter => BED_AUTUMN,
    }
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> hearken::Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// How the sound check plays an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Once, from the centre.
    Once,
    /// From the left, then from the right, as a firefly to one side.
    Sides,
    /// Looped for a few seconds.
    Loop,
    /// Through the music slot.
    Music,
}

/// One row of the sound check.
pub struct Entry {
    pub name: &'static str,
    pub meaning: &'static str,
    pub key: &'static str,
    pub play: Play,
}

const fn e(name: &'static str, meaning: &'static str, key: &'static str, play: Play) -> Entry {
    Entry {
        name,
        meaning,
        key,
        play,
    }
}

/// Every cue but the fireflies' own tones, with its name and meaning.
pub const CATALOG: &[Entry] = &[
    e("Footstep in the grass", "One step across the meadow.", STEP_GRASS, Play::Once),
    e("Tall grass", "The edge of the meadow. Turn back the other way.", EDGE, Play::Once),
    e("Firefly in reach", "A soft hum: a firefly is close enough to catch. Press the main action as it glows.", NEAR, Play::Once),
    e("Flash answered", "A light tick: you answered one flash of a blink with several flashes. Answer the rest.", ANSWER, Play::Once),
    e("Caught", "A glassy clink: the firefly is safe in your jar.", CATCH, Play::Once),
    e("New species", "A rising chime: a firefly you have never caught. It goes in your journal.", NEW_SPECIES, Play::Once),
    e("Not glowing yet", "A soft puff: the firefly is dark just now. Wait for its glow.", EARLY, Play::Once),
    e("Drifted on", "A falling sigh: a blink ended before you answered every flash. Try its next blink.", MISSED, Play::Once),
    e("Jar full", "A tap on the glass: the jar is full. Let them go to make room.", FULL, Play::Once),
    e("Nothing in reach", "A dull hush: no firefly is close enough.", NOTHING, Play::Once),
    e("Release", "A rising shimmer: the jar opens, the fireflies fly up and their notes ring as a chord.", RELEASE, Play::Once),
    e("The meadow lit up", "A glittering shimmer after you set a jar free, while the meadow glows and its fireflies blink more often.", LIT, Play::Loop),
    e("Frogs at the pond", "Frogs singing from the pond, always in the same place: a landmark.", FROGS, Play::Loop),
    e("The creek", "Running water along the creek, always in the same place: a landmark.", CREEK, Play::Loop),
    e("Apple trees", "Leaves rustling in the orchard's trees: a landmark.", TREES, Play::Loop),
    e("Hay in the wind", "Tall hay whispering in the breeze, across the hayfield.", HAY, Play::Loop),
    e("Bonfire", "The midsummer bonfire crackling in the middle of the meadow.", BONFIRE, Play::Loop),
    e("The chorus flash", "Midsummer night: the whole meadow flashes at once.", CHORUS, Play::Once),
    e("In step", "Midsummer night: you pressed with the flash, and a straggler joins the chorus.", IN_STEP, Play::Once),
    e("Off the beat", "Midsummer night: that press missed the flash. Press with the chorus.", OFF_BEAT, Play::Once),
];
