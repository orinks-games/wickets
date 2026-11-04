//! Daily, weekly and monthly challenges: the same run for every player.
//!
//! The daily alternates: Gift Sort on even days of the month, Midnight
//! Countdown on odd ones, each with a twist drawn from the day's seed. The
//! weekly is the holiday double, three Gift Sort shifts then a night,
//! scored together under one twist. The monthly is the Midnight Gala, a
//! New Year special where every instrument plays from the first minute,
//! with its own finale at midnight.

use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Challenges, Date, Pages, Parlor, Period, Rng, ScoreResult};

use crate::count_run::CountRun;
use crate::gifts::{Shift, ShiftSpec};
use crate::mode::Mode;
use crate::night::{Night, NightSpec};
use crate::sort_run::SortRun;
use crate::sound;
use crate::ID;

/// The challenges menu's `State::name`, which results and pause return to.
pub const CHALLENGES_NAME: &str = "winter_wonderland_challenges";

/// The Gift Sort shift the daily plays.
pub const DAILY_SHIFT: u32 = 5;
/// The night the Midnight Countdown daily plays.
pub const DAILY_NIGHT: u32 = 3;
/// The weekly's shifts and night.
pub const WEEKLY_SHIFTS: (u32, u32) = (4, 6);
pub const WEEKLY_NIGHT: u32 = 4;
/// The night the gala is built on (its tempo and timing).
pub const GALA_NIGHT: u32 = 3;
/// The gala's venue, in place of the night's name.
pub const GALA_NAME: &str = "the Midnight Gala";

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
    /// Countdown: the band plays faster.
    UpTempo,
    /// Countdown: each miss drains half again as much energy.
    Fragile,
    /// Countdown: the timing windows are a quarter narrower.
    Tight,
    /// Countdown: the party starts low on energy.
    LateArrivals,
    /// The monthly gala: every instrument from the first minute.
    Gala,
}

impl Twist {
    pub const GIFTS: [Twist; 4] = [
        Twist::RushHour,
        Twist::ShortClock,
        Twist::BigOrder,
        Twist::Picky,
    ];
    pub const COUNT: [Twist; 4] = [
        Twist::UpTempo,
        Twist::Fragile,
        Twist::Tight,
        Twist::LateArrivals,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::RushHour => "Rush hour",
            Twist::ShortClock => "Short clock",
            Twist::BigOrder => "Big order",
            Twist::Picky => "Picky guests",
            Twist::UpTempo => "Up-tempo",
            Twist::Fragile => "Fragile party",
            Twist::Tight => "Tight timing",
            Twist::LateArrivals => "Late arrivals",
            Twist::Gala => "The full orchestra",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Twist::RushHour => "the belt runs a quarter faster, so gifts reach the edge sooner",
            Twist::ShortClock => "the shift clock starts with only four fifths of its usual time",
            Twist::BigOrder => "three extra gifts to sort, with a little extra time for them",
            Twist::Picky => "a wrong guess costs twice the usual clock time",
            Twist::UpTempo => "the band plays about fifteen percent faster",
            Twist::Fragile => "every miss drains half again as much party energy",
            Twist::Tight => "you must answer a quarter closer to the beat",
            Twist::LateArrivals => "the party starts with only 35 energy instead of 60",
            Twist::Gala => "bells, drums and horns all play from ten minutes to midnight, and the party starts at full energy",
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

    /// Applies a countdown twist to a night.
    pub fn night(self, night: &mut Night) {
        match self {
            Twist::UpTempo => night.spec.bpm = (night.spec.bpm * 1.15).round(),
            Twist::Fragile => night.spec.drain = night.spec.drain * 3 / 2,
            Twist::Tight => {
                night.spec.perfect *= 0.75;
                night.spec.good *= 0.75;
            }
            Twist::LateArrivals => night.energy = 35,
            Twist::Gala => {
                night.gala = true;
                night.energy = 100;
                night.spec.name = GALA_NAME;
            }
            _ => {}
        }
    }
}

/// Which part a challenge plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// One Gift Sort shift.
    Gifts,
    /// One night of Midnight Countdown.
    Countdown,
    /// Gift Sort shifts then a night, scored together.
    Double,
    /// The gala night.
    Gala,
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
                    Part::Countdown
                }
            }
            Period::Weekly => Part::Double,
            Period::Monthly => Part::Gala,
        };
        let twist = match part {
            Part::Gifts | Part::Double => *rng.pick(&Twist::GIFTS),
            Part::Countdown => *rng.pick(&Twist::COUNT),
            Part::Gala => Twist::Gala,
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
            Part::Countdown => "Midnight warm-up",
            Part::Double => "The holiday double",
            Part::Gala => "The Midnight Gala",
        }
    }

    /// The Gift Sort shifts it plays, first to last.
    pub fn shifts(&self) -> (u32, u32) {
        match self.part {
            Part::Double => WEEKLY_SHIFTS,
            _ => (DAILY_SHIFT, DAILY_SHIFT),
        }
    }

    /// The night it plays, for the countdown parts.
    pub fn night_level(&self) -> u32 {
        match self.part {
            Part::Double => WEEKLY_NIGHT,
            Part::Gala => GALA_NIGHT,
            _ => DAILY_NIGHT,
        }
    }

    /// Whether the twist applies to Gift Sort (true) or the countdown.
    pub fn twists_gifts(&self) -> bool {
        Twist::GIFTS.contains(&self.twist)
    }

    /// A few words for the challenge's row in a menu.
    pub fn tag(&self) -> String {
        match self.part {
            Part::Gifts => format!("Gift Sort, shift {DAILY_SHIFT}, {}", self.twist.name()),
            Part::Countdown => format!(
                "Midnight Countdown at {}, {}",
                NightSpec::new(DAILY_NIGHT).name,
                self.twist.name()
            ),
            Part::Double => format!(
                "Gift Sort shifts {} to {}, then {}, {}",
                WEEKLY_SHIFTS.0,
                WEEKLY_SHIFTS.1,
                NightSpec::new(WEEKLY_NIGHT).name,
                self.twist.name()
            ),
            Part::Gala => "every instrument from the first minute".to_string(),
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
            Part::Countdown => format!(
                "One night of Midnight Countdown at {}. Keep the party going to midnight.",
                NightSpec::new(DAILY_NIGHT).name
            ),
            Part::Double => format!(
                "Gift Sort shifts {} to {}, then New Year's Eve on {}. Both scores add up. The twist is on the belt.",
                WEEKLY_SHIFTS.0,
                WEEKLY_SHIFTS.1,
                NightSpec::new(WEEKLY_NIGHT).name
            ),
            Part::Gala => {
                "Ring in midnight for the finale.".to_string()
            }
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

    /// The night of a countdown part.
    pub fn night(&self) -> Night {
        let mut rng = Rng::seeded(self.run_seed);
        // The Gift Sort half of the weekly draws first; the night has its own.
        let mut night = Night::new(
            self.night_level(),
            false,
            Rng::seeded(rng.next_u64() ^ 0x5eed),
        );
        if !self.twists_gifts() {
            self.twist.night(&mut night);
        }
        night
    }

    /// Replaces the top screen with the run.
    pub fn start(&self, ctx: &mut Ctx<Parlor>) {
        let mode = Mode::Challenge {
            plan: Box::new(self.clone()),
            gifts: 0,
        };
        match self.part {
            Part::Gifts | Part::Double => ctx.replace(SortRun::new(mode, self.shifts().0)),
            Part::Countdown | Part::Gala => ctx.replace(CountRun::with_night(mode, self.night())),
        }
    }
}

/// Plays the period's challenge music (the gala's song for the monthly).
/// The briefing pages for `period`'s challenge, which then start the run.
/// Escape goes back without starting.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let plan = Plan::current(ctx, period);
    let pages = plan.briefing_pages();
    Pages::briefing(pages).then(move |ctx| {
        plan.start(ctx);
    })
}

/// Submits a finished run and says how it went: result, best, streak and
/// when the next one comes.
pub fn result_line(ctx: &mut Ctx<Parlor>, plan: &Plan, score: u64) -> String {
    let ch = &plan.challenge;
    let result = Challenges::submit(ctx, ch, score);
    let line = match result {
        ScoreResult::First => format!("Your first result on this challenge: {score}."),
        ScoreResult::NewBest { previous } => {
            format!("A new best. The old best was {previous}.")
        }
        ScoreResult::Short { best } => format!("Your best on this challenge is {best}."),
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
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same runs: a new daily each day, a weekly each Monday and a gala each month.".to_string()
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
