// Midnight Lanes: bowl ten frames on lane nine after the alley closes.
//
// The player chooses a board to stand on and an aim, then times the
// release on a rising swing tone. The ball rolls down the lane in stereo
// and the pins fall by a deterministic pin model; only the release, off
// the top of the swing, adds a pull drawn from a seeded source. Standard
// ten-pin scoring, every leave spoken by pin number, records of the high
// game and strikes in a row, and daily, weekly and monthly challenges.

pub mod challenge;
pub mod lane;
pub mod play;
pub mod progress;
pub mod pumpkin;
pub mod rules;
pub mod sound;
pub mod soundcheck;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Nights, CHALLENGES_NAME};
pub use play::{Bowl, Mode};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

/// The game's id: its sounds live under `sounds/midnight-lanes/`.
pub const ID: &str = "midnight-lanes";

/// The game's name.
pub const NAME: &str = "Midnight Lanes";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a game returns to.
pub const TITLE_NAME: &str = "midnight_lanes";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Action,
    blurb: "Bowl ten frames on lane nine after the alley closes, by ear.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(pumpkin::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <MidnightLanes as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Night],
        seasons: &[],
    },
};

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the lane. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the high game and the tallies, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let best = match Records::best(ctx, ID, progress::GAME_BOARD) {
        Some(b) => format!("high game {b}"),
        None => "no game yet".to_string(),
    };
    let run = match p.best_streak {
        0 => "no strikes in a row yet".to_string(),
        1 => "most strikes in a row 1".to_string(),
        n => format!("most strikes in a row {n}"),
    };
    let perfect = if p.perfect {
        " You have bowled a perfect game."
    } else {
        ""
    };
    let series = match p.best_series {
        Some(s) => format!("High series {s}."),
        None => "No series yet: the monthly challenge is a three-game series.".to_string(),
    };
    let games = match p.games {
        1 => "1 game".to_string(),
        n => format!("{n} games"),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!("Records. Full game: {best}, {run}.{perfect} {series}"),
        format!(
            "Bowled so far: {games}, {} strikes and {} spares.",
            p.strikes, p.spares
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct MidnightLanes;

/// The game's title screen.
pub type Title = menus::Title<MidnightLanes>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(MidnightLanes)
}

impl TitleGame for MidnightLanes {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let label = match Records::best(ctx, ID, progress::GAME_BOARD) {
            Some(b) => format!("{}, full game, best {b}", labels::PLAY),
            None => format!("{}, full game", labels::PLAY),
        };
        let mut rows = vec![
            MenuItem::new(label, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Bowl::new(Mode::Game))
            })
            .help("Ten frames on lane nine, scored the standard way."),
        ];
        let today = parlor_core::challenges::today(ctx);
        if pumpkin::is_open(today) {
            let help = pumpkin::row_help(today, Records::best(ctx, ID, pumpkin::BOARD));
            rows.push(
                MenuItem::new(pumpkin::NAME, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Bowl::new(Mode::Pumpkin))
                })
                .help(help),
            );
        } else {
            // Out of season the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(pumpkin::NAME),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.say(pumpkin::locked_line(today))
                    },
                )
                .select_sound(None)
                .help(pumpkin::locked_help(today)),
            );
        }
        rows
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your high game, strikes in a row, high series, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear each sound you need on the lane, with its name and what it tells you.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily game, a weekly set of spares and a monthly series, the same lane for every player. {}",
            daily.resets()
        )
    }

    /// A player who has bowled is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }
}
