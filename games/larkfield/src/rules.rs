//! The hamlet itself: the lane and its places, the fields, the larder,
//! the settlers and their day. No sound or speech here: everything that
//! happens goes out as an [`Event`] for the screen to sound and say, so
//! every rule can be tested alone.

use std::collections::BTreeMap;

use parlor_core::Rng;
use serde::{Deserialize, Serialize};

use crate::almanac::{self, Pace, Trouble};
use crate::buildings::{good, good_name, Building, FURNITURE, GOODS, NAILS};
use crate::crops::{crop, of_season, Crop, Season};
use crate::people::{Needs, Skill, Trade, Trait, INSTRUMENTS, NAMES, WANTS};

/// Days in a season.
pub const SEASON_DAYS: u32 = 10;
/// The most settlers the first hamlet holds.
pub const MAX_SETTLERS: usize = 6;
/// Hours from the morning bell to supper.
pub const DAY_HOURS: f32 = 12.0;
/// The first hour of the day, on the clock.
pub const FIRST_HOUR: f32 = 7.0;
/// Rows and plots in the field.
pub const ROWS: usize = 3;
pub const COLS: usize = 4;
/// Strikes of the axe or the pick for one wood or one stone.
pub const STRIKES: u32 = 3;
/// Fish settlers can catch in a day before the river is fished out.
pub const FISH_A_DAY: u32 = 6;
/// Food one person eats at supper, in tenths of a day's food.
pub const MEAL: u32 = 6;
/// Things to forage in the woods each day.
pub const FORAGE_A_DAY: u32 = 3;

/// The places along the lane, from the gate down to the woods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Place {
    Gate,
    Cottages,
    Green,
    Fields,
    Yard,
    River,
    Woods,
}

impl Place {
    pub const ALL: [Place; 7] = [
        Place::Gate,
        Place::Cottages,
        Place::Green,
        Place::Fields,
        Place::Yard,
        Place::River,
        Place::Woods,
    ];

    pub fn index(self) -> usize {
        Place::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        match self {
            Place::Gate => "the gate",
            Place::Cottages => "the cottages",
            Place::Green => "the green",
            Place::Fields => "the fields",
            Place::Yard => "the yard",
            Place::River => "the river",
            Place::Woods => "the woods",
        }
    }

    /// Where someone is, with the word that fits: "in the yard", "on the green".
    pub fn where_at(self) -> &'static str {
        match self {
            Place::Gate => "at the gate",
            Place::Cottages => "at the cottages",
            Place::Green => "on the green",
            Place::Fields => "in the fields",
            Place::Yard => "in the yard",
            Place::River => "by the river",
            Place::Woods => "in the woods",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Place::Gate => "gate",
            Place::Cottages => "cottages",
            Place::Green => "green",
            Place::Fields => "fields",
            Place::Yard => "yard",
            Place::River => "river",
            Place::Woods => "woods",
        }
    }

    /// The spots inside the place, left to right. The fields have plots
    /// instead.
    pub fn spots(self) -> &'static [Spot] {
        match self {
            Place::Gate => &[Spot::Gate],
            Place::Cottages => &[Spot::Homes, Spot::Hens, Spot::Goat],
            Place::Green => &[
                Spot::Table,
                Spot::Site(Building::Well),
                Spot::Site(Building::Bandstand),
                Spot::Site(Building::Inn),
            ],
            Place::Fields => &[],
            Place::Yard => &[
                Spot::Site(Building::Mill),
                Spot::Site(Building::Bakery),
                Spot::Site(Building::Smokehouse),
                Spot::Site(Building::Workshop),
                Spot::Site(Building::Dairy),
            ],
            Place::River => &[Spot::Fishing, Spot::Dock],
            Place::Woods => &[Spot::Forage, Spot::WoodPile, Spot::Quarry],
        }
    }

    /// Where a building stands.
    pub fn of(b: Building) -> Place {
        match b {
            Building::Well | Building::Bandstand | Building::Inn => Place::Green,
            _ => Place::Yard,
        }
    }
}

/// Somewhere to stand inside a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Spot {
    Gate,
    Homes,
    Hens,
    Goat,
    Table,
    Site(Building),
    Fishing,
    Dock,
    Forage,
    WoodPile,
    Quarry,
}

/// Where you are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum At {
    /// On the lane, beside a place.
    Lane(Place),
    /// Inside a place, at one of its spots.
    In(Place, usize),
    /// In the fields, at a plot.
    Plot(usize, usize),
}

impl At {
    pub fn place(self) -> Place {
        match self {
            At::Lane(p) | At::In(p, _) => p,
            At::Plot(..) => Place::Fields,
        }
    }
}

/// A plot in the field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Plot {
    /// Rough ground: dig it first.
    Scrub,
    /// Dug and ready to sow.
    Tilled,
    /// Something growing.
    Sown {
        crop: String,
        /// Days of growth so far.
        growth: u32,
        watered: bool,
        weeds: bool,
    },
}

impl Plot {
    pub fn crop(&self) -> Option<&'static Crop> {
        match self {
            Plot::Sown { crop: c, .. } => crop(c),
            _ => None,
        }
    }

    pub fn ripe(&self) -> bool {
        match self {
            Plot::Sown {
                crop: c, growth, ..
            } => crop(c).is_some_and(|c| *growth >= c.days),
            _ => false,
        }
    }

    pub fn needs_water(&self) -> bool {
        matches!(self, Plot::Sown { watered: false, .. }) && !self.ripe()
    }

    pub fn weedy(&self) -> bool {
        matches!(self, Plot::Sown { weeds: true, .. })
    }
}

/// A settler's mood, in one word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mood {
    Glum,
    Fine,
    Cheerful,
}

impl Mood {
    pub fn of(score: f32) -> Mood {
        if score <= -0.25 {
            Mood::Glum
        } else if score >= 0.4 {
            Mood::Cheerful
        } else {
            Mood::Fine
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mood::Glum => "glum",
            Mood::Fine => "fine",
            Mood::Cheerful => "cheerful",
        }
    }

    /// How fast a settler in this mood works.
    pub fn pace(self) -> f32 {
        match self {
            Mood::Glum => 0.6,
            Mood::Fine => 1.0,
            Mood::Cheerful => 1.3,
        }
    }
}

/// A reason a settler feels as they do, and for how many more days.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Thought {
    pub why: String,
    pub value: f32,
    pub days: u32,
}

/// What a settler does when a low mood lasts: one harmless thing until
/// someone talks them round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wobble {
    Bridge,
    Scones,
    SameSong,
    WalkedOff,
}

impl Wobble {
    pub fn doing(self) -> &'static str {
        match self {
            Wobble::Bridge => "sulking on the bridge",
            Wobble::Scones => "baking scones nobody asked for",
            Wobble::SameSong => "playing the same song over and over",
            Wobble::WalkedOff => "gone to the next village for the night",
        }
    }

    pub fn place(self) -> Place {
        match self {
            Wobble::Bridge => Place::River,
            Wobble::Scones => Place::Yard,
            Wobble::SameSong => Place::Green,
            Wobble::WalkedOff => Place::Gate,
        }
    }
}

/// What a settler is busy with right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Task {
    Idle,
    Water,
    Harvest,
    Weed,
    Till,
    Cook(Building),
    Supper,
    Build(Building),
    Fix,
    Wood,
    Stone,
    Furniture,
    Hens,
    Goat,
    Fish,
    Mend,
    Cheer,
    Practise,
    Rest,
}

impl Task {
    pub fn doing(self) -> &'static str {
        match self {
            Task::Idle => "taking a break",
            Task::Water => "watering the fields",
            Task::Harvest => "harvesting",
            Task::Weed => "weeding",
            Task::Till => "digging a new plot",
            Task::Cook(Building::Mill) => "grinding flour",
            Task::Cook(Building::Bakery) => "baking",
            Task::Cook(Building::Smokehouse) => "smoking fish",
            Task::Cook(Building::Dairy) => "making cheese",
            Task::Cook(_) => "cooking",
            Task::Supper => "cooking supper",
            Task::Build(_) => "building",
            Task::Fix => "fixing a trouble",
            Task::Wood => "chopping wood",
            Task::Stone => "breaking stone",
            Task::Furniture => "making furniture",
            Task::Hens => "seeing to the hens",
            Task::Goat => "seeing to the goat",
            Task::Fish => "fishing",
            Task::Mend => "nursing the sick",
            Task::Cheer => "keeping people company",
            Task::Practise => "practising music",
            Task::Rest => "in bed",
        }
    }

    pub fn place(self) -> Place {
        match self {
            Task::Water | Task::Harvest | Task::Weed | Task::Till => Place::Fields,
            Task::Cook(b) | Task::Build(b) => Place::of(b),
            Task::Supper | Task::Cheer | Task::Practise | Task::Idle => Place::Green,
            Task::Fix => Place::Yard,
            Task::Wood | Task::Stone => Place::Woods,
            Task::Furniture => Place::Yard,
            Task::Hens | Task::Goat | Task::Mend | Task::Rest => Place::Cottages,
            Task::Fish => Place::River,
        }
    }

    /// The sound a settler makes at it, by id.
    pub fn sound(self) -> &'static str {
        match self {
            Task::Water => "water",
            Task::Harvest | Task::Weed | Task::Till => "dig",
            Task::Cook(_) | Task::Supper => "cook",
            Task::Build(_) | Task::Fix | Task::Furniture => "hammer",
            Task::Wood => "chop",
            Task::Stone => "pick",
            Task::Hens => "hens",
            Task::Goat => "goat",
            Task::Fish => "cast",
            Task::Mend | Task::Cheer => "hum",
            Task::Practise => "practise",
            Task::Idle | Task::Rest => "",
        }
    }
}

/// A job on the job board: a settler's first choice of work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Job {
    /// They choose for themselves.
    Own,
    Skill(Skill),
}

impl Job {
    pub fn name(self) -> String {
        match self {
            Job::Own => "their own choice".to_string(),
            Job::Skill(s) => s.name().to_string(),
        }
    }
}

/// Someone who lives in Larkfield.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settler {
    pub name: String,
    pub trade: Trade,
    pub traits: [Trait; 2],
    /// Skill levels 0 to 10, with practice toward the next level.
    pub skills: [f32; 8],
    pub food: f32,
    pub rest: f32,
    pub comfort: f32,
    pub company: f32,
    pub thoughts: Vec<Thought>,
    pub job: Job,
    pub task: Task,
    /// Progress on the task now, in hours of work.
    pub work: f32,
    pub want: Option<usize>,
    pub story_step: u8,
    pub low_days: u32,
    pub wobble: Option<Wobble>,
    pub ill_days: u32,
    pub talked_today: bool,
    pub instrument: usize,
    pub arrived: u32,
    /// Liking for each other settler by name, -10 to 10.
    pub liking: BTreeMap<String, i32>,
}

impl Settler {
    pub fn has(&self, t: Trait) -> bool {
        self.traits.contains(&t)
    }

    pub fn skill(&self, s: Skill) -> u32 {
        self.skills[s.index()].floor().clamp(0.0, 10.0) as u32
    }

    /// The mood score, -1 to 1.
    pub fn mood_score(&self) -> f32 {
        let needs = (self.food + self.rest + self.comfort + self.company) / 4.0 - 0.5;
        let thoughts: f32 = self.thoughts.iter().map(|t| t.value).sum();
        let lean = if self.has(Trait::Cheerful) {
            0.1
        } else if self.has(Trait::Grumbler) {
            -0.1
        } else {
            0.0
        };
        (needs + thoughts + lean).clamp(-1.0, 1.0)
    }

    pub fn mood(&self) -> Mood {
        Mood::of(self.mood_score())
    }

    pub fn away(&self) -> bool {
        self.wobble == Some(Wobble::WalkedOff)
    }

    /// Where the settler is now.
    pub fn place(&self) -> Place {
        if let Some(w) = self.wobble {
            return w.place();
        }
        self.task.place()
    }

    /// What they are doing, for S and for passing them.
    pub fn doing(&self) -> String {
        if self.ill_days > 0 {
            return "in bed with a cold".to_string();
        }
        if let Some(w) = self.wobble {
            return w.doing().to_string();
        }
        self.task.doing().to_string()
    }

    /// The best reasons for the mood, for S.
    pub fn reasons(&self) -> Vec<String> {
        let mut t: Vec<&Thought> = self.thoughts.iter().collect();
        t.sort_by(|a, b| b.value.abs().total_cmp(&a.value.abs()));
        let mut out: Vec<String> = t.iter().take(2).map(|t| t.why.clone()).collect();
        if self.food < 0.3 {
            out.push("hungry".to_string());
        }
        if self.company < 0.25 {
            out.push("lonely".to_string());
        }
        if self.comfort < 0.25 {
            out.push("uncomfortable at home".to_string());
        }
        out
    }

    pub fn think(&mut self, why: &str, value: f32, days: u32) {
        if let Some(t) = self.thoughts.iter_mut().find(|t| t.why == why) {
            t.value = value;
            t.days = days.max(t.days);
        } else {
            self.thoughts.push(Thought {
                why: why.to_string(),
                value,
                days,
            });
        }
    }

    pub fn instrument(&self) -> &'static str {
        INSTRUMENTS[self.instrument % INSTRUMENTS.len()]
    }

    /// Their skills, best first.
    pub fn best_skills(&self) -> Vec<Skill> {
        let mut s = Skill::ALL.to_vec();
        s.sort_by(|a, b| self.skills[b.index()].total_cmp(&self.skills[a.index()]));
        s
    }
}

/// Day length, in Settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayLength {
    Short,
    #[default]
    Normal,
    Long,
}

impl DayLength {
    pub const ALL: [DayLength; 3] = [DayLength::Short, DayLength::Normal, DayLength::Long];

    /// Real seconds in a game hour.
    pub fn hour_secs(self) -> f32 {
        match self {
            DayLength::Short => 14.0,
            DayLength::Normal => 20.0,
            DayLength::Long => 30.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DayLength::Short => "short",
            DayLength::Normal => "normal",
            DayLength::Long => "long",
        }
    }
}

/// The weather for a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weather {
    Sunny,
    Cloudy,
    Rain,
    Storm,
    Frost,
    Snow,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::Sunny => "sunny",
            Weather::Cloudy => "cloudy",
            Weather::Rain => "rain",
            Weather::Storm => "a storm",
            Weather::Frost => "frost",
            Weather::Snow => "snow",
        }
    }

    pub fn wet(self) -> bool {
        matches!(self, Weather::Rain | Weather::Storm)
    }
}

/// The part of the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// The morning news is waiting to be heard.
    Morning,
    /// The working day.
    Day,
    /// The supper bell has rung: supper waits.
    Supper,
    /// A challenge is over.
    Over,
}

/// What kind of hamlet this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Home,
    Practice,
    Challenge(Goal),
}

/// What a challenge hamlet is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Goal {
    /// Fill the barge's order.
    Market,
    /// Everyone cheerful at once.
    Cheer,
    /// The mill, the bakery and a festival.
    Founders,
}

/// Something that happened, for the screen to sound and say.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A spoken line, the answer to a key or news.
    Say(String),
    /// A sound by id, at a place.
    Cue(&'static str, Place),
    /// The end of the lane, a place or the field.
    Edge,
    /// Stepped somewhere.
    Moved,
    /// The fish has bitten.
    Bite,
    /// The supper bell.
    SupperBell,
    /// A building is finished.
    Built(Building),
    /// A settler joined.
    Arrived(String),
    /// Ask: a seed to sow in the plot.
    AskSeed,
    /// Ask: what to build at a site.
    AskBuild(Building),
    /// Ask: the barge.
    AskBarge,
    /// Ask: let the person at the gate stay?
    AskArrival,
    /// A keepsake for the shelf.
    Keepsake(String),
    /// The challenge's goal is met.
    GoalMet,
}

/// Someone at the gate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Caller {
    /// Someone asking to stay.
    Stranger(Settler),
    /// A visitor passing through.
    Visitor(almanac::Visitor),
}

/// The barge's order, in a market challenge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub goods: Vec<(String, u32)>,
    pub filled: bool,
}

/// The whole hamlet, saved between visits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hamlet {
    pub kind: Kind,
    #[serde(default)]
    pub slot: String,
    pub seed: u64,
    pub rolls: u64,
    pub day: u32,
    /// Hours since the morning bell, 0 to [`DAY_HOURS`].
    pub hour: f32,
    pub phase: Phase,
    pub weather: Weather,
    pub at: At,
    pub plots: Vec<Plot>,
    pub larder: BTreeMap<String, u32>,
    pub coins: u32,
    pub settlers: Vec<Settler>,
    pub built: Vec<Building>,
    /// A building going up, and the work left on it in hours.
    pub building: Option<(Building, f32)>,
    pub hens: u32,
    pub goat: bool,
    pub eggs_waiting: u32,
    pub milk_waiting: bool,
    pub fed_hens: bool,
    pub fed_goat: bool,
    pub seeds: BTreeMap<String, u32>,
    pub barge: bool,
    pub caller: Option<Caller>,
    pub troubles: Vec<Trouble>,
    pub news: Vec<String>,
    pub festivals: u32,
    pub keepsakes: Vec<String>,
    pub pace: Pace,
    pub day_length: DayLength,
    pub quiet_days: u32,
    pub foraged: u32,
    /// Fish settlers caught today.
    #[serde(default)]
    pub fished: u32,
    pub strikes: u32,
    pub casting: Option<f32>,
    pub bite: Option<f32>,
    /// What the day's work added, for bed.
    pub tally: BTreeMap<String, u32>,
    pub supper_seat: Option<String>,
    pub order: Option<Order>,
    pub day_limit: Option<u32>,
    pub goal_met: bool,
    pub turned_away: Vec<String>,
    /// Barge fair today: the barge comes every day and pays a coin more.
    #[serde(default)]
    pub fair_day: bool,
    /// Music night today: a band plays on the green after supper.
    #[serde(default)]
    pub music_night: bool,
    #[serde(skip)]
    pub keys: Keys,
    #[serde(skip)]
    pub events: Vec<Event>,
    #[serde(skip)]
    pub last: String,
}

/// The player's own key names, for lines that name a key.
#[derive(Clone, Debug, PartialEq)]
pub struct Keys {
    pub main: String,
    pub second: String,
    pub up: String,
    pub down: String,
}

impl Default for Keys {
    fn default() -> Keys {
        Keys {
            main: "Space".into(),
            second: "Tab".into(),
            up: "Up".into(),
            down: "Down".into(),
        }
    }
}

/// How a key in the fields answers when nothing changes.
pub const NOTHING_HERE: &str = "Nothing to do here.";

impl Hamlet {
    /// A new home hamlet: you, one settler, a field with four plots dug.
    pub fn home(seed: u64) -> Hamlet {
        let mut h = Hamlet::bare(Kind::Home, seed);
        // The row nearest the lane is dug, so it is the first one you find.
        for i in 0..4 {
            h.plots[(ROWS - 1) * COLS + i] = Plot::Tilled;
        }
        h.add_seeds("turnip", 4);
        h.add_seeds("radish", 4);
        h.larder.insert("bread".into(), 4);
        h.larder.insert("potato".into(), 6);
        h.larder.insert("wood".into(), 6);
        h.larder.insert("stone".into(), 4);
        h.coins = 10;
        h.hens = 2;
        let first = h.make_settler(None);
        h.settlers.push(first);
        h.begin_day();
        h
    }

    /// Practice water: a small, gentle hamlet for the tutorial.
    pub fn practice(seed: u64) -> Hamlet {
        let mut h = Hamlet::home(seed);
        h.kind = Kind::Practice;
        h.pace = Pace::Gentle;
        h.news.clear();
        h
    }

    pub fn bare(kind: Kind, seed: u64) -> Hamlet {
        Hamlet {
            kind,
            slot: String::new(),
            seed,
            rolls: 0,
            day: 1,
            hour: 0.0,
            phase: Phase::Morning,
            weather: Weather::Sunny,
            at: At::Lane(Place::Cottages),
            plots: vec![Plot::Scrub; ROWS * COLS],
            larder: BTreeMap::new(),
            coins: 0,
            settlers: Vec::new(),
            built: Vec::new(),
            building: None,
            hens: 0,
            goat: false,
            eggs_waiting: 0,
            milk_waiting: false,
            fed_hens: false,
            fed_goat: false,
            seeds: BTreeMap::new(),
            barge: false,
            caller: None,
            troubles: Vec::new(),
            news: Vec::new(),
            festivals: 0,
            keepsakes: Vec::new(),
            pace: Pace::Steady,
            day_length: DayLength::Normal,
            quiet_days: 0,
            foraged: 0,
            fished: 0,
            strikes: 0,
            casting: None,
            bite: None,
            tally: BTreeMap::new(),
            supper_seat: None,
            order: None,
            day_limit: None,
            goal_met: false,
            turned_away: Vec::new(),
            fair_day: false,
            music_night: false,
            keys: Keys::default(),
            events: Vec::new(),
            last: String::new(),
        }
    }

    // --- randomness -----------------------------------------------------

    /// A random source that carries on where the last left off, saved
    /// with the hamlet.
    pub fn rng(&mut self) -> Rng {
        self.rolls += 1;
        Rng::seeded(self.seed ^ self.rolls.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.rng().chance(p)
    }

    // --- the calendar ---------------------------------------------------

    pub fn season(&self) -> Season {
        Season::ALL[(((self.day - 1) / SEASON_DAYS) % 4) as usize]
    }

    pub fn day_of_season(&self) -> u32 {
        (self.day - 1) % SEASON_DAYS + 1
    }

    pub fn year(&self) -> u32 {
        (self.day - 1) / (SEASON_DAYS * 4) + 1
    }

    pub fn festival_today(&self) -> bool {
        self.day_of_season() == SEASON_DAYS
    }

    /// The time, as people say it.
    pub fn time_words(&self) -> &'static str {
        let h = self.hour;
        if h < 2.0 {
            "early morning"
        } else if h < 4.5 {
            "mid-morning"
        } else if h < 6.0 {
            "midday"
        } else if h < 9.0 {
            "afternoon"
        } else {
            "evening"
        }
    }

    // --- people ---------------------------------------------------------

    /// Makes a new person, maybe of a given trade.
    pub fn make_settler(&mut self, trade: Option<Trade>) -> Settler {
        let mut rng = self.rng();
        let taken: Vec<String> = self
            .settlers
            .iter()
            .map(|s| s.name.clone())
            .chain(self.turned_away.iter().cloned())
            .collect();
        let free: Vec<&str> = NAMES
            .iter()
            .copied()
            .filter(|n| !taken.iter().any(|t| t == n))
            .collect();
        let name = rng.pick(&free).to_string();
        let trades_here: Vec<Trade> = self.settlers.iter().map(|s| s.trade).collect();
        let trade = trade.unwrap_or_else(|| {
            let fresh: Vec<Trade> = Trade::ALL
                .into_iter()
                .filter(|t| !trades_here.contains(t))
                .collect();
            if fresh.is_empty() {
                *rng.pick(&Trade::ALL)
            } else {
                *rng.pick(&fresh)
            }
        });
        let a = *rng.pick(&Trait::ALL);
        let mut b = *rng.pick(&Trait::ALL);
        while b.clashes(a) {
            b = *rng.pick(&Trait::ALL);
        }
        let mut skills = [0.0f32; 8];
        for s in skills.iter_mut() {
            *s = rng.below(3) as f32;
        }
        let [best, second] = trade.skills();
        skills[best.index()] = 5.0 + rng.below(3) as f32;
        skills[second.index()] = 3.0 + rng.below(2) as f32;
        if a == Trait::Musician || b == Trait::Musician {
            skills[Skill::Music.index()] += 2.0;
        }
        if a == Trait::GreenFingers || b == Trait::GreenFingers {
            skills[Skill::Farming.index()] += 2.0;
        }
        let instrument = self.settlers.len();
        Settler {
            name,
            trade,
            traits: [a, b],
            skills,
            food: 0.7,
            rest: 0.8,
            comfort: 0.5,
            company: 0.5,
            thoughts: vec![Thought {
                why: "new in Larkfield".to_string(),
                value: 0.15,
                days: 3,
            }],
            job: Job::Own,
            task: Task::Idle,
            work: 0.0,
            want: None,
            story_step: 0,
            low_days: 0,
            wobble: None,
            ill_days: 0,
            talked_today: false,
            instrument,
            arrived: self.day,
            liking: BTreeMap::new(),
        }
    }

    /// Sets a settler's first job, and they move to it if there is any.
    pub fn set_job(&mut self, name: &str, job: Job) {
        if let Some(i) = self.settlers.iter().position(|s| s.name == name) {
            self.settlers[i].job = job;
            if self.phase == Phase::Day {
                self.choose_task(i);
            }
        }
    }

    pub fn settler(&self, name: &str) -> Option<&Settler> {
        self.settlers.iter().find(|s| s.name == name)
    }

    /// Mouths to feed: the settlers and you.
    pub fn mouths(&self) -> u32 {
        self.settlers.len() as u32 + 1
    }

    /// Takes someone in.
    pub fn welcome(&mut self, mut s: Settler) {
        s.instrument = self.settlers.len();
        s.arrived = self.day;
        for other in &self.settlers {
            let start = liking_between(&s, other);
            s.liking.insert(other.name.clone(), start);
        }
        let name = s.name.clone();
        for other in self.settlers.iter_mut() {
            let start = liking_between(other, &s);
            other.liking.insert(name.clone(), start);
            other.think("a new neighbour", 0.1, 2);
        }
        self.settlers.push(s);
        self.events.push(Event::Arrived(name));
    }

    // --- the larder -----------------------------------------------------

    pub fn has(&self, id: &str) -> u32 {
        self.larder.get(id).copied().unwrap_or(0)
    }

    pub fn add(&mut self, id: &str, n: u32) {
        *self.larder.entry(id.to_string()).or_insert(0) += n;
        if n > 0 && id != "wood" && id != "stone" {
            *self.tally.entry(id.to_string()).or_insert(0) += n;
        }
    }

    pub fn take(&mut self, id: &str, n: u32) -> bool {
        let have = self.has(id);
        if have < n {
            return false;
        }
        if have == n {
            self.larder.remove(id);
        } else {
            self.larder.insert(id.to_string(), have - n);
        }
        true
    }

    pub fn add_seeds(&mut self, id: &str, n: u32) {
        *self.seeds.entry(id.to_string()).or_insert(0) += n;
    }

    /// Food in the larder, in tenths of a person's day.
    pub fn food_tenths(&self) -> u32 {
        self.larder
            .iter()
            .map(|(id, n)| good(id).map_or(0, |g| g.food) * n)
            .sum()
    }

    /// Days of food for everyone.
    pub fn food_days(&self) -> u32 {
        self.food_tenths() / (MEAL * self.mouths())
    }

    pub fn food_line(&self) -> String {
        match self.food_days() {
            0 => "Less than a day of food.".to_string(),
            1 => "1 day of food.".to_string(),
            n => format!("{n} days of food."),
        }
    }

    /// The store, for S: materials and coins.
    pub fn store_line(&self) -> String {
        format!(
            "{} wood, {} stone, {}, {} {}.",
            self.has("wood"),
            self.has("stone"),
            crate::buildings::count(NAILS, self.has(NAILS)),
            self.coins,
            if self.coins == 1 { "coin" } else { "coins" }
        )
    }

    pub fn is_built(&self, b: Building) -> bool {
        self.built.contains(&b)
    }

    // --- the day --------------------------------------------------------

    /// The morning: weather, the barge, the almanac, and the news.
    pub fn begin_day(&mut self) {
        self.phase = Phase::Morning;
        self.hour = 0.0;
        self.tally.clear();
        self.foraged = 0;
        self.fished = 0;
        self.strikes = 0;
        self.casting = None;
        self.bite = None;
        self.supper_seat = None;
        self.news.clear();
        let season = self.season();
        let mut rng = self.rng();
        self.weather = match season {
            // The first day is dry, so the first watering counts.
            _ if self.day == 1 => Weather::Sunny,
            Season::Winter => *rng.pick(&[
                Weather::Frost,
                Weather::Snow,
                Weather::Cloudy,
                Weather::Sunny,
            ]),
            _ => {
                let r = rng.below(10);
                match r {
                    0..=3 => Weather::Sunny,
                    4..=6 => Weather::Cloudy,
                    _ => Weather::Rain,
                }
            }
        };
        self.barge = self.day % 3 == 1 || self.kind != Kind::Home && self.day == 1 || self.fair_day;
        if self.hens > 0 && (self.fed_hens || self.day == 1) {
            self.eggs_waiting = self.hens;
        }
        if self.goat && (self.fed_goat || self.day == 1) && !self.troubles.contains(&Trouble::Fence)
        {
            self.milk_waiting = true;
        }
        self.fed_hens = false;
        self.fed_goat = false;
        for s in self.settlers.iter_mut() {
            s.talked_today = false;
            s.task = Task::Idle;
            s.work = 0.0;
            s.rest = (s.rest + 0.6).min(1.0);
            if s.away() {
                s.wobble = None;
                s.think("back from the next village", 0.1, 1);
            }
        }
        if self.kind != Kind::Practice {
            almanac::morning(self);
        }
        if self.weather.wet() {
            for p in self.plots.iter_mut() {
                if let Plot::Sown { watered, .. } = p {
                    *watered = true;
                }
            }
        }
        if self.barge {
            self.news.push("The barge is at the river.".to_string());
        }
        if self.festival_today() {
            self.news.push(format!(
                "{} today. Supper is a feast.",
                self.season().festival()
            ));
        }
    }

    /// The morning news, heard: the day starts.
    pub fn start_work(&mut self) {
        if self.phase == Phase::Morning {
            self.phase = Phase::Day;
            for i in 0..self.settlers.len() {
                self.choose_task(i);
            }
        }
    }

    /// The morning line: weather first, then the news.
    pub fn morning_text(&self) -> String {
        let mut parts = vec![format!(
            "Day {} of {}, year {}. {}.",
            self.day_of_season(),
            self.season().name(),
            self.year(),
            crate::rules::sentence_case(self.weather.name())
        )];
        parts.extend(self.news.iter().cloned());
        parts.join(" ")
    }

    /// Advances the day by `dt` real seconds.
    pub fn update(&mut self, dt: f32) {
        if self.phase != Phase::Day {
            return;
        }
        let hours = dt / self.day_length.hour_secs();
        self.hour += hours;
        // A market order can fill at any time of day: say so at once.
        if self.kind == Kind::Challenge(Goal::Market) && !self.goal_met {
            self.check_goal();
            if self.goal_met {
                self.say("The order is filled. The barge takes it at supper.");
            }
        }
        // The fishing line.
        if let Some(t) = self.casting.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.casting = None;
                self.bite = Some(1.6);
                self.events.push(Event::Bite);
            }
        }
        if let Some(t) = self.bite.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.bite = None;
                self.say("It got away.");
            }
        }
        for i in 0..self.settlers.len() {
            self.work(i, hours);
        }
        for s in self.settlers.iter_mut() {
            s.food =
                (s.food - hours * 0.03 * if s.has(Trait::Hungry) { 1.3 } else { 1.0 }).max(0.0);
            s.rest = (s.rest - hours * 0.04).max(0.0);
            let lonely = if s.has(Trait::Chatterbox) {
                0.05
            } else if s.has(Trait::Quiet) {
                0.015
            } else {
                0.03
            };
            s.company = (s.company - hours * lonely).max(0.0);
        }
        // Settlers at the same place keep each other company.
        let places: Vec<(Place, bool)> = self
            .settlers
            .iter()
            .map(|s| (s.place(), s.ill_days == 0 && !s.away()))
            .collect();
        for i in 0..self.settlers.len() {
            if !places[i].1 {
                continue;
            }
            let near = places
                .iter()
                .enumerate()
                .filter(|(j, p)| *j != i && p.1 && p.0 == places[i].0)
                .count();
            if near > 0 {
                let s = &mut self.settlers[i];
                s.company = (s.company + hours * 0.05 * near as f32).min(1.0);
            }
        }
        if self.hour >= DAY_HOURS {
            self.hour = DAY_HOURS;
            self.phase = Phase::Supper;
            self.events.push(Event::SupperBell);
        }
    }

    // --- settlers at work -----------------------------------------------

    /// Picks what settler `i` does next.
    fn choose_task(&mut self, i: usize) {
        let s = &self.settlers[i];
        if s.ill_days > 0 {
            self.settlers[i].task = Task::Rest;
            return;
        }
        if s.wobble.is_some() {
            return;
        }
        let mut order: Vec<Skill> = s.best_skills();
        if let Job::Skill(first) = s.job {
            order.retain(|&k| k != first);
            order.insert(0, first);
        }
        let task = order
            .into_iter()
            .find_map(|k| self.task_for(k))
            .unwrap_or(Task::Idle);
        let s = &mut self.settlers[i];
        if s.task != task {
            s.work = 0.0;
        }
        s.task = task;
    }

    /// Work of a kind that needs doing now.
    fn task_for(&self, skill: Skill) -> Option<Task> {
        match skill {
            Skill::Farming => {
                if self.plots.iter().any(|p| p.ripe()) {
                    Some(Task::Harvest)
                } else if self.is_built(Building::Well)
                    && self.plots.iter().any(|p| p.needs_water())
                {
                    Some(Task::Water)
                } else if self.plots.iter().any(|p| p.weedy()) {
                    Some(Task::Weed)
                } else {
                    None
                }
            }
            Skill::Cooking => {
                for b in [
                    Building::Bakery,
                    Building::Mill,
                    Building::Smokehouse,
                    Building::Dairy,
                ] {
                    if self.can_make(b) {
                        return Some(Task::Cook(b));
                    }
                }
                if self.hour > DAY_HOURS - 3.0 {
                    Some(Task::Supper)
                } else {
                    None
                }
            }
            Skill::Building => {
                if let Some((b, _)) = self.building {
                    Some(Task::Build(b))
                } else if !self.troubles.is_empty() {
                    Some(Task::Fix)
                } else if self.has("stone") < 12 {
                    Some(Task::Stone)
                } else {
                    None
                }
            }
            Skill::Crafting => {
                if self.can_make(Building::Workshop) && self.has(FURNITURE) < self.mouths() {
                    Some(Task::Furniture)
                } else if self.has("wood") < 16 {
                    Some(Task::Wood)
                } else {
                    None
                }
            }
            Skill::Animals => {
                if self.hens > 0 && (!self.fed_hens || self.eggs_waiting > 0) {
                    Some(Task::Hens)
                } else if self.goat && (!self.fed_goat || self.milk_waiting) {
                    Some(Task::Goat)
                } else {
                    None
                }
            }
            Skill::Fishing => {
                if self.weather != Weather::Storm && self.fished < FISH_A_DAY {
                    Some(Task::Fish)
                } else {
                    None
                }
            }
            Skill::Mending => {
                if self.settlers.iter().any(|s| s.ill_days > 0) {
                    Some(Task::Mend)
                } else if self.settlers.iter().any(|s| s.company < 0.35) {
                    Some(Task::Cheer)
                } else {
                    None
                }
            }
            Skill::Music => {
                if self.hour > DAY_HOURS - 4.0 {
                    Some(Task::Practise)
                } else {
                    None
                }
            }
        }
    }

    /// Whether a building has what it needs for a batch.
    pub fn can_make(&self, b: Building) -> bool {
        if !self.is_built(b) || self.troubles.iter().any(|t| t.stops() == Some(b)) {
            return false;
        }
        match b.recipe() {
            Some((inputs, _)) => inputs.iter().all(|(id, n)| self.has(id) >= *n),
            None => false,
        }
    }

    /// One batch at a building: what goes in, what comes out.
    pub fn make(&mut self, b: Building) -> Option<(&'static str, u32)> {
        if !self.can_make(b) {
            return None;
        }
        let (inputs, (out, n)) = b.recipe()?;
        for (id, k) in inputs {
            self.take(id, *k);
        }
        // Fruit and flour make pies.
        if b == Building::Bakery {
            for fruit in ["strawberry", "pumpkin", "berries"] {
                if self.take(fruit, 2) {
                    self.add("pie", 2);
                    return Some(("pie", 2));
                }
            }
        }
        self.add(out, n);
        Some((out, n))
    }

    fn skill_of(&self, i: usize, task: Task) -> Skill {
        let _ = i;
        match task {
            Task::Water | Task::Harvest | Task::Weed | Task::Till => Skill::Farming,
            Task::Cook(_) | Task::Supper => Skill::Cooking,
            Task::Build(_) | Task::Fix | Task::Stone => Skill::Building,
            Task::Wood | Task::Furniture => Skill::Crafting,
            Task::Hens | Task::Goat => Skill::Animals,
            Task::Fish => Skill::Fishing,
            Task::Mend | Task::Cheer => Skill::Mending,
            Task::Practise => Skill::Music,
            Task::Idle | Task::Rest => Skill::Music,
        }
    }

    /// A settler's work for `hours`.
    fn work(&mut self, i: usize, hours: f32) {
        let s = &self.settlers[i];
        if s.ill_days > 0 || s.wobble.is_some() {
            return;
        }
        let task = s.task;
        if task == Task::Idle {
            if (self.hour * 4.0).fract() < hours * 4.0 {
                self.choose_task(i);
            }
            return;
        }
        let skill = self.skill_of(i, task);
        let mut rate = 0.5 + s.skill(skill) as f32 / 10.0;
        rate *= s.mood().pace();
        if s.has(Trait::Clumsy) {
            rate *= 0.85;
        }
        if s.has(Trait::EarlyRiser) && self.hour < 4.0 || s.has(Trait::NightOwl) && self.hour > 8.0
        {
            rate *= 1.2;
        }
        let s = &mut self.settlers[i];
        s.work += hours * rate;
        let level = s.skills[skill.index()];
        s.skills[skill.index()] = (level + hours * 0.05 / (1.0 + level * 0.3)).min(10.0);
        // One unit of work is one finished job.
        let unit = match task {
            Task::Build(_) => 0.25,
            Task::Water | Task::Weed | Task::Harvest => 0.35,
            Task::Practise | Task::Cheer | Task::Supper => 1.0,
            _ => 1.0,
        };
        if s.work < unit {
            return;
        }
        s.work -= unit;
        let name = s.name.clone();
        let green = s.has(Trait::GreenFingers);
        let done = self.finish_task(i, task, &name, green, unit);
        if !done {
            self.settlers[i].task = Task::Idle;
            self.choose_task(i);
        }
    }

    /// One job done. False when there was nothing left to do.
    fn finish_task(&mut self, i: usize, task: Task, name: &str, green: bool, unit: f32) -> bool {
        let place = task.place();
        match task {
            Task::Water => {
                let Some(p) = self.plots.iter_mut().find(|p| p.needs_water()) else {
                    return false;
                };
                if let Plot::Sown { watered, .. } = p {
                    *watered = true;
                }
                self.events.push(Event::Cue("water", place));
            }
            Task::Weed => {
                let Some(p) = self.plots.iter_mut().find(|p| p.weedy()) else {
                    return false;
                };
                if let Plot::Sown { weeds, .. } = p {
                    *weeds = false;
                }
            }
            Task::Harvest => {
                let Some(at) = self.plots.iter().position(|p| p.ripe()) else {
                    return false;
                };
                self.harvest_plot(at, green);
                self.events.push(Event::Cue("harvest", place));
            }
            Task::Till => return false,
            Task::Cook(b) => {
                if self.make(b).is_none() {
                    return false;
                }
                self.events.push(Event::Cue("cook", place));
            }
            Task::Supper => {
                self.settlers[i].think("cooked supper", 0.05, 1);
                return self.hour < DAY_HOURS;
            }
            Task::Build(_) => {
                self.build_work(unit, Some(name));
            }
            Task::Fix => {
                if self.troubles.is_empty() {
                    return false;
                }
                let t = self.troubles.remove(0);
                let line = t.fixed(name);
                self.say_news(line);
                self.events.push(Event::Cue("hammer", place));
            }
            Task::Wood => {
                self.add("wood", 1);
                self.events.push(Event::Cue("chop", place));
            }
            Task::Stone => {
                self.add("stone", 1);
                self.events.push(Event::Cue("pick", place));
            }
            Task::Furniture => {
                if self.make(Building::Workshop).is_none() {
                    return false;
                }
                self.events.push(Event::Cue("hammer", place));
            }
            Task::Hens => {
                if !self.fed_hens {
                    self.fed_hens = true;
                } else if self.eggs_waiting > 0 {
                    let n = self.eggs_waiting;
                    self.eggs_waiting = 0;
                    self.add("eggs", n);
                } else {
                    return false;
                }
                self.events.push(Event::Cue("hens", place));
            }
            Task::Goat => {
                if !self.fed_goat {
                    self.fed_goat = true;
                } else if self.milk_waiting {
                    self.milk_waiting = false;
                    self.add("milk", 2);
                } else {
                    return false;
                }
                self.events.push(Event::Cue("goat", place));
            }
            Task::Fish => {
                if self.fished >= FISH_A_DAY {
                    return false;
                }
                if self.chance(0.6) {
                    self.fished += 1;
                    self.add("fish", 1);
                    self.events.push(Event::Cue("splash", place));
                }
            }
            Task::Mend => {
                let Some(j) = self.settlers.iter().position(|s| s.ill_days > 0) else {
                    return false;
                };
                let patient = &mut self.settlers[j];
                patient.ill_days = patient.ill_days.saturating_sub(1);
                patient.think("well looked after", 0.1, 2);
                if patient.ill_days == 0 {
                    let line = format!("{} is up and about again.", patient.name);
                    patient.task = Task::Idle;
                    self.say_news(line);
                }
            }
            Task::Cheer => {
                let Some(j) = self
                    .settlers
                    .iter()
                    .enumerate()
                    .filter(|(j, s)| *j != i && s.ill_days == 0 && !s.away())
                    .min_by(|a, b| a.1.company.total_cmp(&b.1.company))
                    .map(|(j, _)| j)
                else {
                    return false;
                };
                let lonely = self.settlers[j].company < 0.35;
                self.settlers[j].company = (self.settlers[j].company + 0.3).min(1.0);
                let other = self.settlers[j].name.clone();
                bump_liking(&mut self.settlers, i, &other, 1);
                if !lonely {
                    return false;
                }
            }
            Task::Practise => {
                self.settlers[i].think("played music today", 0.05, 1);
                return self.hour < DAY_HOURS;
            }
            Task::Idle | Task::Rest => return false,
        }
        true
    }

    fn harvest_plot(&mut self, at: usize, green: bool) -> Option<&'static Crop> {
        let c = self.plots[at].crop()?;
        let (good, mut n) = c.yields;
        if green {
            n += 1;
        }
        self.add(good, n);
        // A crop that is done keeps a seed back for the next sowing.
        if c.again.is_none() {
            self.add_seeds(c.id, 1);
        }
        self.plots[at] = match c.again {
            Some(again) => Plot::Sown {
                crop: c.id.to_string(),
                growth: c.days - again,
                watered: false,
                weeds: false,
            },
            None => Plot::Tilled,
        };
        Some(c)
    }

    /// Building work, from a settler or from you.
    fn build_work(&mut self, hours: f32, by: Option<&str>) {
        let Some((b, left)) = self.building.as_mut() else {
            return;
        };
        *left -= hours;
        let b = *b;
        self.events.push(Event::Cue("hammer", Place::of(b)));
        if *left <= 0.0 {
            self.building = None;
            self.built.push(b);
            let line = match by {
                Some(name) => format!(
                    "{} is built. {} put in the last nail.",
                    sentence_case(b.name()),
                    name
                ),
                None => format!("{} is built.", sentence_case(b.name())),
            };
            self.events.push(Event::Built(b));
            self.say_news(line);
            for s in self.settlers.iter_mut() {
                s.think(&format!("{} went up", b.name()), 0.15, 3);
            }
        }
    }

    // --- your own work ----------------------------------------------------

    pub fn say(&mut self, text: impl Into<String>) {
        let t = text.into();
        self.last = t.clone();
        self.events.push(Event::Say(t));
    }

    /// News said now and kept for supper.
    fn say_news(&mut self, text: String) {
        self.say(text);
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Left and Right: along the lane, between spots, or between plots.
    pub fn step(&mut self, d: i32) {
        match self.at {
            At::Lane(p) => {
                let i = p.index() as i32 + d;
                if i < 0 || i >= Place::ALL.len() as i32 {
                    self.events.push(Event::Edge);
                    self.say(if d < 0 {
                        "The gate. The lane ends here."
                    } else {
                        "The woods. The lane ends here."
                    });
                    return;
                }
                self.at = At::Lane(Place::ALL[i as usize]);
                self.events.push(Event::Moved);
                let line = self.lane_line();
                self.say(line);
            }
            At::In(p, i) => {
                let n = p.spots().len() as i32;
                let j = i as i32 + d;
                if j < 0 || j >= n {
                    self.events.push(Event::Edge);
                    self.say(format!("The edge of {}.", p.name()));
                    return;
                }
                self.at = At::In(p, j as usize);
                self.events.push(Event::Moved);
                let line = self.spot_line();
                self.say(line);
            }
            At::Plot(r, c) => {
                let j = c as i32 + d;
                if j < 0 || j >= COLS as i32 {
                    self.events.push(Event::Edge);
                    self.say("The edge of the field.");
                    return;
                }
                self.at = At::Plot(r, j as usize);
                self.events.push(Event::Moved);
                let line = self.plot_line(r, j as usize);
                self.say(line);
            }
        }
    }

    /// Up: into a place, or a row further into the field. Down: back
    /// toward the lane.
    pub fn step_in(&mut self, up: bool) {
        match (self.at, up) {
            (At::Lane(Place::Fields), true) => {
                self.at = At::Plot(ROWS - 1, 0);
                self.events.push(Event::Moved);
                let line = format!("Into the fields. {}", self.plot_line(ROWS - 1, 0));
                self.say(line);
            }
            (At::Lane(p), true) => {
                self.at = At::In(p, 0);
                self.events.push(Event::Moved);
                let line = match p {
                    Place::River => format!("Down to the riverbank. {}", self.spot_line()),
                    Place::Green => format!("Onto the green. {}", self.spot_line()),
                    _ => format!("Into {}. {}", p.name(), self.spot_line()),
                };
                self.say(line);
            }
            (At::Lane(_), false) => {
                let line = format!("You are on the lane. {} goes in.", self.keys.up);
                self.say(line);
            }
            (At::In(p, _), false) => {
                self.at = At::Lane(p);
                self.events.push(Event::Moved);
                self.say(format!("Back on the lane, by {}.", p.name()));
            }
            (At::In(..), true) => {
                let line = format!("{} goes back to the lane.", self.keys.down);
                self.say(line);
            }
            (At::Plot(r, c), true) => {
                if r == 0 {
                    self.events.push(Event::Edge);
                    self.say("The far edge of the field.");
                } else {
                    self.at = At::Plot(r - 1, c);
                    self.events.push(Event::Moved);
                    let line = self.plot_line(r - 1, c);
                    self.say(line);
                }
            }
            (At::Plot(r, c), false) => {
                if r + 1 >= ROWS {
                    self.at = At::Lane(Place::Fields);
                    self.events.push(Event::Moved);
                    self.say("Back on the lane, by the fields.");
                } else {
                    self.at = At::Plot(r + 1, c);
                    self.events.push(Event::Moved);
                    let line = self.plot_line(r + 1, c);
                    self.say(line);
                }
            }
        }
    }

    /// Who is at a place now.
    pub fn people_at(&self, p: Place) -> Vec<&Settler> {
        self.settlers
            .iter()
            .filter(|s| s.place() == p && !s.away())
            .collect()
    }

    /// The line on reaching a place on the lane.
    pub fn lane_line(&self) -> String {
        let p = match self.at {
            At::Lane(p) => p,
            _ => return String::new(),
        };
        let mut line = sentence_case(p.name());
        line.push('.');
        if let Some(extra) = self.place_note(p) {
            line.push(' ');
            line.push_str(&extra);
        }
        let people: Vec<String> = self
            .people_at(p)
            .iter()
            .map(|s| format!("{}, {}", s.name, s.doing()))
            .collect();
        if !people.is_empty() {
            line.push(' ');
            line.push_str(&people.join(". "));
            line.push('.');
        }
        line
    }

    /// Something worth knowing about a place, said as you pass.
    fn place_note(&self, p: Place) -> Option<String> {
        match p {
            Place::Gate => match &self.caller {
                Some(Caller::Stranger(s)) => {
                    Some(format!("{}, a {}, is waiting.", s.name, s.trade.name()))
                }
                Some(Caller::Visitor(v)) => Some(format!("{} is here.", sentence_case(v.who()))),
                None => None,
            },
            Place::River if self.barge => Some("The barge is in.".to_string()),
            Place::Fields => {
                let ripe = self.plots.iter().filter(|p| p.ripe()).count();
                let dry = self.plots.iter().filter(|p| p.needs_water()).count();
                let mut out = Vec::new();
                if ripe > 0 {
                    out.push(format!("{ripe} ripe"));
                }
                if dry > 0 {
                    out.push(format!("{dry} dry"));
                }
                (!out.is_empty()).then(|| format!("{}.", sentence_case(&out.join(", "))))
            }
            _ => None,
        }
    }

    pub fn spot(&self) -> Option<Spot> {
        match self.at {
            At::In(p, i) => p.spots().get(i).copied(),
            _ => None,
        }
    }

    /// The line on reaching a spot.
    pub fn spot_line(&self) -> String {
        let Some(spot) = self.spot() else {
            return String::new();
        };
        match spot {
            Spot::Gate => match &self.caller {
                Some(Caller::Stranger(s)) => format!(
                    "The gate. {}, a {}, asks to stay. Enter talks.",
                    s.name,
                    s.trade.name()
                ),
                Some(Caller::Visitor(v)) => {
                    format!("The gate. {}. Enter talks.", sentence_case(v.who()))
                }
                None => "The gate. Nobody here.".to_string(),
            },
            Spot::Homes => {
                let ill: Vec<&str> = self
                    .settlers
                    .iter()
                    .filter(|s| s.ill_days > 0)
                    .map(|s| s.name.as_str())
                    .collect();
                let furniture = self.has(FURNITURE);
                if !ill.is_empty() {
                    format!(
                        "The cottage doors. In bed: {}.",
                        list_words(&ill.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                    )
                } else if furniture > 0 {
                    format!(
                        "The cottage doors. {} to share out.",
                        crate::buildings::count(FURNITURE, furniture)
                    )
                } else {
                    "The cottage doors.".to_string()
                }
            }
            Spot::Hens => {
                if self.hens == 0 {
                    "The hen house, empty. The barge sells hens.".to_string()
                } else if self.eggs_waiting > 0 {
                    format!("The hen house. {} eggs to gather.", self.eggs_waiting)
                } else if !self.fed_hens {
                    "The hen house. The hens want feeding.".to_string()
                } else {
                    "The hen house. All done for today.".to_string()
                }
            }
            Spot::Goat => {
                if !self.goat {
                    "The goat pen, empty. The barge sells goats.".to_string()
                } else if self.milk_waiting {
                    "The goat pen. The goat is ready for milking.".to_string()
                } else if !self.fed_goat {
                    "The goat pen. The goat wants feeding.".to_string()
                } else {
                    "The goat pen. All done for today.".to_string()
                }
            }
            Spot::Table => "The long table, where everyone eats at supper.".to_string(),
            Spot::Site(b) => self.site_line(b),
            Spot::Fishing => "The fishing spot.".to_string(),
            Spot::Dock => {
                if self.barge {
                    "The dock. The barge is in.".to_string()
                } else {
                    format!("The dock. The barge comes in {}.", self.barge_words())
                }
            }
            Spot::Forage => {
                let left = FORAGE_A_DAY.saturating_sub(self.foraged);
                if left == 0 {
                    "The bramble edge, picked clean for today.".to_string()
                } else {
                    "The bramble edge.".to_string()
                }
            }
            Spot::WoodPile => "The fallen trees.".to_string(),
            Spot::Quarry => "The old quarry.".to_string(),
        }
    }

    fn barge_words(&self) -> &'static str {
        match 3 - (self.day % 3 + 2) % 3 {
            1 => "tomorrow",
            _ => "in 2 days",
        }
    }

    pub fn site_line(&self, b: Building) -> String {
        if self.is_built(b) {
            let mut line = sentence_case(b.name());
            if let Some(t) = self.troubles.iter().find(|t| t.stops() == Some(b)) {
                line.push_str(&format!(", {}", t.name()));
            } else if let Some((inputs, (out, _))) = b.recipe() {
                if self.can_make(b) {
                    line.push_str(&format!(", ready to make {}", good_name(out)));
                } else {
                    let need: Vec<String> = inputs
                        .iter()
                        .map(|(id, n)| crate::buildings::count(id, *n))
                        .collect();
                    line.push_str(&format!(", needs {}", need.join(" and ")));
                }
            }
            line.push('.');
            line
        } else if let Some((ob, left)) = self.building {
            if ob == b {
                return format!(
                    "{}, going up. {} hours of work left.",
                    sentence_case(b.name()),
                    left.ceil() as u32
                );
            }
            format!("A free spot for {}.", b.name())
        } else {
            format!("A free spot for {}.", b.name())
        }
    }

    /// A plot in a word or two, for the list of the whole field.
    fn plot_word(&self, i: usize) -> String {
        let p = &self.plots[i];
        match p {
            Plot::Scrub => "rough".to_string(),
            Plot::Tilled => "dug".to_string(),
            Plot::Sown {
                crop: id,
                watered,
                weeds,
                ..
            } => {
                let name = crop(id).map_or("a crop", |c| c.name);
                let mut s = name.to_string();
                if p.ripe() {
                    s.push_str(" ripe");
                } else if !watered {
                    s.push_str(" dry");
                }
                if *weeds {
                    s.push_str(" with weeds");
                }
                s
            }
        }
    }

    pub fn plot_line(&self, r: usize, c: usize) -> String {
        let p = &self.plots[r * COLS + c];
        let state = match p {
            Plot::Scrub => "rough ground".to_string(),
            Plot::Tilled => "dug, ready to sow".to_string(),
            Plot::Sown {
                crop: id,
                growth,
                watered,
                weeds,
            } => {
                let c = crop(id);
                let name = c.map_or("a crop", |c| c.name);
                let mut s = if p.ripe() {
                    format!("{name}, ripe")
                } else if *growth == 0 {
                    format!("{name}, sown")
                } else {
                    format!("{name}, growing")
                };
                if !p.ripe() {
                    s.push_str(if *watered { ", watered" } else { ", dry" });
                }
                if *weeds {
                    s.push_str(", weeds");
                }
                s
            }
        };
        // Rows count from the lane: the first row you step into is row 1.
        format!("Row {}, plot {}: {state}.", ROWS - r, c + 1)
    }

    fn plot_at(&self) -> Option<usize> {
        match self.at {
            At::Plot(r, c) => Some(r * COLS + c),
            _ => None,
        }
    }

    /// Space: the main job for what is in front of you.
    pub fn act(&mut self) {
        if self.phase != Phase::Day {
            return;
        }
        if let Some(i) = self.plot_at() {
            self.act_plot(i);
            return;
        }
        match self.spot() {
            None => {
                let line = format!("{} {} goes in.", self.lane_line(), self.keys.up);
                self.say(line);
            }
            Some(spot) => self.act_spot(spot),
        }
    }

    fn act_plot(&mut self, i: usize) {
        match self.plots[i].clone() {
            Plot::Scrub => {
                self.plots[i] = Plot::Tilled;
                self.events.push(Event::Cue("dig", Place::Fields));
                self.say("Dug. Ready to sow.");
            }
            Plot::Tilled => {
                if self.seeds.values().all(|&n| n == 0) {
                    self.say("No seeds left. The barge sells them.");
                } else {
                    self.events.push(Event::AskSeed);
                }
            }
            Plot::Sown { watered, .. } => {
                if self.plots[i].ripe() {
                    let c = self.harvest_plot(i, false);
                    self.events.push(Event::Cue("harvest", Place::Fields));
                    if let Some(c) = c {
                        let (good, n) = c.yields;
                        self.say(format!(
                            "{} in the larder.",
                            crate::buildings::count(good, n)
                        ));
                    }
                } else if watered {
                    self.say("Already watered today.");
                } else {
                    if let Plot::Sown { watered, .. } = &mut self.plots[i] {
                        *watered = true;
                    }
                    self.events.push(Event::Cue("water", Place::Fields));
                    self.say("Watered.");
                }
            }
        }
    }

    /// Sows a crop in the plot you are on.
    pub fn sow(&mut self, id: &str) {
        let Some(i) = self.plot_at() else {
            return;
        };
        if self.plots[i] != Plot::Tilled {
            return;
        }
        let have = self.seeds.get(id).copied().unwrap_or(0);
        if have == 0 {
            return;
        }
        self.seeds.insert(id.to_string(), have - 1);
        self.plots[i] = Plot::Sown {
            crop: id.to_string(),
            growth: 0,
            watered: self.weather.wet(),
            weeds: false,
        };
        self.events.push(Event::Cue("sow", Place::Fields));
        let name = crop(id).map_or("Seed", |c| c.name);
        let mut line = format!("{} sown.", sentence_case(name));
        if let Some(c) = crop(id) {
            if c.season != self.season() {
                line.push_str(&format!(" It grows slowly out of {}.", c.season.name()));
            }
        }
        self.say(line);
    }

    /// Seeds you have, best for the season first.
    pub fn seed_list(&self) -> Vec<(&'static Crop, u32)> {
        let season = self.season();
        let mut out: Vec<(&'static Crop, u32)> = self
            .seeds
            .iter()
            .filter(|(_, &n)| n > 0)
            .filter_map(|(id, &n)| crop(id).map(|c| (c, n)))
            .collect();
        out.sort_by_key(|(c, _)| (c.season != season, c.days));
        out
    }

    fn act_spot(&mut self, spot: Spot) {
        let place = self.at.place();
        match spot {
            Spot::Gate => self.talk(),
            Spot::Homes => {
                if self.has(FURNITURE) > 0 {
                    self.take(FURNITURE, 1);
                    let names: Vec<usize> = (0..self.settlers.len()).collect();
                    let who = names.into_iter().min_by(|&a, &b| {
                        self.settlers[a]
                            .comfort
                            .total_cmp(&self.settlers[b].comfort)
                    });
                    if let Some(j) = who {
                        let s = &mut self.settlers[j];
                        s.comfort = (s.comfort + 0.4).min(1.0);
                        s.think("new furniture at home", 0.15, 4);
                        let line = format!("{} has new furniture, and loves it.", s.name);
                        self.events.push(Event::Cue("hammer", place));
                        self.say(line);
                    }
                } else {
                    self.say("No furniture to give. The workshop makes it.");
                }
            }
            Spot::Hens => {
                if self.hens == 0 {
                    self.say("No hens yet. The barge sells them.");
                } else if self.eggs_waiting > 0 {
                    let n = self.eggs_waiting;
                    self.eggs_waiting = 0;
                    self.add("eggs", n);
                    self.events.push(Event::Cue("hens", place));
                    self.say(format!("{} gathered.", crate::buildings::count("eggs", n)));
                } else if !self.fed_hens {
                    self.fed_hens = true;
                    self.events.push(Event::Cue("feed", place));
                    self.say("Hens fed. Eggs tomorrow.");
                } else {
                    self.say("The hens are fed, and no eggs yet.");
                }
            }
            Spot::Goat => {
                if self.fix_trouble_at(Place::Cottages) {
                    return;
                }
                if !self.goat {
                    self.say("No goat yet. The barge sells goats.");
                } else if self.milk_waiting {
                    self.milk_waiting = false;
                    self.add("milk", 2);
                    self.events.push(Event::Cue("goat", place));
                    self.say("2 jugs of milk.");
                } else if !self.fed_goat {
                    self.fed_goat = true;
                    self.events.push(Event::Cue("feed", place));
                    self.say("Goat fed. Milk tomorrow.");
                } else {
                    self.say("The goat is fed and milked.");
                }
            }
            Spot::Table => self.say(format!(
                "Supper is at the end of the day. {}",
                self.food_line()
            )),
            Spot::Site(b) => self.act_site(b),
            Spot::Fishing => {
                if self.bite.is_some() {
                    self.bite = None;
                    self.add("fish", 1);
                    self.events.push(Event::Cue("reel", place));
                    self.say("A fish.");
                } else if self.casting.is_some() {
                    self.say("The line is out. Wait for the bite.");
                } else if self.weather == Weather::Storm {
                    self.say("Too rough to fish today.");
                } else {
                    let wait = 1.5 + self.rng().unit() as f32 * 2.5;
                    self.casting = Some(wait);
                    self.events.push(Event::Cue("cast", place));
                }
            }
            Spot::Dock => {
                if self.barge {
                    self.events.push(Event::AskBarge);
                } else {
                    let line = format!("The barge comes in {}.", self.barge_words());
                    self.say(line);
                }
            }
            Spot::Forage => {
                if self.foraged >= FORAGE_A_DAY {
                    self.say("Picked clean for today.");
                    return;
                }
                self.foraged += 1;
                let (id, n) = match self.season() {
                    Season::Spring => ("mushrooms", 1),
                    Season::Summer => ("berries", 2),
                    Season::Autumn => ("berries", 1),
                    Season::Winter => ("wood", 1),
                };
                self.add(id, n);
                self.events.push(Event::Cue("forage", place));
                self.say(format!("{}.", crate::buildings::count(id, n)));
            }
            Spot::WoodPile | Spot::Quarry => {
                let (id, cue) = if spot == Spot::WoodPile {
                    ("wood", "chop")
                } else {
                    ("stone", "pick")
                };
                self.strikes += 1;
                self.events.push(Event::Cue(cue, place));
                if self.strikes >= STRIKES {
                    self.strikes = 0;
                    self.add(id, 1);
                    let line = format!("1 {id}. {} in all.", self.has(id));
                    self.say(line);
                }
            }
        }
    }

    fn act_site(&mut self, b: Building) {
        if self.is_built(b) {
            if self.troubles.iter().any(|t| t.stops() == Some(b)) {
                let _ = self.fix_trouble_at(Place::of(b));
                return;
            }
            if b.recipe().is_none() {
                let line = self.site_line(b);
                self.say(line);
                return;
            }
            match self.make(b) {
                Some((out, n)) => {
                    self.events.push(Event::Cue("cook", Place::of(b)));
                    self.say(format!("{}.", crate::buildings::count(out, n)));
                }
                None => {
                    let line = self.site_line(b);
                    self.say(line);
                }
            }
        } else if let Some((ob, _)) = self.building {
            if ob == b {
                self.build_work(0.25, None);
                if self.building.is_some() {
                    self.events.push(Event::Cue("hammer", Place::of(b)));
                }
            } else {
                self.say(format!(
                    "{} is going up first. One building at a time.",
                    sentence_case(ob.name())
                ));
            }
        } else {
            self.events.push(Event::AskBuild(b));
        }
    }

    /// Whether you can start a building, or why not.
    pub fn can_build(&self, b: Building) -> Result<(), String> {
        if self.is_built(b) {
            return Err(format!("{} is already built.", sentence_case(b.name())));
        }
        if self.building.is_some() {
            return Err("One building at a time.".to_string());
        }
        if let Some(first) = b.needs() {
            if !self.is_built(first) {
                return Err(format!(
                    "{} needs {} first.",
                    sentence_case(b.name()),
                    first.name()
                ));
            }
        }
        let (wood, stone, coins) = b.cost();
        let mut short = Vec::new();
        if self.has("wood") < wood {
            short.push(format!("{} more wood", wood - self.has("wood")));
        }
        if self.has("stone") < stone {
            short.push(format!("{} more stone", stone - self.has("stone")));
        }
        if self.coins < coins {
            short.push(format!("{} more coins", coins - self.coins));
        }
        if short.is_empty() {
            Ok(())
        } else {
            Err(format!("It needs {}.", short.join(" and ")))
        }
    }

    /// Starts a building: the materials go now, settlers build it.
    pub fn start_building(&mut self, b: Building) -> bool {
        if self.can_build(b).is_err() {
            return false;
        }
        let (wood, stone, coins) = b.cost();
        self.take("wood", wood);
        self.take("stone", stone);
        self.coins -= coins;
        self.building = Some((b, b.work()));
        let builder = self
            .settlers
            .iter()
            .max_by(|a, c| {
                a.skills[Skill::Building.index()].total_cmp(&c.skills[Skill::Building.index()])
            })
            .map(|s| s.name.clone());
        let line = match builder {
            Some(n) => format!(
                "Work starts on {}. Settlers on building jobs put it up, and {} is the best builder.",
                b.name(),
                n
            ),
            None => format!("Work starts on {}. {} at the site hammers.", b.name(), self.keys.main),
        };
        self.say(line);
        for i in 0..self.settlers.len() {
            if self.settlers[i].task == Task::Idle {
                self.choose_task(i);
            }
        }
        true
    }

    /// Fixes a trouble here, if there is one: true when it did.
    fn fix_trouble_at(&mut self, p: Place) -> bool {
        let Some(i) = self.troubles.iter().position(|t| t.place() == p) else {
            return false;
        };
        let t = self.troubles.remove(i);
        let cue = if t == Trouble::Crows {
            "weed"
        } else {
            "hammer"
        };
        self.events.push(Event::Cue(cue, p));
        let line = t.fixed("You");
        self.say(line);
        true
    }

    /// Tab: the second job: weeds, feeding.
    pub fn tend(&mut self) {
        if self.phase != Phase::Day {
            return;
        }
        if let Some(i) = self.plot_at() {
            if self.fix_trouble_at(Place::Fields) {
                return;
            }
            match &mut self.plots[i] {
                Plot::Sown { weeds, .. } if *weeds => {
                    *weeds = false;
                    self.events.push(Event::Cue("weed", Place::Fields));
                    self.say("Weeds pulled.");
                }
                _ => self.say("No weeds here."),
            }
            return;
        }
        match self.spot() {
            Some(Spot::Hens) if self.hens > 0 && !self.fed_hens => {
                self.fed_hens = true;
                self.events.push(Event::Cue("feed", Place::Cottages));
                self.say("Hens fed.");
            }
            Some(Spot::Goat) if self.goat && !self.fed_goat => {
                self.fed_goat = true;
                self.events.push(Event::Cue("feed", Place::Cottages));
                self.say("Goat fed.");
            }
            _ => self.act(),
        }
    }

    /// Enter: talk to whoever is here.
    pub fn talk(&mut self) {
        let place = self.at.place();
        if place == Place::Gate {
            match self.caller.clone() {
                Some(Caller::Stranger(s)) => {
                    let line = crate::talk::arrival(&s);
                    self.say(line);
                    self.events.push(Event::AskArrival);
                    return;
                }
                Some(Caller::Visitor(v)) => {
                    self.caller = None;
                    let line = almanac::visit(self, v);
                    self.say(line);
                    return;
                }
                None => {}
            }
        }
        // A wobbling settler first, then the one who has gone longest
        // without a word.
        let here: Vec<usize> = (0..self.settlers.len())
            .filter(|&i| self.settlers[i].place() == place && !self.settlers[i].away())
            .collect();
        let pick = here
            .iter()
            .copied()
            .find(|&i| self.settlers[i].wobble.is_some())
            .or_else(|| {
                here.iter()
                    .copied()
                    .find(|&i| !self.settlers[i].talked_today)
            })
            .or_else(|| here.first().copied());
        let Some(i) = pick else {
            self.say(format!("Nobody at {} to talk to.", place.name()));
            return;
        };
        let mut rng = self.rng();
        let s = &mut self.settlers[i];
        let line = if s.wobble.is_some() {
            let line = crate::talk::talked_round(s);
            s.wobble = None;
            s.low_days = 0;
            s.think("talked round", 0.25, 2);
            line
        } else {
            crate::talk::chat(s, &mut rng)
        };
        if !s.talked_today {
            s.talked_today = true;
            s.company = (s.company + 0.2).min(1.0);
        }
        self.say(line);
        self.events.push(Event::Cue("talk", place));
    }

    /// Takes in the stranger at the gate.
    pub fn answer_arrival(&mut self, yes: bool) {
        let Some(Caller::Stranger(s)) = self.caller.take() else {
            return;
        };
        if yes {
            let line = format!("{} moves into a cottage.", s.name);
            self.welcome(s);
            self.say(line);
        } else {
            self.say(format!(
                "{} thanks you, and walks on. They may come back.",
                s.name
            ));
            self.turned_away.push(s.name.clone());
        }
    }

    // --- the barge --------------------------------------------------------

    /// Sells one of a good.
    pub fn sell(&mut self, id: &str) -> bool {
        let Some(g) = good(id) else {
            return false;
        };
        if self.take(id, 1) {
            self.coins += g.sells + u32::from(self.fair_day);
            true
        } else {
            false
        }
    }

    /// What the barge sells: id, what it is, its price.
    pub fn barge_stock(&self) -> Vec<(String, String, u32)> {
        let mut out: Vec<(String, String, u32)> = of_season(self.season())
            .chain(of_season(self.season().next()).take(2))
            .map(|c| {
                (
                    format!("seed:{}", c.id),
                    format!("a {} seed", c.name),
                    c.price,
                )
            })
            .collect();
        out.push((NAILS.to_string(), "a nail".to_string(), 1));
        if self.hens < 6 {
            out.push(("hen".to_string(), "a hen".to_string(), 8));
        }
        if !self.goat {
            out.push(("goat".to_string(), "a goat".to_string(), 15));
        }
        out
    }

    /// Buys one thing from the barge.
    pub fn buy(&mut self, id: &str) -> Result<String, String> {
        let Some((_, what, price)) = self.barge_stock().into_iter().find(|(k, _, _)| k == id)
        else {
            return Err("The barge has none of that.".to_string());
        };
        if self.coins < price {
            return Err(format!(
                "{} costs {price} coins. You have {}.",
                sentence_case(&what),
                self.coins
            ));
        }
        self.coins -= price;
        if let Some(c) = id.strip_prefix("seed:") {
            self.add_seeds(c, 1);
        } else if id == "hen" {
            self.hens += 1;
        } else if id == "goat" {
            self.goat = true;
        } else {
            // Bought, not made: it stays out of the day's tally.
            *self.larder.entry(id.to_string()).or_insert(0) += 1;
        }
        Ok(format!("Bought {what}. {} coins left.", self.coins))
    }

    // --- supper and bed -----------------------------------------------------

    /// Supper: everyone eats, the day's talk, then bed. Returns the pages.
    pub fn supper(&mut self, seat: Option<&str>) -> Vec<String> {
        if self.phase != Phase::Supper {
            return Vec::new();
        }
        let mut pages = Vec::new();
        let feast = self.festival_today();
        let (meal, good) = self.eat(feast);
        pages.push(meal);
        if let Some(name) = seat {
            if let Some(i) = self.settlers.iter().position(|s| s.name == name) {
                let s = &mut self.settlers[i];
                s.company = (s.company + 0.3).min(1.0);
                s.think("sat with you at supper", 0.1, 1);
            }
        }
        for s in self.settlers.iter_mut() {
            if s.away() {
                continue;
            }
            if good {
                s.think("a good supper", 0.15, 1);
            } else {
                s.think("a thin supper", -0.2, 1);
            }
            if s.has(Trait::Musician) {
                s.company = (s.company + 0.1).min(1.0);
            }
        }
        if self.is_built(Building::Bandstand) && self.settlers.len() > 1 {
            for s in self.settlers.iter_mut() {
                s.think("music on the green", 0.1, 1);
                s.company = (s.company + 0.1).min(1.0);
            }
        }
        if feast {
            self.festivals += 1;
            let why = self.season_festival_lower();
            for s in self.settlers.iter_mut() {
                s.think(&why, 0.3, 2);
            }
        }
        if self.music_night {
            for s in self.settlers.iter_mut() {
                s.think("music night on the green", 0.15, 1);
                s.company = (s.company + 0.1).min(1.0);
            }
        }
        let mut rng = self.rng();
        let talk = crate::talk::supper_talk(self, seat, &mut rng);
        pages.extend(talk);
        self.wants_at_supper(&mut pages);
        let bed = self.end_day();
        pages.extend(bed);
        pages
    }

    fn season_festival_lower(&self) -> String {
        self.season().festival().to_lowercase()
    }

    /// Everyone eats from the larder. The line, and whether it was good.
    fn eat(&mut self, feast: bool) -> (String, bool) {
        let need = MEAL * self.mouths();
        let mut eaten: Vec<&'static str> = Vec::new();
        let mut got = 0;
        // Eat the best food first, a little of many things.
        // The barge's order is kept back from supper.
        let ordered: Vec<String> = self
            .order
            .as_ref()
            .map(|o| o.goods.iter().map(|(id, _)| id.clone()).collect())
            .unwrap_or_default();
        let mut menu: Vec<&'static crate::buildings::Good> = GOODS
            .iter()
            .filter(|g| g.food > 0 && self.has(g.id) > 0 && !ordered.iter().any(|o| o == g.id))
            .collect();
        menu.sort_by_key(|g| std::cmp::Reverse(g.food));
        let mut round = 0;
        while got < need && round < 20 {
            let mut any = false;
            for g in &menu {
                if got >= need {
                    break;
                }
                if self.take(g.id, 1) {
                    got += g.food;
                    any = true;
                    if !eaten.contains(&g.name) {
                        eaten.push(g.name);
                    }
                }
            }
            if !any {
                break;
            }
            round += 1;
        }
        let enough = got >= need;
        for s in self.settlers.iter_mut() {
            s.food = if enough { 1.0 } else { (s.food + 0.5).min(1.0) };
        }
        let fine = enough && (eaten.len() >= 2);
        let what = if eaten.is_empty() {
            "nothing from the larder".to_string()
        } else {
            list_words(
                &eaten
                    .iter()
                    .take(3)
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            )
        };
        let mut line = if feast {
            format!("{}. A feast of {what}.", self.season().festival())
        } else {
            format!("Supper: {what}.")
        };
        if !enough {
            line.push_str(" A thin supper. The next village sends bread over.");
            self.add("bread", 2);
        } else if fine {
            line.push_str(" A good meal.");
        }
        (line, fine)
    }

    fn wants_at_supper(&mut self, pages: &mut Vec<String>) {
        for i in 0..self.settlers.len() {
            let Some(w) = self.settlers[i].want else {
                continue;
            };
            let (what, needs) = WANTS[w % WANTS.len()];
            if self.met(needs, true) {
                let s = &mut self.settlers[i];
                s.want = None;
                s.think(&format!("got {what}"), 0.3, 3);
                pages.push(format!("{} has {what}. Cheerful.", s.name));
            }
        }
    }

    /// Whether something a story or a want needs is here. `use_up` takes
    /// goods from the larder.
    fn met(&mut self, needs: Needs, use_up: bool) -> bool {
        match needs {
            Needs::Building(b) => self.is_built(b),
            Needs::Goods(id, n) => {
                if self.has(id) >= n {
                    if use_up && id != "wood" {
                        self.take(id, n);
                    }
                    true
                } else {
                    false
                }
            }
            Needs::Settlers(n) => self.settlers.len() as u32 >= n,
            Needs::Festival => self.festivals > 0,
            Needs::Cheerful => true,
        }
    }

    /// Bed: crops grow, moods settle, stories move on, a new day.
    fn end_day(&mut self) -> Vec<String> {
        let mut pages = Vec::new();
        // Crops grow overnight.
        let season = self.season();
        let halt = self.weather == Weather::Frost || self.troubles.contains(&Trouble::Crows);
        let green = self.settlers.iter().any(|s| s.has(Trait::GreenFingers));
        let mut night_rng = self.rng();
        for p in self.plots.iter_mut() {
            if let Plot::Sown {
                crop: id,
                growth,
                watered,
                weeds,
            } = p
            {
                let c = crop(id);
                let in_season = c.is_some_and(|c| c.season == season);
                let ok = *watered && !*weeds && !halt;
                if ok && (in_season || night_rng.chance(0.5)) {
                    *growth += 1;
                    if green && night_rng.chance(0.2) {
                        *growth += 1;
                    }
                }
                *watered = false;
                if !*weeds && night_rng.chance(0.12) {
                    *weeds = true;
                }
            }
        }
        // Moods and stories.
        let mut story_lines = Vec::new();
        for i in 0..self.settlers.len() {
            let s = &mut self.settlers[i];
            for t in s.thoughts.iter_mut() {
                t.days = t.days.saturating_sub(1);
            }
            s.thoughts.retain(|t| t.days > 0);
            if s.ill_days > 0 {
                s.ill_days -= 1;
            }
            let glum = s.mood() == Mood::Glum;
            s.low_days = if glum { s.low_days + 1 } else { 0 };
            if s.want.is_none() && night_rng.chance(0.25) {
                s.want = Some(night_rng.below(WANTS.len() as u32) as usize);
            }
            let step = s.story_step as usize;
            if step < 3 {
                let (needs, line) = s.trade.story().steps[step];
                let cheerful = s.mood() == Mood::Cheerful;
                let name = s.name.clone();
                let ok = match needs {
                    Needs::Cheerful => cheerful,
                    other => self.met(other, false),
                };
                if ok && self.day > self.settlers[i].arrived {
                    let s = &mut self.settlers[i];
                    s.story_step += 1;
                    s.think("a step closer to a dream", 0.25, 3);
                    story_lines.push(line.replace("{name}", &name));
                    if s.story_step == 3 {
                        let keepsake = s.trade.story().keepsake.to_string();
                        self.keepsakes.push(keepsake.clone());
                        self.events.push(Event::Keepsake(keepsake.clone()));
                        story_lines.push(format!("On your shelf: {keepsake}."));
                        for o in self.settlers.iter_mut() {
                            o.think(&format!("{name}'s good news"), 0.15, 2);
                        }
                    }
                }
            }
        }
        // Friendships move with the day.
        let n = self.settlers.len();
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let same = self.settlers[i].place() == self.settlers[j].place();
                let delta = if same { 1 } else { 0 };
                let other = self.settlers[j].name.clone();
                bump_liking(&mut self.settlers, i, &other, delta);
            }
        }
        let well = self.is_built(Building::Well);
        for s in self.settlers.iter_mut() {
            let friends = s.liking.values().filter(|&&v| v >= 5).count();
            let rows = s.liking.values().filter(|&&v| v <= -3).count();
            if friends > 0 {
                s.think("good friends here", 0.1, 1);
            }
            if rows > 0 {
                s.think("bickering with someone", -0.15, 1);
            }
            s.comfort = (s.comfort - 0.03).max(0.0);
            if well {
                s.comfort = (s.comfort + 0.02).min(1.0);
            }
        }
        let tally = self.tally_line();
        pages.extend(story_lines.iter().cloned());
        pages.push(tally);
        // Challenge goals.
        self.check_goal();
        self.day += 1;
        if let Some(limit) = self.day_limit {
            if self.day > limit && !self.goal_met {
                self.phase = Phase::Over;
                return pages;
            }
        }
        if self.goal_met && self.kind != Kind::Home {
            self.phase = Phase::Over;
            return pages;
        }
        self.begin_day();
        self.news.extend(story_lines);
        pages
    }

    fn tally_line(&self) -> String {
        let made: Vec<String> = self
            .tally
            .iter()
            .take(4)
            .map(|(id, n)| crate::buildings::count(id, *n))
            .collect();
        if made.is_empty() {
            format!("Day {} done.", self.day_of_season())
        } else {
            format!(
                "Day {} done. Brought in today: {}.",
                self.day_of_season(),
                list_words(&made)
            )
        }
    }

    /// The challenge's goal.
    pub fn check_goal(&mut self) {
        if self.goal_met {
            return;
        }
        let met = match self.kind {
            Kind::Challenge(Goal::Market) => match &self.order {
                Some(o) => o.goods.iter().all(|(id, n)| self.has(id) >= *n),
                None => false,
            },
            Kind::Challenge(Goal::Cheer) => {
                !self.settlers.is_empty()
                    && self.settlers.iter().all(|s| s.mood() == Mood::Cheerful)
            }
            Kind::Challenge(Goal::Founders) => {
                self.is_built(Building::Mill)
                    && self.is_built(Building::Bakery)
                    && self.festivals > 0
            }
            _ => false,
        };
        if met {
            if let Some(o) = self.order.as_mut() {
                o.filled = true;
            }
            self.goal_met = true;
            self.events.push(Event::GoalMet);
        }
    }

    /// A challenge's score: goods made, moods, the goal, days to spare.
    pub fn score(&self) -> u64 {
        let goods: u32 = self
            .larder
            .iter()
            .filter(|(id, _)| *id != "wood" && *id != "stone")
            .map(|(_, n)| *n)
            .sum();
        let cheerful = self
            .settlers
            .iter()
            .filter(|s| s.mood() == Mood::Cheerful)
            .count() as u64;
        let mut score = goods as u64 * 2 + cheerful * 20 + self.built.len() as u64 * 30;
        if self.goal_met {
            score += 100;
            if let Some(limit) = self.day_limit {
                score += (limit + 1).saturating_sub(self.day) as u64 * 10;
            }
        }
        score
    }

    /// Where the score came from, as one spoken line.
    pub fn score_line(&self) -> String {
        let goods: u32 = self
            .larder
            .iter()
            .filter(|(id, _)| *id != "wood" && *id != "stone")
            .map(|(_, n)| *n)
            .sum();
        let cheerful = self
            .settlers
            .iter()
            .filter(|s| s.mood() == Mood::Cheerful)
            .count() as u64;
        let mut parts = vec![format!("{} for {goods} goods in the larder", goods * 2)];
        if cheerful > 0 {
            parts.push(format!("{} for {cheerful} cheerful", cheerful * 20));
        }
        if !self.built.is_empty() {
            let n = self.built.len();
            parts.push(format!(
                "{} for {n} {}",
                n * 30,
                if n == 1 { "building" } else { "buildings" }
            ));
        }
        if self.goal_met {
            parts.push("100 for the goal".to_string());
            if let Some(limit) = self.day_limit {
                let spare = (limit + 1).saturating_sub(self.day);
                if spare > 0 {
                    parts.push(format!(
                        "{} for {spare} {} to spare",
                        spare * 10,
                        if spare == 1 { "day" } else { "days" }
                    ));
                }
            }
        }
        format!("Points: {}.", list_words(&parts))
    }

    // --- status -------------------------------------------------------------

    /// What needs you now, most urgent first.
    pub fn needs_you(&self) -> Vec<String> {
        let mut out = Vec::new();
        match &self.caller {
            Some(Caller::Stranger(s)) => {
                out.push(format!("{} is at the gate, asking to stay.", s.name))
            }
            Some(Caller::Visitor(v)) => {
                out.push(format!("{} is at the gate.", sentence_case(v.who())))
            }
            None => {}
        }
        for s in &self.settlers {
            if let Some(w) = s.wobble {
                if w != Wobble::WalkedOff {
                    out.push(format!("{} is {}.", s.name, w.doing()));
                }
            }
        }
        for t in &self.troubles {
            out.push(format!("{}.", sentence_case(&t.name())));
        }
        if self.bite.is_some() {
            out.insert(0, "A bite on the line.".to_string());
        }
        out
    }

    /// S, in three layers.
    pub fn status_layers(&self) -> Vec<String> {
        let mut first = self.needs_you();
        if first.is_empty() {
            first.push("Nothing needs you.".to_string());
        }
        first.push(self.food_line());
        if let Some(o) = &self.order {
            let left: Vec<String> = o
                .goods
                .iter()
                .filter(|(id, n)| self.has(id) < *n)
                .map(|(id, n)| crate::buildings::count(id, n - self.has(id)))
                .collect();
            if !left.is_empty() {
                first.push(format!("The order still needs {}.", list_words(&left)));
            }
        }
        let mut second = vec![format!(
            "Day {} of {}, year {}, {}. {}.",
            self.day_of_season(),
            self.season().name(),
            self.year(),
            self.time_words(),
            sentence_case(self.weather.name())
        )];
        if let Some(limit) = self.day_limit {
            let left = (limit + 1).saturating_sub(self.day);
            second.push(format!(
                "{left} {} left.",
                if left == 1 { "day" } else { "days" }
            ));
        }
        second.push(self.where_line());
        if let Some((b, left)) = self.building {
            second.push(format!(
                "{} is going up, {} hours of work left.",
                sentence_case(b.name()),
                left.ceil() as u32
            ));
        }
        second.push(self.store_line());
        let mut third = Vec::new();
        if self.settlers.is_empty() {
            third.push("Nobody lives here yet.".to_string());
        }
        for s in &self.settlers {
            third.push(format!("{}, {}, {}.", s.name, s.mood().name(), s.doing()));
        }
        vec![first.join(" "), second.join(" "), third.join(" ")]
    }

    pub fn where_line(&self) -> String {
        match self.at {
            At::Lane(p) => format!("On the lane by {}.", p.name()),
            At::In(..) => self.spot_line(),
            At::Plot(r, c) => self.plot_line(r, c),
        }
    }

    /// H: the nearest thing worth doing, and the key.
    pub fn hint(&self) -> String {
        if let Some(Caller::Stranger(s)) = &self.caller {
            return format!(
                "{} waits at the gate. Go to the gate and press Enter to meet them.",
                s.name
            );
        }
        if let Some(s) = self
            .settlers
            .iter()
            .find(|s| s.wobble.is_some() && !s.away())
        {
            return format!(
                "{} is {}, {}. Go there and press Enter to talk {} round.",
                s.name,
                s.wobble.map_or("", |w| w.doing()),
                s.place().where_at(),
                s.name
            );
        }
        if self.bite.is_some() {
            return format!("A bite. {} reels it in.", self.keys.main);
        }
        if let Some(t) = self.troubles.first() {
            return format!("{}. {}", sentence_case(&t.name()), t.hint(&self.keys));
        }
        if self.plots.iter().any(|p| p.ripe()) {
            return format!(
                "Something is ripe in the fields. {} harvests it.",
                self.keys.main
            );
        }
        if !self.is_built(Building::Well) && self.plots.iter().any(|p| p.needs_water()) {
            return format!("Plots in the fields are dry. {} waters. Once the well is built, settlers water too.", self.keys.main);
        }
        if self.plots.contains(&Plot::Tilled) && self.seeds.values().any(|&n| n > 0) {
            return format!(
                "A dug plot is ready in the fields. {} on it sows a seed.",
                self.keys.main
            );
        }
        if self.eggs_waiting > 0 {
            return format!(
                "Eggs at the hen house, by the cottages. {} gathers them.",
                self.keys.main
            );
        }
        if self.barge {
            return format!(
                "The barge is at the river dock. {} there trades.",
                self.keys.main
            );
        }
        if self.food_days() < 2 {
            return "Food is low. Fish at the river, forage in the woods, or harvest.".to_string();
        }
        if self.building.is_none() {
            if let Some(b) = Building::ALL
                .into_iter()
                .find(|b| self.can_build(*b).is_ok())
            {
                return format!(
                    "You have what {} needs. It goes on {}.",
                    b.name(),
                    Place::of(b).name()
                );
            }
            if let Some(b) = Building::ALL.into_iter().find(|b| !self.is_built(*b)) {
                let why = self.can_build(b).err().unwrap_or_default();
                return format!("Nothing needs you. Next, {}: {why}", b.name());
            }
        }
        "Nothing needs you. Supper is at the end of the day.".to_string()
    }

    /// L: the place you are in, or the lane.
    pub fn list(&self) -> String {
        match self.at {
            // Along the lane: who is where, from the gate to the woods.
            At::Lane(_) => {
                let mut out: Vec<String> = Place::ALL
                    .iter()
                    .filter_map(|p| {
                        let names: Vec<String> =
                            self.people_at(*p).iter().map(|s| s.name.clone()).collect();
                        (!names.is_empty())
                            .then(|| format!("{} {}.", list_words(&names), p.where_at()))
                    })
                    .collect();
                if out.is_empty() {
                    out.push("Nobody is out on the lane.".to_string());
                }
                if self.barge {
                    out.push("The barge is at the river.".to_string());
                }
                out.join(" ")
            }
            At::In(p, _) => {
                let mut out: Vec<String> = Vec::new();
                for (i, _) in p.spots().iter().enumerate() {
                    let h = Hamlet {
                        at: At::In(p, i),
                        events: Vec::new(),
                        ..self.clone()
                    };
                    out.push(h.spot_line());
                }
                out.join(" ")
            }
            // One short line a row, from the lane in, naming only what needs doing.
            At::Plot(..) => (0..ROWS)
                .rev()
                .map(|r| {
                    let plots: Vec<String> =
                        (0..COLS).map(|c| self.plot_word(r * COLS + c)).collect();
                    format!("Row {}: {}.", ROWS - r, plots.join(", "))
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    pub fn repeat(&mut self) {
        let t = if self.last.is_empty() {
            "Nothing to repeat yet.".to_string()
        } else {
            self.last.clone()
        };
        self.events.push(Event::Say(t));
    }
}

/// How two people start out, from their traits.
fn liking_between(a: &Settler, b: &Settler) -> i32 {
    let mut v = 0;
    if a.has(Trait::Chatterbox) && b.has(Trait::Quiet)
        || a.has(Trait::Quiet) && b.has(Trait::Chatterbox)
    {
        v -= 3;
    }
    if a.has(Trait::Tidy) && b.has(Trait::Clumsy) {
        v -= 2;
    }
    if a.has(Trait::Grumbler) && b.has(Trait::Grumbler) {
        v -= 2;
    }
    if a.has(Trait::Musician) && b.has(Trait::Musician) {
        v += 3;
    }
    if a.has(Trait::Cheerful) || b.has(Trait::Cheerful) {
        v += 1;
    }
    v
}

fn bump_liking(settlers: &mut [Settler], i: usize, other: &str, delta: i32) {
    if delta == 0 {
        return;
    }
    let v = settlers[i].liking.entry(other.to_string()).or_insert(0);
    *v = (*v + delta).clamp(-10, 10);
}

/// "a, b and c".
pub fn list_words(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

pub fn sentence_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
