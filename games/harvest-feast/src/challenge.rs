//! Daily, weekly and monthly challenges: the same kitchen for every player.
//!
//! - Daily, Tonight's dinner: one seeded meal on four stations, with a twist
//!   drawn from the day's seed (rush hour, a picky guest, a hot burner, a
//!   quiet kitchen or butterfingers).
//! - Weekly, Sunday dinner: a seeded three-course dinner (starter, main,
//!   dessert) on three, four and then five stations, with the week's twist.
//!   Each course must earn a star for the next to be served.
//! - Monthly, the harvest banquet: ten dishes, two on every station, and
//!   golden plates: every dish served at 90 or better is worth 200 more.
//!
//! The plan is rebuilt from the challenge's seed whenever it is needed, so
//! every screen (the row, the briefing, the kitchen, the result) agrees.

use hearken::{Bus, Ctx, State};
use parlor_core::menus::{self, labels, Back, ChallengeGame, Results};
use parlor_core::{Challenge, Challenges, Pace, Pages, Parlor, Period, Rng, ScoreResult};

use crate::kitchen::{Kitchen, Mode};
use crate::rules::{
    sentence, stars_phrase, Course, DishKind, Meal, Outcome, Plan, Station, MIN_WINDOW,
};
use crate::sound;
use crate::ID;

/// A dish served at this quality or better in the banquet is a golden plate.
pub const GOLDEN_QUALITY: u32 = 90;
/// What a golden plate adds.
pub const GOLDEN_POINTS: u64 = 200;
/// Below this the picky guest sends their dish back.
pub const PICKY_QUALITY: u32 = 75;
/// What a fumble costs on a butterfingers day.
pub const BUTTERFINGERS_COST: u32 = 20;
/// Rush hour multiplies the meal's timings by this.
pub const RUSH_SPEED: f64 = 0.8;
/// A hot burner multiplies its dishes' waits by this, and their windows by
/// `HOT_WINDOW`.
pub const HOT_WAIT: f64 = 0.6;
pub const HOT_WINDOW: f64 = 0.7;

/// What changes a daily or weekly challenge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Twist {
    Rush,
    Picky,
    HotBurner,
    Quiet,
    Butterfingers,
}

impl Twist {
    pub const ALL: [Twist; 5] = [
        Twist::Rush,
        Twist::Picky,
        Twist::HotBurner,
        Twist::Quiet,
        Twist::Butterfingers,
    ];

    /// "Rush hour".
    pub fn name(self) -> &'static str {
        match self {
            Twist::Rush => "rush hour",
            Twist::Picky => "a picky guest",
            Twist::HotBurner => "a hot burner",
            Twist::Quiet => "a quiet kitchen",
            Twist::Butterfingers => "butterfingers",
        }
    }
}

/// One challenge in progress: which one, which course of it, and the
/// points carried from the courses before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeRun {
    pub challenge: Challenge,
    /// 0 for the first meal; the weekly has three.
    pub stage: usize,
    /// Points from the meals already served.
    pub total: u64,
    /// Golden plates so far (the banquet).
    pub plates: u32,
}

impl ChallengeRun {
    pub fn new(challenge: Challenge) -> ChallengeRun {
        ChallengeRun {
            challenge,
            stage: 0,
            total: 0,
            plates: 0,
        }
    }

    pub fn period(&self) -> Period {
        self.challenge.period
    }

    pub fn plan(&self) -> ChallengePlan {
        ChallengePlan::for_challenge(&self.challenge)
    }

    /// "Daily challenge, Tonight's dinner", "Weekly challenge, the main course".
    pub fn title(&self) -> String {
        format!(
            "{}, {}",
            self.challenge.title(),
            meal_name(self.period(), self.stage)
        )
    }

    /// The same challenge, one course on.
    pub fn next(&self, points: u64, plates: u32) -> ChallengeRun {
        ChallengeRun {
            challenge: self.challenge.clone(),
            stage: self.stage + 1,
            total: self.total + points,
            plates: self.plates + plates,
        }
    }
}

/// The challenge's own name.
pub fn challenge_name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's dinner",
        Period::Weekly => "Sunday dinner",
        Period::Monthly => "The harvest banquet",
    }
}

/// The name of one meal of a challenge.
pub fn meal_name(period: Period, stage: usize) -> &'static str {
    match (period, stage) {
        (Period::Daily, _) => "Tonight's dinner",
        (Period::Weekly, 0) => "the starter",
        (Period::Weekly, 1) => "the main course",
        (Period::Weekly, _) => "dessert",
        (Period::Monthly, _) => "The harvest banquet",
    }
}

/// How a challenge meal scored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MealScore {
    pub points: u64,
    pub plates: u32,
    /// The picky guest's dish and whether they sent it back.
    pub picky: Option<(DishKind, bool)>,
}

/// Everything a challenge is, built from its seed.
#[derive(Clone, Debug)]
pub struct ChallengePlan {
    pub period: Period,
    /// The daily or weekly twist; the banquet has its own rule instead.
    pub twist: Option<Twist>,
    /// The dish the picky guest wants.
    pub picky: Option<DishKind>,
    /// The burner that runs hot.
    pub hot: Option<Station>,
    pub courses: Vec<Course>,
}

const STARTER: [DishKind; 7] = [
    DishKind::Celery,
    DishKind::Herbs,
    DishKind::GreenBeans,
    DishKind::CranberrySauce,
    DishKind::CornCakes,
    DishKind::Cider,
    DishKind::Rolls,
];
const MAIN: [DishKind; 7] = [
    DishKind::Turkey,
    DishKind::Stuffing,
    DishKind::Gravy,
    DishKind::Potatoes,
    DishKind::Sprouts,
    DishKind::Celery,
    DishKind::GreenBeans,
];
const DESSERT: [DishKind; 6] = [
    DishKind::PumpkinPie,
    DishKind::Caramel,
    DishKind::Apples,
    DishKind::Cream,
    DishKind::Fritters,
    DishKind::Cider,
];

/// A seeded meal: `count` stations (always holding `must`, when given),
/// one dish from `pool` at each, called in a random order.
fn seeded_meal(
    rng: &mut Rng,
    pool: &[DishKind],
    count: usize,
    must: Option<Station>,
    speed: f64,
    name: &'static str,
    blurb: &'static str,
) -> Course {
    let mut stations: Vec<Station> = Station::ALL
        .iter()
        .copied()
        .filter(|s| pool.iter().any(|k| k.station() == *s))
        .collect();
    rng.shuffle(&mut stations);
    stations.truncate(count);
    if let Some(m) = must {
        if !stations.contains(&m) {
            if let Some(last) = stations.last_mut() {
                *last = m;
            }
        }
    }
    let mut start = 1.0;
    let plan = stations
        .iter()
        .map(|&station| {
            let here: Vec<DishKind> = pool
                .iter()
                .copied()
                .filter(|k| k.station() == station)
                .collect();
            let here = if here.is_empty() {
                DishKind::ALL
                    .iter()
                    .copied()
                    .filter(|k| k.station() == station)
                    .collect()
            } else {
                here
            };
            let kind = *rng.pick(&here);
            let p = Plan {
                kind,
                start_at: start,
                after: None,
            };
            start += 2.0 + rng.below(3) as f64;
            p
        })
        .collect();
    Course {
        number: 0,
        name,
        blurb,
        speed,
        plan,
    }
}

/// The banquet: two dishes on every station, the second waiting for the
/// first to be served.
fn banquet(rng: &mut Rng) -> Course {
    let mut stations = Station::ALL.to_vec();
    rng.shuffle(&mut stations);
    let mut firsts = Vec::new();
    let mut seconds = Vec::new();
    for &station in &stations {
        let mut pool: Vec<DishKind> = DishKind::ALL
            .iter()
            .copied()
            .filter(|k| k.station() == station)
            .collect();
        rng.shuffle(&mut pool);
        firsts.push(pool[0]);
        seconds.push(pool[1 % pool.len()]);
    }
    let mut plan = Vec::new();
    let mut start = 1.0;
    for &kind in &firsts {
        plan.push(Plan {
            kind,
            start_at: start,
            after: None,
        });
        start += 2.0 + rng.below(2) as f64;
    }
    for (i, &kind) in seconds.iter().enumerate() {
        plan.push(Plan {
            kind,
            start_at: 1.5 + rng.below(3) as f64,
            after: Some(i),
        });
    }
    Course {
        number: 0,
        name: "The harvest banquet",
        blurb: "Ten dishes, two on every station.",
        speed: 0.75,
        plan,
    }
}

impl ChallengePlan {
    pub fn for_challenge(ch: &Challenge) -> ChallengePlan {
        let mut rng = ch.rng();
        if ch.period == Period::Monthly {
            return ChallengePlan {
                period: ch.period,
                twist: None,
                picky: None,
                hot: None,
                courses: vec![banquet(&mut rng)],
            };
        }
        let twist = *rng.pick(&Twist::ALL);
        let hot = (twist == Twist::HotBurner).then(|| {
            if rng.chance(0.5) {
                Station::LeftBurner
            } else {
                Station::RightBurner
            }
        });
        let speed = |s: f64| {
            if twist == Twist::Rush {
                (s * RUSH_SPEED).max(0.5)
            } else {
                s
            }
        };
        let courses = match ch.period {
            Period::Daily => vec![seeded_meal(
                &mut rng,
                &DishKind::ALL,
                4,
                hot,
                speed(0.8),
                "Tonight's dinner",
                "One meal on four stations.",
            )],
            _ => vec![
                seeded_meal(
                    &mut rng,
                    &STARTER,
                    3,
                    hot,
                    speed(0.85),
                    "The starter",
                    "Three stations to warm up.",
                ),
                seeded_meal(
                    &mut rng,
                    &MAIN,
                    4,
                    hot,
                    speed(0.78),
                    "The main course",
                    "Four stations, quicker.",
                ),
                seeded_meal(
                    &mut rng,
                    &DESSERT,
                    5,
                    hot,
                    speed(0.72),
                    "Dessert",
                    "All five stations, quicker still.",
                ),
            ],
        };
        let picky = (twist == Twist::Picky).then(|| {
            // The picky guest's dish is in the daily meal, or the main course.
            let course = &courses[if courses.len() > 1 { 1 } else { 0 }];
            let kinds: Vec<DishKind> = course.plan.iter().map(|p| p.kind).collect();
            *rng.pick(&kinds)
        });
        ChallengePlan {
            period: ch.period,
            twist: Some(twist),
            picky,
            hot,
            courses,
        }
    }

    /// The course for a stage of the run.
    pub fn course(&self, stage: usize) -> Course {
        self.courses[stage.min(self.courses.len() - 1)].clone()
    }

    /// Whether spoken call-outs are off for this challenge.
    pub fn quiet(&self) -> bool {
        self.twist == Some(Twist::Quiet)
    }

    /// Applies the twist to a meal just built from one of the courses.
    pub fn apply(&self, meal: &mut Meal) {
        for d in meal.dishes.iter_mut() {
            if self.twist == Some(Twist::Butterfingers) {
                d.fumble_cost = BUTTERFINGERS_COST;
            }
            if Some(d.station()) == self.hot {
                for s in d.steps.iter_mut().skip(1) {
                    s.wait *= HOT_WAIT;
                    s.window = (s.window * HOT_WINDOW).max(MIN_WINDOW);
                    s.burn = (s.burn * HOT_WINDOW).max(MIN_WINDOW);
                }
            }
        }
    }

    /// A short tag for the challenge's row in the list.
    pub fn tag(&self) -> String {
        match (self.period, self.twist) {
            (Period::Monthly, _) => {
                "ten dishes across five stations, with golden plates".to_string()
            }
            (Period::Weekly, Some(t)) => format!("three courses, with {}", t.name()),
            (_, Some(t)) => format!("one meal on four stations, with {}", t.name()),
            (_, None) => "one meal".to_string(),
        }
    }

    /// What the twist or rule does, in a sentence or two.
    pub fn rule(&self) -> String {
        let twist = match self.twist {
            None => format!(
                "Golden plates: every dish served at quality {GOLDEN_QUALITY} or better adds {GOLDEN_POINTS} points."
            ),
            Some(Twist::Rush) => "Rush hour: everything cooks a fifth faster.".to_string(),
            Some(Twist::Picky) => {
                let dish = self.picky.map_or("dish", |k| k.name());
                format!("A picky guest: Uncle Walt only eats the {dish}. It scores double, but served below {PICKY_QUALITY} he sends it back, for nothing.")
            }
            Some(Twist::HotBurner) => {
                let st = self.hot.map_or("burner", |s| s.name());
                format!("A hot burner: the {st} runs hot. Dishes there come due much sooner and burn faster.")
            }
            Some(Twist::Quiet) => "A quiet kitchen: no spoken call-outs. The recipe still calls each dish, but only its sound marks when a dish needs you. The bonus for cooking without call-outs applies.".to_string(),
            Some(Twist::Butterfingers) => format!("Butterfingers: every fumble costs {BUTTERFINGERS_COST} instead of 5."),
        };
        let shape = match self.period {
            Period::Daily => "One meal on four stations, the same for every player.".to_string(),
            Period::Weekly => "Three courses on three, four, then five stations, each faster. Each course needs a star for the next.".to_string(),
            Period::Monthly => "Ten dishes, two on every station, the second waiting for the first.".to_string(),
        };
        let what = match self.period {
            Period::Monthly => "Rule",
            _ => "Twist",
        };
        format!("{shape} {what}, {twist}")
    }

    /// A reminder of the twist for the kitchen's status and help.
    pub fn reminder(&self) -> String {
        match self.twist {
            None => format!("Golden plates at quality {GOLDEN_QUALITY}."),
            Some(Twist::Picky) => format!(
                "Uncle Walt is waiting on the {}.",
                self.picky.map_or("dish", |k| k.name())
            ),
            Some(Twist::HotBurner) => {
                format!("The {} runs hot.", self.hot.map_or("burner", |s| s.name()))
            }
            Some(t) => format!("Twist: {}.", t.name()),
        }
    }

    /// Scores a meal of this challenge.
    pub fn score(&self, outcome: &Outcome, by_ear: bool) -> MealScore {
        let mut base = 0u64;
        let mut picky = None;
        for &(kind, q) in &outcome.qualities {
            let mut w = 1;
            if Some(kind) == self.picky {
                let sent_back = q < PICKY_QUALITY;
                picky = Some((kind, sent_back));
                w = if sent_back { 0 } else { 2 };
            }
            base += q as u64 * 10 * w;
        }
        let mut points = if by_ear { base + base / 10 } else { base };
        let plates = if self.twist.is_none() {
            outcome
                .qualities
                .iter()
                .filter(|q| q.1 >= GOLDEN_QUALITY)
                .count() as u32
        } else {
            0
        };
        points += plates as u64 * GOLDEN_POINTS;
        MealScore {
            points,
            plates,
            picky,
        }
    }
}

/// What the kitchen adds when a dish of a challenge is served.
pub fn served_note(
    plan: &ChallengePlan,
    kind: DishKind,
    quality: u32,
    plates: u32,
) -> Option<String> {
    if plan.twist.is_none() && quality >= GOLDEN_QUALITY {
        return Some(format!("Golden plate. {plates} this banquet."));
    }
    if Some(kind) == plan.picky {
        return Some(if quality < PICKY_QUALITY {
            format!("Uncle Walt sends the {} back.", kind.name())
        } else {
            format!("Uncle Walt loves the {}: double points.", kind.name())
        });
    }
    None
}

/// What a challenge result did to the record, on the board for `pace`.
fn result_line(result: ScoreResult, pace: Pace) -> String {
    let best = pace.best_word();
    match result {
        ScoreResult::First if pace.separate() => {
            "Your first relaxed result on this challenge.".to_string()
        }
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new {best}, up from {previous}."),
        ScoreResult::Short { best: b } => format!("Your {best} is {b}."),
    }
}

/// Ends one challenge meal: on to the next course at the run's `pace`, or
/// the result, on the board for that pace.
pub fn finish(
    ctx: &mut Ctx<Parlor>,
    run: ChallengeRun,
    outcome: Outcome,
    by_ear: bool,
    pace: Pace,
) {
    let plan = run.plan();
    let score = plan.score(&outcome, by_ear);
    let passed = outcome.passed();
    let last = run.stage + 1 >= plan.courses.len();
    let dishes: Vec<String> = outcome
        .qualities
        .iter()
        .map(|(k, q)| format!("{} {q}", sentence(k.name())))
        .collect();
    let mut summary = if passed {
        format!(
            "{} served. {}. Average quality {}. {}.",
            sentence(meal_name(run.period(), run.stage)),
            sentence(&stars_phrase(outcome.stars)),
            outcome.average,
            dishes.join(". ")
        )
    } else {
        format!(
            "The guests push their plates away: no stars for {}. Average quality {}. {}.",
            meal_name(run.period(), run.stage).to_lowercase(),
            outcome.average,
            dishes.join(". ")
        )
    };
    if let Some((kind, sent_back)) = score.picky {
        if sent_back {
            summary.push_str(&format!(" Uncle Walt sent back the {}.", kind.name()));
        } else {
            summary.push_str(&format!(
                " Uncle Walt loved the {}: double points.",
                kind.name()
            ));
        }
    }
    if plan.twist.is_none() {
        summary.push_str(&format!(
            " {} golden {}, {} points.",
            score.plates,
            if score.plates == 1 { "plate" } else { "plates" },
            score.plates as u64 * GOLDEN_POINTS
        ));
    }
    summary.push_str(&format!(" {} points", score.points));
    if by_ear {
        summary.push_str(", with the by-ear bonus");
    }
    summary.push('.');
    ctx.audio.set_bed(None);
    if let Some(key) = sound::stars(outcome.stars) {
        ctx.audio.play(key, Bus::UI);
    }
    if passed && !last {
        let next = run.next(score.points, score.plates);
        let c = plan.course(next.stage);
        ctx.audio.play(sound::COURSE_UP, Bus::MUSIC);
        let pages = vec![
            format!("{summary} Running total {}.", next.total),
            format!(
                "Next, {}: {} {}.",
                meal_name(next.period(), next.stage),
                c.blurb,
                crate::dishes(c.plan.len())
            ),
        ];
        ctx.replace(Pages::new(pages).then(move |ctx| {
            ctx.replace(Kitchen::new(Mode::Challenge(next.clone())).at_pace(pace));
        }));
        return;
    }
    let total = run.total + score.points;
    let ch = run.challenge.clone();
    let result = Challenges::submit_at(ctx, &ch, pace, total);
    let streak = Challenges::streak(ctx, ID, ch.period).max(1);
    let stinger = if !passed {
        sound::LOSE
    } else if ch.period == Period::Daily {
        sound::WIN
    } else {
        sound::FEAST
    };
    ctx.audio.play(stinger, Bus::MUSIC);
    let ending = if ch.period == Period::Weekly && !passed {
        format!(
            " Sunday dinner ends after {}.",
            match run.stage {
                0 => "the starter".to_string(),
                1 => "the starter and the main course".to_string(),
                _ => "all three courses".to_string(),
            }
        )
    } else {
        String::new()
    };
    let pages = vec![
        format!("{summary}{ending}"),
        format!(
            "{} result: {total} points. {} A {streak} {} streak. {}",
            ch.title(),
            result_line(result, pace),
            ch.period.unit(),
            ch.resets()
        ),
    ];
    let period = ch.period;
    let summary = format!("{} result: {total} points", ch.title());
    let mut results = Some(
        Results::new(summary, pages.clone(), Back::Challenges(CHALLENGES_NAME)).again(
            labels::challenge_again(period),
            "The same menu again, from its briefing. Only your best result is kept.",
            move |ctx| {
                let pages = briefing(ctx, period);
                ctx.replace(pages)
            },
        ),
    );
    ctx.replace(Pages::new(pages).then(move |ctx| {
        if let Some(results) = results.take() {
            ctx.replace(results);
        }
    }));
}

/// The pages read before a challenge, which then open the kitchen. Escape
/// on them goes back without starting.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let plan = ChallengePlan::for_challenge(&ch);
    let pages = vec![format!("{}: {}", challenge_name(period), plan.rule())];
    let run = ChallengeRun::new(ch);
    Pages::briefing(pages).then(move |ctx| {
        ctx.replace(Kitchen::new(Mode::Challenge(run.clone())));
    })
}

/// The challenges menu's `State::name`, which challenge results and the
/// pause screen return to.
pub const CHALLENGES_NAME: &str = "harvest_feast_challenges";

/// The game's side of the shared challenges menu.
pub struct Dinners;

/// The three challenges, each with its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Dinners>;

impl ChallengeGame for Dinners {
    const ID: &'static str = ID;
    const NAME: &'static str = "Harvest Feast";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        challenge_name(period).to_string()
    }

    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        ChallengePlan::for_challenge(&Challenge::current(ctx, ID, period)).tag()
    }

    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String {
        ChallengePlan::for_challenge(&Challenge::current(ctx, ID, period)).rule()
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same kitchen: a new daily dinner each day, a three-course Sunday dinner each week from Monday, and a harvest banquet each month.".to_string()
    }
}

/// Stands in for the challenges menu when the launcher opens a challenge
/// directly: it opens the briefing, and once the run is over, or left from
/// the pause menu, it steps aside, back to the launcher.
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
        "Opening the Harvest Feast challenge.".to_string()
    }
}
