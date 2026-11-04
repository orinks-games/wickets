//! The daily, weekly and monthly challenges: their mazes, the screens that
//! open them, and what a finished one records.
//!
//! - Daily, "Tonight's pantry": one seeded 9 by 9 maze, two cats.
//! - Weekly, "Five pantries": five seeded 9 by 9 mazes in a row, the cats
//!   quicker in each.
//! - Monthly, "The big larder": one seeded 13 by 13 maze, three cats and
//!   no Hint.
//!
//! Every run has 3 whiskers. Losing the last ends it, and the points so far
//! still count.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, Rng, ScoreResult};

use crate::rules::{points_words, Course, Maze, Outcome, Pace, Rules, Run, WHISKERS};
use crate::{sound, Play, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "pantry_mouse_challenges";

/// Everything a challenge run will be, decided by its seed alone.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub period: Period,
    pub mazes: Vec<Maze>,
    pub rules: Rules,
    /// The seed for the cats' wandering.
    pub seed: u64,
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        Plan::new(ch.period, ch.rng())
    }

    pub fn new(period: Period, mut rng: Rng) -> Plan {
        let (count, size, rules) = match period {
            Period::Daily => (
                1,
                9,
                Rules {
                    cats: 2,
                    first_cats: 2,
                    pace: Pace {
                        start: 620,
                        step: 0,
                        floor: 620,
                    },
                    whiskers: true,
                    hints: true,
                },
            ),
            Period::Weekly => (
                5,
                9,
                Rules {
                    cats: 2,
                    first_cats: 2,
                    pace: Pace {
                        start: 650,
                        step: 50,
                        floor: 400,
                    },
                    whiskers: true,
                    hints: true,
                },
            ),
            Period::Monthly => (
                1,
                13,
                Rules {
                    cats: 3,
                    first_cats: 3,
                    pace: Pace {
                        start: 600,
                        step: 0,
                        floor: 600,
                    },
                    whiskers: true,
                    hints: false,
                },
            ),
        };
        let mazes = (0..count)
            .map(|_| Maze::generate(&mut rng, size, size))
            .collect();
        Plan {
            period,
            mazes,
            rules,
            seed: rng.next_u64(),
        }
    }

    /// The run this plan describes, not yet started.
    pub fn run(&self) -> Run {
        Run::new(
            Course::Mazes(self.mazes.clone()),
            self.rules.clone(),
            self.seed,
        )
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's pantry",
        Period::Weekly => "Five pantries",
        Period::Monthly => "The big larder",
    }
}

/// A few words on what the challenge is, for its row.
pub fn tag(period: Period) -> &'static str {
    match period {
        Period::Daily => "one 9 by 9 maze, 2 cats",
        Period::Weekly => "5 mazes in a row, 2 cats",
        Period::Monthly => "a 13 by 13 maze, 3 cats, no hints",
    }
}

/// What the challenge asks, in a sentence or two.
pub fn rule(period: Period) -> String {
    let what = match period {
        Period::Daily => "One 9 by 9 maze with the ginger cat and the tabby cat.",
        Period::Weekly => {
            "Five 9 by 9 mazes in a row with the ginger cat and the tabby cat, quicker in each."
        }
        Period::Monthly => {
            "One 13 by 13 maze with the ginger, tabby and grey cats. Hint does not work here."
        }
    };
    format!("{what} You have {WHISKERS} whiskers.")
}

/// The game's side of the shared challenges menu.
pub struct Larders;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Larders>;

impl ChallengeGame for Larders {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
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
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same mazes: new ones each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(period))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Play::challenge(ch));
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
        "Opening the Pantry Mouse challenge.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// The headline of a finished challenge, the results menu's title.
pub fn summary(ch: &Challenge, run: &Run) -> String {
    format!("{} result: {}", ch.title(), points_words(run.score))
}

/// What happened, in one sentence.
pub fn outcome_words(run: &Run) -> String {
    let total = run.pantries().unwrap_or(1);
    match run.outcome {
        Some(Outcome::Cleared) if total == 1 => "Maze clear.".to_string(),
        Some(Outcome::Cleared) => format!("All {total} mazes clear."),
        _ if total == 1 => "Out of whiskers.".to_string(),
        _ => format!(
            "Out of whiskers in maze {} of {total}, {} cleared.",
            run.pantry, run.cleared
        ),
    }
}

/// Records a finished challenge and returns the pages that read it back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, run: &Run) -> Vec<String> {
    crate::progress::save_crumbs(ctx, run.eaten);
    let result = Challenges::submit(ctx, ch, run.score as u64);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![
        format!("{} {}.", outcome_words(run), points_words(run.score)),
        format!(
            "{} result: {}. {} A {streak} {} streak. {}",
            ch.title(),
            points_words(run.score),
            score_line(result),
            ch.period.unit(),
            ch.resets()
        ),
    ]
}
