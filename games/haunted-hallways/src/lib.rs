//! Nevermore Manor (once Haunted Hallways): Halloween. Find your way down
//! a haunted house of a hundred floors by sound, and freeze when the ghost
//! comes near.
//!
//! A house of ten wings of ten floors, and endless floors below them. Each
//! floor is generated from its number: a chain of rooms walked cell by
//! cell, one sound puzzle per room (find, match, riddle, offering, missing
//! sound, séance), things that call out from where they lie, an
//! inventory, ghosts you must freeze for, a hound you must walk past,
//! rotten boards, a hunting shadow, mimics, dark and mirrored rooms,
//! twists, and a haunt clock per floor. Floors open as you clear them;
//! Nightmare, Endless night, curses, diary pages and more are unlocked by
//! playing.
//!
//! The crate, its id and its sound folder keep the old name, so saves and
//! recordings carry over.

pub mod allhallows;
pub mod challenge;
pub mod haunt;
pub mod progress;
pub mod rules;
pub mod sound;
mod soundcheck;
pub mod tutorial;
pub mod unlocks;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::unlocks::Unlocks;
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Hauntings, CHALLENGES_NAME};
pub use haunt::Haunt;
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use progress::NIGHTMARE_AT;
use rules::{floor_spec, time_short, wing_first, Mode, FLOORS, WINGS, WING_FLOORS};

/// The game's id: its sounds live under `sounds/haunted-hallways/`. It
/// keeps the game's first name, so saves carry over.
pub const ID: &str = "haunted-hallways";

/// The game's name, as players hear it.
pub const NAME: &str = "Nevermore Manor";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a haunt returns to.
pub const TITLE_NAME: &str = "haunted_hallways";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Seasonal,
    blurb: "Halloween: find your way down a haunted house of a hundred floors by sound, and freeze when the ghost comes near.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(allhallows::event),
    event_done: Some(allhallows::event_done),
    tutorial: crate::tutorial::lesson,
    settings: <HauntedHallways as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Night, parlor_core::story::PartOfDay::SmallHours],
        seasons: &[parlor_core::Season::Autumn],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

fn best_score(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    match Records::best(ctx, ID, board) {
        Some(b) => format!("best score {b}"),
        None => "no score yet".to_string(),
    }
}

/// Records: how deep you have gone, best times and scores, and the
/// challenges, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let climb = best_score(ctx, "climb");
    let nightmare = if progress.nightmare_open() {
        format!(
            "Nightmare: deepest floor cleared {}, {}.",
            progress.nightmare_deepest,
            best_score(ctx, "nightmare")
        )
    } else {
        format!("Nightmare: locked until you clear floor {NIGHTMARE_AT}.")
    };
    let endless = if progress.endless_open() {
        format!(
            "Endless night: deepest floor cleared {}, {}.",
            progress.endless_deepest,
            best_score(ctx, "endless")
        )
    } else {
        format!("Endless night: locked until you clear floor {FLOORS}.")
    };
    let unlocks = unlocks::list();
    let earned = Unlocks::count(ctx, ID, &unlocks);
    let mut pages = vec![format!(
        "Records. Highest floor cleared: {} of {FLOORS}. Play, {climb}. {nightmare} {endless} Floors cleared in all: {}. Ghosts evaded in all: {}. Unlocks: {earned} of {}.",
        progress.cleared,
        progress.floors_cleared,
        progress.ghosts_evaded,
        unlocks.len()
    )];
    for (i, w) in WINGS.iter().enumerate().take(10) {
        let first = wing_first(i);
        let last = first + WING_FLOORS - 1;
        if first > progress.open_floors() {
            pages.push(format!("Floors {first} to {last}, {}: locked.", w.name));
            continue;
        }
        let floors: Vec<String> = (first..=last)
            .filter(|&n| n <= progress.open_floors())
            .map(|n| match progress.floor_time(n) {
                Some(t) => format!("floor {n}, {}", time_short(t)),
                None => format!("floor {n}, not yet cleared"),
            })
            .collect();
        pages.push(format!(
            "Floors {first} to {last}, {}: {}.",
            w.name,
            floors.join("; ")
        ));
    }
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    pages.push(allhallows::Ledger::load(ctx).page());
    pages
}

/// The Halloween rows, on every date so they never move: the run-up floor
/// while the house stirs (17 October to 1 November) and candles are left
/// to light, and the Lantern Night, locked until 30 October. Out of
/// season both are locked and say when they open.
fn halloween_rows(ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<HauntedHallways>> {
    let today = parlor_core::challenges::today(ctx);
    let year = allhallows::year_of(today);
    let lit = allhallows::Ledger::load(ctx).year(year).candles;
    let mut rows = Vec::new();
    if let Some((label, help, floor)) = allhallows::stirs_row(today, lit) {
        rows.push(
            MenuItem::new(label, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let h = Haunt::start(ctx, Mode::Stirs(floor));
                ctx.push(h)
            })
            .help(help),
        );
    } else if allhallows::stirs(today) {
        // Every candle lit: the row stays, and says so.
        rows.push(
            MenuItem::new(
                allhallows::stirs_done_label(),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.say(allhallows::stirs_done_line(today))
                },
            )
            .select_sound(None)
            .help(allhallows::stirs_done_line(today)),
        );
    } else {
        rows.push(
            MenuItem::new(
                labels::locked(allhallows::STIRS),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.say(allhallows::stirs_locked_line(today))
                },
            )
            .select_sound(None)
            .help(allhallows::stirs_locked_help(today)),
        );
    }
    let help = allhallows::night_help(today, lit);
    if allhallows::is_night(today) {
        rows.push(
            MenuItem::new(
                allhallows::NIGHT,
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let h = Haunt::start(ctx, Mode::LanternNight(lit));
                    ctx.push(h)
                },
            )
            .help(help),
        );
    } else {
        rows.push(
            MenuItem::new(
                labels::locked(allhallows::NIGHT),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.say(allhallows::locked_line(today))
                },
            )
            .select_sound(None)
            .help(help),
        );
    }
    rows
}

/// The game on the shared title builder: its rows, pages and screens.
pub struct HauntedHallways;

/// The game's title screen.
pub type Title = menus::Title<HauntedHallways>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(HauntedHallways)
}

impl TitleGame for HauntedHallways {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        unlocks::sync(ctx);
        let progress = Progress::load(ctx);
        let from = progress.frontier();
        let curses = progress.curses().count();
        let cursed = match curses {
            0 => String::new(),
            1 => " 1 curse on.".to_string(),
            n => format!(" {n} curses on."),
        };
        let play_help = format!(
            "Climb from floor {from}, your highest open floor, as far as you can. {}.{cursed}",
            sentence_start(&best_score(ctx, "climb"))
        );
        let nightmare = if progress.nightmare_open() {
            let help = format!(
                "The whole house from floor 1 with less time, faster ghosts and courage that lasts the run. Deepest floor cleared {}, {}.",
                progress.nightmare_deepest,
                best_score(ctx, "nightmare")
            );
            MenuItem::new("Nightmare", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let h = Haunt::start(ctx, Mode::Nightmare);
                ctx.push(h)
            })
            .help(help)
        } else {
            MenuItem::new(
                labels::locked("Nightmare"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say(format!(
                        "Nightmare opens once you clear floor {NIGHTMARE_AT}."
                    ))
                },
            )
            .select_sound(None)
            .help(format!("Clear floor {NIGHTMARE_AT} to open Nightmare."))
        };
        let endless = if progress.endless_open() {
            let help = format!(
                "The floors below floor {FLOORS}, without end. Deepest floor cleared {}, {}.",
                progress.endless_deepest.max(FLOORS),
                best_score(ctx, "endless")
            );
            MenuItem::new("Endless night", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let h = Haunt::start(ctx, Mode::Endless);
                ctx.push(h)
            })
            .help(help)
        } else {
            MenuItem::new(
                labels::locked("Endless night"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say(format!(
                        "Endless night opens once you clear floor {FLOORS}."
                    ))
                },
            )
            .select_sound(None)
            .help(format!("Clear floor {FLOORS} to open Endless night."))
        };
        let spec = floor_spec(from, false);
        let halloween = halloween_rows(ctx);
        let mut rows = vec![
            MenuItem::new(
                format!("{}, floor {from}: {}", labels::PLAY, spec.name),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let h = Haunt::climb(ctx);
                    ctx.push(h)
                },
            )
            .help(play_help),
            MenuItem::new("Floor select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(WingSelect::new())
            })
            .help(format!(
                "Replay any floor you have reached for its best time. {} of {FLOORS} open.",
                progress.open_floors()
            )),
            nightmare,
            endless,
        ];
        rows.extend(halloween);
        rows
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "How deep you have gone, your best times and scores, and your challenge results."
            .to_string()
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        vec![parlor_core::unlocks::title_row::<Self, unlocks::Keepsakes>(
            ctx,
        )]
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily floor, a weekly climb and a monthly masked ball, the same haunted house for every player. {}",
            daily.resets()
        )
    }

    /// A player who has cleared a floor is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let progress = Progress::load(ctx);
        !menus::tutorial_read(ctx, ID) && progress.cleared == 0
    }
}

/// Floor select, first step: choose a wing of ten floors.
pub struct WingSelect {
    menu: MenuCore<Self, Parlor>,
}

impl WingSelect {
    pub fn new() -> Self {
        WingSelect {
            menu: MenuCore::new("Floor select").with_intro_help(
                "Up and Down choose a wing of ten floors, Enter lists its floors, Escape goes back.",
            ),
        }
    }
}

impl Default for WingSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for WingSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "haunted_hallways_wings"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let open = progress.open_floors();
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for (i, w) in WINGS.iter().enumerate().take(10) {
            let first = wing_first(i);
            let last = first + WING_FLOORS - 1;
            if first <= open {
                let cleared = (first..=last).filter(|&n| n <= progress.cleared).count();
                items.push(
                    MenuItem::new(
                        format!(
                            "Floors {first} to {last}, {}, {cleared} of 10 cleared",
                            w.name
                        ),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.push(FloorSelect::new(i)),
                    )
                    .help(format!("The {} floors of {}.", WING_FLOORS, w.name)),
                );
            } else {
                items.push(
                    MenuItem::new(
                        labels::locked(&format!("Floors {first} to {last}")),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.say(format!("Clear floor {} to open floor {first}.", first - 1))
                        },
                    )
                    .select_sound(None)
                    .help(format!("Clear floor {} to open floor {first}.", first - 1)),
                );
            }
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help(format!("Return to the {NAME} title.")),
        );
        items
    }
}

/// Floor select, second step: one floor of a wing, to play on its own.
pub struct FloorSelect {
    wing: usize,
    menu: MenuCore<Self, Parlor>,
}

impl FloorSelect {
    pub fn new(wing: usize) -> Self {
        let w = &WINGS[wing.min(9)];
        FloorSelect {
            wing: wing.min(9),
            menu: MenuCore::new(sentence_start(w.name)).with_intro_help(
                "Up and Down choose a floor, Enter plays it on its own, Escape goes back.",
            ),
        }
    }
}

impl Menu for FloorSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "haunted_hallways_floors"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let open = progress.open_floors();
        let first = wing_first(self.wing);
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for n in first..first + WING_FLOORS {
            let spec = floor_spec(n, false);
            if n <= open {
                let time = match progress.floor_time(n) {
                    Some(t) => format!("best {}", time_short(t)),
                    None => "not cleared".to_string(),
                };
                let twists = if spec.twists.is_empty() {
                    String::new()
                } else {
                    let names: Vec<&str> = spec.twists.iter().map(|t| t.name()).collect();
                    format!(" Twists: {}.", names.join(", "))
                };
                let help = format!(
                    "{} rooms, {} on the haunt clock.{twists} {}.",
                    spec.rooms,
                    rules::clock_words(spec.haunt),
                    sentence_start(&best_score(ctx, &format!("floor {n}")))
                );
                items.push(
                    MenuItem::new(
                        format!("Floor {n}, {time}"),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            let h = Haunt::start(ctx, Mode::Floor(n));
                            ctx.push(h)
                        },
                    )
                    .help(help),
                );
            } else {
                items.push(
                    MenuItem::new(
                        labels::locked(&format!("Floor {n}")),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.say(format!("Clear floor {} to open floor {n}.", n - 1))
                        },
                    )
                    .select_sound(None)
                    .help(format!("Clear floor {} to open floor {n}.", n - 1)),
                );
            }
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the list of wings."),
        );
        items
    }
}
