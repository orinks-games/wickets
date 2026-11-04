//! The daily, weekly and monthly challenges: their tunes and calls, the
//! screens that open them, and what a finished one records.
//!
//! - Daily, "Today's tune": one tune, with turns.
//! - Weekly, "The weekly set": three tunes, with every step.
//! - Monthly, "The long set": five tunes, every step and more of them.
//!
//! The tunes and every call come from the period's seed, so every player
//! dances the same challenge. No shamrocks or clovers here: those belong to
//! the sets and the ceili.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::words::streak_sentence;
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, Rng, ScoreResult};

use crate::play::Floor;
use crate::rules::{self, points_words, steps_words, Run};
use crate::tunes::{Level, EVERY_STEP, TUNES, WITH_TURN};
use crate::{progress, sound, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "clover_jig_challenges";

/// How each challenge is called.
pub fn level(period: Period) -> Level {
    match period {
        Period::Daily => Level {
            steps: WITH_TURN,
            downbeat: 0.9,
            offbeat: 0.15,
        },
        Period::Weekly => Level {
            steps: EVERY_STEP,
            downbeat: 1.0,
            offbeat: 0.3,
        },
        Period::Monthly => Level {
            steps: EVERY_STEP,
            downbeat: 1.0,
            offbeat: 0.45,
        },
    }
}

/// How many tunes each challenge dances.
pub fn tune_count(period: Period) -> usize {
    match period {
        Period::Daily => 1,
        Period::Weekly => 3,
        Period::Monthly => 5,
    }
}

/// Everything a challenge run will be, decided by its seed alone.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub period: Period,
    pub plans: Vec<rules::Plan>,
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        Plan::new(ch.period, Rng::seeded(ch.seed))
    }

    pub fn new(period: Period, mut rng: Rng) -> Plan {
        let mut order: Vec<usize> = (0..TUNES.len()).collect();
        rng.shuffle(&mut order);
        let mut picked: Vec<usize> = order.into_iter().take(tune_count(period)).collect();
        // A set goes from the slowest tune to the fastest.
        picked.sort_by_key(|&i| TUNES[i].bpm);
        let level = level(period);
        let plans = picked
            .into_iter()
            .map(|i| rules::Plan::new(&TUNES[i], &level, &mut rng))
            .collect();
        Plan { period, plans }
    }

    pub fn steps(&self) -> u32 {
        self.plans.iter().map(|p| p.calls.len() as u32).sum()
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's tune",
        Period::Weekly => "The weekly set",
        Period::Monthly => "The long set",
    }
}

/// A few words on what the challenge is, for its row.
pub fn tag(plan: &Plan) -> String {
    match plan.plans.as_slice() {
        [one] => format!("{}, {}", one.tune.title(), steps_words(plan.steps())),
        many => format!("{} tunes, {}", many.len(), steps_words(plan.steps())),
    }
}

/// What the challenge asks, in a sentence or two.
pub fn rule(plan: &Plan) -> String {
    let tunes = match plan.plans.as_slice() {
        [one] => one.tune.title(),
        many => {
            let names: Vec<&str> = many.iter().map(|p| p.tune.name).collect();
            format!("Tunes: {}", names.join(", "))
        }
    };
    let steps = if level(plan.period).steps.len() == EVERY_STEP.len() {
        "every step, forward and back included"
    } else {
        "left, right, hop and turn"
    };
    format!("{tunes}. The caller uses {steps}.")
}

/// The game's side of the shared challenges menu.
pub struct Ceilis;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Ceilis>;

impl ChallengeGame for Ceilis {
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
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player dances the same tunes: a new one each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = Plan::for_challenge(&ch);
    let pages = vec![format!("{}: {}", name(period), rule(&plan))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            let f = Floor::challenge(ctx, ch);
            ctx.replace(f);
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
        "Opening the Clover Jig challenge.".to_string()
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
    format!("{} result: {}", ch.title(), points_words(run.score()))
}

/// Records a finished challenge and returns the pages that read it back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, run: &Run) -> Vec<String> {
    progress::save_challenge(ctx, run);
    let result = Challenges::submit(ctx, ch, run.score() as u64);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    let tunes: Vec<String> = run
        .results
        .iter()
        .map(|r| format!("{}: {}.", r.tune.name, points_words(r.score)))
        .collect();
    vec![
        format!(
            "{}: {} danced. {}",
            name(ch.period),
            match run.results.len() {
                1 => "1 tune".to_string(),
                n => format!("{n} tunes"),
            },
            tunes.join(" ")
        ),
        format!("Steps: {}.", run.tally().words()),
        format!(
            "{} result: {}. {} {} {}",
            ch.title(),
            points_words(run.score()),
            score_line(result),
            streak_sentence(streak, ch.period.unit()),
            ch.resets()
        ),
    ]
}
