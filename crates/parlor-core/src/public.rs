// What every Wicket's game shares.
//
// Each game is its own crate under `games/`. It depends on this crate and
// on `hearken`, and exposes one [`GameInfo`] the launcher lists. Every game
// runs on the same [`Parlor`] game type, so one window, one settings file
// and one set of rebindable controls serve the whole collection.

mod actions;
pub mod almanac;
pub mod briefing;
pub mod calendar;
pub mod challenges;
pub mod channel;
pub mod help;
pub mod menu_sounds;
pub mod menus;
pub mod pace;
mod pages;
pub mod preferences;
pub mod presence;
pub mod records;
mod rng;
pub mod score;
pub mod seasonal;
pub mod seats;
pub mod shop_music;
pub mod sim;
pub mod soundtrack;
pub mod status;
pub mod story;
pub mod testkit;
pub mod tutorial;
pub mod units;
pub mod unlocks;
pub mod visuals;
pub mod words;

pub use actions::ParlorAction;
pub use briefing::Briefing;
pub use calendar::{Happening, Holiday, Nearness, Season};
pub use challenges::{Challenge, Challenges, Date, Period};
pub use help::Help;
pub use pace::{Pace, RunClock};
pub use pages::{explain, Pages};
pub use records::{Records, ScoreResult};
pub use rng::Rng;
pub use score::{Mood, Score, Stem};
pub use seats::Seats;
pub use sim::FixedStep;
pub use status::StatusKey;
pub use tutorial::{Lesson, Live, Reaction, Reply, Step};
pub use unlocks::{Unlock, UnlockGame, Unlocks, UnlocksMenu};

use hearken::screens::settings::categories;
use hearken::Ctx;

/// The collection's name, as the window title and the launcher say it.
pub const NAME: &str = "Wicket's";

/// The one game type every screen in the collection runs on.
pub struct Parlor;

/// Shared state: the saved records of every game.
#[derive(Default)]
pub struct ParlorData {
    pub records: Records,
    /// Pins "today" for challenges, so tests are not tied to the clock.
    /// `None` (the default) uses the system clock.
    pub today: Option<Date>,
    /// Which lobby piece the launcher plays next, so each return to the
    /// lobby can bring a different one. `None` until the launcher first
    /// picks, from the clock.
    pub lobby_turn: Option<u32>,
    /// Which of story mode's sitting pieces plays next, the same way.
    pub story_turn: Option<u32>,
    /// The game the player is in, or `None` in the launcher's own screens
    /// and the story's sitting. Set by [`menu_sounds::enter_game`] and
    /// cleared by [`menu_sounds::leave_game`]: every answer to "which game
    /// is this" reads it.
    pub open: Option<OpenGame>,
    /// Whether the lobby has named today's events yet this session.
    pub events_told: bool,
    /// Pins the player's local time for story mode, so tests are not tied
    /// to the clock. `None` (the default) reads the trusted clock.
    pub now: Option<hearken::LocalTime>,
    /// The scene story mode has open, if any: its game, and whether a run
    /// has finished, so the sitting can tell a played scene from one the
    /// player left early (see [`StoryScene`]).
    pub story_scene: Option<StoryScene>,
    /// What the next title opens with, left by the screen sending the
    /// player there and taken whole by the title (see [`TitleOpening`]).
    pub title_opening: Option<TitleOpening>,
}

/// The folder (and seal witness) name the game keeps its files under.
pub const DATA_DIR: &str = "Wickets";

/// The name those files had before 1.1.1, moved to [`DATA_DIR`] at start-up
/// (see `wickets`' main).
pub const OLD_DATA_DIR: &str = "EchoParlor";

impl hearken::Game for Parlor {
    const TITLE: &'static str = NAME;
    const DATA_DIR: &'static str = DATA_DIR;
    const ENV_PREFIX: &'static str = "ECHO_PARLOR";
    // Players from before sealing keep their records (see `records`). Turn
    // off once they have had a sealing release; then no marker is left.
    const IMPORTS_PLAIN_SAVES: bool = true;
    type Event = ();
    type Data = ParlorData;
    type Settings = ();
    type Action = ParlorAction;
    type Category = hearken::NoCategory;

    /// Every game's music, as the launcher registered it (see
    /// [`soundtrack`]).
    fn soundtrack() -> Vec<hearken::Track> {
        soundtrack::tracks()
    }

    /// Released copies update themselves from the public release
    /// repository, orinks-games/wickets (this source repository is
    /// private), where the release workflow uploads each platform's signed
    /// manifest beside its package. Players choose stable, beta or alpha
    /// in Settings (see [`channel`]).
    fn updates() -> Option<hearken::UpdateConfig> {
        hearken::UpdateConfig::github(NAME, "orinks-games", "wickets").map(|c| {
            c.channels(&channel::CHANNELS)
                .own_build(&channel::OWN_BUILD)
                .prompt_at_start()
        })
    }

    /// Report a problem, in Settings: the game log goes ahead to orinks.net
    /// and the report form opens in the browser with it attached; the site
    /// emails the report. The engine asks once after a run that never
    /// exited cleanly. The www host on purpose: the apex answers with a
    /// redirect, which the sender does not follow.
    fn reports() -> Option<hearken::ReportConfig> {
        hearken::ReportConfig::new(
            NAME,
            env!("CARGO_PKG_VERSION"),
            "https://www.orinks.net/api/problem-reports",
        )
    }

    /// Which game the player has open, for a problem report: the report
    /// says `Game: Tightrope`, and its email subject names it. A report
    /// from the lobby says nothing more than the app.
    fn report_details(ctx: &Ctx<Self>) -> Vec<(String, String)> {
        match ctx.game.open {
            Some(game) => vec![("Game".to_string(), game.name.to_string())],
            None => Vec::new(),
        }
    }

    fn discord() -> Option<hearken::DiscordConfig> {
        presence::config()
    }

    fn presence(ctx: &Ctx<Self>) -> Option<hearken::Presence> {
        Some(presence::outside_story(&ctx.game))
    }

    /// Settings opens on categories: the open game's own options first,
    /// named after the game, then Speech, Sound, Display, Controls, Games
    /// (from the launcher), Online and Updates.
    fn settings_categories() -> bool {
        true
    }

    fn settings_rows(
        ctx: &Ctx<Self>,
        mut rows: Vec<hearken::screens::SettingsRow<Self>>,
    ) -> Vec<hearken::screens::SettingsRow<Self>> {
        // The open game's own rows come first, under the game's name, so
        // Settings from a game opens on that game's options. Units ends that
        // list unless the game placed it itself. In the launcher, Units and
        // Game preferences make up Games.
        let mut out = match ctx.game.open {
            Some(game) => (game.settings)(ctx),
            None => Vec::new(),
        };
        let has_units = out.iter().any(|row| row.id == units::ROW_ID);
        if !has_units {
            out.push(units::row());
        }
        let own = match ctx.game.open {
            Some(game) => game.name,
            None => "Games",
        };
        // Preferences shape the launcher's story and events, so they are
        // only offered from the launcher's Settings.
        if ctx.game.open.is_none() {
            out.push(preferences::row());
        }
        let mut out: Vec<_> = out.into_iter().map(|row| row.in_category(own)).collect();
        // The launcher's Games sits after the built-in categories, before
        // Online and Updates; a game's own list leads.
        let online = rows
            .iter()
            .position(|row| {
                matches!(
                    row.category.as_deref(),
                    Some(categories::ONLINE | categories::UPDATES)
                )
            })
            .unwrap_or(rows.len());
        if ctx.game.open.is_some() {
            out.append(&mut rows);
        } else {
            let mut tail = rows.split_off(online);
            rows.append(&mut out);
            rows.append(&mut tail);
            out = rows;
        }
        out
    }
}

/// Builds a game's own Settings rows.
pub type GameSettings = fn(&Ctx<Parlor>) -> Vec<hearken::screens::SettingsRow<Parlor>>;

/// The game the player is in: from its title, or a run opened straight
/// from a launcher list. One record holds everything the shared screens
/// need to know about it, so a game's sounds and Settings rows can neither
/// outlive it nor go missing on one way in.
#[derive(Clone, Copy)]
pub struct OpenGame {
    /// The game's folder id, as `belfry`. Pages keep what the player has
    /// learned under it.
    pub id: &'static str,
    /// The game's name, for Discord status and its Settings category.
    pub name: &'static str,
    /// The game's own rows for the Settings screen (see
    /// [`menus::TitleGame::settings`]), after the shared ones.
    pub settings: GameSettings,
    /// The game's own page cue, if it ships one (see [`menu_sounds`]).
    pub page_sound: Option<&'static str>,
    /// The game's own pause music, if it ships one (see [`menu_sounds`]).
    pub pause_music: Option<&'static str>,
}

impl OpenGame {
    /// A game with none of its own sounds, for tests and for a build
    /// whose sound bank has not loaded them.
    pub fn new(id: &'static str, name: &'static str, settings: GameSettings) -> Self {
        OpenGame {
            id,
            name,
            settings,
            page_sound: None,
            pause_music: None,
        }
    }
}

/// What the next title opens with, when the player came for something on
/// it. The title takes it whole on entering. A first visit starts on
/// Tutorial (or the practice row) whatever the row asked for, and only a
/// first visit says the note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TitleOpening {
    /// Open on the row whose label starts with this, as "The house
    /// stirs": an event from the story or the Happening today list.
    Row(String),
    /// A line said after the first-visit opening, from the screen that
    /// sent the player here: what plays once the tutorial is done.
    Note(String),
}

/// A story scene while its game is open. The sitting sets `Open` when it
/// sends the player into the game; [`tutorial::mark_played`] (a finished
/// run, score or challenge) and a finished lesson move it to `Finished`;
/// the sitting takes it when the player comes back, and the lobby clears
/// it. A scene can only be finished while it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryScene {
    /// The game is open and no run has finished yet.
    Open(&'static str),
    /// The game has finished a run, or its lesson.
    Finished(&'static str),
}

impl StoryScene {
    /// The scene's game id.
    pub fn game(self) -> &'static str {
        match self {
            StoryScene::Open(id) | StoryScene::Finished(id) => id,
        }
    }

    /// Whether the scene's game has finished a run.
    pub fn is_finished(self) -> bool {
        matches!(self, StoryScene::Finished(_))
    }
}

impl ParlorData {
    /// Marks the open story scene finished when it is game `id`'s. Any
    /// other game, or no open scene, changes nothing.
    pub fn finish_story_scene(&mut self, id: &str) {
        if let Some(scene) = self.story_scene {
            if scene.game() == id {
                self.story_scene = Some(StoryScene::Finished(scene.game()));
            }
        }
    }
}

/// Where a game sits in the launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    /// Holiday-themed games, one per celebration.
    Seasonal,
    /// Score-chasing action games.
    Action,
    /// Thinking games with no clock to beat.
    Puzzle,
    /// Calm games with no way to fail.
    Cozy,
    /// Card and dice games, played at your own pace.
    Table,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Seasonal,
        Category::Action,
        Category::Puzzle,
        Category::Cozy,
        Category::Table,
    ];

    /// A stable id, as saved preferences and story files name the kind.
    pub fn id(self) -> &'static str {
        match self {
            Category::Seasonal => "seasonal",
            Category::Action => "action",
            Category::Puzzle => "puzzle",
            // Cozy games were once Relaxing games; the id stays so saved
            // preferences and story files keep working.
            Category::Cozy => "relaxing",
            Category::Table => "table",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Seasonal => "Seasonal games",
            Category::Action => "Action games",
            Category::Puzzle => "Puzzle games",
            Category::Cozy => "Cozy games",
            Category::Table => "Table games",
        }
    }
}

/// One game, as the launcher lists it.
#[derive(Clone, Copy)]
pub struct GameInfo {
    /// A stable id: names the game's sound folder (`sounds/<id>/`) and its
    /// saved records. Never change one that has shipped.
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    /// One sentence the launcher reads after the name.
    pub blurb: &'static str,
    /// Opens the game's own title screen on top of the launcher, with
    /// `ctx.push`. Leaving the game pops back to the launcher.
    pub open: fn(&mut Ctx<Parlor>),
    /// Opens the game's current daily, weekly or monthly challenge on top
    /// of the launcher (see [`challenges`]). `None` while a game has none.
    pub challenge: Option<fn(&mut Ctx<Parlor>, Period)>,
    /// The special event the game holds on `date`, if any, such as a
    /// seasonal game on its holiday (see [`calendar`]). The launcher names
    /// it on the lobby and the game list. `None` for a game with no events.
    pub event: Option<fn(Date) -> Option<Happening>>,
    /// Whether the player has already finished the event `event` names on
    /// `date`, so the title no longer offers it (as Nevermore Manor once
    /// every candle is lit). The launcher and story mode then stop asking
    /// for it. `None` for a game whose events never run out.
    pub event_done: Option<fn(&mut Ctx<Parlor>, Date) -> bool>,
    /// The game's interactive tutorial (see [`tutorial`]), which its
    /// title's Tutorial row opens and story mode opens for a new player.
    pub tutorial: fn(&mut Ctx<Parlor>) -> Lesson,
    /// The game's own Settings rows, the same function as its
    /// `TitleGame::settings`, for runs opened without the title (a
    /// challenge from the launcher, a story scene).
    pub settings: GameSettings,
    /// When the game suits story mode best: parts of the day and seasons
    /// (see [`story`]). Plain data, so the planner runs no game code.
    pub story: story::StoryTags,
}

impl GameInfo {
    /// The `State::name` of the game's title screen, which its runs return
    /// to: the id with `_` for `-`, as every title is named.
    pub fn title_name(&self) -> String {
        self.id.replace('-', "_")
    }

    /// Opens the game's tutorial on top of the current screen. Finishing
    /// it goes back there with "Tutorial done."; for anything else, build
    /// it with `(info.tutorial)(ctx)` and add `Lesson::then`.
    pub fn open_tutorial(&self, ctx: &mut Ctx<Parlor>) {
        let lesson = (self.tutorial)(ctx);
        ctx.push(lesson);
    }

    /// The event the game holds on `date`, unless the player has already
    /// finished it.
    pub fn event_on(&self, ctx: &mut Ctx<Parlor>, date: Date) -> Option<Happening> {
        let h = (self.event?)(date)?;
        match self.event_done {
            Some(done) if done(ctx, date) => None,
            _ => Some(h),
        }
    }

    /// Uses the game's own menu sounds and Settings rows, for a run opened
    /// straight from the launcher rather than through the title.
    pub fn use_game(&self, ctx: &mut Ctx<Parlor>) {
        menu_sounds::enter_game(ctx, self.id, self.name, self.settings);
    }

    /// Whether the player has played this game, from anywhere.
    pub fn has_played(&self, ctx: &mut Ctx<Parlor>) -> bool {
        tutorial::has_played(ctx, self.id)
    }

    /// Whether the player has finished this game's tutorial.
    pub fn tutorial_done(&self, ctx: &mut Ctx<Parlor>) -> bool {
        tutorial::tutorial_done(ctx, self.id)
    }

    /// Whether to teach this game before playing it: never played and
    /// never taught.
    pub fn needs_lesson(&self, ctx: &mut Ctx<Parlor>) -> bool {
        tutorial::needs_lesson(ctx, self.id)
    }
}
