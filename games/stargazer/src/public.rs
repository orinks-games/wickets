// Stargazer: lie on a dark hilltop and sweep the night sky by ear.
//
// The arrows move a listening point across the sky. Stars twinkle as soft
// chimes from their side of the sky, brighter stars louder. Rest on the
// stars of a constellation and trace them in order, and it plays its own
// little tune and goes into your star journal with a line of sky lore.
// Nothing can go wrong and nothing is timed.
//
// The sky follows the real calendar: each season's evening sky has its
// own constellations, the year's meteor showers fall on their real peak
// nights, and the full moon rises on full moon nights.

pub mod almanac;
pub mod challenge;
pub mod journal;
pub mod rules;
pub mod screens;
pub mod shower;
pub mod sky;
pub mod sound;
mod soundcheck;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Pages, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use journal::Journal;
pub use screens::{NightScreen, ShowerScreen};
pub use soundcheck::SoundCheck;

use almanac::Shower;
use sky::{open_tier, season_constellations, tier_needs, CONSTELLATIONS};

/// The game's id: its sounds live under `sounds/stargazer/`.
pub const ID: &str = "stargazer";
/// The game's name.
pub const NAME: &str = "Stargazer";
/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");
/// The title screen's `State::name`, which leaving a night returns to.
pub const TITLE_NAME: &str = "stargazer";
/// The label of the meteor shower row, before its state.
pub const SHOWER_ROW: &str = "Meteor shower";
/// The journal row's label.
pub const JOURNAL_ROW: &str = "Star journal";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Cozy,
    blurb:
        "Lie on a dark hilltop and trace the constellations under the real season's sky.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Stargazer as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Night,
            parlor_core::story::PartOfDay::SmallHours,
        ],
        seasons: &[],
    },
};

/// The game's event on `date`: a meteor shower on its peak nights and
/// run-up, or a full moon.
pub fn event(date: parlor_core::Date) -> Option<parlor_core::Happening> {
    almanac::event(date)
}

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the night. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// The meteor shower row's help: tonight's shower, or when the next one is.
pub fn shower_help(ctx: &mut Ctx<Parlor>) -> String {
    let date = today(ctx);
    match almanac::shower_on(date) {
        Some(night) => format!(
            "The {} are falling tonight, {}. Lie back and wish on the shooting stars.",
            night.shower.name(),
            night.strength_words()
        ),
        None => format!(
            "{} Shooting stars fall here only on real shower nights.",
            almanac::next_shower_words(date)
        ),
    }
}

/// The Play row's help: the season and how much of it is in the journal.
pub fn play_help(ctx: &mut Ctx<Parlor>) -> String {
    let date = today(ctx);
    let season = almanac::sky_season(date);
    let journal = Journal::load(ctx);
    let tier = open_tier(journal.logged.len());
    let up: Vec<usize> = season_constellations(season)
        .into_iter()
        .filter(|&c| CONSTELLATIONS[c].tier <= tier)
        .collect();
    let logged = up.iter().filter(|&&c| journal.has(c)).count();
    let mut text = format!(
        "Lie back under tonight's {} sky. {logged} of the {} constellations up tonight are in your journal.",
        season.name(),
        up.len()
    );
    if let Some(moon) = almanac::full_moon(date) {
        text.push_str(&format!(" {} is up.", sentence_start(moon.name)));
    }
    text
}

/// Records: the journal's tallies, wishes, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let journal = Journal::load(ctx);
    let mut lines = vec![format!(
        "Records. Your star journal holds {}.",
        journal.tally()
    )];
    lines.push(match journal.nights {
        0 => "No nights on the hill yet.".to_string(),
        1 => "1 night on the hill.".to_string(),
        n => format!("{n} nights on the hill."),
    });
    if journal.traces > 0 {
        lines.push(format!(
            "Constellations traced, counting repeats: {}.",
            journal.traces
        ));
    }
    let logged = journal.logged.len();
    let tier = open_tier(logged);
    if tier < 2 {
        lines.push(format!(
            "{} more new constellations open the next tier.",
            tier_needs(tier + 1) - logged
        ));
    }
    let mut wishes = vec![match journal.total_wishes() {
        0 => "Wishes: none yet.".to_string(),
        n => format!("Wishes on shooting stars: {n}."),
    }];
    for shower in Shower::ALL {
        if let Some(best) = Records::best(ctx, ID, &format!("wishes {}", shower.id())) {
            wishes.push(format!("{}: best watch {best}.", shower.name()));
        }
    }
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        lines.join(" "),
        wishes.join(" "),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct Stargazer;

/// The game's title screen.
pub type Title = menus::Title<Stargazer>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(Stargazer)
}

impl TitleGame for Stargazer {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let date = today(ctx);
        let season = almanac::sky_season(date);
        let play_help = play_help(ctx);
        let shower_help = shower_help(ctx);
        let shower_label = match almanac::shower_on(date) {
            Some(night) => format!("{SHOWER_ROW}, the {}", night.shower.name()),
            None => labels::locked(SHOWER_ROW),
        };
        vec![
            MenuItem::new(
                format!("{}, {} sky", labels::PLAY, season.name()),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let screen = NightScreen::free(ctx);
                    ctx.push(screen)
                },
            )
            .help(play_help),
            MenuItem::new(shower_label, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                match ShowerScreen::open(ctx) {
                    Some(screen) => ctx.push(screen),
                    None => {
                        sound::play(ctx, sound::NOTHING);
                        let words = almanac::next_shower_words(today(ctx));
                        ctx.say(format!("No meteor shower tonight. {words}"));
                    }
                }
            })
            .help(shower_help),
        ]
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let tally = Journal::load(ctx).tally();
        vec![MenuItem::new(JOURNAL_ROW, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
            let pages = Journal::load(ctx).pages();
            ctx.push(Pages::new(pages))
        })
        .help(format!(
            "Every constellation you have traced, with its lore, and your wonders, moons and wishes. {}.",
            sentence_start(&tally)
        ))]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your journal's tallies, nights on the hill and wishes, with your challenge results."
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
            "A daily star hunt, a weekly set of patterns and a monthly grand tour, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has spent a night on the hill is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && Journal::load(ctx).nights == 0
    }
}
