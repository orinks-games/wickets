//! The field guide and everything else the game keeps between walks.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records, Season};
use serde::{Deserialize, Serialize};

use crate::birds::{self, Kind, Species};
use crate::trails::Trail;
use crate::ID;

/// The records key the guide is saved under.
pub fn key() -> String {
    format!("{ID}/guide")
}

/// One bird in the guide.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Songs learned: 0 to 3 for a singing bird, 0 or 1 for a flock bird.
    #[serde(default)]
    pub verses: u32,
    /// Times befriended, found or counted.
    #[serde(default)]
    pub met: u32,
    /// Where and when it was first met: "on the woodland trail in autumn".
    #[serde(default)]
    pub first: String,
}

/// What learning a song changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Learned {
    /// The bird was new to the guide.
    pub new_bird: bool,
    /// The song was one the guide did not have yet.
    pub new_verse: bool,
    /// Songs of this bird now in the guide.
    pub verses: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guide {
    /// By species id.
    #[serde(default)]
    pub entries: BTreeMap<String, Entry>,
    /// Trail walks finished, challenges included.
    #[serde(default)]
    pub walks: u32,
    /// Steps taken on every walk.
    #[serde(default)]
    pub steps: u32,
    #[serde(default)]
    pub choruses: u32,
    /// The most birds picked out of one dawn chorus.
    #[serde(default)]
    pub chorus_best: u32,
    #[serde(default)]
    pub counts: u32,
    /// Flocks counted exactly, in every count.
    #[serde(default)]
    pub flocks_exact: u32,
    /// Birds counted in every flock.
    #[serde(default)]
    pub flock_birds: u32,
    /// Visits to the practice glade.
    #[serde(default)]
    pub practice: u32,
    /// Whether the player has turned on spoken notes after each song.
    #[serde(default)]
    pub call_outs: bool,
}

impl Guide {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Guide {
        Records::load(ctx, &key())
    }

    /// The guide as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> Guide {
        Records::peek(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Kinds of bird in the guide.
    pub fn birds(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Songs in the guide, every bird together.
    pub fn songs(&self) -> u32 {
        self.entries.values().map(|e| e.verses).sum()
    }

    pub fn entry(&self, species: &Species) -> Option<&Entry> {
        self.entries.get(species.id)
    }

    pub fn has(&self, species: &Species) -> bool {
        self.entries.contains_key(species.id)
    }

    /// Songs of this bird in the guide.
    pub fn verses_of(&self, species: &Species) -> u32 {
        self.entry(species).map_or(0, |e| e.verses)
    }

    /// The verse a bird will sing to the player next: the first one not
    /// yet learned, or its signature once all are.
    pub fn next_verse(&self, species: &Species) -> u32 {
        let learned = self.verses_of(species);
        if learned >= species.verses() {
            0
        } else {
            learned
        }
    }

    /// Notes that the player answered verse `verse` of `species`, met
    /// `where_when` ("on the woodland trail in autumn").
    pub fn learn(&mut self, species: &Species, verse: u32, where_when: &str) -> Learned {
        let new_bird = !self.has(species);
        let entry = self.entries.entry(species.id.to_string()).or_default();
        if new_bird {
            entry.first = where_when.to_string();
        }
        entry.met += 1;
        let new_verse = verse == entry.verses && entry.verses < species.verses();
        if new_verse {
            entry.verses += 1;
        }
        Learned {
            new_bird,
            new_verse,
            verses: entry.verses,
        }
    }

    /// Notes a bird heard but not yet answered, such as one picked out of
    /// the dawn chorus: it goes in the guide with no songs learned.
    pub fn hear(&mut self, species: &Species, where_when: &str) -> bool {
        let new_bird = !self.has(species);
        let entry = self.entries.entry(species.id.to_string()).or_default();
        if new_bird {
            entry.first = where_when.to_string();
        }
        entry.met += 1;
        new_bird
    }

    /// Whether `trail` is open.
    pub fn opens(&self, trail: Trail) -> bool {
        self.birds() >= trail.needs()
    }

    /// Kinds of bird still to meet.
    pub fn missing(&self) -> u32 {
        birds::ALL.len() as u32 - self.birds()
    }

    /// Whether the player has walked at all.
    pub fn any(&self) -> bool {
        self.walks > 0 || self.birds() > 0 || self.practice > 0
    }
}

/// Every song the guide can hold.
pub fn total_songs() -> u32 {
    birds::ALL.iter().map(|s| s.verses()).sum()
}

/// "on the woodland trail in autumn".
pub fn where_when(trail: Trail, season: Season) -> String {
    format!("on the {} in {}", trail.name(), season.name())
}

/// Where a bird that is not on any trail is met.
pub fn special_where(kind: Kind) -> &'static str {
    match kind {
        Kind::Flock => "on the migration count",
        Kind::Trail => "in the dawn chorus",
    }
}
