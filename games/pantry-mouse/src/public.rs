// Pantry Mouse: eat every crumb in the pantry while the cats prowl.
//
// The player is a mouse in an old farmhouse pantry at night. The pantry is
// a small grid maze; the mouse moves one cell per step, and every crumb it
// eats is 10 points. Two cats prowl by plain rules: the ginger cat heads
// straight for the mouse, the tabby wanders until it is near, then chases.
// Each cat wears its own sound, placed where it is. A catch costs a
// whisker, and the third ends the run. Clearing a pantry opens the next,
// with quicker cats. Records keep the best score and the deepest pantry,
// and daily, weekly and monthly challenges give every player the same
// seeded mazes.

pub mod challenge;
pub mod events;
pub mod play;
pub mod progress;
pub mod rules;
pub mod sound;
pub mod soundcheck;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{pace, Category, Challenge, Challenges, GameInfo, Pace, Parlor, Period};

pub use challenge::{ChallengeHost, ChallengesMenu, Larders, CHALLENGES_NAME};
pub use play::{Mode, Play};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{crumbs_words, WHISKERS};

/// The game's id: its sounds live under `sounds/pantry-mouse/`.
pub const ID: &str = "pantry-mouse";

/// The game's name.
pub const NAME: &str = "Pantry Mouse";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a run returns to.
pub const TITLE_NAME: &str = "pantry_mouse";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Action,
    blurb: "Eat every crumb in the pantry while the cats prowl.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <PantryMouse as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Night,
            parlor_core::story::PartOfDay::SmallHours,
        ],
        seasons: &[parlor_core::Season::Autumn, parlor_core::Season::Winter],
    },
};

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the maze. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the pantry's bests, the tallies, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let best = pace::scored_bests(ctx, ID, progress::PANTRY_BOARD, "best score")
        .unwrap_or_else(|| "no score yet".to_string());
    let deepest = match p.deepest {
        0 => "no pantry reached yet".to_string(),
        n => format!("deepest pantry {n}"),
    };
    let runs = match p.runs {
        1 => "1 pantry run".to_string(),
        n => format!("{n} pantry runs"),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    let mut pages = vec![
        format!("Records. The pantry: {best}, {deepest}."),
        format!(
            "Played so far: {runs}, {} eaten in all, and {} in the practice pantry.",
            crumbs_words(p.crumbs),
            crumbs_words(p.practice_crumbs)
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ];
    let events: Vec<String> = [
        (events::LEFTOVERS, events::LEFTOVERS_BOARD),
        (events::BIN_NIGHT, events::BIN_NIGHT_BOARD),
    ]
    .into_iter()
    .filter_map(|(name, board)| {
        pace::scored_bests(ctx, ID, board, "best").map(|b| format!("{name}, {b}."))
    })
    .collect();
    if !events.is_empty() {
        pages.push(format!("Special nights. {}", events.join(" ")));
    }
    pages
}

/// The best score on an event's `board` and on its relaxed board.
fn event_bests(ctx: &mut Ctx<Parlor>, board: &str) -> events::Bests {
    (
        pace::best(ctx, ID, board),
        pace::best(ctx, ID, &Pace::Relaxed.board(board)),
    )
}

/// The game on the shared title builder.
pub struct PantryMouse;

/// The game's title screen.
pub type Title = menus::Title<PantryMouse>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(PantryMouse)
}

impl TitleGame for PantryMouse {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![pace::row(ID)]
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let label = match pace::scored_bests(ctx, ID, progress::PANTRY_BOARD, "best") {
            Some(b) => format!("{}, the pantry, {b}", labels::PLAY),
            None => format!("{}, the pantry", labels::PLAY),
        };
        let deepest = match Progress::load(ctx).deepest {
            0 => String::new(),
            n => format!(" Deepest pantry so far: {n}."),
        };
        let mut rows = vec![
            MenuItem::new(label, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Play::new(Mode::Pantry))
            })
            .help(format!(
                "One cat in the first pantry, then 2, quicker each time, until your {WHISKERS} whiskers are gone.{deepest}"
            )),
        ];
        let today = parlor_core::challenges::today(ctx);
        if events::leftovers_open(today) {
            let help = events::leftovers_help(event_bests(ctx, events::LEFTOVERS_BOARD));
            rows.push(
                MenuItem::new(
                    events::LEFTOVERS,
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(Play::new(Mode::Leftovers)),
                )
                .help(help),
            );
        } else {
            // Out of season: the row keeps its place and says when it opens.
            let best = event_bests(ctx, events::LEFTOVERS_BOARD);
            rows.push(
                MenuItem::new(
                    labels::locked(events::LEFTOVERS),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::leftovers_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::leftovers_locked_help(today, best)),
            );
        }
        let best = event_bests(ctx, events::BIN_NIGHT_BOARD);
        if events::bin_night_open(today) {
            rows.push(
                MenuItem::new(
                    events::BIN_NIGHT,
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(Play::new(Mode::BinNight)),
                )
                .help(events::bin_night_help(best)),
            );
        } else {
            // Closed today: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::BIN_NIGHT),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::bin_night_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::bin_night_locked_help(today, best)),
            );
        }
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new(
                "Practice pantry",
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(Play::new(Mode::Practice)),
            )
            .help("One slow cat, and a catch costs nothing: learn the maze and the cats."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best score and deepest pantry, your tallies, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Play the cats and cues you need in the pantry.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily, a weekly and a monthly maze, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has played is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }
}
