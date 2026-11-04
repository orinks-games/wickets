//! The shelf: every tea, cup and treat the tea house can hold, and what
//! each one wants from the kettle.

/// What a guest feels like, which some teas answer and others do not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Taste {
    Calm,
    Bright,
    Strong,
    Smoky,
    Floral,
    Warming,
    Sweet,
    Fresh,
}

impl Taste {
    pub const ALL: [Taste; 8] = [
        Taste::Calm,
        Taste::Bright,
        Taste::Strong,
        Taste::Smoky,
        Taste::Floral,
        Taste::Warming,
        Taste::Sweet,
        Taste::Fresh,
    ];

    /// "calm", for "Mister Hale feels like something calm."
    pub fn word(self) -> &'static str {
        match self {
            Taste::Calm => "calm",
            Taste::Bright => "bright",
            Taste::Strong => "strong",
            Taste::Smoky => "smoky",
            Taste::Floral => "floral",
            Taste::Warming => "warming",
            Taste::Sweet => "sweet",
            Taste::Fresh => "fresh",
        }
    }

    /// Ways a regular asks for this taste, said after their name.
    pub fn asks(self) -> &'static [&'static str] {
        match self {
            Taste::Calm => &[
                "A calm cup before bed, please.",
                "Something gentle, for a quiet head.",
                "Something soft and slow, if you have it.",
            ],
            Taste::Bright => &[
                "Something bright to wake me up, please.",
                "Something lively. I have a long afternoon ahead.",
                "Something with a bit of sparkle to it.",
            ],
            Taste::Strong => &[
                "Something strong, please. The kind you could stand a spoon in.",
                "A proper strong cup, if you would.",
                "Something bold. It has been one of those days.",
            ],
            Taste::Smoky => &[
                "Something smoky and strong, please.",
                "Something that tastes like a campfire.",
                "Something dark and smoky, if you have it.",
            ],
            Taste::Floral => &[
                "Something that smells like a garden.",
                "Something flowery, please. I need a bit of spring.",
                "Something with blossom in it, if you have it.",
            ],
            Taste::Warming => &[
                "Something to warm my hands, please.",
                "Something cosy. It is cold out there.",
                "Something toasty, please, like a fireside.",
            ],
            Taste::Sweet => &[
                "Something a little sweet, without sugar.",
                "Something soft and sweet, please.",
                "Something mellow and sweet, if you would.",
            ],
            Taste::Fresh => &[
                "Something fresh and green, please.",
                "Something clean and fresh, to clear my head.",
                "Something light and green, if you would.",
            ],
        }
    }
}

/// How a tea is brewed and served.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Steeped hot and served in a cup.
    Hot,
    /// Steeped short and strong, then poured over ice.
    Iced,
}

/// Where a tea comes from: always on the shelf, bought, a season's blend,
/// or a holiday's special brew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Starter,
    /// Bought in the shop, for this many coins.
    Shop(u32),
    Seasonal,
    Holiday,
}

/// One tea.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tea {
    /// A stable id, saved in records. Never change one that has shipped.
    pub id: &'static str,
    pub name: &'static str,
    /// A short description, said when the shelf lands on it.
    pub about: &'static str,
    /// The water it wants, in degrees.
    pub heat: u32,
    /// How long it wants to steep, in seconds.
    pub steep: u32,
    pub tastes: &'static [Taste],
    pub style: Style,
    pub source: Source,
}

impl Tea {
    pub fn suits(&self, taste: Taste) -> bool {
        self.tastes.contains(&taste)
    }

    /// "75 degrees, just before the boil" or "100 degrees, at the boil".
    pub fn heat_words(&self) -> String {
        let when = match self.heat {
            0..=79 => "well before the boil",
            80..=94 => "just before the boil",
            _ => "at the boil",
        };
        format!("{} degrees, {when}", self.heat)
    }

    /// "Sencha wants water at 75 degrees, well before the boil, and a
    /// 6 second steep."
    pub fn needs(&self) -> String {
        let iced = match self.style {
            Style::Iced => ", then ice",
            Style::Hot => "",
        };
        format!(
            "{} wants water at {}, and a {} second steep{iced}.",
            self.name,
            self.heat_words(),
            self.steep
        )
    }

    pub fn price(&self) -> Option<u32> {
        match self.source {
            Source::Shop(p) => Some(p),
            _ => None,
        }
    }
}

const fn tea(
    id: &'static str,
    name: &'static str,
    about: &'static str,
    heat: u32,
    steep: u32,
    tastes: &'static [Taste],
    source: Source,
) -> Tea {
    Tea {
        id,
        name,
        about,
        heat,
        steep,
        tastes,
        style: Style::Hot,
        source,
    }
}

use Source::{Holiday, Seasonal, Shop, Starter};
use Taste::*;

/// Every tea on the shelf all year: the four you start with, then the
/// shop's, cheapest first.
pub const TEAS: [Tea; 15] = [
    tea(
        "breakfast",
        "Breakfast black",
        "A sturdy black tea, malty and bold.",
        100,
        10,
        &[Strong, Warming],
        Starter,
    ),
    tea(
        "sencha",
        "Sencha",
        "A Japanese green tea, grassy and bright.",
        75,
        6,
        &[Fresh, Bright],
        Starter,
    ),
    tea(
        "chamomile",
        "Chamomile",
        "Golden flowers that taste softly of apple.",
        100,
        14,
        &[Calm, Sweet],
        Starter,
    ),
    tea(
        "peppermint",
        "Peppermint",
        "Cool green leaves that clear the head.",
        100,
        12,
        &[Fresh, Bright],
        Starter,
    ),
    tea(
        "lemon_balm",
        "Lemon balm",
        "A soft lemony herb for a quiet evening.",
        95,
        13,
        &[Calm, Fresh],
        Shop(10),
    ),
    tea(
        "jasmine",
        "Jasmine pearls",
        "Green tea rolled into pearls with jasmine blossoms.",
        80,
        7,
        &[Floral, Calm],
        Shop(12),
    ),
    tea(
        "rooibos",
        "Rooibos",
        "Red bush from the cape, nutty and sweet, with no caffeine.",
        100,
        15,
        &[Calm, Sweet, Warming],
        Shop(12),
    ),
    tea(
        "genmaicha",
        "Genmaicha",
        "Green tea with toasted rice, like warm popcorn.",
        80,
        7,
        &[Warming, Fresh],
        Shop(12),
    ),
    tea(
        "assam",
        "Assam gold",
        "A deep black tea from the river valleys, rich as toffee.",
        100,
        11,
        &[Strong, Sweet],
        Shop(12),
    ),
    tea(
        "white_peony",
        "White peony",
        "Pale silver leaves, delicate and sweet as honeydew.",
        75,
        9,
        &[Sweet, Calm],
        Shop(14),
    ),
    tea(
        "darjeeling",
        "Darjeeling",
        "A light black tea from the hills, with a taste of muscat grapes.",
        95,
        8,
        &[Bright, Floral],
        Shop(14),
    ),
    tea(
        "earl_grey",
        "Earl Grey",
        "Black tea scented with bergamot orange.",
        100,
        9,
        &[Bright, Floral, Strong],
        Shop(14),
    ),
    tea(
        "lapsang",
        "Lapsang souchong",
        "Black tea dried over pine smoke.",
        100,
        10,
        &[Smoky, Strong],
        Shop(16),
    ),
    tea(
        "hojicha",
        "Hojicha",
        "Green tea roasted brown, toasty and mellow.",
        90,
        8,
        &[Warming, Smoky, Calm],
        Shop(16),
    ),
    tea(
        "tieguanyin",
        "Iron goddess oolong",
        "A rolled oolong, creamy, with a scent of orchids.",
        90,
        9,
        &[Floral, Sweet],
        Shop(18),
    ),
];

/// The season's blend, on the shelf for free all season: spring, summer,
/// autumn, winter.
pub const SEASONAL: [Tea; 4] = [
    tea(
        "spring_meadow",
        "Spring meadow",
        "This spring's blend: green tea with elderflower.",
        75,
        7,
        &[Fresh, Floral],
        Seasonal,
    ),
    tea(
        "summer_berry",
        "Summer berry",
        "This summer's blend: hibiscus and blackcurrant, tart and ruby red.",
        100,
        12,
        &[Sweet, Bright],
        Seasonal,
    ),
    tea(
        "autumn_hearth",
        "Autumn hearth",
        "This autumn's blend: black tea with orange peel and clove.",
        100,
        10,
        &[Warming, Strong],
        Seasonal,
    ),
    tea(
        "winter_fireside",
        "Winter fireside",
        "This winter's blend: rooibos with vanilla and cardamom.",
        100,
        14,
        &[Warming, Sweet, Calm],
        Seasonal,
    ),
];

/// The holiday brews, each on the shelf only on its day.
pub const HOLIDAY_TEAS: [Tea; 7] = [
    tea(
        "tangerine_oolong",
        "Tangerine oolong",
        "A guest's gift: oolong steeped with tangerine peel, for a lucky year.",
        90,
        9,
        &[Sweet, Floral, Bright],
        Holiday,
    ),
    tea(
        "rose",
        "Rose tea for two",
        "Rose petals and black tea, one pot poured into two cups.",
        95,
        9,
        &[Floral, Sweet],
        Holiday,
    ),
    tea(
        "cherry_blossom",
        "Cherry blossom",
        "Green tea with salted cherry blossoms that open in the cup.",
        75,
        6,
        &[Floral, Fresh],
        Holiday,
    ),
    Tea {
        style: Style::Iced,
        ..tea(
            "iced_mint",
            "Iced mint",
            "Fresh mint, steeped short and strong, then poured over ice.",
            100,
            5,
            &[Fresh, Bright],
            Holiday,
        )
    },
    tea(
        "barley",
        "Roasted barley",
        "Toasty barley, like bread from the oven.",
        100,
        12,
        &[Warming, Calm],
        Holiday,
    ),
    tea(
        "moonless",
        "Moonless lapsang",
        "A deep smoky lapsang, dark as a night with no moon.",
        100,
        11,
        &[Smoky, Strong],
        Holiday,
    ),
    tea(
        "spiced_apple",
        "Spiced apple",
        "Dried apple, cinnamon and a little nutmeg.",
        100,
        13,
        &[Warming, Sweet],
        Holiday,
    ),
];

/// Every tea there is, for "teas discovered".
pub fn all_teas() -> impl Iterator<Item = &'static Tea> {
    TEAS.iter()
        .chain(SEASONAL.iter())
        .chain(HOLIDAY_TEAS.iter())
}

/// How many teas there are to discover.
pub fn tea_count() -> usize {
    TEAS.len() + SEASONAL.len() + HOLIDAY_TEAS.len()
}

pub fn tea_by_id(id: &str) -> Option<&'static Tea> {
    all_teas().find(|t| t.id == id)
}

/// A cup from the cupboard. Guests whose taste it suits enjoy it a little
/// more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cup {
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    pub suits: &'static [Taste],
    /// Coins in the shop; 0 for the cup you start with.
    pub price: u32,
}

impl Cup {
    pub fn suits(&self, taste: Taste) -> bool {
        self.suits.contains(&taste)
    }
}

pub const CUPS: [Cup; 5] = [
    Cup {
        id: "white",
        name: "Plain white cup",
        about: "Your everyday cup. Good for anything.",
        suits: &[],
        price: 0,
    },
    Cup {
        id: "mug",
        name: "Big mug",
        about: "A heavy mug for strong and warming cups.",
        suits: &[Strong, Warming],
        price: 6,
    },
    Cup {
        id: "clay",
        name: "Clay cup",
        about: "A rough brown cup for calm and smoky teas.",
        suits: &[Calm, Smoky],
        price: 8,
    },
    Cup {
        id: "glass",
        name: "Glass cup",
        about: "A clear glass for fresh and bright teas.",
        suits: &[Fresh, Bright],
        price: 8,
    },
    Cup {
        id: "porcelain",
        name: "Thin porcelain cup",
        about: "A cup as thin as an eggshell, for floral and sweet teas.",
        suits: &[Floral, Sweet],
        price: 10,
    },
];

pub fn cup_by_id(id: &str) -> Option<&'static Cup> {
    CUPS.iter().find(|c| c.id == id)
}

/// A tune for the music box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tune {
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// The music box itself comes with the first tune.
    pub price: u32,
}

pub const TUNES: [Tune; 3] = [
    Tune {
        id: "rain",
        name: "Rain on the roof",
        about: "The music box itself, with its first tune: a slow waltz.",
        price: 15,
    },
    Tune {
        id: "river",
        name: "The slow river",
        about: "A second tune for the music box, low and rolling.",
        price: 8,
    },
    Tune {
        id: "lanterns",
        name: "Lanterns at dusk",
        about: "A third tune for the music box, bright and twinkling.",
        price: 8,
    },
];

pub fn tune_by_id(id: &str) -> Option<&'static Tune> {
    TUNES.iter().find(|t| t.id == id)
}

/// The sand timer's price.
pub const SAND_TIMER_PRICE: u32 = 10;
