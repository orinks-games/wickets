//! The daily, weekly and monthly challenges: their rules, the screens
//! that open them, and what a finished one records.
//!
//! - Daily, "Tonight's lane": one game, with the lane's pull drawn from
//!   the day's seed, so every player bowls the same lane.
//! - Weekly, "Spare week": 20 leaves, one ball each; the score is the
//!   leaves made.
//! - Monthly, "The late league": a three-game series; the score is the
//!   series total.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::lane::{seeded_leaves, Kind, Session};
use crate::rules::points_words;
use crate::{sound, Bowl, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "midnight_lanes_challenges";

/// Leaves in the weekly challenge.
pub const WEEKLY_LEAVES: usize = 20;
/// Games in the monthly series.
pub const MONTHLY_GAMES: u32 = 3;

/// Everything a challenge run will be, decided by its seed alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub period: Period,
    pub kind: Kind,
    /// The seed of the lane's pull.
    pub seed: u64,
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        Plan::new(ch.period, ch.rng())
    }

    pub fn new(period: Period, mut rng: parlor_core::Rng) -> Plan {
        let kind = match period {
            Period::Daily => Kind::Games(1),
            Period::Weekly => Kind::Leaves(seeded_leaves(&mut rng, WEEKLY_LEAVES)),
            Period::Monthly => Kind::Games(MONTHLY_GAMES),
        };
        Plan {
            period,
            kind,
            seed: rng.next_u64(),
        }
    }

    /// The run this plan describes, not yet started.
    pub fn session(&self) -> Session {
        Session::new(self.kind.clone(), self.seed)
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's lane",
        Period::Weekly => "Spare week",
        Period::Monthly => "The late league",
    }
}

/// A few words on what the challenge is, for its row.
pub fn tag(period: Period) -> String {
    match period {
        Period::Daily => "one game on a set lane".to_string(),
        Period::Weekly => format!("{WEEKLY_LEAVES} spares to shoot"),
        Period::Monthly => format!("a {MONTHLY_GAMES} game series"),
    }
}

/// What the challenge asks, in a sentence or two.
pub fn rule(period: Period) -> String {
    match period {
        Period::Daily => {
            "One full game. The lane pulls the same way for everyone today.".to_string()
        }
        Period::Weekly => {
            format!("{WEEKLY_LEAVES} leaves, one ball at each. Your score is the spares you make.")
        }
        Period::Monthly => format!(
            "{MONTHLY_GAMES} full games in a row on one lane. Your score is the series total."
        ),
    }
}

/// The game's side of the shared challenges menu.
pub struct Nights;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Nights>;

impl ChallengeGame for Nights {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(period)
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
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same lane: a new one each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(period))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Bowl::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the run is over, or left from the
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
        "Opening the Midnight Lanes challenge.".to_string()
    }
}

/// The challenge's score for a finished session.
pub fn score(session: &Session) -> u32 {
    match session.kind {
        Kind::Leaves(_) => session.made,
        Kind::Games(_) => session.series_total(),
    }
}

/// "12 spares", "1 spare", or points.
pub fn score_words(period: Period, score: u32) -> String {
    match (period, score) {
        (Period::Weekly, 1) => "1 spare made".to_string(),
        (Period::Weekly, n) => format!("{n} spares made"),
        (_, n) => points_words(n),
    }
}

fn score_line(period: Period, result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!(
            "A new best, beating {}.",
            score_words(period, previous as u32)
        ),
        ScoreResult::Short { best } => {
            format!("Your best is {}.", score_words(period, best as u32))
        }
    }
}

/// The headline of a finished challenge, the results menu's title.
pub fn summary(ch: &Challenge, session: &Session) -> String {
    format!(
        "{} result: {}",
        ch.title(),
        score_words(ch.period, score(session))
    )
}

/// Records a finished challenge and returns the pages that read it back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, session: &Session) -> Vec<String> {
    let total = score(session);
    let first = match &session.kind {
        Kind::Leaves(leaves) => format!("{} of {} leaves made.", session.made, leaves.len()),
        Kind::Games(_) => {
            let games: Vec<String> = session
                .games
                .iter()
                .enumerate()
                .map(|(i, g)| format!("game {}, {}", i + 1, g.score()))
                .collect();
            if games.len() == 1 {
                format!(
                    "Game over. {}.",
                    hearken::text::sentence_start(&points_words(total))
                )
            } else {
                format!("Series over: {}. Total {total}.", games.join(", "))
            }
        }
    };
    crate::progress::save_challenge(ctx, session);
    let result = Challenges::submit(ctx, ch, total as u64);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![
        first,
        format!(
            "{} result: {}. {} A {streak} {} streak. {}",
            ch.title(),
            score_words(ch.period, total),
            score_line(ch.period, result),
            ch.period.unit(),
            ch.resets()
        ),
    ]
}
