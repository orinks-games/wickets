//! Dragon Dance: lead a dragon dance down the Lunar New Year parade.
//!
//! The player is the head of a paper dragon. The drum keeps the beat, and
//! a dancer ahead carries a pearl on a pole: it shimmers from its lane,
//! and the dragon scores on every beat its head is in that lane. Gongs ask
//! for a rear, red envelopes make the dragon longer, and firecrackers in a
//! lane make the tail stumble. Each street is a level.
//!
//! The real calendar shapes the parade (see [`festival`]): the year's
//! animal, the ten-day run-up with lanterns and lion-dance drums, the New
//! Year parade on Lunar New Year itself, with a keepsake for the year's
//! animal, and the Lantern walk on the fifteenth day.

pub mod briefing;
pub mod challenge;
pub mod dance;
pub mod festival;
pub mod progress;
pub mod rules;
pub mod select;
pub mod sound;
mod soundcheck;
pub mod streets;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{
    challenges, Category, Challenge, Challenges, Date, GameInfo, Happening, Pages, Parlor, Period,
    Records,
};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use dance::{DanceScreen, Launch, Mode};
pub use progress::Progress;
pub use select::StreetSelect;
pub use soundcheck::SoundCheck;

use festival::Gate;
use streets::STREETS;

/// The game's id: its sounds live under `sounds/dragon-dance/`.
pub const ID: &str = "dragon-dance";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which a street returns to.
pub const TITLE_NAME: &str = "dragon_dance";

/// The New Year parade row's label.
pub const NEW_YEAR_ROW: &str = "New Year parade";
/// The Lantern walk row's label.
pub const LANTERN_ROW: &str = "Lantern walk";
/// The practice row's label.
pub const PRACTICE_ROW: &str = "Practice street";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Dragon Dance",
    category: Category::Seasonal,
    blurb: "Lead a Lunar New Year dragon dance down the parade, following the pearl to the drum.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <DragonDance as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Afternoon,
            parlor_core::story::PartOfDay::Evening,
        ],
        seasons: &[parlor_core::Season::Winter],
    },
};

/// The event the game holds on `date`: the run-up to Lunar New Year, the
/// New Year parade, or the Lantern walk.
pub fn event(date: Date) -> Option<Happening> {
    festival::event(date)
}

/// Opens the current `period` challenge from the launcher's list: its
/// briefing, then the street. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: each street's stars and best, the festival days, the
/// challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let mut lines = Vec::new();
    for (i, street) in STREETS.iter().enumerate() {
        let n = i + 1;
        if !progress.played(n) {
            continue;
        }
        let best = Records::best(ctx, ID, &format!("street {n}")).unwrap_or(0);
        lines.push(format!(
            "Street {n}, {}: {}, best {best}.",
            street.name,
            dance::stars_words(progress.stars_on(n))
        ));
    }
    let first = if lines.is_empty() {
        "Records. No streets danced yet.".to_string()
    } else {
        format!(
            "Records. {} of {} stars. {}",
            progress.total_stars(),
            STREETS.len() * 3,
            lines.join(" ")
        )
    };
    let mut second = vec![format!(
        "Longest dragon: {} segments. Red envelopes caught: {}.",
        progress.longest.max(rules::START_SEGMENTS),
        progress.envelopes
    )];
    second.push(match progress.parades.len() {
        0 => "No New Year parades yet.".to_string(),
        1 => "New Year parade danced in 1 year.".to_string(),
        n => format!("New Year parade danced in {n} years."),
    });
    if let Some(best) = Records::best(ctx, ID, "new year") {
        second.push(format!("Best New Year parade: {best}."));
    }
    second.push(format!("Keepsakes: {} of 12.", progress.keepsakes.len()));
    second.push(format!(
        "Lanterns found: {} of {}.",
        progress.lanterns.len(),
        festival::LANTERNS.len()
    ));
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        first,
        second.join(" "),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct DragonDance;

/// The game's title screen.
pub type Title = menus::Title<DragonDance>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(DragonDance)
}

/// The Play row's help: the street, its best, and the season.
fn play_help(ctx: &mut Ctx<Parlor>, n: usize) -> String {
    let street = streets::street(n);
    let best = match Records::best(ctx, ID, &format!("street {n}")) {
        Some(best) => format!(" Best {best}."),
        None => String::new(),
    };
    let season = festival::season_words(challenges::today(ctx));
    let season = if season.is_empty() {
        String::new()
    } else {
        format!(" {season}")
    };
    format!(
        "The next street of the parade. {}.{best}{season}",
        sentence_start(&street.describe())
    )
}

/// A festival row: open, it starts its street; locked, it says why.
fn festival_row(
    open: Option<Mode>,
    name: &str,
    open_help: String,
    locked_help: String,
) -> TitleRow<DragonDance> {
    match open {
        Some(mode) => MenuItem::new(name, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
            briefing::open(ctx, mode)
        })
        .help(open_help),
        None => {
            let why = locked_help.clone();
            MenuItem::new(
                labels::locked(name),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    sound::play(ctx, sound::NOPE);
                    ctx.say(why.clone());
                },
            )
            .help(locked_help)
        }
    }
}

impl TitleGame for DragonDance {
    const ID: &'static str = ID;
    const NAME: &'static str = "Dragon Dance";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        let today = challenges::today(ctx);
        let gate = festival::gate(today);
        let n = progress.next_street();
        let play_help = play_help(ctx, n);
        let open_streets = (1..=STREETS.len()).filter(|&s| progress.opened(s)).count();
        let new_year = match gate {
            Gate::NewYear { year } => Some(Mode::NewYear(year)),
            _ => None,
        };
        let lantern = match gate {
            Gate::Lantern { year } => Some(Mode::Lantern(year)),
            _ => None,
        };
        let animal = match gate {
            Gate::NewYear { year } => parlor_core::calendar::zodiac(year),
            _ => festival::year_animal(today),
        };
        vec![
            MenuItem::new(
                format!(
                    "{}, street {n}: {}",
                    labels::PLAY,
                    streets::street(n).name
                ),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    briefing::open(ctx, Mode::Street(n))
                },
            )
            .help(play_help),
            MenuItem::new(
                "Street select",
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(StreetSelect::new()),
            )
            .help(format!(
                "Every street of the parade, with your stars. {open_streets} of {} open.",
                STREETS.len()
            )),
            festival_row(
                new_year,
                NEW_YEAR_ROW,
                format!(
                    "{} parade: a long street that ends at midnight firecrackers, and a keepsake for the year: {}.",
                    sentence_start(&festival::year_of(animal)),
                    festival::keepsake(animal)
                ),
                format!(
                    "{} {}",
                    festival::new_year_locked(today),
                    "It ends at midnight firecrackers, with a keepsake for the year's animal."
                ),
            ),
            festival_row(
                lantern,
                LANTERN_ROW,
                "A calm walk under the lanterns for the Lantern Festival: no firecrackers, no gongs to miss, and lanterns to collect.".to_string(),
                format!(
                    "{} A calm walk, with lanterns to collect.",
                    festival::lantern_locked(today)
                ),
            ),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new(PRACTICE_ROW, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                briefing::open(ctx, Mode::Practice)
            })
            .help("A short, slow street with every call spoken. Nothing is saved, and stumbles cost nothing."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Stars and best scores on every street, your longest dragon and the festival days, with your challenge results.".to_string()
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        vec![MenuItem::new(
            "Keepsakes",
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let pages = Progress::load(ctx).keepsake_pages();
                ctx.push(Pages::new(pages));
            },
        )
        .help(format!(
            "The keepsakes of the years' animals, {} of 12, and the lanterns you have found, {} of {}.",
            progress.keepsakes.len(),
            progress.lanterns.len(),
            festival::LANTERNS.len()
        ))]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        let mut rows = vec![menus::settings::toggle(
            "dragon_dance_call_outs",
            labels::call_outs,
            |ctx| !Progress::peek(ctx).quiet,
            |ctx, on| {
                let mut progress = Progress::load(ctx);
                progress.quiet = !on;
                progress.store(ctx);
            },
            |_ctx, on| {
                Some(if on {
                    "Spoken call-outs: on. The pearl, envelopes and firecrackers are named with their lanes."
                } else {
                    "Spoken call-outs: off. You follow the pearl and dodge firecrackers without call-outs."
                }
                .to_string())
            },
            "When on, a voice names the pearl's lane as it moves, and each envelope and firecracker string as it comes.",
        )];
        rows.extend(parlor_core::briefing::rows());
        rows
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
            "A daily street, a weekly long parade and a monthly grand festival, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has danced a street is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any_played()
    }

    fn intro_help(&self) -> Option<String> {
        Some("Up and Down move, Enter chooses, Escape goes back to the parlor.".to_string())
    }
}
