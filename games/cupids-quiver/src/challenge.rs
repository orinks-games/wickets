//! The daily, weekly and monthly challenges: what each holds, the menu of
//! the three, the briefing, the stand-in screen the launcher opens one
//! from, and the result read after.

use hearken::{Ctx, State};
use parlor_core::menus::{self, labels, Back, ChallengeGame, Results};
use parlor_core::words::streak_sentence;
use parlor_core::{Challenge, Challenges, Pages, Parlor, Period, ScoreResult};

use crate::field::{Field, Launch};
use crate::rules::{arrows_words, hearts_words, Mode, Plan, Run, Twist};
use crate::{sound, ID};

/// The challenge list's `State::name`.
pub const CHALLENGES_NAME: &str = "cupids_quiver_challenges";

/// The mode a challenge plays.
pub fn mode(period: Period) -> Mode {
    match period {
        Period::Daily => Mode::Daily,
        Period::Weekly => Mode::Weekly,
        Period::Monthly => Mode::Monthly,
    }
}

/// The daily's twist, from its seed. The weekly and monthly have none.
pub fn twist(ch: &Challenge) -> Option<Twist> {
    match ch.period {
        Period::Daily => {
            let mut rng = ch.rng();
            Some(Twist::ALL[rng.below(Twist::ALL.len() as u32) as usize])
        }
        _ => None,
    }
}

/// The run a challenge starts, the same for every player.
pub fn launch(ch: &Challenge) -> Launch {
    let mut rng = ch.rng();
    let _twist_roll = rng.next_u64();
    Launch {
        mode: mode(ch.period),
        seed: rng.next_u64(),
        twist: twist(ch),
    }
}

/// The challenge's name.
pub fn name(period: Period) -> &'static str {
    mode(period).name()
}

/// A few words on what it is this time.
pub fn tag(ch: &Challenge) -> String {
    match ch.period {
        Period::Daily => match twist(ch) {
            Some(t) => format!("one long round, with {}", t.name()),
            None => "one long round".to_string(),
        },
        Period::Weekly => "three rooftops, each busier".to_string(),
        Period::Monthly => "five moonlit rounds, with roses".to_string(),
    }
}

/// The rule, in a sentence or two.
pub fn rule(ch: &Challenge) -> String {
    match ch.period {
        Period::Daily => {
            let plan = Plan::new(Mode::Daily, 1, false, twist(ch));
            let twist = twist(ch)
                .map(|t| format!(" Twist: {}.", t.describe()))
                .unwrap_or_default();
            format!(
                "One round with {}, as many hearts as you can.{twist}",
                arrows_words(plan.arrows)
            )
        }
        Period::Weekly => {
            "Three rooftops in a row, each busier than the last. Every round is played, and the points are added.".to_string()
        }
        Period::Monthly => "Five rounds under the full moon, faster each time, with roses in the sky worth 25 more. Every round is played.".to_string(),
    }
}

/// The game's side of the shared challenges menu.
pub struct Archers;

/// The three challenges, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Archers>;

impl ChallengeGame for Archers {
    const ID: &'static str = ID;
    const NAME: &'static str = "Cupid's Quiver";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(&Challenge::current(ctx, ID, period))
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(&Challenge::current(ctx, ID, period))
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(&ch))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(Field::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the challenge is over or left it steps
/// aside, back to the launcher.
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
        "Opening the Cupid's Quiver challenge.".to_string()
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
pub fn finish(ctx: &mut Ctx<Parlor>, ch: &Challenge, run: &Run) -> Vec<String> {
    let score = run.score;
    let result = Challenges::submit(ctx, ch, score);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    let t = &run.tally;
    let pairs = match t.pairs {
        1 => "1 sweetheart pair".to_string(),
        n => format!("{n} sweetheart pairs"),
    };
    vec![
        format!(
            "{} done. {} from {}, {pairs}.",
            name(ch.period),
            hearts_words(t.hits),
            arrows_words(t.shots)
        ),
        format!(
            "{} result: {score} points. {} {} {}",
            ch.title(),
            score_line(result),
            streak_sentence(streak, ch.period.unit()),
            ch.resets()
        ),
    ]
}

/// The results menu after a challenge.
pub fn after(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same sky, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Field::challenge(ch))
        },
    )
}
