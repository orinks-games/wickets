//! The star journal: what the player has traced, found and greeted, kept
//! between visits.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Date, Parlor, Period, Records};
use serde::{Deserialize, Serialize};

use crate::almanac::{self, Shower};
use crate::sky::{self, CONSTELLATIONS, WONDERS};
use crate::ID;

/// The records key the journal is saved under.
pub fn key() -> String {
    format!("{ID}/journal")
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    /// Constellation id to the date it was first traced, "2026-10-22".
    #[serde(default)]
    pub logged: BTreeMap<String, String>,
    /// Wonder id to the date it was first found.
    #[serde(default)]
    pub wonders: BTreeMap<String, String>,
    /// Full moon name to the date it was first greeted.
    #[serde(default)]
    pub moons: BTreeMap<String, String>,
    /// Wishes made, by shower and year: "perseids-2026".
    #[serde(default)]
    pub wishes: BTreeMap<String, u32>,
    /// Nights under the stars, heading home or not.
    #[serde(default)]
    pub nights: u32,
    /// Meteor watches.
    #[serde(default)]
    pub watches: u32,
    /// Constellations traced in all, counting each time.
    #[serde(default)]
    pub traces: u32,
    /// The most wishes in one watch.
    #[serde(default)]
    pub best_watch: u32,
}

fn day(date: Date) -> String {
    date.period_key(Period::Daily)
}

/// "22 October 2026" from "2026-10-22".
pub fn date_words(key: &str) -> String {
    match Date::parse(key) {
        Some(d) => format!("{} {} {}", d.day, almanac::month_name(d.month), d.year),
        None => key.to_string(),
    }
}

impl Journal {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Journal {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    pub fn has(&self, con: usize) -> bool {
        self.logged.contains_key(CONSTELLATIONS[con].id)
    }

    /// Writes a traced constellation in. True when it is new.
    pub fn log(&mut self, con: usize, date: Date) -> bool {
        self.traces += 1;
        let id = CONSTELLATIONS[con].id.to_string();
        if self.logged.contains_key(&id) {
            return false;
        }
        self.logged.insert(id, day(date));
        true
    }

    /// Writes a found wonder in. True when it is new.
    pub fn find(&mut self, wonder: usize, date: Date) -> bool {
        let id = WONDERS[wonder].id.to_string();
        if self.wonders.contains_key(&id) {
            return false;
        }
        self.wonders.insert(id, day(date));
        true
    }

    /// Writes a greeted full moon in. True when it is new.
    pub fn greet(&mut self, name: &str, date: Date) -> bool {
        if self.moons.contains_key(name) {
            return false;
        }
        self.moons.insert(name.to_string(), day(date));
        true
    }

    /// Adds a watch's wishes. Returns the shower's total this year.
    pub fn wish(&mut self, wish_key: &str, wishes: u32) -> u32 {
        self.watches += 1;
        self.best_watch = self.best_watch.max(wishes);
        let total = self.wishes.entry(wish_key.to_string()).or_default();
        *total += wishes;
        *total
    }

    pub fn total_wishes(&self) -> u32 {
        self.wishes.values().sum()
    }

    /// The logged constellations' indices, in catalog order.
    pub fn logged_indices(&self) -> Vec<usize> {
        (0..CONSTELLATIONS.len()).filter(|&c| self.has(c)).collect()
    }

    /// "5 of 21 constellations, 2 of 8 wonders, 1 full moon".
    pub fn tally(&self) -> String {
        let moons = match self.moons.len() {
            1 => "1 full moon".to_string(),
            n => format!("{n} full moons"),
        };
        format!(
            "{} of {} constellations, {} of {} wonders, {moons}",
            self.logged.len(),
            CONSTELLATIONS.len(),
            self.wonders.len(),
            WONDERS.len()
        )
    }

    /// The journal as pages: the tally, then one page per constellation
    /// written in, then wonders, moons and wishes.
    pub fn pages(&self) -> Vec<String> {
        let mut pages = vec![format!(
            "Star journal. {}. Each season's sky holds its own constellations, so some are up at another time of year.",
            hearken::text::sentence_start(&self.tally())
        )];
        if self.logged.is_empty() {
            pages.push(
                "No constellations yet. Trace one under the night sky and it is written here, with a little of its lore."
                    .to_string(),
            );
        }
        for c in self.logged_indices() {
            let def = &CONSTELLATIONS[c];
            let when = self
                .logged
                .get(def.id)
                .map(|d| date_words(d))
                .unwrap_or_default();
            pages.push(format!(
                "{}, {}. First traced {when}. {}",
                def.name,
                def.figure,
                def.lore_after_figure()
            ));
        }
        let found: Vec<&sky::WonderDef> = WONDERS
            .iter()
            .filter(|w| self.wonders.contains_key(w.id))
            .collect();
        if !found.is_empty() {
            let names: Vec<String> = found
                .iter()
                .map(|w| hearken::text::sentence_start(w.name))
                .collect();
            pages.push(format!("Wonders found: {}.", names.join(", ")));
        }
        if !self.moons.is_empty() {
            let names: Vec<&str> = almanac::moon_names()
                .into_iter()
                .chain(std::iter::once("the Blue Moon"))
                .filter(|n| self.moons.contains_key(*n))
                .collect();
            pages.push(format!("Full moons greeted: {}.", names.join(", ")));
        }
        if self.total_wishes() > 0 {
            let mut lines = Vec::new();
            for shower in Shower::ALL {
                let n: u32 = self
                    .wishes
                    .iter()
                    .filter(|(k, _)| k.starts_with(&format!("{}-", shower.id())))
                    .map(|(_, v)| *v)
                    .sum();
                if n > 0 {
                    lines.push(format!("{} {n}", shower.name()));
                }
            }
            pages.push(format!(
                "Wishes on shooting stars: {}. {}.",
                self.total_wishes(),
                lines.join(", ")
            ));
        }
        pages
    }
}
