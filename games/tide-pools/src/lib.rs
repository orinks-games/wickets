//! Tide Pools: Wander the beach at low tide, gather shells and creatures, and stock your own aquarium.
//!
//! A calm game with no way to fail. Walk a strip of rock pools at low
//! tide, lean in to listen, and gather what you hear: crabs, shells,
//! starfish, anemones, little fish, and rare finds that only come out
//! under certain tides and weather. Shy creatures hide when you lean in
//! and creep back out if you keep still. Everything you gather goes home
//! to your aquarium, which you can visit and listen to. The tide coming
//! back in gently ends each trip.

pub mod events;
pub mod menus;
pub mod outing;
pub mod rules;
pub mod save;
pub mod sound;
pub mod tutorial;

use hearken::menu::{Label, MenuItem};
use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::menus::{labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use menus::{
    AquariumScreen, ChallengeHost, ChallengesMenu, Shores, SoundCheck, CHALLENGES_NAME,
};
pub use outing::OutingScreen;
pub use save::Aquarium;

use rules::{Mode, Place, Species};

/// The game's id: its sounds live under `sounds/tide-pools/`.
pub const ID: &str = "tide-pools";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a trip returns to.
pub const TITLE_NAME: &str = "tide_pools";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Tide Pools",
    category: Category::Cozy,
    blurb:
        "Wander the beach at low tide, gather shells and creatures, and stock your own aquarium.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <TidePools as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Morning,
            parlor_core::story::PartOfDay::Afternoon,
        ],
        seasons: &[parlor_core::Season::Spring, parlor_core::Season::Summer],
    },
};

/// Opens this game's current `period` challenge from the launcher's list:
/// its briefing, then the shore. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

fn best_words(ctx: &mut Ctx<Parlor>, place: Place) -> String {
    match Records::best(ctx, ID, place.board()) {
        Some(b) => format!("best {b}"),
        None => "not visited yet".to_string(),
    }
}

/// What the Play row says and does for one shore.
struct Shore {
    place: Place,
    open: bool,
    /// "Sandy Cove, best 120" or "Rocky Point, locked".
    words: String,
    help: String,
}

fn shores(ctx: &mut Ctx<Parlor>) -> Vec<Shore> {
    let aquarium = Aquarium::load(ctx);
    Place::ALL
        .into_iter()
        .map(|place| {
            let open = aquarium.open(place);
            let (words, help) = if open {
                (
                    format!("{}, {}", place.name(), best_words(ctx, place)),
                    format!("{} Left and Right choose the shore.", place.blurb()),
                )
            } else {
                (
                    labels::locked(place.name()),
                    format!(
                        "Gather {} different kinds of creature to open {}. Left and Right choose the shore.",
                        place.unlock_kinds(),
                        place.name()
                    ),
                )
            };
            Shore {
                place,
                open,
                words,
                help,
            }
        })
        .collect()
}

/// Records: the aquarium, each shore's best and the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let aquarium = Aquarium::load(ctx);
    let trips = match aquarium.outings {
        1 => "1 trip".to_string(),
        n => format!("{n} trips"),
    };
    let mut pages = vec![format!(
        "Records. Your aquarium holds {} of {} kinds, {} creatures in all, from {trips}.",
        aquarium.kinds(),
        Species::ALL.len(),
        aquarium.creatures()
    )];
    let bests: Vec<String> = Place::ALL
        .into_iter()
        .map(|place| {
            let words = if aquarium.open(place) {
                best_words(ctx, place)
            } else {
                format!("opens at {} kinds", place.unlock_kinds())
            };
            format!("{}: {words}.", sentence_start(place.name()))
        })
        .collect();
    pages.push(format!("Shore scores. {}", bests.join(" ")));
    let events: Vec<String> = events::Event::ALL
        .into_iter()
        .filter_map(|e| {
            Records::best(ctx, ID, e.board()).map(|b| format!("{}: best {b}.", e.name()))
        })
        .collect();
    if !events.is_empty() {
        pages.push(format!("Event scores. {}", events.join(" ")));
    }
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            hearken::text::end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    pages
}

/// The furthest shore your aquarium has opened: where event trips go.
pub fn event_shore(ctx: &mut Ctx<Parlor>) -> Place {
    let aquarium = Aquarium::load(ctx);
    Place::ALL
        .into_iter()
        .rev()
        .find(|p| aquarium.open(*p))
        .unwrap_or(Place::Cove)
}

/// A row for each calendar event, after the Play row: open today, or
/// locked in its place, saying when it opens.
fn event_rows(ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<TidePools>> {
    let today = parlor_core::challenges::today(ctx);
    let place = event_shore(ctx);
    events::Event::ALL
        .into_iter()
        .map(|event| {
            if !event.is_open(today) {
                return MenuItem::new(
                    labels::locked(event.name()),
                    move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.say(event.locked_line(today))
                    },
                )
                .select_sound(None)
                .help(event.locked_help(today));
            }
            let best = Records::best(ctx, ID, event.board());
            let label = match best {
                Some(b) => format!("{}, {}, best {b}", event.name(), place.name()),
                None => format!("{}, {}", event.name(), place.name()),
            };
            MenuItem::new(label, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                outing::start(ctx, Mode::Event(event, place))
            })
            .help(events::row_help(event, today, place.name(), best))
        })
        .collect()
}

/// The game on the shared title builder. It remembers which shore the
/// Play row is on.
#[derive(Default)]
pub struct TidePools {
    shore: usize,
}

/// The game's title screen.
pub type Title = parlor_core::menus::Title<TidePools>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    parlor_core::menus::Title::new(TidePools::default())
}

impl TitleGame for TidePools {
    const ID: &'static str = ID;
    const NAME: &'static str = "Tide Pools";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let shores = std::rc::Rc::new(shores(ctx));
        let label = shores.clone();
        let adjusted = shores.clone();
        let help = shores.clone();
        let chosen = shores.clone();
        let mut rows = vec![MenuItem::new(
            Label::dynamic(move |s: &Title, _| {
                format!("{}, {}", labels::PLAY, label[s.game.shore].words)
            }),
            move |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let shore = &chosen[s.game.shore];
                if shore.open {
                    outing::start(ctx, Mode::Beach(shore.place));
                } else {
                    let have = Aquarium::load(ctx).kinds();
                    ctx.say(format!(
                        "{} opens once your aquarium holds {} kinds of creature. It holds {have} now.",
                        sentence_start(shore.place.name()),
                        shore.place.unlock_kinds()
                    ));
                }
            },
        )
        .adjustable(|s: &mut Title, _ctx: &mut Ctx<Parlor>, delta: i32| {
            let last = Place::ALL.len() as i32 - 1;
            s.game.shore = (s.game.shore as i32 + delta).clamp(0, last) as usize;
        })
        .adjusted_text(Label::dynamic(move |s: &Title, _| {
            sentence_start(&adjusted[s.game.shore].words)
        }))
        .help(Label::dynamic(move |s: &Title, _| {
            help[s.game.shore].help.clone()
        }))];
        rows.extend(event_rows(ctx));
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice pools", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                outing::start(ctx, Mode::Practice)
            })
            .help(
                "Three sheltered pools with no tide, to learn leaning in, listening and gathering.",
            ),
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let kinds = Aquarium::load(ctx).kinds();
        vec![MenuItem::new(
            format!("Your aquarium, {kinds} of {} kinds", Species::ALL.len()),
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(AquariumScreen::new()),
        )
        .help("Visit everything you have gathered, and listen to it.")]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your aquarium's tally, your best score at each shore, and your challenge results."
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
            "Daily, weekly and monthly shores, the same for every player, each with a gentle goal. {}",
            daily.resets()
        )
    }

    /// A player who has been to the shore is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !parlor_core::menus::tutorial_read(ctx, ID) && Aquarium::load(ctx).outings == 0
    }
}
