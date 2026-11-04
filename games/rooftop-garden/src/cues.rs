//! Every sound key the game plays, and the sound check's list of them.

use crate::rules::Species;

pub const MUSIC_TITLE: &str = "rooftop-garden/music/title";
pub const MUSIC_SHED: &str = "rooftop-garden/music/shed";
pub const MUSIC_HARVEST: &str = "rooftop-garden/music/harvest";
pub const MUSIC_GOAL: &str = "rooftop-garden/music/goal";
pub const MUSIC_SEASON_END: &str = "rooftop-garden/music/season_end";
pub const MUSIC_CHORUS: &str = "rooftop-garden/music/chorus";

pub const SONG_BASE: &str = "rooftop-garden/song/base";

/// Each plant's part in the garden's own tune, all in G major so they play
/// together. The made-up tunes' phrases are in `song.rs`.
pub fn song(species: Species) -> &'static str {
    match species {
        Species::Tomato => "rooftop-garden/song/tomato",
        Species::Squash => "rooftop-garden/song/squash",
        Species::Bean => "rooftop-garden/song/bean",
        Species::Sunflower => "rooftop-garden/song/sunflower",
        Species::Lavender => "rooftop-garden/song/lavender",
        Species::Mint => "rooftop-garden/song/mint",
    }
}

pub const HUM: &str = "rooftop-garden/plant/hum";
pub const SPROUT: &str = "rooftop-garden/plant/sprout";
pub const GROW: &str = "rooftop-garden/plant/grow";
pub const BLOOM: &str = "rooftop-garden/plant/bloom";
pub const RIPE: &str = "rooftop-garden/plant/ripe";
pub const THIRSTY: &str = "rooftop-garden/plant/thirsty";
pub const WILT: &str = "rooftop-garden/plant/wilt";
pub const LEGGY: &str = "rooftop-garden/plant/leggy";
pub const DRY: &str = "rooftop-garden/soil/dry";
/// Waterlogged soil, gurgling from its planter.
pub const SOGGY: &str = "rooftop-garden/soil/soggy";
pub const BUZZ: &str = "rooftop-garden/pest/buzz";
pub const PEST_ARRIVE: &str = "rooftop-garden/pest/arrive";

pub const WATER: &str = "rooftop-garden/tend/water";
pub const DRIP: &str = "rooftop-garden/tend/drip";
/// Watering soil that was already moist: a squelch.
pub const SQUELCH: &str = "rooftop-garden/tend/squelch";
pub const PRUNE: &str = "rooftop-garden/tend/prune";
pub const HARVEST: &str = "rooftop-garden/tend/harvest";
pub const PICK: &str = "rooftop-garden/tend/pick";
pub const PLANT: &str = "rooftop-garden/tend/plant";
pub const COMPOST: &str = "rooftop-garden/tend/compost";
pub const REVIVE: &str = "rooftop-garden/tend/revive";
pub const RUSTLE: &str = "rooftop-garden/tend/rustle";
pub const NO: &str = "rooftop-garden/tend/no";

pub const STEP: &str = "rooftop-garden/move/step";
pub const RAILING: &str = "rooftop-garden/move/railing";
pub const EMPTY: &str = "rooftop-garden/move/empty";

pub const CITY: &str = "rooftop-garden/ambience/city";
pub const WIND: &str = "rooftop-garden/ambience/wind";
pub const SPARROW: &str = "rooftop-garden/ambience/sparrow";
pub const PIGEON: &str = "rooftop-garden/ambience/pigeon";
pub const CHIMES: &str = "rooftop-garden/ambience/chimes";
pub const HORN: &str = "rooftop-garden/ambience/horn";

pub const RAIN: &str = "rooftop-garden/weather/rain";
pub const SHOWER: &str = "rooftop-garden/weather/shower";
pub const CLEAR: &str = "rooftop-garden/weather/clear";
pub const DAWN: &str = "rooftop-garden/day/dawn";
pub const DUSK: &str = "rooftop-garden/day/dusk";
/// On the first Sunday of the month, the evening in your own garden:
/// church bells ring for evensong.
pub const SUNDAY_BELLS: &str = "rooftop-garden/day/sunday-bells";
/// A pumpkin's part in the song: the squash's line as a low bassoon.
pub const PUMPKIN: &str = "rooftop-garden/song/pumpkin";

pub const BENCH: &str = "rooftop-garden/ui/bench";
pub const STAND: &str = "rooftop-garden/ui/stand";
pub const UNLOCK: &str = "rooftop-garden/ui/unlock";
pub const EXPAND: &str = "rooftop-garden/ui/expand";

/// How the sound check plays an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Once, from the centre.
    Once,
    /// From the left, then from the right.
    Sides,
    /// Looped for a few seconds, ahead of you.
    Loop,
    /// Through the music bus, for a few seconds.
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

/// Every cue in the game, with its name and what it means.
pub const SOUND_CHECK: &[Entry] = &[
    e("Sprout hum", "A sprout humming softly in its planter. Seeds are silent; sprouts hum; grown plants sing their part of the song.", HUM, Play::Loop),
    e("Dry soil", "A crackle from a planter whose soil is dry: that plant is thirsty. Water it.", DRY, Play::Loop),
    e("Waterlogged soil", "A slow gurgle from a planter watered too much. The plant stops growing and falls out of tune until the soil drains.", SOGGY, Play::Loop),
    e("Aphids buzzing", "Aphids on a plant. It stops growing until you pick them off with the tend key.", BUZZ, Play::Loop),
    e("Aphids arrive", "The buzz of aphids landing on a plant.", PEST_ARRIVE, Play::Sides),
    e("A plant gets thirsty", "A soft droop: a plant's soil just went dry. Its song part droops in pitch too, until you water it.", THIRSTY, Play::Sides),
    e("A plant wilts", "A long fall: a plant left bone dry has wilted and gone quiet. Water it and it revives. Nothing is ever lost.", WILT, Play::Sides),
    e("A plant gets leggy", "A plant in bloom has grown leggy stems and grows slowly. The tend key prunes it.", LEGGY, Play::Sides),
    e("Sprouting", "A seed has sprouted.", SPROUT, Play::Sides),
    e("Growing", "A sprout is growing, and its voice joins the song softly.", GROW, Play::Sides),
    e("Blooming", "A plant is in bloom and sings in full voice. It now counts toward the harmony.", BLOOM, Play::Sides),
    e("Ripe", "A sparkle: a plant is ready to harvest with the tend key.", RIPE, Play::Sides),
    e("Watering", "The watering can.", WATER, Play::Once),
    e("Too much water", "A squelch: you watered soil that was already moist, and it is waterlogged now.", SQUELCH, Play::Once),
    e("Soil drains", "A drip: waterlogged soil has drained, and the plant grows again.", DRIP, Play::Once),
    e("Picking aphids", "You picked the aphids off.", PICK, Play::Once),
    e("Harvest", "You harvested a plant.", HARVEST, Play::Once),
    e("Pruning", "Snip, snip: you pruned leggy stems.", PRUNE, Play::Once),
    e("Planting", "You sowed a seed.", PLANT, Play::Once),
    e("Composting", "You pulled up a plant.", COMPOST, Play::Once),
    e("Revived", "A wilted plant lifts its leaves again.", REVIVE, Play::Once),
    e("Leaves rustle", "Nothing to tend on this plant right now.", RUSTLE, Play::Once),
    e("Nothing here", "Two wood knocks: there is nothing to do there.", NO, Play::Once),
    e("Footstep", "You step to another planter.", STEP, Play::Once),
    e("Empty planter", "A hollow knock as you step: this planter is empty soil.", EMPTY, Play::Once),
    e("Railing", "The rooftop railing: the edge of your garden.", RAILING, Play::Sides),
    e("The bench", "You sit on the bench, and time passes faster.", BENCH, Play::Once),
    e("Getting up", "You get up from the bench.", STAND, Play::Once),
    e("Rain shower begins", "Distant thunder: a shower is coming, and it will water every planter.", SHOWER, Play::Once),
    e("Rain", "Rain falling on the rooftop. While it rains, every planter is watered.", RAIN, Play::Loop),
    e("Shower passes", "The rain has stopped.", CLEAR, Play::Once),
    e("Morning", "A new garden day begins.", DAWN, Play::Once),
    e("Evening", "The day is nearly over.", DUSK, Play::Once),
    e("Sunday bells", "On the first Sunday of the month in your own garden, the evening: church bells down the street ring for evensong. The day is nearly over.", SUNDAY_BELLS, Play::Once),
    e("Tomato's part", "A kalimba arpeggio.", "rooftop-garden/song/tomato", Play::Music),
    e("Squash's part", "The bass.", "rooftop-garden/song/squash", Play::Music),
    e("Pumpkin's part", "The squash's line as a low bassoon. Pumpkins grow in pumpkin season, from 22 September to Thanksgiving.", PUMPKIN, Play::Music),
    e("Bean's part", "Plucked strings.", "rooftop-garden/song/bean", Play::Music),
    e("Sunflower's part", "The flute melody.", "rooftop-garden/song/sunflower", Play::Music),
    e("Lavender's part", "The chimes.", "rooftop-garden/song/lavender", Play::Music),
    e("Mint's part", "A cool, glassy pad.", "rooftop-garden/song/mint", Play::Music),
];
