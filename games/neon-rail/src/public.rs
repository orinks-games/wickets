// Neon Rail: Black Sky Run: A three-lane sci-fi rail run: weave through traffic, clear jams and survive the stress meter.
//
// A port of the NVGT challenge build (the authored two-minute corridor,
// the guided run, every vehicle and hazard, the stress meter, priority
// traffic, emergency tools, scoring, debrief and story archive), grown
// toward its roguelite design: Black Sky Runs of five sectors with
// upgrades and route choices between them, a workshop of permanent parts
// bought with run currency, an endless rail, and an archive of logs
// recovered across runs.

pub mod challenge;
pub mod cues;
pub mod events;
pub mod menus;
pub mod profile;
pub mod rules;
pub mod screens;
pub mod script;
pub mod story;
pub mod strands;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::SettingsRow;
use hearken::text::end_sentence;
use hearken::{Bus, Ctx};
use parlor_core::menus::{self as shared, labels, TitleGame, TitleRow};
use parlor_core::{pace, Category, Challenge, Challenges, GameInfo, Parlor, Period};

use crate::profile::Profile;
use crate::rules::Mode;
use crate::screens::{launch, Launch};

pub use crate::challenge::CHALLENGES_NAME;
pub use crate::screens::TITLE_NAME;

/// The game's id: its sounds live under `sounds/neon-rail/`.
pub const ID: &str = "neon-rail";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Neon Rail: Black Sky Run",
    category: Category::Action,
    blurb: "A three-lane sci-fi rail run: weave through traffic, clear jams and survive the stress meter.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <NeonRail as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Night],
        seasons: &[],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(challenge::ChallengeHost::new(period));
}

/// "best 500, relaxed best 400", either alone, or "no score yet".
fn best_text(ctx: &mut Ctx<Parlor>, mode: Mode) -> String {
    pace::scored_bests(ctx, ID, mode.board(), "best").unwrap_or_else(|| "no score yet".to_string())
}

/// Records: every mode's best, the pilot's progress, then the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let p = Profile::load(ctx);
    let black_sky = best_text(ctx, Mode::BlackSky);
    let corridor = best_text(ctx, Mode::Challenge);
    let endless = if p.endless_unlocked() {
        format!(
            "{}, deepest sector {}",
            best_text(ctx, Mode::Endless),
            p.best_endless_sector
        )
    } else {
        "locked".to_string()
    };
    let cold = if p.endless_unlocked() {
        let mut cold = best_text(ctx, Mode::ColdStart);
        if p.best_cold_sector > 0 {
            cold.push_str(&format!(
                ", finished {}, furthest sector {}",
                p.cold_clears, p.best_cold_sector
            ));
        }
        cold
    } else {
        "locked".to_string()
    };
    let mut first = format!(
        "Records. Black Sky Run: {black_sky}. Challenge corridor: {corridor}. Endless rail: {endless}. Cold start: {cold}."
    );
    if let Some(best) = pace::scored_bests(ctx, ID, Mode::FridayNight.board(), "best") {
        first.push_str(&format!(" Friday night run: {best}."));
    }
    let mut pages = vec![first];
    pages.extend(p.pages());
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

/// The game on the shared title builder. `seed` makes every run replay
/// the same rail, for tests.
#[derive(Default)]
pub struct NeonRail {
    pub seed: Option<u64>,
}

/// The game's title screen.
pub type Title = shared::Title<NeonRail>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    shared::Title::new(NeonRail::default())
}

/// A title whose runs replay the same rail every time, for tests.
pub fn title_with_seed(seed: u64) -> Title {
    shared::Title::new(NeonRail { seed: Some(seed) })
}

impl NeonRail {
    fn launch(&self, mode: Mode) -> Launch {
        Launch {
            mode,
            seed: self.seed,
            day: None,
        }
    }
}

/// Starts a run from the title.
fn launch_run(ctx: &mut Ctx<Parlor>, run: Launch) {
    launch(ctx, run, false)
}

/// Turns spoken call-outs on or off, saved.
/// Saves the call-outs, from their Settings row.
fn set_callouts(ctx: &mut Ctx<Parlor>, on: bool) {
    let mut p = Profile::load(ctx);
    p.quiet_callouts = !on;
    p.save(ctx);
}

impl TitleGame for NeonRail {
    const ID: &'static str = ID;
    const NAME: &'static str = "Neon Rail: Black Sky Run";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_TITLE), true);
    }

    fn stop_music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Profile::load(ctx);
        let guided = p.needs_tutorial();
        let black_sky = best_text(ctx, Mode::BlackSky);
        let corridor = best_text(ctx, Mode::Challenge);
        let endless = best_text(ctx, Mode::Endless);
        let (play, play_help) = if guided {
            (
                format!("{}, guided run", labels::PLAY),
                "Your first run is the guided training route, because this save has not finished training.".to_string(),
            )
        } else {
            (
                format!("{}, Black Sky Run, {black_sky}", labels::PLAY),
                format!(
                    "Five sectors with an upgrade and a route choice after each, then extraction. Rank {}, credits {}, best score {}{}.",
                    p.rank(),
                    p.credits,
                    p.best_score,
                    crate::profile::relaxed_part(p.relaxed_best_score)
                ),
            )
        };
        let endless_row = if p.endless_unlocked() {
            MenuItem::new(
                format!(
                    "Endless rail, {endless}, deepest sector {}",
                    p.best_endless_sector
                ),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    launch(ctx, s.game.launch(Mode::Endless), false)
                },
            )
            .help("Sectors without end, each harder than the last, until the hull or the network gives out.")
        } else {
            MenuItem::new(
                labels::locked("Endless rail"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.audio.play(cues::DENIED, Bus::UI);
                    ctx.say(
                        "Locked. Finish a Black Sky Run to extraction to open the endless rail.",
                    )
                },
            )
            .select_sound(None)
            .help("Finish a Black Sky Run to extraction to open the endless rail.")
        };
        let cold_row = if p.endless_unlocked() {
            MenuItem::new(
                format!("Cold start, {}", best_text(ctx, Mode::ColdStart)),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    launch(ctx, s.game.launch(Mode::ColdStart), false)
                },
            )
            .help("A bare skimmer, nine sectors, one life. It keeps its own board and pays no credits, parts or intel.")
        } else {
            MenuItem::new(
                labels::locked("Cold start"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.audio.play(cues::DENIED, Bus::UI);
                    ctx.say("Locked. Finish a Black Sky Run to extraction to open the cold start.")
                },
            )
            .select_sound(None)
            .help("Finish a Black Sky Run to extraction to open the cold start: a bare skimmer, nine sectors, one life.")
        };
        let today = parlor_core::challenges::today(ctx);
        let friday_row = if events::friday_open(today) {
            let help = events::friday_help(
                pace::best(ctx, ID, Mode::FridayNight.board()),
                pace::best(
                    ctx,
                    ID,
                    &parlor_core::Pace::Relaxed.board(Mode::FridayNight.board()),
                ),
            );
            MenuItem::new(
                events::FRIDAY,
                move |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let launch = Launch {
                        day: Some(today),
                        ..s.game.launch(Mode::FridayNight)
                    };
                    launch_run(ctx, launch)
                },
            )
            .help(help)
        } else {
            // Closed today: the row keeps its place and says when it opens.
            MenuItem::new(
                labels::locked(events::FRIDAY),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.audio.play(cues::DENIED, Bus::UI);
                    ctx.say(events::friday_locked_line(today))
                },
            )
            .select_sound(None)
            .help(events::friday_locked_help(today))
        };
        let mut rows = vec![
            MenuItem::new(play, |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                let mode = if Profile::load(ctx).needs_tutorial() {
                    Mode::Tutorial
                } else {
                    Mode::BlackSky
                };
                launch(ctx, s.game.launch(mode), false)
            })
            .help(play_help),
            MenuItem::new(
                format!("Challenge corridor, {corridor}"),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    launch(ctx, s.game.launch(Mode::Challenge), false)
                },
            )
            .help("The original route: six sectors in two minutes, the same every time."),
            endless_row,
            cold_row,
        ];
        rows.push(friday_row);
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice run", |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                launch(ctx, s.game.launch(Mode::Tutorial), false)
            })
            .help("The guided one-minute route, with a tutorial page as each new sound arrives."),
        )
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let p = Profile::load(ctx);
        format!(
            "Your best scores, pilot rank {}, credits, unlocked bonuses and challenge results.",
            p.rank()
        )
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(menus::SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Play the sounds you need on the rail.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(challenge::ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "A daily sector sprint, a weekly long haul and the monthly Black Sky event: the same rail for every pilot, on a standard skimmer. {}",
            daily.resets()
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let p = Profile::load(ctx);
        vec![
            MenuItem::new(
                format!(
                    "Workshop, {} credits, {}",
                    p.credits,
                    profile::parts_phrase(p.parts)
                ),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(menus::Workshop::new()),
            )
            .help("Spend credits and parts on permanent parts for the skimmer."),
            MenuItem::new(
                format!(
                    "Story archive, {} of {} logs",
                    p.logs_recovered(),
                    story::total_logs()
                ),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(menus::Archive::new()),
            )
            .help("The Black Sky City logs you have recovered."),
        ]
    }

    fn settings(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![
            shared::settings::toggle(
                "neon_rail_call_outs",
                labels::call_outs,
                |ctx| !Profile::peek(ctx).quiet_callouts,
                set_callouts,
                |_ctx, _on| None,
                "When on, approaching traffic, hazards and pickups are named, such as Debris left. Off stops these announcements. Status and Hint still speak.",
            ),
            pace::row(ID),
        ]
    }

    /// A pilot who has flown a run is not new, tutorial or not.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        !shared::tutorial_read(ctx, ID) && Profile::load(ctx).runs_started == 0
    }

    fn intro_help(&self) -> Option<String> {
        Some(
            "Up and Down choose, Enter starts or opens, Escape goes back to the parlor."
                .to_string(),
        )
    }
}
