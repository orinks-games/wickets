//! The pause screen every game opens on Escape during play.
//!
//! It is built here rather than on `hearken::screens::PauseScreen`, whose
//! labels and Quit are fixed: this one names where its leave row goes
//! ("Back to the title", or "Leave the challenge"), and its last row goes
//! back to the lobby rather than quitting, so a stray Enter never closes
//! the program. Quitting is the lobby's job, and it asks first.
//!
//! While it is open the game's own sounds and music hold still, and a calm
//! pause loop plays in their place (see [`crate::menu_sounds::pause_music`]).
//! Closing it stops the loop and lets the game's sounds carry on.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::screens::{ConfirmOption, ConfirmScreen, SettingsScreen};
use hearken::{Bus, Ctx, Sound};

use super::labels;
use crate::Parlor;

/// The launcher's lobby, by its `State::name`: Back to the lobby pops to it.
pub const LOBBY_NAME: &str = "lobby";

/// The question Quit asks.
pub const QUIT_QUESTION: &str = "Quit Wicket's? Your progress is saved.";
/// Quit's answers.
pub const YES_QUIT: &str = "Yes, quit";
pub const NO_KEEP_PLAYING: &str = "No, keep playing";

/// Where the pause screen's fourth row goes, by the `State::name` it pops
/// back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leave {
    /// A normal run: "Back to the title", to the game's title.
    Title(&'static str),
    /// A challenge run: "Leave the challenge", to the screen that opened
    /// it, which is the game's challenges menu or what stands in for it
    /// when the launcher opened the challenge.
    Challenge(&'static str),
}

impl Leave {
    pub fn label(self) -> &'static str {
        match self {
            Leave::Title(_) => labels::BACK_TO_TITLE,
            Leave::Challenge(_) => labels::LEAVE_CHALLENGE,
        }
    }

    fn help(self) -> &'static str {
        match self {
            Leave::Title(_) => "Stop this run and go back to the title.",
            Leave::Challenge(_) => {
                "Stop without counting this challenge, so you can try it again later."
            }
        }
    }

    fn target(self) -> &'static str {
        match self {
            Leave::Title(name) | Leave::Challenge(name) => name,
        }
    }
}

/// Opens the pause screen over the run.
pub fn pause(ctx: &mut Ctx<Parlor>, help: &'static str, leave: Leave) {
    let help = crate::seasonal::help(ctx, help);
    ctx.push(Pause::new(help, leave));
}

/// The pause screen: Resume, Settings, Help, the leave row, Back to the
/// lobby.
pub struct Pause {
    menu: MenuCore<Self, Parlor>,
    help_doc: &'static str,
    leave: Leave,
    music: Option<Sound>,
}

/// How long the pause loop takes to fade out when the game resumes.
const MUSIC_FADE_MS: u32 = 400;

impl Pause {
    pub fn new(help_doc: &'static str, leave: Leave) -> Pause {
        Pause {
            menu: MenuCore::new("Paused")
                .with_intro_help("Up and Down move, Enter chooses, Escape goes back to the game."),
            help_doc,
            leave,
            music: None,
        }
    }
}

impl Menu for Pause {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "pause"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    /// Holds the game's sound and event speech while paused, and plays the
    /// pause loop on its own sound, so the game's music keeps its place.
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.pause_all();
        ctx.pause_event_speech();
        let key = crate::menu_sounds::pause_music(ctx);
        self.music = Some(ctx.audio.start_loop(key, None, Bus::MUSIC));
        menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(music) = self.music.take() {
            ctx.audio.stop(music, MUSIC_FADE_MS);
        }
        ctx.audio.resume_all();
        ctx.resume_event_speech();
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let leave = self.leave;
        vec![
            MenuItem::new(labels::RESUME, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Go back to where you were."),
            MenuItem::new(labels::SETTINGS, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(SettingsScreen::<Parlor>::new())
            })
            .help("Speech, volume and controls, shared by every game."),
            MenuItem::new(labels::HELP, |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(crate::Help::new(s.help_doc))
            })
            .help("The full guide, by section, with every control."),
            MenuItem::new(
                leave.label(),
                move |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop_to(leave.target()),
            )
            .help(leave.help()),
            MenuItem::new(
                labels::BACK_TO_LOBBY,
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop_to(LOBBY_NAME),
            )
            .help("Stop this run and go back to the lobby, where you can choose another game."),
        ]
    }
}

/// "Quit Wicket's? Your progress is saved." Yes quits, No goes back to
/// where the player was, and the cursor starts on No.
pub fn quit_confirm() -> ConfirmScreen<Parlor> {
    ConfirmScreen::with_options(
        QUIT_QUESTION,
        vec![
            ConfirmOption::new(YES_QUIT, |_ctx: &mut Ctx<Parlor>| {})
                .help("Close Wicket's now.")
                .ends_game(),
            ConfirmOption::close(NO_KEEP_PLAYING).help("Go back to where you were."),
        ],
    )
    .focus_cancel()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::{rows, Base};
    use crate::OpenGame;
    use hearken::{Bus, Headless, Key, State};

    struct Run;
    impl State<Parlor> for Run {
        fn name(&self) -> &'static str {
            "run"
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    fn paused(leave: Leave) -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Run);
        h.app.ctx.push(Pause::new("## Help\n\nText.\n", leave));
        h.frame(0.0);
        h
    }

    #[test]
    fn rows_name_where_leaving_goes() {
        let h = paused(Leave::Title("base"));
        assert_eq!(
            rows(&h),
            [
                "Resume",
                "Settings",
                "Help",
                "Back to the title",
                "Back to the lobby"
            ]
        );
        let h = paused(Leave::Challenge("base"));
        assert_eq!(rows(&h)[3], "Leave the challenge");
    }

    #[test]
    fn leaving_pops_to_its_target() {
        let mut h = paused(Leave::Challenge("base"));
        for _ in 0..3 {
            h.press(Key::Down);
        }
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "base");
    }

    #[test]
    fn the_last_row_goes_back_to_the_lobby_not_out() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Lobbyish);
        h.app.ctx.push(Run);
        h.app
            .ctx
            .push(Pause::new("## Help\n\nText.\n", Leave::Title("run")));
        h.frame(0.0);
        h.press(Key::End);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), LOBBY_NAME);
        assert!(h.app.ctx.running);
    }

    #[test]
    fn quit_asks_first_and_no_goes_back() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(quit_confirm());
        h.frame(0.0);
        assert_eq!(h.spoken(), [QUIT_QUESTION, "No, keep playing. 2 of 2."]);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "base");
        assert!(h.app.ctx.running);
        h.app.ctx.push(quit_confirm());
        h.frame(0.0);
        h.press(Key::Up);
        h.press(Key::Return);
        assert!(!h.app.ctx.running);
    }

    struct Lobbyish;
    impl State<Parlor> for Lobbyish {
        fn name(&self) -> &'static str {
            LOBBY_NAME
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    /// Each live sound's key and whether it is paused, on the music bus.
    fn music(h: &Headless<Parlor>) -> Vec<(String, bool)> {
        h.app
            .ctx
            .audio
            .live()
            .into_iter()
            .filter(|l| l.bus == Bus::MUSIC)
            .map(|l| (l.key, l.paused))
            .collect()
    }

    #[test]
    fn pause_plays_its_own_music_and_hands_back() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Run);
        h.frame(0.0);
        h.app.ctx.audio.play_music(Some("game/music/run"), true);
        h.app
            .ctx
            .push(Pause::new("## Help\n\nText.\n", Leave::Title("base")));
        h.frame(0.0);
        assert_eq!(
            music(&h),
            [
                ("game/music/run".to_string(), true),
                (crate::menu_sounds::PAUSE_MUSIC.to_string(), false)
            ]
        );
        h.press(Key::Escape);
        assert_eq!(music(&h), [("game/music/run".to_string(), false)]);
        assert_eq!(h.app.ctx.audio.music(), Some("game/music/run"));
    }

    #[test]
    fn a_game_with_its_own_pause_music_plays_it() {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(Run);
        h.frame(0.0);
        h.app.ctx.game.open = Some(OpenGame {
            pause_music: Some("belfry/music/pause"),
            ..OpenGame::new("belfry", "Belfry", |_| Vec::new())
        });
        h.app
            .ctx
            .push(Pause::new("## Help\n\nText.\n", Leave::Title("base")));
        h.frame(0.0);
        assert_eq!(music(&h), [("belfry/music/pause".to_string(), false)]);
    }

    #[test]
    fn escape_resumes() {
        let mut h = paused(Leave::Title("base"));
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "run");
    }
}
