//! The daily, weekly and monthly halls: their rules, the screens that open
//! them, and what a finished one records.
//!
//! - Daily, "Tonight's hall": 16 steps, 8 lamps out, a 100 second taper.
//! - Weekly, "The east wing": 24 steps, 12 lamps out, 2 draughts.
//! - Monthly, "The grand gallery": 32 steps, 16 lamps out, 3 draughts.
//!
//! A challenge is the same hall for every player that period, lamps,
//! draughts and all. It ends when every lamp is burning.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::play::Round;
use crate::rules::{lamps_words, points_words, time_words, Hall, Layout};
use crate::{sound, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "lamplighter_challenges";

/// Everything a challenge hall will be, decided by its seed alone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub period: Period,
    pub layout: Layout,
    pub seed: u64,
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        Plan::new(ch.period, ch.rng().next_u64())
    }

    pub fn new(period: Period, seed: u64) -> Plan {
        let layout = match period {
            Period::Daily => Layout {
                length: 16,
                out: 8,
                taper: Some(100.0),
                draughts: 1,
            },
            Period::Weekly => Layout {
                length: 24,
                out: 12,
                taper: Some(150.0),
                draughts: 2,
            },
            Period::Monthly => Layout {
                length: 32,
                out: 16,
                taper: Some(210.0),
                draughts: 3,
            },
        };
        Plan {
            period,
            layout,
            seed,
        }
    }

    /// The hall this plan describes, taper not yet lit.
    pub fn hall(&self) -> Hall {
        Hall::new(self.layout, self.seed)
    }

    fn taper_secs(&self) -> u32 {
        self.layout.taper.unwrap_or(0.0) as u32
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's hall",
        Period::Weekly => "The east wing",
        Period::Monthly => "The grand gallery",
    }
}

/// A few words on what the challenge is, for its row.
pub fn tag(plan: &Plan) -> String {
    format!(
        "{} out along {} steps",
        lamps_words(plan.layout.out),
        plan.layout.length
    )
}

/// What the challenge asks, in a sentence or two.
pub fn rule(plan: &Plan) -> String {
    let l = plan.layout;
    let draughts = match l.draughts {
        0 => String::new(),
        1 => " One draught blows a lamp out on the way.".to_string(),
        n => format!(" {n} draughts blow lamps out on the way."),
    };
    format!(
        "A hall of {} steps with {} out. The taper burns for {}.{draughts}",
        l.length,
        lamps_words(l.out),
        time_words(plan.taper_secs())
    )
}

/// The game's side of the shared challenges menu.
pub struct Halls;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Halls>;

impl ChallengeGame for Halls {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(&Plan::for_challenge(&Challenge::current(ctx, ID, period)))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(&Plan::for_challenge(&Challenge::current(ctx, ID, period)))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a hall, Enter reads its briefing, Escape goes back. Every player gets the same hall: a new one each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge hall, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = Plan::for_challenge(&ch);
    let pages = vec![format!(
        "{}: {} Light them all on the first taper for a bonus.",
        name(period),
        rule(&plan)
    )];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Round::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when a challenge is opened from
/// outside the game: it opens the briefing, and once the hall is done, or
/// left from the pause menu, it steps aside.
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
    /// Named as the challenges menu is, so leaving a hall lands here.
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
        "Opening the Lamplighter challenge.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// The headline of a finished challenge hall, the results menu's title.
pub fn summary(ch: &Challenge, hall: &Hall) -> String {
    format!("{} result: {}", ch.title(), points_words(hall.score))
}

/// Records a finished challenge hall and returns the pages that read it
/// back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, hall: &Hall) -> Vec<String> {
    crate::progress::save_lamps(ctx, hall.lit, false);
    let result = Challenges::submit(ctx, ch, hall.score as u64);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![
        crate::play::lit_line(hall),
        format!(
            "{} result: {}. {} A {streak} {} streak. {}",
            ch.title(),
            points_words(hall.score),
            score_line(result),
            ch.period.unit(),
            ch.resets()
        ),
    ]
}
