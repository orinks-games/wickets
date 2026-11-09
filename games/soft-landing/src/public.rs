// Soft Landing: set a small lander down on moons of the outer planets.
//
// Gravity pulls the lander toward the pad and the main engine pushes
// back while the player holds it. A descent tone rises with the falling
// speed, with "Too fast" and "Safe speed" spoken as it crosses the line;
// mission control calls the altitude at marks the player chooses; and
// the pad beacon quickens as the ground nears. Touch down under the safe
// speed to land, faster to crash. Three survey moons open in turn, the
// practice pad has endless fuel, and daily, weekly and monthly
// challenges give every player the same seeded moons.

pub mod challenge;
pub mod events;
pub mod play;
pub mod progress;
pub mod rules;
pub mod sound;
pub mod soundcheck;
pub mod tutorial;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Landings, CHALLENGES_NAME};
pub use play::{Descent, Mode};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{speed_words, survey_moon, Moon, MOONS};

/// The game's id: its sounds live under `sounds/soft-landing/`.
pub const ID: &str = "soft-landing";

/// The game's name.
pub const NAME: &str = "Soft Landing";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a flight returns to.
pub const TITLE_NAME: &str = "soft_landing";

/// The moon select menu's `State::name`.
pub const MOONS_NAME: &str = "soft_landing_moons";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Action,
    blurb: "Set a small lander down on the moons of the outer planets.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <SoftLanding as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Night],
        seasons: &[],
    },
};

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the flight. Leaving it returns
/// to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// A moon in a sentence part: "light gravity, 300 metres up, safe speed 3
/// metres a second".
pub fn moon_summary(moon: &Moon) -> String {
    format!(
        "{}, {} up, safe speed {}",
        moon.gravity_words(),
        parlor_core::units::length(moon.height),
        speed_words(moon.safe)
    )
}

/// " Your best is 900." for an event row's help, or nothing.
fn event_best(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    match Records::best(ctx, ID, board) {
        Some(b) => format!(" Your best is {b}."),
        None => String::new(),
    }
}

fn best_words(ctx: &mut Ctx<Parlor>, n: u32) -> Option<u64> {
    Records::best(ctx, ID, &progress::board(n))
}

/// Records: moons landed, bests, tallies, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let softest = match p.softest {
        Some(s) => format!("softest touchdown {}", speed_words(s)),
        None => "no landing yet".to_string(),
    };
    let moons: Vec<String> = (1..=MOONS)
        .map(|n| {
            let name = survey_moon(n).name;
            match best_words(ctx, n) {
                Some(b) => format!("Moon {n}, {name}: best score {b}."),
                None if n <= p.open_moons() => format!("Moon {n}, {name}: not landed yet."),
                None => format!("Moon {n}, {name}: locked."),
            }
        })
        .collect();
    let count = |n: u32, one: &str, many: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {many}")
        }
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
            "Records. Moons landed: {} of {MOONS}, {softest}.",
            p.cleared
        ),
        moons.join(" "),
        format!(
            "Flown so far: {}, {} and {}.",
            count(p.landings, "landing", "landings"),
            count(p.crashes, "crash", "crashes"),
            count(p.practice_landings, "practice landing", "practice landings")
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct SoftLanding;

/// The game's title screen.
pub type Title = menus::Title<SoftLanding>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(SoftLanding)
}

impl TitleGame for SoftLanding {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    /// Distances are spoken in the player's units, so Units sits here too.
    fn settings(_ctx: &Ctx<Parlor>) -> Vec<hearken::screens::SettingsRow<Parlor>> {
        vec![parlor_core::units::row()]
    }

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let next = p.next_moon();
        let moon = survey_moon(next);
        let best = match best_words(ctx, next) {
            Some(b) => format!(" Best score {b}."),
            None => String::new(),
        };
        let mut rows = vec![
            MenuItem::new(
                format!("{}, moon {next}: {}", labels::PLAY, moon.name),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Descent::new(Mode::Moon(next)))
                },
            )
            .help(format!("{}.{best}", sentence_start(&moon_summary(&moon)))),
            MenuItem::new("Moon select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(MoonSelect::new())
            })
            .help(format!(
                "Fly any moon you have reached. {} of {MOONS} open.",
                p.open_moons()
            )),
        ];
        let today = parlor_core::challenges::today(ctx);
        if let Some(moon) = events::full_moon_pad(today) {
            let best = event_best(ctx, events::FULL_MOON_BOARD);
            rows.push(
                MenuItem::new(events::FULL_MOON, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Descent::event(Mode::FullMoon, moon))
                })
                .help(format!(
                    "Tonight's pad on {}, the same for every player: {}. Crickets sing behind the radio click.{best}",
                    moon.name,
                    moon_summary(&moon)
                )),
            );
        } else {
            // Closed tonight: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::FULL_MOON),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::full_moon_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::full_moon_locked_help(today)),
            );
        }
        if events::tranquility_open(today) {
            let moon = events::tranquility();
            let best = event_best(ctx, events::TRANQUILITY_BOARD);
            rows.push(
                MenuItem::new(
                    events::TRANQUILITY,
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        ctx.push(Descent::new(Mode::Tranquility))
                    },
                )
                .help(format!(
                    "Moon landing day: Tranquility Base, {}. The radio beeps with the Quindar tone.{best}",
                    moon_summary(&moon)
                )),
            );
        } else {
            // Out of season: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::TRANQUILITY),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::tranquility_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::tranquility_locked_help(today)),
            );
        }
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice pad", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Descent::new(Mode::Practice))
            })
            .help(format!(
                "Endless fuel, call-outs every {} when slow, and nothing to lose.",
                parlor_core::units::count(
                    rules::shown_call_every(rules::PRACTICE_CALL_EVERY, parlor_core::units::us())
                        .into()
                )
            )),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best score on each moon, your softest touchdown, and your challenge results."
            .to_string()
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
            "A daily, a weekly and a monthly landing, the same moons for every player. {}",
            daily.resets()
        )
    }

    /// A player who has flown is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }
}

/// Choose a moon: any survey moon reached so far.
pub struct MoonSelect {
    menu: MenuCore<Self, Parlor>,
}

impl MoonSelect {
    pub fn new() -> Self {
        MoonSelect {
            menu: MenuCore::new("Moon select")
                .with_intro_help("Up and Down choose a moon, Enter flies it, Escape goes back."),
        }
    }
}

impl Default for MoonSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for MoonSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        MOONS_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let open = Progress::load(ctx).open_moons();
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for n in 1..=MOONS {
            let moon = survey_moon(n);
            if n > open {
                let why = format!("Land on {} to open {}.", survey_moon(n - 1).name, moon.name);
                items.push(
                    MenuItem::new(
                        labels::locked(&format!("Moon {n}")),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.play_ui(hearken::UiSound::Error);
                            ctx.say(why.clone())
                        },
                    )
                    .select_sound(None)
                    .help(format!(
                        "Land on {} to open this moon.",
                        survey_moon(n - 1).name
                    )),
                );
                continue;
            }
            let best = best_words(ctx, n);
            let label = match best {
                Some(b) => format!("Moon {n}, {}, best {b}", moon.name),
                None => format!("Moon {n}, {}", moon.name),
            };
            items.push(
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Descent::new(Mode::Moon(n)))
                })
                .help(format!("{}.", sentence_start(&moon_summary(&moon)))),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Soft Landing title."),
        );
        items
    }
}
