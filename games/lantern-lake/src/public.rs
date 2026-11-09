// Lantern Lake: Paddle a quiet lake at night and gather floating lanterns, each adding a note to the night's song.
//
// A calm game with no timer and no way to fail. You turn and paddle a
// canoe by ear, following the chimes of drifting lanterns; each one you
// gather adds a voice to an ambient song. Every night's lake is generated
// afresh, with its own weather, creatures and small discoveries that are
// kept in a saved journal. Daily, weekly and monthly challenges give
// everyone the same lake with a gentle goal.

pub mod cues;
pub mod events;
pub mod journal;
pub mod menus;
pub mod rules;
pub mod screens;
pub mod soundtrack;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{self as shared, labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

use crate::journal::Journal;
use crate::rules::{Feature, Mode};
use crate::screens::{journal_line, start, ChallengeHost, Launch};

pub use crate::menus::Lakes;
pub use crate::screens::{CHALLENGES_NAME, TITLE_NAME};

/// The game's id: its sounds live under `sounds/lantern-lake/`.
pub const ID: &str = "lantern-lake";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Lantern Lake",
    category: Category::Cozy,
    blurb: "Paddle a quiet lake at night and gather floating lanterns, each adding a note to the night's song.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <LanternLake as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Evening, parlor_core::story::PartOfDay::Night],
        seasons: &[parlor_core::Season::Summer],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its rule, then the lake. Leaving it returns to the
/// launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// The game on the shared title builder. `seed` makes the same lake every
/// time, for tests; `None` seeds from the clock.
#[derive(Default)]
pub struct LanternLake {
    pub seed: Option<u64>,
}

impl LanternLake {
    fn launch(&self, mode: Mode) -> Launch {
        Launch {
            mode,
            seed: self.seed,
        }
    }
}

/// The game's title screen.
pub type Title = shared::Title<LanternLake>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    shared::Title::new(LanternLake::default())
}

/// A title whose lakes are the same every time, for tests.
pub fn title_with_seed(seed: u64) -> Title {
    shared::Title::new(LanternLake { seed: Some(seed) })
}

fn count(n: u64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// Records: the most lanterns, the journal's tallies and the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let j = Journal::load(ctx);
    let most = match Records::best(ctx, ID, "lanterns") {
        Some(n) => count(n, "lantern", "lanterns"),
        None => "none yet".to_string(),
    };
    let free = if j.free_open() {
        match Records::best(ctx, ID, "free") {
            Some(n) => count(n, "lantern", "lanterns"),
            None => "none yet".to_string(),
        }
    } else {
        "locked until you finish night 1".to_string()
    };
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    let festival = match Records::best(ctx, ID, events::FESTIVAL_BOARD) {
        Some(n) => format!(
            " Lantern Festival night, most lanterns: {}.",
            count(n, "lantern", "lanterns")
        ),
        None => String::new(),
    };
    vec![
        format!(
            "Records. Most lanterns in a night: {most}.{festival} Free paddle, most lanterns: {free}. Nights finished: {}. Outings: {}. Lanterns gathered in all: {}. Places found: {} of {}.",
            j.nights,
            j.outings,
            j.lanterns,
            j.places(),
            Feature::ALL.len()
        ),
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

impl TitleGame for LanternLake {
    const ID: &'static str = ID;
    const NAME: &'static str = "Lantern Lake";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_TITLE), true);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let j = Journal::load(ctx);
        let next = j.next_night();
        let most = match Records::best(ctx, ID, "lanterns") {
            Some(n) => format!(", most lanterns in a night {n}"),
            None => String::new(),
        };
        let free = if j.free_open() {
            let best = match Records::best(ctx, ID, "free") {
                Some(n) => format!(", best {n} lanterns"),
                None => String::new(),
            };
            MenuItem::new(
                format!("Free paddle{best}"),
                |s: &mut Title, ctx: &mut Ctx<Parlor>| start(ctx, s.game.launch(Mode::Free), false),
            )
            .help("A wide lake where a new lantern appears whenever you gather one. Stay as long as you like.")
        } else {
            MenuItem::new(
                labels::locked("Free paddle"),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say("Free paddle opens once you finish your first night.")
                },
            )
            .select_sound(None)
            .help("Finish night 1 to open free paddle.")
        };
        let today = parlor_core::challenges::today(ctx);
        let best = Records::best(ctx, ID, events::FESTIVAL_BOARD);
        let festival = if events::festival_open(today) {
            MenuItem::new(events::FESTIVAL, |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, s.game.launch(Mode::Festival), false)
            })
            .help(events::festival_help(today, best))
        } else {
            // Out of season the row keeps its place and says when it opens.
            MenuItem::new(
                labels::locked(events::FESTIVAL),
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    let today = parlor_core::challenges::today(ctx);
                    ctx.say(events::festival_locked_line(today))
                },
            )
            .select_sound(None)
            .help(events::festival_locked_help(today, best))
        };
        let mut rows = vec![
            MenuItem::new(
                format!("{}, night {next}{most}", labels::PLAY),
                move |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    start(ctx, s.game.launch(Mode::Night(next)), false)
                },
            )
            .help(format!(
                "Night {next}: {} lanterns on a new lake. Each night is a little wider, with more lanterns drifting a little faster.",
                (3 + next as usize).min(rules::SONG_LAYERS)
            )),
            free,
        ];
        rows.push(festival);
        rows
    }

    fn practice_row(&mut self, _ctx: &mut Ctx<Parlor>) -> Option<TitleRow<Self>> {
        Some(
            MenuItem::new("Practice pond", |s: &mut Title, ctx: &mut Ctx<Parlor>| {
                start(ctx, s.game.launch(Mode::Pond), false)
            })
            .help("A tiny, still pond with 2 lanterns placed to learn on."),
        )
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let j = Journal::load(ctx);
        vec![
            MenuItem::new(journal_line(&j), |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(menus::JournalMenu::new())
            })
            .help("Read the places you have found and your tallies."),
        ]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "The most lanterns you have gathered, your journal's tallies and your challenge results."
            .to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(menus::SoundCheck::new());
    }

    fn sound_check_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Play the sounds you need on the lake.".to_string()
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(menus::ChallengeMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "Daily, weekly and monthly lakes with gentle goals, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player who has paddled any lake to the end is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let j = Journal::load(ctx);
        !shared::tutorial_read(ctx, ID) && !j.pond_done && j.outings == 0
    }

    fn intro_help(&self) -> Option<String> {
        Some("Up and Down move, Enter chooses, Escape goes back to the parlor.".to_string())
    }
}
