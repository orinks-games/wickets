//! The daily, weekly and monthly challenges: summer evenings from the
//! almanac, the same for every player whatever the season outside, on a
//! clock, scored by fireflies, species and chords.

use hearken::{Ctx, State};
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, Back, ChallengeGame, Results};
use parlor_core::words::streak_sentence;
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::evening::{self, Mode};
use crate::meadow::Plan;
use crate::rules::{duration_words, Evening};
use crate::species::list_words;
use crate::{sound, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "firefly_meadow_challenges";

/// The challenge's name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's jar",
        Period::Weekly => "The week's chords",
        Period::Monthly => "The long hay evening",
    }
}

/// The plan a challenge plays today.
pub fn plan(ctx: &mut Ctx<Parlor>, period: Period) -> Plan {
    let ch = Challenge::current(ctx, ID, period);
    Plan::challenge(period, &mut ch.rng(), today(ctx))
}

/// A few words on it: "4 minutes in the orchard".
pub fn tag(plan: &Plan) -> String {
    let time = duration_words(plan.clock.unwrap_or(0.0));
    if plan.jar < crate::meadow::JAR {
        format!("{time} in {}, a jar of {}", plan.place.name(), plan.jar)
    } else {
        format!("{time} in {}", plan.place.name())
    }
}

/// How it scores, in a sentence.
pub const SCORING: &str =
    "10 points a firefly, 25 a species, and 40 for each named chord you set free.";

/// The rule, in a sentence or two.
pub fn rule(period: Period, plan: &Plan) -> String {
    let lead = match period {
        Period::Daily => "A summer evening from the almanac",
        Period::Weekly => "A small jar, so chords come quickly",
        Period::Monthly => "A long evening in the hayfield, with every hayfield firefly out",
    };
    format!("{lead}: {}. {SCORING}", tag(plan))
}

/// The page that opens a challenge evening.
pub fn intro(period: Period, plan: &Plan) -> String {
    format!(
        "{}: {}. {} The clock warns you 30 seconds before the light fades.",
        name(period),
        tag(plan),
        SCORING
    )
}

/// The game's side of the shared challenges menu.
pub struct Almanac;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Almanac>;

impl ChallengeGame for Almanac {
    const ID: &'static str = ID;
    const NAME: &'static str = "Firefly Meadow";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(&plan(ctx, period))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        let plan = plan(ctx, period);
        rule(period, &plan)
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same summer evenings: a new one each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = plan(ctx, period);
    let pages = vec![format!(
        "{}: {} Challenges are always summer evenings, whatever the season outside.",
        name(period),
        rule(period, &plan)
    )];
    let seed = ch.seed;
    let mut plan = Some(plan);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(plan) = plan.take() {
            let screen = evening::EveningScreen::new(ctx, plan, Mode::Challenge(period), seed);
            ctx.replace(screen);
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
        "Opening the Firefly Meadow challenge.".to_string()
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
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, evening: &Evening) -> Vec<String> {
    let score = evening.score();
    let caught = match evening.caught {
        1 => "1 firefly".to_string(),
        n => format!("{n} fireflies"),
    };
    let species = match evening.species_tonight.len() {
        1 => "1 species".to_string(),
        n => format!("{n} species"),
    };
    let chords = if evening.chords_tonight.is_empty() {
        "No named chords.".to_string()
    } else {
        let names: Vec<&str> = evening.chords_tonight.iter().map(|c| c.name()).collect();
        format!("Chords: {}.", list_words(&names))
    };
    let first = format!("{} points. {caught} and {species}. {chords}", score);
    let result = Challenges::submit(ctx, ch, score);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    vec![
        first,
        format!(
            "{} result: {score} points. {} {} {}",
            ch.title(),
            score_line(result),
            streak_sentence(streak, ch.period.unit()),
            ch.resets()
        ),
    ]
}

/// The results menu after a challenge: the challenge again, the results
/// again, and back to the challenges.
pub fn after(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same evening, straight away. Only your best result is kept.",
        move |ctx| {
            let pages = briefing(ctx, period);
            ctx.replace(pages);
        },
    )
}
