use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::screens::SettingsScreen;
use hearken::{Ctx, UiSound};
use parlor_core::menus::{labels, record_words};
use parlor_core::{
    challenges, Category, Challenge, Challenges, GameInfo, Happening, Pages, Parlor, Period,
    Records, NAME,
};

use crate::{music, ABOUT};

/// The `Menu` hooks that put back the shared menu sounds and start `$music`
/// whenever the screen is entered or revealed again, then do what a menu
/// does by default.
macro_rules! menu_music {
    ($music:path) => {
        fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
            parlor_core::menu_sounds::leave_game(ctx);
            $music(ctx);
            hearken::menu::menu_enter(self, ctx);
        }

        fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
            parlor_core::menu_sounds::leave_game(ctx);
            $music(ctx);
            self.refresh(ctx, true);
            self.announce(ctx);
        }

        fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
            parlor_core::menu_sounds::leave_game(ctx);
            $music(ctx);
            self.refresh(ctx, true);
        }

        fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, result: &str) {
            parlor_core::menu_sounds::leave_game(ctx);
            $music(ctx);
            self.refresh(ctx, true);
            ctx.say(hearken::text::end_sentence(result));
            let current = self.current_text(ctx);
            ctx.say_queued(current);
        }
    };
}

/// Where the id of the last game opened from the launcher is saved.
pub const LAST_GAME_KEY: &str = "parlor.last_game";

/// The last game opened from the launcher, if it is still in the parlor.
pub fn last_game(ctx: &mut Ctx<Parlor>) -> Option<GameInfo> {
    let id: Option<String> = Records::load(ctx, LAST_GAME_KEY);
    let id = id?;
    crate::listed(ctx)
        .into_iter()
        .chain(crate::story::unlocked_games(ctx))
        .find(|g| g.id == id)
}

/// The special event `g` holds today, if any.
pub(crate) fn happening(ctx: &mut Ctx<Parlor>, g: &GameInfo) -> Option<Happening> {
    let today = challenges::today(ctx);
    g.event_on(ctx, today)
}

/// Every game holding an event today, in the launcher's order, less the
/// categories and games the player left out in Game preferences.
pub fn happenings(ctx: &mut Ctx<Parlor>) -> Vec<(GameInfo, Happening)> {
    let prefs = parlor_core::preferences::load(ctx);
    all_happenings(ctx)
        .into_iter()
        .filter(|(g, _)| prefs.shows(g))
        .collect()
}

/// Every game holding an event today, whatever the preferences, as Play
/// lists them.
pub fn all_happenings(ctx: &mut Ctx<Parlor>) -> Vec<(GameInfo, Happening)> {
    crate::listed(ctx)
        .into_iter()
        .filter_map(|g| happening(ctx, &g).map(|h| (g, h)))
        .collect()
}

/// The first time the lobby opens, a word on today's events, or `None`:
/// "Happening today: Nevermore Manor, the house stirs." for one, and
/// "6 events today, Nevermore Manor first." for more, so the line stays
/// short on busy days. Happening today lists them all.
pub fn happenings_line(ctx: &mut Ctx<Parlor>) -> Option<String> {
    let all = happenings(ctx);
    let (g, h) = all.first()?;
    Some(match all.len() {
        1 => format!("Happening today: {}, {}.", g.name, after_comma(&h.title)),
        n => format!("{n} events today, {} first.", g.name),
    })
}

/// Names an event title can start with, which keep their capital after a
/// comma: holidays, days and the like.
const NAMED_STARTS: [&str; 22] = [
    "Halloween",
    "Easter",
    "Valentine's",
    "Thanksgiving",
    "Independence",
    "Fourth",
    "New",
    "Lunar",
    "Midsummer",
    "Friday",
    "Tranquility",
    "Geminid",
    "Leonid",
    "Lyrid",
    "Orionid",
    "Perseid",
    "Quadrantid",
    "Eta Aquariid",
    "Lantern Festival",
    "Sundae Thursday",
    "Ice Cream Social",
    "Peanut Butter Day",
];

/// An event title as it reads after a comma, as in "Nevermore Manor, the
/// house stirs": its first letter goes lower case, unless the title starts
/// with an acronym, as in "UFO night", or one of [`NAMED_STARTS`].
pub fn after_comma(title: &str) -> String {
    let named = NAMED_STARTS.iter().any(|name| {
        title
            .strip_prefix(name)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    });
    let mut chars = title.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let acronym = title.chars().nth(1).is_some_and(char::is_uppercase);
    if named || acronym {
        return title.to_string();
    }
    first.to_lowercase().chain(chars).collect()
}

/// What `record_words` says of a challenge with no score and no streak.
const NOT_PLAYED: &str = "not played yet";

/// Back in the lobby, the story's list is closed: a scene left from it
/// (Back to the lobby from a game's pause) stays on the list unplayed,
/// with nothing said about it later.
fn forget_story_scene(ctx: &mut Ctx<Parlor>) {
    ctx.game.story_scene = None;
    ctx.game.story_done = false;
}

/// Saves `g` as the last game played, for the Continue row.
pub(crate) fn note_last_game(ctx: &mut Ctx<Parlor>, g: &GameInfo) {
    Records::store(ctx, LAST_GAME_KEY, &Some(g.id));
}

/// Saves `g` as the last game played, then opens its title.
pub(crate) fn open_game(ctx: &mut Ctx<Parlor>, g: GameInfo) {
    note_last_game(ctx, &g);
    (g.open)(ctx)
}

/// Said when a list offers a holiday special not yet played, but the
/// internet date is not known now.
pub const NEEDS_DATE: &str =
    "New holiday specials need the date from the internet. Games you have played still open offline.";

/// [`open_game`] from a launcher list, checking first that the game is
/// still there: a list can be older than the date. Returns whether it
/// opened; if not, it says why and the caller rebuilds its rows.
pub(crate) fn open_listed(ctx: &mut Ctx<Parlor>, g: GameInfo) -> bool {
    match crate::access(ctx, g.id) {
        crate::Access::Open => {
            open_game(ctx, g);
            true
        }
        crate::Access::NeedsDate => {
            crate::discovery_clock::request();
            ctx.say(NEEDS_DATE);
            false
        }
        crate::Access::Closed => {
            ctx.say(format!("{} is back next season.", g.name));
            false
        }
    }
}

/// The Seasonal games row that checks the internet date again, while it
/// is not known.
fn date_row(ctx: &Ctx<Parlor>) -> Option<MenuItem<GameList, Parlor>> {
    use crate::discovery_clock::{status, Status};
    if crate::discovery_date(ctx).is_some() {
        return None;
    }
    let label = match status() {
        Status::Checking => "Checking for holiday specials",
        _ => "Check for holiday specials",
    };
    Some(
        MenuItem::new(label, |s: &mut GameList, ctx: &mut Ctx<Parlor>| {
            crate::discovery_clock::request();
            s.asked = true;
            ctx.say("Checking.");
        })
        .help(NEEDS_DATE),
    )
}

/// The Back row that ends each of the launcher's lists: it does what
/// Escape does there.
pub(crate) fn back_row<S: Menu<Game = Parlor>>(help: &'static str) -> MenuItem<S, Parlor> {
    MenuItem::new(labels::BACK, |_s: &mut S, ctx: &mut Ctx<Parlor>| {
        ctx.play_ui(UiSound::Back);
        ctx.pop()
    })
    .help(help)
}

/// The launcher's first screen.
pub struct Lobby {
    menu: MenuCore<Self, Parlor>,
}

impl Lobby {
    pub fn new() -> Self {
        parlor_core::preferences::register(crate::games::preference_games);
        crate::soundtrack::register();
        Lobby {
            menu: MenuCore::new(NAME),
        }
    }
}

impl Default for Lobby {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for Lobby {
    fn allows_update_prompt(&self) -> bool {
        true
    }
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "lobby"
    }

    /// Starts the lobby music and, the first time the lobby opens, says
    /// if the saved records had to start over, then names the events the
    /// games hold today.
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        // A lobby made after the app started still gives Hearken the tracks.
        if ctx.soundtrack.tracks().is_empty() {
            parlor_core::soundtrack::refresh(ctx);
        }
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        parlor_core::soundtrack::heard_music(ctx);
        hearken::menu::menu_enter(self, ctx);
        if !ctx.game.events_told {
            ctx.game.events_told = true;
            parlor_core::channel::keep_copy_channel(ctx);
            if let Some(line) = Records::take_notice(ctx) {
                ctx.say_queued(line);
            }
            if let Some(line) = happenings_line(ctx) {
                ctx.say_queued(line);
            }
            if let Some(line) = crate::extra::waiting_line(ctx) {
                ctx.say_queued(line);
            }
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        forget_story_scene(ctx);
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        forget_story_scene(ctx);
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, result: &str) {
        forget_story_scene(ctx);
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
        ctx.say(hearken::text::end_sentence(result));
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    /// Rebuilds the rows keeping the cursor on the same row by label, so
    /// a Continue row that appears does not move it.
    fn refresh(&mut self, ctx: &mut Ctx<Parlor>, keep_index: bool) {
        let old = self
            .menu
            .items
            .get(self.menu.index)
            .map(|item| item.text(self, ctx));
        let items = self.build_items(ctx);
        self.menu.set_items(items);
        let found = old.and_then(|old| {
            self.menu
                .items
                .iter()
                .position(|item| item.text(self, ctx) == old)
        });
        self.menu.index = match found {
            Some(at) if keep_index => at,
            _ if keep_index && self.menu.index < self.menu.items.len() => self.menu.index,
            _ => 0,
        };
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        if let Some(g) = last_game(ctx) {
            items.push(
                MenuItem::new(
                    format!("Continue, {}", g.name),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        if !open_listed(ctx, g) {
                            s.refresh(ctx, true);
                        }
                    },
                )
                .help(format!("Open {}, the last game you played.", g.name)),
            );
        }
        items.extend([
            MenuItem::new("Play", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(CategoryList::new())
            })
            .help("Choose a kind of game, then a game."),
            crate::today::lobby_row(ctx),
        ]);
        items.extend(crate::extra::lobby_row(ctx));
        items.extend([
            MenuItem::new(
                crate::story::row_label(),
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| crate::story::open(ctx),
            )
            .help(crate::story::row_help(ctx)),
            MenuItem::new("Challenges", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(ChallengeBoard::new())
            })
            .help("Every game's daily, weekly and monthly challenge, the same for every player."),
            MenuItem::new(crate::soundtrack::ROW, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(crate::soundtrack::Albums::new())
            })
            .help("Every game's music, to sit and listen. A track unlocks the first time you hear it in play."),
            MenuItem::new("Settings", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(SettingsScreen::<Parlor>::new())
            })
            .help("Volume, speech, and the controls every game shares."),
            MenuItem::new("Help", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(parlor_core::Help::new(crate::help(ctx)))
            })
            .help("How the parlor and its controls work."),
            MenuItem::new("About", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(Pages::new(ABOUT))
            })
            .help("What Wicket's is, its version and who made it."),
            MenuItem::new("Quit", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.push(parlor_core::menus::quit_confirm())
            })
            .help("Close Wicket's."),
        ]);
        items
    }

    /// The root of the stack never pops: Escape asks whether to quit.
    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(parlor_core::menus::quit_confirm());
    }
}

/// The kinds of game, each with how many it holds.
pub struct CategoryList {
    menu: MenuCore<Self, Parlor>,
}

impl CategoryList {
    pub fn new() -> Self {
        CategoryList {
            menu: MenuCore::new("Choose a kind of game"),
        }
    }
}

impl Default for CategoryList {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for CategoryList {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "categories"
    }

    menu_music!(music::lobby);

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let all = crate::listed(ctx);
        let events = all_happenings(ctx);
        Category::ALL
            .into_iter()
            .filter_map(|category| {
                let count = all.iter().filter(|g| g.category == category).count();
                if count == 0 {
                    return None;
                }
                let noun = if count == 1 { "game" } else { "games" };
                let on = events
                    .iter()
                    .filter(|(g, _)| g.category == category)
                    .count();
                let on = match on {
                    0 => String::new(),
                    1 => ", 1 event today".to_string(),
                    n => format!(", {n} events today"),
                };
                Some(
                    MenuItem::new(
                        format!("{}, {count} {noun}{on}", category.label()),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.push(GameList::new(category))
                        },
                    )
                    .help(match category {
                        Category::Seasonal => "Holiday games, one for each celebration.",
                        Category::Action => "Fast games that chase a high score.",
                        Category::Puzzle => "Listening puzzles with no clock to beat.",
                        Category::Cozy => "Calm games to unwind with and no way to fail.",
                        Category::Table => "Card and dice games, played at your own pace.",
                    }),
                )
            })
            .chain(story_row(ctx))
            .chain([back_row("Return to the lobby.")])
            .collect()
    }
}

/// The row of the story games, once story mode has opened one.
fn story_row(ctx: &mut Ctx<Parlor>) -> Option<MenuItem<CategoryList, Parlor>> {
    let count = crate::story::unlocked_games(ctx).len();
    let noun = match count {
        0 => return None,
        1 => "game",
        _ => "games",
    };
    Some(
        MenuItem::new(
            format!("{STORY_LABEL}, {count} {noun}"),
            |_s: &mut CategoryList, ctx: &mut Ctx<Parlor>| ctx.push(StoryGameList::new()),
        )
        .help("Games that story mode has opened for you, played here at any time."),
    )
}

/// The label of the story games, in the kinds of game.
pub const STORY_LABEL: &str = "From the story";

/// The story games the player has opened.
pub struct StoryGameList {
    menu: MenuCore<Self, Parlor>,
}

impl StoryGameList {
    pub fn new() -> Self {
        StoryGameList {
            menu: MenuCore::new(STORY_LABEL),
        }
    }
}

impl Default for StoryGameList {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for StoryGameList {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "story_games"
    }

    menu_music!(music::lobby);

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        crate::story::unlocked_games(ctx)
            .into_iter()
            .map(|g: GameInfo| {
                MenuItem::new(g.name, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    open_game(ctx, g)
                })
                .help(g.blurb)
            })
            .chain([back_row("Return to the kinds of game.")])
            .collect()
    }
}

/// The games of one category.
pub struct GameList {
    category: Category,
    menu: MenuCore<Self, Parlor>,
    /// The internet date's status when the rows were built, so the list
    /// rebuilds when it changes.
    seen: Option<crate::discovery_clock::Status>,
    /// The player chose the check row and waits to hear how it went.
    asked: bool,
}

impl GameList {
    pub fn new(category: Category) -> Self {
        GameList {
            category,
            menu: MenuCore::new(category.label()),
            seen: None,
            asked: false,
        }
    }
}

impl Menu for GameList {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "games"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.category == Category::Seasonal {
            crate::discovery_clock::request();
        }
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        hearken::menu::menu_enter(self, ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, result: &str) {
        parlor_core::menu_sounds::leave_game(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
        ctx.say(hearken::text::end_sentence(result));
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    /// Rebuilds the Seasonal games when the internet date arrives or
    /// fails, and answers a check the player asked for.
    fn update(&mut self, ctx: &mut Ctx<Parlor>, _dt: f64) {
        use crate::discovery_clock::Status;
        if self.category != Category::Seasonal || self.seen.is_none() {
            return;
        }
        let now = crate::discovery_clock::status();
        if self.seen == Some(now) {
            return;
        }
        self.refresh(ctx, true);
        if self.asked && now != Status::Checking {
            self.asked = false;
            ctx.say(match now {
                Status::Ready(_) => "Holiday specials checked.",
                _ => "No internet date. Try again later.",
            });
            let current = self.current_text(ctx);
            ctx.say_queued(current);
        }
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let seasonal = self.category == Category::Seasonal;
        self.seen = seasonal.then(crate::discovery_clock::status);
        let date_row = if seasonal { date_row(ctx) } else { None };
        let mut games: Vec<GameInfo> = crate::listed(ctx)
            .into_iter()
            .filter(|g| g.category == self.category)
            .collect();
        if self.category == Category::Seasonal {
            // Today's events first, then the next holiday round the year.
            let today = challenges::today(ctx);
            games.sort_by_key(|g| {
                (
                    happening(ctx, g).is_none(),
                    crate::days_to_holiday(g, today),
                )
            });
        }
        games
            .into_iter()
            .map(|g: GameInfo| {
                let (label, help) = match happening(ctx, &g) {
                    Some(h) => (
                        format!("{}, {} today", g.name, after_comma(&h.title)),
                        format!("{} {}", g.blurb, h.line),
                    ),
                    None => (g.name.to_string(), g.blurb.to_string()),
                };
                MenuItem::new(label, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    if !open_listed(ctx, g) {
                        s.refresh(ctx, true);
                    }
                })
                .help(help)
            })
            .chain(date_row)
            .chain([back_row("Return to the kinds of game.")])
            .collect()
    }
}

/// The three challenge periods, with how many of each are done.
pub struct ChallengeBoard {
    menu: MenuCore<Self, Parlor>,
}

impl ChallengeBoard {
    pub fn new() -> Self {
        ChallengeBoard {
            menu: MenuCore::new("Challenges"),
        }
    }
}

impl Default for ChallengeBoard {
    fn default() -> Self {
        Self::new()
    }
}

/// The games listed today that have challenges, then the story games
/// the player has opened.
fn challenge_games(ctx: &mut Ctx<Parlor>) -> Vec<GameInfo> {
    crate::listed(ctx)
        .into_iter()
        .chain(crate::story::unlocked_games(ctx))
        .filter(|g| g.challenge.is_some())
        .collect()
}

/// "Daily challenges, 3 of 27 done. A new one tomorrow."
pub(crate) fn period_label(ctx: &mut Ctx<Parlor>, period: Period) -> String {
    let games = challenge_games(ctx);
    let done = games
        .iter()
        .filter(|g| Challenges::done(ctx, &Challenge::current(ctx, g.id, period)))
        .count();
    let resets = Challenge::current(ctx, NAME, period).resets();
    format!(
        "{} challenges, {done} of {} done. {resets}",
        period.label(),
        games.len()
    )
}

/// What one period's challenges are.
pub(crate) fn period_help(period: Period) -> &'static str {
    match period {
        Period::Daily => "A short run in each game, new every day.",
        Period::Weekly => "A longer or harder run in each game, new every Monday.",
        Period::Monthly => "A special event in each game, new on the first of the month.",
    }
}

impl Menu for ChallengeBoard {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "challenges"
    }

    menu_music!(music::challenges);

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Period::ALL
            .into_iter()
            .map(|period| {
                MenuItem::new(
                    period_label(ctx, period),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(ChallengeList::new(period))
                    },
                )
                .help(period_help(period))
            })
            .collect();
        let streak = Challenges::parlor_streak(ctx);
        let line = match streak {
            0 => "No challenge streak yet. Finish any challenge to start one.".to_string(),
            1 => {
                "Challenge streak: 1 day. You finished a challenge today or yesterday.".to_string()
            }
            n => {
                format!("Challenge streak: {n} days in a row with at least one challenge finished.")
            }
        };
        // Enter reads the streak again, so the row never sounds dead.
        let again = line.clone();
        items.push(
            MenuItem::new(line, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.say(again.clone())
            })
            .help("Days in a row with at least one challenge finished, in any game."),
        );
        items.push(back_row("Return to the lobby."));
        items
    }
}

/// Every game's challenge for one period.
pub struct ChallengeList {
    period: Period,
    menu: MenuCore<Self, Parlor>,
}

impl ChallengeList {
    pub fn new(period: Period) -> Self {
        ChallengeList {
            period,
            menu: MenuCore::new(format!("{} challenges", period.label())),
        }
    }
}

impl Menu for ChallengeList {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "challenge list"
    }

    menu_music!(music::challenges);

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let period = self.period;
        challenge_games(ctx)
            .into_iter()
            .filter_map(|g| {
                let open = g.challenge?;
                let challenge = Challenge::current(ctx, g.id, period);
                // The bare name until there is something to say.
                let label = match record_words(ctx, &challenge) {
                    r if r == NOT_PLAYED => g.name.to_string(),
                    r => format!("{}, {r}", g.name),
                };
                Some(
                    MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        note_last_game(ctx, &g);
                        g.use_game(ctx);
                        open(ctx, period)
                    })
                    .help(g.blurb),
                )
            })
            .chain([back_row("Return to the Challenges board.")])
            .collect()
    }
}
