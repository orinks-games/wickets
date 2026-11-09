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
    #[serde(default)]
    pub tutorial_seen: bool,
    /// Colonies started.
    #[serde(default)]
    pub colonies: u32,
    /// Runs that reached Relief.
    #[serde(default)]
    pub reliefs: u32,
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
            tutorial_seen: false,
            colonies: 0,
            reliefs: 0,
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
