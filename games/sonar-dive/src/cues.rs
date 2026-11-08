//! Every sound the game plays, by key, and what each one means.

use hearken::SpatialConfig;

use crate::rules::Kind;

/// How placed sounds fade with distance on a dive: half the engine's usual
/// rolloff, so an echo or the bell 60 metres off is quieter but still heard
/// over the ambience. The sound check uses the same model.
pub fn spatial() -> SpatialConfig {
    SpatialConfig {
        rolloff: 0.5,
        ..SpatialConfig::default()
    }
}

/// The dive's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("sonar-dive");
pub const PING: &str = "sonar-dive/sonar/ping";
pub const NOT_READY: &str = "sonar-dive/sonar/not_ready";
pub const ECHO_WRECK: &str = "sonar-dive/echo/wreck";
pub const ECHO_WRECK_EMPTY: &str = "sonar-dive/echo/wreck_empty";
pub const ECHO_CHEST: &str = "sonar-dive/echo/chest";
pub const ECHO_MINE: &str = "sonar-dive/echo/mine";
pub const ECHO_ROCK: &str = "sonar-dive/echo/rock";
pub const ECHO_WALL: &str = "sonar-dive/echo/wall";
pub const ECHO_WHALE: &str = "sonar-dive/echo/whale";
pub const ECHO_CURRENT: &str = "sonar-dive/echo/current";
pub const ECHO_GALLEON: &str = "sonar-dive/echo/galleon";
pub const ECHO_PEARL: &str = "sonar-dive/echo/pearl";
pub const PEARL: &str = "sonar-dive/loot/pearl";
pub const WHALE_ROAD: &str = "sonar-dive/whale/road";
pub const ENGINE: &str = "sonar-dive/sub/engine";
pub const TURN_TICK: &str = "sonar-dive/sub/turn_tick";
pub const THROTTLE_UP: &str = "sonar-dive/sub/throttle_up";
pub const THROTTLE_DOWN: &str = "sonar-dive/sub/throttle_down";
pub const BELL: &str = "sonar-dive/nav/bell";
pub const BUOY_DROP: &str = "sonar-dive/nav/buoy_drop";
pub const BUOY_BEEP: &str = "sonar-dive/nav/buoy_beep";
pub const MINE_TICK: &str = "sonar-dive/mine/tick";
pub const MINE_BLAST: &str = "sonar-dive/mine/explode";
pub const CHEST: &str = "sonar-dive/loot/chest";
pub const CLANK: &str = "sonar-dive/loot/salvage_clank";
pub const SALVAGED: &str = "sonar-dive/loot/salvage_done";
pub const SCRAPE: &str = "sonar-dive/hazard/scrape";
pub const CURRENT: &str = "sonar-dive/hazard/current";
pub const WHALE_SONG: &str = "sonar-dive/whale/song";
pub const WHALE_AIR: &str = "sonar-dive/whale/air";
pub const AIR_WARNING: &str = "sonar-dive/status/air_warning";
pub const SURFACE: &str = "sonar-dive/status/surface";
pub const DROWN: &str = "sonar-dive/status/drown";
pub const AMB_SHALLOW: &str = "sonar-dive/amb/shallow";
pub const AMB_DEEP: &str = "sonar-dive/amb/deep";
pub const AMB_ABYSS: &str = "sonar-dive/amb/abyss";
pub const MUSIC_TITLE: &str = "sonar-dive/music/title";
pub const MUSIC_WIN: &str = "sonar-dive/music/win";
pub const MUSIC_LOSE: &str = "sonar-dive/music/lose";
pub const MUSIC_SURFACED: &str = "sonar-dive/music/surfaced";
pub const MUSIC_SURFACE: &str = "sonar-dive/music/surface";

/// The echo a kind sends back.
pub fn echo(kind: Kind, empty: bool) -> &'static str {
    match kind {
        Kind::Wreck if empty => ECHO_WRECK_EMPTY,
        Kind::Wreck => ECHO_WRECK,
        Kind::Chest => ECHO_CHEST,
        Kind::Mine => ECHO_MINE,
        Kind::Rock => ECHO_ROCK,
        Kind::Wall => ECHO_WALL,
        Kind::Whale => ECHO_WHALE,
        Kind::Current => ECHO_CURRENT,
        Kind::Galleon => ECHO_GALLEON,
        Kind::Pearl => ECHO_PEARL,
    }
}

/// The ambience for a dive's depth: deeper is darker.
pub fn ambience(dive: u32) -> &'static str {
    match dive {
        0..=2 => AMB_SHALLOW,
        3..=4 => AMB_DEEP,
        _ => AMB_ABYSS,
    }
}

/// How a cue is heard in the sound check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Once, in front of you.
    Ahead,
    /// Once, not placed.
    Flat,
    /// Looping until you choose another, on the given bus.
    Loop,
    /// Music.
    Music,
}

/// One row of the sound check.
pub struct Cue {
    pub key: &'static str,
    pub name: &'static str,
    pub meaning: &'static str,
    pub play: Play,
}

/// Every cue with its name and meaning, in the order the sound check
/// teaches them.
pub const CUES: &[Cue] = &[
    Cue {
        key: PING,
        name: "Sonar ping",
        meaning: "your own ping going out. Echoes follow from wherever things are.",
        play: Play::Flat,
    },
    Cue {
        key: NOT_READY,
        name: "Sonar recharging",
        meaning: "a ping was asked for too soon. Wait a moment.",
        play: Play::Flat,
    },
    Cue {
        key: ECHO_WRECK,
        name: "Wreck echo",
        meaning: "a hollow metal clang: a shipwreck with treasure in its hold. Stop beside it to salvage.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_WRECK_EMPTY,
        name: "Empty wreck echo",
        meaning: "a dull thud: a wreck you have already emptied.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_CHEST,
        name: "Treasure chest echo",
        meaning: "a bright double chime: a chest on the seabed. Swim into it.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_MINE,
        name: "Mine echo",
        meaning: "a harsh buzzing ring: a mine. Steer around it.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_ROCK,
        name: "Rock echo",
        meaning: "a short stony knock: a rock outcrop that will stop you.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_WALL,
        name: "Rock wall echo",
        meaning: "a deep rumble: the cliff at the edge of the dive site.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_WHALE,
        name: "Whale echo",
        meaning: "a soft low hum: a friendly whale. Pass close and it shares air.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_CURRENT,
        name: "Current echo",
        meaning: "a rushing hiss: a current that will push you.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_GALLEON,
        name: "Galleon echo",
        meaning: "a great gong: the treasure galleon of the monthly event.",
        play: Play::Ahead,
    },
    Cue {
        key: ECHO_PEARL,
        name: "Glow pearl echo",
        meaning: "a glassy rising chime: the glow pearl of the new moon's glow dive. Swim into it.",
        play: Play::Ahead,
    },
    Cue {
        key: BELL,
        name: "The boat's bell",
        meaning: "rings every few seconds from the boat. Return to it to surface and bank your treasure.",
        play: Play::Ahead,
    },
    Cue {
        key: BUOY_DROP,
        name: "Buoy dropped",
        meaning: "you let a marker buoy go where you are.",
        play: Play::Flat,
    },
    Cue {
        key: BUOY_BEEP,
        name: "Buoy beep",
        meaning: "a marker buoy you dropped, beeping from where you left it.",
        play: Play::Ahead,
    },
    Cue {
        key: MINE_TICK,
        name: "Mine proximity tick",
        meaning: "a mine is near. The ticks come faster the closer it is.",
        play: Play::Ahead,
    },
    Cue {
        key: MINE_BLAST,
        name: "Mine explosion",
        meaning: "you touched a mine and lost a hull point.",
        play: Play::Flat,
    },
    Cue {
        key: CHEST,
        name: "Chest collected",
        meaning: "coins from a chest are aboard.",
        play: Play::Flat,
    },
    Cue {
        key: PEARL,
        name: "Glow pearl collected",
        meaning: "the glow pearl's coins are aboard.",
        play: Play::Flat,
    },
    Cue {
        key: CLANK,
        name: "Salvage clank",
        meaning: "the grab arm working in a wreck. Stay stopped until it finishes.",
        play: Play::Ahead,
    },
    Cue {
        key: SALVAGED,
        name: "Salvage done",
        meaning: "the wreck's treasure is aboard.",
        play: Play::Flat,
    },
    Cue {
        key: SCRAPE,
        name: "Scrape",
        meaning: "you hit rock. The engine stops and you lose a little air.",
        play: Play::Ahead,
    },
    Cue {
        key: CURRENT,
        name: "In a current",
        meaning: "loops while a current pushes you.",
        play: Play::Loop,
    },
    Cue {
        key: WHALE_SONG,
        name: "Whale song",
        meaning: "a whale calling from where it swims.",
        play: Play::Ahead,
    },
    Cue {
        key: WHALE_AIR,
        name: "Whale's air",
        meaning: "a whale shared air with you: your tank gains time.",
        play: Play::Flat,
    },
    Cue {
        key: AIR_WARNING,
        name: "Air warning",
        meaning: "half, a quarter, or the last fifteen seconds of air.",
        play: Play::Flat,
    },
    Cue {
        key: THROTTLE_UP,
        name: "Throttle up",
        meaning: "the engine telegraph moving one step faster.",
        play: Play::Flat,
    },
    Cue {
        key: THROTTLE_DOWN,
        name: "Throttle down",
        meaning: "the engine telegraph moving one step slower.",
        play: Play::Flat,
    },
    Cue {
        key: TURN_TICK,
        name: "Turn tick",
        meaning: "a soft click for every 30 degrees you turn.",
        play: Play::Flat,
    },
    Cue {
        key: ENGINE,
        name: "Engine",
        meaning: "your sub's motor. It rises in pitch as you go faster.",
        play: Play::Loop,
    },
    Cue {
        key: SURFACE,
        name: "Surfacing",
        meaning: "you reached the bell and rose to the boat.",
        play: Play::Flat,
    },
    Cue {
        key: DROWN,
        name: "Out of air",
        meaning: "the tank ran dry and the dive is lost.",
        play: Play::Flat,
    },
];
