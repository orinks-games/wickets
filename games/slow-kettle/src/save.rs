//! What the tea house remembers between visits: coins, what you have
//! bought, the regulars' stories, keepsakes and the counts for Records.

use std::collections::{BTreeMap, BTreeSet};

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::days::Occasion;
use crate::rules::Known;
use crate::teas::{cup_by_id, tea_count, tune_by_id, Cup, Tune, CUPS};
use crate::ID;

/// The records key the house is saved under.
pub fn key() -> String {
    format!("{ID}/house")
}

/// A keepsake from a holiday guest.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keepsake {
    /// The occasion's id.
    pub occasion: String,
    pub year: i32,
    /// What it is: "an old brass key, cold to the touch".
    pub words: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct House {
    #[serde(default)]
    pub coins: u32,
    /// Teas bought in the shop, by id.
    #[serde(default)]
    pub teas: BTreeSet<String>,
    /// Cups bought, by id. The plain white cup is always yours.
    #[serde(default)]
    pub cups: BTreeSet<String>,
    /// Music box tunes bought, by id. The first one is the music box.
    #[serde(default)]
    pub tunes: BTreeSet<String>,
    /// The tune the music box plays, or `None` for silence.
    #[serde(default)]
    pub tune: Option<String>,
    #[serde(default)]
    pub sand_timer: bool,
    /// The kettle's spoken call-outs are off.
    #[serde(default)]
    pub quiet: bool,
    #[serde(default)]
    pub afternoons: u32,
    #[serde(default)]
    pub evenings: u32,
    #[serde(default)]
    pub cups_served: u32,
    #[serde(default)]
    pub perfect: u32,
    #[serde(default)]
    pub coins_earned: u32,
    /// Every tea brewed at least once, by id.
    #[serde(default)]
    pub tasted: BTreeSet<String>,
    /// How many parts of each regular's story have been told, by id.
    #[serde(default)]
    pub chapters: BTreeMap<String, u32>,
    /// The session each regular last came in, by id.
    #[serde(default)]
    pub last_seen: BTreeMap<String, u32>,
    #[serde(default)]
    pub keepsakes: Vec<Keepsake>,
    /// Practice cups, which count for nothing but are nice to know.
    #[serde(default)]
    pub practice_cups: u32,
}

impl House {
    pub fn load(ctx: &mut Ctx<Parlor>) -> House {
        Records::load(ctx, &key())
    }

    /// The house as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> House {
        Records::peek(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Afternoons and evenings played.
    pub fn sessions(&self) -> u32 {
        self.afternoons + self.evenings
    }

    /// What planning the guests needs.
    pub fn known(&self) -> Known {
        Known {
            sessions: self.sessions(),
            last_seen: self.last_seen.clone(),
        }
    }

    pub fn owns_tea(&self, id: &str) -> bool {
        self.teas.contains(id)
    }

    pub fn owns_cup(&self, id: &str) -> bool {
        id == CUPS[0].id || self.cups.contains(id)
    }

    /// The cups you own, the plain white one first.
    pub fn cupboard(&self) -> Vec<&'static Cup> {
        CUPS.iter().filter(|c| self.owns_cup(c.id)).collect()
    }

    pub fn has_music_box(&self) -> bool {
        !self.tunes.is_empty()
    }

    /// The tune playing, if the music box is on.
    pub fn playing(&self) -> Option<&'static Tune> {
        self.tune.as_deref().and_then(tune_by_id)
    }

    /// The next setting of the music box: each tune you own, then off.
    pub fn next_tune(&mut self) -> Option<&'static Tune> {
        let owned: Vec<&'static str> = crate::teas::TUNES
            .iter()
            .filter(|t| self.tunes.contains(t.id))
            .map(|t| t.id)
            .collect();
        if owned.is_empty() {
            self.tune = None;
            return None;
        }
        let at = self
            .tune
            .as_deref()
            .and_then(|id| owned.iter().position(|o| *o == id));
        self.tune = match at {
            None => Some(owned[0].to_string()),
            Some(i) if i + 1 < owned.len() => Some(owned[i + 1].to_string()),
            Some(_) => None,
        };
        self.playing()
    }

    /// Whether a keepsake for this occasion and year is already kept.
    pub fn has_keepsake(&self, o: Occasion, year: i32) -> bool {
        self.keepsakes
            .iter()
            .any(|k| k.occasion == o.id() && k.year == year)
    }

    /// Keeps a keepsake. False if it was already kept.
    pub fn keep(&mut self, o: Occasion, year: i32) -> bool {
        if self.has_keepsake(o, year) {
            return false;
        }
        self.keepsakes.push(Keepsake {
            occasion: o.id().to_string(),
            year,
            words: o.keepsake(year),
        });
        true
    }

    /// Which occasions have given at least one keepsake.
    pub fn occasions_kept(&self) -> BTreeSet<String> {
        self.keepsakes.iter().map(|k| k.occasion.clone()).collect()
    }

    /// "9 of 28 teas".
    pub fn tasted_words(&self) -> String {
        format!("{} of {} teas", self.tasted.len(), tea_count())
    }

    /// The cup with this id, if you own it, else the plain white cup.
    pub fn cup(&self, id: &str) -> &'static Cup {
        cup_by_id(id)
            .filter(|c| self.owns_cup(c.id))
            .unwrap_or(&CUPS[0])
    }
}
