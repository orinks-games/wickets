// What every Wicket's game shares.
//
// Each game is its own crate under `games/`. It depends on this crate and
// on `hearken`, and exposes one [`GameInfo`] the launcher lists. Every game
// runs on the same [`Parlor`] game type, so one window, one settings file
// and one set of rebindable controls serve the whole collection.

mod actions;
pub mod almanac;
pub mod calendar;
pub mod challenges;
pub mod channel;
pub mod help;
pub mod menu_sounds;
pub mod menus;
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
pub use calendar::{Happening, Holiday, Nearness, Season};
pub use challenges::{Challenge, Challenges, Date, Period};
pub use help::Help;
pub use pages::{explain, Pages};
pub use records::{Records, ScoreResult};
pub use rng::Rng;
pub use score::{Mood, Score, Stem};
pub use seats::Seats;
pub use sim::FixedStep;
pub use status::StatusKey;
pub use tutorial::{Lesson, Live, Reaction, Reply, Step};
pub use unlocks::{Unlock, UnlockGame, Unlocks, UnlocksMenu};

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
    /// The open game's id, from its title, or `None` in the launcher and
    /// story mode. Pages keep what the player has learned under it.
    pub open_game: Option<&'static str>,
    /// The open game's name, for Discord status. Set with `open_game`, and
    /// by story mode for a scene it opens straight into play.
    pub open_game_name: Option<&'static str>,
    /// The open game's own page cue, if it ships one (see [`menu_sounds`]).
    pub page_sound: Option<&'static str>,
    /// The open game's own pause music, if it ships one (see [`menu_sounds`]).
    pub pause_music: Option<&'static str>,
    /// The open game's own rows for the Settings screen (see
    /// [`menus::TitleGame::settings`]), after the shared ones.
    pub game_settings: Option<GameSettings>,
    /// Whether the lobby has named today's events yet this session.
    pub events_told: bool,
    /// Pins the player's local time for story mode, so tests are not tied
    /// to the clock. `None` (the default) reads the trusted clock.
    pub now: Option<hearken::LocalTime>,
    /// The game story mode has open as a scene, if any.
    pub story_scene: Option<&'static str>,
    /// Set once that game finishes a run, so story mode can tell a played
    /// scene from one the player left early.
    pub story_done: bool,
    /// The row the next title opens on, by the start of its label (as
    /// "The house stirs"), when the player came for that row: an event
    /// from the story or the Happening today list. Taken by the title.
    pub title_focus: Option<String>,
    /// A line the next title says after its opening. Taken by the title.
    pub title_note: Option<String>,
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

    fn discord() -> Option<hearken::DiscordConfig> {
        presence::config()
    }

    fn presence(ctx: &Ctx<Self>) -> Option<hearken::Presence> {
        Some(presence::outside_story(&ctx.game))
    }

    fn settings_rows(
        ctx: &Ctx<Self>,
        mut rows: Vec<hearken::screens::SettingsRow<Self>>,
    ) -> Vec<hearken::screens::SettingsRow<Self>> {
        // The open game's own rows come first, so Settings from a game
        // opens on that game's options; the shared ones follow, with Units
        // last unless the game listed it among its own.
        let mut out = match ctx.game.game_settings {
            Some(game) => game(ctx),
            None => Vec::new(),
        };
        let has_units = out.iter().any(|row| row.id == units::ROW_ID);
        out.append(&mut rows);
        if !has_units {
            out.push(units::row());
        }
        // Preferences shape the launcher's story and events, so they are
        // only offered from the launcher's Settings.
        if ctx.game.game_settings.is_none() {
            out.push(preferences::row());
        }
        out
    }
}

/// Builds a game's own Settings rows.
pub type GameSettings = fn(&Ctx<Parlor>) -> Vec<hearken::screens::SettingsRow<Parlor>>;

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
        menu_sounds::use_game(ctx, self.id);
        ctx.game.open_game_name = Some(self.name);
        ctx.game.game_settings = Some(self.settings);
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
