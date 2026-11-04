//! Daily, weekly and monthly challenges: the same drops for every player
//! in the period, each with its own rule.
//!
//! - Daily, "Twist of the day": one 60-second round across five lanes with
//!   a twist drawn from the day's seed (golden rush, bee swarm, fast fall,
//!   mud slide or small basket).
//! - Weekly, "The long meadow": a marathon of five rounds, the meadow's
//!   rounds 4 to 8 back to back, each with its egg target.
//! - Monthly, "The Easter festival": three rounds of festival patterns
//!   (egg sweeps, bee fences, nests), each clean pattern worth a bonus.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pages, Parlor, Period};

use crate::rules::{
    Mode, Twist, DAILY_SECS, FESTIVAL_ROUNDS, PATTERN_BONUS, WEEKLY_FROM, WEEKLY_ROUNDS,
};
use crate::run::{Launch, RunScreen};
use crate::sound;
use crate::ID;

/// The challenges menu's screen name.
pub const CHALLENGES_NAME: &str = "basket_run_challenges";

/// The challenge's own name.
pub fn challenge_name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Twist of the day",
        Period::Weekly => "The long meadow",
        Period::Monthly => "The Easter festival",
    }
}

/// A few words on what the challenge is, for its row in the list.
pub fn challenge_tag(ch: &Challenge) -> String {
    match ch.period {
        Period::Daily => format!(
            "one {}-second round, today's twist: {}",
            DAILY_SECS as u32,
            Twist::for_seed(ch.seed).name()
        ),
        Period::Weekly => format!(
            "{WEEKLY_ROUNDS} rounds back to back from round {WEEKLY_FROM}, each with an egg target"
        ),
        Period::Monthly => format!("{FESTIVAL_ROUNDS} rounds of egg sweeps, bee fences and nests"),
    }
}

/// What the challenge asks, in a sentence or two.
pub fn challenge_rule(ch: &Challenge) -> String {
    match ch.period {
        Period::Daily => {
            let twist = Twist::for_seed(ch.seed);
            format!(
                "One {}-second round, no egg target. Every fifth cracked egg breaks a basket. Twist, {}: {}",
                DAILY_SECS as u32,
                twist.name(),
                twist.describe()
            )
        }
        Period::Weekly => format!(
            "Rounds {WEEKLY_FROM} to {} back to back, each faster, each with an egg target. Your 3 baskets must last all {WEEKLY_ROUNDS} rounds.",
            WEEKLY_FROM + WEEKLY_ROUNDS - 1
        ),
        Period::Monthly => format!(
            "{FESTIVAL_ROUNDS} rounds of patterns, each called as it starts. Each one caught cleanly earns {PATTERN_BONUS} points."
        ),
    }
}

/// The pages read before a challenge run, which then start it. Escape
/// goes back without starting.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    if period == Period::Daily {
        ctx.audio.play(sound::TWIST, hearken::Bus::EFFECTS);
    }
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

/// The game's side of the shared challenges menu.
pub struct Festival;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Festival>;

impl ChallengeGame for Festival {
    const ID: &'static str = ID;
    const NAME: &'static str = "Basket Run";
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

/// Holds a challenge opened from the launcher: it opens the briefing, and
/// when the run is over (or left) it goes back to the launcher. It is named
/// like the challenges menu, so results and pause land here.
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
        "Opening the Basket Run challenge.".to_string()
    }
}
