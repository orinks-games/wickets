// Signal Box: answer the dispatcher's calls before each train reaches the
// points.
//
// The player works a signal box on a night junction. The dispatcher calls
// an order, "Points left" or "Ring back", each with its own sound, and the
// player answers with the matching key inside a time window. A right
// answer passes a train and scores; a wrong or late one derails the run.
// The window shrinks every few trains, and a tick track keeps the tempo.
// Records keep the longest run and the shortest window reached, and daily,
// weekly and monthly timetables give every player the same calls.

pub mod challenge;
pub mod events;
pub mod play;
pub mod progress;
pub mod rules;
pub mod sound;
pub mod soundcheck;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

pub use challenge::{ChallengeHost, ChallengesMenu, Timetables, CHALLENGES_NAME};
pub use play::{Mode, Signal};
pub use progress::Progress;
pub use soundcheck::SoundCheck;

use rules::{trains_words, window_words, SHRINK_EVERY};

/// The game's id: its sounds live under `sounds/signal-box/`.
pub const ID: &str = "signal-box";

/// The game's name.
pub const NAME: &str = "Signal Box";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title screen's `State::name`, which leaving a run returns to.
pub const TITLE_NAME: &str = "signal_box";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: NAME,
    category: Category::Action,
    blurb: "Answer the dispatcher's calls before each train reaches the points.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <SignalBox as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Morning,
            parlor_core::story::PartOfDay::Evening,
        ],
        seasons: &[],
    },
};

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the timetable. Leaving it
/// returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Records: the night shift's bests, the tallies, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Progress::load(ctx);
    let best = match Records::best(ctx, ID, progress::SHIFT_BOARD) {
        Some(b) => format!("best score {b}"),
        None => "no score yet".to_string(),
    };
    let run = match p.best_run {
        0 => "no run yet".to_string(),
        n => format!("longest run {}", trains_words(n)),
    };
    let window = match p.shortest_window {
        Some(w) => format!("shortest window {}", window_words(w)),
        None => "no window reached yet".to_string(),
    };
    let calls = match p.practice_calls {
        1 => "1 practice call".to_string(),
        n => format!("{n} practice calls"),
    };
    let shifts = match p.shifts {
        1 => "1 night shift".to_string(),
        n => format!("{n} night shifts"),
    };
    let mut extra = String::new();
    if let Some(b) = Records::best(ctx, ID, events::GALA_BOARD) {
        extra.push_str(&format!(" {}: best score {b}.", events::GALA_NAME));
    }
    match p.thirteenth_trains {
        0 => {}
        1 => extra.push_str(" You have seen the thirteenth train go by once."),
        n => extra.push_str(&format!(
            " You have seen the thirteenth train go by {n} times."
        )),
    }
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!("Records. Night shift: {best}, {run}, {window}.{extra}"),
        format!(
            "Worked so far: {shifts}, {} passed in all, and {calls} answered.",
            trains_words(p.trains)
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct SignalBox;

/// The game's title screen.
pub type Title = menus::Title<SignalBox>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    menus::Title::new(SignalBox)
}

impl TitleGame for SignalBox {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Progress::load(ctx);
        let label = match p.best_run {
            0 => format!("{}, night shift", labels::PLAY),
            n => format!(
                "{}, night shift, best run {}",
                labels::PLAY,
                trains_words(n)
            ),
        };
        let best = match Records::best(ctx, ID, progress::SHIFT_BOARD) {
            Some(b) => format!(" Best score {b}."),
            None => String::new(),
        };
        let today = parlor_core::challenges::today(ctx);
        let thirteenth = if events::is_thirteenth(today) {
            format!(" {}", events::THIRTEENTH_WORDS)
        } else {
            String::new()
        };
        let mut rows = vec![MenuItem::new(label, |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
            ctx.push(Signal::new(Mode::Shift))
        })
        .help(format!(
            "Calls until a train derails, the window shrinking every {SHRINK_EVERY} trains.{best}{thirteenth}"
        ))];
        if events::GALA.is_open(today) {
            let best = match Records::best(ctx, ID, events::GALA_BOARD) {
                Some(b) => format!(" Your steam gala best is {b}."),
                None => String::new(),
            };
            rows.push(
                MenuItem::new(
                    events::GALA_NAME,
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(Signal::new(Mode::SteamGala)),
                )
                .help(format!(
                    "The steam gala, the last weekend of the month: a night shift of its own. {} It keeps its own best score, apart from the night shift.{best}",
                    events::SPECIAL_WORDS
                )),
            );
        } else {
            // Closed today: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::GALA_NAME),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::gala_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::gala_locked_help(today)),
            );
        }
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice room", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(Signal::new(Mode::Practice))
            })
            .help("Calls with no clock and nothing to lose: learn each sound and its key."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best score, longest run and shortest window, and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear every call and cue in the game, with its name and the key that answers it."
            .to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily, a weekly and a monthly timetable, the same calls for every player. {}",
            daily.resets()
        )
    }

    /// A player who has worked the box is no longer new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any()
    }
}
