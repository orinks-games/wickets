//! Larkfield: farm an empty river hamlet, and make a home for the people
//! who come to stay.
//!
//! A cozy simulation. You walk one lane from the gate to the woods,
//! digging and sowing the fields, fishing, chopping and building. Settlers
//! arrive at the gate, each with a past trade, two traits and a dream in
//! three steps. They choose their own work by what they are best at, and
//! their moods rise and fall with food, company, comfort and what happens
//! each morning. A low mood that lasts turns into a harmless wobble until
//! you talk them round; nobody is ever lost. Every settler plays an
//! instrument in the hamlet's tune, from wherever they are working.

pub mod almanac;
pub mod buildings;
pub mod challenge;
pub mod crops;
pub mod events;
pub mod menus;
pub mod music;
pub mod people;
pub mod rules;
pub mod save;
pub mod screen;
pub mod sound;
pub mod soundtrack;
pub mod talk;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::{ConfirmScreen, SettingsRow};
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self as pm, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Pages, Parlor, Period, Rng};

use crate::almanac::Pace;
use crate::challenge::{ChallengeHost, ChallengesMenu};
use crate::people::STORIES;
use crate::rules::{DayLength, Hamlet};
use crate::save::Progress;
use crate::screen::HamletScreen;

pub use crate::challenge::CHALLENGES_NAME;
pub use crate::screen::TITLE_NAME;

/// The game's id: its sounds live under `sounds/larkfield/`.
pub const ID: &str = "larkfield";

pub const HELP: &str = include_str!("../help.md");

pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Larkfield",
    category: Category::Cozy,
    blurb: "Farm an empty river hamlet, and make a home for the people who come to stay.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Larkfield as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Morning],
        seasons: &[parlor_core::Season::Spring, parlor_core::Season::Autumn],
    },
};

pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// The game on the shared title builder. A seed makes a new hamlet the
/// same every time, for tests.
#[derive(Default)]
pub struct Larkfield {
    pub seed: Option<u64>,
}

pub type Title = pm::Title<Larkfield>;

pub fn title() -> Title {
    pm::Title::new(Larkfield::default())
}

pub fn title_with_seed(seed: u64) -> Title {
    pm::Title::new(Larkfield { seed: Some(seed) })
}

/// Opens your hamlet, starting one if there is none.
pub fn play(ctx: &mut Ctx<Parlor>, seed: Option<u64>) {
    let p = Progress::load(ctx);
    let h = match p.hamlet {
        Some(h) => h,
        None => {
            let seed = seed.unwrap_or_else(|| Rng::from_time().next_u64());
            let mut h = Hamlet::home(seed);
            h.pace = p.pace;
            h
        }
    };
    ctx.push(HamletScreen::new(h, None));
}

/// The keepsake shelf, as pages.
pub fn shelf_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let kept = p.hamlet.map(|h| h.keepsakes).unwrap_or_default();
    if kept.is_empty() {
        return vec![format!(
            "The keepsake shelf is empty. Each settler who sees their dream come true leaves something here: {} to find.",
            STORIES.len()
        )];
    }
    let mut pages = vec![format!(
        "The keepsake shelf: {} of {}.",
        kept.len(),
        STORIES.len()
    )];
    for k in kept {
        pages.push(end_sentence(&rules::sentence_case(&k)));
    }
    pages
}

/// Records, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let hamlet = match &p.hamlet {
        Some(h) => {
            let n = |k: usize, one: &str, many: &str| {
                if k == 1 {
                    format!("1 {one}")
                } else {
                    format!("{k} {many}")
                }
            };
            format!(
                "Your hamlet: day {} of {}, year {}. {}, {} of 8 buildings, {}, {}.",
                h.day_of_season(),
                h.season().name(),
                h.year(),
                n(h.settlers.len(), "settler", "settlers"),
                h.built.len(),
                n(h.keepsakes.len(), "keepsake", "keepsakes"),
                n(h.festivals as usize, "festival held", "festivals held"),
            )
        }
        None => "No hamlet yet.".to_string(),
    };
    let years = match p.years {
        0 => "No full years yet.".to_string(),
        1 => "1 full year in Larkfield.".to_string(),
        n => format!("{n} full years in Larkfield."),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!("Records. {hamlet} {years}"),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

impl TitleGame for Larkfield {
    const ID: &'static str = ID;
    const NAME: &'static str = "Larkfield";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let (label, help) = match &p.hamlet {
            Some(h) => (
                format!(
                    "{}, day {} of {}, year {}",
                    pm::labels::PLAY,
                    h.day_of_season(),
                    h.season().name(),
                    h.year()
                ),
                format!(
                    "Back to your hamlet. {} {} live there.",
                    h.settlers.len(),
                    if h.settlers.len() == 1 { "settler" } else { "settlers" }
                ),
            ),
            None => (
                format!("{}, a new hamlet", pm::labels::PLAY),
                "Starts your hamlet with one settler, four dug plots and a few seeds. It is kept between visits.".to_string(),
            ),
        };
        vec![
            MenuItem::new(label, |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                play(ctx, s.game.seed)
            })
            .help(help),
        ]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your hamlet so far, the years you have kept it, and your challenge results.".to_string()
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
            "Market day, good spirits and founders: a small hamlet with a goal, the same for every player. {}",
            daily.resets()
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let kept = p.hamlet.as_ref().map_or(0, |h| h.keepsakes.len());
        vec![MenuItem::new(
            format!("Keepsake shelf, {kept} of {}", STORIES.len()),
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let pages = shelf_pages(ctx);
                ctx.push(Pages::new(pages));
            },
        )
        .help("What each settler left you when their dream came true.")]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![
            pm::settings::setting(
                "larkfield_pace",
                |ctx| format!("Pace: {}", Progress::peek(ctx).pace.name()),
                |ctx, delta| {
                    let mut p = Progress::load(ctx);
                    let i = Pace::ALL.iter().position(|&x| x == p.pace).unwrap_or(1) as i32;
                    p.pace = Pace::ALL[(i + delta).rem_euclid(Pace::ALL.len() as i32) as usize];
                    let about = p.pace.about();
                    p.save(ctx);
                    Some(format!("Pace: {}. {about}", p.pace.name()))
                },
                "How often visitors, troubles and storms come to your hamlet. Challenges are not changed.",
            ),
            pm::settings::setting(
                "larkfield_day_length",
                |ctx| format!("Day length: {}", Progress::peek(ctx).day_length.name()),
                |ctx, delta| {
                    let mut p = Progress::load(ctx);
                    let i = DayLength::ALL.iter().position(|&x| x == p.day_length).unwrap_or(1) as i32;
                    p.day_length = DayLength::ALL[(i + delta).rem_euclid(DayLength::ALL.len() as i32) as usize];
                    p.save(ctx);
                    None
                },
                "How long a day lasts from the morning to the supper bell. Longer days leave more time to walk and work.",
            ),
            pm::settings::toggle(
                "larkfield_band",
                |on| format!("Settlers' tune: {}", if on { "on" } else { "off" }),
                |ctx| Progress::peek(ctx).band,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.band = on;
                    p.save(ctx);
                },
                |_ctx, _on| None,
                "When on, every settler plays their instrument from where they are working. Off, only the birds and the river.",
            ),
            pm::settings::toggle(
                "larkfield_work_sounds",
                |on| format!("Work from along the lane: {}", if on { "on" } else { "off" }),
                |ctx| Progress::peek(ctx).work_sounds,
                |ctx, on| {
                    let mut p = Progress::load(ctx);
                    p.work_sounds = on;
                    p.save(ctx);
                },
                |_ctx, _on| None,
                "When on, settlers chopping, hammering and fishing play from wherever they are. Off, only work where you stand.",
            ),
        ]
    }

    fn destructive(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        vec![MenuItem::new("Start over", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
            ctx.push(
                ConfirmScreen::<Parlor>::new(
                    "Leave your hamlet and start a new one? Its settlers, buildings and keepsake shelf go. Settings and challenge results are kept.",
                    |ctx| {
                        let mut p = Progress::load(ctx);
                        p.hamlet = None;
                        p.save(ctx);
                        ctx.pop_with("Started over");
                    },
                )
                .yes_label("Yes, start a new hamlet")
                .no_label("No, keep my hamlet"),
            )
        })
        .help("Leaves your hamlet for a new, empty one. Settings and challenge results are kept.")]
    }

    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let p = Progress::load(ctx);
        !pm::tutorial_read(ctx, ID) && p.hamlet.is_none()
    }
}
