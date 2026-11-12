// Tightrope: Cross ever-longer wires high above the city, keeping your balance as the wind gusts.
//
// Your balance is a tone panned and pitched by your lean. Shift your
// weight to stay over the wire, step when balanced, ride out gusts that
// are always warned ahead, and deal with pigeons that land on the wire.
// Ten crossings make the skyline; a night skyline opens after it, and
// daily, weekly and monthly challenges give everyone the same wires.

pub mod cues;
pub mod menus;
pub mod progress;
pub mod rules;
pub mod screens;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self as shared, labels, TitleGame, TitleRow};
use parlor_core::{pace, Category, Challenge, Challenges, GameInfo, Parlor, Period};

use crate::menus::time_text;
use crate::rules::{skyline_level, Mode, SKYLINE_LEN};
use crate::screens::{start, ChallengeHost, Launch};

pub use crate::menus::Wires;
pub use crate::screens::{CHALLENGES_NAME, TITLE_NAME};

/// The game's id: its sounds live under `sounds/tightrope/`.
pub const ID: &str = "tightrope";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Tightrope",
    category: Category::Action,
    blurb: "Cross ever-longer wires high above the city, keeping your balance as the wind gusts.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: None,
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <Tightrope as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Afternoon],
        seasons: &[],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its rule, then the wire. Leaving it returns to the
/// launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// The game on the shared title builder. `seed` replays the same wind on
/// every run, for tests; `None` seeds from the clock.
#[derive(Default)]
pub struct Tightrope {
    pub seed: Option<u64>,
}

impl Tightrope {
    fn launch(&self, mode: Mode) -> Launch {
        Launch {
            mode,
            seed: self.seed,
        }
    }
}

/// The game's title screen.
pub type Title = shared::Title<Tightrope>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    shared::Title::new(Tightrope::default())
}

/// A title whose runs replay the same wind every time, for tests.
pub fn title_with_seed(seed: u64) -> Title {
    shared::Title::new(Tightrope { seed: Some(seed) })
}

/// "best 30, relaxed best 41", leaving out a board with no score, or
/// "no score yet".
fn best_text(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    pace::scored_bests(ctx, ID, board, "best").unwrap_or_else(|| "no score yet".to_string())
}

/// Records: the best scores, the furthest crossing, every best time and
/// the challenges, a page at a time.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let furthest = progress::furthest(ctx);
    let skyline = best_text(ctx, "skyline");
    let night = if progress::night_unlocked(ctx) {
        best_text(ctx, "night")
    } else {
        format!("locked until you finish all {SKYLINE_LEN} skyline crossings")
    };
    let practice = if progress::trained(ctx) || shared::tutorial_read(ctx, ID) {
        "walked to the end"
    } else {
        "not yet walked to the end"
    };
    let mut pages = vec![format!(
        "Records. Skyline run: {skyline}, furthest crossing {furthest} of {SKYLINE_LEN}. Night skyline: {night}. Practice wire: {practice}."
    )];
    let times: Vec<String> = (1..=SKYLINE_LEN)
        .map(|n| {
            let level = skyline_level(n, false);
            if n <= progress::unlocked(ctx) {
                let best = progress::best_times(ctx, &level.time_board());
                format!("Crossing {n}, {}.", time_text(best))
            } else {
                format!("Crossing {n}, locked.")
            }
        })
        .collect();
    pages.push(format!("Best times. {}", times.join(" ")));
    if progress::night_unlocked(ctx) {
        let night: Vec<String> = (1..=SKYLINE_LEN)
            .map(|n| {
                let best = progress::best_times(ctx, &format!("night {n}"));
                format!("Night crossing {n}, {}.", time_text(best))
            })
            .collect();
        pages.push(format!("Night best times. {}", night.join(" ")));
    }
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    pages.push(format!("Challenges. {}", challenges.join(" ")));
    pages
}

impl TitleGame for Tightrope {
    const ID: &'static str = ID;
    const NAME: &'static str = "Tightrope";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_TITLE), true);
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![pace::row(ID)]
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let furthest = progress::furthest(ctx);
        let skyline = best_text(ctx, "skyline");
        let night = if progress::night_unlocked(ctx) {
            let night = best_text(ctx, "night");
            MenuItem::new(
                format!("Night skyline, {night}"),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    start(ctx, s.game.launch(Mode::Night), false)
                },
            )
            .help("The skyline after dark: a twitchier wire, harder wind and busier pigeons.")
        } else {
            MenuItem::new(
                labels::locked("Night skyline"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say(format!(
                        "Night skyline opens once you finish all {SKYLINE_LEN} skyline crossings."
                    ))
                },
            )
            .select_sound(Some(cues::LOCKED))
            .help(format!(
                "Finish all {SKYLINE_LEN} skyline crossings to open the night skyline."
            ))
        };
        vec![
            MenuItem::new(
                format!(
                    "{}, skyline run, {skyline}, furthest crossing {furthest} of {SKYLINE_LEN}",
                    labels::PLAY
                ),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    start(ctx, s.game.launch(Mode::Skyline), false)
                },
            )
            .help("Ten crossings across the city, each longer, higher and windier than the last, with 3 lives."),
            night,
            MenuItem::new(
                format!(
                    "Crossing select, {} of {SKYLINE_LEN} open",
                    progress::unlocked(ctx)
                ),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.push(menus::CrossingSelect::new(s.game.seed))
                },
            )
            .help("Walk one crossing alone for its best time."),
        ]
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice wire", |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, s.game.launch(Mode::Training), false)
            })
            .help("A short, low wire where every sound is explained as it comes, and falling costs nothing."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your best scores, your furthest crossing, every best time and your challenge results."
            .to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(menus::SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear each sound you need on the wire, with its meaning, starting with the balance tone."
            .to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(menus::ChallengeMenu::new());
    }

    /// A player who has walked any wire to the end is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !shared::tutorial_read(ctx, ID) && !progress::trained(ctx) && progress::furthest(ctx) == 0
    }

    fn intro_help(&self) -> Option<String> {
        Some("Up and Down move, Enter chooses, Escape goes back to the parlor.".to_string())
    }
}
