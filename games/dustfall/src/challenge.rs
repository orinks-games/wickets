//! The daily, weekly and monthly challenges, the same sky for every
//! player: Tonight's sky is one night on a colony set up for it, Siege
//! week is five nights from night 4 with the stores to meet them, and The
//! long watch is a whole colony to Relief. The menu of the three, the
//! briefing read before one, and the screen that stands in for the menu
//! when the launcher opens a challenge.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pages, Parlor, Period};

use crate::colony::{share_as, ColonyScreen, Mode};
use crate::night::NightScreen;
use crate::rules::grid::CENTRE;
use crate::rules::{Building, Input, Perks, Run, Sector, WorkKind, RELIEF_NIGHT};
use crate::{sound, ID};

/// The challenges menu's `State::name`.
pub const CHALLENGES_NAME: &str = "dustfall_challenges";

/// Tonight's sky draws its night from these.
pub const SKY_FIRST: u8 = 3;
pub const SKY_LAST: u8 = 9;
/// Siege week's first and last night.
pub const SIEGE_FIRST: u8 = 4;
pub const SIEGE_LAST: u8 = 8;

/// The nights a challenge runs, first and last.
pub fn nights(ch: &Challenge) -> (u8, u8) {
    match ch.period {
        Period::Daily => {
            let span = u64::from(SKY_LAST - SKY_FIRST + 1);
            let night = SKY_FIRST + (ch.rng().next_u64() % span) as u8;
            (night, night)
        }
        Period::Weekly => (SIEGE_FIRST, SIEGE_LAST),
        Period::Monthly => (1, RELIEF_NIGHT),
    }
}

/// The challenge's name.
pub fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
    match period {
        Period::Daily => "Tonight's sky",
        Period::Weekly => "Siege week",
        Period::Monthly => "The long watch",
    }
    .to_string()
}

/// A few words on what it is this time.
pub fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
    match period {
        Period::Daily => {
            let (night, _) = nights(&Challenge::current(ctx, ID, period));
            format!("one night, the raiders of night {night}, a colony set up for them")
        }
        Period::Weekly => {
            format!("five nights, {SIEGE_FIRST} to {SIEGE_LAST}, with stores to spend first")
        }
        Period::Monthly => "a whole colony, to Relief and on past it".to_string(),
    }
}

/// The rule, in a sentence or two.
pub fn rule(ctx: &mut Ctx<Parlor>, period: Period) -> String {
    match period {
        Period::Daily => {
            let (night, _) = nights(&Challenge::current(ctx, ID, period));
            format!(
                "One night against the raiders of night {night}. The colony is built and the gun is yours: hold the dome. The score is the night's points."
            )
        }
        Period::Weekly => format!(
            "Five nights, from night {SIEGE_FIRST} to night {SIEGE_LAST}. The colony holds the stores the first nights would have brought, so spend them before the raiders come. Points add up across the nights."
        ),
        Period::Monthly => format!(
            "A whole colony under the same sky as everyone else this month, from night 1 to Relief on night {RELIEF_NIGHT} and on past it, the nights harder each round, until the dome falls. The score is the colony's points."
        ),
    }
}

/// The game's side of the shared challenges menu.
pub struct Sky;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Sky>;

impl ChallengeGame for Sky {
    const ID: &'static str = ID;
    const NAME: &'static str = "Dustfall";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(ctx, period)
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(ctx, period)
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(ctx, period)
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same sky: a new night each day, a siege each week and a whole colony each month.".to_string()
    }
}

/// The page read before a challenge, which then starts it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let page = format!("{}: {}", name(ctx, period), rule(ctx, period));
    Pages::briefing([page]).then(move |ctx| start(ctx, period, true))
}

/// Starts the current `period` challenge in place of the screen on top.
/// Tonight's sky opens on the night itself; the others open on the colony
/// phase before their first night.
pub fn start(ctx: &mut Ctx<Parlor>, period: Period, replace: bool) {
    let ch = Challenge::current(ctx, ID, period);
    let (first, last) = nights(&ch);
    let mut run = if period == Period::Monthly {
        Run::long_watch(ch.seed, Perks::default())
    } else {
        Run::prepared(ch.seed, first, last)
    };
    let mode = Mode::Challenge(period);
    if period == Period::Daily {
        set_up(&mut run);
        run.take_events();
        let _ = run.apply(Input::Ready);
        let screen = NightScreen::new(share_as(run, mode));
        if replace {
            ctx.replace(screen);
        } else {
            ctx.push(screen);
        }
    } else {
        let screen = ColonyScreen::new(share_as(run, mode));
        if replace {
            ctx.replace(screen);
        } else {
            ctx.push(screen);
        }
    }
}

/// Tonight's sky's colony: power and a scanner under the grid, two
/// missiles, walls either side of the centre, a mine on the centre's path
/// and a flak post over the dome. What the stores cannot cover is left out.
fn set_up(run: &mut Run) {
    let builds = [
        Input::Buy {
            building: Building::Solar,
            col: CENTRE - 1,
        },
        Input::Buy {
            building: Building::Scanner,
            col: CENTRE + 1,
        },
        Input::MakeMissile,
        Input::MakeMissile,
        Input::Place {
            work: WorkKind::Wall,
            at: Sector::new(CENTRE - 1, 3),
        },
        Input::Place {
            work: WorkKind::Wall,
            at: Sector::new(CENTRE + 1, 3),
        },
        Input::Place {
            work: WorkKind::Mine,
            at: Sector::new(CENTRE, 4),
        },
        Input::Place {
            work: WorkKind::Flak,
            at: Sector::new(CENTRE, 2),
        },
    ];
    for input in builds {
        let _ = run.apply(input);
    }
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the challenge is over (or left from the
/// pause menu) it steps aside, back to the launcher.
pub struct ChallengeHost {
    period: Period,
    opened: bool,
}

impl ChallengeHost {
    pub fn new(period: Period) -> ChallengeHost {
        ChallengeHost {
            period,
            opened: false,
        }
    }
}

impl State<Parlor> for ChallengeHost {
    /// Named as the challenges menu is, so leaving a challenge lands here
    /// and steps aside to the launcher.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            sound::title_music(ctx);
            let pages = briefing(ctx, self.period);
            ctx.push(pages);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, _result: &str) {
        ctx.pop();
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        "Opening the Dustfall challenge.".to_string()
    }
}

/// Opens the current `period` challenge from the launcher's list of
/// today's challenges: its briefing, then the night. Leaving it returns to
/// the launcher.
pub fn open_challenge(ctx: &mut Ctx<Parlor>, period: Period) {
    ctx.push(ChallengeHost::new(period));
}
