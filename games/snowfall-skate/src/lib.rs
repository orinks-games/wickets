//! Snowfall Skate: a relaxing skate across a frozen pond at night, taking
//! lanterns as they light, while the snow comes down.
//!
//! A cozy Seasonal game for winter. Left and Right lean you across five
//! lanes; each lantern lights ahead in its lane, and you take it by being
//! there as it passes. Lanterns one after another build a line worth more
//! each. Snow settles on the ice as you skate and the night ends when it
//! covers it; each lantern keeps the ice clear a little longer. Drifts slow
//! you and thin ice cracks under you, and between lanterns you can spin.
//! Three ponds open with stars. The Frost Fair is on the winter solstice,
//! the lantern skate every evening, and the midnight skate on New Year's
//! Eve.

pub mod challenge;
pub mod events;
pub mod music;
pub mod picker;
pub mod play;
pub mod ponds;
pub mod progress;
pub mod rules;
pub mod sound;
pub mod soundcheck;
pub mod soundtrack;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::{ConfirmScreen, SettingsRow};
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::calendar::Holiday;
use parlor_core::menus::{self as pm, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

use crate::challenge::{ChallengeHost, ChallengesMenu};
use crate::picker::{best_words, skate};
use crate::play::Mode;
use crate::ponds::{pond, LANTERN_PAIRS, PONDS};
use crate::progress::{Progress, FAIR_BOARD, LANTERN_BOARD};
use crate::rules::{lanterns_words, points_words, stars_words};

pub use crate::challenge::CHALLENGES_NAME;

/// The game's id: its sounds live under `sounds/snowfall-skate/`.
pub const ID: &str = "snowfall-skate";

pub const NAME: &str = "Snowfall Skate";

pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a skate returns to.
pub const TITLE_NAME: &str = "snowfall_skate";

/// The holiday the game's own event falls on: the Frost Fair.
pub const HOLIDAY: Holiday = events::FAIR_DAY;

/// Where the launcher lists the game all year, once it is kept.
pub const HOME: Category = Category::Cozy;

/// The blurb in winter, until the game is kept.
pub const KEEP_BLURB: &str = "A winter special. Skate a frozen pond at night, taking lanterns as they light, while the snow comes down. Earn a star on the mill pond to keep the game all year.";

/// The blurb out of winter, once the game is kept.
pub const YEAR_ROUND_BLURB: &str = "Skate a frozen pond at night, taking lanterns as they light, while the snow comes down. The Frost Fair waits for winter.";

/// Said in the results when a skate first keeps the game.
pub const KEPT_LINE: &str =
    "Snowfall Skate is yours to keep. After the winter, it moves to the Cozy games.";

/// Whether the player has kept the game all year: a star on the mill pond,
/// which can only be skated while the game is listed in winter.
pub fn kept(ctx: &mut Ctx<Parlor>) -> bool {
    Progress::load(ctx).kept()
}

/// Whether `date` is in the game's season: December to February.
pub fn in_season(date: parlor_core::Date) -> bool {
    events::winter(date)
}

pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Seasonal,
    blurb: "A winter special. Skate a frozen pond at night, taking lanterns as they light, while the snow comes down.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <SnowfallSkate as TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Evening],
        seasons: &[parlor_core::Season::Winter],
    },
};

pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

fn call_outs_key() -> String {
    format!("{ID}/call_outs")
}

/// Whether the screen reader also names each lantern's lane as it lights.
pub fn call_outs(ctx: &mut Ctx<Parlor>) -> bool {
    Records::load(ctx, &call_outs_key())
}

fn call_outs_peek(ctx: &Ctx<Parlor>) -> bool {
    Records::peek(ctx, &call_outs_key())
}

fn set_call_outs(ctx: &mut Ctx<Parlor>, on: bool) {
    Records::store(ctx, &call_outs_key(), &on);
}

/// The game on the shared title builder. A seed makes every skate the
/// same, for tests.
#[derive(Default)]
pub struct SnowfallSkate {
    pub seed: Option<u64>,
}

pub type Title = pm::Title<SnowfallSkate>;

pub fn title() -> Title {
    pm::Title::new(SnowfallSkate::default())
}

pub fn title_with_seed(seed: u64) -> Title {
    pm::Title::new(SnowfallSkate { seed: Some(seed) })
}

fn board_best(ctx: &mut Ctx<Parlor>, board: &str) -> Option<u32> {
    Records::best(ctx, ID, board).map(|b| b as u32)
}

/// Records, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let mut ponds = Vec::new();
    for pd in &PONDS {
        if p.pond_open(pd.number) {
            ponds.push(format!(
                "{}: {}, {}.",
                pd.title(),
                stars_words(p.stars_for(pd.number)),
                best_words(ctx, pd.number)
            ));
        } else {
            ponds.push(format!("{}: locked.", pd.title()));
        }
    }
    let fair = match board_best(ctx, FAIR_BOARD) {
        Some(b) => format!("The Frost Fair: best {}.", points_words(b)),
        None => "The Frost Fair: not skated yet.".to_string(),
    };
    let ribbons = match p.ribbons.len() {
        1 => "1 Frost Fair ribbon".to_string(),
        n => format!("{n} Frost Fair ribbons"),
    };
    let lanterns = match board_best(ctx, LANTERN_BOARD) {
        Some(b) => format!(
            "Lantern skate: best {} of {LANTERN_PAIRS} in order, the whole tune {} times.",
            b, p.whole_tunes
        ),
        None => "Lantern skate: not skated yet.".to_string(),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!(
            "Records. {} in all. {} {fair} {lanterns}",
            stars_words(p.total_stars()),
            ponds.join(" ")
        ),
        format!(
            "{}. Lanterns taken: {}. Longest line: {}. Spins landed: {}. Drifts: {}. Cracks in the ice: {}. {}.",
            match p.skates {
                1 => "1 skate".to_string(),
                n => format!("{n} skates"),
            },
            p.lanterns,
            p.best_line,
            p.spins,
            p.drifts,
            p.cracks,
            hearken::text::sentence_start(&ribbons)
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

fn locked_row(label: &str, text: String, help: String) -> TitleRow<SnowfallSkate> {
    MenuItem::new(
        labels::locked(label),
        move |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.say(text.clone()),
    )
    .select_sound(None)
    .help(help)
}

impl TitleGame for SnowfallSkate {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let today = parlor_core::challenges::today(ctx);
        let now = parlor_core::story::now(ctx);
        let next = p.current_pond();
        let def = pond(next);
        let open = PONDS.iter().filter(|d| p.pond_open(d.number)).count();
        let mut rows = vec![
            MenuItem::new(
                format!(
                    "{}, {}, {}",
                    labels::PLAY,
                    def.title(),
                    stars_words(p.stars_for(next))
                ),
                move |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    skate(ctx, Mode::Pond(next), s.game.seed)
                },
            )
            .help(def.scene),
            MenuItem::new(
                format!(
                    "Choose a pond, {open} of 3 open, {}",
                    stars_words(p.total_stars())
                ),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(picker::Ponds::new(s.game.seed)),
            )
            .help("Every pond with its stars and best. A star on one opens the next."),
        ];
        // The lantern skate, every evening.
        let today_best = p.lantern_today(today);
        if events::dusk(now.hour) {
            let label = match today_best {
                Some(n) => format!("{}, {n} of {LANTERN_PAIRS} tonight", events::LANTERN_ROW),
                None => format!("{}, tonight's lanterns", events::LANTERN_ROW),
            };
            rows.push(
                MenuItem::new(label, move |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    skate(ctx, Mode::Lanterns(today), s.game.seed)
                })
                .help(format!(
                    "{} in pairs, the same for everyone tonight. Take the first lit of each pair.",
                    hearken::text::sentence_start(&lanterns_words(LANTERN_PAIRS * 2))
                )),
            );
        } else {
            rows.push(locked_row(
                events::LANTERN_ROW,
                "The lantern skate is lit every evening, from 4 in the afternoon to 9 at night, by your clock.".to_string(),
                "Lanterns in pairs, every evening by your clock.".to_string(),
            ));
        }
        // The Frost Fair.
        if p.fair_open(today) {
            let label = match board_best(ctx, FAIR_BOARD) {
                Some(b) => format!("{}, best {}", events::FAIR_ROW, points_words(b)),
                None => events::FAIR_ROW.to_string(),
            };
            rows.push(
                MenuItem::new(label, |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    skate(ctx, Mode::Fair, s.game.seed)
                })
                .help(format!(
                    "The long skate down the frozen river, past the stalls. {} on the day earns this winter's ribbon.",
                    hearken::text::sentence_start(&stars_words(1))
                )),
            );
        } else {
            rows.push(locked_row(
                events::FAIR_ROW,
                format!("The Frost Fair opens on {}.", events::fair_date_words()),
                "The river freezes once a winter for the fair, and you skate it for a ribbon."
                    .to_string(),
            ));
        }
        // The midnight skate, on New Year's Eve.
        if events::midnight_open(now) {
            rows.push(
                MenuItem::new(
                    events::MIDNIGHT_ROW,
                    |s: &mut Title, ctx: &mut Ctx<Parlor>| skate(ctx, Mode::Midnight, s.game.seed),
                )
                .help("Skate the village pond to midnight and the fireworks."),
            );
        } else {
            rows.push(locked_row(
                events::MIDNIGHT_ROW,
                "The midnight skate opens at 11 at night on New Year's Eve, by your clock, until 1 in the morning.".to_string(),
                "Skate to midnight on New Year's Eve.".to_string(),
            ));
        }
        rows.push(
            MenuItem::new("Practice skate", |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                skate(ctx, Mode::Practice, s.game.seed)
            })
            .help("The village pond, slowly, with no snow and nothing scored. Every lantern is named."),
        );
        rows
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your stars and bests on every pond, the Frost Fair and the lantern skate, your tallies, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(crate::soundcheck::SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear the lanterns, the drifts, the thin ice, your partner and the spin, each from where it is heard on the ice.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "Tonight's pond, thin ice week and the midwinter skate: the same ice for every player. {}",
            daily.resets()
        )
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![pm::settings::toggle(
            "snowfall_skate_call_outs",
            labels::call_outs,
            call_outs_peek,
            set_call_outs,
            |_ctx, on| {
                Some(
                    if on {
                        "Spoken call-outs: on. Your screen reader also names each lantern's lane."
                    } else {
                        "Spoken call-outs: off. You hear the lanterns alone."
                    }
                    .to_string(),
                )
            },
            "When on, your screen reader also names each lantern's lane as it lights.",
        )]
    }

    fn destructive(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        vec![MenuItem::new(
            "Start over",
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(
                    ConfirmScreen::<Parlor>::new(
                        "Clear your stars and ribbons, and start Snowfall Skate over? Best scores and settings are kept.",
                        |ctx| {
                            Progress::default().store(ctx);
                            ctx.pop_with("Started over");
                        },
                    )
                    .yes_label("Yes, start over")
                    .no_label("No, keep everything"),
                )
            },
        )
        .help("Clears your stars, ribbons and tallies. Best scores and challenges are kept.")]
    }

    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !pm::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }
}
