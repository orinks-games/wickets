//! Runs for special nights.
//!
//! From 17 October to the day after Halloween, the ghost cars come out:
//! in the guided run, a Black Sky Run, the endless rail and the challenge corridor, flow
//! vehicles wear a hollow whistle over their usual pulse.
//! They play exactly as flow vehicles, so no board changes. The first one
//! in a run is named in words, and the status key names the night.
//!
//! On the last Friday of every month the title has a Friday night run
//! row: three sectors made from that Friday's date, the same for every
//! pilot, on a standard skimmer, where salvage comes as cargo crates worth
//! double. It keeps its own
//! board. On other days the row stays in its place, locked, and says when
//! it opens.
//!
//! Everything here is a plain function of the date, so tests pin it.

use parlor_core::almanac::{Week, Window};
use parlor_core::calendar::Holiday;
use parlor_core::{Date, Happening, Rng};

use crate::rules::{Loadout, Mode, Route, Run};

/// The Halloween touch's name.
pub const GHOST_CARS: &str = "Ghost cars";
/// When the ghost cars run: Halloween's run-up, 17 October to 1 November.
pub const GHOST_WINDOW: Window = Window::RunUp(Holiday::Halloween);
/// The Friday night row's name.
pub const FRIDAY: &str = "Friday night run";
/// When it is open: the last Friday of every month.
pub const FRIDAY_WINDOW: Window = Window::Monthly(Week::Last, 4);
/// The Friday night run's rule, for the status key.
pub const FRIDAY_RULE: &str = "Friday night: salvage comes as cargo crates, worth double.";
/// What Play again says once the Friday night run has closed.
pub const FRIDAY_OVER: &str =
    "Friday night is over. The Friday night run opens again on the last Friday of next month.";
/// The middle routes a Friday night run can draw.
const FRIDAY_ROUTES: [Route; 4] = [Route::Standard, Route::Ambush, Route::Storm, Route::Relay];

/// Whether the ghost cars are out.
pub fn ghost_cars(date: Date) -> bool {
    GHOST_WINDOW.is_open(date)
}

/// Whether a run of `mode` wears the ghost cars on `date`: the ordinary
/// runs and the guided run (a new save's Play), never a timed challenge or
/// another event.
pub fn ghost_cars_for(mode: Mode, date: Date) -> bool {
    matches!(
        mode,
        Mode::Tutorial | Mode::BlackSky | Mode::Endless | Mode::Challenge | Mode::ColdStart
    ) && ghost_cars(date)
}

/// Whether the Friday night run is open on `date`.
pub fn friday_open(date: Date) -> bool {
    FRIDAY_WINDOW.is_open(date)
}

/// What Enter on the locked Friday night row says.
pub fn friday_locked_line(date: Date) -> String {
    format!(
        "The Friday night run opens {}.",
        FRIDAY_WINDOW.next_words(date)
    )
}

/// The locked Friday night row's help.
pub fn friday_locked_help(date: Date) -> String {
    format!(
        "Opens {}, {}: three sectors where salvage comes as cargo crates, worth double.",
        FRIDAY_WINDOW.every(),
        FRIDAY_WINDOW.next_words(date)
    )
}

/// The routes of the Friday night run on `date`: the salvage stretch,
/// one more drawn from the date, then extraction.
pub fn friday_routes(date: Date) -> Vec<Route> {
    let day = date.days() as u64;
    let middle = FRIDAY_ROUTES[(day % FRIDAY_ROUTES.len() as u64) as usize];
    vec![Route::Salvage, middle, Route::Extraction]
}

/// The Friday night run on `date`, seeded by the date, so every pilot
/// flies one rail.
pub fn friday_run(date: Date, story_stage: usize) -> Run {
    let day = date;
    let rng = Rng::seeded(0x4d4f_4f4e_0000_0000 ^ ((day.days() as u64) << 1));
    let mut run = Run::challenge_event(
        Mode::FridayNight,
        Loadout::default(),
        story_stage,
        rng,
        &[],
        2,
        None,
        false,
    );
    run.cargo = true;
    run.build_route(&friday_routes(date));
    run.event_rule = FRIDAY_RULE.to_string();
    run
}

/// What the launcher names while something is on.
pub fn event(date: Date) -> Option<Happening> {
    if friday_open(date) {
        return Some(Happening::new(
            FRIDAY,
            "Fly three sectors on the busiest night of the month, where salvage comes as cargo crates worth double.",
        ));
    }
    ghost_cars(date).then(|| {
        Happening::new(
            GHOST_CARS,
            "Until 1 November, flow vehicles come in costume. Meet them as usual.",
        )
    })
}

/// The Friday night row's help, with the best and the relaxed best.
pub fn friday_help(best: Option<u64>, relaxed: Option<u64>) -> String {
    let best = match (best, relaxed) {
        (Some(b), Some(r)) => format!(" Your best is {b}, relaxed best {r}."),
        (Some(b), None) => format!(" Your best is {b}."),
        (None, Some(r)) => format!(" Your relaxed best is {r}."),
        (None, None) => String::new(),
    };
    format!(
        "Friday night, the last of the month: three sectors made from tonight's date, the same for every pilot, on a standard skimmer. Salvage comes as cargo crates, worth double. It keeps its own board and pays no credits, rank or story. Open all day.{best}"
    )
}
