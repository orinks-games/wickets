//! Daily, weekly and monthly challenges: the same ice for every player in
//! the period.
//!
//! - Daily, "Tonight's pond": the mill pond, with drifts and thin ice.
//! - Weekly, "Thin ice week": the long lake, with thin ice all the way.
//! - Monthly, "Midwinter skate": the long lake as the snow comes down
//!   hard, each lantern keeping less of the ice clear.

use hearken::text::sentence_start;
use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::play::SkateScreen;
use crate::ponds::pond;
use crate::progress;
use crate::rules::{points_words, Course, Level, Skate};
use crate::sound;
use crate::ID;

pub const CHALLENGES_NAME: &str = "snowfall_skate_challenges";

pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's pond",
        Period::Weekly => "Thin ice week",
        Period::Monthly => "Midwinter skate",
    }
}

/// The ice a challenge skates.
pub fn level(period: Period) -> Level {
    match period {
        Period::Daily => pond(2).level,
        Period::Weekly => Level {
            ice: 0.6,
            drifts: 0.15,
            ..pond(3).level
        },
        Period::Monthly => Level {
            snow: 2.1,
            gain: 2.0,
            gap_ms: 2700,
            lead_ms: 2600,
            ..pond(3).level
        },
    }
}

pub fn course_seeded(period: Period, seed: u64) -> Course {
    Course::new(level(period), seed, Vec::new())
}

pub fn course_for(ch: &Challenge) -> Course {
    course_seeded(ch.period, ch.seed)
}

fn period_of(level: &Level) -> Period {
    Period::ALL
        .into_iter()
        .find(|&p| self::level(p) == *level)
        .unwrap_or(Period::Daily)
}

/// What a challenge asks, from its course.
pub fn rule(course: &Course) -> String {
    match period_of(&course.level) {
        Period::Daily => "The mill pond, with drifts and thin ice, the same for everyone tonight. Skate until the snow covers the ice.".to_string(),
        Period::Weekly => "The long lake, with thin ice all the way, the same for everyone this week. Skate until the snow covers the ice.".to_string(),
        Period::Monthly => "The long lake in a heavy snowfall, the same for everyone this month. Each lantern keeps less of the ice clear.".to_string(),
    }
}

fn tag(period: Period) -> &'static str {
    match period {
        Period::Daily => "the mill pond, drifts and thin ice",
        Period::Weekly => "the long lake, thin ice all the way",
        Period::Monthly => "the long lake in a heavy snowfall",
    }
}

fn result_words(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first try.".to_string(),
        ScoreResult::NewBest { previous } => {
            format!("A new best, up from {}.", points_words(previous as u32))
        }
        ScoreResult::Short { best } => format!("Your best is {}.", points_words(best as u32)),
    }
}

/// Records a challenge skate, and gives the results pages.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, skate: &Skate) -> Vec<String> {
    progress::save_challenge(ctx, skate);
    let result = Challenges::submit(ctx, ch, u64::from(skate.score));
    let streak = Challenges::streak(ctx, ID, ch.period);
    let unit = if streak == 1 {
        ch.period.unit().to_string()
    } else {
        format!("{}s", ch.period.unit())
    };
    vec![
        format!(
            "{}: {}. {}",
            name(ch.period),
            points_words(skate.score),
            result_words(result)
        ),
        crate::play::tally_words(skate),
        format!(
            "{} streak: {streak} {unit}. {}",
            ch.period.label(),
            ch.resets()
        ),
    ]
}

pub fn summary(ch: &Challenge, skate: &Skate) -> String {
    format!(
        "{} over. {}.",
        name(ch.period),
        sentence_start(&points_words(skate.score))
    )
}

/// The pages read before a challenge skate, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(&course_for(&ch)))];
    Pages::briefing(pages).then(move |ctx| {
        let s = SkateScreen::challenge(ctx, ch.clone());
        ctx.replace(s)
    })
}

pub struct Skates;

pub type ChallengesMenu = menus::ChallengesMenu<Skates>;

impl ChallengeGame for Skates {
    const ID: &'static str = ID;
    const NAME: &'static str = crate::NAME;
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(period).to_string()
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(&course_for(&Challenge::current(ctx, ID, period)))
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
}

/// Holds a challenge opened from the launcher, and goes back there after.
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
        "Opening the Snowfall Skate challenge.".to_string()
    }
}
