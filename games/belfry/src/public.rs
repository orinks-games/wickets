// Belfry: ring rows of bells to find the old ringer's secret peal.
//
// A code-breaking puzzle for the ear. The secret is a row of four bells
// chosen from six. The player builds a row, rings it, and hears the
// answer: a firm knock for each bell right and in its place, a soft tap
// for each right bell in the wrong place, then the count in words. The
// tower allows ten rows. Village lets a bell come twice; Plain does not.

pub mod briefing;
pub mod challenge;
pub mod events;
pub mod progress;
pub mod rules;
pub mod sound;
mod soundcheck;
pub mod tower;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Ringers, CHALLENGES_NAME};
pub use progress::Progress;
pub use soundcheck::SoundCheck;
pub use tower::Tower;

use rules::Mode;

/// The game's id: its sounds live under `sounds/belfry/`.
pub const ID: &str = "belfry";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a peal returns to.
pub const TITLE_NAME: &str = "belfry";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Belfry",
    category: Category::Puzzle,
    blurb: "Ring rows of bells to find the old ringer's secret peal.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Belfry as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Morning,
            parlor_core::story::PartOfDay::Evening,
        ],
        seasons: &[],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its briefing, then the peal. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// "best score 700, fewest rows 4" or "not solved yet".
pub fn bests(ctx: &mut Ctx<Parlor>, mode: Mode) -> String {
    let board = Progress::load(ctx).board(mode);
    match (Records::best(ctx, ID, mode.board()), board.fewest) {
        (Some(best), Some(rows)) => format!("best score {best}, fewest rows {rows}"),
        _ => "not solved yet".to_string(),
    }
}

/// A mode's rules and bests, for its row's help. An event row adds how
/// long it stays.
pub fn mode_help(ctx: &mut Ctx<Parlor>, mode: Mode) -> String {
    let today = parlor_core::challenges::today(ctx);
    let lead = match mode {
        Mode::Village => "The village peal".to_string(),
        Mode::Plain => "For beginners".to_string(),
        Mode::PracticeNight => events::lead(mode, today),
    };
    format!(
        "{lead}. {}. {}.{}",
        sentence_start(&mode.rules().describe()),
        sentence_start(&bests(ctx, mode)),
        events::stays(mode, today)
    )
}

/// Records: each mode's bests and streak, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let mut lines = Vec::new();
    for mode in Mode::ALL
        .into_iter()
        .chain(Mode::EVENTS)
    {
        let b = progress.board(mode);
        if b.played == 0 {
            if Mode::ALL.contains(&mode) {
                lines.push(format!("{}: not played yet.", mode.name()));
            }
            continue;
        }
        let mut line = format!("{}: {} of {} solved", mode.name(), b.solved, b.played);
        if let Some(rows) = b.fewest {
            line.push_str(&format!(", fewest rows {rows}"));
        }
        if let Some(best) = Records::best(ctx, ID, mode.board()) {
            line.push_str(&format!(", best score {best}"));
        }
        line.push_str(&format!(
            ", streak {}, best streak {}.",
            b.streak, b.best_streak
        ));
        lines.push(line);
    }
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!("Records. {}", lines.join(" ")),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct Belfry;

/// The game's title screen.
pub type Title = menus::Title<Belfry>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(Belfry)
}

impl TitleGame for Belfry {
    const ID: &'static str = ID;
    const NAME: &'static str = "Belfry";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let village = mode_help(ctx, Mode::Village);
        let plain = mode_help(ctx, Mode::Plain);
        let today = parlor_core::challenges::today(ctx);
        let events: Vec<TitleRow<Self>> = events::open_modes(today)
            .into_iter()
            .map(|mode| {
                let help = mode_help(ctx, mode);
                MenuItem::new(mode.name(), move |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    briefing::open(ctx, mode)
                })
                .help(help)
            })
            .collect();
        let mut rows = vec![
            MenuItem::new(
                format!("{}, Village", labels::PLAY),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| briefing::open(ctx, Mode::Village),
            )
            .help(village),
            MenuItem::new(
                format!("{}, the easier peal", Mode::Plain.name()),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| briefing::open(ctx, Mode::Plain),
            )
            .help(plain),
        ];
        rows.extend(events);
        if !events::PRACTICE.is_open(today) {
            // Closed today: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::PRACTICE_NAME),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::practice_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::practice_locked_help(today)),
            );
        }
        rows
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<hearken::screens::SettingsRow<Parlor>> {
        vec![parlor_core::briefing::row()]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Peals solved, fewest rows, best scores and streaks, with your challenge results."
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
            "A daily peal, five peals each week and a monthly cathedral peal, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has rung a peal is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any_played()
    }
}
