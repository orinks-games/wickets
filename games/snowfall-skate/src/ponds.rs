//! Where you skate: three ponds, each opened by a star on the one before,
//! and the courses with an end: practice, the lantern skate and the Frost
//! Fair.

use crate::rules::Level;

/// One of the ponds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pond {
    pub number: u32,
    /// "the village pond".
    pub name: &'static str,
    /// A line on where it is, for the title and the pond select.
    pub scene: &'static str,
    pub level: Level,
    /// The points for one, two and three stars.
    pub marks: [u32; 3],
}

impl Pond {
    /// "Pond 1, the village pond".
    pub fn title(&self) -> String {
        format!("Pond {}, {}", self.number, self.name)
    }

    /// "The village pond".
    pub fn subject(&self) -> String {
        hearken::text::sentence_start(self.name)
    }
}

const fn pond_level(gap_ms: i64, lead_ms: i64, snow: f64, drifts: f64, ice: f64) -> Level {
    Level {
        gap_ms,
        lead_ms,
        snow,
        gain: 2.5,
        drifts,
        ice,
        patch_width: 14.0,
        spread: 2,
        partner: true,
        count: None,
        pairs: false,
        stalls: false,
    }
}

pub const PONDS: [Pond; 3] = [
    Pond {
        number: 1,
        name: "the village pond",
        scene: "The village pond at dusk, the snow just starting. Drifts lie here and there.",
        level: Level {
            spread: 1,
            ..pond_level(3400, 3200, 1.4, 0.35, 0.0)
        },
        marks: [50, 110, 170],
    },
    Pond {
        number: 2,
        name: "the mill pond",
        scene: "The mill pond, by the frozen wheel. Thin ice where the stream runs in.",
        level: pond_level(3200, 3000, 1.5, 0.25, 0.3),
        marks: [50, 100, 160],
    },
    Pond {
        number: 3,
        name: "the long lake",
        scene: "The long lake, out past the willows. Quicker, and the snow comes harder.",
        level: Level {
            gain: 2.4,
            patch_width: 15.0,
            ..pond_level(2900, 2700, 1.7, 0.25, 0.4)
        },
        marks: [40, 80, 120],
    },
];

/// Pond `number`, from 1.
pub fn pond(number: u32) -> &'static Pond {
    &PONDS[(number.clamp(1, PONDS.len() as u32) - 1) as usize]
}

/// Practice: slow, everything named, ten lanterns and no snow settling.
pub const PRACTICE: Level = Level {
    gap_ms: 4000,
    lead_ms: 3600,
    snow: 0.0,
    gain: 0.0,
    drifts: 0.3,
    ice: 0.2,
    patch_width: 13.0,
    spread: 1,
    partner: true,
    count: Some(10),
    pairs: false,
    stalls: false,
};

/// Lanterns on the lantern skate: sixteen pairs, one tune.
pub const LANTERN_PAIRS: u32 = 16;

/// The lantern skate: lanterns in pairs, and no partner to follow.
pub const LANTERN_SKATE: Level = Level {
    gap_ms: 3800,
    lead_ms: 3200,
    snow: 0.0,
    gain: 0.0,
    drifts: 0.0,
    ice: 0.0,
    patch_width: 14.0,
    spread: 2,
    partner: false,
    count: Some(LANTERN_PAIRS),
    pairs: true,
    stalls: false,
};

/// Lanterns down the river at the Frost Fair.
pub const FAIR_LANTERNS: u32 = 36;

/// The Frost Fair: a long course down the frozen river, past the stalls.
pub const FAIR: Level = Level {
    gap_ms: 3200,
    lead_ms: 3000,
    snow: 0.0,
    gain: 0.0,
    drifts: 0.25,
    ice: 0.25,
    patch_width: 14.0,
    spread: 2,
    partner: true,
    count: Some(FAIR_LANTERNS),
    pairs: false,
    stalls: true,
};

/// The Frost Fair's marks: a ribbon for one star.
pub const FAIR_MARKS: [u32; 3] = [60, 110, 150];
