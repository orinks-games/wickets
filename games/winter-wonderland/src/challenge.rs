//! Daily, weekly and monthly challenges: the same run for every player.
//!
//! The daily alternates: Gift Sort on even days of the month, the Wish
//! Desk on odd ones, each with a twist drawn from the day's seed. The
//! weekly is the holiday double, three Gift Sort shifts then a stack at
//! the desk, scored together under one twist on the belt. The monthly is
//! the midwinter post: three stacks at the desk in a row, every box packed
//! in straw.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Date, Pace, Pages, Parlor, Period, Rng, ScoreResult};

use crate::desk::{Stack, StackSpec};
use crate::desk_run::DeskRun;
use crate::gifts::{Shift, ShiftSpec};
use crate::mode::Mode;
use crate::sort_run::SortRun;
use crate::sound;
use crate::ID;

/// The challenges menu's `State::name`, which results and pause return to.
pub const CHALLENGES_NAME: &str = "winter_wonderland_challenges";

/// The Gift Sort shift the daily plays.
pub const DAILY_SHIFT: u32 = 5;
/// The Wish Desk stack the daily plays.
pub const DAILY_STACK: u32 = 3;
/// The weekly's shifts and stack.
pub const WEEKLY_SHIFTS: (u32, u32) = (4, 6);
pub const WEEKLY_STACK: u32 = 4;
/// The monthly's stacks.
pub const MONTHLY_STACKS: (u32, u32) = (3, 5);

/// A rule that changes a challenge run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Twist {
    /// Gift Sort: the belt runs a quarter faster.
    RushHour,
    /// Gift Sort: four fifths of the usual clock.
    ShortClock,
    /// Gift Sort: three more gifts, with time for them.
    BigOrder,
    /// Gift Sort: wrong guesses cost double.
    Picky,
    /// Wish Desk: every box packed in straw.
    Straw,
    /// Wish Desk: two more boxes on the shelf.
    LongShelf,
    /// Wish Desk: a wrong box costs double.
    Careful,
    /// Wish Desk: only two shakes a letter before they cost points.
    FewShakes,
    /// The monthly: straw in every box, three stacks in a row.
    Midwinter,
}

impl Twist {
    pub const GIFTS: [Twist; 4] = [
        Twist::RushHour,
        Twist::ShortClock,
        Twist::BigOrder,
        Twist::Picky,
    ];
    pub const DESK: [Twist; 4] = [
        Twist::Straw,
        Twist::LongShelf,
        Twist::Careful,
        Twist::FewShakes,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::RushHour => "Rush hour",
            Twist::ShortClock => "Short clock",
            Twist::BigOrder => "Big order",
            Twist::Picky => "Picky guests",
            Twist::Straw => "Packed in straw",
            Twist::LongShelf => "A long shelf",
            Twist::Careful => "Careful hands",
            Twist::FewShakes => "Few shakes",
            Twist::Midwinter => "Straw everywhere",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Twist::RushHour => "the belt runs a quarter faster, so gifts reach the edge sooner",
            Twist::ShortClock => "the shift clock starts with only four fifths of its usual time",
            Twist::BigOrder => "three extra gifts to sort, with a little extra time for them",
            Twist::Picky => "a wrong guess costs twice the usual clock time",
            Twist::Straw => "every box is packed in straw, so its first shake is only straw",
            Twist::LongShelf => "two more boxes on the shelf",
            Twist::Careful => "a wrong box costs twice the usual points",
            Twist::FewShakes => "only two shakes a letter before shakes cost points",
            Twist::Midwinter => "every box is packed in straw, so its first shake is only straw",
        }
    }

    /// Applies a Gift Sort twist to a shift spec.
    pub fn shift_spec(self, spec: &mut ShiftSpec) {
        match self {
            Twist::RushHour => spec.travel *= 0.75,
            Twist::ShortClock => spec.time = (spec.time * 0.8).round(),
            Twist::BigOrder => {
                spec.quota += 3;
                spec.time += 15.0;
            }
            Twist::Picky => spec.wrong_penalty *= 2.0,
            _ => {}
        }
    }

    /// Applies a Wish Desk twist to a stack spec.
    pub fn stack_spec(self, spec: &mut StackSpec) {
        match self {
            Twist::Straw | Twist::Midwinter => spec.straw = true,
            Twist::LongShelf => spec.boxes = (spec.boxes + 2).min(10),
            Twist::Careful => spec.wrong_cost *= 2,
            Twist::FewShakes => spec.free_shakes = 2,
            _ => {}
        }
    }
}

/// Which part a challenge plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// One Gift Sort shift.
    Gifts,
    /// One stack at the Wish Desk.
    Desk,
    /// Gift Sort shifts then a stack, scored together.
    Double,
    /// Three stacks in a row, the monthly.
    Post,
}

/// A challenge, worked out from its seed: what it plays and how.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub challenge: Challenge,
    pub part: Part,
    pub twist: Twist,
    /// The seed the run itself is built from.
    pub run_seed: u64,
}

impl Plan {
    pub fn new(challenge: Challenge) -> Plan {
        let mut rng = challenge.rng();
        let part = match challenge.period {
            Period::Daily => {
                let day = Date::parse(&challenge.key).map(|d| d.day).unwrap_or(0);
                if day.is_multiple_of(2) {
                    Part::Gifts
                } else {
                    Part::Desk
                }
            }
            Period::Weekly => Part::Double,
            Period::Monthly => Part::Post,
        };
        let twist = match part {
            Part::Gifts | Part::Double => *rng.pick(&Twist::GIFTS),
            Part::Desk => *rng.pick(&Twist::DESK),
            Part::Post => Twist::Midwinter,
        };
        Plan {
            challenge,
            part,
            twist,
            run_seed: rng.next_u64(),
        }
    }

    pub fn current(ctx: &Ctx<Parlor>, period: Period) -> Plan {
        Plan::new(Challenge::current(ctx, ID, period))
    }

    pub fn period(&self) -> Period {
        self.challenge.period
    }

    /// The challenge's own name.
    pub fn name(&self) -> &'static str {
        match self.part {
            Part::Gifts => "Rush",
            Part::Desk => "Morning post",
            Part::Double => "The holiday double",
            Part::Post => "The midwinter post",
        }
    }

    /// The Gift Sort shifts it plays, first to last.
    pub fn shifts(&self) -> (u32, u32) {
        match self.part {
            Part::Double => WEEKLY_SHIFTS,
            _ => (DAILY_SHIFT, DAILY_SHIFT),
        }
    }

    /// The Wish Desk stacks it plays, first to last.
    pub fn stacks(&self) -> (u32, u32) {
        match self.part {
            Part::Double => (WEEKLY_STACK, WEEKLY_STACK),
            Part::Post => MONTHLY_STACKS,
            _ => (DAILY_STACK, DAILY_STACK),
        }
    }

    /// Whether the twist applies to Gift Sort (true) or the Wish Desk.
    pub fn twists_gifts(&self) -> bool {
        Twist::GIFTS.contains(&self.twist)
    }

    /// A few words for the challenge's row in a menu.
    pub fn tag(&self) -> String {
        match self.part {
            Part::Gifts => format!("Gift Sort, shift {DAILY_SHIFT}, {}", self.twist.name()),
            Part::Desk => format!("the Wish Desk, stack {DAILY_STACK}, {}", self.twist.name()),
            Part::Double => format!(
                "Gift Sort shifts {} to {}, then the Wish Desk, {}",
                WEEKLY_SHIFTS.0,
                WEEKLY_SHIFTS.1,
                self.twist.name()
            ),
            Part::Post => format!(
                "the Wish Desk, stacks {} to {}, every box in straw",
                MONTHLY_STACKS.0, MONTHLY_STACKS.1
            ),
        }
    }

    /// The twist, as a sentence: "Today's twist, Rush hour: ...".
    pub fn twist_line(&self) -> String {
        let when = match self.period() {
            Period::Daily => "Today's twist",
            Period::Weekly => "This week's twist",
            Period::Monthly => "This month's rule",
        };
        format!("{when}, {}: {}.", self.twist.name(), self.twist.describe())
    }

    /// What the challenge asks, in a sentence or three.
    pub fn rule(&self) -> String {
        let what = match self.part {
            Part::Gifts => format!(
                "One Gift Sort shift at shift {DAILY_SHIFT}. Sort the whole order before the clock runs out."
            ),
            Part::Desk => format!(
                "One stack of letters at the Wish Desk, stack {DAILY_STACK}. Fewer shakes and no wrong boxes score more."
            ),
            Part::Double => format!(
                "Gift Sort shifts {} to {}, then stack {WEEKLY_STACK} at the Wish Desk. Both scores add up. The twist is on the belt.",
                WEEKLY_SHIFTS.0, WEEKLY_SHIFTS.1
            ),
            Part::Post => format!(
                "Stacks {} to {} at the Wish Desk, one after another.",
                MONTHLY_STACKS.0, MONTHLY_STACKS.1
            ),
        };
        format!("{what} {}", self.twist_line())
    }

    /// The page read before the run.
    pub fn briefing_pages(&self) -> Vec<String> {
        vec![format!("{}: {}", self.name(), self.rule())]
    }

    /// The first shift of a Gift Sort part.
    pub fn shift(&self, level: u32, rng: Rng) -> Shift {
        let mut shift = Shift::new(level, rng);
        if self.twists_gifts() {
            self.twist.shift_spec(&mut shift.spec);
            shift.clock = shift.spec.time;
        }
        shift
    }

    /// Stack `level` of a Wish Desk part.
    pub fn stack(&self, level: u32, rng: Rng) -> Stack {
        let mut spec = StackSpec::new(level);
        if !self.twists_gifts() {
            self.twist.stack_spec(&mut spec);
        }
        Stack::new(spec, rng)
    }

    /// Replaces the top screen with the run.
    pub fn start(&self, ctx: &mut Ctx<Parlor>) {
        let mode = Mode::Challenge {
            plan: Box::new(self.clone()),
            gifts: 0,
        };
        match self.part {
            Part::Gifts | Part::Double => ctx.replace(SortRun::new(mode, self.shifts().0)),
            Part::Desk | Part::Post => ctx.replace(DeskRun::new(mode, self.stacks().0)),
        }
    }
}

/// The briefing pages for `period`'s challenge, which then start the run.
/// Escape goes back without starting.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let plan = Plan::current(ctx, period);
    let pages = plan.briefing_pages();
    Pages::briefing(pages).then(move |ctx| {
        plan.start(ctx);
    })
}

/// Submits a finished run, played at `pace`, on that pace's board and
/// says how it went: result, best, streak and when the next one comes.
pub fn result_line(ctx: &mut Ctx<Parlor>, plan: &Plan, pace: Pace, score: u64) -> String {
    let ch = &plan.challenge;
    let result = Challenges::submit_at(ctx, ch, pace, score);
    let r = crate::progress::relaxed_word(pace);
    let line = match result {
        ScoreResult::First => format!("Your first {r}result on this challenge: {score}."),
        ScoreResult::NewBest { previous } => {
            format!("A new {r}best. The old {r}best was {previous}.")
        }
        ScoreResult::Short { best } => format!("Your {r}best on this challenge is {best}."),
    };
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    format!(
        "{} score {score}. {line} A {streak} {} streak. {}",
        ch.title(),
        ch.period.unit(),
        ch.resets()
    )
}

/// The game's side of the shared challenges menu.
pub struct Parties;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Parties>;

impl ChallengeGame for Parties {
    const ID: &'static str = ID;
    const NAME: &'static str = "Winter Wonderland";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        Plan::current(ctx, period).name().to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        Plan::current(ctx, period).tag()
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        Plan::current(ctx, period).rule()
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(sound::TITLE_MUSIC), true);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same runs: a new daily each day, a weekly each Monday and a monthly each month.".to_string()
    }
}

/// Stands in for the challenges menu when the launcher opens a challenge
/// directly: it opens the briefing, and once the run is over (or left from
/// the pause menu) it steps aside, back to the launcher.
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
    /// Named as the challenges menu is, so leaving a run, by its results or
    /// the pause menu, lands here and steps aside to the launcher.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            let pages = briefing(ctx, self.period);
            ctx.push(pages);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
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
        "Opening the Winter Wonderland challenge.".to_string()
    }
}
