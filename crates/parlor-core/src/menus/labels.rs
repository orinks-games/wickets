//! One word per purpose, on every screen of every game. Games use these
//! constants instead of typing their own, so the labels cannot drift.

use hearken::Ctx;

use crate::{Challenge, Challenges, Parlor, Period};

/// The first word of the main mode's row: "Play, safe 3: the pawnbroker's safe".
pub const PLAY: &str = "Play";
/// The walkthrough, read a page at a time.
pub const TUTORIAL: &str = "Tutorial";
/// The sectioned guide, from help.md.
pub const HELP: &str = "Help";
/// Every sound in the game with its name.
pub const SOUND_CHECK: &str = "Sound check";
/// Bests, progress and streaks.
pub const RECORDS: &str = "Records";
/// The engine's settings screen.
pub const SETTINGS: &str = "Settings";
/// The first word of the challenges row.
pub const CHALLENGES: &str = "Challenges";
/// Leave a sub-menu.
pub const BACK: &str = "Back";
/// Leave a game's title for the launcher's game list.
pub const BACK_TO_PARLOR: &str = "Back to the parlor";
/// Leave results or a run for the game's title.
pub const BACK_TO_TITLE: &str = "Back to the title";
/// Leave results for the challenges menu that opened the run.
pub const BACK_TO_CHALLENGES: &str = "Back to the challenges";
/// Leave a challenge run from the pause screen.
pub const LEAVE_CHALLENGE: &str = "Leave the challenge";
/// Everything a game lets the player earn.
pub const UNLOCKS: &str = "Unlocks";
/// Close the pause screen.
pub const RESUME: &str = "Resume";
/// Leave a run for the launcher's lobby, from the pause screen.
pub const BACK_TO_LOBBY: &str = "Back to the lobby";
/// Close the whole program. It asks first.
pub const QUIT: &str = "Quit Wicket's";
/// Results: the same mode again, after a win.
pub const PLAY_AGAIN: &str = "Play again";
/// Results: the same mode again, after a loss.
pub const TRY_AGAIN: &str = "Try again";
/// Results: the full results, as pages.
pub const HEAR_RESULTS: &str = "Hear the results again";

/// A mode the player has not opened yet: "Time attack, locked".
pub fn locked(name: &str) -> String {
    format!("{name}, locked")
}

/// The title's unlocks row: "Unlocks, 3 of 40".
pub fn unlocks(earned: usize, of: usize) -> String {
    format!("{UNLOCKS}, {earned} of {of}")
}

/// The spoken cue toggle: "Spoken call-outs: on" or "Spoken call-outs: off".
pub fn call_outs(on: bool) -> String {
    format!("Spoken call-outs: {}", if on { "on" } else { "off" })
}

/// Results after a challenge: "Play this daily challenge again".
pub fn challenge_again(period: Period) -> String {
    format!(
        "Play this {} challenge again",
        period.label().to_lowercase()
    )
}

/// The title's challenges row: "Challenges, daily done" or
/// "Challenges, daily not played yet".
pub fn challenges_row_label(ctx: &mut Ctx<Parlor>, id: &'static str) -> String {
    let daily = Challenge::current(ctx, id, Period::Daily);
    if Challenges::done(ctx, &daily) {
        format!("{CHALLENGES}, daily done")
    } else {
        format!("{CHALLENGES}, daily not played yet")
    }
}
