//! The four trails: how long each is, what lies at its end, what is
//! underfoot, and how many birds open it.

use parlor_core::Season;

use crate::birds::Habitat;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trail {
    Woodland,
    Meadow,
    Marsh,
    Riverside,
}

/// What a step lands on. Only flavour: no step is ever too loud.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ground {
    Earth,
    Leaves,
    Grass,
    Boardwalk,
    Pebbles,
    Snow,
}

impl Ground {
    pub const ALL: [Ground; 6] = [
        Ground::Earth,
        Ground::Leaves,
        Ground::Grass,
        Ground::Boardwalk,
        Ground::Pebbles,
        Ground::Snow,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Ground::Earth => "earth",
            Ground::Leaves => "leaves",
            Ground::Grass => "grass",
            Ground::Boardwalk => "boardwalk",
            Ground::Pebbles => "pebbles",
            Ground::Snow => "snow",
        }
    }

    /// The step in words, for the sound check and the first step.
    pub fn words(self) -> &'static str {
        match self {
            Ground::Earth => "a soft step on the earth path",
            Ground::Leaves => "a crunch of dry leaves",
            Ground::Grass => "a swish through long grass",
            Ground::Boardwalk => "a hollow knock on the boardwalk",
            Ground::Pebbles => "a crunch of river pebbles",
            Ground::Snow => "a soft squeak in fresh snow",
        }
    }
}

impl Trail {
    pub const ALL: [Trail; 4] = [
        Trail::Woodland,
        Trail::Meadow,
        Trail::Marsh,
        Trail::Riverside,
    ];

    /// "woodland trail".
    pub fn name(self) -> &'static str {
        match self {
            Trail::Woodland => "woodland trail",
            Trail::Meadow => "meadow trail",
            Trail::Marsh => "marsh trail",
            Trail::Riverside => "riverside trail",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Trail::Woodland => "woodland",
            Trail::Meadow => "meadow",
            Trail::Marsh => "marsh",
            Trail::Riverside => "riverside",
        }
    }

    pub fn habitat(self) -> Habitat {
        match self {
            Trail::Woodland => Habitat::Woodland,
            Trail::Meadow => Habitat::Meadow,
            Trail::Marsh => Habitat::Marsh,
            Trail::Riverside => Habitat::Riverside,
        }
    }

    /// Kinds of bird in the field guide that open this trail.
    pub fn needs(self) -> u32 {
        match self {
            Trail::Woodland => 0,
            Trail::Meadow => 4,
            Trail::Marsh => 9,
            Trail::Riverside => 15,
        }
    }

    /// From the start to the trail's end, in metres.
    pub fn length(self) -> f32 {
        match self {
            Trail::Woodland => 44.0,
            Trail::Meadow => 48.0,
            Trail::Marsh => 50.0,
            Trail::Riverside => 56.0,
        }
    }

    /// What waits at the trail's end, and makes its sound.
    pub fn end_name(self) -> &'static str {
        match self {
            Trail::Woodland => "the brook",
            Trail::Meadow => "the gate with the wind chimes",
            Trail::Marsh => "the bird hide",
            Trail::Riverside => "the weir",
        }
    }

    /// The sound of the trail's end, in words.
    pub fn end_words(self) -> &'static str {
        match self {
            Trail::Woodland => "a brook babbling over stones",
            Trail::Meadow => "wooden wind chimes on a gate",
            Trail::Marsh => "frogs croaking by the bird hide",
            Trail::Riverside => "water spilling over the weir",
        }
    }

    /// What stops a step off the edge of the trail.
    pub fn edge(self) -> &'static str {
        match self {
            Trail::Woodland => "Thick brambles.",
            Trail::Meadow => "A hawthorn hedge.",
            Trail::Marsh => "Deep reeds and water.",
            Trail::Riverside => "The river's edge.",
        }
    }

    /// What is underfoot, on the path or off it.
    pub fn ground(self, on_path: bool, season: Season) -> Ground {
        if season == Season::Winter {
            return Ground::Snow;
        }
        match (self, on_path) {
            (Trail::Woodland, true) if season == Season::Autumn => Ground::Leaves,
            (Trail::Woodland, true) | (Trail::Meadow, true) => Ground::Earth,
            (Trail::Woodland, false) => Ground::Leaves,
            (Trail::Marsh, true) => Ground::Boardwalk,
            (Trail::Riverside, true) => Ground::Pebbles,
            _ => Ground::Grass,
        }
    }

    /// A line for the start of a walk, by season.
    pub fn opening(self, season: Season) -> &'static str {
        use Season::*;
        match (self, season) {
            (Trail::Woodland, Spring) => {
                "Dawn in the spring woods. New leaves, and the migrants are back."
            }
            (Trail::Woodland, Summer) => "A warm summer dawn under the trees.",
            (Trail::Woodland, Autumn) => {
                "An autumn dawn. Leaves drift down, and the summer birds have gone south."
            }
            (Trail::Woodland, Winter) => {
                "A still winter dawn. Snow on the branches, and the woods are quiet."
            }
            (Trail::Meadow, Spring) => "Dew on the spring meadow, and larks already up.",
            (Trail::Meadow, Summer) => "A summer meadow, tall with grass and flowers.",
            (Trail::Meadow, Autumn) => "A misty autumn meadow, the grass gone to seed.",
            (Trail::Meadow, Winter) => "A white winter field, hushed under snow.",
            (Trail::Marsh, Spring) => "The spring marsh, the reeds loud with new song.",
            (Trail::Marsh, Summer) => "A summer marsh, warm and busy among the reeds.",
            (Trail::Marsh, Autumn) => "An autumn marsh, the reeds turning gold.",
            (Trail::Marsh, Winter) => "A frozen winter marsh, with only a few birds about.",
            (Trail::Riverside, Spring) => "The river in spring, high and quick.",
            (Trail::Riverside, Summer) => "A slow summer river under the willows.",
            (Trail::Riverside, Autumn) => "An autumn river, leaves riding the current.",
            (Trail::Riverside, Winter) => "A cold winter river, steaming in the dawn.",
        }
    }
}
