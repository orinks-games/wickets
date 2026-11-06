//! What Little Rapids keeps between runs: stars, boats, scraps, the
//! shelf, ribbons and settings. Best scores live in `Records`' boards.

use hearken::Ctx;
use parlor_core::{Parlor, Records};
use serde::{Deserialize, Serialize};

use crate::rules::{Boat, CleanWidth, Kind, Run, Stream};
use crate::ID;

const KEY: &str = "little-rapids/progress";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// Best stars per stream id.
    #[serde(default)]
    pub stars: Vec<(String, u32)>,
    /// Boats folded, by id. The paper boat is always yours.
    #[serde(default)]
    pub boats: Vec<String>,
    /// The boat you sail.
    #[serde(default)]
    pub boat: String,
    /// Paper scraps to fold with.
    #[serde(default)]
    pub scraps: u32,
    /// Trinkets on the shelf, by id.
    #[serde(default)]
    pub trinkets: Vec<String>,
    /// Ribbons earned, by id.
    #[serde(default)]
    pub ribbons: Vec<String>,
    /// Kinds already explained.
    #[serde(default)]
    pub met: Vec<String>,
    /// How wide a clean catch is: 0 narrow, 1 normal, 2 wide.
    #[serde(default = "normal")]
    pub clean_width: u8,
    /// Say each trinket's name when it is caught.
    #[serde(default = "yes")]
    pub trinket_names: bool,
    /// The bank warning's lapping gets louder near the bank.
    #[serde(default = "yes")]
    pub bank_lapping: bool,
    /// Gentle streams: every stream at its easiest.
    #[serde(default)]
    pub gentle: bool,
    #[serde(default)]
    pub tutorial_seen: bool,
}

fn normal() -> u8 {
    1
}

fn yes() -> bool {
    true
}

impl Progress {
    pub fn load(ctx: &mut Ctx<Parlor>) -> Progress {
        let mut p: Progress = Records::load(ctx, KEY);
        p.fix();
        p
    }

    pub fn peek(ctx: &Ctx<Parlor>) -> Progress {
        let mut p: Progress = Records::peek(ctx, KEY);
        p.fix();
        p
    }

    /// A fresh save has the paper boat.
    fn fix(&mut self) {
        if !self.boats.iter().any(|b| b == Boat::Paper.id()) {
            self.boats.insert(0, Boat::Paper.id().to_string());
        }
        if Boat::from_id(&self.boat).is_none() {
            self.boat = Boat::Paper.id().to_string();
        }
    }

    pub fn save(&self, ctx: &mut Ctx<Parlor>) {
        Records::store(ctx, KEY, self);
    }

    pub fn stars_for(&self, stream: Stream) -> u32 {
        self.stars
            .iter()
            .find(|(id, _)| id == stream.id())
            .map_or(0, |(_, n)| *n)
    }

    pub fn total_stars(&self) -> u32 {
        Stream::ALL.iter().map(|&s| self.stars_for(s)).sum()
    }

    pub fn is_open(&self, stream: Stream) -> bool {
        self.total_stars() >= stream.stars_to_open()
    }

    pub fn open_streams(&self) -> Vec<Stream> {
        Stream::ALL
            .into_iter()
            .filter(|&s| self.is_open(s))
            .collect()
    }

    /// Endless opens once Garden brook has a star.
    pub fn endless_open(&self) -> bool {
        self.stars_for(Stream::GardenBrook) > 0
    }

    /// Keeps `stars` if it beats the stream's best. True when it did.
    pub fn keep_stars(&mut self, stream: Stream, stars: u32) -> bool {
        let had = self.stars_for(stream);
        if stars <= had {
            return false;
        }
        self.stars.retain(|(id, _)| id != stream.id());
        self.stars.push((stream.id().to_string(), stars));
        true
    }

    pub fn boat(&self) -> Boat {
        Boat::from_id(&self.boat).unwrap_or(Boat::Paper)
    }

    pub fn has_boat(&self, boat: Boat) -> bool {
        self.boats.iter().any(|b| b == boat.id())
    }

    pub fn clean_width(&self) -> CleanWidth {
        CleanWidth::from_id(self.clean_width)
    }

    pub fn has_met(&self, kind: Kind) -> bool {
        self.met.iter().any(|k| k == kind.id())
    }

    pub fn meet(&mut self, kind: Kind) {
        if !self.has_met(kind) {
            self.met.push(kind.id().to_string());
        }
    }

    pub fn has_ribbon(&self, id: &str) -> bool {
        self.ribbons.iter().any(|r| r == id)
    }
}

/// A small goal on one stream, paid in paper scraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ribbon {
    pub id: &'static str,
    pub stream: Stream,
    pub name: &'static str,
}

/// Scraps each ribbon pays.
pub const RIBBON_SCRAPS: u32 = 10;

/// Three ribbons on every stream.
pub const RIBBONS: &[Ribbon] = &[
    Ribbon { id: "garden_catch", stream: Stream::GardenBrook, name: "Catch 30 things in one run" },
    Ribbon { id: "garden_clean", stream: Stream::GardenBrook, name: "Make 8 clean catches in one run" },
    Ribbon { id: "garden_dry", stream: Stream::GardenBrook, name: "Reach the mouth without a bump" },
    Ribbon { id: "gutter_catch", stream: Stream::RainGutter, name: "Catch 35 things in one run" },
    Ribbon { id: "gutter_clean", stream: Stream::RainGutter, name: "Make 10 clean catches in one run" },
    Ribbon { id: "gutter_falls", stream: Stream::RainGutter, name: "Go through every waterfall in one run" },
    Ribbon { id: "mill_catch", stream: Stream::MillRace, name: "Catch 40 things in one run" },
    Ribbon { id: "mill_clean", stream: Stream::MillRace, name: "Make 12 clean catches in one run" },
    Ribbon { id: "mill_wheels", stream: Stream::MillRace, name: "Pass every mill wheel in one run" },
    Ribbon { id: "canal_catch", stream: Stream::MarketCanal, name: "Catch 45 things in one run" },
    Ribbon { id: "canal_clean", stream: Stream::MarketCanal, name: "Make 14 clean catches in one run" },
    Ribbon { id: "canal_ducks", stream: Stream::MarketCanal, name: "Bump no ducks in one run" },
];

impl Ribbon {
    /// Whether a run that reached the mouth earned this ribbon.
    pub fn earned(&self, run: &Run) -> bool {
        if run.course.mode != crate::rules::Mode::Stream(self.stream) || !run.reached_mouth() {
            return false;
        }
        let n = |s: &str| s.split(' ').nth(1).and_then(|w| w.parse::<u32>().ok());
        match self.id {
            id if id.ends_with("_catch") => run.caught >= n(self.name).unwrap_or(u32::MAX),
            id if id.ends_with("_clean") => run.clean >= n(self.name).unwrap_or(u32::MAX),
            "garden_dry" => run.bumps == 0,
            "gutter_falls" => run.falls_total > 0 && run.falls_through == run.falls_total,
            "mill_wheels" => run.mills_total > 0 && run.mills_passed == run.mills_total,
            "canal_ducks" => run.ducks_total > 0 && run.ducks_bumped == 0,
            _ => false,
        }
    }
}

/// The ribbons of `stream`.
pub fn ribbons_of(stream: Stream) -> impl Iterator<Item = &'static Ribbon> {
    RIBBONS.iter().filter(move |r| r.stream == stream)
}

/// "best 540", or "no score yet".
pub fn best_line(ctx: &mut Ctx<Parlor>, board: &str) -> String {
    match Records::best(ctx, ID, board) {
        Some(b) => format!("best {b}"),
        None => "no score yet".to_string(),
    }
}
