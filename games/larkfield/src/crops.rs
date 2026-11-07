//! What grows in Larkfield's fields, by season.

use serde::{Deserialize, Serialize};

/// A season of the hamlet's year. Each lasts [`crate::rules::SEASON_DAYS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub const ALL: [Season; 4] = [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Season::Spring => "spring",
            Season::Summer => "summer",
            Season::Autumn => "autumn",
            Season::Winter => "winter",
        }
    }

    pub fn index(self) -> usize {
        Season::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub fn next(self) -> Season {
        Season::ALL[(self.index() + 1) % 4]
    }

    /// The festival on the season's last day.
    pub fn festival(self) -> &'static str {
        match self {
            Season::Spring => "Sowing day",
            Season::Summer => "Midsummer",
            Season::Autumn => "The harvest fair",
            Season::Winter => "The lantern walk",
        }
    }
}

/// One kind of crop.
#[derive(Debug)]
pub struct Crop {
    pub id: &'static str,
    pub name: &'static str,
    pub plural: &'static str,
    pub season: Season,
    /// Watered days from sowing to ripe.
    pub days: u32,
    /// Days to ripen again after a harvest, for crops that keep cropping.
    pub again: Option<u32>,
    /// What one harvest puts in the larder: the good's id and how many.
    pub yields: (&'static str, u32),
    /// Coins a seed packet costs at the barge.
    pub price: u32,
}

pub static CROPS: [Crop; 15] = [
    Crop {
        id: "turnip",
        name: "turnip",
        plural: "turnips",
        season: Season::Spring,
        days: 3,
        again: None,
        yields: ("turnip", 4),
        price: 2,
    },
    Crop {
        id: "radish",
        name: "radish",
        plural: "radishes",
        season: Season::Spring,
        days: 2,
        again: None,
        yields: ("radish", 2),
        price: 1,
    },
    Crop {
        id: "pea",
        name: "pea",
        plural: "peas",
        season: Season::Spring,
        days: 4,
        again: Some(2),
        yields: ("pea", 2),
        price: 3,
    },
    Crop {
        id: "potato",
        name: "potato",
        plural: "potatoes",
        season: Season::Spring,
        days: 5,
        again: None,
        yields: ("potato", 6),
        price: 3,
    },
    Crop {
        id: "strawberry",
        name: "strawberry",
        plural: "strawberries",
        season: Season::Spring,
        days: 6,
        again: Some(3),
        yields: ("strawberry", 2),
        price: 5,
    },
    Crop {
        id: "wheat",
        name: "wheat",
        plural: "wheat",
        season: Season::Summer,
        days: 4,
        again: None,
        yields: ("grain", 4),
        price: 2,
    },
    Crop {
        id: "bean",
        name: "bean",
        plural: "beans",
        season: Season::Summer,
        days: 4,
        again: Some(2),
        yields: ("bean", 2),
        price: 3,
    },
    Crop {
        id: "tomato",
        name: "tomato",
        plural: "tomatoes",
        season: Season::Summer,
        days: 5,
        again: Some(2),
        yields: ("tomato", 2),
        price: 4,
    },
    Crop {
        id: "corn",
        name: "sweetcorn",
        plural: "sweetcorn",
        season: Season::Summer,
        days: 6,
        again: None,
        yields: ("corn", 4),
        price: 4,
    },
    Crop {
        id: "sunflower",
        name: "sunflower",
        plural: "sunflowers",
        season: Season::Summer,
        days: 5,
        again: None,
        yields: ("seeds", 4),
        price: 3,
    },
    Crop {
        id: "pumpkin",
        name: "pumpkin",
        plural: "pumpkins",
        season: Season::Autumn,
        days: 7,
        again: None,
        yields: ("pumpkin", 6),
        price: 5,
    },
    Crop {
        id: "cabbage",
        name: "cabbage",
        plural: "cabbages",
        season: Season::Autumn,
        days: 5,
        again: None,
        yields: ("cabbage", 4),
        price: 3,
    },
    Crop {
        id: "carrot",
        name: "carrot",
        plural: "carrots",
        season: Season::Autumn,
        days: 3,
        again: None,
        yields: ("carrot", 4),
        price: 2,
    },
    Crop {
        id: "barley",
        name: "barley",
        plural: "barley",
        season: Season::Autumn,
        days: 4,
        again: None,
        yields: ("grain", 4),
        price: 2,
    },
    Crop {
        id: "kale",
        name: "winter kale",
        plural: "winter kale",
        season: Season::Winter,
        days: 4,
        again: Some(3),
        yields: ("kale", 2),
        price: 3,
    },
];

pub fn crop(id: &str) -> Option<&'static Crop> {
    CROPS.iter().find(|c| c.id == id)
}

pub fn of_season(season: Season) -> impl Iterator<Item = &'static Crop> {
    CROPS.iter().filter(move |c| c.season == season)
}
