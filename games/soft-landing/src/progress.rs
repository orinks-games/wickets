//! What the game remembers between visits: moons landed, the softest
//! touchdown, tallies and the call-out interval.

use hearken::Ctx;
use parlor_core::{Pace, Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::challenge::Job;
use crate::rules::{survey_moon, Flight, Outcome, CALL_EVERY, DEFAULT_CALL_EVERY, MOONS};
use crate::ID;

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

/// The score board of survey moon `n`: "moon/thistle".
pub fn board(n: u32) -> String {
    format!("moon/{}", survey_moon(n).name.to_lowercase())
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Survey moons landed in order: moon `cleared + 1` is the next.
    #[serde(default)]
    pub cleared: u32,
    /// The slowest touchdown that landed, in metres a second.
    #[serde(default)]
    pub softest: Option<f64>,
    /// The slowest touchdown that landed at the relaxed pace, kept apart.
    #[serde(default)]
    pub relaxed_softest: Option<f64>,
    #[serde(default)]
    pub landings: u32,
    #[serde(default)]
    pub crashes: u32,
    #[serde(default)]
    pub practice_landings: u32,
    /// Metres between altitude call-outs, as the player last set it.
    #[serde(default)]
    pub call_every: Option<u32>,
}

/// What a finished survey flight changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    /// The score on the moon's board, when it landed.
    pub score: Option<ScoreResult>,
    /// The moon this landing opened.
    pub opened: Option<u32>,
    /// This was the softest touchdown yet.
    pub softest: bool,
    /// Every survey moon is now landed, for the first time.
    pub survey_done: bool,
}

impl Saved {
    /// A flight that saved nothing.
    pub fn nothing() -> Saved {
        Saved {
            score: None,
            opened: None,
            softest: false,
            survey_done: false,
        }
    }
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Whether the player has flown at all.
    pub fn any(&self) -> bool {
        self.landings > 0 || self.crashes > 0 || self.practice_landings > 0
    }

    /// The survey moons open to fly, from 1.
    pub fn open_moons(&self) -> u32 {
        (self.cleared + 1).min(MOONS)
    }

    /// The moon the Play row flies.
    pub fn next_moon(&self) -> u32 {
        self.open_moons()
    }

    /// The call-out interval, always one of [`CALL_EVERY`].
    pub fn call_every(&self) -> u32 {
        self.call_every
            .filter(|e| CALL_EVERY.contains(e))
            .unwrap_or(DEFAULT_CALL_EVERY)
    }

    /// Adds a landing or a crash to the tallies.
    fn tally(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Landed { .. } => self.landings += 1,
            Outcome::Crashed { .. } => self.crashes += 1,
        }
    }

    /// Tallies a survey flight, and keeps its touchdown when it is the
    /// softest yet at `pace`: relaxed keeps its own.
    fn count(&mut self, outcome: Outcome, pace: Pace) -> bool {
        self.tally(outcome);
        match outcome {
            Outcome::Landed { speed } => {
                let kept = if pace.separate() {
                    &mut self.relaxed_softest
                } else {
                    &mut self.softest
                };
                let softest = kept.is_none_or(|s| speed < s);
                if softest {
                    *kept = Some(speed);
                }
                softest
            }
            Outcome::Crashed { .. } => false,
        }
    }
}

/// Saves a finished flight onto survey moon `n`, flown at `pace`. The
/// moon it opens opens at every pace; its score goes on the pace's board.
pub fn save_flight(ctx: &mut Ctx<Parlor>, n: u32, flight: &Flight, pace: Pace) -> Saved {
    let mut p = Progress::load(ctx);
    let Some(outcome) = flight.outcome else {
        return Saved::nothing();
    };
    let softest = p.count(outcome, pace);
    let mut opened = None;
    let mut survey_done = false;
    if outcome.landed() && n == p.cleared + 1 {
        p.cleared = n;
        if n < MOONS {
            opened = Some(n + 1);
        } else {
            survey_done = true;
        }
    }
    p.store(ctx);
    // A score of 0 goes on no board.
    let score = outcome
        .landed()
        .then(|| {
            parlor_core::pace::record(
                ctx,
                ID,
                &pace.board(&board(n)),
                flight.score(outcome) as u64,
            )
        })
        .flatten();
    Saved {
        score,
        opened,
        softest,
        survey_done,
    }
}

/// Saves a finished event row flight: its tallies, and its score on
/// `board` when it landed. An event moon has rules of its own (the
/// Tranquility pad's low gravity), so it never sets the standing softest
/// touchdown.
pub fn save_event(ctx: &mut Ctx<Parlor>, board: &str, flight: &Flight, pace: Pace) -> Saved {
    let mut p = Progress::load(ctx);
    let Some(outcome) = flight.outcome else {
        return Saved::nothing();
    };
    p.tally(outcome);
    p.store(ctx);
    let score = outcome
        .landed()
        .then(|| {
            parlor_core::pace::record(ctx, ID, &pace.board(board), flight.score(outcome) as u64)
        })
        .flatten();
    Saved {
        score,
        opened: None,
        softest: false,
        survey_done: false,
    }
}

/// Adds a challenge run's landings and crash to the tallies.
pub fn save_challenge(ctx: &mut Ctx<Parlor>, job: &Job) {
    let mut p = Progress::load(ctx);
    p.landings += job.landed;
    if let Some(Outcome::Crashed { .. }) = job.outcome {
        p.crashes += 1;
    }
    p.store(ctx);
}

/// Adds practice landings.
pub fn save_practice(ctx: &mut Ctx<Parlor>, landings: u32) {
    if landings == 0 {
        return;
    }
    let mut p = Progress::load(ctx);
    p.practice_landings += landings;
    p.store(ctx);
}

/// Remembers the call-out interval the player chose.
pub fn save_call_every(ctx: &mut Ctx<Parlor>, metres: u32) {
    let mut p = Progress::load(ctx);
    p.call_every = Some(metres);
    p.store(ctx);
}

/// The records key for the teaching lines already heard on this save.
pub fn taught_key() -> String {
    format!("{ID}/taught")
}

/// Marks `cue` heard on this save. True the first time.
pub fn learn(ctx: &mut Ctx<Parlor>, cue: &str) -> bool {
    let mut taught: Vec<String> = Records::load(ctx, &taught_key());
    if taught.iter().any(|c| c == cue) {
        return false;
    }
    taught.push(cue.to_string());
    Records::store(ctx, &taught_key(), &taught);
    true
}
