// Winter Wonderland: Winter holidays: sort shaken gifts by their sound, then answer children's letters at the Wish Desk.
//
// Two parts and a story that plays both. Gift Sort ([`gifts`]): shake
// each gift on the workshop conveyor, hear what is inside and send it to
// whoever wished for it before it rides off the belt. The Wish Desk
// ([`desk`]): read each child's letter, find its gift on the shelf by
// shaking, and wrap it, with no clock. Midnight Countdown ([`night`]), a
// New Year's Eve rhythm game, keeps its own row.

pub mod challenge;
pub mod count_run;
pub mod desk;
pub mod desk_music;
pub mod desk_run;
pub mod events;
pub mod gifts;
pub mod mode;
pub mod night;
pub mod progress;
pub mod results;
pub mod sort_run;
pub mod sound;
pub mod soundcheck;
pub mod title;
pub mod tutorial;

use std::cell::Cell;

use hearken::Ctx;
use parlor_core::{Category, GameInfo, Parlor, Period, Rng};

pub use challenge::{ChallengeHost, ChallengesMenu, Parties, Plan, CHALLENGES_NAME};
pub use count_run::CountRun;
pub use desk_run::DeskRun;
pub use mode::Mode;
pub use sort_run::SortRun;
pub use soundcheck::SoundCheck;
pub use title::{title, Title, Winter};

/// The game's id: its sounds live under `sounds/winter-wonderland/`.
pub const ID: &str = "winter-wonderland";

/// The help document, compiled in.
pub const HELP: &str = include_str!("../help.md");

/// The title menu's screen name, for returning to it.
pub const TITLE_NAME: &str = "winter_wonderland";

/// How the launcher lists the game.
pub const INFO: GameInfo = GameInfo {
    id: ID,
    name: "Winter Wonderland",
    category: Category::Seasonal,
    blurb: "Winter holidays: sort shaken gifts, then answer children's letters at the Wish Desk.",
    open: |ctx| ctx.push(title()),
    challenge: Some(open_challenge),
    event: Some(events::event),
    event_done: None,
    tutorial: crate::tutorial::lesson,
    settings: <title::Winter as parlor_core::menus::TitleGame>::settings,
    story: parlor_core::story::StoryTags {
        times: &[
            parlor_core::story::PartOfDay::Morning,
            parlor_core::story::PartOfDay::Evening,
        ],
        seasons: &[parlor_core::Season::Winter],
    },
};

/// Opens the current `period` challenge, from the launcher's list of
/// today's challenges: its briefing, then the run. Leaving it, or
/// finishing it, returns to the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}

thread_local! {
    static SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Makes every run started on this thread use `seed`, so tests replay a
/// run exactly. `None` goes back to the clock.
pub fn set_seed(seed: Option<u64>) {
    SEED.with(|s| s.set(seed));
}

/// A random source for a new run: the test seed when one is set.
pub fn fresh_rng() -> Rng {
    match SEED.with(|s| s.get()) {
        Some(seed) => Rng::seeded(seed),
        None => Rng::from_time(),
    }
}

/// Opens the pause screen over a run: its leave row goes back to the
/// challenges for a challenge run, or to the title.
pub fn pause(ctx: &mut Ctx<Parlor>, mode: &Mode) {
    let leave = if mode.plan().is_some() {
        parlor_core::menus::Leave::Challenge(CHALLENGES_NAME)
    } else {
        parlor_core::menus::Leave::Title(TITLE_NAME)
    };
    parlor_core::menus::pause(ctx, HELP, leave);
}
