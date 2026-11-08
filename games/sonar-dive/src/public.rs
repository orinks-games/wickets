// Sonar Dive: Pilot a small submarine by sonar: find wrecks and treasure, and steer clear of mines and currents.

pub mod cues;
pub mod events;
pub mod menus;
pub mod rules;
pub mod save;
pub mod screens;
pub mod tutorial;

use std::cell::Cell;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::Ctx;
use parlor_core::menus::{self as shared, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records, Rng};

use crate::menus::{ChallengesMenu, SoundCheck};
use crate::rules::{Config, Expedition};
use crate::save::{Saved, DEEP_UNLOCK};
use crate::screens::{ChallengeHost, DiveScreen, Mode};

pub use crate::menus::Dives;
pub use crate::screens::{CHALLENGES_NAME, TITLE_NAME};

/// The game's id: its sounds live under `sounds/sonar-dive/`.
pub const ID: &str = "sonar-dive";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Sonar Dive",
    category: Category::Action,
    blurb: "Pilot a small submarine by sonar: find wrecks and treasure, and steer clear of mines and currents.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <SonarDive as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Afternoon],
        seasons: &[parlor_core::Season::Summer],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

thread_local! {
    static NEXT_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Seeds the next expedition started from the title, so a test can replay
/// a sea exactly. Real play seeds from the clock.
pub fn seed_next_run(seed: u64) {
    NEXT_SEED.with(|s| s.set(Some(seed)));
}

fn run_rng() -> Rng {
    match NEXT_SEED.with(|s| s.take()) {
        Some(seed) => Rng::seeded(seed),
        None => Rng::from_time(),
    }
}

/// The config a scored or practice mode starts from.
pub fn config_for(mode: Mode) -> Config {
    match mode {
        Mode::Deep => Config::deep_start(),
        Mode::Practice => Config::practice(),
        Mode::Glow => Config::glow(),
        Mode::Expedition | Mode::Challenge(_) => Config::expedition(),
    }
}

/// Starts a new expedition in `mode` on top of the screen below.
pub(crate) fn start(ctx: &mut Ctx<Parlor>, mode: Mode) {
    let dive = dive_on(ctx, mode);
    ctx.push(dive);
}

/// A fresh dive screen for `mode` today: the glow dive's sea comes from
/// the night's new moon, the rest as [`new_dive`].
pub(crate) fn dive_on(ctx: &Ctx<Parlor>, mode: Mode) -> DiveScreen {
    match mode {
        Mode::Glow => {
            let today = parlor_core::challenges::today(ctx);
            DiveScreen::new(
                Expedition::new(config_for(mode), events::sea_rng(today)),
                mode,
                None,
            )
        }
        _ => new_dive(mode),
    }
}

/// A fresh dive screen for `mode`, outside a challenge.
pub(crate) fn new_dive(mode: Mode) -> DiveScreen {
    DiveScreen::new(Expedition::new(config_for(mode), run_rng()), mode, None)
}

fn haul(best: Option<u64>) -> String {
    match best {
        Some(b) => format!("best haul {b} coins"),
        None => "no haul banked yet".to_string(),
    }
}

/// Records: both best hauls, how deep you have been, and the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let saved = Saved::load(ctx);
    let best = haul(Records::best(ctx, ID, "best"));
    let deep = if saved.deep_unlocked() {
        haul(Records::best(ctx, ID, "deep"))
    } else {
        format!("locked until you reach dive {DEEP_UNLOCK}")
    };
    let expeditions = match saved.expeditions {
        1 => "1 expedition".to_string(),
        n => format!("{n} expeditions"),
    };
    let deepest = match saved.deepest {
        0 => "No dives yet.".to_string(),
        n => format!(
            "Deepest dive reached: dive {n}, {}.",
            screens::depth(f64::from(30 * n))
        ),
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            hearken::text::end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    vec![
        format!(
            "Records. Expedition: {best}. Deep start: {deep}. {deepest} {} finished.",
            hearken::text::sentence_start(&expeditions)
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder.
pub struct SonarDive;

/// The game's title screen.
pub type Title = shared::Title<SonarDive>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    shared::Title::new(SonarDive)
}

/// Saves the echo readout, from its Settings row.
fn set_readout(ctx: &mut Ctx<Parlor>, on: bool) {
    let mut saved = Saved::load(ctx);
    saved.readout = on;
    saved.store(ctx);
}

impl TitleGame for SonarDive {
    const ID: &'static str = ID;
    const NAME: &'static str = "Sonar Dive";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_TITLE), true);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let saved = Saved::load(ctx);
        let best = Records::best(ctx, ID, "best");
        let deep = if saved.deep_unlocked() {
            let deep_best = Records::best(ctx, ID, "deep");
            MenuItem::new(
                format!("Deep start at dive {DEEP_UNLOCK}, {}", haul(deep_best)),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| start(ctx, Mode::Deep),
            )
            .help(format!("An expedition that begins {} down, where treasure is worth more, mines are thicker and currents stronger.", screens::depth(120.0)))
        } else {
            MenuItem::new(
                labels::locked("Deep start"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say(format!(
                        "Deep start opens once an expedition reaches dive {DEEP_UNLOCK}."
                    ))
                },
            )
            .select_sound(None)
            .help(format!(
                "Reach dive {DEEP_UNLOCK} in an expedition to begin future expeditions there."
            ))
        };
        let mut rows = vec![
            MenuItem::new(
                format!("{}, expedition, {}", labels::PLAY, haul(best)),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| start(ctx, Mode::Expedition),
            )
            .help(
                "Dive from the boat, bank treasure each time you surface, and go deeper each dive.",
            ),
            deep,
        ];
        let today = parlor_core::challenges::today(ctx);
        if events::glow_open(today) {
            let best = match Records::best(ctx, ID, events::GLOW_BOARD) {
                Some(b) => format!(" Your best haul is {b} coins."),
                None => String::new(),
            };
            rows.push(
                MenuItem::new(
                    events::GLOW_DIVE,
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| start(ctx, Mode::Glow),
                )
                .help(format!(
                    "An expedition on tonight's glowing sea, under the new moon, the same for every player. A glow pearl lies on every dive, with an echo of its own.{best}"
                )),
            );
        } else {
            // Closed tonight: the row keeps its place and says when it opens.
            rows.push(
                MenuItem::new(
                    labels::locked(events::GLOW_DIVE),
                    |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                        let today = parlor_core::challenges::today(ctx);
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say(events::glow_locked_line(today))
                    },
                )
                .select_sound(None)
                .help(events::glow_locked_help(today)),
            );
        }
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice dive", |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, Mode::Practice)
            })
            .help("Endless air and harmless mines, to learn the sonar. Not scored."),
        )
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![shared::settings::toggle(
            "sonar_dive_call_outs",
            labels::call_outs,
            |ctx| Saved::peek(ctx).readout,
            set_readout,
            |_ctx, on| {
                Some(if on {
                    "Spoken call-outs: on. Each ping's nearest echoes are spoken."
                } else {
                    "Spoken call-outs: off. You hear the echoes alone."
                }
                .to_string())
            },
            "When on, each ping's nearest echoes are also spoken, with distance and clock direction.",
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
        "Your best hauls, your deepest dive and your challenge results.".to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Hear the gameplay cues and compare an echo from five positions.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    /// A player who has finished an expedition is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let saved = Saved::load(ctx);
        !shared::tutorial_read(ctx, ID) && saved.expeditions == 0 && saved.deepest == 0
    }

    fn intro_help(&self) -> Option<String> {
        Some("Up and Down move, Enter chooses, Escape goes back to the parlor.".to_string())
    }
}
