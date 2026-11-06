//! What Larkfield keeps between visits: your hamlet, and your settings.

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::almanac::Pace;
use crate::rules::{DayLength, Hamlet};

const KEY: &str = "larkfield/progress";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Your own hamlet, once started.
    #[serde(default)]
    pub hamlet: Option<Hamlet>,
    /// How busy the almanac is.
    #[serde(default)]
    pub pace: Pace,
    /// How long a day lasts.
    #[serde(default)]
    pub day_length: DayLength,
    /// Settlers play their parts in the hamlet's tune.
    #[serde(default = "yes")]
    pub band: bool,
    /// Work sounds from along the lane, quieter the further away.
    #[serde(default = "yes")]
    pub work_sounds: bool,
    /// Years finished, across every hamlet you have had.
    #[serde(default)]
    pub years: u32,
}

impl Default for Progress {
    fn default() -> Progress {
        Progress {
            hamlet: None,
            pace: Pace::Steady,
            day_length: DayLength::Normal,
            band: true,
            work_sounds: true,
            years: 0,
        }
    }
}

fn yes() -> bool {
    true
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        Records::load(ctx, KEY)
    }

    pub fn peek(ctx: &Ctx<Parlor>) -> Progress {
        Records::peek(ctx, KEY)
    }

    pub fn save(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, KEY, self);
    }

    /// Keeps the hamlet, and counts a year when one turns.
    pub fn keep(ctx: &mut Ctx<Parlor>, h: &Hamlet) {
        let mut p = Progress::load(ctx);
        let before = p.hamlet.as_ref().map_or(1, |o| o.year());
        if h.year() > before {
            p.years += h.year() - before;
        }
        let mut copy = h.clone();
        copy.events.clear();
        p.hamlet = Some(copy);
        p.save(ctx);
    }
}
