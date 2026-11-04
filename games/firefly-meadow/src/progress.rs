//! What the game remembers between visits: the journal of species and
//! chords, fireflies caught, midsummer keepsakes, the
//! meadow chosen on the title, and the longest evening.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::meadow::Place;
use crate::species::{Chord, Species};
use crate::ID;

/// The records key the journal is saved under.
pub fn key() -> String {
    format!("{ID}/journal")
}

/// The board the longest evening is kept on, in seconds.
pub const LONGEST: &str = "longest_evening";
/// The board the best midsummer harmony is kept on, in percent.
pub const MIDSUMMER: &str = "midsummer";

/// A midsummer night's keepsake.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keepsake {
    pub name: String,
    /// The best percent in step that year.
    pub harmony: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    /// Fireflies caught in all, practice aside.
    #[serde(default)]
    pub caught: u32,
    /// Each species found, by its slug, with how many were caught.
    #[serde(default)]
    pub species: BTreeMap<String, u32>,
    /// Each named chord made, by its slug, with how many times.
    #[serde(default)]
    pub chords: BTreeMap<String, u32>,
    #[serde(default)]
    pub evenings: u32,
    #[serde(default)]
    pub releases: u32,
    /// Midsummer keepsakes, by year.
    #[serde(default)]
    pub keepsakes: BTreeMap<i32, Keepsake>,
    /// The meadow the title's Meadow row is on, by slug.
    #[serde(default)]
    pub meadow: Option<String>,
}

impl Journal {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Journal {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    pub fn found(&self, species: Species) -> bool {
        self.species.contains_key(species.slug())
    }

    pub fn count(&self, species: Species) -> u32 {
        self.species.get(species.slug()).copied().unwrap_or(0)
    }

    /// Species found, of the ten.
    pub fn species_found(&self) -> usize {
        Species::ALL.iter().filter(|&&s| self.found(s)).count()
    }

    /// The species found, in journal order, the chorus aside.
    pub fn jar_species(&self) -> Vec<Species> {
        Species::JAR
            .into_iter()
            .filter(|&s| self.found(s))
            .collect()
    }

    pub fn has_chord(&self, chord: Chord) -> bool {
        self.chords.contains_key(chord.slug())
    }

    pub fn chords_found(&self) -> usize {
        Chord::ALL.iter().filter(|&&c| self.has_chord(c)).count()
    }

    pub fn unlocked(&self, place: Place) -> bool {
        self.caught >= place.opens_at()
    }

    /// The meadow the title's Meadow row is on: the one chosen, when open.
    pub fn meadow(&self) -> Place {
        self.meadow
            .as_deref()
            .and_then(|slug| Place::WALKS.into_iter().find(|p| p.slug() == slug))
            .filter(|&p| self.unlocked(p))
            .unwrap_or(Place::Home)
    }

    /// The next open meadow after the chosen one, round to the first.
    pub fn next_meadow(&self) -> Place {
        let open: Vec<Place> = Place::WALKS
            .into_iter()
            .filter(|&p| self.unlocked(p))
            .collect();
        let at = open.iter().position(|&p| p == self.meadow()).unwrap_or(0);
        open[(at + 1) % open.len()]
    }

    /// The meadows that open with fireflies caught, still closed.
    pub fn locked(&self) -> Vec<Place> {
        Place::WALKS
            .into_iter()
            .filter(|&p| !self.unlocked(p))
            .collect()
    }

    pub fn any_played(&self) -> bool {
        self.evenings > 0 || self.caught > 0
    }
}

/// Notes a catch in the journal. Returns whether the species is new.
pub fn note_catch(ctx: &mut Ctx<Parlor>, species: Species) -> bool {
    let mut journal = Journal::load(ctx);
    let new = !journal.found(species);
    *journal
        .species
        .entry(species.slug().to_string())
        .or_default() += 1;
    if species != Species::MidsummerChorus {
        journal.caught += 1;
    }
    journal.store(ctx);
    new
}

/// Notes a released jar, and its chord. Returns whether the chord is new.
pub fn note_release(ctx: &mut Ctx<Parlor>, chord: Option<Chord>) -> bool {
    let mut journal = Journal::load(ctx);
    journal.releases += 1;
    let new = chord.is_some_and(|c| !journal.has_chord(c));
    if let Some(c) = chord {
        *journal.chords.entry(c.slug().to_string()).or_default() += 1;
    }
    journal.store(ctx);
    new
}

/// Notes a finished evening, and its length on the longest board.
pub fn note_evening(ctx: &mut Ctx<Parlor>, seconds: f64) -> ScoreResult {
    let mut journal = Journal::load(ctx);
    journal.evenings += 1;
    journal.store(ctx);
    Records::record(ctx, ID, LONGEST, seconds.round() as u64)
}

/// What finishing a midsummer night changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSake {
    pub name: String,
    /// The first time this year's keepsake was taken home.
    pub new: bool,
    /// The year's best harmony before tonight.
    pub previous: Option<u32>,
}

/// Saves a midsummer night: the year's keepsake and its best harmony.
pub fn note_keepsake(ctx: &mut Ctx<Parlor>, year: i32, name: &str, harmony: u32) -> KeptSake {
    let mut journal = Journal::load(ctx);
    let entry = journal.keepsakes.get(&year).cloned();
    let previous = entry.as_ref().map(|k| k.harmony);
    journal.keepsakes.insert(
        year,
        Keepsake {
            name: name.to_string(),
            harmony: previous.unwrap_or(0).max(harmony),
        },
    );
    journal.store(ctx);
    Records::record(ctx, ID, MIDSUMMER, harmony as u64);
    KeptSake {
        name: name.to_string(),
        new: entry.is_none(),
        previous,
    }
}

/// Chooses the meadow the title's Play row walks.
pub fn choose_meadow(ctx: &mut Ctx<Parlor>, place: Place) {
    let mut journal = Journal::load(ctx);
    journal.meadow = Some(place.slug().to_string());
    journal.store(ctx);
}
