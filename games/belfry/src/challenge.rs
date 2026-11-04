//! The daily, weekly and monthly challenges: what each rings, the menu of
//! the three, the briefing read before a challenge, the stand-in screen
//! the launcher opens one from, and the result read after it.

use hearken::text::sentence_start;
use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::rules::{points, row_words, Peal, Rules, CATHEDRAL, VILLAGE};
use crate::{sound, Tower, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "belfry_challenges";

/// Peals in the weekly challenge.
pub const WEEKLY_PEALS: usize = 5;

/// The challenge's name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's peal",
        Period::Weekly => "The week's five",
        Period::Monthly => "The cathedral peal",
    }
}

/// The rules a challenge rings.
pub fn rules(period: Period) -> Rules {
    match period {
        Period::Daily | Period::Weekly => VILLAGE,
        Period::Monthly => CATHEDRAL,
    }
}

/// How many peals a challenge rings.
pub fn peals(period: Period) -> usize {
    match period {
        Period::Weekly => WEEKLY_PEALS,
        _ => 1,
    }
}

/// A few words on the challenge.
pub fn tag(period: Period) -> &'static str {
    match period {
        Period::Daily => "one peal, Village rules",
        Period::Weekly => "five Village peals, scores added",
        Period::Monthly => "5 bells from 8, no hints",
    }
}

/// The rule, in a sentence or two.
pub fn rule(period: Period) -> String {
    let r = rules(period);
    match period {
        Period::Daily => format!("One peal: {}. Hints halve the score.", r.describe()),
        Period::Weekly => format!(
            "Five peals in a row, each with {}. The five scores are added. Hints halve a score.",
            r.describe()
        ),
        Period::Monthly => format!("The cathedral tower: {}. No hints.", r.describe()),
    }
}

/// A challenge in progress: its peals, one after another, and what they
/// have scored.
#[derive(Clone, Debug)]
pub struct Job {
    pub period: Period,
    pub seeds: Vec<u64>,
    /// The peal being rung, from 0.
    pub index: usize,
    pub banked: u64,
    pub solved: u32,
    pub rows: u32,
}

impl Job {
    pub fn new(ch: &Challenge) -> Job {
        let mut rng = ch.rng();
        Job {
            period: ch.period,
            seeds: (0..peals(ch.period)).map(|_| rng.next_u64()).collect(),
            index: 0,
            banked: 0,
            solved: 0,
            rows: 0,
        }
    }

    pub fn peals(&self) -> usize {
        self.seeds.len()
    }

    /// The peal at `index`.
    pub fn peal(&self) -> Peal {
        Peal::new(rules(self.period), self.seeds[self.index])
    }

    /// Banks a finished peal, and returns the next one, if any.
    pub fn advance(&mut self, peal: &Peal) -> Option<Peal> {
        self.banked += peal.score();
        self.rows += peal.rows_used();
        if peal.solved() {
            self.solved += 1;
        }
        self.index += 1;
        (self.index < self.peals()).then(|| self.peal())
    }

    pub fn score(&self) -> u64 {
        self.banked
    }
}

/// The game's side of the shared challenges menu.
pub struct Ringers;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Ringers>;

impl ChallengeGame for Ringers {
    const ID: &'static str = ID;
    const NAME: &'static str = "Belfry";
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
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same peals: a new one each day, five each Monday and a cathedral peal each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(period))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Tower::challenge(ch));
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
        "Opening the Belfry challenge.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// How the last peal ended, in a sentence.
pub fn peal_words(peal: &Peal) -> String {
    if peal.solved() {
        let rows = peal.rows_used();
        format!(
            "Solved in {rows} {}. {}.",
            if rows == 1 { "row" } else { "rows" },
            points(peal.score())
        )
    } else {
        format!("Out of rows. The peal was {}.", row_words(&peal.code))
    }
}

/// Records a finished challenge and returns the pages that read it back.
pub fn finish_job(ctx: &mut Ctx<Parlor>, ch: &Challenge, job: &Job, last: &Peal) -> Vec<String> {
    let score = job.score();
    let first = match ch.period {
        Period::Weekly => format!(
            "{} {} of {} peals solved, in {} rows.",
            peal_words(last),
            job.solved,
            job.peals(),
            job.rows
        ),
        _ => peal_words(last),
    };
    let result = Challenges::submit(ctx, ch, score);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![
        first,
        format!(
            "{} result: {}. {} A {streak} {} streak. {}",
            ch.title(),
            points(score),
            score_line(result),
            ch.period.unit(),
            ch.resets()
        ),
    ]
}

/// "Peal 2 of 5" and what it is, as a new weekly peal starts.
pub fn next_words(job: &Job) -> String {
    format!(
        "Peal {} of {}. {}.",
        job.index + 1,
        job.peals(),
        sentence_start(&rules(job.period).describe())
    )
}
