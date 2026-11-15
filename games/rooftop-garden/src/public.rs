// Rooftop Garden: Tend a rooftop garden, where each plant grows into a layer of music.
//
// A calm game with no way to fail. You walk a small grid of planters on a
// city rooftop, watering, picking off aphids, pruning and harvesting by
// sound. Seeds are silent, sprouts hum, and grown plants each play one
// part of the garden's song, all in one key. The garden lives on between
// sessions: days pass while you play, and harvests unlock new seeds and
// more planters. Daily, weekly and monthly challenges give everyone the
// same garden with a gentle goal.

pub mod breeding;
pub mod cues;
pub mod events;
pub mod menus;
pub mod rules;
pub mod screens;
pub mod song;
pub mod soundtrack;
pub mod tutorial;

use hearken::menu::MenuItem;
use hearken::screens::ConfirmScreen;
use hearken::text::end_sentence;
use hearken::Ctx;
use parlor_core::menus::{labels, TitleGame, TitleRow};
use parlor_core::{Category, Challenge, Challenges, GameInfo, Parlor, Period, Records};

use crate::rules::{Garden, Species};
use crate::screens::{start_home, Launch, Progress, GARDEN_KEY, PROGRESS_KEY};

pub use crate::menus::{ChallengeHost, ChallengesMenu, Gardens, CHALLENGES_NAME};
pub use crate::screens::TITLE_NAME;

/// The game's id: its sounds live under `sounds/rooftop-garden/`.
pub const ID: &str = "rooftop-garden";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Rooftop Garden",
    category: Category::Cozy,
    blurb: "Tend a rooftop garden, where each plant grows into a layer of music.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <RooftopGarden as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[parlor_core::story::PartOfDay::Morning],
        seasons: &[parlor_core::Season::Spring, parlor_core::Season::Summer],
    },
};

/// Opens this game's current `period` challenge from the launcher's list:
/// its goal and twist, then the garden. Leaving it returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

/// Your own garden in a few words: "day 4, 2 plants, 1 harvest".
fn garden_words(ctx: &mut Ctx<Parlor>, g: &Garden) -> String {
    let plants = g.plants().count();
    let mut s = format!(
        "day {}, {plants} {}, {} {}",
        g.day,
        if plants == 1 { "plant" } else { "plants" },
        g.harvests,
        if g.harvests == 1 {
            "harvest"
        } else {
            "harvests"
        }
    );
    if let Some(h) = Records::best(ctx, ID, "harmony").filter(|&h| h > 0) {
        s.push_str(&format!(", fullest harmony {h} parts"));
    }
    s
}

/// The Play row for your own garden.
fn garden_line(ctx: &mut Ctx<Parlor>, saved: &Option<Garden>) -> String {
    match saved {
        None => format!("{}, start your garden", labels::PLAY),
        Some(g) => format!(
            "{}, tend your garden: {}",
            labels::PLAY,
            garden_words(ctx, g)
        ),
    }
}

/// How many kinds of seed your own garden has unlocked.
fn seeds_open(ctx: &mut Ctx<Parlor>, saved: &Option<Garden>) -> usize {
    let book = crate::screens::load_collection(ctx);
    let (harvests, chorus) = saved.as_ref().map_or((0, 0), |g| (g.harvests, g.chorus));
    Species::ALL
        .iter()
        .filter(|s| s.open(harvests, chorus, &book))
        .count()
}

/// Records: your garden, its bests, the seeds and the challenges.
pub fn records_pages(ctx: &mut Ctx<Parlor>) -> Vec<String> {
    let saved: Option<Garden> = Records::load(ctx, GARDEN_KEY);
    let first = match &saved {
        None => "Records. You have not started a garden yet.".to_string(),
        Some(g) => format!("Records. Your garden: {}.", garden_words(ctx, g)),
    };
    let best = |ctx: &mut Ctx<Parlor>, board: &str| Records::best(ctx, ID, board).unwrap_or(0);
    let harvests = best(ctx, "harvests");
    let harmony = best(ctx, "harmony");
    let days = best(ctx, "days");
    let bests = format!(
        "Bests. Most harvests: {harvests}. Fullest harmony: {harmony} {}. Longest garden: {days} {}. Seeds unlocked: {} of {}.",
        if harmony == 1 { "part" } else { "parts" },
        if days == 1 { "day" } else { "days" },
        seeds_open(ctx, &saved),
        Species::ALL.len()
    );
    let challenges: Vec<String> = Period::ALL
        .into_iter()
        .map(|period| {
            let ch = Challenge::current(ctx, ID, period);
            end_sentence(&Challenges::summary(ctx, &ch))
        })
        .collect();
    let book = crate::screens::load_collection(ctx);
    let collection = format!(
        "Collection. Varieties found: {} of {}. Heirlooms: {}. Requests filled: {}. Keepsakes: {} of {}. Stars: {}.",
        book.found.len(),
        crate::breeding::Variety::count(),
        book.heirlooms(),
        book.thanks,
        book.keepsakes.len(),
        crate::breeding::Keepsake::ALL.len(),
        book.stars()
    );
    vec![
        first,
        bests,
        collection,
        format!("Challenges. {}", challenges.join(" ")),
    ]
}

/// The game on the shared title builder. A seed makes its gardens roll
/// the same weather and pests every time, for tests.
#[derive(Default)]
pub struct RooftopGarden {
    seed: Option<u64>,
}

impl RooftopGarden {
    fn launch(&self) -> Launch {
        Launch { seed: self.seed }
    }
}

/// The game's title screen.
pub type Title = parlor_core::menus::Title<RooftopGarden>;

/// A fresh title screen, as the launcher opens it.
pub fn title() -> Title {
    parlor_core::menus::Title::new(RooftopGarden::default())
}

/// A title whose gardens roll the same weather and pests every time, for
/// tests.
pub fn title_with_seed(seed: u64) -> Title {
    parlor_core::menus::Title::new(RooftopGarden { seed: Some(seed) })
}

impl TitleGame for RooftopGarden {
    const ID: &'static str = ID;
    const NAME: &'static str = "Rooftop Garden";
    const HELP: &'static str = HELP;
    const TITLE_NAME: &'static str = TITLE_NAME;

    fn music(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_TITLE), true);
    }

    fn play_rows(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let saved: Option<Garden> = Records::load(ctx, GARDEN_KEY);
        vec![MenuItem::new(
            garden_line(ctx, &saved),
            |s: &mut Title, ctx: &mut Ctx<Parlor>| start_home(ctx, s.game.launch()),
        )
        .help("Tend your own rooftop. Days pass only while you play; your garden is saved when you leave.")]
    }

    fn extras(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let saved: Option<Garden> = Records::load(ctx, GARDEN_KEY);
        vec![MenuItem::new(
            {
                let book = crate::screens::load_collection(ctx);
                let seeds = format!(
                    "Seed shed, {} of {} seeds",
                    seeds_open(ctx, &saved),
                    Species::ALL.len()
                );
                match book.found.len() {
                    0 => seeds,
                    1 => format!("{seeds}, 1 variety"),
                    n => format!("{seeds}, {n} varieties"),
                }
            },
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| ctx.push(menus::SeedShed::new()),
        )
        .help("Every seed and variety, your keepsakes, and the neighbours' requests.")]
    }

    fn destructive(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<TitleRow<Self>> {
        let saved: Option<Garden> = Records::load(ctx, GARDEN_KEY);
        if saved.is_none() {
            return vec![MenuItem::new(
                "Start the garden over, nothing to clear",
                |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                    ctx.say("No garden yet. Play starts one.")
                },
            )
            .select_sound(None)
            .help("Clears your own garden once you have one. Challenges and best records are not touched.")];
        }
        vec![MenuItem::new(
            "Start the garden over",
            |_s: &mut Title, ctx: &mut Ctx<Parlor>| {
                ctx.push(
                    ConfirmScreen::<Parlor>::new(
                        "Clear every planter and start your garden over from day 1? Your seeds go back in the shed too. Varieties, keepsakes and best records are kept.",
                        |ctx| {
                            Records::store(ctx, GARDEN_KEY, &Option::<Garden>::None);
                            ctx.pop_with("Garden cleared");
                        },
                    )
                    .yes_label("Yes, start over")
                    .no_label("No, keep my garden"),
                )
            },
        )
        .help("Clears your own garden. Challenges and best records are not touched.")]
    }

    fn lesson(&mut self, ctx: &mut Ctx<Parlor>) -> parlor_core::Lesson {
        crate::tutorial::lesson(ctx)
    }

    fn records(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        records_pages(ctx)
    }

    fn records_help(&mut self, _ctx: &mut Ctx<Parlor>) -> String {
        "Your garden, its most harvests and fullest harmony, your seeds, and your challenge results."
            .to_string()
    }

    fn sound_check(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(menus::SoundCheck::new());
    }

    fn challenges(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.push(ChallengesMenu::new());
    }

    fn challenges_help(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        let daily = Challenge::current(ctx, ID, Period::Daily);
        format!(
            "Daily, weekly and monthly gardens with goals, the same for every player. {}",
            daily.resets()
        )
    }

    /// A player with a garden, or who read the tutorial before the shared
    /// title, is no longer new.
    fn first_visit(&mut self, ctx: &mut Ctx<Parlor>) -> bool {
        let progress: Progress = Records::load(ctx, PROGRESS_KEY);
        let saved: Option<Garden> = Records::load(ctx, GARDEN_KEY);
        !parlor_core::menus::tutorial_read(ctx, ID) && !progress.tutorial && saved.is_none()
    }
}
