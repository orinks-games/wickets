//! The menus every game shares, so no game's menus drift from the others:
//!
//! - [`labels`]: one word per purpose, on every screen.
//! - [`Title`] and [`TitleGame`]: the title screen in its fixed order.
//! - [`settings`]: a game's own rows on the shared Settings screen.
//! - [`ChallengesMenu`] and [`ChallengeGame`]: the daily, weekly and
//!   monthly challenge, then Back.
//! - [`pause`] and [`Leave`]: the pause screen, with an honest leave row
//!   and a Quit that asks first.
//! - [`Results`] and [`Back`]: the end of a run, with Play again and a Back
//!   row that Escape matches.
//!
//! `Pages::briefing` is the read-then-play pages whose Escape backs out.

mod challenges;
pub mod labels;
mod pause;
mod results;
pub mod settings;
mod title;

pub use challenges::{record_words, row_label, ChallengeGame, ChallengesMenu};
pub use pause::{
    pause, quit_confirm, Leave, Pause, LOBBY_NAME, NO_KEEP_PLAYING, QUIT_QUESTION, YES_QUIT,
};
pub use results::{Back, Results};
pub use title::{mark_tutorial_read, tutorial_read, FirstCursor, Title, TitleGame, TitleRow};

/// A menu's rows from its `top_lines()`: the title line dropped and the
/// `"> "` or `"  "` cursor marker stripped from each row.
pub fn rows_of(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .skip(1)
        .map(|l| {
            l.strip_prefix("> ")
                .or_else(|| l.strip_prefix("  "))
                .unwrap_or(l)
                .to_string()
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod testing {
    use hearken::{Ctx, Headless, State};

    use crate::Parlor;

    /// Stands in for the launcher beneath the screen under test.
    pub struct Base;

    impl State<Parlor> for Base {
        fn name(&self) -> &'static str {
            "base"
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    pub fn rows(h: &Headless<Parlor>) -> Vec<String> {
        super::rows_of(&h.app.top_lines())
    }
}
