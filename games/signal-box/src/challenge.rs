//! The daily, weekly and monthly timetables: their rules, the screens that
//! open them, and what a finished one records.
//!
//! - Daily, "Today's timetable": 40 trains from the day's seed.
//! - Weekly, "The express week": 60 trains, a faster start and a lower floor.
//! - Monthly, "The night mail": 100 trains, down to the fastest window.
//!
//! A timetable is the same list of calls for every player that period. A
//! derailment ends it, and the trains passed so far still count.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::rules::{
    points_words, timetable, trains_words, window_words, Call, Line, Outcome, Pace, Shift,
};
use crate::{sound, Signal, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "signal_box_challenges";

/// Everything a challenge run will be, decided by its seed alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub period: Period,
    pub pace: Pace,
    pub calls: Vec<Call>,
    /// The seed for anything else the run draws.
    pub seed: u64,
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        Plan::new(ch.period, ch.rng())
    }

    pub fn new(period: Period, mut rng: parlor_core::Rng) -> Plan {
        let (trains, pace) = match period {
            Period::Daily => (
                40,
                Pace {
                    start: 2500,
                    step: 150,
                    every: 4,
                    floor: 1000,
                },
            ),
            Period::Weekly => (
                60,
                Pace {
                    start: 2200,
                    step: 150,
                    every: 5,
                    floor: 850,
                },
            ),
            Period::Monthly => (
                100,
                Pace {
                    start: 2500,
                    step: 100,
                    every: 5,
                    floor: 700,
                },
            ),
        };
        let calls = timetable(&mut rng, trains);
        Plan {
            period,
            pace,
            calls,
            seed: rng.next_u64(),
        }
    }

    pub fn trains(&self) -> u32 {
        self.calls.len() as u32
    }

    /// The run this plan describes, not yet started.
    pub fn shift(&self) -> Shift {
        Shift::new(Line::Timetable(self.calls.clone()), self.pace, self.seed)
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's timetable",
        Period::Weekly => "The express week",
        Period::Monthly => "The night mail",
    }
}

/// A few words on what the challenge is, for its row.
pub fn tag(plan: &Plan) -> String {
    format!(
        "{}, starting at {}",
        trains_words(plan.trains()),
        window_words(plan.pace.start)
    )
}

/// What the challenge asks, in a sentence or two.
pub fn rule(plan: &Plan) -> String {
    let p = plan.pace;
    let step = match p.every {
        1 => "every train".to_string(),
        n => format!("every {n} trains"),
    };
    // The run can end before the window reaches its floor: name the
    // lowest window the timetable really gets to.
    let shrinks = plan.trains().saturating_sub(1) / p.every.max(1);
    let lowest = p.start.saturating_sub(p.step * shrinks).max(p.floor);
    format!(
        "A timetable of {}. The window starts at {}, shrinks {step}, and never goes below {}. One wrong or late answer ends the run.",
        trains_words(plan.trains()),
        window_words(p.start),
        window_words(lowest)
    )
}

/// The game's side of the shared challenges menu.
pub struct Timetables;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Timetables>;

impl ChallengeGame for Timetables {
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
        "Up and Down choose a timetable, Enter reads its briefing, Escape goes back. Every player gets the same calls: a new timetable each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a timetable, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = Plan::for_challenge(&ch);
    let pages = vec![format!("{}: {}", name(period), rule(&plan))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Signal::challenge(ch));
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
    /// Named as the challenges menu is, so leaving a run lands here.
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
        "Opening the Signal Box challenge.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// The headline of a finished timetable, the results menu's title.
pub fn summary(ch: &Challenge, shift: &Shift) -> String {
    format!("{} result: {}", ch.title(), points_words(shift.score))
}

/// Records a finished timetable and returns the pages that read it back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, shift: &Shift) -> Vec<String> {
    let total = shift.trains().unwrap_or(0);
    // How the shift ended was said as it happened: the pages lead with
    // the trains through, then the result.
    let first = match shift.outcome {
        Some(Outcome::Derailed(_)) => Some(format!("{} of {total} trains through.", shift.passed)),
        _ => None,
    };
    crate::progress::save_trains(ctx, shift.passed);
    let result = Challenges::submit(ctx, ch, shift.score as u64);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    first
        .into_iter()
        .chain([format!(
            "{} result: {}. {} A {streak} {} streak. {}",
            ch.title(),
            points_words(shift.score),
            score_line(result),
            ch.period.unit(),
            ch.resets()
        )])
        .collect()
}
