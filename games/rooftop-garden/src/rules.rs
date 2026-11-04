//! The garden's rules: plants, water, pests, weather and the passing days.
//!
//! Everything here is plain data updated by `dt`, so tests can grow a
//! garden without a window. The screen turns the [`Note`]s the garden
//! leaves behind into speech and sound.

use parlor_core::{Period, Rng};
use serde::{Deserialize, Serialize};

use crate::cues;

/// Seconds of play in one garden day.
pub const DAY_SECONDS: f32 = 90.0;
/// Below this much water a plant is thirsty: its soil crackles and its
/// voice droops.
pub const THIRSTY: f32 = 0.3;
/// At this much water the soil is still moist from the last watering,
/// and more water is too much. A plant drinks down to this in about a
/// fifth of a day.
pub const MOIST: f32 = 0.7;
/// Above this much water the soil is waterlogged: the plant stops growing
/// and falls out of tune until the extra drains away.
pub const SOAKED: f32 = 1.0;
/// What one watering too many adds on top of soaked soil.
pub const OVERWATER: f32 = 0.35;
/// The most water a planter holds, waterlogged.
pub const MAX_WATER: f32 = SOAKED + 2.0 * OVERWATER;
/// Days a sprouted plant can sit in bone-dry soil before it wilts.
pub const WILT_AFTER: f32 = 0.25;
/// How long a rain shower lasts, in days.
pub const SHOWER_DAYS: f32 = 0.12;
/// How much faster time passes while you sit on the bench.
pub const BENCH_SPEED: f32 = 6.0;
/// Aphids per growing plant per day, before weather and twists.
pub const PEST_RATE: f32 = 0.35;
/// Leggy stems per blooming plant per day.
pub const LEGGY_RATE: f32 = 0.3;
/// Harvests that win a third row of planters, then a fourth column.
pub const THIRD_ROW_AT: u32 = 4;
pub const FOURTH_COLUMN_AT: u32 = 9;
/// The largest plot.
pub const MAX_ROWS: usize = 3;
pub const MAX_COLS: usize = 4;

/// What can grow in a planter. Each has a voice in the garden's song.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Species {
    Tomato,
    Squash,
    Bean,
    Sunflower,
    Lavender,
    Mint,
}

impl Species {
    pub const ALL: [Species; 6] = [
        Species::Tomato,
        Species::Squash,
        Species::Bean,
        Species::Sunflower,
        Species::Lavender,
        Species::Mint,
    ];

    pub fn index(self) -> usize {
        Species::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        match self {
            Species::Tomato => "tomato",
            Species::Squash => "squash",
            Species::Bean => "bean",
            Species::Sunflower => "sunflower",
            Species::Lavender => "lavender",
            Species::Mint => "mint",
        }
    }

    /// The part it plays once grown.
    pub fn voice(self) -> &'static str {
        match self {
            Species::Tomato => "a kalimba arpeggio",
            Species::Squash => "the bass",
            Species::Bean => "plucked strings",
            Species::Sunflower => "the flute melody",
            Species::Lavender => "the chimes",
            Species::Mint => "a cool, glassy pad",
        }
    }

    /// "tomatoes", for counts.
    pub fn plural(self) -> &'static str {
        match self {
            Species::Tomato => "tomatoes",
            Species::Squash => "squashes",
            Species::Bean => "bean harvests",
            Species::Sunflower => "sunflowers",
            Species::Lavender => "lavender bunches",
            Species::Mint => "mint sprigs",
        }
    }

    /// What a harvest gives you.
    pub fn harvest_text(self) -> &'static str {
        match self {
            Species::Tomato => "a ripe tomato",
            Species::Squash => "a fat squash",
            Species::Bean => "a handful of beans",
            Species::Sunflower => "a sunflower head full of seeds",
            Species::Lavender => "a bunch of lavender",
            Species::Mint => "a sprig of mint",
        }
    }

    /// How fast its soil dries, in full planters a day.
    pub fn thirst(self) -> f32 {
        match self {
            Species::Tomato => 1.6,
            Species::Squash => 1.5,
            Species::Bean => 1.3,
            Species::Sunflower => 1.2,
            Species::Lavender => 0.8,
            Species::Mint => 2.0,
        }
    }

    /// How fast it grows, against an ordinary plant.
    pub fn speed(self) -> f32 {
        match self {
            Species::Tomato => 1.0,
            Species::Squash => 0.9,
            Species::Bean => 1.2,
            Species::Sunflower => 0.9,
            Species::Lavender => 0.8,
            Species::Mint => 1.1,
        }
    }

    /// Harvests in your own garden before its seeds are in the shed.
    pub fn unlock_at(self) -> u32 {
        match self {
            Species::Tomato | Species::Squash | Species::Bean => 0,
            Species::Sunflower => 1,
            Species::Lavender => 3,
            Species::Mint => 6,
        }
    }

    /// A line of advice for the seed shed.
    pub fn tip(self) -> &'static str {
        match self {
            Species::Tomato => "Thirsty, and ripens again and again once it blooms.",
            Species::Squash => "Slow and steady; it carries the bass under everything.",
            Species::Bean => "The quickest grower, with a bouncy plucked part.",
            Species::Sunflower => "Tall and patient; it sings the melody.",
            Species::Lavender => "Happy in dry soil and slow to grow; it rings the chimes.",
            Species::Mint => "The thirstiest of all, with a cool pad over the top.",
        }
    }
}

/// How far along a plant is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Stage {
    Seed,
    Sprout,
    Growing,
    Blooming,
    Ripe,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Seed => "a seed",
            Stage::Sprout => "a sprout",
            Stage::Growing => "growing",
            Stage::Blooming => "in bloom",
            Stage::Ripe => "ripe",
        }
    }

    /// Days of good care to reach the next stage.
    pub fn days(self) -> f32 {
        match self {
            Stage::Seed => 0.4,
            Stage::Sprout => 0.6,
            Stage::Growing => 0.8,
            Stage::Blooming => 0.9,
            Stage::Ripe => f32::INFINITY,
        }
    }

    pub fn next(self) -> Stage {
        match self {
            Stage::Seed => Stage::Sprout,
            Stage::Sprout => Stage::Growing,
            Stage::Growing => Stage::Blooming,
            Stage::Blooming | Stage::Ripe => Stage::Ripe,
        }
    }
}

/// One plant in one planter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plant {
    pub species: Species,
    pub stage: Stage,
    /// Days into the current stage.
    pub growth: f32,
    /// 0 is bone dry, 1 freshly watered, above 1 waterlogged.
    pub water: f32,
    /// Days the soil has been bone dry.
    pub dry: f32,
    pub wilted: bool,
    pub pest: bool,
    pub leggy: bool,
    /// A pumpkin: a squash in every way but its name and its voice, a
    /// low bassoon. Only sown in pumpkin season.
    #[serde(default)]
    pub pumpkin: bool,
}

impl Plant {
    /// A pumpkin seed: it grows exactly as a squash does.
    pub fn pumpkin() -> Plant {
        Plant {
            pumpkin: true,
            ..Plant::new(Species::Squash)
        }
    }

    /// "pumpkin", or the species' name.
    pub fn name(&self) -> &'static str {
        if self.pumpkin {
            "pumpkin"
        } else {
            self.species.name()
        }
    }

    /// Its voice in the song: "a low bassoon" for a pumpkin, or the
    /// species' voice.
    pub fn voice(&self) -> &'static str {
        if self.pumpkin {
            "a low bassoon"
        } else {
            self.species.voice()
        }
    }

    /// The loop of its part in the song.
    pub fn song(&self) -> &'static str {
        if self.pumpkin {
            cues::PUMPKIN
        } else {
            cues::song(self.species)
        }
    }

    pub fn new(species: Species) -> Plant {
        Plant {
            species,
            stage: Stage::Seed,
            growth: 0.0,
            water: 0.6,
            dry: 0.0,
            wilted: false,
            pest: false,
            leggy: false,
            pumpkin: false,
        }
    }

    /// A plant already at `stage`, watered.
    pub fn at(species: Species, stage: Stage) -> Plant {
        Plant {
            stage,
            water: 0.9,
            ..Plant::new(species)
        }
    }

    pub fn thirsty(&self) -> bool {
        self.water < THIRSTY
    }

    /// Watered too much: its roots sit in soggy soil, and it stops growing
    /// until the extra drains away.
    pub fn waterlogged(&self) -> bool {
        self.water > SOAKED
    }

    /// Whether its part plays in the song.
    pub fn voiced(&self) -> bool {
        self.stage >= Stage::Growing && !self.wilted
    }

    /// Whether it counts toward the harmony: in bloom, well watered, well.
    pub fn in_tune(&self) -> bool {
        self.voiced() && self.stage >= Stage::Blooming && !self.thirsty() && !self.waterlogged()
    }

    /// "tomato, in bloom, thirsty, aphids".
    pub fn describe(&self) -> String {
        let mut parts = vec![self.name().to_string()];
        if self.wilted {
            parts.push("wilted".into());
        } else {
            parts.push(self.stage.name().into());
        }
        if self.water <= 0.0 {
            parts.push("bone dry".into());
        } else if self.thirsty() {
            parts.push("thirsty".into());
        } else if self.waterlogged() {
            parts.push("waterlogged".into());
        }
        if self.pest {
            parts.push("aphids".into());
        }
        if self.leggy {
            parts.push("leggy".into());
        }
        parts.join(", ")
    }
}

/// The day's weather.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weather {
    Sunny,
    Cloudy,
    Breezy,
    Showery,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::Sunny => "sunny",
            Weather::Cloudy => "cloudy",
            Weather::Breezy => "breezy",
            Weather::Showery => "showery",
        }
    }

    /// One sentence for the morning.
    pub fn describe(self) -> &'static str {
        match self {
            Weather::Sunny => "Sunny: soil dries fast.",
            Weather::Cloudy => "Cloudy: soil stays damp.",
            Weather::Breezy => "Breezy: fewer aphids.",
            Weather::Showery => "Showers on the way.",
        }
    }

    pub fn thirst(self) -> f32 {
        match self {
            Weather::Sunny => 1.4,
            Weather::Cloudy => 0.8,
            Weather::Breezy => 1.1,
            Weather::Showery => 0.9,
        }
    }

    pub fn pests(self) -> f32 {
        match self {
            Weather::Breezy => 0.5,
            Weather::Showery => 1.2,
            _ => 1.0,
        }
    }
}

/// Which garden this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Home,
    Daily,
    Weekly,
    Monthly,
}

impl Kind {
    pub fn of(period: Period) -> Kind {
        match period {
            Period::Daily => Kind::Daily,
            Period::Weekly => Kind::Weekly,
            Period::Monthly => Kind::Monthly,
        }
    }

    pub fn period(self) -> Option<Period> {
        match self {
            Kind::Home => None,
            Kind::Daily => Some(Period::Daily),
            Kind::Weekly => Some(Period::Weekly),
            Kind::Monthly => Some(Period::Monthly),
        }
    }
}

/// A challenge's modifier, announced before play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Twist {
    HeatWave,
    Drizzle,
    Aphids,
    GreenThumb,
    Festival,
}

impl Twist {
    /// The twists a daily or weekly can roll.
    pub const ROLLED: [Twist; 4] = [
        Twist::HeatWave,
        Twist::Drizzle,
        Twist::Aphids,
        Twist::GreenThumb,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::HeatWave => "Heat wave",
            Twist::Drizzle => "Drizzly days",
            Twist::Aphids => "Aphid season",
            Twist::GreenThumb => "Green thumb",
            Twist::Festival => "Rooftop festival",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Twist::HeatWave => "Every day is sunny and the soil dries almost twice as fast. No showers come.",
            Twist::Drizzle => "A rain shower comes every day, but the grey skies slow growing a little.",
            Twist::Aphids => "Aphids come far more often. Pick them off quickly with the tend key.",
            Twist::GreenThumb => "Everything grows half again as fast, and drinks a little more.",
            Twist::Festival => "The whole building is coming up for the party. Four planters start with sprouts already up, plants grow a quarter faster, and every one of the six seeds is yours to plant.",
        }
    }

    fn mods(self) -> Mods {
        let mut m = Mods::default();
        match self {
            Twist::HeatWave => {
                m.thirst = 1.8;
                m.showers = Showers::Never;
            }
            Twist::Drizzle => {
                m.growth = 0.9;
                m.showers = Showers::Daily;
            }
            Twist::Aphids => m.pests = 2.5,
            Twist::GreenThumb => {
                m.growth = 1.5;
                m.thirst = 1.2;
            }
            Twist::Festival => m.growth = 1.25,
        }
        m
    }
}

/// When showers come.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Showers {
    /// By the weather.
    #[default]
    Weather,
    Daily,
    Never,
}

/// Multipliers a twist sets.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mods {
    pub thirst: f32,
    pub growth: f32,
    pub pests: f32,
    /// How often plants in bloom get leggy.
    #[serde(default = "one")]
    pub leggy: f32,
    pub showers: Showers,
}

fn one() -> f32 {
    1.0
}

impl Default for Mods {
    fn default() -> Mods {
        Mods {
            thirst: 1.0,
            growth: 1.0,
            pests: 1.0,
            leggy: 1.0,
            showers: Showers::Weather,
        }
    }
}

/// A challenge's gentle goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Goal {
    /// This many different plants in bloom and well watered at once.
    Harmony(u32),
    /// Harvest this many of one plant.
    Harvest(Species, u32),
}

impl Goal {
    pub fn describe(self) -> String {
        match self {
            Goal::Harmony(n) => format!(
                "Grow a {}-part harmony: {n} different plants in bloom and well watered at the same time.",
                number(n)
            ),
            Goal::Harvest(s, n) => format!("Harvest {n} {}.", s.plural()),
        }
    }

    pub fn target(self) -> u32 {
        match self {
            Goal::Harmony(n) | Goal::Harvest(_, n) => n,
        }
    }
}

/// A small number as a word, for "a three-part harmony".
pub fn number(n: u32) -> String {
    match n {
        1 => "one".into(),
        2 => "two".into(),
        3 => "three".into(),
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        n => n.to_string(),
    }
}

/// What the garden asks the screen to do, and say.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// Speak now, interrupting.
    Say(String),
    /// Speak after whatever is being said.
    Queue(String),
    /// A cue from the centre.
    Cue(&'static str),
    /// A cue from a planter.
    At(&'static str, usize),
    /// A cue from just past the edge you walked into: rows down, planters right.
    Beyond(&'static str, i32, i32),
    /// A short piece of music over the garden.
    Stinger(&'static str),
    /// A new day began: a good moment to save.
    NewDay,
    /// The plot grew; every planter moved.
    Expanded,
    /// A challenge's goal is met.
    GoalMet,
    /// A challenge ran out of days.
    SeasonOver,
}

/// What the screen must ask the player before the garden can act.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    Nothing,
    /// Choose a seed for the empty planter under you.
    Seeds,
    /// Confirm pulling up the plant under you.
    Compost,
}

/// The player's controls by name, for spoken instructions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keys {
    pub arrows: String,
    pub water: String,
    pub tend: String,
    pub status: String,
    pub repeat: String,
    pub hint: String,
}

impl Keys {
    /// Defaults, for tests that run the rules alone.
    pub fn plain() -> Keys {
        Keys {
            arrows: "the arrow keys".into(),
            water: "Space".into(),
            tend: "Tab".into(),
            status: "S".into(),
            repeat: "R".into(),
            hint: "H".into(),
        }
    }
}

fn fresh_rng() -> Rng {
    Rng::from_time()
}

/// The garden: the planters, the time, the weather and the tallies.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Garden {
    pub kind: Kind,
    /// The challenge slot this garden belongs to ("weekly:2026-W42").
    #[serde(default)]
    pub slot: String,
    pub rows: usize,
    pub cols: usize,
    pub plots: Vec<Option<Plant>>,
    pub row: usize,
    pub col: usize,
    pub day: u32,
    /// How far through the day, 0 to 1.
    pub clock: f32,
    pub weather: Weather,
    pub shower_at: Option<f32>,
    pub rain_left: f32,
    pub harvests: u32,
    pub harvested: [u32; 6],
    pub pests_picked: u32,
    pub best_harmony: u32,
    /// The seeds you may plant.
    pub seeds: Vec<Species>,
    pub mods: Mods,
    pub twist: Option<Twist>,
    pub goal: Option<Goal>,
    pub day_limit: Option<u32>,
    pub goal_met: bool,
    /// Whether the whole six-part chorus has been heard in this garden.
    pub chorus: bool,
    #[serde(skip)]
    pub finished: bool,
    #[serde(skip)]
    pub sitting: bool,
    #[serde(skip)]
    pub notes: Vec<Note>,
    #[serde(skip)]
    pub last: String,
    #[serde(skip)]
    pub keys: Keys,
    #[serde(skip, default = "fresh_rng")]
    pub rng: Rng,
    /// Pumpkin seeds are in the packet: your own garden, in their season.
    #[serde(skip)]
    pub pumpkin_season: bool,
    /// The Sunday bells ring this evening, in your own garden.
    #[serde(skip)]
    pub bells: bool,
    #[serde(skip)]
    bells_named: bool,
}

impl Garden {
    fn blank(kind: Kind, rows: usize, cols: usize, rng: Rng) -> Garden {
        Garden {
            kind,
            slot: String::new(),
            rows,
            cols,
            plots: vec![None; rows * cols],
            row: 0,
            col: 0,
            day: 1,
            clock: 0.05,
            weather: Weather::Sunny,
            shower_at: None,
            rain_left: 0.0,
            harvests: 0,
            harvested: [0; 6],
            pests_picked: 0,
            best_harmony: 0,
            seeds: vec![Species::Tomato, Species::Squash, Species::Bean],
            mods: Mods::default(),
            twist: None,
            goal: None,
            day_limit: None,
            goal_met: false,
            chorus: false,
            finished: false,
            sitting: false,
            notes: Vec::new(),
            last: String::new(),
            keys: Keys::plain(),
            rng,
            pumpkin_season: false,
            bells: false,
            bells_named: false,
        }
    }

    /// Your own rooftop on the first day: two rows of three planters, a
    /// squash already growing and a tomato seed just sown.
    pub fn home(rng: Rng) -> Garden {
        let mut g = Garden::blank(Kind::Home, 2, 3, rng);
        // The squash starts as moist as a new seed, so a first Space
        // waters it instead of waterlogging it.
        let mut squash = Plant::at(Species::Squash, Stage::Growing);
        squash.water = Plant::new(Species::Squash).water;
        g.plots[0] = Some(squash);
        g.plots[1] = Some(Plant::new(Species::Tomato));
        g.weather = Weather::Cloudy;
        g
    }

    /// A challenge garden, built from the challenge's own seed.
    pub fn challenge(period: Period, slot: String, mut rng: Rng) -> Garden {
        let mut g = match period {
            Period::Daily => {
                let mut g = Garden::blank(Kind::Daily, 2, 3, Rng::seeded(0));
                g.twist = Some(*rng.pick(&Twist::ROLLED));
                g.seeds = vec![
                    Species::Tomato,
                    Species::Squash,
                    Species::Bean,
                    Species::Sunflower,
                ];
                g.goal = Some(Goal::Harmony(3));
                g.day_limit = Some(4);
                g
            }
            Period::Weekly => {
                let mut g = Garden::blank(Kind::Weekly, 3, 3, Rng::seeded(0));
                g.twist = Some(*rng.pick(&Twist::ROLLED));
                g.seeds = Species::ALL[..5].to_vec();
                g.goal = Some(Goal::Harvest(Species::Tomato, 5));
                g.day_limit = Some(8);
                g
            }
            Period::Monthly => {
                let mut g = Garden::blank(Kind::Monthly, 3, 4, Rng::seeded(0));
                g.twist = Some(Twist::Festival);
                g.seeds = Species::ALL.to_vec();
                g.goal = Some(Goal::Harmony(6));
                g.day_limit = Some(10);
                let mut kinds = Species::ALL.to_vec();
                rng.shuffle(&mut kinds);
                let corners = [0, 3, 8, 11];
                for (i, &plot) in corners.iter().enumerate() {
                    g.plots[plot] = Some(Plant::at(kinds[i], Stage::Sprout));
                }
                g
            }
        };
        g.slot = slot;
        g.mods = g.twist.map(Twist::mods).unwrap_or_default();
        g.rng = rng;
        g.roll_weather();
        g
    }

    /// Mends a saved garden that does not add up (a hand-edited or damaged
    /// save), so it can never index outside its planters.
    pub fn repair(&mut self) {
        self.rows = self.rows.clamp(1, MAX_ROWS);
        self.cols = self.cols.clamp(1, MAX_COLS);
        self.plots.resize(self.rows * self.cols, None);
        self.row = self.row.min(self.rows - 1);
        self.col = self.col.min(self.cols - 1);
        self.clock = if self.clock.is_finite() {
            self.clock.clamp(0.0, 1.0)
        } else {
            0.05
        };
        self.day = self.day.max(1);
        if self.seeds.is_empty() {
            self.seeds = vec![Species::Tomato, Species::Squash, Species::Bean];
        }
        for p in self.plots.iter_mut().flatten() {
            if !p.water.is_finite() {
                p.water = 0.6;
            }
            p.water = p.water.clamp(0.0, MAX_WATER);
            if !p.growth.is_finite() || p.growth < 0.0 {
                p.growth = 0.0;
            }
        }
    }

    // -- the plot ---------------------------------------------------------

    pub fn index(&self, row: usize, col: usize) -> usize {
        row * self.cols + col
    }

    pub fn here(&self) -> usize {
        self.index(self.row, self.col)
    }

    /// (row, col) of a planter.
    pub fn cell(&self, i: usize) -> (usize, usize) {
        (i / self.cols, i % self.cols)
    }

    /// "row 1, planter 2".
    pub fn place(&self, i: usize) -> String {
        let (r, c) = self.cell(i);
        format!("row {}, planter {}", r + 1, c + 1)
    }

    pub fn plant_here(&self) -> Option<&Plant> {
        self.plots[self.here()].as_ref()
    }

    /// The plants with their planters.
    pub fn plants(&self) -> impl Iterator<Item = (usize, &Plant)> {
        self.plots
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.as_ref().map(|p| (i, p)))
    }

    /// Grows the plot, keeping every plant in its row and column.
    pub fn expand(&mut self, rows: usize, cols: usize) {
        let rows = rows.clamp(self.rows, MAX_ROWS);
        let cols = cols.clamp(self.cols, MAX_COLS);
        let mut plots = vec![None; rows * cols];
        for (i, p) in self.plots.drain(..).enumerate() {
            let (r, c) = (i / self.cols, i % self.cols);
            plots[r * cols + c] = p;
        }
        // Cues already waiting still come from the same planter.
        let old = self.cols;
        for note in self.notes.iter_mut() {
            if let Note::At(_, i) = note {
                *i = (*i / old) * cols + *i % old;
            }
        }
        self.plots = plots;
        self.rows = rows;
        self.cols = cols;
        self.notes.push(Note::Expanded);
    }

    /// The different plants in tune right now, in seed order.
    pub fn harmony_parts(&self) -> Vec<Species> {
        Species::ALL
            .into_iter()
            .filter(|&s| self.plants().any(|(_, p)| p.species == s && p.in_tune()))
            .collect()
    }

    /// A harmony part's name: the squash's part is "pumpkin" when only
    /// pumpkins are singing it.
    pub fn part_name(&self, part: Species) -> &'static str {
        let singers: Vec<&Plant> = self
            .plants()
            .map(|(_, p)| p)
            .filter(|p| p.species == part && p.in_tune())
            .collect();
        if !singers.is_empty() && singers.iter().all(|p| p.pumpkin) {
            "pumpkin"
        } else {
            part.name()
        }
    }

    pub fn harmony(&self) -> u32 {
        self.harmony_parts().len() as u32
    }

    /// The goal's progress: (now, target).
    pub fn progress(&self) -> Option<(u32, u32)> {
        let goal = self.goal?;
        let now = match goal {
            Goal::Harmony(_) => self
                .harmony()
                .max(if self.goal_met { goal.target() } else { 0 }),
            Goal::Harvest(s, _) => self.harvested[s.index()],
        };
        Some((now.min(goal.target()), goal.target()))
    }

    /// The challenge score: gentle, and higher is better.
    pub fn score(&self) -> u64 {
        let mut score =
            5 * self.harvests as u64 + 2 * self.pests_picked as u64 + 5 * self.best_harmony as u64;
        if self.goal_met {
            let limit = self.day_limit.unwrap_or(self.day);
            score += 100 + 10 * limit.saturating_sub(self.day) as u64;
        }
        score
    }

    /// "morning", "midday", "afternoon", "evening".
    pub fn phase(&self) -> &'static str {
        match self.clock {
            c if c < 0.3 => "morning",
            c if c < 0.55 => "midday",
            c if c < 0.8 => "afternoon",
            _ => "evening",
        }
    }

    pub fn raining(&self) -> bool {
        self.rain_left > 0.0
    }

    // -- speech -------------------------------------------------------------

    fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        self.notes.push(Note::Say(text));
    }

    fn queue(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        self.notes.push(Note::Queue(text));
    }

    fn cue(&mut self, key: &'static str) {
        self.notes.push(Note::Cue(key));
    }

    fn at(&mut self, key: &'static str, i: usize) {
        self.notes.push(Note::At(key, i));
    }

    /// Something needs you: it also gets you up off the bench.
    fn need(&mut self, key: &'static str, i: usize, text: String) {
        self.at(key, i);
        self.queue(text);
        if self.sitting {
            self.sitting = false;
            self.cue(cues::STAND);
            self.queue("You get up.");
        }
    }

    /// Takes what the garden has to say.
    pub fn drain_notes(&mut self) -> Vec<Note> {
        std::mem::take(&mut self.notes)
    }

    /// The planter under you, as a sentence.
    pub fn describe_here(&self) -> String {
        self.describe(self.here())
    }

    pub fn describe(&self, i: usize) -> String {
        let place = sentence(&self.place(i));
        match &self.plots[i] {
            Some(p) => format!("{place}: {}.", p.describe()),
            None => format!("{place}: empty soil."),
        }
    }

    /// The opening line.
    pub fn intro(&self) -> String {
        let mut s = match self.kind {
            Kind::Home => format!(
                "Your garden, {} rows of {} planters. Day {}, {}, {}.",
                self.rows,
                self.cols,
                self.day,
                self.phase(),
                self.weather.name()
            ),
            _ => {
                let twist = self.twist.map(|t| t.name()).unwrap_or("Challenge");
                format!(
                    "{twist}. {} Day {} of {}, {}.",
                    self.goal.map(|g| g.describe()).unwrap_or_default(),
                    self.day,
                    self.day_limit.unwrap_or(self.day),
                    self.weather.name()
                )
            }
        };
        s.push_str(&format!(
            " {} {} for status, {} for a hint, F1 for controls.",
            self.describe_here(),
            sentence(&self.keys.status),
            self.keys.hint
        ));
        s
    }

    /// The status key, in layers. First: what needs you, any rain, and
    /// the planter under you. A second press: the day and weather, the
    /// harmony, harvests and any goal.
    pub fn status_layers(&self) -> Vec<String> {
        let count = |f: &dyn Fn(&Plant) -> bool| self.plants().filter(|(_, p)| f(p)).count();
        let mut needs = Vec::new();
        let n = count(&|p| p.wilted);
        if n > 0 {
            needs.push(format!("{n} wilted"));
        }
        let n = count(&|p| p.thirsty() && !p.wilted);
        if n > 0 {
            needs.push(format!("{n} thirsty"));
        }
        let n = count(&|p| p.waterlogged());
        if n > 0 {
            needs.push(format!("{n} waterlogged"));
        }
        let n = count(&|p| p.pest);
        if n > 0 {
            needs.push(format!("{n} with aphids"));
        }
        let n = count(&|p| p.stage == Stage::Ripe);
        if n > 0 {
            needs.push(format!("{n} ready to harvest"));
        }
        let n = count(&|p| p.leggy);
        if n > 0 {
            needs.push(format!("{n} leggy"));
        }
        let mut first = String::new();
        if needs.is_empty() {
            first.push_str("All content.");
        } else {
            first.push_str(&format!("{}.", sentence(&list(&needs))));
        }
        if self.raining() {
            first.push_str(" Raining.");
        }
        first.push(' ');
        first.push_str(&self.describe_here());

        let mut s = format!(
            "Day {}, {}, {}.",
            self.day,
            self.phase(),
            self.weather.name()
        );
        let parts = self.harmony_parts();
        if parts.is_empty() {
            s.push_str(" No harmony yet.");
        } else {
            let names: Vec<_> = parts.iter().map(|p| self.part_name(*p)).collect();
            s.push_str(&format!(
                " Harmony: {} {}, {}.",
                parts.len(),
                if parts.len() == 1 { "part" } else { "parts" },
                list(&names)
            ));
        }
        s.push_str(&format!(
            " {} {}.",
            self.harvests,
            if self.harvests == 1 {
                "harvest"
            } else {
                "harvests"
            }
        ));
        if let (Some(goal), Some((now, target))) = (self.goal, self.progress()) {
            let what = match goal {
                Goal::Harmony(_) => "harmony parts".to_string(),
                Goal::Harvest(sp, _) => sp.plural().to_string(),
            };
            s.push_str(&format!(
                " Goal: {now} of {target} {what}, day {} of {}.",
                self.day,
                self.day_limit.unwrap_or(self.day)
            ));
        }
        if self.sitting {
            s.push_str(" You are sitting on the bench.");
        }
        if self.bells {
            s.push_str(" The church bells ring for evensong this evening.");
        }
        if self.pumpkin_season {
            s.push_str(" Pumpkin seeds are in the packet until Thanksgiving.");
        }
        vec![first, s]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }

    /// Every planter, row by row: L.
    pub fn list(&self) -> String {
        let mut rows = Vec::new();
        for r in 0..self.rows {
            let mut cells = Vec::new();
            for c in 0..self.cols {
                let i = self.index(r, c);
                cells.push(match &self.plots[i] {
                    Some(p) => format!("{} {}", c + 1, p.describe()),
                    None => format!("{} empty", c + 1),
                });
            }
            rows.push(format!("Row {}: {}.", r + 1, cells.join("; ")));
        }
        rows.join(" ")
    }

    /// The most pressing job, and where: H.
    pub fn hint(&self) -> String {
        let jobs: [Job; 5] = [
            (
                &|p| p.wilted,
                "has wilted",
                format!("{} waters it back to life.", sentence(&self.keys.water)),
            ),
            (
                &|p| p.pest,
                "has aphids",
                format!("{} picks them off.", sentence(&self.keys.tend)),
            ),
            (
                &|p| p.thirsty(),
                "is thirsty",
                format!("{} waters it.", sentence(&self.keys.water)),
            ),
            (
                &|p| p.stage == Stage::Ripe,
                "is ready to harvest",
                format!("{} harvests it.", sentence(&self.keys.tend)),
            ),
            (
                &|p| p.leggy,
                "is leggy",
                format!("{} prunes it.", sentence(&self.keys.tend)),
            ),
        ];
        for (test, what, how) in jobs.iter() {
            if let Some(i) = self.nearest(|p| p.as_ref().is_some_and(test)) {
                let name = self.plots[i].as_ref().map(|p| p.name()).unwrap_or("");
                return format!("{} {how}", self.pointer(i, &format!("the {name}"), what));
            }
        }
        if let Some(i) = self.nearest(|p| p.is_none()) {
            return format!(
                "{} {} plants a seed there.",
                self.pointer(i, "an empty planter", "is waiting"),
                sentence(&self.keys.water)
            );
        }
        if let Some(i) = self.nearest(|p| p.as_ref().is_some_and(|p| p.water < 0.6)) {
            let name = self.plots[i].as_ref().map(|p| p.name()).unwrap_or("");
            return format!(
                "All content. {}",
                self.pointer(i, &format!("the {name}"), "needs water soon")
            );
        }
        "All content. Enter sits on the bench to pass time.".to_string()
    }

    /// "The squash 1 up and 2 to the right has aphids." or "Right here, the squash has aphids."
    fn pointer(&self, i: usize, who: &str, what: &str) -> String {
        let (r, c) = self.cell(i);
        let dr = r as i32 - self.row as i32;
        let dc = c as i32 - self.col as i32;
        let mut way = Vec::new();
        if dr != 0 {
            way.push(format!(
                "{} {}",
                dr.abs(),
                if dr < 0 { "up" } else { "down" }
            ));
        }
        if dc != 0 {
            way.push(format!(
                "{} to the {}",
                dc.abs(),
                if dc < 0 { "left" } else { "right" }
            ));
        }
        let what = if what.is_empty() {
            String::new()
        } else {
            format!(" {what}")
        };
        if way.is_empty() {
            format!("Right here, {who}{what}.")
        } else {
            format!("{} {}{what}.", sentence(who), way.join(" and "))
        }
    }

    fn nearest(&self, test: impl Fn(&Option<Plant>) -> bool) -> Option<usize> {
        (0..self.plots.len())
            .filter(|&i| test(&self.plots[i]))
            .min_by_key(|&i| {
                let (r, c) = self.cell(i);
                (r as i32 - self.row as i32).abs() + (c as i32 - self.col as i32).abs()
            })
    }

    // -- actions ----------------------------------------------------------

    fn stand(&mut self) {
        if self.sitting {
            self.sitting = false;
            self.cue(cues::STAND);
        }
    }

    /// Steps one planter: Up and Down change row, Left and Right planter.
    pub fn step(&mut self, dr: i32, dc: i32) {
        self.stand();
        let r = self.row as i32 + dr;
        let c = self.col as i32 + dc;
        if r < 0 || c < 0 || r >= self.rows as i32 || c >= self.cols as i32 {
            self.notes.push(Note::Beyond(cues::RAILING, dr, dc));
            let side = match (dr, dc) {
                (-1, _) => "far",
                (1, _) => "near",
                (_, -1) => "left",
                _ => "right",
            };
            self.say(format!("Railing, {side} side."));
            return;
        }
        self.row = r as usize;
        self.col = c as usize;
        self.cue(cues::STEP);
        if self.plant_here().is_none() {
            self.cue(cues::EMPTY);
        }
        let text = self.describe_here();
        self.say(text);
    }

    /// Space: water the plant here, or ask for a seed for empty soil.
    pub fn water(&mut self) -> Ask {
        self.stand();
        let i = self.here();
        let Some(p) = self.plots[i].as_mut() else {
            return Ask::Seeds;
        };
        let name = p.name();
        if p.water >= MOIST && !p.wilted {
            // Too much: the soil waterlogs, and the plant stops growing
            // until the extra drains. More on top keeps it soggy longer.
            let already = p.waterlogged();
            p.water = (p.water.max(SOAKED) + OVERWATER).min(MAX_WATER);
            self.at(cues::SQUELCH, i);
            if already {
                self.say("Still waterlogged.");
            } else {
                self.say("Too much water. Waterlogged.");
            }
            return Ask::Nothing;
        }
        p.water = 1.0;
        p.dry = 0.0;
        if p.wilted {
            p.wilted = false;
            self.at(cues::WATER, i);
            self.at(cues::REVIVE, i);
            self.say(format!("{} revived.", sentence(name)));
        } else {
            self.at(cues::WATER, i);
            self.say("Watered.");
        }
        Ask::Nothing
    }

    /// Tab: pick pests, harvest, or prune, whichever is needed most.
    pub fn tend(&mut self) {
        self.stand();
        let i = self.here();
        let Some(p) = self.plots[i].as_mut() else {
            self.cue(cues::NO);
            let water = sentence(&self.keys.water);
            self.say(format!("Empty soil. {water} plants a seed."));
            return;
        };
        if p.pest {
            p.pest = false;
            self.pests_picked += 1;
            self.at(cues::PICK, i);
            self.say("Aphids picked off.");
        } else if p.stage == Stage::Ripe {
            self.harvest(i);
        } else if p.leggy {
            p.leggy = false;
            self.at(cues::PRUNE, i);
            self.say("Pruned.");
        } else if p.wilted {
            self.at(cues::RUSTLE, i);
            let water = self.keys.water.clone();
            self.say(format!("Wilted. {} revives it.", sentence(&water)));
        } else if p.stage == Stage::Seed {
            self.at(cues::RUSTLE, i);
            self.say("A seed. Nothing to tend yet.");
        } else {
            let thirst = if p.thirsty() { " It is thirsty." } else { "" };
            self.at(cues::RUSTLE, i);
            self.say(format!("Nothing to tend.{thirst}"));
        }
    }

    fn harvest(&mut self, i: usize) {
        let Some(p) = self.plots[i].as_mut() else {
            return;
        };
        let species = p.species;
        let what = if p.pumpkin {
            "a round orange pumpkin"
        } else {
            species.harvest_text()
        };
        p.stage = Stage::Blooming;
        p.growth = 0.0;
        p.leggy = false;
        self.harvests += 1;
        self.harvested[species.index()] += 1;
        self.at(cues::HARVEST, i);
        self.notes.push(Note::Stinger(cues::MUSIC_HARVEST));
        let n = self.harvests;
        self.say(format!("{}. {n} in all.", sentence(what)));
        if self.kind == Kind::Home {
            self.unlocks();
        }
    }

    /// New seeds and planters, earned by harvests in your own garden.
    pub fn unlocks(&mut self) {
        for s in Species::ALL {
            if self.harvests >= s.unlock_at() && !self.seeds.contains(&s) {
                self.seeds.push(s);
                self.cue(cues::UNLOCK);
                self.queue(format!("New seed: {}, {}.", s.name(), s.voice()));
            }
        }
        if self.harvests >= THIRD_ROW_AT && self.rows < 3 {
            self.expand(3, self.cols);
            self.cue(cues::EXPAND);
            self.queue("A third row of planters, nearest you.");
        }
        if self.harvests >= FOURTH_COLUMN_AT && self.cols < 4 {
            self.expand(self.rows, 4);
            self.cue(cues::EXPAND);
            self.queue("A fourth planter on the right of every row.");
        }
    }

    /// Plants a seed in the empty planter under you.
    pub fn sow(&mut self, species: Species) {
        let i = self.here();
        if self.plots[i].is_some() {
            self.cue(cues::NO);
            self.say("Something is already growing here.");
            return;
        }
        self.plots[i] = Some(Plant::new(species));
        self.at(cues::PLANT, i);
        self.say(format!("{} planted.", sentence(species.name())));
    }

    /// Plants a pumpkin seed in the empty planter under you.
    pub fn sow_pumpkin(&mut self) {
        let i = self.here();
        if self.plots[i].is_some() {
            self.cue(cues::NO);
            self.say("Something is already growing here.");
            return;
        }
        self.plots[i] = Some(Plant::pumpkin());
        self.at(cues::PLANT, i);
        self.say("Pumpkin planted.");
    }

    /// Whether there is a plant here to pull up.
    pub fn ask_compost(&mut self) -> Ask {
        self.stand();
        if self.plant_here().is_some() {
            Ask::Compost
        } else {
            self.cue(cues::NO);
            self.say("Nothing to pull up here.");
            Ask::Nothing
        }
    }

    /// Pulls up the plant here.
    pub fn compost(&mut self) {
        let i = self.here();
        if let Some(p) = self.plots[i].take() {
            self.at(cues::COMPOST, i);
            self.say(format!("{} composted.", sentence(p.name())));
        }
    }

    /// Enter: sit on the bench, or get up.
    pub fn bench(&mut self) {
        if self.sitting {
            self.sitting = false;
            self.cue(cues::STAND);
            self.say("You get up.");
        } else {
            self.sitting = true;
            self.cue(cues::BENCH);
            self.say(format!(
                "On the bench. Time passes {} times faster.",
                BENCH_SPEED as u32
            ));
        }
    }

    /// Repeats the last thing said: R.
    pub fn repeat(&mut self) {
        if self.last.is_empty() {
            self.notes.push(Note::Say("Nothing to repeat yet.".into()));
        } else {
            self.notes.push(Note::Say(self.last.clone()));
        }
    }

    pub fn speak_hint(&mut self) {
        let s = self.hint();
        self.say(s);
    }

    pub fn speak_list(&mut self) {
        let s = self.list();
        self.say(s);
    }

    // -- time -------------------------------------------------------------

    /// Lets `seconds` of play pass.
    pub fn update(&mut self, seconds: f32) {
        let speed = if self.sitting { BENCH_SPEED } else { 1.0 };
        let mut left = seconds * speed;
        while left > 0.0 && !self.finished {
            let step = left.min(0.5);
            self.tick(step / DAY_SECONDS);
            left -= step;
        }
    }

    /// Lets whole days pass at once, for tests.
    pub fn pass_days(&mut self, days: f32) {
        self.update(days * DAY_SECONDS);
    }

    fn roll_weather(&mut self) {
        self.weather = match self.mods.showers {
            Showers::Never => Weather::Sunny,
            _ => match self.rng.below(10) {
                0..=2 => Weather::Sunny,
                3..=5 => Weather::Cloudy,
                6..=7 => Weather::Breezy,
                _ => Weather::Showery,
            },
        };
        let shower = match self.mods.showers {
            Showers::Never => false,
            Showers::Daily => true,
            Showers::Weather => match self.weather {
                Weather::Showery => true,
                Weather::Cloudy => self.rng.chance(0.3),
                _ => false,
            },
        };
        self.shower_at = shower.then(|| 0.2 + self.rng.unit() as f32 * 0.5);
    }

    fn tick(&mut self, days: f32) {
        // Showers.
        if let Some(at) = self.shower_at {
            if self.clock >= at && !self.raining() {
                self.shower_at = None;
                self.rain_left = SHOWER_DAYS;
                self.cue(cues::SHOWER);
                self.queue("Rain shower. It waters every planter.");
            }
        }
        let raining = self.raining();
        if raining {
            self.rain_left -= days;
            if self.rain_left <= 0.0 {
                self.rain_left = 0.0;
                self.cue(cues::CLEAR);
                self.queue("The rain stops.");
            }
        }
        let thirst = self.weather.thirst() * self.mods.thirst;
        let pests = PEST_RATE * self.weather.pests() * self.mods.pests;
        let growth = self.mods.growth;
        for i in 0..self.plots.len() {
            let Some(mut p) = self.plots[i].take() else {
                continue;
            };
            let name = p.name();
            let place = self.place(i);
            let was_thirsty = p.thirsty();
            let was_soggy = p.waterlogged();
            if raining {
                // Rain fills dry soil, but never drains a waterlogged one.
                p.water = p.water.max((p.water + days * 8.0).min(SOAKED));
                p.dry = 0.0;
                if p.wilted && !p.thirsty() {
                    p.wilted = false;
                    self.at(cues::REVIVE, i);
                    self.queue(format!("Rain revived the {name}, {place}."));
                }
            } else {
                p.water = (p.water - p.species.thirst() * thirst * days).max(0.0);
            }
            if was_soggy && !p.waterlogged() {
                self.at(cues::DRIP, i);
                self.queue(format!("{} drained, {place}.", sentence(name)));
            }
            // One meaning of thirsty everywhere: the dry-soil crackle, S, H
            // and this call all count a seed too, since a dry seed grows
            // at half speed.
            if !was_thirsty && p.thirsty() {
                let what = if p.stage == Stage::Seed {
                    format!("{name} seed")
                } else {
                    name.to_string()
                };
                self.need(
                    cues::THIRSTY,
                    i,
                    format!("{} thirsty, {place}.", sentence(&what)),
                );
            }
            if p.water <= 0.0 {
                p.dry += days;
                if p.dry >= WILT_AFTER && !p.wilted && p.stage >= Stage::Sprout {
                    p.wilted = true;
                    p.pest = false;
                    let water = self.keys.water.clone();
                    self.need(
                        cues::WILT,
                        i,
                        format!("{} wilted, {place}. {water} revives it.", sentence(name)),
                    );
                }
            } else {
                p.dry = 0.0;
            }
            if !p.wilted && !p.pest && !p.waterlogged() && p.water > 0.0 && p.stage != Stage::Ripe {
                let mut rate = p.species.speed() * growth;
                if p.thirsty() {
                    rate *= 0.5;
                }
                if p.leggy {
                    rate *= 0.5;
                }
                p.growth += days * rate;
                if p.growth >= p.stage.days() {
                    p.growth = 0.0;
                    p.stage = p.stage.next();
                    self.grew(i, &p);
                }
            }
            if p.stage >= Stage::Growing
                && !p.pest
                && !p.wilted
                && self.rng.chance((pests * days) as f64)
            {
                p.pest = true;
                let tend = self.keys.tend.clone();
                self.need(
                    cues::PEST_ARRIVE,
                    i,
                    format!("Aphids on the {name}, {place}. {tend} picks them off."),
                );
            }
            if p.stage == Stage::Blooming
                && !p.leggy
                && !p.wilted
                && self
                    .rng
                    .chance((LEGGY_RATE * self.mods.leggy * days) as f64)
            {
                p.leggy = true;
                let tend = self.keys.tend.clone();
                self.need(
                    cues::LEGGY,
                    i,
                    format!("{} leggy, {place}. {tend} prunes it.", sentence(name)),
                );
            }
            self.plots[i] = Some(p);
        }
        // Harmony.
        let h = self.harmony();
        if h > self.best_harmony {
            self.best_harmony = h;
            if h >= 2 {
                let names: Vec<_> = self
                    .harmony_parts()
                    .iter()
                    .map(|s| self.part_name(*s))
                    .collect();
                self.queue(format!("{}-part harmony: {}.", number(h), list(&names)));
            }
        }
        if h as usize == Species::ALL.len() && !self.chorus {
            self.chorus = true;
            self.notes.push(Note::Stinger(cues::MUSIC_CHORUS));
            self.queue("Full chorus: every voice in tune.");
        }
        // The goal.
        if let Some((now, target)) = self.progress() {
            if !self.goal_met && now >= target {
                self.goal_met = true;
                self.finished = true;
                self.notes.push(Note::GoalMet);
                return;
            }
        }
        // The day.
        let before = self.clock;
        self.clock += days;
        if before < 0.8 && self.clock >= 0.8 {
            if self.bells {
                self.cue(cues::SUNDAY_BELLS);
                if self.bells_named {
                    self.queue("Evening, with the Sunday bells.");
                } else {
                    self.bells_named = true;
                    self.queue("Evening. The church down the street rings its bells for evensong.");
                }
            } else {
                self.cue(cues::DUSK);
                self.queue("Evening.");
            }
        }
        if self.clock >= 1.0 {
            self.clock -= 1.0;
            self.day += 1;
            if let Some(limit) = self.day_limit {
                if self.day > limit {
                    self.day = limit;
                    self.clock = 1.0;
                    self.finished = true;
                    self.notes.push(Note::SeasonOver);
                    return;
                }
            }
            self.roll_weather();
            self.cue(cues::DAWN);
            let mut text = format!("Day {}. {}", self.day, self.weather.describe());
            if let Some(limit) = self.day_limit {
                text.push_str(&format!(" Day {} of {limit}.", self.day));
            }
            self.queue(text);
            self.notes.push(Note::NewDay);
        }
    }

    fn grew(&mut self, i: usize, p: &Plant) {
        let name = p.name();
        let place = self.place(i);
        match p.stage {
            Stage::Seed => {}
            Stage::Sprout => {
                self.at(cues::SPROUT, i);
                self.queue(format!("{} sprouted, {place}.", sentence(name)));
            }
            Stage::Growing => {
                self.at(cues::GROW, i);
                self.queue(format!(
                    "{} growing, {place}: {}.",
                    sentence(name),
                    p.voice()
                ));
            }
            Stage::Blooming => {
                self.at(cues::BLOOM, i);
                self.queue(format!("{} in bloom, {place}.", sentence(name)));
            }
            Stage::Ripe => {
                let tend = self.keys.tend.clone();
                self.need(
                    cues::RIPE,
                    i,
                    format!("{} ripe, {place}. {tend} harvests it.", sentence(name)),
                );
            }
        }
    }
}

/// A job for the hint: which plants need it, what is wrong, what fixes it.
type Job<'a> = (&'a dyn Fn(&Plant) -> bool, &'static str, String);

/// "a, b and c".
pub fn list<S: AsRef<str>>(items: &[S]) -> String {
    match items {
        [] => String::new(),
        [one] => one.as_ref().to_string(),
        [rest @ .., last] => format!(
            "{} and {}",
            rest.iter()
                .map(|s| s.as_ref())
                .collect::<Vec<_>>()
                .join(", "),
            last.as_ref()
        ),
    }
}

/// Capitalises the first letter.
pub fn sentence(text: &str) -> String {
    let mut c = text.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
