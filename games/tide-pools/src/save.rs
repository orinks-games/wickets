//! The home aquarium: everything ever gathered, saved in the records.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::{Place, Species};
use crate::ID;

fn key() -> String {
    format!("{ID}/aquarium")
}

/// The saved collection.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Aquarium {
    /// Species id to how many are in the tank.
    #[serde(default)]
    pub counts: BTreeMap<String, u32>,
    /// Species id to where and when the first one was found.
    #[serde(default)]
    pub first: BTreeMap<String, String>,
    /// Finished outings.
    #[serde(default)]
    pub outings: u32,
}

impl Aquarium {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Aquarium {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    pub fn count(&self, s: Species) -> u32 {
        self.counts.get(s.id()).copied().unwrap_or(0)
    }

    pub fn has(&self, s: Species) -> bool {
        self.count(s) > 0
    }

    /// Kinds in the tank, in the game's order.
    pub fn species(&self) -> Vec<Species> {
        Species::ALL.into_iter().filter(|s| self.has(*s)).collect()
    }

    pub fn kinds(&self) -> usize {
        self.species().len()
    }

    pub fn creatures(&self) -> u32 {
        Species::ALL.into_iter().map(|s| self.count(s)).sum()
    }

    pub fn missing(&self) -> Vec<Species> {
        Species::ALL.into_iter().filter(|s| !self.has(*s)).collect()
    }

    /// Whether a place is open.
    pub fn open(&self, place: Place) -> bool {
        self.kinds() >= place.unlock_kinds()
    }

    /// Adds a trip's finds; returns the kinds that are new to the tank.
    /// `found` describes the trip, for the first find's note. Only a
    /// finished trip adds to `outings`, in the outing's own finish.
    pub fn add(&mut self, finds: &[Species], found: &str) -> Vec<Species> {
        let mut new = Vec::new();
        for s in finds {
            if !self.has(*s) && !new.contains(s) {
                new.push(*s);
                self.first.insert(s.id().to_string(), found.to_string());
            }
            *self.counts.entry(s.id().to_string()).or_default() += 1;
        }
        new
    }

    /// "First found at Sandy Cove on a sunny morning."
    pub fn first_note(&self, s: Species) -> Option<String> {
        self.first.get(s.id()).map(|w| format!("First found {w}."))
    }
}
