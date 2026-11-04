//! The monsters, the songs and the nights they are danced on.
//!
//! Every song has two dance beats a bar, and is two parts, A and B, of
//! four bars each, played A A B B twice: 32 bars, 64 beats.

use parlor_core::ParlorAction;

/// One monster that comes up to dance. Each has its own sound and its own
/// move, danced with one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Monster {
    Zombie,
    Skeleton,
    Mummy,
    Vampire,
    Werewolf,
    Ghost,
}

impl Monster {
    pub const ALL: [Monster; 6] = [
        Monster::Zombie,
        Monster::Skeleton,
        Monster::Mummy,
        Monster::Vampire,
        Monster::Werewolf,
        Monster::Ghost,
    ];

    /// The monster's name, capitalised: "Zombie".
    pub fn word(self) -> &'static str {
        match self {
            Monster::Zombie => "Zombie",
            Monster::Skeleton => "Skeleton",
            Monster::Mummy => "Mummy",
            Monster::Vampire => "Vampire",
            Monster::Werewolf => "Werewolf",
            Monster::Ghost => "Ghost",
        }
    }

    /// The name inside a sentence: "the zombie".
    pub fn lower(self) -> &'static str {
        match self {
            Monster::Zombie => "the zombie",
            Monster::Skeleton => "the skeleton",
            Monster::Mummy => "the mummy",
            Monster::Vampire => "the vampire",
            Monster::Werewolf => "the werewolf",
            Monster::Ghost => "the ghost",
        }
    }

    /// The monster's move: "the drag".
    pub fn dance(self) -> &'static str {
        match self {
            Monster::Zombie => "the drag",
            Monster::Skeleton => "the rattle",
            Monster::Mummy => "the unwrap",
            Monster::Vampire => "the swoop",
            Monster::Werewolf => "the howl",
            Monster::Ghost => "the float",
        }
    }

    /// The shared control that dances its move.
    pub fn action(self) -> ParlorAction {
        match self {
            Monster::Zombie => ParlorAction::Down,
            Monster::Skeleton => ParlorAction::Up,
            Monster::Mummy => ParlorAction::Primary,
            Monster::Vampire => ParlorAction::Left,
            Monster::Werewolf => ParlorAction::Right,
            Monster::Ghost => ParlorAction::Secondary,
        }
    }

    pub fn for_action(action: ParlorAction) -> Option<Monster> {
        Monster::ALL.into_iter().find(|m| m.action() == action)
    }

    /// What the monster sounds like as it comes up, for the sound check.
    pub fn call_words(self) -> &'static str {
        match self {
            Monster::Zombie => "a low groan",
            Monster::Skeleton => "a rattle of dry bones",
            Monster::Mummy => "a dusty moan",
            Monster::Vampire => "a cape swishing and a hiss",
            Monster::Werewolf => "a short howl",
            Monster::Ghost => "a high, wavering whoo",
        }
    }

    /// What its move sounds like when you dance it.
    pub fn move_words(self) -> &'static str {
        match self {
            Monster::Zombie => "a foot dragged through the dirt",
            Monster::Skeleton => "bones clacking together",
            Monster::Mummy => "a strip of cloth tearing",
            Monster::Vampire => "a cape whooshing past on your left",
            Monster::Werewolf => "claws scraping on your right",
            Monster::Ghost => "a chain clinking as it floats up",
        }
    }
}

/// One song the graveyard band plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Song {
    /// Names its sounds: `graveyard-shuffle/song/<id>/a`.
    pub id: &'static str,
    pub name: &'static str,
    /// Dance beats a minute, two to a bar.
    pub bpm: u32,
}

impl Song {
    /// The sound key of part A (`b == false`) or part B.
    pub fn part_key(&self, b: bool) -> String {
        let part = if b { "b" } else { "a" };
        format!("{}/song/{}/{part}", crate::ID, self.id)
    }

    /// Milliseconds a beat lasts.
    pub fn beat_ms(&self) -> f64 {
        60_000.0 / self.bpm as f64
    }
}

const fn song(id: &'static str, name: &'static str, bpm: u32) -> Song {
    Song { id, name, bpm }
}

/// Every song of the four nights, in order, three to a night. The ids
/// match `sound-specs/graveyard-shuffle.json`.
pub const SONGS: [Song; 12] = [
    song("warm-up", "The Graveyard Warm-up", 88),
    song("boneyard", "Boneyard Boogie", 92),
    song("shallow-grave", "The Shallow Grave Shuffle", 96),
    song("cobweb", "The Cobweb Crawl", 98),
    song("coffin-lid", "The Coffin Lid Clap", 100),
    song("drip", "Drip in the Crypt", 104),
    song("bat-wing", "Bat Wing Boogie", 106),
    song("full-moon", "The Full Moon Stomp", 108),
    song("bell-tower", "The Bell Tower Bop", 112),
    song("lightning-rod", "Lightning Rod Rock", 114),
    song("slab", "Off the Slab", 118),
    song("big-shuffle", "The Graveyard Shuffle", 124),
];

/// How a night of songs is danced: which monsters come, and how often.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    /// The monsters that come up.
    pub monsters: &'static [Monster],
    /// The chance of a monster on each bar's first beat.
    pub downbeat: f64,
    /// The chance of a monster on each bar's second beat.
    pub offbeat: f64,
}

pub const FIRST_THREE: &[Monster] = &[Monster::Zombie, Monster::Skeleton, Monster::Mummy];
pub const WITH_VAMPIRE: &[Monster] = &[
    Monster::Zombie,
    Monster::Skeleton,
    Monster::Mummy,
    Monster::Vampire,
];
pub const EVERY_MONSTER: &[Monster] = &Monster::ALL;

/// Where the bed sound of a night comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Gate,
    Crypt,
    Church,
    Lab,
}

/// One night of three songs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Night {
    pub number: u32,
    /// "the graveyard gate".
    pub name: &'static str,
    /// One sentence on the scene, for the night's help.
    pub scene: &'static str,
    pub place: Place,
    pub level: Level,
    /// Indexes into [`SONGS`].
    pub songs: [usize; 3],
}

impl Night {
    /// "Night 2, the crypt".
    pub fn title(&self) -> String {
        format!("Night {}, {}", self.number, self.name)
    }

    /// The title inside a sentence: "night 4, Doctor Volt's laboratory".
    pub fn title_in_sentence(&self) -> String {
        format!("night {}, {}", self.number, self.name)
    }

    pub fn songs(&self) -> [&'static Song; 3] {
        self.songs.map(|i| &SONGS[i])
    }

    /// The monsters this night adds, for the intro.
    pub fn new_monsters(&self) -> &'static [Monster] {
        match self.number {
            1 => FIRST_THREE,
            2 => &[Monster::Vampire],
            3 => &[Monster::Werewolf, Monster::Ghost],
            _ => &[],
        }
    }
}

pub const NIGHTS: [Night; 4] = [
    Night {
        number: 1,
        name: "the graveyard gate",
        scene: "A rusty gate, a few loose headstones, and the first monsters to hear about the new dance.",
        place: Place::Gate,
        level: Level {
            monsters: FIRST_THREE,
            downbeat: 0.8,
            offbeat: 0.0,
        },
        songs: [0, 1, 2],
    },
    Night {
        number: 2,
        name: "the crypt",
        scene: "Down the steps into the family crypt, where the dripping ceiling keeps time.",
        place: Place::Crypt,
        level: Level {
            monsters: WITH_VAMPIRE,
            downbeat: 0.9,
            offbeat: 0.12,
        },
        songs: [3, 4, 5],
    },
    Night {
        number: 3,
        name: "the old church",
        scene: "The ruined church on the hill, with bats in the bell tower and a full moon through the roof.",
        place: Place::Church,
        level: Level {
            monsters: EVERY_MONSTER,
            downbeat: 1.0,
            offbeat: 0.3,
        },
        songs: [6, 7, 8],
    },
    Night {
        number: 4,
        name: "Doctor Volt's laboratory",
        scene: "The laboratory where the dance began, with the lightning rods crackling and every monster in town.",
        place: Place::Lab,
        level: Level {
            monsters: EVERY_MONSTER,
            downbeat: 1.0,
            offbeat: 0.55,
        },
        songs: [9, 10, 11],
    },
];

/// The night a song belongs to, by its index in [`SONGS`].
pub fn night_of(song: usize) -> &'static Night {
    &NIGHTS[(song / 3).min(NIGHTS.len() - 1)]
}

/// A song by id.
pub fn by_id(id: &str) -> Option<&'static Song> {
    SONGS.iter().find(|t| t.id == id)
}
