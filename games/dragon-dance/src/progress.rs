//! What the game remembers between visits: stars on each street, the
//! longest dragon, the keepsakes of the years' animals, the lanterns found
//! and the first-time explanations already heard.

use std::collections::{BTreeMap, BTreeSet};

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::festival::{self, ANIMALS};
use crate::streets::STREETS;
use crate::ID;

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// The best stars on each street of the main parade, street 1 first.
    #[serde(default)]
    pub stars: Vec<u8>,
    /// Streets danced to the end, of any kind.
    #[serde(default)]
    pub danced: u32,
    #[serde(default)]
    pub longest: u32,
    #[serde(default)]
    pub envelopes: u32,
    /// Each animal's keepsake, with the year it was first earned.
    #[serde(default)]
    pub keepsakes: BTreeMap<String, i32>,
    /// The lunar years whose New Year parade was danced.
    #[serde(default)]
    pub parades: BTreeSet<i32>,
    /// The lanterns found on Lantern walks, by number.
    #[serde(default)]
    pub lanterns: BTreeSet<usize>,
    /// The years whose Lantern walk was walked.
    #[serde(default)]
    pub walks: BTreeSet<i32>,
    /// First-time explanations already given.
    #[serde(default)]
    pub taught: BTreeSet<String>,
    /// Spoken call-outs turned off.
    #[serde(default)]
    pub quiet: bool,
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    /// The progress as last read, for a Settings row.
    pub fn peek(ctx: &Ctx<Parlor>) -> Progress {
        Records::peek(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Street `n`'s best stars, counting from 1.
    pub fn stars_on(&self, n: usize) -> u8 {
        n.checked_sub(1)
            .and_then(|i| self.stars.get(i))
            .copied()
            .unwrap_or(0)
    }

    /// Whether street `n` (from 1) has been danced to the end.
    pub fn played(&self, n: usize) -> bool {
        n >= 1 && self.stars.len() >= n
    }

    /// Street 1 is always open; each street after opens with a star on
    /// the one before.
    pub fn opened(&self, n: usize) -> bool {
        n <= 1 || self.stars_on(n - 1) >= 1
    }

    /// The street Play goes to: the first open street without a star, or
    /// the last street once every one has a star.
    pub fn next_street(&self) -> usize {
        (1..=STREETS.len())
            .find(|&n| self.opened(n) && self.stars_on(n) == 0)
            .unwrap_or(STREETS.len())
    }

    /// Saves a street's stars, keeping the better. Returns the old best.
    pub fn set_stars(&mut self, n: usize, stars: u8) -> u8 {
        if self.stars.len() < n {
            self.stars.resize(n, 0);
        }
        let old = self.stars[n - 1];
        self.stars[n - 1] = old.max(stars);
        old
    }

    pub fn total_stars(&self) -> u32 {
        self.stars.iter().map(|&s| s as u32).sum()
    }

    /// Saves `animal`'s keepsake for `year`. True when it is new.
    pub fn add_keepsake(&mut self, animal: &str, year: i32) -> bool {
        if self.keepsakes.contains_key(animal) {
            return false;
        }
        self.keepsakes.insert(animal.to_string(), year);
        true
    }

    /// Whether anything has been played, so the title need not start on
    /// the tutorial.
    pub fn any_played(&self) -> bool {
        self.danced > 0
    }

    /// Marks an explanation heard. True the first time.
    pub fn teach(&mut self, what: &str) -> bool {
        self.taught.insert(what.to_string())
    }

    /// The animals still to collect, in zodiac order.
    pub fn missing_animals(&self) -> Vec<&'static str> {
        ANIMALS
            .into_iter()
            .filter(|a| !self.keepsakes.contains_key(*a))
            .collect()
    }

    /// The keepsakes pages: the animals' keepsakes, then the lanterns.
    pub fn keepsake_pages(&self) -> Vec<String> {
        let mut first = match self.keepsakes.len() {
            0 => "Keepsakes, none yet. Dance the New Year parade on Lunar New Year to earn the year's animal keepsake, one for each of the twelve animals.".to_string(),
            n => format!("Keepsakes, {n} of 12."),
        };
        for animal in ANIMALS {
            if let Some(year) = self.keepsakes.get(animal) {
                first.push_str(&format!(
                    " {}, {year}: {}.",
                    sentence_start(&festival::year_of(animal)),
                    festival::keepsake(animal)
                ));
            }
        }
        let mut pages = vec![first];
        let missing = self.missing_animals();
        if !self.keepsakes.is_empty() && !missing.is_empty() {
            pages.push(format!(
                "Still to come: {}. Each Lunar New Year brings the next animal.",
                missing.join(", ")
            ));
        }
        let lanterns: Vec<&str> = self
            .lanterns
            .iter()
            .map(|&k| festival::lantern_name(k))
            .collect();
        pages.push(match lanterns.len() {
            0 => "Lanterns, none yet. The Lantern walk opens on the Lantern Festival, 14 days after Lunar New Year.".to_string(),
            n => format!(
                "Lanterns, {n} of {}: {}.",
                festival::LANTERNS.len(),
                lanterns.join(", ")
            ),
        });
        pages
    }
}
