// Birdsong Trail: walk a woodland trail at dawn by ear, and befriend the
// birds by answering their songs.
//
// A calm game with no way to fail. Birds sing around you, each from its
// true place; walk close to one and it sings to you, and you whistle its
// song back note for note, from three notes, low, middle and high. Each
// bird befriended goes into a field guide with its song in words. The
// birds follow the real seasons, four trails open as the guide grows, and
// special days of the bird year open special walks: the dawn chorus, the
// migration counts of the equinoxes.

pub mod birds;
pub mod challenge;
pub mod chorus;
pub mod count;
pub mod days;
pub mod field;
pub mod guide;
pub mod rules;
pub mod sound;
pub mod soundcheck;
pub mod trails;
pub mod tutorial;
pub mod walk;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::{end_sentence, sentence_start};
use hearken::{Ctx, UiSound};
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{
    challenges, Category, Challenge, Challenges, Date, GameInfo, Happening, Parlor, ParlorAction,
    Period, Records,
};

pub use challenge::{ChallengeHost, ChallengesMenu, CHALLENGES_NAME};
pub use chorus::ChorusScreen;
pub use count::CountScreen;
pub use days::Special;
pub use guide::Guide;
pub use soundcheck::SoundCheck;
pub use trails::Trail;
pub use walk::{Mode, WalkScreen};

/// The game's id: its sounds live under `sounds/birdsong-trail/`.
pub const ID: &str = "birdsong-trail";

/// The game's name.
pub const NAME: &str = "Birdsong Trail";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a walk returns to.
pub const TITLE_NAME: &str = "birdsong_trail";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Relaxing,
    blurb: "Walk a trail at dawn by ear, and befriend the birds by answering their songs.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <BirdsongTrail as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Dawn,
            parlor_core::story::PartOfDay::Morning,
        ],
        seasons: &[parlor_core::Season::Spring],
    },
};

/// The special walk the game holds on `date`, for the launcher: the dawn
/// chorus or a migration count.
pub fn event(date: Date) -> Option<Happening> {
    days::event(date).map(|h| h.on_row("Special walk"))
}

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the run. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the field guide, the walks, the special walks, the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let g = Guide::load(ctx);
    let walks = match g.walks {
        1 => "1 walk".to_string(),
        n => format!("{n} walks"),
    };
    let chorus = match g.choruses {
        0 => "no dawn chorus yet".to_string(),
        _ => format!("dawn chorus best {} birds", g.chorus_best),
    };
    let count = match Records::best(ctx, ID, count::COUNT_BOARD) {
        Some(b) => format!("migration count best {b} points"),
        None => "no migration count yet".to_string(),
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
            "Records. Field guide: {} of {} birds, {} of {} songs. {}, {} steps in all.",
            g.birds(),
            birds::ALL.len(),
            g.songs(),
            guide::total_songs(),
            sentence_start(&walks),
            g.steps
        ),
        format!("Special walks: {chorus}, {count}."),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The title row for `trail`.
fn trail_row(g: &Guide, trail: Trail, season: parlor_core::Season) -> TitleRow<BirdsongTrail> {
    let open = g.opens(trail);
    let label = match (trail, open) {
        (Trail::Woodland, _) => format!("{}, {}", labels::PLAY, trail.name()),
        (_, true) => sentence_start(trail.name()),
        (_, false) => labels::locked(&sentence_start(trail.name())),
    };
    let kinds = birds::on_trail(trail.habitat(), season).len();
    let help = if open {
        format!(
            "Walk the {} at dawn, in {}. {kinds} kinds of bird sing there this season.",
            trail.name(),
            season.name()
        )
    } else {
        let more = trail.needs() - g.birds();
        let more = if more == 1 {
            "1 to go".to_string()
        } else {
            format!("{more} to go")
        };
        format!(
            "Meet {} kinds of bird to open the {}, {more}.",
            trail.needs(),
            trail.name()
        )
    };
    let said = help.clone();
    MenuItem::new(label, move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
        if Guide::load(ctx).opens(trail) {
            let s = WalkScreen::trail(ctx, trail);
            ctx.push(s);
        } else {
            ctx.play_ui(UiSound::Error);
            ctx.say(format!("Locked. {said}"));
        }
    })
    .help(help)
}

/// The special walk's title row.
fn special_row(today: Date) -> TitleRow<BirdsongTrail> {
    match days::special(today) {
        Some(s) => MenuItem::new(
            format!("Special walk: {}", s.name().to_lowercase()),
            move |_s: &mut Title, ctx: &mut Ctx<Parlor>| open_special(ctx, s),
        )
        .help(format!("{} Open today, for {}.", s.line(), s.day_name())),
        None => {
            let help = format!(
                "Opens on special days of the bird year. {}",
                days::countdown(today)
            );
            let said = help.clone();
            MenuItem::new(
                labels::locked("Special walk"),
                move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.play_ui(UiSound::Error);
                    ctx.say(format!("Locked. {said}"));
                },
            )
            .help(help)
        }
    }
}

/// Opens a special walk.
pub fn open_special(ctx: &mut Ctx<Parlor>, special: Special) {
    match special {
        Special::DawnChorus => {
            let s = ChorusScreen::today(ctx);
            ctx.push(s)
        }
        Special::SpringMigration | Special::AutumnMigration => {
            let s = CountScreen::today(ctx);
            ctx.push(s)
        }
    }
}

/// The game on the shared title builder.
pub struct BirdsongTrail;

/// The game's title screen.
pub type Title = menus::Title<BirdsongTrail>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(BirdsongTrail)
}

impl TitleGame for BirdsongTrail {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let g = Guide::load(ctx);
        let today = challenges::today(ctx);
        let season = days::season(today);
        let mut rows: Vec<TitleRow<Self>> = Trail::ALL
            .iter()
            .map(|t| trail_row(&g, *t, season))
            .collect();
        rows.push(special_row(today));
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice glade", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(WalkScreen::practice())
            })
            .help("A short glade with two birds and two-note songs, and nothing kept: learn to answer."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your field guide, walks and special walks, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear every bird voice, step and cue in the game, with what it means.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily walk, a weekly chorus and a monthly count, the same birds for every player. {}",
            daily.resets()
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let g = Guide::load(ctx);
        vec![MenuItem::new(
            field::row_label(&g),
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(field::FieldGuide::new()),
        )
        .help("Every bird you have met, its song in words, and the songs you have learned.")]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![menus::settings::toggle(
            "birdsong_trail_call_outs",
            labels::call_outs,
            |ctx| Guide::peek(ctx).call_outs,
            |ctx, on| {
                let mut g = Guide::load(ctx);
                g.call_outs = on;
                g.store(ctx);
            },
            |ctx, on| {
                Some(if on {
                    "Spoken call-outs: on. After a bird sings to you, its notes are spoken."
                        .to_string()
                } else {
                    format!(
                        "Spoken call-outs: off. Answer by ear, and {} speaks the notes when you want them.",
                        ctx.control_name(ParlorAction::Hint)
                    )
                })
            },
            "When on, the notes of every song a bird sings to you are spoken after it, low, middle or high.",
        ),
            parlor_core::units::row(),
        ]
    }

    /// A player who has walked is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Guide::load(ctx).any()
    }
}
