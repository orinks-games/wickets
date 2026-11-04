// Firefly Meadow: walk a summer meadow at dusk and catch fireflies in
// time with their blink.
//
// A gentle seasonal game with no way to fail. Fireflies blink as soft
// tones from where they drift; walk close, press the main action as one
// glows, and it goes in your jar. Each species blinks its own pattern and
// rings its own note, and a jar set free rings them as a chord. The
// meadow follows the real season: full in summer, a few in spring, one
// late firefly in autumn. Midsummer
// night opens on the solstice, and the Fourth of July weekend brings
// fireworks on the horizon.

pub mod almanac;
pub mod challenge;
pub mod evening;
pub mod journal;
pub mod meadow;
pub mod night;
pub mod progress;
pub mod rules;
pub mod show;
pub mod sound;
pub mod soundcheck;
pub mod species;
pub mod tutorial;

use hearken::menu::{Menu, MenuItem};
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{
    Category, Challenge, Challenges, Date, GameInfo, Happening, Parlor, Period, Records,
};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use evening::{EveningScreen, Mode};
pub use journal::JournalMenu;
pub use night::Night;
pub use progress::Journal;
pub use soundcheck::SoundCheck;

use meadow::{Place, Plan};
use rules::duration_words;
use species::{Chord, Species};

/// The game's id: its sounds live under `sounds/firefly-meadow/`.
pub const ID: &str = "firefly-meadow";

/// The game's name.
pub const NAME: &str = "Firefly Meadow";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving an evening returns to.
pub const TITLE_NAME: &str = "firefly_meadow";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Seasonal,
    blurb: "Walk a meadow at dusk and catch fireflies in time with their blink.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Meadow as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Evening,
            parlor_core::story::PartOfDay::Night,
        ],
        seasons: &[parlor_core::Season::Summer],
    },
};

/// The special event the meadow holds on `date`: midsummer night, or the
/// Fourth of July fireworks.
pub fn event(date: Date) -> Option<Happening> {
    almanac::event(date)
}

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the evening. Leaving it returns
/// to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the journal's totals and bests, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let journal = Journal::load(ctx);
    let longest = Records::best(ctx, ID, progress::LONGEST);
    let mut lines = vec![format!(
        "Records. {} caught. {} of {} species found. {} of {} chords.",
        match journal.caught {
            1 => "1 firefly".to_string(),
            n => format!("{n} fireflies"),
        },
        journal.species_found(),
        Species::ALL.len(),
        journal.chords_found(),
        Chord::ALL.len()
    )];
    lines.push(match longest {
        Some(secs) => format!("Longest evening: {}.", duration_words(secs as f64)),
        None => "Longest evening: none yet.".to_string(),
    });
    lines.push(match journal.evenings {
        1 => "1 evening walked.".to_string(),
        n => format!("{n} evenings walked."),
    });
    if journal.releases > 0 {
        lines.push(match journal.releases {
            1 => "1 jar set free.".to_string(),
            n => format!("{n} jars set free."),
        });
    }
    if let Some(best) = Records::best(ctx, ID, progress::MIDSUMMER) {
        lines.push(format!(
            "Midsummer night: best {best} percent in step, {}.",
            match journal.keepsakes.len() {
                1 => "1 keepsake".to_string(),
                n => format!("{n} keepsakes"),
            }
        ));
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
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct Meadow;

/// The game's title screen.
pub type Title = menus::Title<Meadow>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(Meadow)
}

/// The Play row's label: "Play, the home meadow, an autumn evening".
pub fn play_label(ctx: &mut Ctx<Parlor>) -> String {
    let place = Journal::load(ctx).meadow();
    let plan = Plan::on(place, today(ctx));
    format!("{}, {}", labels::PLAY, plan.title())
}

fn meadow_help(journal: &Journal) -> String {
    let open = Place::WALKS
        .into_iter()
        .filter(|&p| journal.unlocked(p))
        .count();
    let mut text = format!("Enter chooses the next of your {open} open meadows for the Play row.");
    if let Some(next) = next_meadow_words(journal) {
        text.push_str(&format!(" {next}"));
    }
    text
}

/// "The orchard opens after 8 more fireflies caught.", while one is closed.
fn next_meadow_words(journal: &Journal) -> Option<String> {
    let next = journal.locked().into_iter().next()?;
    let more = next.opens_at() - journal.caught;
    let more = match more {
        1 => "1 more firefly".to_string(),
        n => format!("{n} more fireflies"),
    };
    Some(format!(
        "{} opens after {more} caught.",
        sentence_start(next.name())
    ))
}

fn midsummer_help(date: Date) -> String {
    if almanac::is_midsummer(date) {
        "Open tonight: the synchronous firefly show round a bonfire, and this year's keepsake."
            .to_string()
    } else {
        format!(
            "Opens on midsummer, the June solstice. {}",
            almanac::midsummer_countdown(date)
        )
    }
}

impl TitleGame for Meadow {
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
        let date = today(ctx);
        let journal = Journal::load(ctx);
        let place = journal.meadow();
        let play_help = almanac::season_words(date);
        let midsummer = almanac::is_midsummer(date);
        let mid_label = if midsummer {
            "Midsummer night".to_string()
        } else {
            labels::locked("Midsummer night")
        };
        let several = journal.next_meadow() != place;
        let play_help = if several {
            play_help
        } else {
            // One meadow open: no Meadow row yet, so the Play row says
            // when the next opens.
            match next_meadow_words(&journal) {
                Some(next) => format!("{play_help} {next}"),
                None => play_help,
            }
        };
        let mut rows = vec![MenuItem::new(
            play_label(ctx),
            move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                evening::start(ctx, Mode::Walk(place), false)
            },
        )
        .help(play_help)];
        if several {
            rows.push(
                MenuItem::new(
                    format!("Meadow: {}", place.name()),
                    |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let next = Journal::load(ctx).next_meadow();
                        progress::choose_meadow(ctx, next);
                        ctx.play_ui(hearken::UiSound::Move);
                        s.refresh(ctx, true);
                        ctx.say(format!("Meadow: {}.", next.name()));
                    },
                )
                .help(meadow_help(&journal)),
            );
        }
        rows.extend([
            MenuItem::new(mid_label, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let date = today(ctx);
                if almanac::is_midsummer(date) {
                    night::start(ctx, false);
                } else {
                    ctx.play_ui(hearken::UiSound::Back);
                    ctx.say(format!(
                        "Midsummer night opens on midsummer, the June solstice. {}",
                        almanac::midsummer_countdown(date)
                    ));
                }
            })
            .help(midsummer_help(date)),
        ]);
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice catch", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                evening::start(ctx, Mode::Practice, false)
            })
            .help("A small summer patch with one easy firefly just ahead. Nothing counts, in any season."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Fireflies caught, species found, your longest evening and midsummer nights, with your challenge results.".to_string()
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let journal = Journal::load(ctx);
        vec![MenuItem::new("Journal", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
            ctx.push(JournalMenu::new())
        })
        .help(format!(
            "Every firefly you have found, with its blink and note, and your chords and keepsakes: {} of {} species so far.",
            journal.species_found(),
            Species::ALL.len()
        ))]
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear every firefly's tone and each sound you need in the meadow, with its meaning."
            .to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A summer evening on the clock each day, each week and each month, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has walked an evening is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Journal::load(ctx).any_played()
    }
}
