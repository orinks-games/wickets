//! What the game keeps between visits: the saved colony, the player's
//! settings, and how the runs have gone.

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::Run;
use crate::ID;

/// The best score board.
pub const SCORE_BOARD: &str = "score";
/// The most nights held board.
pub const NIGHTS_BOARD: &str = "nights";
/// The most nights held on the long watch.
pub const LONG_WATCH_BOARD: &str = "long_watch";
/// The best score through the dust storm.
pub const STORM_BOARD: &str = "storm";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    /// A colony left between nights, at the start of its colony phase.
    #[serde(default)]
    pub saved: Option<Run>,
    /// Each raider's step is named as it moves.
    #[serde(default = "yes")]
    pub call_outs: bool,
    /// Enter asks before ending a turn with actions left.
    #[serde(default)]
    pub confirm_end: bool,
    /// Up and Down scan the grid left to right, not nearest first.
    #[serde(default)]
    pub scan_left_to_right: bool,
    /// Colonies started.
    #[serde(default)]
    pub colonies: u32,
    /// Runs that reached Relief.
    #[serde(default)]
    pub reliefs: u32,
    /// Tech points from every colony, which the unlocks open on.
    #[serde(default)]
    pub tech: u32,
    /// Raiders downed in all, and motherships among them.
    #[serde(default)]
    pub downed: u32,
    #[serde(default)]
    pub motherships: u32,
    /// Hard vacuum is on for the next colony.
    #[serde(default)]
    pub hard_vacuum: bool,
    /// The dust storm's wind is heard.
    #[serde(default = "yes")]
    pub storm_wind: bool,
}

fn yes() -> bool {
    true
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            saved: None,
            call_outs: true,
            confirm_end: false,
            scan_left_to_right: false,
            colonies: 0,
            reliefs: 0,
            tech: 0,
            downed: 0,
            motherships: 0,
            hard_vacuum: false,
            storm_wind: true,
        }
    }
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, ID)
    }

    pub fn peek(ctx: &Ctx<Parlor>) -> Progress {
        Records::peek(ctx, ID)
    }

    pub fn save(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, ID, self);
    }

    /// Keeps `run` as the colony to continue.
    pub fn keep(ctx: &mut Ctx<Parlor>, run: &Run) {
        let mut p = Progress::load(ctx);
        p.saved = Some(run.clone());
        p.save(ctx);
    }

    /// Forgets the saved colony.
    pub fn forget(ctx: &mut Ctx<Parlor>) {
        let mut p = Progress::load(ctx);
        p.saved = None;
        p.save(ctx);
    }

    /// Adds what a finished colony brought to the totals.
    pub fn add_run(&mut self, run: &Run, won: bool) {
        self.saved = None;
        if won {
            self.reliefs += 1;
        }
        self.tech += run.tech;
        self.downed += run.downed;
        self.motherships += run.motherships_downed;
    }
}

/// "The long watch: 15 nights held at most. Dust storm: best 820 points.
/// Tech 31 in all." Only the parts there are.
pub fn more_records(ctx: &mut Ctx<Parlor>) -> String {
    let mut parts = Vec::new();
    if let Some(nights) = Records::best(ctx, ID, LONG_WATCH_BOARD) {
        parts.push(format!(
            "The long watch: {} held at most.",
            nights_phrase(nights as u8)
        ));
    }
    if let Some(score) = Records::best(ctx, ID, STORM_BOARD) {
        parts.push(format!("Dust storm: best {score} points."));
    }
    let p = Progress::load(ctx);
    if p.tech > 0 {
        parts.push(format!("Tech {} in all.", p.tech));
    }
    parts.join(" ")
}

/// "Best 1240 points, 12 nights held at most", or "No colony finished yet".
pub fn records_line(ctx: &mut Ctx<Parlor>) -> String {
    let score = Records::best(ctx, ID, SCORE_BOARD);
    let nights = Records::best(ctx, ID, NIGHTS_BOARD);
    match (score, nights) {
        (Some(score), Some(nights)) => format!(
            "Best {score} points. {} held at most.",
            nights_phrase(nights as u8)
        ),
        _ => "No colony finished yet.".to_string(),
    }
}

/// "1 night" or "4 nights".
pub fn nights_phrase(n: u8) -> String {
    if n == 1 {
        "1 night".to_string()
    } else {
        format!("{n} nights")
    }
}
