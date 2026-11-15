//! The saved journal: nights paddled, lanterns gathered and the places
//! found, kept in the parlor's records under `lantern-lake/journal`.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::{Feature, Lake, Mode};
use crate::ID;

/// The records key.
pub const KEY: &str = "lantern-lake/journal";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Journal {
    /// The highest night finished.
    #[serde(default)]
    pub nights: u32,
    /// Lanterns gathered on every outing.
    #[serde(default)]
    pub lanterns: u64,
    /// How many outings have ended, by any road home.
    #[serde(default)]
    pub outings: u32,
    /// Each place found, by id, with how many times.
    #[serde(default)]
    pub found: BTreeMap<String, u32>,
    /// Whether the practice pond has been paddled to the end.
    #[serde(default)]
    pub pond_done: bool,
}

impl Journal {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Journal {
        Records::load(ctx, KEY)
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, KEY, self);
    }

    /// The next night to paddle.
    pub fn next_night(&self) -> u32 {
        self.nights + 1
    }

    /// Whether a place is in the journal.
    pub fn has(&self, f: Feature) -> bool {
        self.found.contains_key(f.id())
    }

    /// How many of the places are in the journal.
    pub fn places(&self) -> usize {
        Feature::ALL.iter().filter(|f| self.has(**f)).count()
    }

    /// Free paddle opens once the first night is done.
    pub fn free_open(&self) -> bool {
        self.nights >= 1
    }
}

/// Writes a find into the journal at once. True when it is new.
pub fn record_find(ctx: &mut Ctx<Parlor>, f: Feature) -> bool {
    let mut j = Journal::load(ctx);
    let new = !j.has(f);
    *j.found.entry(f.id().to_string()).or_default() += 1;
    j.store(ctx);
    new
}

/// Saves a finished outing and says what changed.
pub fn save_outing(ctx: &mut Ctx<Parlor>, lake: &Lake) -> Vec<String> {
    let mut j = Journal::load(ctx);
    let mut lines = Vec::new();
    j.outings += 1;
    j.lanterns += lake.gathered as u64;
    match lake.plan.mode {
        Mode::Night(n) if lake.complete && n > j.nights => {
            j.nights = n;
            lines.push(format!(
                "Night {n} is in your journal. Night {} is next.",
                n + 1
            ));
            if n == 1 {
                lines.push("Free paddle is open on the title.".to_string());
            }
        }
        Mode::Pond if lake.complete && !j.pond_done => {
            j.pond_done = true;
            lines.push("You are ready for night 1.".to_string());
        }
        _ => {}
    }
    j.store(ctx);
    if lake.plan.mode != Mode::Pond {
        let board = match lake.plan.mode {
            Mode::Free => "free",
            Mode::Festival => crate::events::FESTIVAL_BOARD,
            _ => "lanterns",
        };
        // An empty outing is no best: nothing goes on the boards.
        if lake.gathered > 0 {
            Records::record(ctx, ID, board, lake.gathered as u64);
        }
        if j.nights > 0 {
            Records::record(ctx, ID, "nights", j.nights as u64);
        }
    }
    lines.push(format!(
        "Your journal: {} of {} places found, {} lantern{} gathered in all.",
        j.places(),
        Feature::ALL.len(),
        j.lanterns,
        if j.lanterns == 1 { "" } else { "s" }
    ));
    lines
}
