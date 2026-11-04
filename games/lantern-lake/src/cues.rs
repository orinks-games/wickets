//! Every sound key the game plays, and the sound check's list of them.

pub const MUSIC_TITLE: &str = "lantern-lake/music/title";
pub const MUSIC_COMPLETE: &str = "lantern-lake/music/complete";
pub const MUSIC_GOODNIGHT: &str = "lantern-lake/music/goodnight";

pub const SONG_BASE: &str = "lantern-lake/song/base";
pub const SONG_LAYERS: [&str; crate::rules::SONG_LAYERS] = [
    "lantern-lake/song/layer_1",
    "lantern-lake/song/layer_2",
    "lantern-lake/song/layer_3",
    "lantern-lake/song/layer_4",
    "lantern-lake/song/layer_5",
    "lantern-lake/song/layer_6",
    "lantern-lake/song/layer_7",
    "lantern-lake/song/layer_8",
    "lantern-lake/song/layer_9",
    "lantern-lake/song/layer_10",
];
/// On Lantern Festival night, the song's last voice: temple bells, in
/// place of the choir.
pub const TEMPLE_BELLS: &str = "lantern-lake/song/temple_bells";
/// One song loop, seconds: two bars at 60 beats a minute.
pub const SONG_LOOP: f64 = 8.0;

pub const CHIMES: [&str; 3] = [
    "lantern-lake/lantern/chime_1",
    "lantern-lake/lantern/chime_2",
    "lantern-lake/lantern/chime_3",
];
pub const BEACON: &str = "lantern-lake/lantern/beacon";
pub const GATHER: &str = "lantern-lake/lantern/gather";
pub const ASLEEP: &str = "lantern-lake/lantern/asleep";

pub const STROKE: &str = "lantern-lake/canoe/stroke";
pub const BACKSTROKE: &str = "lantern-lake/canoe/backstroke";
pub const TURN: &str = "lantern-lake/canoe/turn";
pub const REEDS: &str = "lantern-lake/canoe/reeds";
pub const BUMP: &str = "lantern-lake/canoe/bump";
pub const LAP: &str = "lantern-lake/water/lap";
pub const GLIDE: &str = "lantern-lake/water/glide";

pub const FROGS: &str = "lantern-lake/night/frogs";
pub const CRICKETS: &str = "lantern-lake/night/crickets";
pub const REEDS_WIND: &str = "lantern-lake/night/reeds_wind";
pub const LOON: &str = "lantern-lake/night/loon";
pub const OWL: &str = "lantern-lake/night/owl";
pub const FISH: &str = "lantern-lake/night/fish";
/// A wolf far off on land, on a full moon night.
pub const WOLF: &str = "lantern-lake/night/wolf";

pub const RAIN: &str = "lantern-lake/weather/rain";
pub const BREEZE: &str = "lantern-lake/weather/breeze";
pub const MIST: &str = "lantern-lake/weather/mist";

pub const DOCK: &str = "lantern-lake/discover/dock";
pub const ISLAND: &str = "lantern-lake/discover/island";
pub const FIREFLIES: &str = "lantern-lake/discover/fireflies";
pub const STREAM: &str = "lantern-lake/discover/stream";
pub const LILY_PADS: &str = "lantern-lake/discover/lilypads";
pub const BUOY: &str = "lantern-lake/discover/buoy";
pub const BEAVER: &str = "lantern-lake/discover/beaver";
pub const WILLOW: &str = "lantern-lake/discover/willow";
pub const FOUND: &str = "lantern-lake/discover/found";

/// How the sound check plays an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Once, from the centre.
    Once,
    /// From the left, then from the right.
    Sides,
    /// Looped for a few seconds, ahead of you.
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

/// Every cue in the game, with its name and what it means.
pub const SOUND_CHECK: &[Entry] = &[
    e("Lantern chime, first voice", "A lantern on the water. It rings from where it floats, and rings faster as you get near.", CHIMES[0], Play::Sides),
    e("Lantern chime, second voice", "Another lantern. Lanterns ring one of 3 bells; all 3 mean the same thing.", CHIMES[1], Play::Sides),
    e("Lantern chime, third voice", "The third lantern bell.", CHIMES[2], Play::Sides),
    e("Lit lantern", "In a constellation, only the lit lantern rings, with this brighter two-note chime. Gather it to light the next.", BEACON, Play::Sides),
    e("Lantern gathered", "You reached a lantern and gathered it. A new voice joins the song.", GATHER, Play::Once),
    e("Sleeping lantern", "An unlit lantern in a constellation: not yet. Follow the lit one.", ASLEEP, Play::Once),
    e("Paddle stroke", "One stroke forward, on alternating sides of the canoe.", STROKE, Play::Sides),
    e("Back paddle", "A stroke backward: it slows you, then backs you up.", BACKSTROKE, Play::Once),
    e("Turning", "A short press of a turn key: the canoe turns a little.", TURN, Play::Once),
    e("Reeds", "The canoe has reached the shore. Turn toward open water.", REEDS, Play::Once),
    e("Bump", "The canoe nudged the island's rocks.", BUMP, Play::Once),
    e("Gliding", "Water rushing past the hull: louder the faster you go.", GLIDE, Play::Loop),
    e("Frogs", "Frogs singing from one place on the shore all night: a landmark.", FROGS, Play::Loop),
    e("Crickets", "Crickets in one place on the shore: another landmark.", CRICKETS, Play::Loop),
    e("Wind in the reeds", "The reeds rustling at one spot on the shore.", REEDS_WIND, Play::Loop),
    e("Owl", "The owl on the island hoots every so often: the island is where it calls from.", OWL, Play::Sides),
    e("The old dock", "A discovery on the shore: creaking boards and water slapping posts.", DOCK, Play::Loop),
    e("The island", "A discovery out on the water: waves lapping on rocks, and an owl.", ISLAND, Play::Loop),
    e("The firefly meadow", "A discovery on the shore: a faint, twinkling shimmer.", FIREFLIES, Play::Loop),
    e("The little stream", "A discovery on the shore: running, bubbling water.", STREAM, Play::Loop),
    e("The lily pads", "A discovery on the water: a bullfrog and soft plops.", LILY_PADS, Play::Loop),
    e("The bell buoy", "A discovery on the water: a low bell rocking.", BUOY, Play::Loop),
    e("The beaver lodge", "A discovery on the shore: gnawing, then a tail slap.", BEAVER, Play::Loop),
    e("The weeping willow", "A discovery on the shore: leaves whispering.", WILLOW, Play::Loop),
    e("Discovery", "You found a place. It goes in your journal.", FOUND, Play::Once),
];
