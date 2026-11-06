//! What can be built along Larkfield's lane, and what each one makes.

use serde::{Deserialize, Serialize};

/// A batch: what goes in, and what comes out.
pub type Recipe = (&'static [(&'static str, u32)], (&'static str, u32));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Building {
    Well,
    Mill,
    Bakery,
    Smokehouse,
    Workshop,
    Dairy,
    Inn,
    Bandstand,
}

impl Building {
    pub const ALL: [Building; 8] = [
        Building::Well,
        Building::Mill,
        Building::Bakery,
        Building::Smokehouse,
        Building::Workshop,
        Building::Dairy,
        Building::Inn,
        Building::Bandstand,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Building::Well => "well",
            Building::Mill => "mill",
            Building::Bakery => "bakery",
            Building::Smokehouse => "smokehouse",
            Building::Workshop => "workshop",
            Building::Dairy => "dairy",
            Building::Inn => "inn",
            Building::Bandstand => "bandstand",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Building::Well => "the well",
            Building::Mill => "the mill",
            Building::Bakery => "the bakery",
            Building::Smokehouse => "the smokehouse",
            Building::Workshop => "the workshop",
            Building::Dairy => "the dairy",
            Building::Inn => "the inn",
            Building::Bandstand => "the bandstand",
        }
    }

    /// What it does, for the build menu.
    pub fn about(self) -> &'static str {
        match self {
            Building::Well => "Settlers water the fields from it, and a bench by it lifts moods.",
            Building::Mill => "Grinds grain into flour.",
            Building::Bakery => "Bakes flour into bread, and bread and fruit into pies.",
            Building::Smokehouse => "Smokes fish so it keeps all winter.",
            Building::Workshop => "Makes furniture from wood, which makes cottages comfortable.",
            Building::Dairy => "Turns the goat's milk into cheese.",
            Building::Inn => "Room for visitors, and a sick room. Visitors bring trade and news.",
            Building::Bandstand => "Music on the green after supper, which everyone enjoys.",
        }
    }

    /// Wood, stone and coins it takes.
    pub fn cost(self) -> (u32, u32, u32) {
        match self {
            Building::Well => (4, 6, 0),
            Building::Mill => (10, 8, 10),
            Building::Bakery => (8, 10, 10),
            Building::Smokehouse => (8, 4, 5),
            Building::Workshop => (12, 4, 5),
            Building::Dairy => (8, 6, 15),
            Building::Inn => (16, 10, 25),
            Building::Bandstand => (10, 2, 10),
        }
    }

    /// Building work it takes, in settler hours.
    pub fn work(self) -> f32 {
        match self {
            Building::Well => 4.0,
            Building::Mill | Building::Bakery | Building::Workshop => 8.0,
            Building::Smokehouse | Building::Dairy | Building::Bandstand => 6.0,
            Building::Inn => 12.0,
        }
    }

    /// Another building it needs first.
    pub fn needs(self) -> Option<Building> {
        match self {
            Building::Bakery => Some(Building::Mill),
            _ => None,
        }
    }

    /// What it turns into what, one batch at a time: inputs, output.
    pub fn recipe(self) -> Option<Recipe> {
        match self {
            Building::Mill => Some((&[("grain", 2)], ("flour", 2))),
            Building::Bakery => Some((&[("flour", 2)], ("bread", 3))),
            Building::Smokehouse => Some((&[("fish", 2)], ("smoked_fish", 2))),
            Building::Workshop => Some((&[("wood", 3)], ("furniture", 1))),
            Building::Dairy => Some((&[("milk", 2)], ("cheese", 1))),
            _ => None,
        }
    }
}

/// A good in the larder or the store: its name and what it is worth.
pub struct Good {
    pub id: &'static str,
    pub name: &'static str,
    /// Days of food for one person, in tenths.
    pub food: u32,
    /// Coins the barge pays for one.
    pub sells: u32,
}

pub static GOODS: [Good; 26] = [
    Good {
        id: "turnip",
        name: "turnips",
        food: 3,
        sells: 1,
    },
    Good {
        id: "radish",
        name: "radishes",
        food: 2,
        sells: 1,
    },
    Good {
        id: "pea",
        name: "peas",
        food: 3,
        sells: 2,
    },
    Good {
        id: "potato",
        name: "potatoes",
        food: 4,
        sells: 1,
    },
    Good {
        id: "strawberry",
        name: "strawberries",
        food: 3,
        sells: 3,
    },
    Good {
        id: "grain",
        name: "grain",
        food: 0,
        sells: 1,
    },
    Good {
        id: "bean",
        name: "beans",
        food: 3,
        sells: 2,
    },
    Good {
        id: "tomato",
        name: "tomatoes",
        food: 3,
        sells: 2,
    },
    Good {
        id: "corn",
        name: "sweetcorn",
        food: 4,
        sells: 2,
    },
    Good {
        id: "seeds",
        name: "sunflower seeds",
        food: 2,
        sells: 2,
    },
    Good {
        id: "pumpkin",
        name: "pumpkins",
        food: 5,
        sells: 3,
    },
    Good {
        id: "cabbage",
        name: "cabbages",
        food: 4,
        sells: 2,
    },
    Good {
        id: "carrot",
        name: "carrots",
        food: 3,
        sells: 1,
    },
    Good {
        id: "kale",
        name: "winter kale",
        food: 3,
        sells: 2,
    },
    Good {
        id: "fish",
        name: "fish",
        food: 4,
        sells: 2,
    },
    Good {
        id: "smoked_fish",
        name: "smoked fish",
        food: 5,
        sells: 4,
    },
    Good {
        id: "eggs",
        name: "eggs",
        food: 2,
        sells: 1,
    },
    Good {
        id: "milk",
        name: "milk",
        food: 2,
        sells: 1,
    },
    Good {
        id: "cheese",
        name: "cheese",
        food: 6,
        sells: 5,
    },
    Good {
        id: "flour",
        name: "flour",
        food: 0,
        sells: 2,
    },
    Good {
        id: "bread",
        name: "bread",
        food: 6,
        sells: 3,
    },
    Good {
        id: "pie",
        name: "pies",
        food: 8,
        sells: 6,
    },
    Good {
        id: "berries",
        name: "berries",
        food: 2,
        sells: 1,
    },
    Good {
        id: "mushrooms",
        name: "mushrooms",
        food: 2,
        sells: 2,
    },
    Good {
        id: "wood",
        name: "wood",
        food: 0,
        sells: 1,
    },
    Good {
        id: "stone",
        name: "stone",
        food: 0,
        sells: 1,
    },
];

/// Goods that are not food and not materials, kept in the store.
pub const NAILS: &str = "nails";
pub const FURNITURE: &str = "furniture";

pub fn good(id: &str) -> Option<&'static Good> {
    GOODS.iter().find(|g| g.id == id)
}

/// A number of a good, as said: "1 turnip", "3 turnips", "2 loaves of bread".
pub fn count(id: &str, n: u32) -> String {
    let one = match id {
        "turnip" => "turnip",
        "radish" => "radish",
        "pea" => "pea",
        "potato" => "potato",
        "strawberry" => "strawberry",
        "bean" => "bean",
        "tomato" => "tomato",
        "pumpkin" => "pumpkin",
        "cabbage" => "cabbage",
        "carrot" => "carrot",
        "eggs" => "egg",
        "pie" => "pie",
        "berries" => "berry",
        "mushrooms" => "mushroom",
        NAILS => "nail",
        "bread" => {
            return if n == 1 {
                "1 loaf of bread".into()
            } else {
                format!("{n} loaves of bread")
            }
        }
        "milk" => {
            return if n == 1 {
                "1 jug of milk".into()
            } else {
                format!("{n} jugs of milk")
            }
        }
        "flour" => {
            return if n == 1 {
                "1 sack of flour".into()
            } else {
                format!("{n} sacks of flour")
            }
        }
        "cheese" => {
            return if n == 1 {
                "1 cheese".into()
            } else {
                format!("{n} cheeses")
            }
        }
        FURNITURE => {
            return if n == 1 {
                "1 piece of furniture".into()
            } else {
                format!("{n} pieces of furniture")
            }
        }
        _ => return format!("{n} {}", good_name(id)),
    };
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {}", good_name(id))
    }
}

/// A good's name for speech.
pub fn good_name(id: &str) -> &'static str {
    match id {
        NAILS => "nails",
        FURNITURE => "furniture",
        _ => good(id).map_or("goods", |g| g.name),
    }
}
