//! Lamplighter: walk the Long Hall and light its lamps by ear.
//!
//! A story game: it opens once Mabel hands the keeper her old taper, on
//! opening night in story mode, and it has no lamp of its own in the hall
//! it lights. The hall is a row of steps with an alcove on each wall. Some
//! lamps are out, and each one whistles faintly from its wall. The player
//! walks with Up and Down and reaches with Left and Right to light them. In
//! the evening round a taper burns down as they go; light the hall on the
//! first taper for a bonus. Nothing fails: a burnt-out taper is replaced.

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

pub use challenge::{ChallengeHost, ChallengesMenu, Halls, CHALLENGES_NAME};
pub use play::{Mode, Round};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{lamps_words, time_words, ROUND};

/// The game's id: its sounds live under `sounds/lamplighter/`.
pub const ID: &str = "lamplighter";

/// The game's name.
pub const NAME: &str = "Lamplighter";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a round returns to.
pub const TITLE_NAME: &str = "lamplighter";

/// How the launcher lists the game, once the story opens it.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Relaxing,
    blurb: "Walk the Long Hall and light its lamps by ear.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: None,
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Lamplighter as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[],
        seasons: &[],
    },
};

/// Opens the current `period` challenge: its briefing, then the hall.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the evening round's bests, the tallies, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let best = match Records::best(ctx, ID, progress::ROUND_BOARD) {
        Some(b) => format!("best score {b}"),
        None => "no score yet".to_string(),
    };
    let fastest = match p.fastest {
        Some(t) => format!("fastest hall {}", time_words(t)),
        None => "no hall lit yet".to_string(),
    };
    let rounds = match p.rounds {
        1 => "1 evening round".to_string(),
        n => format!("{n} evening rounds"),
    };
    let practice = match p.practice_halls {
        1 => "1 practice hall".to_string(),
        n => format!("{n} practice halls"),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!("Records. Evening round: {best}, {fastest}."),
        format!(
            "Lit so far: {rounds}, {practice}, and {} in all.",
            lamps_words(p.lamps as usize)
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct Lamplighter;

/// The game's title screen.
pub type Title = menus::Title<Lamplighter>;

/// A fresh title screen.
pub fn title() -> Title {
    menus::Title::new(Lamplighter)
}

impl TitleGame for Lamplighter {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let best = match Records::best(ctx, ID, progress::ROUND_BOARD) {
            Some(b) => format!("{}, evening round, best {b}", labels::PLAY),
            None => format!("{}, evening round", labels::PLAY),
        };
        vec![
            MenuItem::new(best, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Round::new(Mode::Round))
            })
            .help(format!(
                "A hall of {} steps with {} out, and a taper that burns for {}.",
                ROUND.length,
                lamps_words(ROUND.out),
                time_words(ROUND.taper.unwrap_or(0.0) as u32)
            )),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice hall", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Round::new(Mode::Practice))
            })
            .help("A short hall with no taper and no score: learn the lamps by ear."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best score, fastest hall, lamps lit, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear each sound you need in the hall, with its name and what it means.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily, a weekly and a monthly hall, the same lamps for every player. {}",
            daily.resets()
        )
    }

    /// A player who has lit a lamp is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }
}
