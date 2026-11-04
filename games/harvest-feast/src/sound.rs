//! Sound keys, and where each station sits around the listener.

use hearken::{Bus, Ctx, Vec3};
use parlor_core::Parlor;

use crate::rules::{Action, Grade, Station};

pub const WAITING: &str = "harvest-feast/cue/waiting";
pub const WAITING_LATE: &str = "harvest-feast/cue/waiting_late";
pub const FOCUS: &str = "harvest-feast/cue/focus";
pub const EDGE: &str = "harvest-feast/cue/edge";
pub const EMPTY: &str = "harvest-feast/cue/empty";
pub const RECIPE: &str = "harvest-feast/cue/recipe";
pub const HINT: &str = "harvest-feast/cue/hint";
pub const BELL: &str = "harvest-feast/cue/bell";
pub const PLATE: &str = "harvest-feast/act/plate";
pub const EARLY: &str = "harvest-feast/judge/early";
pub const RUINED: &str = "harvest-feast/judge/ruined";
pub const KITCHEN: &str = "harvest-feast/ambience/kitchen";
/// The kitchen's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("harvest-feast");
pub const TITLE_MUSIC: &str = "harvest-feast/music/title";
pub const WIN: &str = "harvest-feast/music/win";
pub const LOSE: &str = "harvest-feast/music/lose";
pub const FEAST: &str = "harvest-feast/music/feast";
/// The banquet: a dish served at 90 or better.
pub const GOLDEN: &str = "harvest-feast/cue/golden_plate";
/// A challenge meal opens: the dinner gong.
pub const TWIST: &str = "harvest-feast/cue/gong";
/// The family arrives for Thanksgiving dinner: knocks at the door, then a chime.
pub const FAMILY: &str = "harvest-feast/cue/family";
/// A weekly course served, the next one coming.
pub const COURSE_UP: &str = "harvest-feast/music/course_up";

/// Where a station is heard: an arc in front of you, far left to far right.
pub fn place(station: Station) -> Vec3 {
    match station {
        Station::LeftBurner => Vec3::new(-1.6, 0.5, 0.0),
        Station::Board => Vec3::new(-0.9, 1.4, 0.0),
        Station::Oven => Vec3::new(0.0, 1.7, 0.0),
        Station::Griddle => Vec3::new(0.9, 1.4, 0.0),
        Station::RightBurner => Vec3::new(1.6, 0.5, 0.0),
    }
}

/// A station's cooking loop while all is well (the board has none).
pub fn cooking(station: Station) -> Option<&'static str> {
    match station {
        Station::LeftBurner => Some("harvest-feast/saucepan/simmer"),
        Station::Board => None,
        Station::Oven => Some("harvest-feast/oven/hum"),
        Station::Griddle => Some("harvest-feast/griddle/sizzle"),
        Station::RightBurner => Some("harvest-feast/stockpot/simmer"),
    }
}

/// A station's loop once its dish needs you.
pub fn due(station: Station) -> &'static str {
    match station {
        Station::LeftBurner => "harvest-feast/saucepan/bubbling",
        Station::Board => "harvest-feast/board/ready",
        Station::Oven => "harvest-feast/oven/timer",
        Station::Griddle => "harvest-feast/griddle/sharp",
        Station::RightBurner => "harvest-feast/stockpot/boiling",
    }
}

/// A station's loop while its dish burns.
pub fn burning(station: Station) -> &'static str {
    match station {
        Station::LeftBurner => "harvest-feast/saucepan/scorching",
        Station::Board => "harvest-feast/board/hurry",
        Station::Oven => "harvest-feast/oven/smoke",
        Station::Griddle => "harvest-feast/griddle/smoke",
        Station::RightBurner => "harvest-feast/stockpot/boilover",
    }
}

/// What starting a dish at a station sounds like.
pub fn start(station: Station) -> &'static str {
    match station {
        Station::LeftBurner | Station::RightBurner => "harvest-feast/start/pot",
        Station::Board => "harvest-feast/start/board",
        Station::Oven => "harvest-feast/start/oven",
        Station::Griddle => "harvest-feast/start/griddle",
    }
}

/// The foley of an action.
pub fn action(station: Station, action: Action) -> &'static str {
    match action {
        Action::Start => start(station),
        Action::Stir => "harvest-feast/act/stir",
        Action::Whisk => "harvest-feast/act/whisk",
        Action::Flip => "harvest-feast/act/flip",
        Action::Baste => "harvest-feast/act/baste",
        Action::Rotate => "harvest-feast/act/rotate",
        Action::Pull => "harvest-feast/act/pull",
        Action::TakeOff => "harvest-feast/act/takeoff",
        Action::Drain => "harvest-feast/act/drain",
        Action::Chop => "harvest-feast/act/chop",
        Action::Plate => "harvest-feast/act/plate",
    }
}

/// The chime that judges a step.
pub fn grade(grade: Grade) -> &'static str {
    match grade {
        Grade::Perfect => "harvest-feast/judge/perfect",
        Grade::Good => "harvest-feast/judge/good",
        Grade::Late => "harvest-feast/judge/late",
        Grade::Ruined => RUINED,
    }
}

/// One, two or three rising chimes for the stars a meal earned.
pub fn stars(n: u32) -> Option<&'static str> {
    match n {
        0 => None,
        1 => Some("harvest-feast/cue/stars_1"),
        2 => Some("harvest-feast/cue/stars_2"),
        _ => Some("harvest-feast/cue/stars_3"),
    }
}

/// Plays a cue at a station.
pub fn at(ctx: &mut Ctx<Parlor>, key: &str, station: Station) {
    ctx.audio.play_at(key, place(station), Bus::EFFECTS);
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The kitchen: music off, the warm room on: the range fire, the clock.
pub fn kitchen(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(KITCHEN));
}
