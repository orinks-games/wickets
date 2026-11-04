//! The daily, weekly and monthly challenges: the same guests for every
//! player, the whole year-round shelf and every cup, scored by how well
//! each cup went.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::play::Counter;
use crate::rules::{Mode, Sitting};
use crate::talk::cups_words;
use crate::{sound, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "slow_kettle_challenges";

pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's tasting",
        Period::Weekly => "The busy week",
        Period::Monthly => "The tasting society",
    }
}

pub fn tag(period: Period) -> &'static str {
    match period {
        Period::Daily => "3 guests, the whole shelf",
        Period::Weekly => "6 guests, scores added",
        Period::Monthly => "5 tasters, each names a tea",
    }
}

/// The rule, in a sentence or two.
pub fn rule(period: Period) -> String {
    let n = Mode::Challenge(period).guests();
    match period {
        Period::Monthly => format!(
            "{n} members of the tasting society each ask for a tea by name. Each cup scores up to 100 for the right tea, water and steep."
        ),
        _ => format!(
            "{n} guests each ask for a taste. Every year-round tea and cup is yours for the day. Each cup scores up to 100."
        ),
    }
}

/// The game's side of the shared challenges menu.
pub struct Tastings;

pub type ChallengesMenu = menus::ChallengesMenu<Tastings>;

impl ChallengeGame for Tastings {
    const ID: &'static str = ID;
    const NAME: &'static str = "Slow Kettle";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(period).to_string()
    }

    fn help(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(period)
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same guests: new ones each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!(
        "{}: {} Challenge cups earn no coins, and your sand timer stays at home.",
        name(period),
        rule(period)
    )];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            let counter = Counter::challenge(ctx, ch);
            ctx.replace(counter);
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge.
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
        "Opening the Slow Kettle challenge.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// Records a finished challenge and returns the pages that read it back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, sitting: &Sitting) -> Vec<String> {
    let score = sitting.points();
    let result = Challenges::submit(ctx, ch, score);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    let n = sitting.verdicts.len();
    let perfect = sitting.perfect();
    vec![
        format!(
            "{} result: {score} points from {}, {perfect} perfect. {}",
            ch.title(),
            cups_words(n),
            score_line(result)
        ),
        format!("A {streak} {} streak. {}", ch.period.unit(), ch.resets()),
    ]
}
