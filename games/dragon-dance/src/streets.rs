//! The streets the parade dances down: the eight of the main parade, the
//! practice street, the festival streets and the challenges' streets.

use crate::rules::{Finale, Street};

const MARKET: &[&str] = &[
    "the noodle stall",
    "the fishmonger",
    "the fruit stand",
    "the dumpling shop",
    "the spice seller",
    "the bakery",
];
const TEA: &[&str] = &[
    "the tea house",
    "the bookshop",
    "the calligrapher",
    "the teapot maker",
    "the music shop",
    "the sweet shop",
];
const SILK: &[&str] = &[
    "the silk weaver",
    "the tailor",
    "the embroidery shop",
    "the fan maker",
    "the shoe shop",
    "the button seller",
];
const BRIDGE: &[&str] = &[
    "the lantern maker",
    "the boat house",
    "the kite seller",
    "the corner shop",
    "the candle shop",
    "the toy stall",
];
const TEMPLE: &[&str] = &[
    "the incense shop",
    "the temple gate",
    "the bell foundry",
    "the flower seller",
    "the fortune teller",
    "the herbalist",
];
const HARBOUR: &[&str] = &[
    "the fish market",
    "the rope maker",
    "the oyster stall",
    "the ferry office",
    "the net mender",
    "the seafood restaurant",
];
const JADE: &[&str] = &[
    "the jade carver",
    "the goldsmith",
    "the porcelain shop",
    "the tea merchant",
    "the antique dealer",
    "the jeweller",
];
const AVENUE: &[&str] = &[
    "the grand hotel",
    "the opera house",
    "the department store",
    "the banquet hall",
    "the cinema",
    "the town hall",
];
const NEW_YEAR_SHOPS: &[&str] = &[
    "the red envelope stall",
    "the paper-cut shop",
    "the orange seller",
    "the dumpling shop",
    "the couplet writer",
    "the sweet shop",
    "the flower market",
    "the lantern maker",
];
const LANTERN_SHOPS: &[&str] = &[
    "the rice ball stall",
    "the riddle board",
    "the lantern maker",
    "the tea house",
    "the moon gate",
    "the river steps",
];

/// The main parade, street 1 first.
pub const STREETS: [Street; 8] = [
    Street {
        name: "Market Row",
        lanes: 3,
        bpm: 84.0,
        bars: 24,
        gong_every: 4,
        move_chance: 0.45,
        envelope_chance: 0.3,
        firecracker_chance: 0.0,
        jumps: false,
        tight: false,
        finale: Finale::Gong,
        shops: MARKET,
    },
    Street {
        name: "Tea House Lane",
        lanes: 3,
        bpm: 88.0,
        bars: 28,
        gong_every: 4,
        move_chance: 0.5,
        envelope_chance: 0.3,
        firecracker_chance: 0.3,
        jumps: false,
        tight: false,
        finale: Finale::Gong,
        shops: TEA,
    },
    Street {
        name: "Silk Street",
        lanes: 4,
        bpm: 92.0,
        bars: 28,
        gong_every: 3,
        move_chance: 0.5,
        envelope_chance: 0.35,
        firecracker_chance: 0.35,
        jumps: false,
        tight: false,
        finale: Finale::Gong,
        shops: SILK,
    },
    Street {
        name: "Lantern Bridge",
        lanes: 4,
        bpm: 96.0,
        bars: 32,
        gong_every: 3,
        move_chance: 0.55,
        envelope_chance: 0.35,
        firecracker_chance: 0.4,
        jumps: true,
        tight: false,
        finale: Finale::Gong,
        shops: BRIDGE,
    },
    Street {
        name: "Temple Road",
        lanes: 4,
        bpm: 100.0,
        bars: 32,
        gong_every: 2,
        move_chance: 0.55,
        envelope_chance: 0.4,
        firecracker_chance: 0.45,
        jumps: true,
        tight: false,
        finale: Finale::Gong,
        shops: TEMPLE,
    },
    Street {
        name: "Harbour Front",
        lanes: 5,
        bpm: 104.0,
        bars: 36,
        gong_every: 3,
        move_chance: 0.6,
        envelope_chance: 0.4,
        firecracker_chance: 0.5,
        jumps: true,
        tight: false,
        finale: Finale::Gong,
        shops: HARBOUR,
    },
    Street {
        name: "Jade Gate",
        lanes: 5,
        bpm: 108.0,
        bars: 36,
        gong_every: 2,
        move_chance: 0.6,
        envelope_chance: 0.45,
        firecracker_chance: 0.55,
        jumps: true,
        tight: true,
        finale: Finale::Gong,
        shops: JADE,
    },
    Street {
        name: "The Grand Avenue",
        lanes: 5,
        bpm: 112.0,
        bars: 40,
        gong_every: 2,
        move_chance: 0.65,
        envelope_chance: 0.45,
        firecracker_chance: 0.6,
        jumps: true,
        tight: true,
        finale: Finale::Gong,
        shops: AVENUE,
    },
];

/// The practice street: slow, short, and nothing to lose.
pub const PRACTICE: Street = Street {
    name: "the practice street",
    lanes: 3,
    bpm: 76.0,
    bars: 16,
    gong_every: 3,
    move_chance: 0.5,
    envelope_chance: 0.4,
    firecracker_chance: 0.35,
    jumps: false,
    tight: false,
    finale: Finale::Gong,
    shops: MARKET,
};

/// The New Year parade: long, lucky, and ending at midnight.
pub const NEW_YEAR: Street = Street {
    name: "the New Year parade",
    lanes: 4,
    bpm: 100.0,
    bars: 48,
    gong_every: 3,
    move_chance: 0.55,
    envelope_chance: 0.6,
    firecracker_chance: 0.4,
    jumps: true,
    tight: false,
    finale: Finale::Midnight,
    shops: NEW_YEAR_SHOPS,
};

/// The Lantern walk: slow, calm, no firecrackers and no gongs to miss.
pub const LANTERN_WALK: Street = Street {
    name: "the Lantern walk",
    lanes: 3,
    bpm: 70.0,
    bars: 32,
    gong_every: 0,
    move_chance: 0.4,
    envelope_chance: 0.5,
    firecracker_chance: 0.0,
    jumps: false,
    tight: false,
    finale: Finale::Calm,
    shops: LANTERN_SHOPS,
};

/// The daily challenge's street.
pub const DAILY: Street = Street {
    name: "today's street",
    lanes: 4,
    bpm: 96.0,
    bars: 24,
    gong_every: 3,
    move_chance: 0.55,
    envelope_chance: 0.4,
    firecracker_chance: 0.4,
    jumps: true,
    tight: false,
    finale: Finale::Gong,
    shops: TEA,
};

/// The weekly challenge's street.
pub const WEEKLY: Street = Street {
    name: "the long parade",
    lanes: 4,
    bpm: 100.0,
    bars: 48,
    gong_every: 2,
    move_chance: 0.55,
    envelope_chance: 0.4,
    firecracker_chance: 0.45,
    jumps: true,
    tight: false,
    finale: Finale::Gong,
    shops: TEMPLE,
};

/// The monthly challenge's street.
pub const MONTHLY: Street = Street {
    name: "the grand festival",
    lanes: 5,
    bpm: 112.0,
    bars: 40,
    gong_every: 2,
    move_chance: 0.65,
    envelope_chance: 0.45,
    firecracker_chance: 0.6,
    jumps: true,
    tight: true,
    finale: Finale::Gong,
    shops: AVENUE,
};

/// Street `n`, counting from 1.
pub fn street(n: usize) -> Street {
    STREETS[n.clamp(1, STREETS.len()) - 1]
}
