// Wicket's launcher: the lobby, the category and game lists, and the
// collection's settings, help and about screens.

mod extra;
mod games;
pub mod happening;
mod lobby;
pub mod music;
pub mod soundtrack;
pub mod story;
pub mod today;

use hearken::Ctx;
use parlor_core::Parlor;

pub use games::all as games;
pub use games::story as story_games;
pub use games::{in_season, keepers, listed, specials, specials_listed, story_hall, Keeper};
pub use lobby::{
    after_comma, happenings, happenings_line, last_game, CategoryList, ChallengeBoard,
    ChallengeList, GameList, Lobby, StoryGameList, LAST_GAME_KEY,
};
pub use today::TodayList;

/// The launcher's help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// Read on About.
pub const ABOUT: [&str; 5] = [
    concat!(
        "Wicket's, version ",
        env!("CARGO_PKG_VERSION"),
        ". Games played by ear, for blind and visually impaired players, on the Hearken engine."
    ),
    "Games speak through your screen reader or their own voice, and play with keyboard or controller. Sounds are made with genny.",
    "Built-in voice: genny speech, by Oriol Gomez and blal1, used with permission.",
    "Sound synthesis powered by Klang, by Chris Nash.",
    "Made by Orinks, with the assistance of an agentic coder.",
];

pub fn help(_ctx: &Ctx<Parlor>) -> &'static str {
    HELP
}
