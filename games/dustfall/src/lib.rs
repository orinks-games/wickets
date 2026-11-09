//! Dustfall: hold a dome through twelve nights of raiders, one turn at a
//! time.
//!
//! A turn-based defence game. Raiders come over the horizon onto a grid of
//! five columns and six rows and walk down toward the dome, finding their
//! own way round whatever stands in their path. You have a gun on a rail
//! along the dome's edge, a few missiles, and two actions a turn. Between
//! nights you spend the salvage on walls, flak posts, mines and the
//! buildings that keep the colony fed and powered. Hold twelve nights and
//! Relief lands.

pub mod challenge;
pub mod colony;
pub mod menus;
pub mod night;
pub mod progress;
pub mod rules;
pub mod sound;
pub mod soundtrack;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::{ConfirmScreen, SettingsRow};
use hearken::Ctx;
use parlor_core::menus::{self as pm, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Rng};

pub use crate::challenge::{ChallengeHost, ChallengesMenu, Sky, CHALLENGES_NAME};
use crate::colony::{share, ColonyScreen};
use crate::progress::{nights_phrase, records_line, Progress};
use crate::rules::nights::RELIEF_NIGHT;
use crate::rules::Run;

/// The game's id: its sounds live under `sounds/dustfall/`.
pub const ID: &str = "dustfall";

pub const HELP: &str = include_str!("../help.md");

/// The title's screen name.
pub const TITLE_NAME: &str = "dustfall";

pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Dustfall",
    category: Category::Puzzle,
    blurb: "Hold a dome through twelve nights of raiders, one turn at a time.",
    open: |ctx| ctx.push(title()),
    challenge: Some(challenge::open_challenge),
    event: None,
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Dustfall as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Night,
            parlor_core::story::PartOfDay::SmallHours,
        ],
        seasons: &[],
    },
};

/// The game on the shared title builder. A seed makes every night the
/// same, for tests.
#[derive(Default)]
pub struct Dustfall {
    pub seed: Option<u64>,
}

pub type Title = pm::Title<Dustfall>;

pub fn title() -> Title {
    pm::Title::new(Dustfall::default())
}

pub fn title_with_seed(seed: u64) -> Title {
    pm::Title::new(Dustfall { seed: Some(seed) })
}

/// Starts a new colony at night 1, replacing the current screen or on top
/// of it.
pub fn begin(ctx: &mut Ctx<Parlor>, seed: Option<u64>, replace: bool) {
    let seed = seed.unwrap_or_else(|| Rng::from_time().next_u64());
    let run = Run::new(seed);
    let mut p = Progress::load(ctx);
    p.colonies += 1;
    p.saved = Some(run.clone());
    p.save(ctx);
    let screen = ColonyScreen::new(share(run));
    if replace {
        ctx.replace(screen);
    } else {
        ctx.push(screen);
    }
}

/// Carries on the saved colony.
pub fn resume(ctx: &mut Ctx<Parlor>, run: Run) {
    ctx.push(ColonyScreen::new(share(run)));
}

impl TitleGame for Dustfall {
    const ID: &'static str = ID;
    const NAME: &'static str = "Dustfall";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let mut rows = Vec::new();
        if let Some(saved) = p.saved {
            let night = saved.night;
            rows.push(
                MenuItem::new(
                    format!("Continue, night {night}, dome {}", saved.colony.dome),
                    move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let p = Progress::load(ctx);
                        match p.saved {
                            Some(run) => resume(ctx, run),
                            None => begin(ctx, None, false),
                        }
                    },
                )
                .help(format!(
                    "Carries on your colony from the start of night {night}'s colony phase."
                )),
            );
            rows.push(
                MenuItem::new("New colony", |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let seed = s.game.seed;
                    ctx.push(
                        ConfirmScreen::<Parlor>::new(
                            "Start a new colony? The saved one is lost.",
                            move |ctx| {
                                ctx.pop_quiet();
                                begin(ctx, seed, false);
                            },
                        )
                        .yes_label("Yes, start a new colony")
                        .no_label("No, keep the saved one"),
                    )
                })
                .help("Starts again from night 1. The saved colony is lost."),
            );
        } else {
            rows.push(
                MenuItem::new(labels::PLAY, |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    begin(ctx, s.game.seed, false)
                })
                .help(format!(
                    "A new colony. Hold {RELIEF_NIGHT} nights and Relief lands. The game saves between nights."
                )),
            );
        }
        rows
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        let p = Progress::load(ctx);
        let colonies = match p.colonies {
            0 => "No colony started yet.".to_string(),
            1 => "1 colony started.".to_string(),
            n => format!("{n} colonies started."),
        };
        let reliefs = match p.reliefs {
            0 => "Relief has not landed yet.".to_string(),
            1 => "Relief landed once.".to_string(),
            n => format!("Relief landed {n} times."),
        };
        let challenges: Vec<String> = Period::ALL
            .into_iter()
            .map(|period| {
                let ch = Challenge::current(ctx, ID, period);
                Challenges::summary(ctx, &ch)
            })
            .collect();
        let colony_line = if p.colonies == 0 {
            colonies
        } else {
            format!("{} {colonies} {reliefs}", records_line(ctx))
        };
        vec![
            format!("Records. {colony_line}"),
            format!("Challenges. {}", challenges.join(" ")),
        ]
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        format!(
            "Your best score, the most nights held of {RELIEF_NIGHT}, how many colonies reached Relief, and your challenge results."
        )
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(crate::menus::SoundCheck::new());
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A night each day, a siege each week and a whole colony each month, the same sky for every player. {}",
            daily.resets()
        )
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![
            pm::settings::toggle(
                "dustfall_call_outs",
                labels::call_outs,
                |ctx| Progress::peek(ctx).call_outs,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.call_outs = on;
                    p.save(ctx);
                },
                |_ctx, on| {
                    Some(if on {
                        "Spoken call-outs: on. Every step is named."
                    } else {
                        "Spoken call-outs: off. Steps are only heard, and a raider at the dome is still named."
                    }
                    .to_string())
                },
                "When on, each raider's step is named with its column and row. Off, you hear the steps and only a raider reaching the dome is named.",
            ),
            pm::settings::toggle(
                "dustfall_confirm_end",
                |on| format!("Confirm end of turn: {}", if on { "on" } else { "off" }),
                |ctx| Progress::peek(ctx).confirm_end,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.confirm_end = on;
                    p.save(ctx);
                },
                |_ctx, _on| None,
                "When on, Enter asks before ending a turn, and a second Enter ends it.",
            ),
            pm::settings::toggle(
                "dustfall_scan_order",
                |on| {
                    format!(
                        "Scan order: {}",
                        if on { "left to right" } else { "nearest first" }
                    )
                },
                |ctx| Progress::peek(ctx).scan_left_to_right,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.scan_left_to_right = on;
                    p.save(ctx);
                },
                |_ctx, _on| None,
                "Which way Up and Down scan the raiders: nearest the dome first, or across the grid from the far left.",
            ),
        ]
    }

    fn destructive(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let Some(saved) = p.saved else {
            return Vec::new();
        };
        let night = saved.night;
        vec![MenuItem::new(
            "Abandon the saved colony",
            move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(
                    ConfirmScreen::<Parlor>::new(
                        format!(
                            "Abandon the colony saved at night {night}? Records and settings are kept."
                        ),
                        |ctx| {
                            Progress::forget(ctx);
                            ctx.pop_with("Colony abandoned");
                        },
                    )
                    .yes_label("Yes, abandon it")
                    .no_label("No, keep it"),
                )
            },
        )
        .help(format!(
            "Forgets the colony saved at night {night}. Records and settings are kept."
        ))]
    }

    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !pm::tutorial_read(ctx, ID) && Progress::load(ctx).colonies == 0
    }
}

/// "12 nights" for the title's lines.
pub fn relief_phrase() -> String {
    nights_phrase(RELIEF_NIGHT)
}
