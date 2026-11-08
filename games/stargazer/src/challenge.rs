//! The daily, weekly and monthly challenges: what each asks, the menu of
//! the three, the briefing, the stand-in screen the launcher opens one
//! from, and the result read after it.

use hearken::text::sentence_start;
use hearken::{Ctx, State};
use parlor_core::challenges::today;
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult, Season};

use crate::almanac;
use crate::rules::{star_words, Goal, Night};
use crate::screens::NightScreen;
use crate::sky::{season_constellations, season_wonders, CONSTELLATIONS};
use crate::{sound, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "stargazer_challenges";
/// Stars in the daily hunt.
pub const HUNT_STARS: usize = 5;
/// Constellations in the weekly challenge.
pub const WEEKLY_PATTERNS: usize = 3;

pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's star hunt",
        Period::Weekly => "The week's patterns",
        Period::Monthly => "The grand tour",
    }
}

pub fn tag(period: Period, season: Season) -> String {
    match period {
        Period::Daily => format!("find {HUNT_STARS} named stars"),
        Period::Weekly => format!("trace {WEEKLY_PATTERNS} constellations"),
        Period::Monthly => format!(
            "every constellation and wonder of the {} sky",
            season.name()
        ),
    }
}

pub fn rule(period: Period) -> String {
    match period {
        Period::Daily => format!(
            "Find {HUNT_STARS} stars, named one at a time. Each is worth 100 points. Each hint costs 25 points; using the guide tone costs 50."
        ),
        Period::Weekly => format!(
            "Trace {WEEKLY_PATTERNS} constellations. 10 points a star and 100 a constellation, less 10 a hint."
        ),
        Period::Monthly => "Trace every constellation in this season's sky and find its wonders. 10 points a star, 100 a constellation and 150 a wonder, less 10 a hint.".to_string(),
    }
}

/// What a challenge asks for, from its seed: the same for every player in
/// the same season.
pub fn goal(ch: &Challenge, season: Season) -> Goal {
    let mut rng = ch.rng();
    let cons = season_constellations(season);
    match ch.period {
        Period::Daily => {
            let mut stars: Vec<(usize, usize)> = cons
                .iter()
                .flat_map(|&c| (0..CONSTELLATIONS[c].stars.len()).map(move |s| (c, s)))
                .collect();
            rng.shuffle(&mut stars);
            stars.truncate(HUNT_STARS);
            Goal::Stars(stars)
        }
        Period::Weekly => {
            let mut list = cons;
            rng.shuffle(&mut list);
            list.truncate(WEEKLY_PATTERNS);
            list.sort();
            Goal::Patterns(list)
        }
        Period::Monthly => Goal::Tour,
    }
}

/// The game's side of the shared challenges menu.
pub struct Skywatch;

pub type ChallengesMenu = menus::ChallengesMenu<Skywatch>;

impl ChallengeGame for Skywatch {
    const ID: &'static str = ID;
    const NAME: &'static str = "Stargazer";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(period, almanac::sky_season(today(ctx)))
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
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same sky: a star hunt each day, three patterns each week and a grand tour each month.".to_string()
    }
}

/// What the challenge holds, for its briefing.
fn goal_words(goal: &Goal, season: Season) -> String {
    match goal {
        Goal::Stars(list) => {
            let first = list
                .first()
                .map(|&(c, s)| star_words(c, s))
                .unwrap_or_default();
            format!("The first star to find is {first}. The rest are named as you go.")
        }
        Goal::Patterns(list) => {
            let names: Vec<&str> = list.iter().map(|&c| CONSTELLATIONS[c].name).collect();
            format!("This week's constellations: {}.", names.join(", "))
        }
        Goal::Tour => format!(
            "The {} sky holds {} constellations and {} wonders.",
            season.name(),
            season_constellations(season).len(),
            season_wonders(season).len()
        ),
        Goal::Free => String::new(),
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let season = almanac::sky_season(today(ctx));
    let pages = vec![format!(
        "{}: {} {}",
        name(period),
        rule(period),
        goal_words(&goal(&ch, season), season)
    )];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            let screen = NightScreen::challenge(ctx, ch);
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
        "Opening the Stargazer challenge.".to_string()
    }
}

fn score_line(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// How far the night got, in a sentence.
pub fn outcome_words(night: &Night) -> String {
    match &night.goal {
        Goal::Stars(list) => format!("{} of {} stars found.", night.hunted, list.len()),
        Goal::Patterns(list) => {
            let traced = list.iter().filter(|c| night.done.contains(c)).count();
            format!("{traced} of {} constellations traced.", list.len())
        }
        Goal::Tour => {
            let season = night.season();
            format!(
                "{} of {} constellations traced and {} of {} wonders found.",
                night.done.len(),
                season_constellations(season).len(),
                night.found.len(),
                season_wonders(season).len()
            )
        }
        Goal::Free => String::new(),
    }
}

/// Records a finished challenge and returns the pages that read it back.
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, night: &Night) -> Vec<String> {
    let score = night.score();
    let result = Challenges::submit(ctx, ch, score);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    let hints = match night.hints {
        0 => "No hints used.".to_string(),
        1 => "1 hint used.".to_string(),
        n => format!("{n} hints used."),
    };
    vec![
        format!("{} {hints}", sentence_start(&outcome_words(night))),
        format!(
            "{} result: {score} points. {} A {streak} {} streak. {}",
            ch.title(),
            score_line(result),
            ch.period.unit(),
            ch.resets()
        ),
    ]
}
