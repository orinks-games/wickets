//! Liberty Lights: Fourth of July fireworks: hear a show, then launch it back in time.
//!
//! A port of the Orinks Arcade NVGT game: twenty levels of fireworks
//! patterns to hear and recreate, scored on type, place and timing, plus a
//! freestyle mode. This version adds a count-in, combos, best scores per
//! level, a grand finale, saved freestyle shows to replay and take on, a
//! launch range to practise in and a sound check.

mod challenge_menu;
mod controls;
pub mod festival;
mod freestyle;
pub mod independence;
mod practice;
pub mod progress;
mod replay;
mod results;
pub mod rules;
pub mod show;
pub mod sound;
pub mod tutorial;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::{end_sentence, sentence_start};
use hearken::{Ctx, UiSound};
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge_menu::{briefing, ChallengeHost, ChallengesMenu, Shows, CHALLENGES_NAME};

pub use freestyle::{Freestyle, FreestyleMenu};
pub use independence::NightRun;
pub use practice::{Range, SoundCheck};
pub use progress::Progress;
pub use replay::Replay;
pub use show::{Mode, Show};

use rules::{level_size, tier_description, TOTAL_LEVELS};

/// The game's id: its sounds live under `sounds/liberty-lights/`.
pub const ID: &str = "liberty-lights";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a show returns to.
pub const TITLE_NAME: &str = "liberty_lights";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Liberty Lights",
    category: Category::Seasonal,
    blurb: "Fourth of July fireworks: hear a show, then launch it back in time.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(independence::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <LibertyLights as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Evening,
            parlor_core::story::PartOfDay::Night,
        ],
        seasons: &[parlor_core::Season::Summer],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its briefing, then its shows. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// A best score on its own, for lists that already say "best".
fn best_value(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    match Records::best(ctx, ID, board) {
        Some(b) => b.to_string(),
        None => "no score yet".to_string(),
    }
}

fn best_line(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    match Records::best(ctx, ID, board) {
        Some(b) => format!("best {b}"),
        None => "no score yet".to_string(),
    }
}

/// Records: every best score, then the challenges, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let total = best_value(ctx, progress::TOTAL_BOARD);
    let finale = if progress.finale_open() {
        format!("Grand finale: {}.", best_value(ctx, progress::FINALE_BOARD))
    } else {
        "Grand finale: locked until level 20 is cleared.".to_string()
    };
    let my_show = best_value(ctx, progress::MY_SHOW_BOARD);
    let mut pages = vec![format!(
        "Records. Run total: {total}. {finale} Your saved show: {my_show}. Longest combo: {}. Levels cleared: {} of {TOTAL_LEVELS}.",
        progress.best_combo,
        progress.cleared.len()
    )];
    for first in (1..=TOTAL_LEVELS).step_by(5) {
        let lines: Vec<String> = (first..first + 5)
            .map(|level| {
                let best = best_line(ctx, &progress::level_board(level));
                let cleared = if progress.is_cleared(level) {
                    "cleared"
                } else {
                    "not cleared"
                };
                format!("Level {level}: {cleared}, {best}.")
            })
            .collect();
        pages.push(lines.join(" "));
    }
    let today = parlor_core::challenges::today(ctx);
    pages.push(independence::Keepsakes::load(ctx).page(today));
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

/// The Independence night row: open on the Fourth of July, a rehearsal in
/// the week before, locked with a countdown the rest of the year. It keeps
/// its place on every date.
fn independence_row(ctx: &mut Ctx<Parlor>) -> TitleRow<LibertyLights> {
    let today = parlor_core::challenges::today(ctx);
    let help = independence::row_help(today, Records::best(ctx, ID, independence::BOARD));
    if independence::is_open(today) {
        let year = independence::year_of(today);
        MenuItem::new(
            independence::NAME,
            move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Show::new(Mode::Independence(Box::new(NightRun::new(year)))))
            },
        )
        .help(help)
    } else if independence::is_rehearsal(today) {
        MenuItem::new(
            format!("{}, rehearsal", independence::NAME),
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let salute = independence::salute();
                ctx.push(Replay::show(
                    "Rehearsal: the opening salute of Independence night.",
                    salute,
                ))
            },
        )
        .help(help)
    } else {
        MenuItem::new(
            labels::locked(independence::NAME),
            move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let today = parlor_core::challenges::today(ctx);
                ctx.play_ui(UiSound::Error);
                ctx.say(independence::locked_line(today))
            },
        )
        .select_sound(None)
        .help(help)
    }
}

/// The game on the shared title builder: its rows, pages and screens.
pub struct LibertyLights;

/// The game's title screen.
pub type Title = menus::Title<LibertyLights>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(LibertyLights)
}

impl TitleGame for LibertyLights {
    const ID: &'static str = ID;
    const NAME: &'static str = "Liberty Lights";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        let next = progress.next_level();
        let next_best = best_line(ctx, &progress::level_board(next));
        let finale_best = best_line(ctx, progress::FINALE_BOARD);
        let finale = if progress.finale_open() {
            MenuItem::new("Grand finale", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Show::new(Mode::Finale { round: 1, total: 0 }))
            })
            .help(format!(
                "An endless run of longer, faster, overlapping shows, until one scores under 60 percent. {}.",
                sentence_start(&finale_best)
            ))
        } else {
            MenuItem::new(
                labels::locked("Grand finale"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say("The grand finale opens once you clear level 20.")
                },
            )
            .select_sound(None)
            .help("Clear level 20 to open an endless run of ever faster shows.")
        };
        vec![
            MenuItem::new(
                format!("{}, level {next}", labels::PLAY),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Show::new(Mode::Level {
                        level: next,
                        total: 0,
                    }))
                },
            )
            .help(format!(
                "{} to match. {} {}.",
                match level_size(next) {
                    1 => "One firework".to_string(),
                    n => format!("{n} fireworks"),
                },
                tier_description(next),
                sentence_start(&next_best)
            )),
            finale,
            MenuItem::new("Freestyle show", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(FreestyleMenu::new())
            })
            .help("Launch your own fireworks without scoring, save your show, then match it, scored like a level."),
            MenuItem::new("Level select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let next = Progress::load(ctx).next_level();
                ctx.push(LevelSelect::new(next))
            })
            .help("Play any of the 20 levels."),
            independence_row(ctx),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice room", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Range::new())
            })
            .help("A launch range with nothing to score: fire freely, and each firework is named after it goes up."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best show points for every level, the best run, the grand finale, your Independence night keepsakes and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear every firework, place and sound of the game by name.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily show with a twist, a weekly five-show gauntlet and a monthly festival, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has cleared a level is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && Progress::load(ctx).cleared.is_empty()
    }
}

/// The twenty levels, any of which can be played.
pub struct LevelSelect {
    menu: MenuCore<Self, Parlor>,
}

impl LevelSelect {
    /// Opens on `level`.
    pub fn new(level: u32) -> Self {
        let mut menu = MenuCore::new("Level select");
        menu.index = level.clamp(1, TOTAL_LEVELS) as usize - 1;
        LevelSelect { menu }
    }
}

impl Menu for LevelSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "level_select"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    /// Opens on the current level, not the first.
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.play_ui(UiSound::Open);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = (1..=TOTAL_LEVELS)
            .map(|level| {
                let cleared = if progress.is_cleared(level) {
                    format!(
                        ", cleared, {}",
                        best_line(ctx, &progress::level_board(level))
                    )
                } else {
                    String::new()
                };
                let n = level_size(level);
                MenuItem::new(
                    format!("Level {level}{cleared}"),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.replace(Show::new(Mode::Level { level, total: 0 }))
                    },
                )
                .help(if n == 1 {
                    format!("{} 1 firework.", tier_description(level))
                } else {
                    format!("{} {n} fireworks.", tier_description(level))
                })
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Liberty Lights title."),
        );
        items
    }
}

mod visuals;
