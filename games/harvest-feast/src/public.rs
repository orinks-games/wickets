// Harvest Feast: Thanksgiving: run a busy kitchen, stirring, flipping and pulling dishes at the right moment.
//
// Up to five stations sit in an arc around you, each dish with its own
// cooking sound that changes when it needs you. The recipe calls each dish
// in order; you move between stations and act at the right moment. Eight
// courses build to the full feast, then Second helpings runs on endlessly.

pub mod challenge;
pub mod kitchen;
mod practice;
pub mod progress;
pub mod results;
pub mod rules;
pub mod sound;
pub mod thanksgiving;
pub mod tutorial;

use hearken::menu::{Label, Menu, MenuCore, MenuItem};
use hearken::screens::SettingsRow;
use hearken::text::{end_sentence, sentence_start};
use hearken::{Ctx, UiSound};
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{pace, Category, Challenge, Challenges, GameInfo, Pace, Parlor, Period};

pub use challenge::{
    ChallengeHost, ChallengePlan, ChallengeRun, ChallengesMenu, Dinners, Twist, CHALLENGES_NAME,
};
pub use kitchen::{Kitchen, Mode, Phase};
pub use practice::SoundCheck;
pub use progress::Progress;

use rules::{course, stars_phrase, COURSES};

/// The game's id: its sounds live under `sounds/harvest-feast/`.
pub const ID: &str = "harvest-feast";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a meal returns to.
pub const TITLE_NAME: &str = "harvest_feast";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Harvest Feast",
    category: Category::Seasonal,
    blurb: "Thanksgiving: run a busy kitchen, stirring, flipping and pulling dishes at the right moment.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(thanksgiving::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <HarvestFeast as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Afternoon, parlor_core::story::PartOfDay::Evening],
        seasons: &[parlor_core::Season::Autumn],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// The bests on `base`'s boards: "best 1200, relaxed best 900", or only
/// the one that has a score, or "no score yet".
fn best_line(ctx: &mut Ctx<Parlor>, base: &str) -> String {
    let bests: Vec<String> = [Pace::Normal, Pace::Relaxed]
        .into_iter()
        .filter_map(|pace| {
            parlor_core::pace::best(ctx, ID, &pace.board(base))
                .map(|b| format!("{} {b}", pace.best_word()))
        })
        .collect();
    if bests.is_empty() {
        "no score yet".to_string()
    } else {
        bests.join(", ")
    }
}

/// Records: every best score, then the challenges, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    progress::migrate_pace(ctx);
    let progress = Progress::load(ctx);
    let helpings = if progress.helpings_open() {
        format!(
            "Second helpings: {}, most rounds {}.",
            best_line(ctx, progress::HELPINGS_BOARD),
            progress.best_rounds
        )
    } else {
        "Second helpings: locked until the full feast is served.".to_string()
    };
    let mut pages = vec![format!(
        "Records. {} of {} stars earned. {helpings}",
        progress.total_stars(),
        COURSES * 3
    )];
    for first in [1, 5] {
        let lines: Vec<String> = (first..first + 4)
            .map(|n| {
                let best = best_line(ctx, &progress::course_board(n));
                format!(
                    "Course {n}, {}: {}, {best}.",
                    course(n).name,
                    stars_phrase(progress.stars(n))
                )
            })
            .collect();
        pages.push(lines.join(" "));
    }
    let today = parlor_core::challenges::today(ctx);
    pages.push(thanksgiving::Keepsakes::load(ctx).page(today));
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    pages
}

/// The Thanksgiving dinner row: open on the day, the year's menu in the
/// run-up, locked with a countdown the rest of the year. It keeps its
/// place on every date.
fn thanksgiving_row(ctx: &mut Ctx<Parlor>) -> TitleRow<HarvestFeast> {
    let today = parlor_core::challenges::today(ctx);
    let best = pace::best(ctx, ID, thanksgiving::BOARD);
    let relaxed = pace::best(ctx, ID, &Pace::Relaxed.board(thanksgiving::BOARD));
    let help = thanksgiving::row_help(today, best, relaxed);
    if thanksgiving::is_open(today) {
        let year = thanksgiving::year_of(today);
        MenuItem::new(
            thanksgiving::NAME,
            move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Kitchen::new(Mode::Thanksgiving { year }))
            },
        )
        .help(help)
    } else {
        MenuItem::new(
            labels::locked(thanksgiving::NAME),
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let today = parlor_core::challenges::today(ctx);
                if thanksgiving::is_run_up(today) {
                    ctx.audio.play(sound::RECIPE, hearken::Bus::UI);
                }
                ctx.say(thanksgiving::locked_line(today))
            },
        )
        .select_sound(None)
        .help(help)
    }
}

/// "1 dish" or "3 dishes".
pub(crate) fn dishes(n: usize) -> String {
    if n == 1 {
        "1 dish".to_string()
    } else {
        format!("{n} dishes")
    }
}

fn course_help(ctx: &mut Ctx<Parlor>, n: u32) -> String {
    let progress = Progress::load(ctx);
    let c = course(n);
    let best = best_line(ctx, &progress::course_board(n));
    format!(
        "{} {}. {}, {best}.",
        c.blurb,
        dishes(c.plan.len()),
        sentence_start(&stars_phrase(progress.stars(n)))
    )
}

/// The game on the shared title builder: its rows, pages and screens.
pub struct HarvestFeast;

/// The game's title screen.
pub type Title = menus::Title<HarvestFeast>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(HarvestFeast)
}

impl TitleGame for HarvestFeast {
    const ID: &'static str = ID;
    const NAME: &'static str = "Harvest Feast";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        progress::migrate_pace(ctx);
        let progress = Progress::load(ctx);
        let next = progress.next_course();
        let next_help = course_help(ctx, next);
        let helpings = if progress.helpings_open() {
            let best = best_line(ctx, progress::HELPINGS_BOARD);
            MenuItem::new(
                "Second helpings",
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Kitchen::new(Mode::Helpings { round: 1, total: 0 }))
                },
            )
            .help(format!(
                "An endless run of random meals, faster every round, until one earns no stars. {}.",
                sentence_start(&best)
            ))
        } else {
            MenuItem::new(
                labels::locked("Second helpings"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say("Second helpings opens once you serve the full feast, course 8.")
                },
            )
            .select_sound(None)
            .help("Serve course 8, the full feast, to open an endless run of random meals.")
        };
        vec![
            MenuItem::new(
                format!("{}, course {next}: {}", labels::PLAY, course(next).name),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Kitchen::new(Mode::Course(next)))
                },
            )
            .help(next_help),
            helpings,
            MenuItem::new(
                "Course select",
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(CourseSelect::new(next)),
            )
            .help("Cook any course you have opened."),
            thanksgiving_row(ctx),
        ]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![menus::settings::toggle(
            "harvest_feast_call_outs",
            labels::call_outs,
            |ctx| Progress::peek(ctx).callouts,
            set_callouts,
            |_ctx, _on| None,
            "When on, the kitchen says which dish needs you as its sound changes. Cook a whole course with it off for 10 percent more points.",
        ),
            parlor_core::pace::row_with(ID, progress::pace),
        ]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your stars and best points for every course, second helpings, your Thanksgiving keepsakes and your challenge results."
            .to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear every station, cue and stinger of the kitchen by name.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "The daily dinner, the weekly three-course Sunday dinner and the monthly harvest banquet, the same menus for every player. {}",
            daily.resets()
        )
    }

    /// A player who has served a course is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && Progress::load(ctx).total_stars() == 0
    }
}

/// Saves the call-outs, from their Settings row.
fn set_callouts(ctx: &mut Ctx<Parlor>, on: bool) {
    let mut p = Progress::load(ctx);
    p.callouts = on;
    p.store(ctx);
}

/// Every course, any of which can be cooked once opened.
pub struct CourseSelect {
    menu: MenuCore<Self, Parlor>,
}

impl CourseSelect {
    /// Opens on course `n`.
    pub fn new(n: u32) -> Self {
        let mut menu = MenuCore::new("Course select");
        menu.index = n.clamp(1, COURSES) as usize - 1;
        CourseSelect { menu }
    }
}

impl Menu for CourseSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "harvest_feast_courses"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    /// Opens on the current course, not the first.
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.play_ui(UiSound::Open);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        progress::migrate_pace(ctx);
        let progress = Progress::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = (1..=COURSES)
            .map(|n| {
                let c = course(n);
                if progress.open(n) {
                    let best = best_line(ctx, &progress::course_board(n));
                    MenuItem::new(
                        Label::from(format!(
                            "Course {n}: {}, {}, {best}",
                            c.name,
                            stars_phrase(progress.stars(n))
                        )),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.replace(Kitchen::new(Mode::Course(n)))
                        },
                    )
                    .help(format!("{} {}.", c.blurb, dishes(c.plan.len())))
                } else {
                    MenuItem::new(
                        format!("Course {n}: {}, locked", c.name),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.say(format!(
                                "Serve course {} with at least one star to open this one.",
                                n - 1
                            ))
                        },
                    )
                    .select_sound(None)
                    .help(c.blurb)
                }
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Harvest Feast title."),
        );
        items
    }
}
