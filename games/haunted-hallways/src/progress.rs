//! What the game remembers between runs: floors opened, best times, how
//! deep each mode has gone, and the curses switched on.

use std::collections::BTreeMap;

use hearken::Ctx;
use parlor_core::{Parlor, Records, ScoreResult};
use serde::{Deserialize, Serialize};

use crate::rules::{Curses, Mode, Run, Twist, FLOORS};
use crate::ID;

/// The records key the progress is saved under.
pub fn key() -> String {
    format!("{ID}/progress")
}

/// Clearing this floor opens Nightmare.
pub const NIGHTMARE_AT: u32 = 25;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// The highest floor of the house ever cleared, 0 for none.
    #[serde(default)]
    pub cleared: u32,
    /// Best time per floor in tenths of a second, by floor number.
    #[serde(default)]
    pub floor_best: BTreeMap<u32, u64>,
    /// The highest floor cleared in Nightmare.
    #[serde(default)]
    pub nightmare_deepest: u32,
    /// The highest floor cleared in Endless night.
    #[serde(default)]
    pub endless_deepest: u32,
    /// Times you reached the top of the house: floor 100 cleared.
    #[serde(default)]
    pub escapes: u32,
    #[serde(default)]
    pub ghosts_evaded: u32,
    /// Floors cleared in all, in every mode but challenges.
    #[serde(default)]
    pub floors_cleared: u32,
    /// The curses switched on, by twist id.
    #[serde(default)]
    pub curses: Vec<String>,
}

/// What clearing a floor changed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloorSaved {
    /// The time beat the old best, or was the first.
    pub record: bool,
    /// The old best, in seconds.
    pub previous: Option<f64>,
    /// The floor this opened in Floor select.
    pub opened: Option<u32>,
    /// Clearing this floor opened Nightmare.
    pub nightmare: bool,
    /// Clearing this floor opened Endless night.
    pub endless: bool,
}

/// What ending a run changed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EndSaved {
    pub score: Option<ScoreResult>,
    /// The run went deeper than ever in its mode.
    pub deepest: bool,
}

fn tenths(secs: f64) -> u64 {
    (secs.max(0.0) * 10.0).round() as u64
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, &key())
    }

    pub fn store(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, &key(), self);
    }

    /// Floors open in Floor select: every one cleared, and the next.
    pub fn open_floors(&self) -> u32 {
        (self.cleared + 1).min(FLOORS)
    }

    /// Where Play begins: the highest open floor.
    pub fn frontier(&self) -> u32 {
        self.open_floors()
    }

    pub fn nightmare_open(&self) -> bool {
        self.cleared >= NIGHTMARE_AT
    }

    pub fn endless_open(&self) -> bool {
        self.cleared >= FLOORS
    }

    /// A floor's best time in seconds.
    pub fn floor_time(&self, floor: u32) -> Option<f64> {
        self.floor_best.get(&floor).map(|t| *t as f64 / 10.0)
    }

    /// The curses switched on.
    pub fn curses(&self) -> Curses {
        Twist::ALL
            .into_iter()
            .filter(|t| self.curses.iter().any(|c| c == t.id()))
            .fold(Curses::default(), |c, t| c.with(t))
    }

    /// Switches a curse on or off, and says which it is now.
    pub fn toggle_curse(&mut self, t: Twist) -> bool {
        if let Some(at) = self.curses.iter().position(|c| c == t.id()) {
            self.curses.remove(at);
            false
        } else {
            self.curses.push(t.id().to_string());
            true
        }
    }

    /// A floor cleared in `secs`, in `mode`.
    pub fn floor_done(&mut self, mode: Mode, floor: u32, secs: f64) -> FloorSaved {
        let mut saved = FloorSaved::default();
        if matches!(mode, Mode::Challenge(_)) {
            return saved;
        }
        self.floors_cleared += 1;
        if matches!(mode, Mode::Stirs(_) | Mode::LanternNight(_)) {
            return saved;
        }
        match mode {
            Mode::Nightmare => {
                self.nightmare_deepest = self.nightmare_deepest.max(floor);
                return saved;
            }
            Mode::Endless => {
                self.endless_deepest = self.endless_deepest.max(floor);
                return saved;
            }
            Mode::Challenge(_) | Mode::Stirs(_) | Mode::LanternNight(_) => return saved,
            Mode::Climb(_) | Mode::Floor(_) => {}
        }
        let t = tenths(secs);
        saved.previous = self.floor_time(floor);
        saved.record = self.floor_best.get(&floor).is_none_or(|&b| t < b);
        if saved.record {
            self.floor_best.insert(floor, t);
        }
        if floor > self.cleared {
            let was = self.clone();
            self.cleared = floor;
            if floor < FLOORS {
                saved.opened = Some(floor + 1);
            }
            saved.nightmare = !was.nightmare_open() && self.nightmare_open();
            saved.endless = !was.endless_open() && self.endless_open();
        }
        if floor == FLOORS {
            self.escapes += 1;
        }
        saved
    }

    /// A run over: ghosts counted, and how deep it went.
    pub fn run_done(&mut self, run: &Run) -> bool {
        self.ghosts_evaded += run.ghosts_evaded;
        match run.mode {
            Mode::Nightmare | Mode::Endless => {
                // The depth was saved floor by floor; say if it is a record.
                let best = if run.mode == Mode::Nightmare {
                    self.nightmare_deepest
                } else {
                    self.endless_deepest
                };
                run.deepest > 0 && run.deepest >= best
            }
            _ => false,
        }
    }
}

/// Saves a cleared floor at once, so leaving mid-run keeps it.
pub fn save_floor(ctx: &mut Ctx<Parlor>, mode: Mode, floor: u32, secs: f64) -> FloorSaved {
    let mut progress = Progress::load(ctx);
    let saved = progress.floor_done(mode, floor, secs);
    progress.store(ctx);
    saved
}

/// Saves the end of a run: ghosts, and its score.
pub fn save_end(ctx: &mut Ctx<Parlor>, run: &Run) -> EndSaved {
    let mut progress = Progress::load(ctx);
    let deepest = progress.run_done(run);
    progress.store(ctx);
    // A challenge's result goes to its own record, not the best boards.
    let scored = run.score > 0 && !matches!(run.mode, Mode::Challenge(_));
    let score = scored.then(|| Records::record(ctx, ID, &run.mode.board(), run.score));
    EndSaved { score, deepest }
}
