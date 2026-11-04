//! The challenges' screens: the game's side of the shared challenges menu,
//! the briefing read before a run, and the stand-in the launcher opens a
//! challenge over.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pages, Parlor, Period};

use crate::festival::{Plan, Run};
use crate::show::{Mode, Show};
use crate::ID;

/// The challenges menu's `State::name`, which challenge results and the
/// pause screen return to.
pub const CHALLENGES_NAME: &str = "liberty_lights_challenges";

/// The game's side of the shared challenges menu.
pub struct Shows;

/// The daily, weekly and monthly challenges, each with its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Shows>;

impl ChallengeGame for Shows {
    const ID: &'static str = ID;
    const NAME: &'static str = "Liberty Lights";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let ch = Challenge::current(ctx, ID, period);
        Plan::for_challenge(&ch).name()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let ch = Challenge::current(ctx, ID, period);
        Plan::for_challenge(&ch).tag()
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let ch = Challenge::current(ctx, ID, period);
        Plan::for_challenge(&ch).rule()
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same shows: a new daily each day, a weekly gauntlet each Monday, a festival each month.".to_string()
    }
}

/// The pages read before a challenge: what it is, the twist or the
/// festival's rule, the record and when the next one comes, then the run.
/// Escape on them goes back without starting.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let run = Run::new(ch.clone());
    // The festival's rule opens with its own name.
    let (name, rule) = (run.plan.name(), run.plan.rule());
    let pages = if rule.starts_with(&name) {
        vec![rule]
    } else {
        vec![format!("{name}: {rule}")]
    };
    let mut run = Some(run);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(run) = run.take() {
            ctx.replace(Show::new(Mode::Challenge(Box::new(run))));
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the run is over, or left from the pause
/// menu, it steps aside, back to the launcher.
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
    /// Named as the challenges menu is, so leaving a run, by its results or
    /// the pause menu, lands here and steps aside to the launcher.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            let pages = briefing(ctx, self.period);
            ctx.push(pages);
        }
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
        "Opening the Liberty Lights challenge.".to_string()
    }
}
