//! The challenge screens: the list of the three challenge rides, the
//! briefing read before one, and the stand-in the launcher opens a
//! challenge from.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pages, Parlor, Period};

use crate::challenge::{self, Plan};
use crate::ride::Ride;
use crate::{sound, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "freewheel_challenges";

/// The game's side of the shared challenges menu.
pub struct Rides;

/// The three challenge rides, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Rides>;

impl ChallengeGame for Rides {
    const ID: &'static str = ID;
    const NAME: &'static str = "Freewheel";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge::name(period).to_string()
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

    fn leave(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a ride, Enter reads its briefing, Escape goes back. Every player gets the same course: a new morning run each day, a rush hour each Monday and a cross-town ride each month.".to_string()
    }
}

/// The pages read before a challenge ride, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = Plan::for_challenge(&ch);
    let pages = vec![format!("{}: {}", challenge::name(period), plan.rule())];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Ride::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the ride is over, or left from the
/// pause menu, it steps aside, back to the launcher.
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
        "Opening the Freewheel challenge.".to_string()
    }
}
