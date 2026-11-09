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
//!
//! Past that, the long watch goes on until the dome falls, tech points
//! open unlocks that set the next colony up better, and on the fourth
//! Monday of the month the dust storm crosses the grid.

pub mod challenge;
pub mod colony;
pub mod events;
pub mod menus;
pub mod music;
pub mod night;
pub mod progress;
pub mod rules;
pub mod sound;
pub mod soundtrack;
pub mod tutorial;
pub mod unlocks;

use hearken::menu::MenuItem;
use hearken::screens::{ConfirmScreen, SettingsRow};
use hearken::Ctx;
use parlor_core::menus::{self as pm, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Rng};

pub use crate::challenge::{ChallengeHost, ChallengesMenu, Sky, CHALLENGES_NAME};
use crate::colony::{share, ColonyScreen};
use crate::progress::{more_records, nights_phrase, records_line, Progress};
use crate::rules::nights::RELIEF_NIGHT;
use crate::rules::Run;
pub use crate::unlocks::{Tech, UNLOCKS_NAME};

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
    event: Some(events::event),
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

/// The kinds of colony the title starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// Twelve nights to Relief.
    Colony,
    /// On past Relief until the dome falls.
    LongWatch,
    /// Twelve nights through the dust storm.
    Storm,
}

impl Start {
    /// Which kind a saved run is.
    pub fn of(run: &Run) -> Start {
        if run.storm.is_some() {
            Start::Storm
        } else if run.endless() {
            Start::LongWatch
        } else {
            Start::Colony
        }
    }

    /// "the colony", "the long watch", "the dust storm colony".
    pub fn the(self) -> &'static str {
        match self {
            Start::Colony => "the colony",
            Start::LongWatch => "the long watch",
            Start::Storm => "the dust storm colony",
        }
    }
}

/// Starts a new colony at night 1, replacing the current screen or on top
/// of it.
pub fn begin(ctx: &mut Ctx<Parlor>, seed: Option<u64>, replace: bool) {
    begin_as(ctx, Start::Colony, seed, replace);
}

/// Starts a new colony of `start`'s kind, set up by the player's unlocks.
pub fn begin_as(ctx: &mut Ctx<Parlor>, start: Start, seed: Option<u64>, replace: bool) {
    let seed = seed.unwrap_or_else(|| Rng::from_time().next_u64());
    let quiet = !Progress::load(ctx).call_outs;
    let perks = unlocks::perks(ctx, quiet);
    let run = match start {
        Start::Colony => Run::with_perks(seed, perks),
        Start::LongWatch => Run::long_watch(seed, perks),
        Start::Storm => Run::in_storm(seed, perks),
    };
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

/// A row that starts a colony of `start`'s kind, asking first when a
/// colony is saved.
fn start_row(label: String, start: Start, saved: bool, help: String) -> TitleRow<Dustfall> {
    MenuItem::new(label, move |s: &mut Title, ctx: &mut Ctx<Parlor>| {
        let seed = s.game.seed;
        if !saved {
            begin_as(ctx, start, seed, false);
            return;
        }
        ctx.push(
            ConfirmScreen::<Parlor>::new(
                format!("Start {}? The saved colony is lost.", start.the()),
                move |ctx| {
                    ctx.pop_quiet();
                    begin_as(ctx, start, seed, false);
                },
            )
            .yes_label(format!("Yes, start {}", start.the()))
            .no_label("No, keep the saved one"),
        )
    })
    .help(help)
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
        unlocks::sync(ctx);
        let p = Progress::load(ctx);
        let today = parlor_core::challenges::today(ctx);
        let saved = p.saved.is_some();
        let mut rows = Vec::new();
        if let Some(run) = p.saved {
            let night = run.night;
            let what = match Start::of(&run) {
                Start::Colony => String::new(),
                start => format!(" {}", start.the()),
            };
            rows.push(
                MenuItem::new(
                    format!("Continue{what}, night {night}, dome {}", run.colony.dome),
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
            rows.push(start_row(
                "New colony".to_string(),
                Start::Colony,
                true,
                "Starts again from night 1. The saved colony is lost.".to_string(),
            ));
        } else {
            rows.push(start_row(
                labels::PLAY.to_string(),
                Start::Colony,
                false,
                format!(
                    "A new colony. Hold {RELIEF_NIGHT} nights and Relief lands. The game saves between nights."
                ),
            ));
        }
        if unlocks::long_watch_open(ctx) {
            rows.push(start_row(
                "The long watch, past Relief".to_string(),
                Start::LongWatch,
                saved,
                "A colony that goes on past Relief until the dome falls, the nights harder each round. Its record is the most nights held.".to_string(),
            ));
        } else {
            rows.push(
                MenuItem::new(
                    labels::locked("The long watch"),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say("The long watch opens once Relief has landed.");
                    },
                )
                .select_sound(None)
                .help("Opens once a colony of yours has held to Relief: a colony that goes on past Relief until the dome falls."),
            );
        }
        if events::STORM.is_open(today) {
            rows.push(start_row(
                format!("{}, tonight", events::STORM_NAME),
                Start::Storm,
                saved,
                events::help(),
            ));
        } else {
            rows.push(
                MenuItem::new(
                    labels::locked(events::STORM_NAME),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::locked_line(today));
                    },
                )
                .select_sound(None)
                .help(events::locked_help(today)),
            );
        }
        rows
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        vec![parlor_core::unlocks::title_row::<Self, Tech>(ctx)]
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
            let more = more_records(ctx);
            let more = if more.is_empty() {
                more
            } else {
                format!(" {more}")
            };
            format!("{} {colonies} {reliefs}{more}", records_line(ctx))
        };
        vec![
            format!("Records. {colony_line}"),
            format!("Challenges. {}", challenges.join(" ")),
        ]
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        format!(
            "Your best score, the most nights held of {RELIEF_NIGHT}, how many colonies reached Relief, the long watch and dust storm records, your tech in all, and your challenge results."
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
            pm::settings::toggle(
                "dustfall_storm_wind",
                |on| format!("Storm wind: {}", if on { "on" } else { "off" }),
                |ctx| Progress::peek(ctx).storm_wind,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.storm_wind = on;
                    p.save(ctx);
                },
                |_ctx, _on| None,
                "Whether the dust storm's wind is heard at the column it covers. Off, the storm is only named as it moves.",
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
