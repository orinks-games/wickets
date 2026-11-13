//! The rules of a challenge job, with nothing spoken: which safes, which
//! twist, and how the score adds up across them.
//!
//! - Daily, "Tonight's job": one safe built from the day's seed, with one
//!   twist from [`Twist`] announced before play.
//! - Weekly, "The vault of three": three safes in a row, harder each time,
//!   with the week's twist on all three. One relocker guards the whole
//!   vault, so three strikes across all three safes jam it, and the three
//!   hints are shared too.
//! - Monthly, "The Crown heist": a story event of two safes, the curator's
//!   safe and the Crown vault, against one clock for the whole job, with a
//!   guard on the corridor.

use parlor_core::{Challenge, Period, Rng};

use crate::rules::{GuardSpec, Mode, Outcome, Run, SafeSpec, STRIKES};

/// One clock for the whole monthly heist, in seconds.
pub const HEIST_CLOCK: f64 = 300.0;
/// Hints shared across the weekly vault.
pub const VAULT_HINTS: u32 = 3;

/// A rule that changes a daily or weekly safe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Twist {
    /// The clunk is barely there.
    QuietTumblers,
    /// A guard who comes often and stays long.
    NervousGuard,
    /// Right, left, right: the real dial's rule.
    DirectionRule,
    /// Three false gates on every wheel.
    CrowdedGates,
    /// No hints at all.
    NoHints,
    /// The building never stops knocking.
    BusyBuilding,
}

impl Twist {
    pub const ALL: [Twist; 6] = [
        Twist::QuietTumblers,
        Twist::NervousGuard,
        Twist::DirectionRule,
        Twist::CrowdedGates,
        Twist::NoHints,
        Twist::BusyBuilding,
    ];

    /// The weekly vault's safes already keep the direction rule.
    pub const WEEKLY: [Twist; 5] = [
        Twist::QuietTumblers,
        Twist::NervousGuard,
        Twist::CrowdedGates,
        Twist::NoHints,
        Twist::BusyBuilding,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::QuietTumblers => "quiet tumblers",
            Twist::NervousGuard => "a nervous guard",
            Twist::DirectionRule => "the direction rule",
            Twist::CrowdedGates => "crowded gates",
            Twist::NoHints => "no hints",
            Twist::BusyBuilding => "a busy building",
        }
    }

    /// What the twist does, in a sentence.
    pub fn describe(self) -> &'static str {
        match self {
            Twist::QuietTumblers => {
                "the tumblers are freshly oiled and harder to find, as in the Crown vault"
            }
            Twist::NervousGuard => {
                "a night guard walks by often and listens long at the door. When he stops, keep still until he walks away"
            }
            Twist::DirectionRule => {
                "wheel 1 is found turning right, wheel 2 turning left, and so on, and the tumbler falls only the right way"
            }
            Twist::CrowdedGates => "every wheel has three false gates at wrong numbers",
            Twist::NoHints => "there are no hints",
            Twist::BusyBuilding => {
                "pipes knock and floors creak every few seconds, off to your left and right"
            }
        }
    }

    pub fn apply(self, spec: SafeSpec) -> SafeSpec {
        match self {
            Twist::QuietTumblers => SafeSpec {
                clunk: 0.45,
                ..spec
            },
            Twist::NervousGuard => SafeSpec {
                guard: Some(GuardSpec {
                    gap: (12.0, 18.0),
                    approach: 5.0,
                    near: (6.0, 9.0),
                    leave: 4.0,
                }),
                ..spec
            },
            Twist::DirectionRule => SafeSpec {
                alternate: true,
                ..spec
            },
            Twist::CrowdedGates => SafeSpec { decoys: 3, ..spec },
            Twist::NoHints => SafeSpec { hints: 0, ..spec },
            Twist::BusyBuilding => SafeSpec {
                noises: Some((3.0, 6.0)),
                ..spec
            },
        }
    }
}

/// Names for the daily safe, one picked by the day's seed.
const DAILY_NAMES: [&str; 8] = [
    "the tailor's safe",
    "the auction house safe",
    "the shipping office safe",
    "the theatre box office safe",
    "the observatory safe",
    "the racetrack safe",
    "the antique dealer's safe",
    "the newspaper's safe",
];

fn base(number: u32, name: &'static str) -> SafeSpec {
    SafeSpec {
        number,
        name,
        wheels: 3,
        decoys: 1,
        clunk: 0.75,
        alternate: false,
        guard: None,
        call_guard: true,
        noises: Some((12.0, 20.0)),
        hints: 2,
        par: 100.0,
        limit: 0.0,
        news: "",
    }
}

const GUARD: GuardSpec = GuardSpec {
    gap: (22.0, 34.0),
    approach: 6.0,
    near: (5.0, 7.0),
    leave: 4.0,
};

/// Everything a challenge job will be, decided by its seed alone.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub period: Period,
    pub twist: Option<Twist>,
    pub safes: Vec<SafeSpec>,
    /// Each safe's own seed, for its numbers.
    pub seeds: Vec<u64>,
}

impl Plan {
    pub fn for_challenge(ch: &Challenge) -> Plan {
        Plan::new(ch.period, ch.rng())
    }

    pub fn new(period: Period, mut rng: Rng) -> Plan {
        let (twist, safes) = match period {
            Period::Daily => {
                let twist = *rng.pick(&Twist::ALL);
                let name = *rng.pick(&DAILY_NAMES);
                let wheels = 3 + rng.below(2) as usize;
                let spec = SafeSpec {
                    wheels,
                    par: 70.0 + 30.0 * wheels as f64,
                    ..base(1, name)
                };
                (Some(twist), vec![twist.apply(spec)])
            }
            Period::Weekly => {
                let twist = *rng.pick(&Twist::WEEKLY);
                let safes = vec![
                    SafeSpec {
                        par: 100.0,
                        ..base(1, "the outer safe")
                    },
                    SafeSpec {
                        wheels: 4,
                        decoys: 2,
                        clunk: 0.65,
                        alternate: true,
                        par: 150.0,
                        ..base(2, "the inner safe")
                    },
                    SafeSpec {
                        wheels: 4,
                        decoys: 2,
                        clunk: 0.6,
                        alternate: true,
                        guard: Some(GUARD),
                        call_guard: false,
                        par: 180.0,
                        ..base(3, "the strongroom safe")
                    },
                ]
                .into_iter()
                .map(|s| twist.apply(s))
                .map(|s| SafeSpec {
                    hints: if s.hints == 0 { 0 } else { VAULT_HINTS },
                    ..s
                })
                .collect();
                (Some(twist), safes)
            }
            Period::Monthly => (
                None,
                vec![
                    SafeSpec {
                        alternate: true,
                        guard: Some(GUARD),
                        call_guard: true,
                        hints: 2,
                        limit: HEIST_CLOCK,
                        ..base(1, "the curator's safe")
                    },
                    SafeSpec {
                        wheels: 5,
                        decoys: 2,
                        clunk: 0.55,
                        alternate: true,
                        guard: Some(GuardSpec {
                            gap: (18.0, 28.0),
                            ..GUARD
                        }),
                        call_guard: false,
                        hints: 2,
                        limit: HEIST_CLOCK,
                        ..base(2, "the Crown vault")
                    },
                ],
            ),
        };
        let seeds = safes.iter().map(|_| rng.next_u64()).collect();
        Plan {
            period,
            twist,
            safes,
            seeds,
        }
    }

    pub fn mode(&self) -> Mode {
        Mode::Challenge {
            period: self.period,
            timed: self.period == Period::Monthly,
        }
    }

    /// The weekly vault shares its strikes and hints across safes.
    pub fn pooled(&self) -> bool {
        self.period == Period::Weekly
    }

    /// The monthly heist runs on one clock.
    pub fn clock(&self) -> Option<f64> {
        (self.period == Period::Monthly).then_some(HEIST_CLOCK)
    }
}

/// A challenge job under way: the safe now, and what is banked.
#[derive(Clone, Debug)]
pub struct Job {
    pub plan: Plan,
    /// The safe being cracked, 0-based.
    pub index: usize,
    /// Safes opened so far.
    pub opened: usize,
    /// Points banked from opened safes.
    pub banked: u64,
    /// Strikes and hints already spent when this safe began.
    strikes_before: u32,
    /// Set once the job is over.
    pub outcome: Option<Outcome>,
    /// The bonus for time left on the heist clock, once earned.
    pub clock_bonus: u64,
}

impl Job {
    pub fn new(plan: Plan) -> Job {
        Job {
            plan,
            index: 0,
            opened: 0,
            banked: 0,
            strikes_before: 0,
            outcome: None,
            clock_bonus: 0,
        }
    }

    pub fn safes(&self) -> usize {
        self.plan.safes.len()
    }

    /// The run for the current safe, fresh.
    pub fn run(&self) -> Run {
        Run::with_spec(
            self.plan.mode(),
            self.plan.safes[self.index],
            self.plan.seeds[self.index],
        )
    }

    /// A safe's points: 100 a wheel, a bonus under par when there is no
    /// clock, less 30 a strike and 20 a hint spent on it.
    pub fn safe_score(&self, run: &Run) -> u64 {
        let wheels = run.spec.wheels as f64 * 100.0;
        let bonus = if self.plan.clock().is_some() {
            0.0
        } else {
            (run.spec.par - run.time).max(0.0) * 2.0
        };
        let strikes = run.strikes.saturating_sub(self.strikes_before);
        let cost = strikes as f64 * 30.0 + run.hints_used as f64 * 20.0;
        (wheels + bonus - cost).round().max(10.0) as u64
    }

    /// The safe `run` ended. On an opening with safes still to go, the next
    /// safe's run comes back, carrying the shared strikes, hints or clock.
    /// Otherwise the job is over and `None` comes back.
    pub fn advance(&mut self, run: &Run) -> Option<Run> {
        let outcome = run.outcome?;
        if !outcome.won() {
            self.outcome = Some(outcome);
            return None;
        }
        self.banked += self.safe_score(run);
        self.opened += 1;
        if self.index + 1 >= self.safes() {
            if self.plan.clock().is_some() {
                self.clock_bonus = (run.time_left.max(0.0) * 3.0).round() as u64;
                self.banked += self.clock_bonus;
            }
            self.outcome = Some(Outcome::Opened);
            return None;
        }
        self.index += 1;
        let mut next = self.run();
        if self.plan.pooled() {
            next.strikes = run.strikes;
            next.hints_left = run.hints_left;
            self.strikes_before = run.strikes;
        }
        if self.plan.clock().is_some() {
            next.carry_clock(run.time_left);
        }
        Some(next)
    }

    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }

    /// Every safe opened.
    pub fn won(&self) -> bool {
        self.outcome.is_some_and(|o| o.won())
    }

    /// The score the job submits.
    pub fn score(&self) -> u64 {
        self.banked
    }
}

/// The challenge's own name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's job",
        Period::Weekly => "The vault of three",
        Period::Monthly => "The Crown heist",
    }
}

/// A few words on what the challenge is, for its row in a list.
pub fn tag(plan: &Plan) -> String {
    match plan.period {
        Period::Daily => format!(
            "{}, {} wheels, with {}",
            plan.safes[0].name,
            crate::rules::count_word(plan.safes[0].wheels),
            plan.twist.map(Twist::name).unwrap_or("no twist")
        ),
        Period::Weekly => format!(
            "three safes, one relocker, with {}",
            plan.twist.map(Twist::name).unwrap_or("no twist")
        ),
        Period::Monthly => "a story heist of two safes on one five-minute clock".to_string(),
    }
}

/// What the challenge asks, in a sentence or three.
pub fn rule(plan: &Plan) -> String {
    match plan.period {
        Period::Daily => {
            let s = &plan.safes[0];
            let t = plan.twist.unwrap_or(Twist::NoHints);
            format!(
                "One safe, {}: {} wheels. Twist, {}: {}.",
                s.name,
                crate::rules::count_word(s.wheels),
                t.name(),
                t.describe()
            )
        }
        Period::Weekly => {
            let t = plan.twist.unwrap_or(Twist::NoHints);
            let hints = match plan.safes[0].hints {
                0 => "No hints".to_string(),
                n => format!("{n} hints, shared"),
            };
            format!(
                "Three safes, one relocker: {STRIKES} strikes across all three jam it. {hints}. Twist, {}: {}.",
                t.name(),
                t.describe()
            )
        }
        Period::Monthly => format!(
            "Two safes on one clock of {}. It stops while the guard listens, and a hint costs five seconds. Open both for 3 points a second left.",
            crate::rules::time_words(HEIST_CLOCK)
        ),
    }
}

/// The monthly heist's story, read before play.
pub fn story() -> Vec<String> {
    vec![
        "The Crown heist. Once a month the Crown jewels leave the vault for cleaning and come back at midnight. For five minutes, while the night shift changes, the museum's east wing is yours.".to_string(),
        "Your fence has the plan. Through the curator's office first: his safe holds the key card to the vault corridor. Then the Crown vault itself, five wheels deep. A single guard walks the corridor between the two. You must be out before the new shift clocks in.".to_string(),
    ]
}

/// The heist's ending, by how far you got.
pub fn ending(job: &Job) -> &'static str {
    match (job.won(), job.opened) {
        (true, _) => {
            "The vault door swings wide. You lift the Crown from its velvet and slip out of the east wing as the new shift's footsteps reach the stairs. By morning, the city has the most famous empty case in its history."
        }
        (false, 1) => {
            "You had the key card in your hand and the Crown vault in front of you. Next month, the jewels come home again."
        }
        _ => "You never got past the curator's office. Next month, the jewels come home again.",
    }
}
