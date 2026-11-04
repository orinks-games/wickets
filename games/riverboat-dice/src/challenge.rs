//! The daily, weekly and monthly challenges: what each plays, the menu of
//! the three, the briefing read before a challenge, the stand-in screen
//! the launcher opens one from, and the result read after it.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::rules::{Night, Rules, CAPTAINS_TABLE, CAPTAIN_LANDINGS, SALOON, SNAG_SALOON, YOU};
use crate::{sound, TableScreen, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "riverboat_dice_challenges";

/// Nights in the weekly challenge.
pub const WEEKLY_NIGHTS: usize = 3;

/// The challenge's name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's table",
        Period::Weekly => "Snag week",
        Period::Monthly => "The Captain's table",
    }
}

/// The rules a challenge plays.
pub fn rules(period: Period) -> Rules {
    match period {
        Period::Daily => SALOON,
        Period::Weekly => SNAG_SALOON,
        Period::Monthly => CAPTAINS_TABLE,
    }
}

/// How many nights a challenge plays.
pub fn nights(period: Period) -> usize {
    match period {
        Period::Weekly => WEEKLY_NIGHTS,
        _ => 1,
    }
}

/// A few words on the challenge.
pub fn tag(period: Period) -> String {
    match period {
        Period::Daily => "one saloon night, the same dice for everyone".to_string(),
        Period::Weekly => "three nights with the snag rule, totals added".to_string(),
        Period::Monthly => format!("the Captain alone, {CAPTAIN_LANDINGS} landings"),
    }
}

/// The rule, in a sentence or two.
pub fn rule(period: Period) -> String {
    match period {
        Period::Daily => "One saloon night against the Widow, the Gambler and the Captain. Your score is your night total.".to_string(),
        Period::Weekly => "Three saloon nights. Snag rule: a double 1 wipes the night total of everyone still seated. Your score is the three nights added.".to_string(),
        Period::Monthly => format!("A night of {CAPTAIN_LANDINGS} landings at the Captain's table, you and the Captain alone. Your score is your night total."),
    }
}

/// A challenge in progress: its nights, one after another, and what they
/// have scored.
#[derive(Clone, Debug)]
pub struct Job {
    pub period: Period,
    pub seeds: Vec<u64>,
    /// The night being played, from 0.
    pub index: usize,
    /// Your night totals so far, added.
    pub banked: u64,
    pub wins: u32,
}

impl Job {
    pub fn new(ch: &Challenge) -> Job {
        let mut rng = ch.rng();
        Job {
            period: ch.period,
            seeds: (0..nights(ch.period)).map(|_| rng.next_u64()).collect(),
            index: 0,
            banked: 0,
            wins: 0,
        }
    }

    pub fn nights(&self) -> usize {
        self.seeds.len()
    }

    /// The night at `index`.
    pub fn night(&self) -> Night {
        Night::new(rules(self.period), self.seeds[self.index])
    }

    /// Banks a finished night, and returns the next one, if any.
    pub fn advance(&mut self, night: &Night) -> Option<Night> {
        self.banked += night.you().night as u64;
        if night.winners().contains(&YOU) {
            self.wins += 1;
        }
        self.index += 1;
        (self.index < self.nights()).then(|| self.night())
    }

    pub fn score(&self) -> u64 {
        self.banked
    }
}

/// The game's side of the shared challenges menu.
pub struct Nights;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Nights>;

impl ChallengeGame for Nights {
    const ID: &'static str = ID;
    const NAME: &'static str = "Riverboat Dice";
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
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same dice: a new table each day, a snag week each Monday and a Captain's table each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(period))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(TableScreen::challenge(ch));
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
        "Opening the Riverboat Dice challenge.".to_string()
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
pub fn finish_job(ctx: &mut Ctx<Parlor>, ch: &Challenge, job: &Job, last: &Night) -> Vec<String> {
    let score = job.score();
    let first = match ch.period {
        Period::Weekly => format!(
            "{} You won {} of {} nights. Final totals of the last night: {}.",
            crate::table::outcome_words(last),
            job.wins,
            job.nights(),
            last.standings()
        ),
        _ => format!(
            "{} Final totals: {}.",
            crate::table::outcome_words(last),
            last.standings()
        ),
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

/// "Night 2 of 3", as a new weekly night starts.
pub fn next_words(job: &Job) -> String {
    format!(
        "Night {} of {}. {} banked so far.",
        job.index + 1,
        job.nights(),
        job.banked
    )
}
