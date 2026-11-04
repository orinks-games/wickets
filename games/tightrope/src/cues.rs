//! Every sound the game plays, by key, and the sound check's list of them
//! with what each one means. The keys are the asset contract: a recording
//! at `sounds/tightrope/<path>.ogg` replaces a prototype with no code change.

// Your balance.
pub const TONE: &str = "tightrope/balance/tone";
pub const STEADY: &str = "tightrope/balance/steady";
pub const DANGER: &str = "tightrope/balance/danger";
pub const SHIFT: &str = "tightrope/balance/shift";

// The wire and your feet.
pub const CREAK_1: &str = "tightrope/wire/creak_1";
pub const CREAK_2: &str = "tightrope/wire/creak_2";
pub const WOBBLE: &str = "tightrope/wire/wobble";
pub const STEP: &str = "tightrope/step/step";
pub const SLIP: &str = "tightrope/step/slip";
pub const REFUSED: &str = "tightrope/step/refused";
pub const CROUCH: &str = "tightrope/body/crouch";
pub const STAND: &str = "tightrope/body/stand";

// Wind.
pub const GUST_ALERT: &str = "tightrope/gust/alert";
pub const GUST_WARNING: &str = "tightrope/gust/warning";
pub const GUST_BLAST: &str = "tightrope/gust/blast";

// Pigeons.
pub const PIGEON_LAND: &str = "tightrope/pigeon/land";
pub const PIGEON_COO: &str = "tightrope/pigeon/coo";
pub const FLAP: &str = "tightrope/pigeon/flap";
pub const SHOO: &str = "tightrope/pigeon/shoo";

// Falling and arriving.
pub const FALL: &str = "tightrope/fall/fall";
pub const NET: &str = "tightrope/fall/net";
pub const CLIMB: &str = "tightrope/fall/climb";
pub const START: &str = "tightrope/goal/start";
pub const BEACON: &str = "tightrope/goal/beacon";
pub const ARRIVE: &str = "tightrope/goal/arrive";


// The city below.
pub const STREET: &str = "tightrope/ambience/street";
pub const WIND: &str = "tightrope/ambience/wind";
pub const HORN: &str = "tightrope/street/horn";
pub const SIREN: &str = "tightrope/street/siren";

// Menus.
pub const LOCKED: &str = "tightrope/ui/locked";

// Music.
/// The wire's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("tightrope");
pub const MUSIC_TITLE: &str = "tightrope/music/title";
pub const MUSIC_CROSSED: &str = "tightrope/music/crossed";
pub const MUSIC_VICTORY: &str = "tightrope/music/victory";
pub const MUSIC_LOSE: &str = "tightrope/music/lose";

/// How the sound check plays an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Once, from the middle.
    Once,
    /// Once, from the left, then from the right.
    Sides,
    /// Once, from straight ahead and a little away.
    Ahead,
    /// Looped for a few seconds.
    Loop,
    /// As music.
    Music,
}

/// One row of the sound check.
pub struct Entry {
    pub name: &'static str,
    pub meaning: &'static str,
    pub key: &'static str,
    pub play: Play,
}

/// Every cue, in the order a player meets them.
pub const SOUND_CHECK: &[Entry] = &[
    Entry {
        name: "Steady chime",
        meaning: "You are balanced again after leaning: safe to step.",
        key: STEADY,
        play: Play::Once,
    },
    Entry {
        name: "Danger alarm",
        meaning: "You are leaning hard and close to falling. It sounds from the side you lean to, and is spoken.",
        key: DANGER,
        play: Play::Sides,
    },
    Entry {
        name: "Weight shift",
        meaning: "You shifted your weight left or right.",
        key: SHIFT,
        play: Play::Sides,
    },
    Entry {
        name: "Step",
        meaning: "A safe step forward along the wire.",
        key: STEP,
        play: Play::Once,
    },
    Entry {
        name: "Wobble",
        meaning: "You stepped while leaning, or walked into a pigeon: the wire throws you further over.",
        key: WOBBLE,
        play: Play::Once,
    },
    Entry {
        name: "Slip",
        meaning: "Slick wire challenges only: a step slid your foot to one side.",
        key: SLIP,
        play: Play::Sides,
    },
    Entry {
        name: "Step refused",
        meaning: "You cannot step yet: too soon after the last step, or you are crouching.",
        key: REFUSED,
        play: Play::Once,
    },
    Entry {
        name: "Wire creak",
        meaning: "The wire under you. It creaks faster the harder you sway, from the side you lean to.",
        key: CREAK_1,
        play: Play::Sides,
    },
    Entry {
        name: "Second wire creak",
        meaning: "Another creak of the wire. It means the same as the first.",
        key: CREAK_2,
        play: Play::Once,
    },
    Entry {
        name: "Crouch",
        meaning: "You crouched low: steadier, and gusts push less, but you cannot step.",
        key: CROUCH,
        play: Play::Once,
    },
    Entry {
        name: "Stand",
        meaning: "You stood up and can walk again.",
        key: STAND,
        play: Play::Once,
    },
    Entry {
        name: "Gust alert",
        meaning: "A gust is coming, from the side this whistle sounds. It is always spoken too.",
        key: GUST_ALERT,
        play: Play::Sides,
    },
    Entry {
        name: "Gust whoosh",
        meaning: "The rising whoosh of a gust on its way. It peaks the moment the wind hits.",
        key: GUST_WARNING,
        play: Play::Sides,
    },
    Entry {
        name: "Gust blast",
        meaning: "The gust is pushing you now, away from the side it blows from.",
        key: GUST_BLAST,
        play: Play::Sides,
    },
    Entry {
        name: "Pigeon landing",
        meaning: "A pigeon landed on the wire ahead and bounced it a little.",
        key: PIGEON_LAND,
        play: Play::Sides,
    },
    Entry {
        name: "Pigeon cooing",
        meaning: "A pigeon sits on the wire ahead. It gets closer as you walk.",
        key: PIGEON_COO,
        play: Play::Loop,
    },
    Entry {
        name: "Shoo",
        meaning: "You waved and hissed at a pigeon.",
        key: SHOO,
        play: Play::Once,
    },
    Entry {
        name: "Wings flapping",
        meaning: "A pigeon flew off.",
        key: FLAP,
        play: Play::Once,
    },
    Entry {
        name: "Fall",
        meaning: "You fell off the wire.",
        key: FALL,
        play: Play::Once,
    },
    Entry {
        name: "Safety net",
        meaning: "The net caught you. You lose a life.",
        key: NET,
        play: Play::Once,
    },
    Entry {
        name: "Climb back",
        meaning: "You are back on the wire where you fell.",
        key: CLIMB,
        play: Play::Once,
    },
    Entry {
        name: "Wire start",
        meaning: "A new wire begins.",
        key: START,
        play: Play::Once,
    },
    Entry {
        name: "Rooftop beacon",
        meaning: "A chime from the far rooftop, straight ahead. It grows louder as you get close.",
        key: BEACON,
        play: Play::Ahead,
    },
    Entry {
        name: "Arrival",
        meaning: "You stepped onto the far rooftop.",
        key: ARRIVE,
        play: Play::Once,
    },
];
