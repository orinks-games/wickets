//! The Echo: tune the Echo's pipes by ear.
//!
//! A story game: it opens in act 4, when Theo's reel arrives and the keeper
//! learns to tune the listening machine in the cellar. A rank of pipes,
//! low to high, each a little sharp or flat. The pipe plays against its
//! true note and the two wobble; the player raises or lowers the pipe
//! until the wobble is still, then sets it. Once the rank is in tune the
//! Echo plays it back as a tune. Nothing fails.

pub mod challenge;
pub mod play;
pub mod progress;
pub mod rules;
pub mod sound;
pub mod soundcheck;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Ranks, CHALLENGES_NAME};
pub use play::{Mode, Tuning};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{pipes_words, steps_words, RANK};

/// The game's id: its sounds live under `sounds/the-echo/`.
pub const ID: &str = "the-echo";

/// The game's name.
pub const NAME: &str = "The Echo";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a rank returns to.
pub const TITLE_NAME: &str = "the_echo";

/// How the launcher lists the game, once the story opens it.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Puzzle,
    blurb: "Tune the Echo's pipes until the wobble is still.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: None,
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <TheEcho as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[],
        seasons: &[],
    },
};

/// Opens the current `period` challenge: its briefing, then the rank.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the evening's best, the tallies, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let best = match Records::best(ctx, ID, progress::RANK_BOARD) {
        Some(b) => format!("best score {b}"),
        None => "no score yet".to_string(),
    };
    let ranks = match p.ranks {
        1 => "1 rank".to_string(),
        n => format!("{n} ranks"),
    };
    let practice = match p.practice_ranks {
        1 => "1 practice rank".to_string(),
        n => format!("{n} practice ranks"),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!("Records. Evening tuning: {best}, {ranks} tuned."),
        format!(
            "Tuned so far: {} in all, {} at the first try without a hint, and {practice}.",
            pipes_words(p.pipes as usize),
            p.perfect
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct TheEcho;

/// The game's title screen.
pub type Title = menus::Title<TheEcho>;

/// A fresh title screen.
pub fn title() -> Title {
    menus::Title::new(TheEcho)
}

impl TitleGame for TheEcho {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let label = match Records::best(ctx, ID, progress::RANK_BOARD) {
            Some(b) => format!("{}, evening tuning, best {b}", labels::PLAY),
            None => format!("{}, evening tuning", labels::PLAY),
        };
        vec![
            MenuItem::new(label, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Tuning::new(Mode::Rank))
            })
            .help(format!(
                "A rank of {}, each between {} and {} out of tune.",
                pipes_words(RANK.pipes),
                steps_words(RANK.near),
                steps_words(RANK.far)
            )),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice rank", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Tuning::new(Mode::Practice))
            })
            .help("Four pipes with free hints and no score: learn to hear the wobble."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best score, the pipes you have tuned, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Play the sounds you need in the cellar.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily, a weekly and a monthly rank, the same pipes for every player. {}",
            daily.resets()
        )
    }

    /// A player who has tuned a pipe is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }
}
