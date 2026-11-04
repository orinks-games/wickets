//! The challenge screens: the list of the three challenges, the briefing
//! read before a job, the stand-in title the launcher opens a job from,
//! and the result read after it.

use hearken::text::sentence_start;
use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::challenge::{self, Job, Plan};
use crate::rules::{time_words, Outcome, Run};
use crate::{sound, Crack, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "safecracker_challenges";

/// The game's side of the shared challenges menu.
pub struct Jobs;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Jobs>;

impl ChallengeGame for Jobs {
    const ID: &'static str = ID;
    const NAME: &'static str = "Safecracker";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge::name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let ch = Challenge::current(ctx, ID, period);
        challenge::tag(&Plan::for_challenge(&ch))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let ch = Challenge::current(ctx, ID, period);
        challenge::rule(&Plan::for_challenge(&ch))
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

    fn intro_help() -> String {
        "Up and Down choose a job, Enter reads its briefing, Escape goes back. Every player gets the same safes: a new daily job each day, a new vault each Monday and a heist each month.".to_string()
    }
}

/// A challenge's row and its help, as the challenges menu reads them.
pub fn row(ctx: &mut Ctx<Parlor>, period: Period) -> (String, String) {
    let name = Jobs::name(ctx, period);
    let tag = Jobs::tag(ctx, period);
    let label = menus::row_label(ctx, ID, period, &name, &tag);
    let ch = Challenge::current(ctx, ID, period);
    (
        label,
        format!("{} {}", Jobs::help(ctx, period), ch.resets()),
    )
}

/// The pages read before a challenge job, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = Plan::for_challenge(&ch);
    let mut pages = vec![format!(
        "{}: {}",
        challenge::name(period),
        challenge::rule(&plan)
    )];
    if period == Period::Monthly {
        pages.extend(challenge::story());
    }
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Crack::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the job is over (or left from the pause
/// menu) it steps aside, back to the launcher.
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
    /// Named as the challenges menu is, so leaving a job, by its results or
    /// the pause menu, lands here and steps aside to the launcher.
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
        "Opening the Safecracker challenge.".to_string()
    }
}

/// How a safe was lost, in a sentence.
pub fn loss_words(outcome: Outcome, name: &str) -> String {
    match outcome {
        Outcome::Opened => String::new(),
        Outcome::Jammed => {
            format!("Three strikes: the relocker fires and {name} jams shut.")
        }
        Outcome::Caught => format!("The guard caught you at {name}."),
        Outcome::TimeUp => format!("The clock ran out at {name}."),
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => {
            format!("A new best, beating {previous}.")
        }
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// Records a finished job and returns the pages that read it back.
pub fn finish_job(ctx: &mut Ctx<Parlor>, ch: &Challenge, job: &Job, last: &Run) -> Vec<String> {
    let score = job.score();
    let n = job.safes();
    let mut first = match ch.period {
        Period::Daily if job.won() => format!(
            "Cracked. {} swings open in {}.",
            sentence_start(last.spec.name),
            time_words(last.time)
        ),
        Period::Daily => loss_words(job.outcome.unwrap_or(Outcome::Jammed), last.spec.name),
        Period::Weekly if job.won() => {
            "The vault of three is yours: all three safes open.".to_string()
        }
        Period::Weekly => format!(
            "{} {} of {n} safes open.",
            loss_words(job.outcome.unwrap_or(Outcome::Jammed), last.spec.name),
            job.opened
        ),
        Period::Monthly if job.won() => challenge::ending(job).to_string(),
        Period::Monthly => format!(
            "{} {}",
            loss_words(job.outcome.unwrap_or(Outcome::Jammed), last.spec.name),
            challenge::ending(job)
        ),
    };
    if job.clock_bonus > 0 {
        first.push_str(&format!(
            " {} left on the clock: {} bonus points.",
            sentence_start(&time_words(last.time_left)),
            job.clock_bonus
        ));
    }
    let result = Challenges::submit(ctx, ch, score);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![
        first,
        format!(
            "{} result: {score} points. {} A {streak} {} streak. {}",
            ch.title(),
            score_line(result),
            ch.period.unit(),
            ch.resets()
        ),
    ]
}
