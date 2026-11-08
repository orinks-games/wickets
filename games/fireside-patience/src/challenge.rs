//! The daily, weekly and monthly challenges: what each deals, the menu of
//! the three, the briefing read before one, the stand-in screen the
//! launcher opens one from, and the result read after it.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::pool::{DRAW_THREE, RELAXED};
use crate::rules::{moves_words, points, Game, POINTS};
use crate::{sound, PlayScreen, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "fireside_patience_challenges";

/// Deals in the monthly challenge.
pub const MONTHLY_DEALS: usize = 5;

/// The challenge's name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's deal",
        Period::Weekly => "Draw three week",
        Period::Monthly => "Five by the fire",
    }
}

/// A few words on the challenge.
pub fn tag(period: Period) -> String {
    match period {
        Period::Daily => "one deal, fewest moves".to_string(),
        Period::Weekly => "one draw three deal, fewest moves".to_string(),
        Period::Monthly => format!("{MONTHLY_DEALS} deals, fewest moves in all"),
    }
}

/// The rule, in a sentence or two.
pub fn rule(period: Period) -> String {
    let score = format!("Each won deal scores {POINTS} points, less 1 for every move.");
    match period {
        Period::Daily => format!("One draw one deal that can always be won. {score}"),
        Period::Weekly => format!("One draw three deal: the stock turns 3 cards at a time, and only the top one plays. It can always be won. {score}"),
        Period::Monthly => format!("{MONTHLY_DEALS} draw one deals in a row, each one winnable. {score} Your total is the sum of the {MONTHLY_DEALS} deal scores."),
    }
}

/// How many cards the challenge's stock turns at a time.
pub fn draw(period: Period) -> usize {
    match period {
        Period::Weekly => 3,
        _ => 1,
    }
}

/// A challenge in progress: its deals, one after another, and the moves
/// they took.
#[derive(Clone, Debug)]
pub struct Job {
    pub period: Period,
    pub seeds: Vec<u64>,
    /// The deal being played, from 0.
    pub index: usize,
    /// Moves in the deals won so far, added.
    pub moves: u32,
    /// Points from the deals won so far.
    pub score: u64,
}

impl Job {
    /// The challenge's deals, picked from the proved pool by its seed.
    pub fn new(ch: &Challenge) -> Job {
        let mut rng = ch.rng();
        let pool = if ch.period == Period::Weekly {
            DRAW_THREE
        } else {
            RELAXED
        };
        let count = if ch.period == Period::Monthly {
            MONTHLY_DEALS
        } else {
            1
        };
        let mut picks: Vec<u64> = pool.to_vec();
        rng.shuffle(&mut picks);
        picks.truncate(count);
        Job {
            period: ch.period,
            seeds: picks,
            index: 0,
            moves: 0,
            score: 0,
        }
    }

    pub fn deals(&self) -> usize {
        self.seeds.len()
    }

    /// The deal at `index`.
    pub fn game(&self) -> Game {
        Game::deal(self.seeds[self.index], draw(self.period))
    }

    /// Banks a won deal, and returns the next one, if any.
    pub fn advance(&mut self, won: &Game) -> Option<Game> {
        self.moves += won.moves;
        self.score += points(won.moves);
        self.index += 1;
        (self.index < self.deals()).then(|| self.game())
    }
}

/// The game's side of the shared challenges menu.
pub struct Deals;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Deals>;

impl ChallengeGame for Deals {
    const ID: &'static str = ID;
    const NAME: &'static str = "Fireside Patience";
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
        sound::fireside(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same cards: a new deal each day, a draw three deal each Monday and five deals each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(period))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(PlayScreen::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the challenge is over (or left from the
/// pause menu) it steps aside, back to the launcher.
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
    /// Named as the challenges menu is, so leaving a challenge lands here
    /// and steps aside to the launcher.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            sound::fireside(ctx);
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
        "Opening the Fireside Patience challenge.".to_string()
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
pub fn finish_job(ctx: &mut Ctx<Parlor>, ch: &Challenge, job: &Job) -> Vec<String> {
    let score = job.score;
    let first = if job.deals() > 1 {
        format!(
            "All {} deals won, in {} in all.",
            job.deals(),
            moves_words(job.moves)
        )
    } else {
        format!("Deal won in {}.", moves_words(job.moves))
    };
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

/// "Deal 2 of 5.", as a new monthly deal starts.
pub fn next_words(job: &Job) -> String {
    format!(
        "Deal {} of {}. {} so far.",
        job.index + 1,
        job.deals(),
        moves_words(job.moves)
    )
}
