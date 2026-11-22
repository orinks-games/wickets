//! Wicket's band: each run's own song, written by Hearken's composer in a
//! game's style and played from genny notes (see `hearken::band`).
//!
//! The notes are recorded by `tools/band_notes.py` into `sounds/band/`, and
//! the bank that names them is `band/bank.json`. Each game's style sheet is
//! a JSON file under `band/styles/`, named by the style (`neon-rail.daily`
//! is the style `neon-rail/daily`). A game picks its song with a
//! [`SongPlan`] and hands it to [`crate::Score::start_song`].

use std::sync::OnceLock;

pub use hearken::band::{Band, BandState, Bank, Feel, Song, Style};

/// Every style sheet, as written.
const STYLES: &[&str] = &[
    include_str!("styles/basket-run.json"),
    include_str!("styles/harvest-feast.json"),
    include_str!("styles/haunted-hallways.json"),
    include_str!("styles/liberty-lights.json"),
    include_str!("styles/neon-rail.black-sky.json"),
    include_str!("styles/neon-rail.corridor.json"),
    include_str!("styles/neon-rail.daily.json"),
    include_str!("styles/neon-rail.friday.json"),
    include_str!("styles/neon-rail.run.json"),
    include_str!("styles/neon-rail.weekly.json"),
    include_str!("styles/safecracker.json"),
    include_str!("styles/sonar-dive.json"),
    include_str!("styles/tide-pools.json"),
    include_str!("styles/tightrope.json"),
    include_str!("styles/winter-wonderland.json"),
];

/// The band's recorded notes and hits.
pub fn bank() -> &'static Bank {
    static BANK: OnceLock<Bank> = OnceLock::new();
    BANK.get_or_init(|| serde_json::from_str(include_str!("bank.json")).expect("band/bank.json reads"))
}

/// Every style sheet.
pub fn styles() -> &'static [Style] {
    static ALL: OnceLock<Vec<Style>> = OnceLock::new();
    ALL.get_or_init(|| {
        STYLES
            .iter()
            .map(|s| serde_json::from_str(s).expect("a band style reads"))
            .collect()
    })
}

/// The style named `name`, such as `"neon-rail/run"`.
pub fn style(name: &str) -> Option<&'static Style> {
    styles().iter().find(|s| s.name == name)
}

/// Where a song's seed comes from, which decides when it is new.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seed {
    /// New every run. One run in [`FIXED_ONE_IN`] plays the game's own
    /// written score instead, so it is still heard.
    Fresh(u64),
    /// Tied to a day, a week or a month: everyone hears the same song, and
    /// a retry hears it again.
    Dated(u64),
}

/// One run in this many with a fresh seed plays the written score.
pub const FIXED_ONE_IN: u64 = 10;

impl Seed {
    /// A new seed for a new run.
    pub fn fresh() -> Seed {
        Seed::Fresh(crate::Rng::from_time().next_u64())
    }

    /// The seed for a dated challenge: `key` names the game, the mode and
    /// its day, week or month, such as `"neon-rail/daily/2026-11-22"`.
    pub fn dated(key: &str) -> Seed {
        Seed::Dated(hearken::band::rng::hash(key))
    }

    pub fn value(self) -> u64 {
        match self {
            Seed::Fresh(s) | Seed::Dated(s) => s,
        }
    }
}

/// A run's song: which style, from which seed, starting at which section,
/// and at what speed on top of the song's own tempo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SongPlan {
    pub style: &'static str,
    pub seed: Seed,
    /// The section to start at: a run that comes back between sectors
    /// carries on its song from the next one.
    pub section: usize,
    pub speed: f32,
}

impl SongPlan {
    pub fn new(style: &'static str, seed: Seed) -> SongPlan {
        SongPlan {
            style,
            seed,
            section: 0,
            speed: 1.0,
        }
    }

    pub fn section(mut self, section: usize) -> SongPlan {
        self.section = section;
        self
    }

    pub fn speed(mut self, speed: f32) -> SongPlan {
        self.speed = speed;
        self
    }

    /// Whether this plan plays the game's written score instead of a song:
    /// one fresh run in [`FIXED_ONE_IN`], decided by the seed so a run
    /// that comes back between sectors stays with what it drew.
    pub fn plays_written_score(&self) -> bool {
        matches!(self.seed, Seed::Fresh(s) if s % FIXED_ONE_IN == 0)
    }

    /// The song this plan writes, or `None` when its style is unknown.
    pub fn song(&self) -> Option<Song> {
        let style = style(self.style)?;
        Some(Song::compose(style, bank(), self.seed.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_reads_and_names_only_what_the_bank_has() {
        assert!(!styles().is_empty());
        for s in styles() {
            let problems = s.problems(bank());
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn every_note_the_bank_names_is_on_disk() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
        let missing: Vec<String> = bank()
            .keys()
            .into_iter()
            .filter(|k| !root.join(format!("{k}.ogg")).exists())
            .collect();
        assert!(missing.is_empty(), "{} missing, first {:?}", missing.len(), missing.first());
    }

    #[test]
    fn a_dated_seed_is_the_same_for_everyone_and_a_fresh_one_is_not() {
        assert_eq!(Seed::dated("neon-rail/daily/2026-11-22"), Seed::dated("neon-rail/daily/2026-11-22"));
        assert_ne!(Seed::dated("neon-rail/daily/2026-11-22"), Seed::dated("neon-rail/daily/2026-11-23"));
        let plan = SongPlan::new("neon-rail/run", Seed::Fresh(30));
        assert!(plan.plays_written_score());
        assert!(!SongPlan::new("neon-rail/run", Seed::Dated(30)).plays_written_score());
    }
}
