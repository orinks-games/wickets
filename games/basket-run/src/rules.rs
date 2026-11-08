//! The rules of Basket Run, with no sound or speech: the lanes, the things
//! falling into them, the basket, the cart and the clock. The run screen
//! turns the [`Event`]s these rules raise into sound and speech, and tests
//! drive the rules directly with a seeded [`Rng`].

use parlor_core::status::sentences;
use parlor_core::{Period, Rng};

/// Rounds in the meadow.
pub const ROUNDS: u32 = 8;
/// Baskets (lives) a run starts with.
pub const LIVES: u32 = 3;
/// The shortest warning a falling thing ever gives, in seconds: enough to
/// hear where it is and cross the whole meadow, whatever the round.
pub const MIN_FALL: f64 = 1.6;
/// Seconds between "Ready" and the first drop of a round.
pub const LEAD_IN: f64 = 4.0;
/// In endless, daily and practice runs, every this many cracked eggs costs a
/// basket (in practice it costs nothing).
pub const CRACKS_PER_BASKET: u32 = 5;
/// An egg's base value.
pub const EGG_POINTS: u64 = 10;
/// A golden egg's base value.
pub const GOLDEN_POINTS: u64 = 50;
/// Bonus for banking a full basket.
pub const FULL_BANK_BONUS: u64 = 25;
/// Bonus per round number for clearing a round.
pub const ROUND_BONUS: u64 = 50;
/// Bonus for a round with no cracked eggs.
pub const PERFECT_BONUS: u64 = 100;
/// The longest streak that still raises an egg's value.
pub const STREAK_CAP: u32 = 10;
/// Seconds left when the round warns that time is short.
pub const WARNING_AT: f64 = 10.0;
/// Rounds in the weekly marathon.
pub const WEEKLY_ROUNDS: u32 = 5;
/// The meadow round the weekly marathon starts from.
pub const WEEKLY_FROM: u32 = 4;
/// Rounds of the monthly Easter festival.
pub const FESTIVAL_ROUNDS: u32 = 3;
/// Seconds on the daily challenge's clock.
pub const DAILY_SECS: f64 = 60.0;
/// Bonus for a festival pattern completed cleanly.
pub const PATTERN_BONUS: u64 = 100;
/// Seconds between the eggs of an egg sweep.
pub const SWEEP_STEP: f64 = 0.6;
/// Seconds between the eggs of a nest.
pub const NEST_STEP: f64 = 0.55;
/// Seconds after a nest's last egg that its mud clod lands.
pub const NEST_MUD_AFTER: f64 = 0.65;
/// Rounds of the Easter morning hunt.
pub const EASTER_ROUNDS: u32 = 3;

/// What falls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Egg,
    Golden,
    Mud,
    Bee,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Egg, Kind::Golden, Kind::Mud, Kind::Bee];

    /// "egg", "golden egg", "mud clod", "bee".
    pub fn name(self) -> &'static str {
        match self {
            Kind::Egg => "egg",
            Kind::Golden => "golden egg",
            Kind::Mud => "mud clod",
            Kind::Bee => "bee",
        }
    }

    /// "an egg", "a golden egg", "a mud clod", "a bee".
    pub fn with_article(self) -> &'static str {
        match self {
            Kind::Egg => "an egg",
            Kind::Golden => "a golden egg",
            Kind::Mud => "a mud clod",
            Kind::Bee => "a bee",
        }
    }

    /// The saved id for "already introduced".
    pub fn id(self) -> &'static str {
        match self {
            Kind::Egg => "egg",
            Kind::Golden => "golden",
            Kind::Mud => "mud",
            Kind::Bee => "bee",
        }
    }

    /// Whether it goes in the basket.
    pub fn is_egg(self) -> bool {
        matches!(self, Kind::Egg | Kind::Golden)
    }

    fn index(self) -> usize {
        match self {
            Kind::Egg => 0,
            Kind::Golden => 1,
            Kind::Mud => 2,
            Kind::Bee => 3,
        }
    }

    pub fn points(self) -> u64 {
        match self {
            Kind::Golden => GOLDEN_POINTS,
            _ => EGG_POINTS,
        }
    }
}

/// Which run this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Eight rounds; each needs a number of eggs banked.
    Meadow,
    /// Opens once the meadow is cleared: rounds without end, until the
    /// baskets run out.
    Endless,
    /// The daily challenge: one round with the day's twist, the same for
    /// everyone today.
    Daily,
    /// Slow, nothing to lose, every cue introduced and called.
    Practice,
    /// The weekly challenge: a marathon of five rounds from the meadow's
    /// fourth, each with an egg target.
    Weekly,
    /// The monthly challenge: the Easter festival, rounds of drop patterns.
    Festival,
    /// Easter morning, on Easter itself: a golden-egg hunt of three
    /// gentle rounds, golden eggs worth double.
    Easter,
}

impl Mode {
    /// The best-score board, for modes that keep one.
    pub fn board(self) -> Option<&'static str> {
        match self {
            Mode::Meadow => Some("meadow"),
            Mode::Endless => Some("endless"),
            Mode::Easter => Some("easter morning"),
            Mode::Daily | Mode::Practice | Mode::Weekly | Mode::Festival => None,
        }
    }

    /// Whether cracked eggs wear baskets out.
    pub fn cracks_cost_baskets(self) -> bool {
        matches!(self, Mode::Endless | Mode::Daily | Mode::Festival)
    }

    /// How many rounds the run has, when it has an end.
    pub fn rounds(self) -> Option<u32> {
        match self {
            Mode::Meadow => Some(ROUNDS),
            Mode::Daily => Some(1),
            Mode::Weekly => Some(WEEKLY_ROUNDS),
            Mode::Festival => Some(FESTIVAL_ROUNDS),
            Mode::Easter => Some(EASTER_ROUNDS),
            Mode::Endless | Mode::Practice => None,
        }
    }

    /// The challenge period this mode plays, if it is a challenge.
    pub fn period(self) -> Option<Period> {
        match self {
            Mode::Daily => Some(Period::Daily),
            Mode::Weekly => Some(Period::Weekly),
            Mode::Festival => Some(Period::Monthly),
            _ => None,
        }
    }

    /// The challenge mode for a period.
    pub fn for_period(period: Period) -> Mode {
        match period {
            Period::Daily => Mode::Daily,
            Period::Weekly => Mode::Weekly,
            Period::Monthly => Mode::Festival,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Meadow => "The meadow",
            Mode::Endless => "Endless meadow",
            Mode::Daily => "Daily challenge",
            Mode::Practice => "Practice lanes",
            Mode::Weekly => "Weekly marathon",
            Mode::Festival => "Easter festival",
            Mode::Easter => "Easter morning",
        }
    }
}

/// The daily challenge's twist on the meadow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Twist {
    GoldenRush,
    BeeSwarm,
    FastFall,
    MudSlide,
    SmallBasket,
}

impl Twist {
    pub const ALL: [Twist; 5] = [
        Twist::GoldenRush,
        Twist::BeeSwarm,
        Twist::FastFall,
        Twist::MudSlide,
        Twist::SmallBasket,
    ];

    /// The day's twist, drawn first from the challenge's random source.
    pub fn pick(rng: &mut Rng) -> Twist {
        Twist::ALL[rng.below(Twist::ALL.len() as u32) as usize]
    }

    /// The twist a run seeded with `seed` gets.
    pub fn for_seed(seed: u64) -> Twist {
        Twist::pick(&mut Rng::seeded(seed))
    }

    /// "golden rush", "bee swarm"...
    pub fn name(self) -> &'static str {
        match self {
            Twist::GoldenRush => "golden rush",
            Twist::BeeSwarm => "bee swarm",
            Twist::FastFall => "fast fall",
            Twist::MudSlide => "mud slide",
            Twist::SmallBasket => "small basket",
        }
    }

    /// What it changes, as a sentence.
    pub fn describe(self) -> &'static str {
        match self {
            Twist::GoldenRush => "golden eggs fall three times as often, among plenty of mud and bees.",
            Twist::BeeSwarm => "no mud, twice as many bees, and more of them dive right behind an egg.",
            Twist::FastFall => "everything falls as fast as the meadow ever allows, and the drops come closer together.",
            Twist::MudSlide => "no bees, but mud clods fall three times as often.",
            Twist::SmallBasket => "the basket holds only 3 eggs, so you run to the cart far more often.",
        }
    }

    /// The round with the twist applied.
    pub fn apply(self, spec: RoundSpec) -> RoundSpec {
        match self {
            Twist::GoldenRush => RoundSpec {
                golden: 0.25,
                ..spec
            },
            Twist::BeeSwarm => RoundSpec {
                mud: 0.0,
                bee: (spec.bee * 2.0).min(0.3),
                chasers: (spec.chasers * 2.0).min(0.3),
                ..spec
            },
            Twist::FastFall => RoundSpec {
                fall: MIN_FALL,
                gap: (spec.gap * 0.85).max(1.2),
                ..spec
            },
            Twist::MudSlide => RoundSpec {
                bee: 0.0,
                chasers: 0.0,
                mud: (spec.mud * 3.0).min(0.45),
                ..spec
            },
            Twist::SmallBasket => RoundSpec {
                capacity: 3,
                ..spec
            },
        }
    }
}

/// A festival pattern: several drops that belong together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// An egg in every lane in turn, from one edge to the other.
    Sweep,
    /// A bee in every lane but one, all at once; the gap holds a golden egg.
    Fence,
    /// Three eggs in one lane, then a mud clod right behind them.
    Nest,
}

impl Pattern {
    pub const ALL: [Pattern; 3] = [Pattern::Sweep, Pattern::Fence, Pattern::Nest];

    /// "egg sweep", "bee fence", "nest".
    pub fn name(self) -> &'static str {
        match self {
            Pattern::Sweep => "egg sweep",
            Pattern::Fence => "bee fence",
            Pattern::Nest => "nest",
        }
    }

    /// Seconds from its first drop starting to its last.
    pub fn span(self, lanes: usize) -> f64 {
        match self {
            Pattern::Sweep => SWEEP_STEP * (lanes.max(1) - 1) as f64,
            Pattern::Fence => 0.0,
            Pattern::Nest => 2.0 * NEST_STEP + NEST_MUD_AFTER,
        }
    }
}

/// One festival pattern in a round, and how it is going.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Group {
    pub id: u32,
    pub pattern: Pattern,
    /// A sweep's first lane, a fence's gap, a nest's lane.
    pub lane: usize,
    /// Drops still to land.
    pub to_land: u32,
    pub announced: bool,
    /// An egg of it cracked, or one of its hazards hit.
    pub failed: bool,
}

/// How one round plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundSpec {
    pub number: u32,
    pub lanes: usize,
    /// Seconds on the clock.
    pub duration: f64,
    /// Average seconds between drops.
    pub gap: f64,
    /// Seconds from a drop's first sound to its landing.
    pub fall: f64,
    pub capacity: usize,
    /// Share of the round's eggs to bank, 0 for none.
    pub quota: f64,
    /// Chances of each kind of drop (the rest are eggs).
    pub golden: f64,
    pub mud: f64,
    pub bee: f64,
    /// Chance an egg brings a second egg close behind in a nearby lane.
    pub doubles: f64,
    /// Chance an egg brings a bee right behind it in the same lane.
    pub chasers: f64,
}

/// The meadow's rounds, 1 to 8.
pub fn meadow_round(n: u32) -> RoundSpec {
    let n = n.clamp(1, ROUNDS);
    // number, lanes, secs, gap, fall, cap, quota, golden, mud, bee, doubles, chasers
    let row: (usize, f64, f64, f64, usize, f64, f64, f64, f64, f64, f64) = match n {
        1 => (3, 45.0, 2.6, 3.2, 4, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0),
        2 => (3, 50.0, 2.4, 3.0, 4, 0.55, 0.0, 0.18, 0.0, 0.0, 0.0),
        3 => (4, 55.0, 2.2, 2.8, 5, 0.55, 0.07, 0.15, 0.12, 0.0, 0.0),
        4 => (4, 60.0, 2.0, 2.6, 5, 0.6, 0.07, 0.15, 0.13, 0.15, 0.0),
        5 => (5, 60.0, 1.9, 2.4, 5, 0.6, 0.08, 0.16, 0.14, 0.15, 0.12),
        6 => (5, 65.0, 1.8, 2.25, 5, 0.6, 0.08, 0.17, 0.15, 0.18, 0.14),
        7 => (5, 70.0, 1.7, 2.1, 6, 0.65, 0.09, 0.18, 0.16, 0.2, 0.15),
        _ => (5, 75.0, 1.6, 2.0, 6, 0.65, 0.1, 0.18, 0.18, 0.22, 0.17),
    };
    RoundSpec {
        number: n,
        lanes: row.0,
        duration: row.1,
        gap: row.2,
        fall: row.3,
        capacity: row.4,
        quota: row.5,
        golden: row.6,
        mud: row.7,
        bee: row.8,
        doubles: row.9,
        chasers: row.10,
    }
}

/// "Sunrise in the garden", the name of Easter morning's round `n`.
pub fn easter_round_name(n: u32) -> &'static str {
    match n.clamp(1, EASTER_ROUNDS) {
        1 => "sunrise in the garden",
        2 => "the orchard",
        _ => "the big hunt",
    }
}

/// Easter morning's round `n`: from three lanes to five, golden eggs in
/// plenty, no bees, and only a little mud from the orchard on.
pub fn easter_round(n: u32) -> RoundSpec {
    let n = n.clamp(1, EASTER_ROUNDS);
    let step = (n - 1) as f64;
    RoundSpec {
        number: n,
        lanes: 2 + n as usize,
        duration: 50.0 + 5.0 * step,
        gap: 2.3 - 0.2 * step,
        fall: 3.0 - 0.2 * step,
        capacity: 5,
        quota: 0.0,
        golden: 0.3,
        mud: if n == 1 { 0.0 } else { 0.06 * step },
        bee: 0.0,
        doubles: 0.1 * step,
        chasers: 0.0,
    }
}

/// A round of the given mode.
pub fn round_spec(mode: Mode, n: u32) -> RoundSpec {
    match mode {
        Mode::Meadow => meadow_round(n),
        Mode::Endless => {
            // Starts where round 4 does and keeps tightening, but never
            // past what a listener can answer.
            let base = meadow_round((n + 3).min(ROUNDS));
            let past = n.saturating_sub(ROUNDS - 3) as f64;
            RoundSpec {
                number: n,
                duration: 60.0,
                gap: (base.gap - 0.05 * past).max(1.0),
                fall: (base.fall - 0.05 * past).max(MIN_FALL),
                quota: 0.0,
                mud: (base.mud + 0.01 * past).min(0.25),
                bee: (base.bee + 0.01 * past).min(0.25),
                ..base
            }
        }
        Mode::Daily => RoundSpec {
            number: 1,
            duration: DAILY_SECS,
            quota: 0.0,
            ..meadow_round(5)
        },
        Mode::Weekly => RoundSpec {
            number: n,
            ..meadow_round((n + WEEKLY_FROM - 1).min(ROUNDS))
        },
        Mode::Festival => {
            let step = n.clamp(1, FESTIVAL_ROUNDS).saturating_sub(1) as f64;
            RoundSpec {
                number: n,
                lanes: 5,
                duration: 60.0,
                gap: 2.2 - 0.2 * step,
                fall: (2.6 - 0.2 * step).max(MIN_FALL),
                capacity: 6,
                quota: 0.0,
                golden: 0.06,
                mud: 0.14,
                bee: 0.0,
                doubles: 0.0,
                chasers: 0.0,
            }
        }
        Mode::Easter => easter_round(n),
        Mode::Practice => RoundSpec {
            number: n,
            lanes: 3,
            duration: 60.0,
            gap: 2.6,
            fall: 3.2,
            capacity: 4,
            quota: 0.0,
            golden: 0.1,
            mud: 0.15,
            bee: 0.15,
            doubles: 0.0,
            chasers: 0.0,
        },
    }
}

/// A drop still to come: `at` seconds into the round it starts falling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Planned {
    pub at: f64,
    pub kind: Kind,
    pub lane: usize,
    /// The festival pattern it belongs to, 0 for none.
    pub group: u32,
}

/// Something in the air.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Falling {
    pub id: u32,
    pub kind: Kind,
    pub lane: usize,
    pub total: f64,
    pub left: f64,
    /// The festival pattern it belongs to, 0 for none.
    pub group: u32,
}

impl Falling {
    /// 0 as it starts to fall, 1 as it lands.
    pub fn progress(&self) -> f64 {
        if self.total <= 0.0 {
            1.0
        } else {
            (1.0 - self.left / self.total).clamp(0.0, 1.0)
        }
    }
}

/// Builds a round's drops, in order of when they start to fall. Every drop
/// lands before the clock runs out.
pub fn plan_round(spec: &RoundSpec, rng: &mut Rng) -> Vec<Planned> {
    let mut plan = Vec::new();
    let lanes = spec.lanes.max(1);
    let last_start = spec.duration - spec.fall - 0.5;
    let mut t = 0.5;
    let mut prev_lane: Option<usize> = None;
    while t <= last_start {
        let roll = rng.unit();
        let kind = if roll < spec.golden {
            Kind::Golden
        } else if roll < spec.golden + spec.mud {
            Kind::Mud
        } else if roll < spec.golden + spec.mud + spec.bee {
            Kind::Bee
        } else {
            Kind::Egg
        };
        let mut lane = rng.below(lanes as u32) as usize;
        // A hazard never lands right where the last egg did straight after
        // it, unless it is a chaser the round announces as a pattern.
        if !kind.is_egg() && prev_lane == Some(lane) && lanes > 1 {
            lane = (lane + 1 + rng.below(lanes as u32 - 1) as usize) % lanes;
        }
        plan.push(Planned {
            at: t,
            kind,
            lane,
            group: 0,
        });
        prev_lane = Some(lane);
        let mut extra = 0.0;
        if kind == Kind::Egg && t + 0.7 <= last_start {
            if rng.chance(spec.doubles) && lanes > 1 {
                let step = 1 + rng.below(2.min(lanes as u32 - 1)) as usize;
                let other = if lane >= step && (lane + step >= lanes || rng.chance(0.5)) {
                    lane - step
                } else {
                    (lane + step).min(lanes - 1)
                };
                if other != lane {
                    plan.push(Planned {
                        at: t + 0.6 + 0.3 * step as f64,
                        kind: Kind::Egg,
                        lane: other,
                        group: 0,
                    });
                    prev_lane = Some(other);
                    extra = 0.6 + 0.3 * step as f64;
                }
            } else if rng.chance(spec.chasers) {
                plan.push(Planned {
                    at: t + 0.7,
                    kind: Kind::Bee,
                    lane,
                    group: 0,
                });
                extra = 0.7;
            }
        }
        t += extra + spec.gap * (0.8 + 0.4 * rng.unit());
    }
    plan.sort_by(|a, b| a.at.total_cmp(&b.at));
    plan
}

/// Builds a festival round: the three patterns in a shuffled turn, with
/// single eggs and mud between them. Group ids start at `next_group`, which
/// is moved past the ones used. Every drop lands before the clock runs out.
pub fn plan_festival(
    spec: &RoundSpec,
    rng: &mut Rng,
    next_group: &mut u32,
) -> (Vec<Planned>, Vec<Group>) {
    let mut plan = Vec::new();
    let mut groups = Vec::new();
    let lanes = spec.lanes.max(3);
    let last_start = spec.duration - spec.fall - 0.5;
    let mut order = Pattern::ALL;
    rng.shuffle(&mut order);
    let mut turn = 0usize;
    let mut t = 0.5;
    let mut prev_lane: Option<usize> = None;
    while t <= last_start {
        let pattern = order[turn % order.len()];
        let room = last_start - t;
        if rng.chance(0.6) && pattern.span(lanes) <= room {
            turn += 1;
            let id = *next_group;
            *next_group += 1;
            let mut push = |at: f64, kind: Kind, lane: usize| {
                plan.push(Planned {
                    at,
                    kind,
                    lane,
                    group: id,
                })
            };
            let (lane, count) = match pattern {
                Pattern::Sweep => {
                    let rightward = rng.chance(0.5);
                    for i in 0..lanes {
                        let lane = if rightward { i } else { lanes - 1 - i };
                        push(t + SWEEP_STEP * i as f64, Kind::Egg, lane);
                    }
                    (if rightward { 0 } else { lanes - 1 }, lanes as u32)
                }
                Pattern::Fence => {
                    let gap = rng.below(lanes as u32) as usize;
                    for lane in 0..lanes {
                        let kind = if lane == gap { Kind::Golden } else { Kind::Bee };
                        push(t, kind, lane);
                    }
                    (gap, lanes as u32)
                }
                Pattern::Nest => {
                    let lane = rng.below(lanes as u32) as usize;
                    for i in 0..3 {
                        push(t + NEST_STEP * i as f64, Kind::Egg, lane);
                    }
                    push(t + pattern.span(lanes), Kind::Mud, lane);
                    (lane, 4)
                }
            };
            groups.push(Group {
                id,
                pattern,
                lane,
                to_land: count,
                announced: false,
                failed: false,
            });
            prev_lane = None;
            t += pattern.span(lanes) + spec.gap * (1.4 + 0.4 * rng.unit());
            continue;
        }
        let roll = rng.unit();
        let kind = if roll < spec.golden {
            Kind::Golden
        } else if roll < spec.golden + spec.mud {
            Kind::Mud
        } else {
            Kind::Egg
        };
        let mut lane = rng.below(lanes as u32) as usize;
        if !kind.is_egg() && prev_lane == Some(lane) {
            lane = (lane + 1 + rng.below(lanes as u32 - 1) as usize) % lanes;
        }
        plan.push(Planned {
            at: t,
            kind,
            lane,
            group: 0,
        });
        prev_lane = Some(lane);
        t += spec.gap * (0.8 + 0.4 * rng.unit());
    }
    plan.sort_by(|a, b| a.at.total_cmp(&b.at));
    (plan, groups)
}

/// Where the run stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// "Ready": seconds until the first drop.
    LeadIn(f64),
    Playing,
    /// A round is over and cleared; the next starts with
    /// [`Run::next_round`].
    Between,
    Over {
        won: bool,
    },
}

/// What happened, for the screen to sound and say.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The first drops are about to fall.
    Go,
    /// Met for the first time in this run: explain it before it falls.
    Introduce(Kind),
    Spawn {
        id: u32,
        kind: Kind,
        lane: usize,
    },
    /// An egg in the basket, worth `value`; the basket now holds `held`.
    Caught {
        kind: Kind,
        lane: usize,
        value: u64,
        held: usize,
    },
    /// The basket has just filled.
    BasketFull,
    /// An egg landed in a full basket and bounced out.
    Overflow {
        kind: Kind,
        lane: usize,
    },
    /// An egg hit the grass.
    Cracked {
        kind: Kind,
        lane: usize,
    },
    /// A streak of `n` broke.
    StreakBroken(u32),
    /// A streak reached a milestone.
    Streak(u32),
    /// Mud landed in the basket and spoiled what was in it.
    Splat {
        lane: usize,
        spoiled: usize,
    },
    /// Mud landed in the grass.
    MudMissed {
        lane: usize,
    },
    /// A bee stung; `lives` baskets are left.
    Stung {
        lane: usize,
        lives: u32,
    },
    BeeMissed {
        lane: usize,
    },
    /// Too many cracked eggs wore a basket out.
    CrackedBasket {
        lives: u32,
    },
    /// Time is short.
    Warning,
    /// A second of the last five.
    Tick(u32),
    /// The clock ran out; `lost` eggs were still in the basket.
    TimeUp {
        lost: usize,
    },
    RoundCleared {
        round: u32,
        banked: u32,
        bonus: u64,
        perfect: bool,
    },
    /// Not enough eggs banked: the run is over.
    RoundFailed {
        banked: u32,
        needed: u32,
    },
    OutOfBaskets,
    /// Every round of the meadow cleared.
    Won,
    /// A daily run's single round is done.
    Finished,
    /// A festival pattern starts to fall; `lane` is a sweep's first lane, a
    /// fence's gap or a nest's lane.
    Pattern {
        pattern: Pattern,
        lane: usize,
    },
    /// A festival pattern landed cleanly and earned `bonus`.
    PatternDone {
        pattern: Pattern,
        bonus: u64,
    },
    /// A festival pattern landed with a miss or a hit.
    PatternBroken {
        pattern: Pattern,
    },
}

/// What a step to the side did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Moved {
        lane: usize,
        at_cart: bool,
    },
    Edge,
    /// Not while the round is not running.
    Idle,
}

/// What pressing bank did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bank {
    Banked {
        eggs: usize,
        points: u64,
        full: bool,
    },
    Empty,
    /// The cart is `lanes` lanes away, to the left when `left`.
    NotAtCart {
        lanes: usize,
        left: bool,
    },
    Idle,
}

/// One run of rounds.
pub struct Run {
    pub mode: Mode,
    pub rng: Rng,
    pub spec: RoundSpec,
    pub round: u32,
    pub phase: Phase,
    /// The player's lane, 0 at the left.
    pub lane: usize,
    /// Each egg's value, in the order caught.
    pub basket: Vec<u64>,
    pub score: u64,
    pub lives: u32,
    pub streak: u32,
    pub best_streak: u32,
    /// Seconds since the round's first drop could fall.
    pub clock: f64,
    /// Drops still to come, in order.
    pub schedule: Vec<Planned>,
    pub falling: Vec<Falling>,
    /// Eggs this round will drop, golden ones included.
    pub eggs_planned: u32,
    pub banked_round: u32,
    pub banked_total: u32,
    pub caught_total: u32,
    /// Golden eggs caught this run.
    pub golden_caught: u32,
    pub cracks_round: u32,
    pub cracks_total: u32,
    cracks_toward_basket: u32,
    pub rounds_cleared: u32,
    /// The daily challenge's twist.
    pub twist: Option<Twist>,
    /// This round's festival patterns.
    pub groups: Vec<Group>,
    /// Festival patterns completed cleanly.
    pub patterns_done: u32,
    next_group: u32,
    /// Raise [`Event::Introduce`] before a kind's first drop.
    pub introduce: bool,
    introduced: [bool; 4],
    /// Callouts are off: banked points earn a tenth more.
    pub by_ear: bool,
    events: Vec<Event>,
    next_id: u32,
    warned: bool,
    ticked: u32,
}

impl Run {
    /// A run at its first round, in the lead-in. A daily run draws its
    /// twist first from `rng`, so the seed decides both.
    pub fn new(mode: Mode, mut rng: Rng) -> Run {
        let twist = (mode == Mode::Daily).then(|| Twist::pick(&mut rng));
        Run::with_twist(mode, rng, twist)
    }

    /// A run with the given twist (or none).
    pub fn with_twist(mode: Mode, rng: Rng, twist: Option<Twist>) -> Run {
        let spec = round_spec(mode, 1);
        let mut run = Run {
            mode,
            rng,
            spec,
            round: 1,
            phase: Phase::LeadIn(LEAD_IN),
            lane: 0,
            basket: Vec::new(),
            score: 0,
            lives: LIVES,
            streak: 0,
            best_streak: 0,
            clock: 0.0,
            schedule: Vec::new(),
            falling: Vec::new(),
            eggs_planned: 0,
            banked_round: 0,
            banked_total: 0,
            caught_total: 0,
            golden_caught: 0,
            cracks_round: 0,
            cracks_total: 0,
            cracks_toward_basket: 0,
            rounds_cleared: 0,
            twist,
            groups: Vec::new(),
            patterns_done: 0,
            next_group: 1,
            introduce: false,
            introduced: [false; 4],
            by_ear: false,
            events: Vec::new(),
            next_id: 1,
            warned: false,
            ticked: 0,
        };
        run.setup_round();
        run
    }

    /// A run that starts at round `n` (for tests, and the meadow's later
    /// rounds).
    pub fn at_round(mode: Mode, rng: Rng, n: u32) -> Run {
        let mut run = Run::new(mode, rng);
        run.round = n.max(1);
        run.setup_round();
        run
    }

    /// The round's spec, with the twist applied.
    pub fn spec_for(&self, n: u32) -> RoundSpec {
        let spec = round_spec(self.mode, n);
        match self.twist {
            Some(t) => t.apply(spec),
            None => spec,
        }
    }

    fn setup_round(&mut self) {
        self.spec = self.spec_for(self.round);
        if self.mode == Mode::Festival {
            let (plan, groups) = plan_festival(&self.spec, &mut self.rng, &mut self.next_group);
            self.schedule = plan;
            self.groups = groups;
        } else {
            self.schedule = plan_round(&self.spec, &mut self.rng);
            self.groups.clear();
        }
        self.eggs_planned = self.schedule.iter().filter(|p| p.kind.is_egg()).count() as u32;
        self.falling.clear();
        self.basket.clear();
        self.clock = 0.0;
        self.banked_round = 0;
        self.cracks_round = 0;
        self.warned = false;
        self.ticked = 0;
        self.lane = self.cart_lane();
        self.phase = Phase::LeadIn(LEAD_IN);
    }

    /// Marks kinds the player has already been told about.
    pub fn mark_introduced(&mut self, kind: Kind) {
        self.introduced[kind.index()] = true;
    }

    pub fn was_introduced(&self, kind: Kind) -> bool {
        self.introduced[kind.index()]
    }

    /// The cart's lane: the left edge in odd rounds, the right in even ones.
    pub fn cart_lane(&self) -> usize {
        if matches!(self.mode, Mode::Meadow | Mode::Weekly) && self.round.is_multiple_of(2) {
            self.spec.lanes - 1
        } else {
            0
        }
    }

    pub fn lanes(&self) -> usize {
        self.spec.lanes
    }

    pub fn capacity(&self) -> usize {
        self.spec.capacity
    }

    pub fn at_cart(&self) -> bool {
        self.lane == self.cart_lane()
    }

    pub fn basket_full(&self) -> bool {
        self.basket.len() >= self.spec.capacity
    }

    /// Eggs to bank this round to clear it, 0 when the mode has no quota.
    pub fn needed(&self) -> u32 {
        if self.spec.quota <= 0.0 {
            0
        } else {
            ((self.eggs_planned as f64 * self.spec.quota).ceil() as u32).max(1)
        }
    }

    /// Seconds left on the clock (the lead-in does not count down it).
    pub fn time_left(&self) -> f64 {
        (self.spec.duration - self.clock).max(0.0)
    }

    pub fn is_running(&self) -> bool {
        matches!(self.phase, Phase::LeadIn(_) | Phase::Playing)
    }

    pub fn over(&self) -> Option<bool> {
        match self.phase {
            Phase::Over { won } => Some(won),
            _ => None,
        }
    }

    /// The events raised since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Starts something falling now (tests, and the practice room).
    pub fn drop_now(&mut self, kind: Kind, lane: usize) -> u32 {
        let fall = self.spec.fall;
        self.spawn(kind, lane.min(self.spec.lanes - 1), fall, 0)
    }

    /// Starts one of this round's festival patterns falling now, whatever
    /// its plan said (tests). Returns false when there is no such pattern.
    pub fn drop_pattern_now(&mut self, pattern: Pattern) -> bool {
        let Some(id) = self
            .groups
            .iter()
            .find(|g| g.pattern == pattern && !g.announced)
            .map(|g| g.id)
        else {
            return false;
        };
        let drops: Vec<Planned> = self
            .schedule
            .iter()
            .filter(|p| p.group == id)
            .copied()
            .collect();
        self.schedule.retain(|p| p.group != id);
        let start = drops.first().map_or(0.0, |p| p.at);
        for p in drops {
            self.schedule.push(Planned {
                at: self.clock + (p.at - start),
                ..p
            });
        }
        self.schedule.sort_by(|a, b| a.at.total_cmp(&b.at));
        true
    }

    /// The festival pattern a drop belongs to.
    pub fn group(&self, id: u32) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    fn spawn(&mut self, kind: Kind, lane: usize, fall: f64, group: u32) -> u32 {
        if self.introduce && !self.introduced[kind.index()] {
            self.introduced[kind.index()] = true;
            self.events.push(Event::Introduce(kind));
        }
        let id = self.next_id;
        self.next_id += 1;
        self.falling.push(Falling {
            id,
            kind,
            lane,
            total: fall,
            left: fall,
            group,
        });
        self.events.push(Event::Spawn { id, kind, lane });
        id
    }

    /// One step to the left (`-1`) or right (`1`).
    pub fn step(&mut self, delta: i32) -> Move {
        if !self.is_running() {
            return Move::Idle;
        }
        let to = self.lane as i32 + delta.signum();
        if to < 0 || to >= self.spec.lanes as i32 {
            return Move::Edge;
        }
        self.lane = to as usize;
        Move::Moved {
            lane: self.lane,
            at_cart: self.at_cart(),
        }
    }

    /// Empties the basket into the cart, when standing in its lane.
    pub fn bank(&mut self) -> Bank {
        if !self.is_running() {
            return Bank::Idle;
        }
        if !self.at_cart() {
            let cart = self.cart_lane();
            return Bank::NotAtCart {
                lanes: self.lane.abs_diff(cart),
                left: cart < self.lane,
            };
        }
        if self.basket.is_empty() {
            return Bank::Empty;
        }
        let eggs = self.basket.len();
        let full = eggs >= self.spec.capacity;
        let mut points: u64 = self.basket.iter().sum();
        if full {
            points += FULL_BANK_BONUS;
        }
        if self.by_ear {
            points += points / 10;
        }
        self.basket.clear();
        self.score += points;
        self.banked_round += eggs as u32;
        self.banked_total += eggs as u32;
        Bank::Banked { eggs, points, full }
    }

    /// What an egg caught now is worth: its base, plus a tenth for each
    /// egg of the streak before it, up to double.
    pub fn value_of(&self, kind: Kind) -> u64 {
        let mut base = kind.points();
        if self.mode == Mode::Easter && kind == Kind::Golden {
            base *= 2;
        }
        base + base * self.streak.min(STREAK_CAP) as u64 / 10
    }

    /// Ends the lead-in at once, for a player who has heard enough of the
    /// round's announcement. True when it did (the screen sounds the start
    /// itself, so no [`Event::Go`] is raised).
    pub fn start_now(&mut self) -> bool {
        if matches!(self.phase, Phase::LeadIn(_)) {
            self.phase = Phase::Playing;
            true
        } else {
            false
        }
    }

    /// Advances the run by `dt` seconds.
    pub fn update(&mut self, dt: f64) {
        match self.phase {
            Phase::LeadIn(t) => {
                let t = t - dt;
                if t <= 0.0 {
                    self.phase = Phase::Playing;
                    self.events.push(Event::Go);
                } else {
                    self.phase = Phase::LeadIn(t);
                }
            }
            Phase::Playing => self.play(dt),
            Phase::Between | Phase::Over { .. } => {}
        }
    }

    fn play(&mut self, dt: f64) {
        self.clock += dt;
        while self.schedule.first().is_some_and(|p| p.at <= self.clock) {
            let p = self.schedule.remove(0);
            // Late by part of a frame: fall that much less, never below
            // the floor.
            let fall = (self.spec.fall - (self.clock - p.at)).max(MIN_FALL.min(self.spec.fall));
            if let Some(g) = self.groups.iter_mut().find(|g| g.id == p.group) {
                if !g.announced {
                    g.announced = true;
                    let (pattern, lane) = (g.pattern, g.lane);
                    self.events.push(Event::Pattern { pattern, lane });
                }
            }
            self.spawn(p.kind, p.lane, fall, p.group);
        }
        for f in &mut self.falling {
            f.left -= dt;
        }
        let mut landed: Vec<Falling> = self
            .falling
            .iter()
            .filter(|f| f.left <= 0.0)
            .copied()
            .collect();
        landed.sort_by(|a, b| a.left.total_cmp(&b.left));
        self.falling.retain(|f| f.left > 0.0);
        for f in landed {
            self.land(f);
            if self.over().is_some() {
                return;
            }
        }
        let left = self.time_left();
        if !self.warned && left <= WARNING_AT {
            self.warned = true;
            self.events.push(Event::Warning);
        }
        if left <= 5.0 && left > 0.0 {
            let whole = left.ceil() as u32;
            if self.ticked == 0 || whole < self.ticked {
                self.ticked = whole;
                self.events.push(Event::Tick(whole));
            }
        }
        if self.clock >= self.spec.duration && self.falling.is_empty() && self.schedule.is_empty() {
            self.end_round();
        }
    }

    fn break_streak(&mut self) {
        if self.streak >= 3 {
            self.events.push(Event::StreakBroken(self.streak));
        }
        self.streak = 0;
    }

    fn crack(&mut self, kind: Kind, lane: usize, overflow: bool) {
        self.cracks_round += 1;
        self.cracks_total += 1;
        if overflow {
            self.events.push(Event::Overflow { kind, lane });
        } else {
            self.events.push(Event::Cracked { kind, lane });
        }
        self.break_streak();
        if self.mode.cracks_cost_baskets() {
            self.cracks_toward_basket += 1;
            if self.cracks_toward_basket >= CRACKS_PER_BASKET {
                self.cracks_toward_basket = 0;
                self.lose_basket();
                self.events.push(Event::CrackedBasket { lives: self.lives });
                self.check_out();
            }
        }
    }

    /// Cracked eggs until the next one costs a basket, when cracks cost.
    pub fn cracks_to_spare(&self) -> Option<u32> {
        self.mode
            .cracks_cost_baskets()
            .then(|| CRACKS_PER_BASKET - self.cracks_toward_basket)
    }

    fn lose_basket(&mut self) {
        if self.mode != Mode::Practice {
            self.lives = self.lives.saturating_sub(1);
        }
    }

    fn check_out(&mut self) {
        if self.lives == 0 && self.over().is_none() {
            self.events.push(Event::OutOfBaskets);
            self.phase = Phase::Over { won: false };
        }
    }

    fn land(&mut self, f: Falling) {
        let here = f.lane == self.lane;
        // Whether it went the player's way: an egg caught, a hazard dodged.
        let good = match f.kind {
            Kind::Egg | Kind::Golden => here && !self.basket_full(),
            Kind::Mud | Kind::Bee => !here,
        };
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == f.group) {
            g.to_land = g.to_land.saturating_sub(1);
            g.failed |= !good;
        }
        match f.kind {
            Kind::Egg | Kind::Golden => {
                if !here {
                    self.crack(f.kind, f.lane, false);
                } else if self.basket_full() {
                    self.crack(f.kind, f.lane, true);
                } else {
                    let value = self.value_of(f.kind);
                    self.basket.push(value);
                    self.streak += 1;
                    self.best_streak = self.best_streak.max(self.streak);
                    self.caught_total += 1;
                    if f.kind == Kind::Golden {
                        self.golden_caught += 1;
                    }
                    self.events.push(Event::Caught {
                        kind: f.kind,
                        lane: f.lane,
                        value,
                        held: self.basket.len(),
                    });
                    if self.streak.is_multiple_of(5) {
                        self.events.push(Event::Streak(self.streak));
                    }
                    if self.basket_full() {
                        self.events.push(Event::BasketFull);
                    }
                }
            }
            Kind::Mud => {
                if here {
                    let spoiled = self.basket.len();
                    self.basket.clear();
                    self.events.push(Event::Splat {
                        lane: f.lane,
                        spoiled,
                    });
                    self.break_streak();
                } else {
                    self.events.push(Event::MudMissed { lane: f.lane });
                }
            }
            Kind::Bee => {
                if here {
                    self.lose_basket();
                    self.events.push(Event::Stung {
                        lane: f.lane,
                        lives: self.lives,
                    });
                    self.break_streak();
                    self.check_out();
                } else {
                    self.events.push(Event::BeeMissed { lane: f.lane });
                }
            }
        }
        if let Some(g) = self.groups.iter().find(|g| g.id == f.group).copied() {
            if g.to_land == 0 && self.over().is_none() {
                if g.failed {
                    self.events
                        .push(Event::PatternBroken { pattern: g.pattern });
                } else {
                    self.score += PATTERN_BONUS;
                    self.patterns_done += 1;
                    self.events.push(Event::PatternDone {
                        pattern: g.pattern,
                        bonus: PATTERN_BONUS,
                    });
                }
            }
        }
    }

    fn end_round(&mut self) {
        let lost = self.basket.len();
        self.basket.clear();
        self.events.push(Event::TimeUp { lost });
        let needed = self.needed();
        if self.banked_round < needed {
            self.events.push(Event::RoundFailed {
                banked: self.banked_round,
                needed,
            });
            self.phase = Phase::Over { won: false };
            return;
        }
        let perfect = self.cracks_round == 0 && self.banked_round > 0;
        let mut bonus = ROUND_BONUS * self.round as u64;
        if perfect {
            bonus += PERFECT_BONUS;
        }
        if self.mode == Mode::Practice {
            bonus = 0;
        }
        self.score += bonus;
        self.rounds_cleared += 1;
        self.events.push(Event::RoundCleared {
            round: self.round,
            banked: self.banked_round,
            bonus,
            perfect,
        });
        match self.mode {
            Mode::Daily => {
                self.events.push(Event::Finished);
                self.phase = Phase::Over { won: true };
            }
            m if m.rounds().is_some_and(|r| self.round >= r) => {
                self.events.push(Event::Won);
                self.phase = Phase::Over { won: true };
            }
            _ => self.phase = Phase::Between,
        }
    }

    /// Starts the next round after one is cleared.
    pub fn next_round(&mut self) {
        if self.phase != Phase::Between {
            return;
        }
        self.round += 1;
        self.setup_round();
    }

    /// The round as a sentence: "Round 3 of 8. 4 lanes, the cart in lane 1
    /// on the left. Bank 9 eggs in 55 seconds. Basket holds 5."
    pub fn round_intro(&self) -> String {
        let round = match self.mode {
            Mode::Meadow => format!("Round {} of {ROUNDS}.", self.round),
            Mode::Endless => format!("Endless, round {}.", self.round),
            Mode::Daily => match self.twist {
                Some(t) => format!("Today's meadow, with the {} twist.", t.name()),
                None => "Today's meadow.".to_string(),
            },
            Mode::Practice => format!("Practice, round {}.", self.round),
            Mode::Weekly => format!("Marathon, round {} of {WEEKLY_ROUNDS}.", self.round),
            Mode::Festival => format!("Festival, round {} of {FESTIVAL_ROUNDS}.", self.round),
            Mode::Easter => format!(
                "Easter morning, round {} of {EASTER_ROUNDS}: {}.",
                self.round,
                easter_round_name(self.round)
            ),
        };
        let cart = self.cart_lane();
        let side = if cart == 0 { "left" } else { "right" };
        let goal = match self.needed() {
            0 => format!("{} seconds.", self.spec.duration as u32),
            n => format!(
                "Bank {n} of {} eggs in {} seconds.",
                self.eggs_planned, self.spec.duration as u32
            ),
        };
        format!(
            "{round} {} lanes, the cart in lane {} on the {side}. {goal} The basket holds {}. You are at the cart. Starting.",
            self.spec.lanes,
            cart + 1,
            self.spec.capacity
        )
    }

    /// The next thing to land, if any.
    pub fn next_landing(&self) -> Option<&Falling> {
        self.falling.iter().min_by(|a, b| a.left.total_cmp(&b.left))
    }

    /// "2 lanes right", "this lane", "1 lane left".
    pub fn relative(&self, lane: usize) -> String {
        let d = lane as i32 - self.lane as i32;
        match d {
            0 => "your lane".to_string(),
            1 => "1 lane right".to_string(),
            -1 => "1 lane left".to_string(),
            d if d > 0 => format!("{d} lanes right"),
            d => format!("{} lanes left", -d),
        }
    }

    /// Everything a player could want to know, as speech.
    /// The status key, in layers. First: where you are, how full the
    /// basket is and what lands next. Second: the round's clock and what
    /// it costs. Third: score and the mode's extras.
    pub fn status_layers(&self) -> Vec<String> {
        let mut first = vec![format!(
            "Lane {} of {}{}, basket {} of {}.",
            self.lane + 1,
            self.spec.lanes,
            if self.at_cart() { ", at the cart" } else { "" },
            self.basket.len(),
            self.spec.capacity
        )];
        let mut falling: Vec<&Falling> = self.falling.iter().collect();
        falling.sort_by(|a, b| a.left.total_cmp(&b.left));
        for f in falling.iter().take(2) {
            first.push(format!(
                "{} in lane {}, {}, lands in {:.1} seconds.",
                hearken::text::sentence_start(f.kind.name()),
                f.lane + 1,
                self.relative(f.lane),
                f.left.max(0.0)
            ));
        }
        let mut round = Vec::new();
        match self.phase {
            Phase::LeadIn(_) => round.push("Getting ready.".to_string()),
            Phase::Playing => round.push(format!("{} seconds left.", self.time_left().ceil())),
            _ => {}
        }
        match self.needed() {
            0 => round.push(format!("Banked {} this round.", self.banked_round)),
            n => round.push(format!("Banked {} of {n} needed.", self.banked_round)),
        }
        if self.mode == Mode::Practice {
            round.push("Practice: nothing costs a basket.".to_string());
        } else {
            round.push(baskets_left(self.lives));
        }
        if let Some(n) = self.cracks_to_spare() {
            round.push(match n {
                1 => "1 more cracked egg costs a basket.".to_string(),
                n => format!("{n} more cracked eggs cost a basket."),
            });
        }
        let mut extra = vec![format!("Score {}.", self.score)];
        if self.streak > 0 {
            extra.push(format!("Streak {}.", self.streak));
        }
        if let Some(t) = self.twist {
            extra.push(format!("Twist: {}.", t.name()));
        }
        if self.mode == Mode::Festival {
            extra.push(format!("Patterns caught cleanly: {}.", self.patterns_done));
        }
        vec![sentences(first), sentences(round), sentences(extra)]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        sentences(self.status_layers())
    }

    /// What to do right now.
    pub fn hint(&self) -> String {
        if !self.is_running() {
            return "Nothing is falling right now.".to_string();
        }
        let mut falling: Vec<&Falling> = self.falling.iter().collect();
        falling.sort_by(|a, b| a.left.total_cmp(&b.left));
        // A hazard about to land on you first, unless an egg in your lane
        // lands before it: then catch, and step aside after.
        let first_here = falling.iter().find(|f| f.lane == self.lane);
        let hazard_here = falling
            .iter()
            .find(|f| !f.kind.is_egg() && f.lane == self.lane);
        if let (Some(egg), Some(h)) = (first_here, hazard_here) {
            if egg.kind.is_egg() && !self.basket_full() {
                return format!(
                    "Stay here: {} lands in your lane in {:.1} seconds. Then step aside: {} follows it.",
                    egg.kind.with_article(),
                    egg.left.max(0.0),
                    h.kind.with_article()
                );
            }
        }
        if let Some(f) = hazard_here {
            let lanes = self.spec.lanes;
            let unsafe_lane = |lane: usize| {
                falling
                    .iter()
                    .any(|g| !g.kind.is_egg() && g.lane == lane && (g.left - f.left).abs() < 0.8)
            };
            // The nearest lane clear of hazards, to the right first.
            let mut to = None;
            for d in 1..lanes {
                let right = self.lane + d;
                if right < lanes && !unsafe_lane(right) {
                    to = Some(right);
                    break;
                }
                if let Some(left) = self.lane.checked_sub(d) {
                    if !unsafe_lane(left) {
                        to = Some(left);
                        break;
                    }
                }
            }
            let to = to.unwrap_or(if self.lane + 1 < lanes {
                self.lane + 1
            } else {
                self.lane.saturating_sub(1)
            });
            let way = if to > self.lane { "right" } else { "left" };
            let step = match to.abs_diff(self.lane) {
                1 => format!("Step {way}."),
                n => format!("Step {n} lanes {way}, to lane {}.", to + 1),
            };
            return format!(
                "{} is coming down in your lane in {:.1} seconds. {step}",
                hearken::text::sentence_start(f.kind.with_article()),
                f.left.max(0.0)
            );
        }
        let cart = self.cart_lane();
        let to_cart = if cart < self.lane { "left" } else { "right" };
        if self.basket_full() {
            if self.at_cart() {
                return "Your basket is full and you are at the cart: bank it now.".to_string();
            }
            return format!(
                "Your basket is full. The cart is {} {to_cart}, in lane {}.",
                lanes_phrase(self.lane.abs_diff(cart)),
                cart + 1
            );
        }
        if let Some(f) = falling.iter().find(|f| f.kind.is_egg()) {
            if f.lane == self.lane {
                return format!(
                    "Stay here: {} lands in your lane in {:.1} seconds.",
                    f.kind.with_article(),
                    f.left.max(0.0)
                );
            }
            let way = if f.lane > self.lane { "right" } else { "left" };
            return format!(
                "Move {way}: {} lands in lane {}, {}, in {:.1} seconds.",
                f.kind.with_article(),
                f.lane + 1,
                self.relative(f.lane),
                f.left.max(0.0)
            );
        }
        if !self.basket.is_empty() && !self.at_cart() {
            return format!(
                "Nothing to catch yet. A good moment to bank: the cart is {} {to_cart}.",
                lanes_phrase(self.lane.abs_diff(cart))
            );
        }
        if !self.basket.is_empty() {
            return "You are at the cart: bank your basket.".to_string();
        }
        "Nothing falling yet. Wait in the middle of the meadow and listen.".to_string()
    }
}

/// "1 lane", "3 lanes".
pub fn lanes_phrase(n: usize) -> String {
    if n == 1 {
        "1 lane".to_string()
    } else {
        format!("{n} lanes")
    }
}

/// "2 baskets left.", "1 basket left.", "No baskets left."
pub fn baskets_left(n: u32) -> String {
    match n {
        0 => "No baskets left.".to_string(),
        1 => "1 basket left.".to_string(),
        n => format!("{n} baskets left."),
    }
}

/// "1 egg", "4 eggs".
pub fn eggs_phrase(n: usize) -> String {
    if n == 1 {
        "1 egg".to_string()
    } else {
        format!("{n} eggs")
    }
}
