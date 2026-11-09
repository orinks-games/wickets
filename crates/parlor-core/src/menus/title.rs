//! Every game's title screen, in one fixed order.
//!
//! The game implements [`TitleGame`] and opens `Title::new(game)`. The
//! builder supplies the required rows itself, with the shared labels and
//! F1 help, in three groups:
//!
//! - play: the game's `play_rows` (Play first), Challenges, its practice row;
//! - learn: Tutorial, Help, Sound check;
//! - yours: Records, the game's extras, Settings, its destructive rows,
//!   Back to the parlor.
//!
//! A game's own options, such as spoken call-outs, never sit on the title:
//! they go in [`TitleGame::settings`], after the shared rows of Settings.
//!
//! Rows never move or appear: a locked mode is a ", locked" row in its
//! place. On a first visit the cursor starts on Tutorial (or the practice
//! row) instead of Play, and the title says so. Tutorial opens the game's
//! interactive [`Lesson`]; once it is done the cursor goes to Play.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::screens::{SettingsRow, SettingsScreen};
use hearken::text::end_sentence;
use hearken::{Ctx, UiSound};

use super::labels;
use crate::tutorial::{self, Lesson};
use crate::{Challenge, Pages, Parlor, Period, TitleOpening};

/// A row a game adds to its title. Its closures get the whole `Title`, so
/// they reach the game's own state through `s.game`.
pub type TitleRow<G> = MenuItem<Title<G>, Parlor>;

/// Where the cursor starts on a first visit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirstCursor {
    Tutorial,
    /// The game's practice row, for a game whose practice is the right start.
    Practice,
}

/// What a game gives its title. Only the play rows, the tutorial, the
/// records, the sound check and the challenges are required; the rest have
/// defaults.
pub trait TitleGame: Sized + 'static {
    /// The game's id, as in `GameInfo::id`.
    const ID: &'static str;
    /// The game's name, which the title says first.
    const NAME: &'static str;
    /// The game's help.md, for the Help row.
    const HELP: &'static str;
    /// The title's `State::name`, which runs return to.
    const TITLE_NAME: &'static str;

    /// Group 1 before Challenges: the Play row first, then other modes and
    /// any "{Unit} select" row. Every row needs `.help(..)`.
    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>>;
    /// The game's interactive tutorial, usually the same function as
    /// `GameInfo::tutorial`.
    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> Lesson;
    /// The Records pages. The first page starts "Records.".
    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String>;
    /// Pushes the game's sound check.
    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>);
    /// Pushes the game's challenges menu, usually a `ChallengesMenu`.
    fn challenges(&mut self, ctx: &mut Ctx<Parlor>);

    /// Each frame while the title is on top, for a game that times
    /// something there, such as a voice that waits for the title to be
    /// said.
    fn update(&mut self, _ctx: &mut Ctx<Parlor>, _dt: f64) {}

    /// Starts the title's music. Called on entering and on every return.
    fn music(&mut self, _ctx: &mut Ctx<Parlor>) {}
    /// Stops it, when the player leaves for the launcher.
    fn stop_music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
    }
    /// Group 1 after Challenges: one "Practice {thing}" row.
    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        None
    }
    /// Group 3 after Records: collections such as a journal or a workshop.
    fn extras(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        Vec::new()
    }
    /// The game's own options, such as `labels::call_outs`, as rows on the
    /// Settings screen after the shared ones, while its title is open: from
    /// the title's Settings row or a run's pause menu. Build them with
    /// [`settings::toggle`](super::settings::toggle) or
    /// [`settings::setting`](super::settings::setting). A run that keeps a
    /// setting reads it again in `resume`, since the pause menu may change it.
    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        Vec::new()
    }
    /// Group 3 after Settings: rows that lose progress, behind a ConfirmScreen.
    fn destructive(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        Vec::new()
    }

    /// F1 on the Challenges row.
    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, Self::ID, Period::Daily);
        format!(
            "A daily, a weekly and a monthly challenge, the same for every player. {}",
            daily.resets()
        )
    }
    /// F1 on the Records row.
    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your bests and progress, with your challenge results.".to_string()
    }
    /// F1 on the Sound check row.
    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Choose and play the sounds you need to recognise during the game.".to_string()
    }

    /// Whether this is the player's first visit, which starts the cursor on
    /// Tutorial. By default: until the tutorial is done or the game played.
    /// A game whose own saves show play says so here, and the title then
    /// marks the game played for story mode.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        tutorial::needs_lesson(ctx, Self::ID)
    }
    /// Where a first visit starts the cursor.
    fn first_cursor(&self) -> FirstCursor {
        FirstCursor::Tutorial
    }
    /// F1 on a row with no help, and on the title itself.
    fn intro_help(&self) -> Option<String> {
        None
    }
}

/// Whether the player has finished this game's tutorial (or read the old
/// paged one). The same as `tutorial::tutorial_done`.
pub fn tutorial_read(ctx: &mut Ctx<Parlor>, id: &str) -> bool {
    tutorial::tutorial_done(ctx, id)
}

/// Notes that the player has finished this game's tutorial.
pub fn mark_tutorial_read(ctx: &mut Ctx<Parlor>, id: &str) {
    tutorial::mark_tutorial_done(ctx, id);
}

/// A game's title screen. See the module docs.
pub struct Title<G: TitleGame> {
    /// The game's own state, for its rows' closures.
    pub game: G,
    menu: MenuCore<Self, Parlor>,
    tutorial_at: usize,
    practice_at: Option<usize>,
    /// The tutorial is open above, and was not done when it opened.
    teaching: bool,
}

impl<G: TitleGame> Title<G> {
    pub fn new(game: G) -> Self {
        let mut menu = MenuCore::new(G::NAME);
        if let Some(help) = game.intro_help() {
            menu = menu.with_intro_help(help);
        }
        Title {
            game,
            menu,
            tutorial_at: 0,
            practice_at: None,
            teaching: false,
        }
    }

    /// Where the Tutorial row is, counting from 0.
    pub fn tutorial_index(&self) -> usize {
        self.tutorial_at
    }

    /// Back from the tutorial: once it is done, the cursor goes to Play.
    /// The first row whose label starts with `start`, ignoring case.
    fn row_starting(&self, ctx: &Ctx<Parlor>, start: &str) -> Option<usize> {
        let start = start.to_lowercase();
        self.menu
            .items
            .iter()
            .position(|item| item.text(self, ctx).to_lowercase().starts_with(&start))
    }

    fn after_lesson(&mut self, ctx: &mut Ctx<Parlor>) {
        if std::mem::take(&mut self.teaching) && tutorial::tutorial_done(ctx, G::ID) {
            self.menu.index = 0;
        }
    }

    fn row(
        label: &str,
        help: impl Into<String>,
        action: impl FnMut(&mut Self, &mut Ctx<Parlor>) + 'static,
    ) -> TitleRow<G> {
        MenuItem::new(label, action).help(help.into())
    }

    /// The game's own menu sounds and Settings rows, on entering or coming
    /// back to its title.
    fn use_game(ctx: &mut Ctx<Parlor>) {
        crate::menu_sounds::enter_game(ctx, G::ID, G::NAME, G::settings);
        // Back from the Sound check: music heard in play unlocks again.
        crate::soundtrack::listen(ctx);
    }
}

impl<G: TitleGame> Menu for Title<G> {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        G::TITLE_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        Self::use_game(ctx);
        self.game.music(ctx);
        ctx.play_ui(UiSound::Open);
        self.refresh(ctx, false);
        let opening = ctx.game.title_opening.take();
        let first = self.game.first_visit(ctx);
        if !first && !tutorial::has_played(ctx, G::ID) && !tutorial::tutorial_done(ctx, G::ID) {
            // The game's own saves show play from before story mode kept
            // track. Opening the title finishes no story scene.
            tutorial::note_played(ctx, G::ID);
        }
        if first {
            let (index, start) = match (self.game.first_cursor(), self.practice_at) {
                (FirstCursor::Practice, Some(at)) => (at, "Start with practice."),
                _ => (self.tutorial_at, "Start with the tutorial."),
            };
            self.menu.index = index;
            ctx.say(format!("{} New here? {start}", end_sentence(G::NAME)));
            if let Some(TitleOpening::Note(note)) = opening {
                ctx.say_queued(note);
            }
            let current = self.current_text(ctx);
            ctx.say_queued(current);
        } else {
            if let Some(TitleOpening::Row(start)) = opening {
                if let Some(at) = self.row_starting(ctx, &start) {
                    self.menu.index = at;
                }
            }
            self.announce(ctx);
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.game.update(ctx, dt);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        Self::use_game(ctx);
        self.game.music(ctx);
        self.refresh(ctx, true);
        self.after_lesson(ctx);
        self.announce(ctx);
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        Self::use_game(ctx);
        self.game.music(ctx);
        self.refresh(ctx, true);
        self.after_lesson(ctx);
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, result: &str) {
        Self::use_game(ctx);
        self.game.music(ctx);
        self.refresh(ctx, true);
        self.after_lesson(ctx);
        ctx.say(end_sentence(result));
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    /// Leaving for the launcher stops the music and closes the game.
    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        crate::soundtrack::listen(ctx);
        self.game.stop_music(ctx);
        crate::menu_sounds::leave_game(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<G>> {
        // Group 1, play.
        let mut items = self.game.play_rows(ctx);
        let challenges_help = self.game.challenges_help(ctx);
        items.push(Self::row(
            &labels::challenges_row_label(ctx, G::ID),
            challenges_help,
            |s: &mut Self, ctx: &mut Ctx<Parlor>| s.game.challenges(ctx),
        ));
        self.practice_at = None;
        if let Some(practice) = self.game.practice_row(ctx) {
            self.practice_at = Some(items.len());
            items.push(practice);
        }
        // Group 2, learn.
        self.tutorial_at = items.len();
        items.push(Self::row(
            labels::TUTORIAL,
            "Learn to play by playing, a step at a time, with your own controls.",
            |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                s.teaching = !tutorial::tutorial_done(ctx, G::ID);
                let lesson = s.game.lesson(ctx);
                ctx.push(lesson);
            },
        ));
        items.push(Self::row(
            labels::HELP,
            "The full guide, by section, with every control.",
            |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                let help = crate::seasonal::help(ctx, G::HELP);
                ctx.push(crate::Help::new(help))
            },
        ));
        let sound_help = self.game.sound_check_help(ctx);
        items.push(Self::row(
            labels::SOUND_CHECK,
            sound_help,
            |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                // A sound heard in the Sound check unlocks no track.
                crate::soundtrack::quiet(ctx);
                s.game.sound_check(ctx)
            },
        ));
        // Group 3, yours.
        let records_help = self.game.records_help(ctx);
        items.push(Self::row(
            labels::RECORDS,
            records_help,
            |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                let pages = s.game.records(ctx);
                ctx.push(Pages::new(pages));
            },
        ));
        items.extend(self.game.extras(ctx));
        items.push(Self::row(
            labels::SETTINGS,
            "Speech, volume and controls, shared by every game.",
            |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.push(SettingsScreen::<Parlor>::new()),
        ));
        items.extend(self.game.destructive(ctx));
        items.push(Self::row(
            labels::BACK_TO_PARLOR,
            "Return to the game list.",
            |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            },
        ));
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::{rows, Base};
    use hearken::{Headless, Key};

    #[derive(Default)]
    struct Dummy {
        practice: bool,
        extras: bool,
        practice_first: bool,
    }

    impl TitleGame for Dummy {
        const ID: &'static str = "dummy";
        const NAME: &'static str = "Dummy";
        const HELP: &'static str = "## About the game\n\nA test.\n";
        const TITLE_NAME: &'static str = "dummy";

        fn play_rows(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
            vec![
                MenuItem::new(
                    "Play, round 1",
                    |_s: &mut Title<Self>, ctx: &mut Ctx<Parlor>| ctx.say("Playing."),
                )
                .help("Round 1."),
                MenuItem::new(
                    labels::locked("Hard mode"),
                    |_s: &mut Title<Self>, _ctx: &mut Ctx<Parlor>| {},
                )
                .help("Finish round 1 to open hard mode."),
            ]
        }
        fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
            self.practice.then(|| {
                MenuItem::new(
                    "Practice room",
                    |_s: &mut Title<Self>, _ctx: &mut Ctx<Parlor>| {},
                )
                .help("Nothing to lose.")
            })
        }
        fn extras(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
            if self.extras {
                vec![
                    MenuItem::new("Journal", |_s: &mut Title<Self>, _ctx: &mut Ctx<Parlor>| {})
                        .help("Places found."),
                ]
            } else {
                Vec::new()
            }
        }
        fn first_cursor(&self) -> FirstCursor {
            if self.practice_first {
                FirstCursor::Practice
            } else {
                FirstCursor::Tutorial
            }
        }
        fn lesson(&mut self, _ctx: &mut Ctx<Parlor>) -> Lesson {
            Lesson::new(Self::ID, Self::NAME, "A test").step(crate::Step::press(
                "Space plays.",
                crate::ParlorAction::Primary,
                |_| "Played".to_string(),
            ))
        }
        fn records(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<String> {
            vec!["Records. None yet.".to_string()]
        }
        fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
            ctx.say("Sounds.");
        }
        fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
            ctx.say("Challenges.");
        }
    }

    fn open(game: Dummy) -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Title::new(game));
        h.frame(0.0);
        h
    }

    #[test]
    fn required_rows_appear_in_order() {
        let h = open(Dummy::default());
        assert_eq!(
            rows(&h),
            [
                "Play, round 1",
                "Hard mode, locked",
                "Challenges, daily not played yet",
                "Tutorial",
                "Help",
                "Sound check",
                "Records",
                "Settings",
                "Back to the parlor",
            ]
        );
    }

    #[test]
    fn optional_rows_go_in_their_groups() {
        let h = open(Dummy {
            practice: true,
            extras: true,
            ..Default::default()
        });
        assert_eq!(
            rows(&h),
            [
                "Play, round 1",
                "Hard mode, locked",
                "Challenges, daily not played yet",
                "Practice room",
                "Tutorial",
                "Help",
                "Sound check",
                "Records",
                "Journal",
                "Settings",
                "Back to the parlor",
            ]
        );
    }

    #[test]
    fn first_visit_puts_the_cursor_on_tutorial_then_on_play() {
        let mut h = open(Dummy::default());
        assert_eq!(
            h.spoken(),
            [
                "Dummy. New here? Start with the tutorial.",
                "Tutorial. 4 of 9."
            ]
        );
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "lesson");
        h.clear();
        h.press(Key::Space);
        assert_eq!(h.app.top_name(), "dummy");
        assert_eq!(
            h.spoken(),
            ["Played. Tutorial done.", "Play, round 1. 1 of 9."]
        );
        h.press(Key::Escape);
        h.clear();
        h.app.ctx.push(Title::new(Dummy::default()));
        h.frame(0.0);
        assert_eq!(h.spoken(), ["Dummy.", "Play, round 1. 1 of 9."]);
    }

    #[test]
    fn first_visit_can_start_on_practice() {
        let h = open(Dummy {
            practice: true,
            practice_first: true,
            ..Default::default()
        });
        assert_eq!(
            h.spoken(),
            [
                "Dummy. New here? Start with practice.",
                "Practice room. 4 of 10."
            ]
        );
    }

    #[test]
    fn p_jumps_to_play_then_practice() {
        let mut h = open(Dummy {
            practice: true,
            ..Default::default()
        });
        h.clear();
        h.press(Key::P);
        assert_eq!(h.spoken(), ["Play, round 1. 1 of 10."]);
        h.clear();
        h.press(Key::P);
        assert_eq!(h.spoken(), ["Practice room. 4 of 10."]);
    }

    #[test]
    fn every_row_has_help_and_back_pops() {
        let mut h = open(Dummy {
            practice: true,
            extras: true,
            ..Default::default()
        });
        let n = rows(&h).len();
        h.press(Key::Home);
        for _ in 0..n {
            h.clear();
            h.press(Key::F1);
            let said = h.spoken().join(" ");
            assert!(
                !said.is_empty() && said != hearken::menu::DEFAULT_INTRO_HELP,
                "{said}"
            );
            h.press(Key::Down);
        }
        h.press(Key::End);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "base");
    }

    #[test]
    fn the_sound_check_stops_music_unlocking_tracks_until_the_title_returns() {
        let mut h = open(Dummy::default());
        let at = rows(&h).iter().position(|r| r == "Sound check").unwrap();
        h.press(Key::Home);
        for _ in 0..at {
            h.press(Key::Down);
        }
        h.press(Key::Return);
        assert!(!h.app.ctx.soundtrack.is_listening());
        h.app.ctx.push(Pages::new(vec!["A page.".to_string()]));
        h.frame(0.0);
        h.press(Key::Escape);
        assert!(h.app.ctx.soundtrack.is_listening());
    }

    #[test]
    fn escape_leaves_for_the_launcher() {
        let mut h = open(Dummy::default());
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "base");
    }
}
