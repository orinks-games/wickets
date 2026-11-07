//! The almanac: what each morning brings. Weather turns, people come to
//! the gate, small troubles turn up, and a low mood that lasts becomes a
//! wobble.
//!
//! Past Gentle, there are dangers too: a fire, a fox, a flood, a hard
//! winter, someone lost in the woods. Each can cost something you can win
//! back, crops, a hen, a building's repair, if nobody deals with it in
//! time. Nobody ever dies. A settler who stays glum too long warns you
//! twice, then leaves.
//!
//! The pace in Settings sets how often all this happens, and how harsh.

use serde::{Deserialize, Serialize};

use crate::buildings::{Building, NAILS};
use crate::crops::{of_season, Season};
use crate::people::Trait;
use crate::rules::{Caller, Hamlet, Mood, Place, Weather, Wobble, MAX_SETTLERS};

/// How busy the almanac is, in Settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pace {
    Gentle,
    #[default]
    Steady,
    Lively,
}

impl Pace {
    pub const ALL: [Pace; 3] = [Pace::Gentle, Pace::Steady, Pace::Lively];

    pub fn name(self) -> &'static str {
        match self {
            Pace::Gentle => "gentle",
            Pace::Steady => "steady",
            Pace::Lively => "lively",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Pace::Gentle => "Few troubles, no dangers, kind weather, and nobody leaves.",
            Pace::Steady => {
                "Some dangers that can cost you, and a settler kept glum too long may leave."
            }
            Pace::Lively => "More visitors, dangers and storms, and glum settlers leave sooner.",
        }
    }

    /// How often troubles and visitors come, against Steady.
    fn often(self) -> f64 {
        match self {
            Pace::Gentle => 0.5,
            Pace::Steady => 1.0,
            Pace::Lively => 1.5,
        }
    }

    /// The chance of a danger each day: none when gentle.
    pub fn danger(self) -> f64 {
        match self {
            Pace::Gentle => 0.0,
            Pace::Steady => 0.15,
            Pace::Lively => 0.25,
        }
    }

    /// Glum days in a row before a settler leaves, if they ever do.
    pub fn leave_after(self) -> Option<u32> {
        match self {
            Pace::Gentle => None,
            Pace::Steady => Some(7),
            Pace::Lively => Some(5),
        }
    }

    /// Low days before a mood turns into a wobble.
    fn wobble_after(self) -> u32 {
        match self {
            Pace::Gentle => 3,
            Pace::Steady => 2,
            Pace::Lively => 2,
        }
    }
}

/// Buckets of water a fire takes to put out.
pub const FIRE_BUCKETS: u32 = 6;
/// Hours before a fire burns through, or a fox gets a hen.
pub const FIRE_HOURS: f32 = 2.0;
pub const FOX_HOURS: f32 = 1.5;
/// Wood a burnt building takes to repair, and stone to hold back a flood.
pub const REPAIR_WOOD: u32 = 3;
pub const FLOOD_STONE: u32 = 2;

/// A trouble stops one thing until someone fixes it. A danger also costs
/// something if nobody deals with it in time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Trouble {
    /// Crows in the fields: crops do not grow until they are scared off.
    Crows,
    /// The fence by the goat pen is down.
    Fence,
    /// A building's roof leaks: it makes nothing until it is mended.
    Leak(Building),
    /// A building is on fire: buckets left, and hours before it burns.
    Fire {
        b: Building,
        buckets: u32,
        left: f32,
    },
    /// A building the fire burnt through: it makes nothing until repaired.
    Burnt(Building),
    /// A fox by the hen house: hours before it takes a hen.
    Fox { left: f32 },
    /// The river is rising: by supper it takes the far row of crops.
    Flood,
    /// Someone lost in the woods at dusk.
    Lost(String),
}

impl Trouble {
    /// The building it stops, if any.
    pub fn stops(&self) -> Option<Building> {
        match self {
            Trouble::Leak(b) | Trouble::Burnt(b) | Trouble::Fire { b, .. } => Some(*b),
            _ => None,
        }
    }

    /// Where it is fixed.
    pub fn place(&self) -> Place {
        match self {
            Trouble::Crows => Place::Fields,
            Trouble::Fence | Trouble::Fox { .. } => Place::Cottages,
            Trouble::Leak(b) | Trouble::Burnt(b) | Trouble::Fire { b, .. } => Place::of(*b),
            Trouble::Flood => Place::River,
            Trouble::Lost(_) => Place::Woods,
        }
    }

    /// A danger, as against a small trouble: it costs something if left.
    pub fn danger(&self) -> bool {
        matches!(
            self,
            Trouble::Fire { .. } | Trouble::Fox { .. } | Trouble::Flood | Trouble::Lost(_)
        )
    }

    /// Settlers on building jobs can see to it. Only you can find someone lost.
    pub fn settlers_fix(&self) -> bool {
        !matches!(self, Trouble::Lost(_))
    }

    pub fn name(&self) -> String {
        match self {
            Trouble::Crows => "crows in the fields".to_string(),
            Trouble::Fence => "the fence by the goat pen is down".to_string(),
            Trouble::Leak(b) => format!("the roof of {} leaks", b.name()),
            Trouble::Fire { b, buckets, .. } => {
                format!("fire in {}, {buckets} buckets to put it out", b.name())
            }
            Trouble::Burnt(b) => format!("{} is burnt and needs repairing", b.name()),
            Trouble::Fox { .. } => "a fox by the hen house".to_string(),
            Trouble::Flood => "the river is rising toward the fields".to_string(),
            Trouble::Lost(n) => format!("{n} is lost in the woods"),
        }
    }

    /// The line when it is fixed, by a settler's name or "You".
    pub fn fixed(&self, by: &str) -> String {
        match self {
            Trouble::Crows => format!("{by} scared the crows off the fields."),
            Trouble::Fence => format!("{by} put the fence by the goat pen back up."),
            Trouble::Leak(b) => format!("{by} mended the roof of {}.", b.name()),
            Trouble::Fire { b, .. } => format!("The fire in {} is out.", b.name()),
            Trouble::Burnt(b) => format!("{by} repaired {}. It works again.", b.name()),
            Trouble::Fox { .. } => format!("{by} chased the fox off. The hens are safe."),
            Trouble::Flood => format!("{by} piled stone along the bank. The fields are safe."),
            Trouble::Lost(n) if by == "You" => {
                format!("You find {n} by the old quarry, and walk home together.")
            }
            Trouble::Lost(n) => format!("{by} found {n} in the woods."),
        }
    }

    pub fn hint(&self, keys: &crate::rules::Keys) -> String {
        match self {
            Trouble::Crows => format!(
                "{} in the fields scares them off. Until then, nothing grows.",
                keys.second
            ),
            Trouble::Fence => format!(
                "{} at the goat pen, by the cottages, puts it back up.",
                keys.main
            ),
            Trouble::Leak(_) => format!(
                "{} at the building mends it. Until then, it makes nothing.",
                keys.main
            ),
            Trouble::Fire { b, .. } => format!(
                "{} at {} throws a bucket of water. Settlers on building jobs help. Put it out soon, or it burns through.",
                keys.main,
                b.name()
            ),
            Trouble::Burnt(b) => format!(
                "{} at {} repairs it with {REPAIR_WOOD} wood. Settlers on building jobs repair it too.",
                keys.main,
                b.name()
            ),
            Trouble::Fox { .. } => format!(
                "{} at the hen house, by the cottages, chases it off. Be quick, or it takes a hen.",
                keys.main
            ),
            Trouble::Flood => format!(
                "{} at the dock piles {FLOOD_STONE} stone along the bank. Do it before supper, or the far row of crops is lost.",
                keys.main
            ),
            Trouble::Lost(n) => format!(
                "Go into the woods and press Enter to find {n} before supper."
            ),
        }
    }
}

/// Someone passing through, who stops at the gate for a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visitor {
    Tinker,
    Postman,
    Walker,
    SeedSeller,
    Busker,
}

impl Visitor {
    pub const ALL: [Visitor; 5] = [
        Visitor::Tinker,
        Visitor::Postman,
        Visitor::Walker,
        Visitor::SeedSeller,
        Visitor::Busker,
    ];

    pub fn who(&self) -> &'static str {
        match self {
            Visitor::Tinker => "a tinker with a cart of pots",
            Visitor::Postman => "the postman, with a letter",
            Visitor::Walker => "a tired walker",
            Visitor::SeedSeller => "a seed seller",
            Visitor::Busker => "a busker with a squeezebox",
        }
    }
}

/// The visitor's word at the gate, and what they leave.
pub fn visit(h: &mut Hamlet, v: Visitor) -> String {
    let inn = h.is_built(Building::Inn);
    let tip = if inn {
        h.coins += 3;
        " They stay the night at the inn, and pay 3 coins."
    } else {
        ""
    };
    let line = match v {
        Visitor::Tinker => {
            h.add(NAILS, 3);
            "The tinker mends a kettle and leaves you 3 nails.".to_string()
        }
        Visitor::Postman => {
            let mut rng = h.rng();
            let homesick: Vec<usize> = (0..h.settlers.len())
                .filter(|&i| h.settlers[i].has(Trait::Homesick))
                .collect();
            let pick = if homesick.is_empty() {
                (!h.settlers.is_empty()).then(|| rng.below(h.settlers.len() as u32) as usize)
            } else {
                Some(homesick[rng.below(homesick.len() as u32) as usize])
            };
            match pick {
                Some(i) => {
                    let s = &mut h.settlers[i];
                    let lift = if s.has(Trait::Homesick) { 0.4 } else { 0.2 };
                    s.think("a letter from home", lift, 3);
                    format!("A letter for {}, from home. They read it twice.", s.name)
                }
                None => {
                    "A letter for you, from an old friend. They hope Larkfield grows.".to_string()
                }
            }
        }
        Visitor::Walker => {
            let food = ["bread", "pie", "cheese", "potato", "turnip"]
                .into_iter()
                .find(|id| h.has(id) > 0);
            match food {
                Some(id) => {
                    h.take(id, 1);
                    for s in h.settlers.iter_mut() {
                        s.think("helped a walker", 0.1, 2);
                    }
                    format!(
                        "You share {} with the walker, who tells everyone about Larkfield on the way.",
                        crate::buildings::count(id, 1)
                    )
                }
                None => "The walker rests at the gate for a while, then goes on.".to_string(),
            }
        }
        Visitor::SeedSeller => {
            let next = h.season().next();
            let mut rng = h.rng();
            let crops: Vec<&'static str> = of_season(next).map(|c| c.id).collect();
            let id = *rng.pick(&crops);
            h.add_seeds(id, 2);
            let name = crate::crops::crop(id).map_or("seed", |c| c.name);
            format!(
                "The seed seller gives you 2 {name} seeds for {}.",
                next.name()
            )
        }
        Visitor::Busker => {
            for s in h.settlers.iter_mut() {
                s.think("a tune at the gate", 0.15, 1);
                s.company = (s.company + 0.15).min(1.0);
            }
            "The busker plays a jig at the gate. Everyone stops work to listen.".to_string()
        }
    };
    format!("{line}{tip}")
}

/// The morning: the weather turns, someone may come, a trouble may turn
/// up, and low moods may wobble. Adds lines to the morning news.
pub fn morning(h: &mut Hamlet) {
    let often = h.pace.often();
    let season = h.season();
    // Stormier when lively, kinder when gentle.
    if h.weather == Weather::Rain {
        if h.pace == Pace::Lively && h.chance(0.35) {
            h.weather = Weather::Storm;
        } else if h.pace == Pace::Gentle && h.chance(0.4) {
            h.weather = Weather::Cloudy;
        }
    }

    // Someone at the gate.
    if h.caller.is_none() {
        let room = h.settlers.len() < MAX_SETTLERS;
        let fed = h.food_days() >= 1 || h.settlers.len() < 2;
        // Word spreads after a festival: someone always comes the next day.
        let after_festival = h.day > 1 && h.day_of_season() == 1;
        let first_guest = h.day == 2 && h.settlers.len() < 2;
        let arrive_chance = 0.2 * often.min(1.2);
        if room && (first_guest || after_festival || fed && h.day > 2 && h.chance(arrive_chance)) {
            let s = h.make_settler(None);
            h.news.push(format!(
                "Someone is at the gate: {}, asking to stay.",
                s.name
            ));
            h.caller = Some(Caller::Stranger(s));
        } else if h.day > 1 {
            let visit_chance = if h.is_built(Building::Inn) { 0.35 } else { 0.2 } * often;
            if h.chance(visit_chance) {
                let mut rng = h.rng();
                let v = *rng.pick(&Visitor::ALL);
                h.news.push(format!("At the gate: {}.", v.who()));
                h.caller = Some(Caller::Visitor(v));
            }
        }
    }

    // A small trouble.
    if h.troubles.len() < 2 && h.day > 3 && h.chance(0.12 * often) {
        let mut options = vec![Trouble::Crows];
        if h.goat {
            options.push(Trouble::Fence);
        }
        if h.weather.wet() {
            for b in h.built.iter().filter(|b| b.recipe().is_some()) {
                options.push(Trouble::Leak(*b));
            }
        }
        options.retain(|t| !h.troubles.contains(t));
        if !options.is_empty() {
            let mut rng = h.rng();
            let t = rng.pick(&options).clone();
            h.news.push(format!("Overnight, {}.", t.name()));
            h.troubles.push(t);
        }
    }

    // Dangers, past Gentle, and only in your own hamlet.
    let dangers = h.kind == crate::rules::Kind::Home && h.pace.danger() > 0.0;
    if dangers {
        // The river rises after rain in spring and autumn.
        let far_row_sown = h.plots[..crate::rules::COLS]
            .iter()
            .any(|p| p.crop().is_some());
        if h.weather.wet()
            && matches!(season, Season::Spring | Season::Autumn)
            && far_row_sown
            && !h.troubles.contains(&Trouble::Flood)
            && h.chance(h.pace.danger() * 0.8)
        {
            h.news.push(
                "The river is rising after the rain. By supper it will reach the far row of the fields."
                    .to_string(),
            );
            h.troubles.push(Trouble::Flood);
        }
        // A hard frost that lasts: supper takes more food.
        if season == Season::Winter && h.cold_snap == 0 && h.chance(h.pace.danger() * 0.6) {
            h.cold_snap = 3;
            h.news.push(
                "A hard frost sets in. For 3 days, everyone eats more at supper.".to_string(),
            );
        } else if h.cold_snap > 0 {
            h.news
                .push("Still bitterly cold. Supper takes more food.".to_string());
        }
        // Something during the day: a fire, a fox, or someone lost at dusk.
        if h.chance(h.pace.danger()) {
            let mut rng = h.rng();
            h.alarm_at = Some(3.0 + rng.below(6) as f32);
        }
    }

    // Someone kept glum too long leaves: two warnings, then they go.
    if let (Some(after), true) = (h.pace.leave_after(), h.kind == crate::rules::Kind::Home) {
        let mut leaving = Vec::new();
        for s in h.settlers.iter_mut() {
            let days = s.low_days;
            if days >= after {
                leaving.push(s.name.clone());
            } else if days + 2 == after && s.warned == 0 {
                s.warned = 1;
                h.news
                    .push(format!("{} is thinking of leaving Larkfield.", s.name));
            } else if days + 1 == after && s.warned < 2 {
                s.warned = 2;
                h.news.push(format!(
                    "{} says they will leave tomorrow unless things get better.",
                    s.name
                ));
            } else if days + 2 < after {
                s.warned = 0;
            }
        }
        for name in leaving {
            h.settlers.retain(|s| s.name != name);
            h.troubles.retain(|t| t != &Trouble::Lost(name.clone()));
            h.news.push(format!(
                "{name} packed up in the night and left for the next village."
            ));
        }
    }

    // A cold in the cool seasons.
    if matches!(season, Season::Autumn | Season::Winter)
        && h.settlers.len() > 1
        && h.chance(0.05 * often)
    {
        let mut rng = h.rng();
        let i = rng.below(h.settlers.len() as u32) as usize;
        let s = &mut h.settlers[i];
        if s.ill_days == 0 && !s.away() {
            s.ill_days = 2;
            h.news
                .push(format!("{} has a cold, and stays in bed.", s.name));
        }
    }

    // How the day feels to each person.
    let wet = h.weather.wet();
    let low_larder = h.food_days() < 2;
    let sweets = h.has("bread") + h.has("pie") > 0;
    let wobble_after = h.pace.wobble_after();
    let mut lines = Vec::new();
    let mut rng = h.rng();
    for s in h.settlers.iter_mut() {
        if wet && s.has(Trait::HatesMud) {
            s.think("mud everywhere", -0.15, 1);
        }
        if low_larder && s.has(Trait::Worrier) {
            s.think("worried about the larder", -0.2, 1);
        }
        if sweets && s.has(Trait::SweetTooth) {
            s.think("something sweet in the larder", 0.1, 1);
        }
        if h.pace == Pace::Gentle {
            s.think("an easy life", 0.1, 1);
        }
        if s.wobble.is_none()
            && s.ill_days == 0
            && s.mood() == Mood::Glum
            && s.low_days >= wobble_after
        {
            let w = *rng.pick(&[
                Wobble::Bridge,
                Wobble::Scones,
                Wobble::SameSong,
                Wobble::WalkedOff,
            ]);
            s.wobble = Some(w);
            lines.push(match w {
                Wobble::WalkedOff => format!(
                    "{} has gone to the next village for the night, to be cheered up there.",
                    s.name
                ),
                _ => format!("{} is {}, {}.", s.name, w.doing(), w.place().where_at()),
            });
        }
    }
    h.news.extend(lines);
}
