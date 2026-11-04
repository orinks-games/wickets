//! The daily, weekly and monthly challenges as plain rules: which shows a
//! challenge holds, the twist it plays under, and a run through it. No
//! screens or sound, so tests can build and score a challenge directly.

use parlor_core::{Challenge, Period, Rng};

use crate::rules::{
    generate, generate_finale, uses_positions, Judgement, Kind, Launch, Pos, Scoring,
    TOUGH_WINDOW_MS,
};

/// The daily and weekly challenges' twists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Twist {
    /// Every show plays at double speed.
    Quickstep,
    /// Timing points only within 0.3 seconds.
    ToughCrowd,
    /// The show plays once: no second listen, no hints.
    OneShot,
    /// The show's fireworks sound far off and quiet through the fog.
    NightFog,
}

impl Twist {
    pub const ALL: [Twist; 4] = [
        Twist::Quickstep,
        Twist::ToughCrowd,
        Twist::OneShot,
        Twist::NightFog,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::Quickstep => "quickstep",
            Twist::ToughCrowd => "tough crowd",
            Twist::OneShot => "one shot",
            Twist::NightFog => "night fog",
        }
    }

    /// What the twist does, in a sentence.
    pub fn describe(self) -> String {
        match self {
            Twist::Quickstep => {
                "the shows play at double speed, fireworks a second apart instead of two"
                    .to_string()
            }
            Twist::ToughCrowd => format!(
                "the crowd only pays for timing within {} seconds; any firework further off earns no timing points",
                TOUGH_WINDOW_MS as f64 / 1000.0
            ),
            Twist::OneShot => {
                "each show plays only once: no hearing it again, and no hints".to_string()
            }
            Twist::NightFog => {
                "fog hangs over the field, so the show's fireworks sound far off and quiet; your own launches sound as usual"
                    .to_string()
            }
        }
    }
}

/// A monthly festival: its name and the firework it celebrates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Festival {
    pub name: &'static str,
    pub signature: Kind,
}

/// The festival a month holds, January first.
pub fn festival(month: u32) -> Festival {
    let (name, signature) = match month {
        1 => ("Winter Lights", Kind::Sparkle),
        2 => ("Lantern Night", Kind::Whistle),
        3 => ("Spring Thunder", Kind::Boom),
        4 => ("April Showers", Kind::Crackle),
        5 => ("Blossom Burst", Kind::Sparkle),
        6 => ("Midsummer Night", Kind::Whistle),
        7 => ("Independence Day", Kind::Boom),
        8 => ("Summer Crackle", Kind::Crackle),
        9 => ("Harvest Moon", Kind::Sparkle),
        10 => ("Bonfire Parade", Kind::Crackle),
        11 => ("Bonfire Night", Kind::Boom),
        _ => ("New Year Countdown", Kind::Whistle),
    };
    Festival { name, signature }
}

/// One show of a challenge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    /// "The build", "The finale".
    pub name: String,
    pub show: Vec<Launch>,
    /// Whether its fireworks may go up off center.
    pub placed: bool,
}

/// Everything a challenge holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub period: Period,
    pub acts: Vec<Act>,
    pub twist: Option<Twist>,
    pub festival: Option<Festival>,
    /// A show under 60 percent ends the run (the weekly gauntlet).
    pub ends_on_fail: bool,
}

/// Halves every firework's time, for the quickstep.
fn quicken(show: &mut [Launch]) {
    for l in show.iter_mut() {
        l.ms /= 2;
    }
}

fn twist_from(rng: &mut Rng) -> Twist {
    Twist::ALL[rng.range(0, Twist::ALL.len() as i32 - 1) as usize]
}

fn act(name: String, level: u32, twist: Option<Twist>, rng: &mut Rng) -> Act {
    let mut show = generate(level, rng);
    if twist == Some(Twist::Quickstep) {
        quicken(&mut show);
    }
    Act {
        name,
        show,
        placed: uses_positions(level),
    }
}

impl Plan {
    /// The daily: one show drawn from the harder half of the campaign
    /// (levels 11 to 20), under a twist.
    pub fn daily(rng: &mut Rng) -> Plan {
        let twist = twist_from(rng);
        let level = rng.range(11, 20) as u32;
        Plan {
            period: Period::Daily,
            acts: vec![act("Tonight's show".to_string(), level, Some(twist), rng)],
            twist: Some(twist),
            festival: None,
            ends_on_fail: false,
        }
    }

    /// The weekly gauntlet: five shows, each harder than the last, under a
    /// twist. A show under 60 percent ends it.
    pub fn weekly(rng: &mut Rng) -> Plan {
        let twist = twist_from(rng);
        let bands = [(10, 11), (12, 13), (14, 16), (16, 18), (18, 20)];
        // Named apart from "show 1 of 5", which the intro already says.
        let names = [
            "The warm-up",
            "The build",
            "The big show",
            "The bigger show",
            "The last show",
        ];
        let acts = bands
            .iter()
            .enumerate()
            .map(|(i, &(lo, hi))| {
                let level = rng.range(lo, hi) as u32;
                act(names[i].to_string(), level, Some(twist), rng)
            })
            .collect();
        Plan {
            period: Period::Weekly,
            acts,
            twist: Some(twist),
            festival: None,
            ends_on_fail: true,
        }
    }

    /// The monthly festival: two acts and a long finale, all leaning on
    /// the festival's signature firework, which pays a bonus whenever it is
    /// matched near-perfectly. Every act is played, pass or fail.
    pub fn monthly(rng: &mut Rng, month: u32) -> Plan {
        let fest = festival(month);
        let lean = |show: &mut Vec<Launch>, rng: &mut Rng| {
            // range is inclusive: 0 or 1, so half of them.
            for l in show.iter_mut() {
                if rng.range(0, 1) == 0 {
                    l.kind = fest.signature;
                }
            }
        };
        let mut first = generate(14, rng);
        lean(&mut first, rng);
        let mut second = generate(18, rng);
        lean(&mut second, rng);
        let mut finale = generate_finale(9, rng);
        lean(&mut finale, rng);
        Plan {
            period: Period::Monthly,
            acts: vec![
                Act {
                    name: "The opening act".to_string(),
                    show: first,
                    placed: true,
                },
                Act {
                    name: "The second act".to_string(),
                    show: second,
                    placed: true,
                },
                Act {
                    name: "The festival finale".to_string(),
                    show: finale,
                    placed: true,
                },
            ],
            twist: None,
            festival: Some(fest),
            ends_on_fail: false,
        }
    }

    /// The plan for a parlor challenge, from its shared seed.
    pub fn for_challenge(ch: &Challenge) -> Plan {
        let mut rng = ch.rng();
        match ch.period {
            Period::Daily => Plan::daily(&mut rng),
            Period::Weekly => Plan::weekly(&mut rng),
            Period::Monthly => Plan::monthly(&mut rng, month_of(ch)),
        }
    }

    pub fn scoring(&self) -> Scoring {
        Scoring {
            tough: self.twist == Some(Twist::ToughCrowd),
            signature: self.festival.map(|f| f.signature),
        }
    }

    /// The challenge's own name.
    pub fn name(&self) -> String {
        match (self.period, self.festival) {
            (Period::Daily, _) => "Tonight's show".to_string(),
            (Period::Weekly, _) => "The five-show gauntlet".to_string(),
            (_, Some(f)) => format!("The {} festival", f.name),
            _ => "The festival".to_string(),
        }
    }

    /// A few words for the challenge's row: "one show, quickstep".
    pub fn tag(&self) -> String {
        match self.period {
            Period::Daily => format!(
                "one show of {} fireworks, {}",
                self.acts[0].show.len(),
                self.twist.map_or("", |t| t.name())
            ),
            Period::Weekly => format!(
                "five shows, one failure ends it, {}",
                self.twist.map_or("", |t| t.name())
            ),
            Period::Monthly => format!(
                "three acts ending in a {}-firework finale, {}s pay a bonus",
                self.acts.last().map_or(0, |a| a.show.len()),
                self.festival.map_or("", |f| f.signature.name())
            ),
        }
    }

    /// What the challenge asks, in a sentence or two.
    pub fn rule(&self) -> String {
        let twist = self
            .twist
            .map(|t| format!(" The twist, {}: {}.", t.name(), t.describe()))
            .unwrap_or_default();
        match self.period {
            Period::Daily => format!(
                "One show of {} fireworks{}, from the harder half of the campaign. Score as many show points as you can.{twist}",
                self.acts[0].show.len(),
                if self.acts[0].placed {
                    ", left, center and right"
                } else {
                    ", all in the center"
                }
            ),
            Period::Weekly => format!(
                "Five shows in a row, from {} up to {} fireworks. Show points add up, and a show under 60 percent ends the gauntlet there, its points still counted.{twist}",
                self.acts[0].show.len(),
                self.acts.last().map_or(0, |a| a.show.len())
            ),
            Period::Monthly => {
                let f = self.festival.unwrap_or_else(|| festival(12));
                format!(
                    "The {} festival: three acts, played whatever they score, ending with a {}-firework finale. The festival celebrates the {}, and its shows are full of them. On a show that clears, every {} matched near-perfectly, right kind and place within 0.3 seconds, adds {} bonus points.",
                    f.name,
                    self.acts.last().map_or(0, |a| a.show.len()),
                    f.signature.name(),
                    f.signature.name(),
                    crate::rules::SIGNATURE_BONUS
                )
            }
        }
    }
}

/// The month a monthly challenge belongs to, from its key ("2026-10").
pub fn month_of(ch: &Challenge) -> u32 {
    ch.key
        .split('-')
        .nth(1)
        .and_then(|m| m.parse().ok())
        .filter(|m| (1..=12).contains(m))
        .unwrap_or(1)
}

/// A challenge being played: which act is next and the points so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub challenge: Challenge,
    pub plan: Plan,
    /// The act being played, from 0.
    pub stage: usize,
    /// Show points of the acts finished.
    pub total: u64,
    /// Acts that scored 60 percent or more.
    pub cleared: u32,
}

impl Run {
    pub fn new(challenge: Challenge) -> Run {
        let plan = Plan::for_challenge(&challenge);
        Run::with_plan(challenge, plan)
    }

    pub fn with_plan(challenge: Challenge, plan: Plan) -> Run {
        Run {
            challenge,
            plan,
            stage: 0,
            total: 0,
            cleared: 0,
        }
    }

    pub fn act(&self) -> &Act {
        &self.plan.acts[self.stage.min(self.plan.acts.len() - 1)]
    }

    /// "show 2 of 5", for a run of several acts.
    pub fn progress(&self) -> String {
        format!("show {} of {}", self.stage + 1, self.plan.acts.len())
    }

    /// Scores the current act into the run and moves on. True when the run
    /// is over: every act played, or a gauntlet show failed.
    pub fn record(&mut self, j: &Judgement) -> bool {
        self.total += j.points;
        if j.passed() {
            self.cleared += 1;
        }
        self.stage += 1;
        self.stage >= self.plan.acts.len() || (self.plan.ends_on_fail && !j.passed())
    }

    /// Whether the act's show uses places other than the center.
    pub fn placed(&self) -> bool {
        let act = self.act();
        act.placed || act.show.iter().any(|l| l.pos != Pos::Center)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{judge, judge_with};
    use parlor_core::Date;

    #[test]
    fn each_period_has_its_own_shape() {
        let d = Date::new(2026, 10, 16);
        let daily = Plan::for_challenge(&Challenge::on(d, "liberty-lights", Period::Daily));
        assert_eq!(daily.acts.len(), 1);
        assert!(daily.twist.is_some());
        let weekly = Plan::for_challenge(&Challenge::on(d, "liberty-lights", Period::Weekly));
        assert_eq!(weekly.acts.len(), 5);
        assert!(weekly.ends_on_fail);
        let lens: Vec<usize> = weekly.acts.iter().map(|a| a.show.len()).collect();
        assert!(lens.windows(2).all(|w| w[0] <= w[1]), "{lens:?}");
        let monthly = Plan::for_challenge(&Challenge::on(d, "liberty-lights", Period::Monthly));
        assert_eq!(monthly.acts.len(), 3);
        assert_eq!(monthly.festival, Some(festival(10)));
        assert_eq!(monthly.acts[2].show.len(), 14);
        assert!(!monthly.ends_on_fail);
    }

    #[test]
    fn the_same_challenge_is_the_same_for_everyone() {
        let d = Date::new(2026, 10, 16);
        let ch = Challenge::on(d, "liberty-lights", Period::Daily);
        assert_eq!(Plan::for_challenge(&ch), Plan::for_challenge(&ch));
        let next = Challenge::on(Date::new(2026, 10, 17), "liberty-lights", Period::Daily);
        assert_ne!(Plan::for_challenge(&ch), Plan::for_challenge(&next));
    }

    #[test]
    fn every_twist_turns_up_and_the_quickstep_halves_time() {
        let mut seen = Vec::new();
        for seed in 0..200 {
            let plan = Plan::daily(&mut Rng::seeded(seed));
            let t = plan.twist.unwrap();
            if !seen.contains(&t) {
                seen.push(t);
            }
            // The same draws, without the quickstep, give the show at full
            // length: the quickstep's is every time halved.
            let mut rng = Rng::seeded(seed);
            twist_from(&mut rng);
            let level = rng.range(11, 20) as u32;
            let full = generate(level, &mut rng);
            let halved: Vec<i32> = full.iter().map(|l| l.ms / 2).collect();
            let times: Vec<i32> = plan.acts[0].show.iter().map(|l| l.ms).collect();
            if t == Twist::Quickstep {
                assert_eq!(times, halved);
            } else {
                assert_eq!(plan.acts[0].show, full);
            }
        }
        assert_eq!(seen.len(), 4, "{seen:?}");
    }

    #[test]
    fn a_tough_crowd_pays_no_timing_beyond_three_tenths() {
        let show = vec![Launch::new(Kind::Boom, Pos::Center, 1000)];
        let late = vec![Launch::new(Kind::Boom, Pos::Center, 1400)];
        assert_eq!(judge(&show, &late, false).accuracy, 90);
        let tough = Scoring {
            tough: true,
            signature: None,
        };
        assert_eq!(judge_with(&show, &late, false, tough).accuracy, 75);
        let close = vec![Launch::new(Kind::Boom, Pos::Center, 1250)];
        assert_eq!(judge_with(&show, &close, false, tough).accuracy, 95);
    }

    #[test]
    fn signature_fireworks_pay_only_on_a_cleared_show() {
        let show = vec![
            Launch::new(Kind::Whistle, Pos::Left, 0),
            Launch::new(Kind::Boom, Pos::Right, 2000),
        ];
        let scoring = Scoring {
            tough: false,
            signature: Some(Kind::Whistle),
        };
        let j = judge_with(&show, &show, false, scoring);
        assert_eq!(j.signature_bonus, 100);
        assert_eq!(j.points, 1000 + 50 + 100);
        let j = judge_with(&show[..1], &[], false, scoring);
        assert_eq!(j.signature_bonus, 0);
    }

    #[test]
    fn a_gauntlet_ends_at_its_first_failure() {
        let d = Date::new(2026, 10, 16);
        let mut run = Run::new(Challenge::on(d, "liberty-lights", Period::Weekly));
        let show = run.act().show.clone();
        let good = judge(&show, &show, false);
        assert!(!run.record(&good));
        assert_eq!(run.stage, 1);
        let bad = judge(&run.act().show.clone(), &[], false);
        assert!(run.record(&bad));
        assert_eq!(run.total, good.points);
        assert_eq!(run.cleared, 1);
    }

    #[test]
    fn a_festival_plays_every_act() {
        let d = Date::new(2026, 7, 4);
        let ch = Challenge::on(d, "liberty-lights", Period::Monthly);
        assert_eq!(month_of(&ch), 7);
        let mut run = Run::new(ch);
        assert_eq!(run.plan.festival.unwrap().name, "Independence Day");
        let bad = judge(&run.act().show.clone(), &[], false);
        assert!(!run.record(&bad));
        assert!(!run.record(&bad));
        assert!(run.record(&bad));
    }
}
