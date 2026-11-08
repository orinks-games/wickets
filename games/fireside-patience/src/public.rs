// Fireside Patience: a quiet game of patience by the fire, played by ear.
//
// Klondike solitaire with spoken navigation. Seven piles, a stock and
// four foundations lie in one row; the player moves along it, picks up a
// card or a run, and places it. Relaxed deals come from a pool the solver
// has proved winnable, undo is unlimited, and there is no clock, no move
// limit and no loss.

pub mod challenge;
pub mod events;
pub mod play;
pub mod pool;
pub mod progress;
pub mod rules;
pub mod solver;
pub mod sound;
mod soundcheck;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Deals, CHALLENGES_NAME};
pub use play::PlayScreen;
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{moves_words, POINTS};

/// The game's id: its sounds live under `sounds/fireside-patience/`.
pub const ID: &str = "fireside-patience";

/// The game's name.
pub const NAME: &str = "Fireside Patience";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a deal returns to.
pub const TITLE_NAME: &str = "fireside_patience";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Table,
    blurb: "A quiet game of patience by the fire, with deals that can always be won.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <FiresidePatience as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Evening,
            parlor_core::story::PartOfDay::Night,
        ],
        seasons: &[parlor_core::Season::Autumn, parlor_core::Season::Winter],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its briefing, then the deal. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the relaxed bests, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let progress = Progress::load(ctx);
    let first = if progress.deals == 0 {
        "Records. No deal played yet.".to_string()
    } else {
        let deals = match progress.deals {
            1 => "1 deal".to_string(),
            n => format!("{n} deals"),
        };
        let fewest = match progress.fewest {
            Some(f) => format!(" Fewest moves: {f}."),
            None => String::new(),
        };
        let best = match parlor_core::Records::best(ctx, ID, progress::RELAXED_BOARD) {
            Some(b) => format!(" Best score: {b} points."),
            None => String::new(),
        };
        format!(
            "Records. Relaxed: {} won of {deals}.{fewest}{best}",
            progress.wins
        )
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    let mut first = first;
    for (name, board) in [(events::RABBIT_NAME, events::RABBIT_BOARD)] {
        if let Some(b) = Records::best(ctx, ID, board) {
            first.push_str(&format!(" {name}: best {b} points."));
        }
    }
    vec![first, format!("Challenges. {}", challenges.join(" "))]
}

/// A row for each event open today, after the Play row.
fn event_rows(ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<FiresidePatience>> {
    let today = parlor_core::challenges::today(ctx);
    let mut rows = Vec::new();
    if events::RABBIT.is_open(today) {
        let best = Records::best(ctx, ID, events::RABBIT_BOARD);
        rows.push(
            MenuItem::new(
                events::RABBIT_NAME,
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.push(PlayScreen::event(events::EventDeal::Rabbit, today));
                },
            )
            .help(events::rabbit_help(best)),
        );
    } else {
        // Closed today: the row keeps its place and says when it opens.
        rows.push(
            MenuItem::new(
                labels::locked(events::RABBIT_NAME),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.play_ui(hearken::UiSound::Error);
                    ctx.say(events::rabbit_locked_line(today))
                },
            )
            .select_sound(None)
            .help(events::rabbit_locked_help(today)),
        );
    }
    rows
}

/// The game on the shared title builder.
pub struct FiresidePatience;

/// The game's title screen.
pub type Title = menus::Title<FiresidePatience>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(FiresidePatience)
}

impl TitleGame for FiresidePatience {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::fireside(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let progress = Progress::load(ctx);
        let record = match progress.fewest {
            Some(f) => format!(" {} won, fewest {}.", progress.wins, moves_words(f)),
            None => String::new(),
        };
        let mut rows = vec![MenuItem::new(
            format!("{}, Relaxed", labels::PLAY),
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(PlayScreen::new()),
        )
        .help(format!(
            "Draw one, unlimited undo, and every deal can be won. {POINTS} points less 1 a move.{record}"
        ))];
        rows.extend(event_rows(ctx));
        rows
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Deals won, fewest moves and best score, with your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Play the card and table cues you need in a deal.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily deal, a weekly draw three deal and five deals a month, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has started a deal is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && Progress::load(ctx).deals == 0
    }
}
