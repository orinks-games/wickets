//! Little Rapids: steer a little boat down a stream, catching what floats
//! along with the music.
//!
//! A cozy arcade game. The boat drifts downstream on its own; Left and
//! Right steer it smoothly across. Petals, leaves, paper scraps and rare
//! trinkets float down on the beat, and each catch plays a note in the
//! river's song, which grows a part at a time as the combo climbs. A bump
//! on a bank, a stone, a mill wheel or a duck costs the combo, never the
//! run. Four streams open with stars, scraps fold new boats, and trinkets
//! fill a keepsake shelf. The endless river runs until the boat sinks;
//! Drift has no score at all.

pub mod challenge;
pub mod events;
pub mod menus;
pub mod music;
pub mod progress;
pub mod rules;
pub mod run;
pub mod shelf;
pub mod sound;
pub mod soundtrack;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::{ConfirmScreen, SettingsRow};
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self as pm, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

use crate::challenge::{ChallengeHost, ChallengesMenu};
use crate::progress::{best_line, Progress, RIBBONS};
use crate::rules::{stars_phrase, CleanWidth, Mode, Stream};
use crate::run::{start, Launch};

pub use crate::challenge::CHALLENGES_NAME;
pub use crate::run::TITLE_NAME;

/// The game's id: its sounds live under `sounds/little-rapids/`.
pub const ID: &str = "little-rapids";

pub const HELP: &str = include_str!("../help.md");

pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Little Rapids",
    category: Category::Cozy,
    blurb: "Steer a little boat down a stream, catching what floats along with the music.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <LittleRapids as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Afternoon],
        seasons: &[parlor_core::Season::Spring, parlor_core::Season::Summer],
    },
};

pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// The game on the shared title builder. A seed makes every stream float
/// the same things down, for tests.
#[derive(Default)]
pub struct LittleRapids {
    pub seed: Option<u64>,
}

pub type Title = pm::Title<LittleRapids>;

pub fn title() -> Title {
    pm::Title::new(LittleRapids::default())
}

pub fn title_with_seed(seed: u64) -> Title {
    pm::Title::new(LittleRapids { seed: Some(seed) })
}

/// Records, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let mut streams = Vec::new();
    for s in Stream::ALL {
        if p.is_open(s) {
            let best = best_line(ctx, s.id());
            streams.push(format!(
                "{}: {}, {best}.",
                s.name(),
                stars_phrase(p.stars_for(s))
            ));
        } else {
            streams.push(format!("{}: locked.", s.name()));
        }
    }
    let endless = match Records::best(ctx, ID, "endless") {
        Some(m) => format!("The endless river: {m} metres at best."),
        None => "The endless river: not sailed yet.".to_string(),
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
            "Records. {} in all. {} {endless}",
            stars_phrase(p.total_stars()),
            streams.join(" ")
        ),
        format!(
            "Your boats: {} of 3. Paper scraps: {}. Trinkets on the shelf: {} of {}. Ribbons: {} of {}.",
            p.boats.len(),
            p.scraps,
            p.trinkets.len(),
            shelf::TRINKETS.len(),
            p.ribbons.len(),
            RIBBONS.len()
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

impl TitleGame for LittleRapids {
    const ID: &'static str = ID;
    const NAME: &'static str = "Little Rapids";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let open = p.open_streams().len();
        let endless = if p.endless_open() {
            let best = match Records::best(ctx, ID, "endless") {
                Some(m) => format!("best {m} metres"),
                None => "not sailed yet".to_string(),
            };
            MenuItem::new(
                format!("The endless river, {best}"),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    start(ctx, Launch { mode: Mode::Endless, seed: s.game.seed }, false)
                },
            )
            .help("A river with no end, made of every stream you have opened, busier as it goes. Each bump makes the boat soggier, and the fifth sinks it.")
        } else {
            MenuItem::new(
                labels::locked("The endless river"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say("The endless river opens with your first star on Garden brook.")
                },
            )
            .select_sound(None)
            .help("Reach the mouth of Garden brook to open a river with no end.")
        };
        vec![
            MenuItem::new(
                format!(
                    "{}, {} of 4 streams open, {}",
                    labels::PLAY,
                    open,
                    stars_phrase(p.total_stars())
                ),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(menus_streams(s.game.seed)),
            )
            .help("Choose a stream. Stars from one open the next."),
            endless,
            MenuItem::new("Drift", |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, Launch { mode: Mode::Drift, seed: s.game.seed }, false)
            })
            .help("Slow water and the whole band from the start. No score, and the banks only nudge you."),
        ]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your stars and bests on every stream, your boats, scraps, trinkets and ribbons, and your challenge results.".to_string()
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
            "Today's stream, the boat of the week and the long run: the same water for every player. {}",
            daily.resets()
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        vec![
            MenuItem::new(
                format!("Boatyard, {} paper scraps", p.scraps),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(crate::menus::Boatyard::new()),
            )
            .help("Fold new boats from paper scraps, and choose which one to sail."),
            MenuItem::new(
                format!(
                    "Keepsake shelf, {} of {} trinkets",
                    p.trinkets.len(),
                    shelf::TRINKETS.len()
                ),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(crate::shelf::Shelf::new()),
            )
            .help("Every trinket you have fished out of the streams, with a line about each."),
        ]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![
            pm::settings::setting(
                "little_rapids_clean_width",
                |ctx| format!("Clean catch width: {}", Progress::peek(ctx).clean_width().name()),
                |ctx, delta| {
                    let mut p = Progress::load(ctx);
                    let i = CleanWidth::ALL
                        .iter()
                        .position(|&w| w == p.clean_width())
                        .unwrap_or(1) as i32;
                    let n = (i + delta).rem_euclid(CleanWidth::ALL.len() as i32) as usize;
                    p.clean_width = CleanWidth::ALL[n].id();
                    p.save(ctx);
                    None
                },
                "How close to dead centre a catch must be for a clean catch and double points.",
            ),
            pm::settings::toggle(
                "little_rapids_gentle",
                |on| format!("Gentle streams: {}", if on { "on" } else { "off" }),
                |ctx| Progress::peek(ctx).gentle,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.gentle = on;
                    p.save(ctx);
                },
                |_ctx, on| {
                    Some(if on {
                        "Gentle streams: on. Fewer things float down, and the current swings less."
                    } else {
                        "Gentle streams: off."
                    }
                    .to_string())
                },
                "When on, every stream and the endless river stay at their easiest. Challenges are not changed.",
            ),
            pm::settings::toggle(
                "little_rapids_trinket_names",
                |on| format!("Say trinket names: {}", if on { "on" } else { "off" }),
                |ctx| Progress::peek(ctx).trinket_names,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.trinket_names = on;
                    p.save(ctx);
                },
                |_ctx, _on| None,
                "When on, a trinket's name is said as you catch it. Off, the results name them.",
            ),
            pm::settings::toggle(
                "little_rapids_bank_lapping",
                |on| format!("Banks louder up close: {}", if on { "on" } else { "off" }),
                |ctx| Progress::peek(ctx).bank_lapping,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.bank_lapping = on;
                    p.save(ctx);
                },
                |_ctx, _on| None,
                "When on, the lapping at each bank gets louder as you near it. Off, both banks stay soft.",
            ),
        ]
    }

    fn destructive(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        vec![MenuItem::new(
            "Start over",
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(
                    ConfirmScreen::<Parlor>::new(
                        "Clear your stars, boats, scraps, ribbons and shelf, and start Little Rapids over? Best scores and settings are kept.",
                        |ctx| {
                            let old = Progress::load(ctx);
                            let fresh = Progress {
                                clean_width: old.clean_width,
                                trinket_names: old.trinket_names,
                                bank_lapping: old.bank_lapping,
                                gentle: old.gentle,
                                tutorial_seen: old.tutorial_seen,
                                ..Progress::default()
                            };
                            fresh.save(ctx);
                            ctx.pop_with("Started over");
                        },
                    )
                    .yes_label("Yes, start over")
                    .no_label("No, keep everything"),
                )
            },
        )
        .help("Clears your stars, boats, scraps, ribbons and shelf. Best scores and challenges are kept.")]
    }

    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let p = Progress::load(ctx);
        !pm::tutorial_read(ctx, ID) && !p.tutorial_seen && p.total_stars() == 0
    }
}

fn menus_streams(seed: Option<u64>) -> crate::menus::Streams {
    crate::menus::Streams::new(seed)
}
