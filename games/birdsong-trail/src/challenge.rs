//! The daily, weekly and monthly challenges, the same for every player:
//!
//! - Daily, "Morning walk": a trail and 5 of its birds, each singing its
//!   signature song. 3 points for a bird answered first time, 2 after one
//!   miss, 1 after more.
//! - Weekly, "Chorus week": 8 birds of the season singing at once, to
//!   pick out one by one. 2 points first time, 1 after a miss.
//! - Monthly, "The great count": 8 flocks of up to 12 birds. 3 points
//!   for an exact count, 1 for one off.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{
    challenges, Challenge, Challenges, Date, Pages, Parlor, Period, ScoreResult, Season,
};

use crate::birds::{self, Species};
use crate::chorus::ChorusScreen;
use crate::count::CountScreen;
use crate::rules::{best_possible, flocks, Chorus, Count, Flock, Walk};
use crate::trails::Trail;
use crate::walk::WalkScreen;
use crate::{days, sound, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "birdsong_trail_challenges";
/// Birds on the morning walk.
pub const WALK_BIRDS: usize = 5;
/// Birds in the chorus week.
pub const CHORUS_BIRDS: usize = 8;
/// Flocks in the great count.
pub const COUNT_FLOCKS: usize = 8;

/// Everything a challenge run will be, decided by its seed and date.
#[derive(Clone, Debug)]
pub enum Plan {
    Walk {
        trail: Trail,
        season: Season,
        birds: Vec<&'static Species>,
        seed: u64,
    },
    Chorus {
        season: Season,
        birds: Vec<&'static Species>,
        seed: u64,
    },
    Count {
        flocks: Vec<Flock>,
        north: bool,
    },
}

impl Plan {
    /// The plan of `ch`, a challenge of the period holding `today`.
    pub fn new(ch: &Challenge, today: Date) -> Plan {
        let mut rng = ch.rng();
        let season = days::season(today);
        match ch.period {
            Period::Daily => {
                let trail = Trail::ALL[rng.below(Trail::ALL.len() as u32) as usize];
                let mut all = birds::on_trail(trail.habitat(), season);
                rng.shuffle(&mut all);
                all.truncate(WALK_BIRDS);
                Plan::Walk {
                    trail,
                    season,
                    birds: all,
                    seed: rng.next_u64(),
                }
            }
            Period::Weekly => {
                let mut all = birds::of_season(season);
                rng.shuffle(&mut all);
                all.truncate(CHORUS_BIRDS);
                Plan::Chorus {
                    season,
                    birds: all,
                    seed: rng.next_u64(),
                }
            }
            Period::Monthly => Plan::Count {
                flocks: flocks(COUNT_FLOCKS, 3, 12, &mut rng),
                north: today.month <= 6,
            },
        }
    }

    /// The plan of the current challenge.
    pub fn current(ctx: &Ctx<Parlor>, period: Period) -> Plan {
        let ch = Challenge::current(ctx, ID, period);
        Plan::new(&ch, challenges::today(ctx))
    }

    pub fn walk(&self) -> Walk {
        match self {
            Plan::Walk {
                trail,
                season,
                birds,
                seed,
            } => Walk::new(
                *trail,
                *season,
                birds.iter().map(|b| (*b, 0)).collect(),
                *seed,
            ),
            _ => Walk::practice(),
        }
    }

    pub fn chorus(&self) -> Chorus {
        match self {
            Plan::Chorus { birds, seed, .. } => Chorus::new(birds.clone(), *seed),
            _ => Chorus::new(Vec::new(), 0),
        }
    }

    pub fn count(&self) -> Count {
        match self {
            Plan::Count { flocks, north } => Count::new(flocks.clone(), *north),
            _ => Count::new(Vec::new(), true),
        }
    }

    /// The best score the run can make.
    pub fn most(&self) -> u32 {
        match self {
            Plan::Walk { birds, .. } => birds.len() as u32 * 3,
            Plan::Chorus { birds, .. } => birds.len() as u32 * 2,
            Plan::Count { flocks, .. } => best_possible(flocks.len()),
        }
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Morning walk",
        Period::Weekly => "Chorus week",
        Period::Monthly => "The great count",
    }
}

/// A few words on what it is this time, for its row.
pub fn tag(plan: &Plan) -> String {
    match plan {
        Plan::Walk {
            trail,
            season,
            birds,
            ..
        } => format!(
            "the {} in {}, {} birds",
            trail.name(),
            season.name(),
            birds.len()
        ),
        Plan::Chorus { season, birds, .. } => {
            format!("{} birds of {} singing at once", birds.len(), season.name())
        }
        Plan::Count { flocks, north } => format!(
            "{} flocks flying {}",
            flocks.len(),
            if *north { "north" } else { "south" }
        ),
    }
}

/// What the challenge asks, in a sentence or two.
pub fn rule(plan: &Plan) -> String {
    match plan {
        Plan::Walk { trail, birds, .. } => format!(
            "Walk the {} and answer its {} birds, each singing its signature song. A bird answered first time is 3 points, after one miss 2, after more 1.",
            trail.name(),
            birds.len()
        ),
        Plan::Chorus { birds, .. } => format!(
            "{} birds sing around you at once. Pick out each one as it is asked for: 2 points first time, 1 after a miss.",
            birds.len()
        ),
        Plan::Count { flocks, .. } => format!(
            "{} flocks fly over, the largest with up to 12 birds. Count each one: 3 points for an exact count, 1 for one off.",
            flocks.len()
        ),
    }
}

/// The game's side of the shared challenges menu.
pub struct BirdChallenges;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<BirdChallenges>;

impl ChallengeGame for BirdChallenges {
    const ID: &'static str = ID;
    const NAME: &'static str = NAME;
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(&Plan::current(ctx, period))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(&Plan::current(ctx, period))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same birds: a new walk each day, a new chorus each Monday and a new count each month.".to_string()
    }
}

/// Starts the challenge's run, replacing the screen on top or pushing it.
pub fn start(ctx: &mut Ctx<Parlor>, ch: Challenge, replace: bool) {
    let plan = Plan::new(&ch, challenges::today(ctx));
    match (&plan, replace) {
        (Plan::Walk { .. }, false) => ctx.push(WalkScreen::challenge(ch, &plan)),
        (Plan::Walk { .. }, true) => ctx.replace(WalkScreen::challenge(ch, &plan)),
        (Plan::Chorus { season, .. }, false) => {
            ctx.push(ChorusScreen::challenge(ch, plan.chorus(), *season))
        }
        (Plan::Chorus { season, .. }, true) => {
            ctx.replace(ChorusScreen::challenge(ch, plan.chorus(), *season))
        }
        (Plan::Count { .. }, false) => ctx.push(CountScreen::challenge(ch, plan.count())),
        (Plan::Count { .. }, true) => ctx.replace(CountScreen::challenge(ch, plan.count())),
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = Plan::new(&ch, challenges::today(ctx));
    let pages = vec![format!("{}: {}", name(period), rule(&plan))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            start(ctx, ch, true);
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
        "Opening the Birdsong Trail challenge.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// Submits a finished run's score and returns the page that reads it.
fn submit(ctx: &mut Ctx<Parlor>, ch: &Challenge, score: u32, most: u32) -> Vec<String> {
    let result = Challenges::submit(ctx, ch, score as u64);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![format!(
        "{} result: {score} of {most} points. {} A {streak} {} streak. {}",
        ch.title(),
        score_line(result),
        ch.period.unit(),
        ch.resets()
    )]
}

/// A run's score and the most it could make.
pub fn walk_points(walk: &Walk) -> (u32, u32) {
    (walk.score(), walk.birds.len() as u32 * 3)
}

pub fn chorus_points(chorus: &Chorus) -> (u32, u32) {
    (chorus.score(), chorus.total as u32 * 2)
}

pub fn count_points(count: &Count) -> (u32, u32) {
    (count.score(), best_possible(count.flocks.len()))
}

/// The results headline with the score after it, for S to repeat.
pub fn scored(summary: &str, (score, most): (u32, u32)) -> String {
    format!("{summary} Score {score} of {most}.")
}

pub fn finish_walk(ctx: &mut Ctx<Parlor>, ch: &Challenge, walk: &Walk) -> Vec<String> {
    let (score, most) = walk_points(walk);
    submit(ctx, ch, score, most)
}

pub fn finish_chorus(ctx: &mut Ctx<Parlor>, ch: &Challenge, chorus: &Chorus) -> Vec<String> {
    let (score, most) = chorus_points(chorus);
    submit(ctx, ch, score, most)
}

pub fn finish_count(ctx: &mut Ctx<Parlor>, ch: &Challenge, count: &Count) -> Vec<String> {
    let (score, most) = count_points(count);
    submit(ctx, ch, score, most)
}
