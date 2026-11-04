// Freewheel: ride a bike through Courier City traffic by ear.
//
// A street-riding game after Cyclepath (Talon, 2017). You are a new bike
// courier in Downtown Courier City: parked cars on your right, the bike
// lane, the traffic lane and cars from behind. Speed, braking and risk
// are yours. Every hazard warns by time, at least 1.5 seconds before you
// reach it, from where it is. This first slice has Training, three
// Downtown time trials on the road bike and the BMX, a free ride and the
// daily, weekly and monthly rides.

pub mod bike;
pub mod challenge;
pub mod course;
pub mod events;
pub mod jobs;
pub mod progress;
pub mod ride;
pub mod rides;
pub mod shop;
pub mod sound;
mod soundcheck;
pub mod training;
pub mod tutorial;
pub mod world;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::screens::SettingsRow;
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Rng};

pub use bike::Bike;
pub use progress::{CallOuts, Choices, Progress};
pub use ride::{Mode, Ride};
pub use rides::{ChallengeHost, ChallengesMenu, Rides, CHALLENGES_NAME};
pub use soundcheck::SoundCheck;

use course::COURSES;
use progress::{best_time, board, distance_words, dollar_words, endless_board, time_words};

/// The game's id: its sounds live under `sounds/freewheel/`.
pub const ID: &str = "freewheel";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a ride returns to.
pub const TITLE_NAME: &str = "freewheel";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Freewheel",
    category: Category::Action,
    blurb: "Ride a bike through city traffic by ear, as fast as you dare.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Freewheel as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Morning],
        seasons: &[
            parlor_core::Season::Spring,
            parlor_core::Season::Summer,
            parlor_core::Season::Autumn,
        ],
    },
};

/// Opens the current `period` challenge from the launcher's list: its
/// briefing, then the ride. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

fn best_words(ctx: &mut Ctx<Parlor>, course: usize, bike: Bike) -> String {
    match best_time(ctx, &board(COURSES[course].id, bike)) {
        Some(t) => format!("best {}", time_words(t)),
        None => "no time yet".to_string(),
    }
}

/// " Your best is 1 minute 4 seconds, 98000 ride points." for an event
/// row's help, or nothing.
fn event_best(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    match best_time(ctx, board) {
        Some(t) => format!(
            " Your best is {}, {} ride points.",
            time_words(t),
            parlor_core::Records::best(ctx, ID, board).unwrap_or(0)
        ),
        None => String::new(),
    }
}

/// Records: best times, totals and the challenges, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let mut pages = vec![format!(
        "Records. Rides finished: {}. Distance ridden: {}. Training lessons passed: {} of {}. Crashes: {}. Longest jump: {:.1} seconds of air.",
        progress.rides,
        parlor_core::units::long(progress.metres / 1000.0),
        progress.lessons,
        training::LESSONS,
        progress.crashes,
        progress.best_air
    )];
    for (i, info) in COURSES.iter().enumerate() {
        let parts: Vec<String> = Bike::ALL
            .into_iter()
            .filter(|&bike| progress.owns_bike(bike))
            .map(|bike| {
                let b = board(info.id, bike);
                match best_time(ctx, &b) {
                    Some(t) => format!(
                        "on the {}, {}, {} ride points",
                        bike.name(),
                        time_words(t),
                        parlor_core::Records::best(ctx, ID, &b).unwrap_or(0)
                    ),
                    None => format!("on the {}, no time yet", bike.name()),
                }
            })
            .collect();
        pages.push(format!(
            "Course {}, {}: {}.",
            i + 1,
            info.name,
            parts.join("; ")
        ));
    }
    let endless: Vec<String> = course::District::ALL
        .into_iter()
        .filter_map(|d| {
            parlor_core::Records::best(ctx, ID, &endless_board(d))
                .map(|m| format!("{}, {}", d.name(), distance_words(m as f64)))
        })
        .collect();
    pages.push(format!(
        "Endless ride. {}. {} to spend, {} earned in all. Deliveries made: {}.",
        if endless.is_empty() {
            "No rides yet".to_string()
        } else {
            format!("Best: {}", endless.join("; "))
        },
        sentence_start(&dollar_words(progress.cash)),
        dollar_words(progress.cash_total),
        progress.deliveries
    ));
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    let mut events: Vec<String> = Vec::new();
    for (name, stem) in [
        (events::LEAF_SEASON, events::LEAF_BOARD),
        (events::SUNDAY, events::SUNDAY_BOARD),
    ] {
        for bike in Bike::ALL {
            let board = events::board(stem, bike);
            if let Some(t) = best_time(ctx, &board) {
                events.push(format!(
                    "{name} on the {}, best {}, {} ride points.",
                    bike.name(),
                    time_words(t),
                    parlor_core::Records::best(ctx, ID, &board).unwrap_or(0)
                ));
            }
        }
    }
    if !events.is_empty() {
        pages.push(format!("Special rides. {}", events.join(" ")));
    }
    pages
}

/// The game on the shared title builder.
pub struct Freewheel;

/// The game's title screen.
pub type Title = menus::Title<Freewheel>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(Freewheel)
}

/// The call-outs' next setting, from their Settings row.
fn cycle_call_outs(ctx: &mut Ctx<Parlor>, delta: i32) -> Option<String> {
    let mut c = Choices::load(ctx);
    c.call_outs = if delta < 0 {
        c.call_outs.next().next()
    } else {
        c.call_outs.next()
    };
    c.store(ctx);
    None
}

impl TitleGame for Freewheel {
    const ID: &'static str = ID;
    const NAME: &'static str = "Freewheel";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let c = Choices::load(ctx);
        let info = &COURSES[c.course];
        let best = best_words(ctx, c.course, c.bike);
        let mut rows = vec![
            MenuItem::new(
                format!(
                    "{}, time trial: {} on the {}",
                    labels::PLAY,
                    info.name,
                    c.bike.name()
                ),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(Ride::time_trial(c.course, c.bike))
                },
            )
            .help(format!("{} {}.", info.about(), sentence_start(&best))),
            {
                let district = Choices::load(ctx).district;
                let best = match parlor_core::Records::best(ctx, ID, &endless_board(district)) {
                    Some(m) => format!(" Best {}.", distance_words(m as f64)),
                    None => String::new(),
                };
                MenuItem::new(
                    format!("Endless ride: {} on the {}", district.name(), c.bike.name()),
                    move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        ctx.push(Ride::endless(c.bike, district, Rng::from_time().next_u64()))
                    },
                )
                .help(format!(
                    "Ride until your third crash. Dodge oncoming cars and grab parcels.{best}"
                ))
            },
            MenuItem::new("Job board", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(jobs::JobBoard::new())
            })
            .help("Courier deliveries for cash. Full pay on time, half pay late."),
            MenuItem::new("Free ride", move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Ride::free_ride(c.bike, Rng::from_time().next_u64()))
            })
            .help(format!(
                "{} of Downtown with no clock, to learn the sounds of the street.",
                parlor_core::units::route(ride::FREE_LENGTH)
            )),
            MenuItem::new("Course select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(CourseSelect::new())
            })
            .help(format!(
                "Choose one of the {} Downtown courses for the time trial.",
                COURSES.len()
            )),
            MenuItem::new("Bike select", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(BikeSelect::new())
            })
            .help(format!(
                "Choose your bike. Riding the {} now.",
                c.bike.name()
            )),
            MenuItem::new(
                "District select",
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(shop::DistrictSelect::new()),
            )
            .help("Choose where the endless ride goes."),
            MenuItem::new("Bike shop", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(shop::Shop::new())
            })
            .help(format!(
                "Spend cash on bikes and districts. You have {}.",
                dollar_words(Progress::load(ctx).cash)
            )),
        ];
        let today = parlor_core::challenges::today(ctx);
        let on = format!(" On the {}.", c.bike.name());
        if events::leaf_open(today) {
            let help = format!(
                "{}{on}{}",
                events::leaf_help(),
                event_best(ctx, &events::board(events::LEAF_BOARD, c.bike))
            );
            rows.push(
                MenuItem::new(
                    events::LEAF_SEASON,
                    move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.push(Ride::leaf_season(c.bike, today))
                    },
                )
                .help(help),
            );
        } else {
            // Out of season: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::LEAF_SEASON),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::leaf_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::leaf_locked_help(today)),
            );
        }
        if let Some(course) = events::sunday_course(today) {
            let help = format!(
                "{}{on}{}",
                events::sunday_help(),
                event_best(ctx, &events::board(events::SUNDAY_BOARD, c.bike))
            );
            rows.push(
                MenuItem::new(
                    events::SUNDAY,
                    move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        ctx.push(Ride::sunday(c.bike, course.clone()))
                    },
                )
                .help(help),
            );
        } else {
            // Closed today: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::SUNDAY),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::sunday_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::sunday_locked_help(today)),
            );
        }
        rows
    }

    fn practice_row(&mut self, ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        let passed = Progress::load(ctx).lessons;
        Some(
            MenuItem::new("Practice lessons", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                // The moped cannot hop, so it trains on the road bike.
                let bike = match Choices::load(ctx).bike {
                    b if b.motor() => Bike::Road,
                    b => b,
                };
                let pages = training::start(ctx, bike);
                ctx.push(pages)
            })
            .help(format!(
                "Training: {} short rides on pedalling, braking, steering, the door zone and hopping. {passed} passed.",
                training::LESSONS
            )),
        )
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![menus::settings::setting(
            "freewheel_call_outs",
            |ctx| format!("Spoken call-outs: {}", Choices::peek(ctx).call_outs.word()),
            cycle_call_outs,
            "Choose when gameplay cues are named: every time, once per session, or off. A session lasts until you close Wicket's. Status and Hint always speak.",
        ),
            parlor_core::units::row(),
        ]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best times and ride points for each course and bike, totals, and your challenge results."
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
            "A daily morning run, a weekly rush hour and a monthly cross-town ride, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has finished a ride or a lesson is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let p = Progress::load(ctx);
        !menus::tutorial_read(ctx, ID) && p.rides == 0 && p.lessons == 0
    }
}

/// Choose a time trial course. Enter rides it and keeps it as the Play row's course.
pub struct CourseSelect {
    menu: MenuCore<Self, Parlor>,
}

impl CourseSelect {
    pub fn new() -> Self {
        CourseSelect {
            menu: MenuCore::new("Course select")
                .with_intro_help("Up and Down choose a course, Enter rides it, Escape goes back."),
        }
    }
}

impl Default for CourseSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for CourseSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "freewheel_courses"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let c = Choices::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for (i, info) in COURSES.iter().enumerate() {
            let best = best_words(ctx, i, c.bike);
            items.push(
                MenuItem::new(
                    format!("Course {}, {}, {best}", i + 1, info.name),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let mut c = Choices::load(ctx);
                        c.course = i;
                        c.store(ctx);
                        ctx.replace(Ride::time_trial(i, c.bike));
                    },
                )
                .help(format!("{} On the {}.", info.about(), c.bike.name())),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Freewheel title."),
        );
        items
    }
}

/// Choose the bike every ride uses.
pub struct BikeSelect {
    menu: MenuCore<Self, Parlor>,
}

impl BikeSelect {
    pub fn new() -> Self {
        BikeSelect {
            menu: MenuCore::new("Bike select").with_intro_help(
                "Up and Down choose a bike, Enter rides it from now on, Escape goes back.",
            ),
        }
    }
}

impl Default for BikeSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for BikeSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "freewheel_bikes"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let current = Choices::load(ctx).bike;
        let mut items: Vec<MenuItem<Self, Parlor>> = Progress::load(ctx)
            .my_bikes()
            .into_iter()
            .map(|bike| {
                let name = sentence_start(bike.name());
                let label = if bike == current {
                    format!("{name}, riding now")
                } else {
                    name.clone()
                };
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let mut c = Choices::load(ctx);
                    c.bike = bike;
                    c.store(ctx);
                    ctx.pop_with(format!("Riding the {}", bike.name()));
                })
                .help(bike.about())
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Freewheel title."),
        );
        items
    }
}
