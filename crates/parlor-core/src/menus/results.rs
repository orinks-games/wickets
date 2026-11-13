//! The menu at the end of every run:
//!
//! ```text
//! {one-line summary}
//!   Next {unit}: {name}          optional, first
//!   Play again | Try again | Play this daily challenge again
//!   Hear the results again
//!   {game extras}
//!   Back to the title | Back to the challenges
//! ```
//!
//! Escape does exactly what the last row does. Status and Repeat say the
//! summary again, unless a row starts with that key's letter, which keeps
//! its jump.

use std::cell::RefCell;
use std::rc::Rc;

use hearken::menu::{menu_input, Menu, MenuCore, MenuItem};
use hearken::text::end_sentence;
use hearken::{Ctx, InputEvent, Key, UiSound};

use super::labels;
use crate::{Pages, Parlor, ParlorAction};

type Action = Rc<RefCell<dyn FnMut(&mut Ctx<Parlor>)>>;

/// Where the last row and Escape go, by the `State::name` they pop back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    /// "Back to the title", to the game's title.
    Title(&'static str),
    /// "Back to the challenges", to the challenges menu that opened the
    /// run, or what stands in for it when the launcher opened it.
    Challenges(&'static str),
}

impl Back {
    pub fn label(self) -> &'static str {
        match self {
            Back::Title(_) => labels::BACK_TO_TITLE,
            Back::Challenges(_) => labels::BACK_TO_CHALLENGES,
        }
    }

    fn target(self) -> &'static str {
        match self {
            Back::Title(name) | Back::Challenges(name) => name,
        }
    }

    fn go(self, ctx: &mut Ctx<Parlor>) {
        ctx.play_ui(UiSound::Back);
        ctx.pop_to(self.target());
    }
}

struct Row {
    label: String,
    help: String,
    action: Action,
}

impl Row {
    fn new(
        label: impl Into<String>,
        help: impl Into<String>,
        action: impl FnMut(&mut Ctx<Parlor>) + 'static,
    ) -> Row {
        Row {
            label: label.into(),
            help: help.into(),
            action: Rc::new(RefCell::new(action)),
        }
    }

    fn item(&self) -> MenuItem<Results, Parlor> {
        let action = Rc::clone(&self.action);
        MenuItem::new(
            self.label.as_str(),
            move |_s: &mut Results, ctx: &mut Ctx<Parlor>| (action.borrow_mut())(ctx),
        )
        .help(self.help.as_str())
    }
}

/// The shared results menu. Build it with `Results::new`, add rows, and
/// push it (or `replace` the run with it).
pub struct Results {
    menu: MenuCore<Self, Parlor>,
    pages: Vec<String>,
    next: Option<Row>,
    again: Option<Row>,
    extras: Vec<Row>,
    back: Back,
}

impl Results {
    /// `summary` is the menu's title, spoken on entry; `pages` are the full
    /// results, which "Hear the results again" reads. `back` names where
    /// the last row and Escape go.
    pub fn new<S: Into<String>>(
        summary: impl Into<String>,
        pages: impl IntoIterator<Item = S>,
        back: Back,
    ) -> Results {
        Results {
            menu: MenuCore::new(summary).with_intro_help(format!(
                "Up and Down choose, Enter goes, Escape goes {}.",
                back.label().trim_start_matches("Back ")
            )),
            pages: pages.into_iter().map(Into::into).collect(),
            next: None,
            again: None,
            extras: Vec::new(),
            back,
        }
    }

    /// "Next safe: safe 4, the jeweller's safe", first, when the player
    /// cleared this one.
    pub fn next(
        mut self,
        label: impl Into<String>,
        help: impl Into<String>,
        action: impl FnMut(&mut Ctx<Parlor>) + 'static,
    ) -> Results {
        self.next = Some(Row::new(label, help, action));
        self
    }

    /// The same run again: `labels::PLAY_AGAIN`, `labels::TRY_AGAIN` or
    /// `labels::challenge_again(period)`.
    pub fn again(
        mut self,
        label: impl Into<String>,
        help: impl Into<String>,
        action: impl FnMut(&mut Ctx<Parlor>) + 'static,
    ) -> Results {
        self.again = Some(Row::new(label, help, action));
        self
    }

    /// A game's own row, after "Hear the results again".
    pub fn extra(
        mut self,
        label: impl Into<String>,
        help: impl Into<String>,
        action: impl FnMut(&mut Ctx<Parlor>) + 'static,
    ) -> Results {
        self.extras.push(Row::new(label, help, action));
        self
    }
}

impl Results {
    /// Whether a row starts with the letter `event`'s key types.
    fn has_row_for(&self, ctx: &Ctx<Parlor>, event: &InputEvent) -> bool {
        let Some(key) = event.pressed() else {
            return false;
        };
        self.menu.items.iter().any(|item| {
            item.text(self, ctx)
                .chars()
                .next()
                .is_some_and(|c| Key::from_char(c.to_ascii_lowercase()) == key)
        })
    }
}

impl Menu for Results {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "results"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    /// A run that reaches its results counts as played, for story mode.
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Back::Title(name) = self.back {
            crate::tutorial::mark_played(ctx, &name.replace('_', "-"));
        }
        hearken::menu::menu_enter(self, ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        if let Some(next) = &self.next {
            items.push(next.item());
        }
        if let Some(again) = &self.again {
            items.push(again.item());
        }
        items.push(
            MenuItem::new(
                labels::HEAR_RESULTS,
                |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let pages = if s.pages.is_empty() {
                        vec![end_sentence(&s.menu.title)]
                    } else {
                        s.pages.clone()
                    };
                    ctx.push(Pages::new(pages))
                },
            )
            .help("The full results again, a page at a time."),
        );
        items.extend(self.extras.iter().map(Row::item));
        if let Some(kept) = crate::songs::fresh_for(ctx, self.back.target()) {
            items.push(
                MenuItem::new(
                    crate::songs::SAVE_ROW,
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| crate::songs::save(ctx, &kept),
                )
                .help("Writes this run's music to a sound file. It is kept in the Soundtrack either way."),
            );
        }
        let back = self.back;
        let help = match back {
            Back::Title(_) => "Return to the game's title.",
            Back::Challenges(_) => "Return to the challenges.",
        };
        items.push(
            MenuItem::new(back.label(), move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                back.go(ctx)
            })
            .select_sound(None)
            .help(help),
        );
        items
    }

    /// Status, or Repeat, says the summary again. A row whose first letter
    /// is the key pressed keeps its first-letter jump.
    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        let action = ctx.action_for(event);
        if matches!(action, Some(ParlorAction::Status | ParlorAction::Repeat))
            && !self.has_row_for(ctx, event)
        {
            ctx.say(end_sentence(&self.menu.title));
            return;
        }
        menu_input(self, ctx, event);
    }

    /// Escape does what the last row does.
    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        self.back.go(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::{rows, Base};
    use hearken::{Headless, Key, State};

    struct Run;
    impl State<Parlor> for Run {
        fn name(&self) -> &'static str {
            "run"
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    fn shown(results: Results) -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Run);
        h.app.ctx.push(results);
        h.frame(0.0);
        h
    }

    fn full() -> Results {
        Results::new(
            "Safe 3 open. 640 points",
            ["Safe 3 open. 640 points.", "Strikes: 0."],
            Back::Title("base"),
        )
        .extra("Compare", "Hear both.", |ctx| ctx.say("Comparing."))
        .again(labels::PLAY_AGAIN, "The same safe.", |ctx| {
            ctx.say("Again.")
        })
        .next("Next safe: safe 4", "Four wheels.", |ctx| ctx.say("Next."))
    }

    #[test]
    fn rows_come_in_the_standard_order() {
        let h = shown(full());
        assert_eq!(h.app.top_name(), "results");
        assert_eq!(
            rows(&h),
            [
                "Next safe: safe 4",
                "Play again",
                "Hear the results again",
                "Compare",
                "Back to the title",
            ]
        );
        assert_eq!(
            h.spoken(),
            ["Safe 3 open. 640 points.", "Next safe: safe 4. 1 of 5."]
        );
        let h = shown(
            Results::new("Lost", ["Lost."], Back::Challenges("base")).again(
                labels::TRY_AGAIN,
                "Again.",
                |_ctx| {},
            ),
        );
        assert_eq!(
            rows(&h),
            [
                "Try again",
                "Hear the results again",
                "Back to the challenges"
            ]
        );
    }

    #[test]
    fn a_run_that_kept_its_song_can_save_it_from_the_results() {
        use hearken::Bus;
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Run);
        h.frame(0.0);
        // The run keeps its song, then opens its results.
        crate::songs::begin(&mut h.app.ctx);
        let s = h
            .app
            .ctx
            .audio
            .start_loop("run/score/base", None, Bus::MUSIC);
        h.frame(20.0);
        h.app.ctx.audio.stop(s, 0);
        assert!(crate::songs::end(&mut h.app.ctx, "run").is_some());
        h.app.ctx.push(Results::new("Done", ["Done."], Back::Title("run")));
        h.frame(0.0);
        assert_eq!(
            rows(&h),
            [
                "Hear the results again",
                "Make a sound file of this run's song",
                "Back to the title"
            ]
        );
        // Another game's results do not offer it.
        h.app.ctx.pop();
        h.frame(0.0);
        h.app
            .ctx
            .push(Results::new("Done", ["Done."], Back::Title("base")));
        h.frame(0.0);
        assert_eq!(rows(&h), ["Hear the results again", "Back to the title"]);
    }

    #[test]
    fn status_repeats_the_summary() {
        let mut h = shown(full());
        h.clear();
        h.press(Key::S);
        assert_eq!(h.spoken(), ["Safe 3 open. 640 points."]);
        assert_eq!(h.app.top_name(), "results");
    }

    #[test]
    fn repeat_says_the_summary_too() {
        let mut h = shown(full());
        h.clear();
        h.press(Key::R);
        assert_eq!(h.spoken(), ["Safe 3 open. 640 points."]);
        assert_eq!(h.app.top_name(), "results");
    }

    #[test]
    fn a_row_starting_with_s_keeps_its_jump() {
        let mut h = shown(full().extra("Shop", "Spend.", |_ctx| {}));
        h.clear();
        h.press(Key::S);
        assert_eq!(h.spoken(), ["Shop. 5 of 6."]);
    }

    #[test]
    fn escape_does_what_back_does() {
        let mut h = shown(full());
        h.press(Key::Escape);
        let by_escape = (h.app.top_name(), h.app.depth());
        let mut h = shown(full());
        h.press(Key::End);
        h.press(Key::Return);
        assert_eq!((h.app.top_name(), h.app.depth()), by_escape);
        assert_eq!(by_escape, ("base", 1));
    }

    #[test]
    fn rows_run_their_actions_more_than_once() {
        let mut h = shown(full());
        h.press(Key::Down);
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Again."]);
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Again."]);
        h.press(Key::Down);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "pages");
        h.press(Key::Return);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "results");
    }
}
