//! The almanac: what each morning brings. Weather turns, people come to
//! the gate, small troubles turn up, and a low mood that lasts becomes a
//! wobble. Nothing here is ever a disaster: a trouble stops one thing
//! until someone fixes it, and nobody is ever lost.
//!
//! The pace in Settings sets how often things happen.

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
            Pace::Gentle => "Few troubles, kind weather, and moods that lift easily.",
            Pace::Steady => "A little of everything.",
            Pace::Lively => "More visitors, more troubles, and stormier weather.",
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

    /// Low days before a mood turns into a wobble.
    fn wobble_after(self) -> u32 {
        match self {
            Pace::Gentle => 3,
            Pace::Steady => 2,
            Pace::Lively => 2,
        }
    }
}

/// A small trouble that stops one thing until someone fixes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Trouble {
    /// Crows in the fields: crops do not grow until they are scared off.
    Crows,
    /// The fence by the goat pen is down.
    Fence,
    /// A building's roof leaks: it makes nothing until it is mended.
    Leak(Building),
}

impl Trouble {
    /// The building it stops, if any.
    pub fn stops(&self) -> Option<Building> {
        match self {
            Trouble::Leak(b) => Some(*b),
            _ => None,
        }
    }

    /// Where it is fixed.
    pub fn place(&self) -> Place {
        match self {
            Trouble::Crows => Place::Fields,
            Trouble::Fence => Place::Cottages,
            Trouble::Leak(b) => Place::of(*b),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Trouble::Crows => "crows in the fields".to_string(),
            Trouble::Fence => "the fence by the goat pen is down".to_string(),
            Trouble::Leak(b) => format!("the roof of {} leaks", b.name()),
        }
    }

    /// The line when it is fixed, by a settler's name or "You".
    pub fn fixed(&self, by: &str) -> String {
        match self {
            Trouble::Crows => format!("{by} scared the crows off the fields."),
            Trouble::Fence => format!("{by} put the fence by the goat pen back up."),
            Trouble::Leak(b) => format!("{by} mended the roof of {}.", b.name()),
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
