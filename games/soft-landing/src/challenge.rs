//! The daily, weekly and monthly landings: their plans, the screens that
//! open them, and what a finished one records.
//!
//! - Daily, "Today's landing": one seeded moon.
//! - Weekly, "The weekly survey": three seeded moons in a row on one tank.
//! - Monthly, "The deep moon": one heavy seeded moon, little spare fuel,
//!   and no Hint.
//!
//! A plan comes from the challenge's seed alone, so every player gets the
//! same moons that period. A crash ends a run, and the landings before it
//! still count.

use hearken::text::sentence_start;
use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pace, Pages, Parlor, Period, Rng, ScoreResult};

use crate::rules::{fuel_points, landing_points, speed_words, Flight, Moon, Outcome};
use crate::{sound, Descent, ID, NAME};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "soft_landing_challenges";

/// How many times the best burn a daily tank holds.
pub const DAILY_MARGIN: f64 = 2.0;
/// The weekly tank, as a multiple of the three best burns together.
pub const WEEKLY_MARGIN: f64 = 1.7;
/// The monthly tank: little to spare.
pub const MONTHLY_MARGIN: f64 = 1.4;

/// Everything a challenge run will be, from its seed alone.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub period: Period,
    pub moons: Vec<Moon>,
    /// The one tank for the whole run, in seconds of burn.
    pub tank: f64,
    /// Whether the Hint key helps.
    pub hints: bool,
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        Plan::new(ch.period, ch.rng())
    }

    pub fn new(period: Period, mut rng: Rng) -> Plan {
        match period {
            Period::Daily => {
                let moon = Moon::seeded(&mut rng, DAILY_MARGIN, false);
                Plan {
                    period,
                    tank: moon.fuel,
                    moons: vec![moon],
                    hints: true,
                }
            }
            Period::Weekly => {
                let moons: Vec<Moon> = (0..3).map(|_| Moon::seeded(&mut rng, 1.0, false)).collect();
                let need: f64 = moons.iter().map(Moon::needed_burn).sum();
                let tank = (need * WEEKLY_MARGIN).ceil();
                let moons = moons
                    .into_iter()
                    .map(|m| Moon { fuel: tank, ..m })
                    .collect();
                Plan {
                    period,
                    moons,
                    tank,
                    hints: true,
                }
            }
            Period::Monthly => {
                let moon = Moon::seeded(&mut rng, MONTHLY_MARGIN, true);
                Plan {
                    period,
                    tank: moon.fuel,
                    moons: vec![moon],
                    hints: false,
                }
            }
        }
    }
}

/// A challenge run in progress, moon after moon.
#[derive(Clone, Debug)]
pub struct Job {
    pub plan: Plan,
    /// The moon being flown, from 0.
    pub index: usize,
    /// Points earned so far.
    pub banked: u32,
    /// Fuel carried to the next moon.
    pub fuel: f64,
    pub landed: u32,
    /// The fuel bonus, paid once at the end of a run that lands every moon.
    pub fuel_bonus: u32,
    /// The last flight's end.
    pub outcome: Option<Outcome>,
}

impl Job {
    pub fn new(plan: Plan) -> Job {
        Job {
            fuel: plan.tank,
            plan,
            index: 0,
            banked: 0,
            landed: 0,
            fuel_bonus: 0,
            outcome: None,
        }
    }

    pub fn moons(&self) -> usize {
        self.plan.moons.len()
    }

    /// The flight onto the current moon, with the fuel left.
    pub fn flight(&self) -> Flight {
        Flight::with_fuel(self.plan.moons[self.index], self.fuel, self.plan.tank)
    }

    /// Counts a finished flight. Returns the next one, or `None` when the
    /// run is over.
    pub fn advance(&mut self, flight: &Flight) -> Option<Flight> {
        let outcome = flight.outcome?;
        self.outcome = Some(outcome);
        if !outcome.landed() {
            return None;
        }
        self.landed += 1;
        self.banked += landing_points(&flight.moon, outcome.speed());
        self.fuel = flight.fuel;
        if self.index + 1 < self.moons() {
            self.index += 1;
            return Some(self.flight());
        }
        self.fuel_bonus = fuel_points(self.fuel, self.plan.tank);
        self.banked += self.fuel_bonus;
        None
    }

    pub fn won(&self) -> bool {
        self.landed as usize == self.moons()
    }

    pub fn score(&self) -> u32 {
        self.banked
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's landing",
        Period::Weekly => "The weekly survey",
        Period::Monthly => "The deep moon",
    }
}

/// A moon in a few words: "Saffron, medium gravity, 450 metres up".
pub fn moon_words(moon: &Moon) -> String {
    format!(
        "{}, {}, {} up",
        moon.name,
        moon.gravity_words(),
        parlor_core::units::length(moon.height)
    )
}

/// A few words on what the challenge is this time, for its row.
pub fn tag(plan: &Plan) -> String {
    match plan.period {
        Period::Daily => format!("one moon, {}", moon_words(&plan.moons[0])),
        Period::Weekly => {
            let names: Vec<&str> = plan.moons.iter().map(|m| m.name).collect();
            format!("{} on one tank", names.join(", "))
        }
        Period::Monthly => format!("{}, no hints", moon_words(&plan.moons[0])),
    }
}

/// What the challenge asks, in a sentence or two.
pub fn rule(plan: &Plan) -> String {
    match plan.period {
        Period::Daily => format!(
            "One landing on {}. Safe speed {}.",
            moon_words(&plan.moons[0]),
            speed_words(plan.moons[0].safe)
        ),
        Period::Weekly => format!(
            "Three landings in a row on one tank of fuel: {}. A crash ends the run, and the landings before it still count.",
            plan.moons
                .iter()
                .map(|m| m.name)
                .collect::<Vec<_>>()
                .join(", then ")
        ),
        Period::Monthly => format!(
            "One hard landing on {}, with little spare fuel and no hints. Safe speed {}.",
            moon_words(&plan.moons[0]),
            speed_words(plan.moons[0].safe)
        ),
    }
}

/// The game's side of the shared challenges menu.
pub struct Landings;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Landings>;

impl ChallengeGame for Landings {
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
        "Up and Down choose a landing, Enter reads its briefing, Escape goes back. Every player gets the same moons: a new one each day, each Monday and each month.".to_string()
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
            ctx.replace(Descent::challenge(ch));
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
        "Opening the Soft Landing challenge.".to_string()
    }
}

fn score_line(result: ScoreResult, pace: Pace) -> String {
    let best = pace.best_word();
    match result {
        ScoreResult::First if pace.separate() => {
            "Your first relaxed result on this challenge.".to_string()
        }
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new {best}, beating {previous}."),
        ScoreResult::Short { best: b } => format!("Your {best} is {b}."),
    }
}

/// The headline of a finished challenge, the results menu's title.
pub fn summary(ch: &Challenge, job: &Job) -> String {
    format!("{} result: {} points", ch.title(), job.score())
}

/// Records a finished challenge flown at `pace` and returns the pages
/// that read it back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, job: &Job, pace: Pace) -> Vec<String> {
    let n = job.moons();
    // Touchdown already said where and how fast: a one-moon challenge's
    // pages lead with the result, a longer one's with the moons landed.
    let first = match job.outcome {
        _ if n == 1 => None,
        Some(Outcome::Crashed { .. }) => Some(format!("{} of {n} moons landed.", job.landed)),
        _ => Some(format!(
            "All {n} moons landed. {} points of fuel bonus.",
            job.fuel_bonus
        )),
    };
    crate::progress::save_challenge(ctx, job);
    let result = Challenges::submit_at(ctx, ch, pace, job.score() as u64);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    first
        .into_iter()
        .chain([format!(
            "{} result: {} points. {} A {streak} {} streak. {}",
            ch.title(),
            job.score(),
            score_line(result, pace),
            ch.period.unit(),
            sentence_start(&ch.resets())
        )])
        .collect()
}
