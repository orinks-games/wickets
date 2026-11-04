//! The daily, weekly and monthly ranks: their rules, the screens that open
//! them, and what a finished one records.
//!
//! - Daily, "Tonight's rank": 6 pipes.
//! - Weekly, "The swell organ": 8 pipes, starting farther out.
//! - Monthly, "The full Echo": all 10 pipes, from very near to very far.
//!
//! A challenge is the same rank for every player that period, notes and
//! offsets and all. It ends when every pipe is in tune.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::play::Tuning;
use crate::rules::{pipes_words, points_words, steps_words, Layout, Rank, STEP_CENTS};
use crate::{sound, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "the_echo_challenges";

/// Everything a challenge rank will be, decided by its seed alone.
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
                pipes: 6,
                near: 3,
                far: 10,
                free_hints: false,
            },
            Period::Weekly => Layout {
                pipes: 8,
                near: 4,
                far: 14,
                free_hints: false,
            },
            Period::Monthly => Layout {
                pipes: 10,
                near: 1,
                far: 18,
                free_hints: false,
            },
        };
        Plan {
            period,
            layout,
            seed,
        }
    }

    /// The rank this plan describes.
    pub fn rank(&self) -> Rank {
        Rank::new(self.layout, self.seed)
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's rank",
        Period::Weekly => "The swell organ",
        Period::Monthly => "The full Echo",
    }
}

/// A few words on what the challenge is, for its row.
pub fn tag(plan: &Plan) -> String {
    pipes_words(plan.layout.pipes)
}

/// What the challenge asks, in a sentence or two.
pub fn rule(plan: &Plan) -> String {
    let l = plan.layout;
    format!(
        "A rank of {}. Each starts between {} and {} out, at {STEP_CENTS} cents a step.",
        pipes_words(l.pipes),
        steps_words(l.near),
        steps_words(l.far)
    )
}

/// The game's side of the shared challenges menu.
pub struct Ranks;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Ranks>;

impl ChallengeGame for Ranks {
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
        "Up and Down choose a rank, Enter reads its briefing, Escape goes back. Every player gets the same rank: a new one each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge rank, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = Plan::for_challenge(&ch);
    let pages = vec![format!("{}: {}", name(period), rule(&plan))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Tuning::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when a challenge is opened from
/// outside the game: it opens the briefing, and once the rank is tuned, or
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
    /// Named as the challenges menu is, so leaving a rank lands here.
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
        "Opening the challenge of The Echo.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// The headline of a finished challenge rank, the results menu's title.
pub fn summary(ch: &Challenge, rank: &Rank) -> String {
    format!("{} result: {}", ch.title(), points_words(rank.score))
}

/// Records a finished challenge rank and returns the pages that read it
/// back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, rank: &Rank) -> Vec<String> {
    let result = Challenges::submit(ctx, ch, rank.score as u64);
    // A replay of the same challenge adds no pipes to the totals.
    if result == ScoreResult::First {
        crate::progress::save_other(ctx, rank, false);
    }
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![
        crate::play::tuned_line(rank),
        format!(
            "{} result: {}. {} A {streak} {} streak. {}",
            ch.title(),
            points_words(rank.score),
            score_line(result),
            ch.period.unit(),
            ch.resets()
        ),
    ]
}
