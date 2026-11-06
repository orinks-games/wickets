//! Who comes to live in Larkfield: names, past trades, traits, wants and
//! each settler's own three-step story. Settlers are made from these parts,
//! so every hamlet has different people.

use serde::{Deserialize, Serialize};

/// The eight kinds of work, each a skill from 0 to 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Skill {
    Farming,
    Cooking,
    Building,
    Crafting,
    Animals,
    Fishing,
    Mending,
    Music,
}

impl Skill {
    pub const ALL: [Skill; 8] = [
        Skill::Farming,
        Skill::Cooking,
        Skill::Building,
        Skill::Crafting,
        Skill::Animals,
        Skill::Fishing,
        Skill::Mending,
        Skill::Music,
    ];

    pub fn index(self) -> usize {
        Skill::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub fn name(self) -> &'static str {
        match self {
            Skill::Farming => "farming",
            Skill::Cooking => "cooking",
            Skill::Building => "building",
            Skill::Crafting => "crafting",
            Skill::Animals => "animals",
            Skill::Fishing => "fishing",
            Skill::Mending => "mending",
            Skill::Music => "music",
        }
    }
}

/// A trait: a small part of who someone is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Trait {
    EarlyRiser,
    NightOwl,
    SweetTooth,
    GreenFingers,
    HatesMud,
    Chatterbox,
    Homesick,
    Tidy,
    Clumsy,
    Brave,
    Worrier,
    Musician,
    Quiet,
    Hungry,
    Cheerful,
    Grumbler,
}

impl Trait {
    pub const ALL: [Trait; 16] = [
        Trait::EarlyRiser,
        Trait::NightOwl,
        Trait::SweetTooth,
        Trait::GreenFingers,
        Trait::HatesMud,
        Trait::Chatterbox,
        Trait::Homesick,
        Trait::Tidy,
        Trait::Clumsy,
        Trait::Brave,
        Trait::Worrier,
        Trait::Musician,
        Trait::Quiet,
        Trait::Hungry,
        Trait::Cheerful,
        Trait::Grumbler,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Trait::EarlyRiser => "early riser",
            Trait::NightOwl => "night owl",
            Trait::SweetTooth => "sweet tooth",
            Trait::GreenFingers => "green fingers",
            Trait::HatesMud => "hates mud",
            Trait::Chatterbox => "chatterbox",
            Trait::Homesick => "homesick",
            Trait::Tidy => "tidy",
            Trait::Clumsy => "clumsy",
            Trait::Brave => "brave",
            Trait::Worrier => "worrier",
            Trait::Musician => "musician",
            Trait::Quiet => "quiet",
            Trait::Hungry => "big appetite",
            Trait::Cheerful => "sunny",
            Trait::Grumbler => "grumbler",
        }
    }

    /// What the trait means in play, for the settler's page.
    pub fn about(self) -> &'static str {
        match self {
            Trait::EarlyRiser => "works best in the morning",
            Trait::NightOwl => "works best in the evening",
            Trait::SweetTooth => "cheered by bread and pies",
            Trait::GreenFingers => "grows crops faster",
            Trait::HatesMud => "glum on rainy days",
            Trait::Chatterbox => "needs company more",
            Trait::Homesick => "misses home, and loves a letter",
            Trait::Tidy => "needs comfort more",
            Trait::Clumsy => "works a little slower",
            Trait::Brave => "first to fix a trouble",
            Trait::Worrier => "glum when the larder is low",
            Trait::Musician => "plays at supper",
            Trait::Quiet => "needs company less",
            Trait::Hungry => "eats more",
            Trait::Cheerful => "slow to get glum",
            Trait::Grumbler => "quick to get glum",
        }
    }

    /// Pairs that cannot be the same person.
    pub fn clashes(self, other: Trait) -> bool {
        use Trait::*;
        matches!(
            (self, other),
            (EarlyRiser, NightOwl)
                | (NightOwl, EarlyRiser)
                | (Chatterbox, Quiet)
                | (Quiet, Chatterbox)
                | (Cheerful, Grumbler)
                | (Grumbler, Cheerful)
                | (Brave, Worrier)
                | (Worrier, Brave)
        ) || self == other
    }
}

/// A past trade: where a settler's best skill and story come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Trade {
    Baker,
    Sailor,
    Schoolteacher,
    Clockmaker,
    Shepherd,
    Carpenter,
    Fiddler,
    Nurse,
}

impl Trade {
    pub const ALL: [Trade; 8] = [
        Trade::Baker,
        Trade::Sailor,
        Trade::Schoolteacher,
        Trade::Clockmaker,
        Trade::Shepherd,
        Trade::Carpenter,
        Trade::Fiddler,
        Trade::Nurse,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Trade::Baker => "baker",
            Trade::Sailor => "sailor",
            Trade::Schoolteacher => "schoolteacher",
            Trade::Clockmaker => "clockmaker",
            Trade::Shepherd => "shepherd",
            Trade::Carpenter => "carpenter",
            Trade::Fiddler => "fiddler",
            Trade::Nurse => "nurse",
        }
    }

    /// The skills the trade brings, best first.
    pub fn skills(self) -> [Skill; 2] {
        match self {
            Trade::Baker => [Skill::Cooking, Skill::Farming],
            Trade::Sailor => [Skill::Fishing, Skill::Building],
            Trade::Schoolteacher => [Skill::Mending, Skill::Music],
            Trade::Clockmaker => [Skill::Crafting, Skill::Building],
            Trade::Shepherd => [Skill::Animals, Skill::Farming],
            Trade::Carpenter => [Skill::Building, Skill::Crafting],
            Trade::Fiddler => [Skill::Music, Skill::Cooking],
            Trade::Nurse => [Skill::Mending, Skill::Cooking],
        }
    }

    /// The trade's own story.
    pub fn story(self) -> &'static Story {
        &STORIES[Trade::ALL.iter().position(|&t| t == self).unwrap_or(0)]
    }
}

/// What a step of a story waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Needs {
    /// A building standing in the hamlet.
    Building(crate::buildings::Building),
    /// Goods in the larder, by id, and how many.
    Goods(&'static str, u32),
    /// A number of settlers in the hamlet.
    Settlers(u32),
    /// A festival held since the story began.
    Festival,
    /// The settler cheerful at the end of a day.
    Cheerful,
}

/// A settler's own three-step story.
#[derive(Debug)]
pub struct Story {
    /// The want that starts it, in the settler's words.
    pub hook: &'static str,
    /// Each step: what it needs, and the line when it is done.
    pub steps: [(Needs, &'static str); 3],
    /// The keepsake it leaves on the shelf.
    pub keepsake: &'static str,
}

use crate::buildings::Building as B;

pub static STORIES: [Story; 8] = [
    Story {
        hook: "I'd give anything for a proper oven again.",
        steps: [
            (Needs::Building(B::Mill), "{name} has flour from the mill, and says the oven comes next."),
            (Needs::Building(B::Bakery), "{name} lit the bakery oven for the first time. The whole lane smells of bread."),
            (Needs::Festival, "{name} baked a loaf as big as a cartwheel for the festival. Everyone had a slice."),
        ],
        keepsake: "the first loaf tin from the bakery",
    },
    Story {
        hook: "I miss the sea. A boat on the river would do.",
        steps: [
            (Needs::Goods("wood", 10), "{name} has the wood for a boat, and is drawing it out on the barn floor."),
            (Needs::Building(B::Workshop), "{name} built a little rowing boat in the workshop."),
            (Needs::Cheerful, "{name} rowed down the river at dawn and came back singing."),
        ],
        keepsake: "a carved oar with a sailor's knot",
    },
    Story {
        hook: "A hamlet ought to have a school, even a small one.",
        steps: [
            (Needs::Settlers(4), "{name} says there are enough of us now for lessons on the green."),
            (Needs::Building(B::Inn), "{name} holds lessons in the inn's back room, and visitors' children join in."),
            (Needs::Festival, "{name}'s pupils sang a song at the festival. Nobody missed a word."),
        ],
        keepsake: "a slate with everyone's names on it",
    },
    Story {
        hook: "The clock over the gate stopped years ago. I could mend it.",
        steps: [
            (Needs::Building(B::Workshop), "{name} set up a bench in the workshop for the clock's parts."),
            (Needs::Goods("nails", 6), "{name} built a ladder up to the gate clock."),
            (Needs::Festival, "{name} wound the gate clock on festival night. It chimed for the first time in years."),
        ],
        keepsake: "an old brass clock key",
    },
    Story {
        hook: "I'd like a flock again. Even two or three.",
        steps: [
            (Needs::Goods("eggs", 6), "{name} says the hens are doing well, and the goat next."),
            (Needs::Building(B::Dairy), "{name} keeps the goat's milk cool in the new dairy."),
            (Needs::Cheerful, "{name} made the hamlet's first cheese and named it after the goat."),
        ],
        keepsake: "a shepherd's crook with a bell on it",
    },
    Story {
        hook: "Every hamlet needs a good bench to sit on.",
        steps: [
            (Needs::Goods("wood", 8), "{name} has picked the oak for the bench."),
            (Needs::Building(B::Workshop), "{name} made a long bench for the green."),
            (Needs::Building(B::Bandstand), "{name} carved everyone's initials into the bandstand rail."),
        ],
        keepsake: "a curl of oak shaving, kept in a matchbox",
    },
    Story {
        hook: "I want to play somewhere people can dance.",
        steps: [
            (Needs::Settlers(3), "{name} says there are enough of us for a proper dance now."),
            (Needs::Building(B::Bandstand), "{name} played the bandstand's first night. Feet were tapping all down the lane."),
            (Needs::Festival, "{name} wrote a tune called Larkfield and played it at the festival."),
        ],
        keepsake: "a fiddle string tied in a bow",
    },
    Story {
        hook: "A place to rest when you're ill, that's what's missing.",
        steps: [
            (Needs::Building(B::Inn), "{name} keeps two beds aired at the inn for anyone poorly."),
            (Needs::Goods("bread", 4), "{name} has a cupboard of bread and broth for the sick room."),
            (Needs::Cheerful, "{name} says nobody in Larkfield has missed a day's work this week."),
        ],
        keepsake: "a jar of dried camomile",
    },
];

/// First names, chosen to read clearly under a screen reader.
pub const NAMES: [&str; 32] = [
    "Wren", "Tomas", "Odile", "Maggie", "Ezra", "Ivy", "Rowan", "Nell", "Arlo", "Bea", "Hugo",
    "Lena", "Otis", "Rosa", "Felix", "Ada", "Jonah", "Clara", "Milo", "Edie", "Silas", "June",
    "Abel", "Greta", "Lewis", "Hazel", "Owen", "Vera", "Caleb", "Mabel", "Rufus", "Iris",
];

/// The small wants a settler has at a time.
pub const WANTS: [(&str, Needs); 6] = [
    ("fresh bread", Needs::Goods("bread", 1)),
    ("a pie at supper", Needs::Goods("pie", 1)),
    ("a bench by the well", Needs::Building(B::Well)),
    ("music on the green", Needs::Building(B::Bandstand)),
    ("smoked fish for winter", Needs::Goods("smoked_fish", 2)),
    ("a bit of cheese", Needs::Goods("cheese", 1)),
];

/// The instrument each settler plays in the hamlet's tune, by arrival order.
pub const INSTRUMENTS: [&str; 6] = ["fiddle", "flute", "accordion", "harp", "cello", "clarinet"];
