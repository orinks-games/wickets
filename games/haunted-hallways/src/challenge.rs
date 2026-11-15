//! Daily, weekly and monthly challenges: the same house for every player.
//!
//! The daily is one floor with a twist; the weekly three floors in a row
//! with a twist and courage that does not refill; the monthly a hidden
//! ballroom wing with its own séance puzzles.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pace, Pages, Parlor, Period, ScoreResult};

use crate::rules::{floor_spec, Event};
use crate::{sound, Haunt, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "haunted_hallways_challenges";

/// The challenge's own name.
pub fn challenge_name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's haunting",
        Period::Weekly => "The long night",
        Period::Monthly => "The midnight masquerade",
    }
}

/// The event a challenge runs.
pub fn event(ch: &Challenge) -> Event {
    Event::from_challenge(ch).0
}

/// A few words on what the challenge is, for its row in the list.
pub fn challenge_tag(ch: &Challenge) -> String {
    let e = event(ch);
    let twist = e.twist.map(|t| t.name()).unwrap_or("no twist");
    match ch.period {
        Period::Daily => format!("floor {}, twist {twist}", e.first),
        Period::Weekly => format!(
            "floors {} to {}, courage carried, twist {twist}",
            e.first, e.last
        ),
        Period::Monthly => "the hidden ballroom wing, séance rooms".to_string(),
    }
}

/// What the challenge asks, in a few sentences.
pub fn challenge_rule(ch: &Challenge) -> String {
    let e = event(ch);
    let twist = match e.twist {
        Some(t) => format!(" Twist, {}: {}.", t.name(), t.describe()),
        None => String::new(),
    };
    match ch.period {
        Period::Daily => format!(
            "Floor {}, in {}.{twist}",
            e.first,
            floor_spec(e.first, false).name
        ),
        Period::Weekly => format!(
            "Floors {} to {}, in {}. Courage does not refill between floors.{twist}",
            e.first,
            e.last,
            floor_spec(e.first, false).name,
        ),
        Period::Monthly => format!(
            "A hidden ballroom wing: {} rooms, every ghost abroad. In a séance room the medium calls up two sounds. Pick up what made each, in order. Out of order costs ten seconds.",
            crate::rules::ball_spec().rooms
        ),
    }
}

/// The result spoken when a challenge run at `pace` ends: score, best on
/// the pace's board, and streak.
pub fn result_line(
    ch: &Challenge,
    score: u64,
    result: ScoreResult,
    streak: u32,
    pace: Pace,
) -> String {
    let best = pace.best_word();
    let record = match result {
        ScoreResult::First if pace.separate() => {
            "Your first relaxed result on this challenge.".to_string()
        }
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new {best}, beating {previous}."),
        ScoreResult::Short { best: b } => format!("Your {best} is {b}."),
    };
    format!(
        "{} result: score {score}. {record} A {} {} streak. {}",
        ch.title(),
        streak.max(1),
        ch.period.unit(),
        ch.resets()
    )
}

/// The pages read before a challenge run, which then start it. Escape on
/// them goes back without starting.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!(
        "{}: {}",
        challenge_name(period),
        challenge_rule(&ch)
    )];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Haunt::challenge(ch));
        }
    })
}

/// The game's side of the shared challenges menu.
pub struct Hauntings;

/// The three challenges, each with its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Hauntings>;

impl ChallengeGame for Hauntings {
    const ID: &'static str = ID;
    const NAME: &'static str = crate::NAME;
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

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same haunted house: a new daily each day, a weekly each Monday, and a monthly event each month.".to_string()
    }
}

/// Stands in for the challenges menu when the launcher opens a challenge
/// directly: it opens the briefing, and once the run is over, or left from
/// the pause menu, it steps aside, back to the launcher.
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
        format!("Opening the {} challenge.", crate::NAME)
    }
}
