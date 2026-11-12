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
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

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
    blurb: "Easter: run the meadow lanes catching falling eggs in your basket, and dodge the mud.",
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
    vec![
        format!(
            "Records. The meadow: {meadow}, {} of {ROUNDS} rounds cleared. {endless}",
            progress.cleared.min(ROUNDS)
        ),
        Keepsakes::load(ctx).page(parlor_core::challenges::today(ctx)),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The Easter morning row: open on Easter, the bunny's news in the run-up,
/// locked with a countdown the rest of the year. It keeps its place on
/// every date.
fn easter_row(ctx: &mut Ctx<Parlor>) -> TitleRow<BasketRun> {
    let today = parlor_core::challenges::today(ctx);
    let help = easter::row_help(today, Records::best(ctx, ID, "easter morning"));
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
            .help("Rounds without end, faster each time, with no egg target: play until your baskets are gone. Every fifth cracked egg breaks a basket.")
        } else {
            MenuItem::new(
                labels::locked("Endless meadow"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say(format!(
                        "The endless meadow opens once you clear all {ROUNDS} rounds of the meadow."
                    ))
                },
            )
            .select_sound(None)
            .help(format!(
                "Clear all {ROUNDS} rounds of the meadow to open rounds without end."
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
                "{ROUNDS} rounds, from three lanes to five, with 3 baskets. Each round asks you to bank a share of its eggs before the bell. {} of {ROUNDS} cleared so far.",
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
            .help("Three slow lanes with every kind of drop, each explained the first time it falls. Nothing costs a basket and no score is saved."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best scores, the rounds you have cleared, your Easter keepsakes and your challenge results. Enter turns the page.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Each sound you need in the meadow, with its name.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "The daily twist, the weekly marathon and the monthly Easter festival: the same drops for every player. {}",
            daily.resets()
        )
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![menus::settings::toggle(
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
                    "Spoken call-outs: off. From your next run, a tenth more points on everything you bank.".to_string()
                })
            },
            "When on, a voice names each drop and its lane as it starts to fall. Off for a whole run, you earn a tenth more on everything you bank.",
        )]
    }

    /// A player who has read the tutorial or cleared a round is not new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let progress = Progress::load(ctx);
        !menus::tutorial_read(ctx, ID) && !progress.tutorial_seen && progress.cleared == 0
    }

    fn intro_help(&self) -> Option<String> {
        Some("Up and Down move, Enter chooses, Escape goes back to the parlor.".to_string())
    }
}
