//! Basket Run: an Easter lane game. Catch falling eggs in your basket by ear and dodge the mud.
//!
//! Easter in a meadow of lanes: eggs, golden eggs, mud clods and bees fall,
//! each with its own falling sound from its own lane. Catch the eggs in a
//! basket that only holds a few, bank them at the cart before the bell, and
//! dodge the rest. Eight rounds grow from three lanes to five and from a
//! gentle drop to a busy one; clearing them opens an endless meadow. Daily,
//! weekly and monthly challenges give everyone the same drops: a daily
//! twist, a weekly marathon and a monthly Easter festival of patterns.

pub mod challenge;
pub mod easter;
pub mod practice;
pub mod progress;
pub mod rules;
pub mod run;
pub mod sound;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{pace, Category, Challenge, Challenges, GameInfo, Pace, Parlor, Period};

use crate::challenge::{ChallengeHost, ChallengesMenu};
use crate::easter::Keepsakes;
use crate::practice::SoundCheck;
use crate::progress::{best_line, Progress};
use crate::rules::{Mode, ROUNDS};
use crate::run::{start, Launch};

pub use crate::challenge::CHALLENGES_NAME;
pub use crate::run::TITLE_NAME;

/// The game's id: its sounds live under `sounds/basket-run/`.
pub const ID: &str = "basket-run";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Basket Run",
    category: Category::Seasonal,
    blurb: "Run the Easter meadow lanes to catch falling eggs, bank them at the cart, and dodge mud clods and bees.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(easter::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <BasketRun as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Morning],
        seasons: &[parlor_core::Season::Spring],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its briefing, then the run. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// The game on the shared title builder. `seed` makes every run replay
/// the same drops, for tests.
#[derive(Default)]
pub struct BasketRun {
    pub seed: Option<u64>,
}

/// The game's title screen.
pub type Title = menus::Title<BasketRun>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(BasketRun::default())
}

/// A title whose runs replay the same drops every time, for tests.
pub fn title_with_seed(seed: u64) -> Title {
    menus::Title::new(BasketRun { seed: Some(seed) })
}

impl BasketRun {
    fn launch(&self, mode: Mode) -> Launch {
        Launch {
            mode,
            seed: self.seed,
        }
    }
}

/// Records: every best score and how far you have come, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let meadow = best_line(ctx, "meadow");
    let endless = if progress.endless_open() {
        format!("Endless meadow: {}.", best_line(ctx, "endless"))
    } else {
        "Endless meadow: locked until the meadow is cleared.".to_string()
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let challenge = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &challenge))
        })
        .collect();
    let mut pages = vec![
        format!(
            "Records. The meadow: {meadow}, {} of {ROUNDS} rounds cleared. {endless}",
            progress.cleared.min(ROUNDS)
        ),
        Keepsakes::load(ctx).page(parlor_core::challenges::today(ctx)),
    ];
    pages.extend(Keepsakes::load_at(ctx, Pace::Relaxed).relaxed_page());
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    pages
}

/// The Easter morning row: open on Easter, the bunny's news in the run-up,
/// locked with a countdown the rest of the year. It keeps its place on
/// every date.
fn easter_row(ctx: &mut Ctx<Parlor>) -> TitleRow<BasketRun> {
    let today = parlor_core::challenges::today(ctx);
    let bests = (
        pace::best(ctx, ID, "easter morning"),
        pace::best(ctx, ID, &Pace::Relaxed.board("easter morning")),
    );
    let help = easter::row_help(today, bests);
    if easter::is_open(today) {
        return MenuItem::new(easter::NAME, |s: &mut Title, ctx: &mut Ctx<Parlor>| {
            start(ctx, s.game.launch(Mode::Easter), false)
        })
        .help(help);
    }
    MenuItem::new(
        labels::locked(easter::NAME),
        |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
            let today = parlor_core::challenges::today(ctx);
            if easter::run_up(today).is_some() {
                ctx.audio.play(sound::CATCH_GOLDEN, hearken::Bus::UI);
            }
            ctx.say(easter::locked_line(today))
        },
    )
    .select_sound(None)
    .help(help)
}

impl TitleGame for BasketRun {
    const ID: &'static str = ID;
    const NAME: &'static str = "Basket Run";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        let meadow_best = best_line(ctx, "meadow");
        let endless = if progress.endless_open() {
            let best = best_line(ctx, "endless");
            MenuItem::new(
                format!("Endless meadow, {best}"),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    start(ctx, s.game.launch(Mode::Endless), false)
                },
            )
            .help("Endless rounds that get faster each time, with no egg target. Every fifth cracked egg breaks a basket, so play until your baskets run out.")
        } else {
            MenuItem::new(
                labels::locked("Endless meadow"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say(format!(
                        "Clear all {ROUNDS} rounds of the meadow to open the endless meadow."
                    ))
                },
            )
            .select_sound(None)
            .help(format!(
                "Clear all {ROUNDS} rounds of the meadow to unlock endless play."
            ))
        };
        vec![
            MenuItem::new(
                format!("{}, the meadow, {meadow_best}", labels::PLAY),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    start(ctx, s.game.launch(Mode::Meadow), false)
                },
            )
            .help(format!(
                "Play {ROUNDS} rounds across three to five lanes with 3 baskets, banking enough eggs before the bell. You have cleared {} of {ROUNDS} rounds so far.",
                progress.cleared.min(ROUNDS)
            )),
            endless,
            easter_row(ctx),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice lanes", |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, s.game.launch(Mode::Practice), false)
            })
            .help("Practice across three slow lanes with every kind of drop introduced as it first falls. Nothing costs a basket and no score is saved."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Check your best scores, cleared rounds, Easter keepsakes, and challenge results. Enter turns the page.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Preview every gameplay sound in the meadow by name.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "Take on the daily twist, weekly marathon, and monthly Easter festival, with the same drops for every player. {}",
            daily.resets()
        )
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![
            menus::settings::toggle(
            "basket_run_call_outs",
            labels::call_outs,
            |ctx| !Progress::peek(ctx).quiet_calls,
            |ctx, on| {
                let mut progress = Progress::load(ctx);
                progress.quiet_calls = !on;
                progress.save(ctx);
            },
            |_ctx, on| {
                Some(if on {
                    "Spoken call-outs: on. Each drop is called with its lane.".to_string()
                } else {
                    "Spoken call-outs: off. From your next run, everything you bank scores a tenth more.".to_string()
                })
            },
            "When on, a voice announces each drop and its lane as it starts to fall. Leave it off for a whole run to earn a tenth more on everything you bank.",
        ),
            pace::row(ID),
        ]
    }

    /// A player who has read the tutorial or cleared a round is not new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let progress = Progress::load(ctx);
        !menus::tutorial_read(ctx, ID) && !progress.tutorial_seen && progress.cleared == 0
    }

    fn intro_help(&self) -> Option<String> {
        Some(
            "Up and Down move through the menu, Enter chooses, and Escape returns to the parlor."
                .to_string(),
        )
    }
}
