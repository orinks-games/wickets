//! Daily, weekly and monthly challenges: the same stream for every player
//! in the period.
//!
//! - Daily, "Today's stream": a stream with every twist, on the paper boat.
//! - Weekly, "Boat of the week": a longer stream of mill wheels and ducks,
//!   on the week's own boat.
//! - Monthly, "The long run": the longest stream, every twist, getting
//!   busier all the way down.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pages, Parlor, Period};

use crate::progress::Progress;
use crate::rules::{Course, Mode};
use crate::run::{boat_for, Launch, RunScreen};
use crate::sound;
use crate::ID;

pub const CHALLENGES_NAME: &str = "little_rapids_challenges";

pub fn challenge_name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's stream",
        Period::Weekly => "Boat of the week",
        Period::Monthly => "The long run",
    }
}

/// A few words for the challenge's row.
pub fn challenge_tag(ch: &Challenge) -> String {
    let course = Course::challenge(Mode::for_period(ch.period));
    let bars = course.bars.unwrap_or(0);
    match ch.period {
        Period::Daily => format!("{bars} bars, every twist, on the paper boat"),
        Period::Weekly => {
            let boat = boat_for(Mode::Weekly, &Progress::default(), ch.seed);
            format!(
                "{bars} bars of mill wheels and ducks, on the {}",
                boat.name().to_lowercase()
            )
        }
        Period::Monthly => format!("{bars} bars, every twist, busier all the way down"),
    }
}

/// What the challenge asks.
pub fn challenge_rule(ch: &Challenge) -> String {
    match ch.period {
        Period::Daily => "One stream with waterfalls, mill wheels and ducks, the same for everyone today. Score as much as you can on the paper boat.".to_string(),
        Period::Weekly => {
            let boat = boat_for(Mode::Weekly, &Progress::default(), ch.seed);
            format!(
                "A longer stream of mill wheels and ducks, on the {}: {}.",
                boat.name().to_lowercase(),
                boat.handling()
            )
        }
        Period::Monthly => "The longest stream, every twist, and it gets busier all the way to the mouth.".to_string(),
    }
}

/// The pages read before a challenge run, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!(
        "{}: {}",
        challenge_name(period),
        challenge_rule(&ch)
    )];
    let launch = Launch {
        mode: Mode::for_period(period),
        seed: Some(ch.seed),
    };
    Pages::briefing(pages).then(move |ctx| ctx.replace(RunScreen::new(launch, Some(ch.clone()))))
}

pub struct Streams;

pub type ChallengesMenu = menus::ChallengesMenu<Streams>;

impl ChallengeGame for Streams {
    const ID: &'static str = ID;
    const NAME: &'static str = "Little Rapids";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_tag(&Challenge::current(ctx, ID, period))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_rule(&Challenge::current(ctx, ID, period))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn leave(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }
}

/// Holds a challenge opened from the launcher, and goes back there after.
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
        "Opening the Little Rapids challenge.".to_string()
    }
}
