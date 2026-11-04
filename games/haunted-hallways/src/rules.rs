//! The rules of a haunt, with no sound or speech: a house of a hundred
//! floors in ten wings, and endless floors below them, each floor a chain
//! of rooms laid out on a grid, one puzzle per room, ghosts that must not
//! see you move (and one hound that must not find you still), hazards that
//! grow with depth, and a haunt clock per floor.
//!
//! Every floor is generated from its number: deeper floors have more and
//! bigger rooms, more decoys, faster and more frequent ghosts, rotten
//! boards, a hunting shadow, mimics, dark and mirrored rooms, and twists.
//!
//! Everything advances through [`Run::walk`], [`Run::interact`],
//! [`Run::cycle`], [`Run::hint`] and [`Run::update`], each of which returns
//! the [`Ev`]ents it caused, so the screen only has to voice them and tests
//! can drive a whole floor deterministically.

use std::cell::Cell;
use std::collections::VecDeque;
use std::f32::consts::TAU;

use parlor_core::{Challenge, Period, Rng};

/// How many floors the house has. Endless night goes on below them.
pub const FLOORS: u32 = 100;
/// Floors in a wing.
pub const WING_FLOORS: u32 = 10;
/// Courage per floor: each catch or fall costs one.
pub const COURAGE: u32 = 3;
/// Haunt seconds lost for touching or offering the wrong thing, at first.
pub const WRONG_PENALTY: f64 = 10.0;
/// Haunt seconds a hint costs, at first.
pub const HINT_COST: f64 = 5.0;
/// Metres between neighbouring cells, for placing sounds.
pub const CELL: f32 = 1.5;
/// Where a ghost is first heard, in metres.
pub const GHOST_START: f32 = 12.0;
/// Inside this distance the ghost is passing you.
pub const GHOST_NEAR: f32 = 3.0;
/// The haunt-clock warnings, in seconds left.
pub const WARNINGS: [f64; 3] = [60.0, 30.0, 10.0];
/// In a dark room, things call out only this many cells away or nearer.
pub const DARK_REACH: f32 = 3.0;
/// While the hound passes, standing still this long lets it find you.
pub const HOUND_STILL: f64 = 1.2;
/// A step this soon after the stab is the one already under the player's
/// finger: it is let through. Freezing is about listening, not reflexes.
pub const STAB_GRACE: f64 = 0.4;
/// The shadow hunts you for this long after your last footstep.
pub const SHADOW_HEARS: f64 = 2.5;
/// The first floor of each kind of danger, for teaching and for help.
pub const OFFERINGS_FROM: u32 = 2;
pub const MISSING_FROM: u32 = 6;
pub const ROTTEN_FROM: u32 = 8;
pub const TWISTS_FROM: u32 = 11;
pub const SEANCES_FROM: u32 = 12;
pub const DARK_FROM: u32 = 15;
pub const SHADOW_FROM: u32 = 20;
pub const HOUND_FROM: u32 = 25;
pub const MIMICS_FROM: u32 = 35;
pub const MIRRORS_FROM: u32 = 45;
pub const CHARMS_FROM: u32 = 3;
pub const PENDULUM_FROM: u32 = 51;
pub const WATER_FROM: u32 = 61;
pub const STONES_FROM: u32 = 71;
pub const FOUNDER_FROM: u32 = 81;
pub const SHIFTS_FROM: u32 = 91;
/// Seconds a candle stub adds to the haunt clock.
pub const STUB_SECONDS: f64 = 20.0;
/// The most salt you can carry.
pub const SALT_MAX: u32 = 3;
/// Seconds a step into deep water holds you.
pub const WADE_TIME: f64 = 0.7;
/// Seconds a splash takes off the wait for the next ghost.
pub const SPLASH_CALL: f64 = 4.0;
/// Seconds between the dust and the stone.
pub const STONE_DELAY: f64 = 2.0;
/// Seconds after you enter a room before the founder follows you in.
pub const FOUNDER_DELAY: f64 = 15.0;
/// How far a hit knocks the founder back, in cells.
pub const KNOCKBACK: i32 = 2;

thread_local! {
    static SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Fixes the seed every new run uses (tests), or `None` for the clock.
pub fn fix_seed(seed: Option<u64>) {
    SEED.with(|s| s.set(seed));
}

/// The seed a new run uses: the fixed one, else the clock.
pub fn run_seed() -> u64 {
    SEED.with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}

/// The things that sit in rooms, each with its own sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Clock,
    Crystal,
    Potion,
    MusicBox,
    Bell,
    Candle,
    Raven,
    Keys,
    Heart,
}

impl Item {
    pub const ALL: [Item; 9] = [
        Item::Clock,
        Item::Crystal,
        Item::Potion,
        Item::MusicBox,
        Item::Bell,
        Item::Candle,
        Item::Raven,
        Item::Keys,
        Item::Heart,
    ];

    /// "the ticking clock".
    pub fn name(self) -> &'static str {
        match self {
            Item::Clock => "ticking clock",
            Item::Crystal => "humming crystal",
            Item::Potion => "dripping potion",
            Item::MusicBox => "music box",
            Item::Bell => "silver bell",
            Item::Candle => "crackling candle",
            Item::Raven => "caged raven",
            Item::Keys => "ring of keys",
            Item::Heart => "beating heart",
        }
    }

    /// What it sounds like, in words.
    pub fn sound(self) -> &'static str {
        match self {
            Item::Clock => "a steady tick tock",
            Item::Crystal => "a low glassy hum",
            Item::Potion => "slow drips into glass",
            Item::MusicBox => "a tinkling little tune",
            Item::Bell => "a bright silver ring",
            Item::Candle => "the crackle and spit of a flame",
            Item::Raven => "a hoarse croak",
            Item::Keys => "a jangle of keys",
            Item::Heart => "a muffled double thump",
        }
    }

    /// The riddle that names it without saying it.
    pub fn riddle(self) -> &'static str {
        match self {
            Item::Clock => {
                "I have a face and two hands, yet I hold nothing. I count the hours of the haunt."
            }
            Item::Crystal => {
                "I sing with no mouth, one long low note, and I glow when the dead are near."
            }
            Item::Potion => "Drop by drop I fill my glass. One sip, and you will see the dead.",
            Item::MusicBox => "Turn my key and I play a tune for a child long gone.",
            Item::Bell => {
                "I have a tongue, but I never speak. Shake me and I call servants who died here."
            }
            Item::Candle => "The longer I live, the shorter I grow, and my flame spits as I go.",
            Item::Raven => "Black feathers in a cage of brass. I know one word: nevermore.",
            Item::Keys => {
                "Many teeth, but I never bite. We jangle together and open what is locked."
            }
            Item::Heart => "Beneath the floorboards I beat and beat, and I will not be still.",
        }
    }
}

/// The ghosts that drift past you, each with its own voice and pace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GhostKind {
    Widow,
    Butler,
    Child,
    /// The blind hound: it finds you if you stand still as it passes.
    Hound,
}

impl GhostKind {
    pub const ALL: [GhostKind; 4] = [
        GhostKind::Widow,
        GhostKind::Butler,
        GhostKind::Child,
        GhostKind::Hound,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GhostKind::Widow => "the weeping widow",
            GhostKind::Butler => "the chained butler",
            GhostKind::Child => "the whispering child",
            GhostKind::Hound => "the blind hound",
        }
    }

    pub fn voice(self) -> &'static str {
        match self {
            GhostKind::Widow => "a long falling wail",
            GhostKind::Butler => "dragging chains and a groan",
            GhostKind::Child => "soft whispers and a giggle",
            GhostKind::Hound => "panting and a low growl",
        }
    }

    /// Metres a second, before the floor's own speed-up.
    pub fn speed(self) -> f32 {
        match self {
            GhostKind::Widow => 1.8,
            GhostKind::Butler => 3.0,
            GhostKind::Child => 2.4,
            GhostKind::Hound => 2.6,
        }
    }

    /// The hound hunts stillness: keep walking while it passes.
    pub fn hunts_stillness(self) -> bool {
        self == GhostKind::Hound
    }
}

/// What caught you.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Threat {
    Ghost(GhostKind),
    /// The shadow that walks a room, hunting your footsteps.
    Shadow,
    /// A rotten board gave way under you.
    Board,
    /// The pendulum's blade.
    Blade,
    /// A stone from the ceiling.
    Stone,
    /// The founder's blow.
    Founder,
}

impl Threat {
    pub fn name(self) -> &'static str {
        match self {
            Threat::Ghost(g) => g.name(),
            Threat::Shadow => "the shadow",
            Threat::Board => "a rotten board",
            Threat::Blade => "the pendulum",
            Threat::Stone => "a falling stone",
            Threat::Founder => "the founder",
        }
    }

    /// Salt wards off what is dead: ghosts, the shadow and the founder.
    pub fn warded_by_salt(self) -> bool {
        matches!(self, Threat::Ghost(_) | Threat::Shadow | Threat::Founder)
    }
}

/// A wall of a room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    North,
    East,
    South,
    West,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::North, Side::East, Side::South, Side::West];

    pub fn name(self) -> &'static str {
        match self {
            Side::North => "north",
            Side::East => "east",
            Side::South => "south",
            Side::West => "west",
        }
    }

    pub fn opposite(self) -> Side {
        match self {
            Side::North => Side::South,
            Side::East => Side::West,
            Side::South => Side::North,
            Side::West => Side::East,
        }
    }

    /// One step this way.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Side::North => (0, 1),
            Side::East => (1, 0),
            Side::South => (0, -1),
            Side::West => (-1, 0),
        }
    }

    /// The side a one-step offset points to.
    pub fn of(dx: i32, dy: i32) -> Option<Side> {
        Side::ALL.into_iter().find(|s| s.delta() == (dx, dy))
    }
}

/// A door: which wall, and how far along it (column for north and south,
/// row for east and west).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Door {
    pub side: Side,
    pub at: i32,
}

/// Up to four things a séance calls up, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seq {
    items: [Item; 4],
    len: u8,
}

impl Seq {
    pub fn new(items: &[Item]) -> Seq {
        let mut a = [Item::Clock; 4];
        let len = items.len().clamp(1, 4);
        a[..len].copy_from_slice(&items[..len]);
        Seq {
            items: a,
            len: len as u8,
        }
    }

    pub fn items(&self) -> &[Item] {
        &self.items[..self.len as usize]
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// One room's puzzle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Puzzle {
    /// The thing is named: find it by its sound and pick it up.
    Find(Item),
    /// A phonograph plays its sound, unnamed: find what makes it.
    Match(Item),
    /// A riddle describes it: find it.
    Riddle(Item),
    /// A spirit seals the door and plays the sound of what it wants: offer
    /// it from what you carry.
    Offering(Item),
    /// The phonograph plays every sound in the room but one: find the one
    /// it leaves out.
    Missing(Item),
    /// A medium plays two to four sounds in turn, unnamed; pick up their
    /// things in that order.
    Seance(Seq),
}

impl Puzzle {
    pub fn item(self) -> Item {
        match self {
            Puzzle::Find(i)
            | Puzzle::Match(i)
            | Puzzle::Riddle(i)
            | Puzzle::Offering(i)
            | Puzzle::Missing(i) => i,
            Puzzle::Seance(s) => s.items()[0],
        }
    }

    /// Every thing the puzzle asks for, in order.
    pub fn wanted(self) -> Vec<Item> {
        match self {
            Puzzle::Seance(s) => s.items().to_vec(),
            p => vec![p.item()],
        }
    }

    pub fn kind_name(self) -> String {
        match self {
            Puzzle::Find(item) => format!("find the {}", item.name()),
            Puzzle::Match(_) => "match the phonograph's sound".to_string(),
            Puzzle::Riddle(_) => "answer the riddle".to_string(),
            Puzzle::Offering(_) => "make the spirit's offering".to_string(),
            Puzzle::Missing(_) => "find the sound the phonograph leaves out".to_string(),
            Puzzle::Seance(s) => format!("follow the séance, {} sounds in order", s.len()),
        }
    }
}

/// A thing lying in a room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub item: Item,
    pub x: i32,
    pub y: i32,
    pub taken: bool,
    /// A copy of the answer that hops from place to place. The real one
    /// keeps still.
    pub mimic: bool,
}

/// The shadow that walks a room cell by cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub x: i32,
    pub y: i32,
    /// Where it starts, and goes back to after it catches you.
    pub home: (i32, i32),
    /// Seconds between its steps.
    pub interval: f64,
    pub step_in: f64,
}

/// A charm lying in a room, taken by walking onto it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharmKind {
    /// Wards off one catch by a ghost, the shadow or the founder.
    Salt,
    /// Adds seconds to the haunt clock.
    Stub,
}

impl CharmKind {
    pub fn name(self) -> &'static str {
        match self {
            CharmKind::Salt => "a pinch of salt",
            CharmKind::Stub => "a candle stub",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Charm {
    pub kind: CharmKind,
    pub x: i32,
    pub y: i32,
    pub taken: bool,
}

/// A blade that swings along one row, one column at a time, back and forth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pendulum {
    pub row: i32,
    pub x: i32,
    /// 1 swinging east, -1 west.
    pub dir: i32,
    pub interval: f64,
    pub step_in: f64,
}

/// One room: a grid `w` cells west to east and `h` south to north. Cell
/// (0, 0) is the south-west corner.
#[derive(Clone, Debug, PartialEq)]
pub struct Room {
    pub name: &'static str,
    pub w: i32,
    pub h: i32,
    pub entry: Door,
    pub exit: Door,
    pub puzzle: Puzzle,
    pub items: Vec<Placed>,
    pub solved: bool,
    /// How many of a séance's things have been taken, in order.
    pub stage: usize,
    /// Boards that give way if you step on them.
    pub rotten: Vec<(i32, i32)>,
    /// Boards that gave way: holes you cannot walk into.
    pub holes: Vec<(i32, i32)>,
    /// Things call out only when you are near.
    pub dark: bool,
    /// Every sound comes from the other side: east from your left.
    pub mirror: bool,
    pub shadow: Option<Shadow>,
    pub charm: Option<Charm>,
    pub pendulum: Option<Pendulum>,
    /// Flooded cells: wading through them is slow and loud.
    pub water: Vec<(i32, i32)>,
    /// Stones fall from the ceiling here.
    pub stones: bool,
    /// The exit door moves now and then.
    pub shifts: bool,
}

impl Room {
    /// The cell just inside a door.
    pub fn inside(&self, door: Door) -> (i32, i32) {
        match door.side {
            Side::North => (door.at, self.h - 1),
            Side::South => (door.at, 0),
            Side::East => (self.w - 1, door.at),
            Side::West => (0, door.at),
        }
    }

    /// The door itself, half a cell beyond its inside cell, for sound.
    pub fn door_point(&self, door: Door) -> (f32, f32) {
        let (x, y) = self.inside(door);
        let (dx, dy) = door.side.delta();
        (x as f32 + dx as f32 * 0.6, y as f32 + dy as f32 * 0.6)
    }

    pub fn item_at(&self, x: i32, y: i32) -> Option<usize> {
        self.items
            .iter()
            .position(|p| !p.taken && p.x == x && p.y == y)
    }

    /// The puzzle's thing, while it still lies in the room: never a mimic.
    pub fn target(&self) -> Option<&Placed> {
        if self.solved {
            return None;
        }
        let want = self.next_wanted()?;
        self.items
            .iter()
            .find(|i| i.item == want && !i.taken && !i.mimic)
    }

    /// The thing the room wants picked up next, if any lies here.
    pub fn next_wanted(&self) -> Option<Item> {
        if self.solved {
            return None;
        }
        match self.puzzle {
            Puzzle::Offering(_) => None,
            p => p.wanted().get(self.stage).copied(),
        }
    }

    /// Flooded cells next to a cell, as offsets from it.
    pub fn water_near(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
        Side::ALL
            .iter()
            .map(|s| s.delta())
            .filter(|&(dx, dy)| self.water.contains(&(x + dx, y + dy)))
            .collect()
    }

    /// The charm still lying at a cell.
    pub fn charm_at(&self, x: i32, y: i32) -> Option<Charm> {
        self.charm.filter(|c| !c.taken && (c.x, c.y) == (x, y))
    }

    pub fn mimics(&self) -> usize {
        self.items.iter().filter(|p| p.mimic).count()
    }

    fn in_grid(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    /// Rotten boards next to a cell, as offsets from it.
    pub fn rotten_near(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
        Side::ALL
            .iter()
            .map(|s| s.delta())
            .filter(|&(dx, dy)| self.rotten.contains(&(x + dx, y + dy)))
            .collect()
    }

    /// A cell free for something to stand: no thing, board, hole or door.
    fn free(&self, x: i32, y: i32) -> bool {
        self.in_grid(x, y)
            && self.item_at(x, y).is_none()
            && !self.rotten.contains(&(x, y))
            && !self.holes.contains(&(x, y))
            && self.inside(self.exit) != (x, y)
            && self.inside(self.entry) != (x, y)
    }
}

/// A wing of the house: ten floors with one name and one set of rooms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wing {
    pub name: &'static str,
    pub rooms: &'static [&'static str],
    /// Footsteps ring on stone here, not wood.
    pub stone: bool,
}

/// The ten wings of the house, top to bottom, and the endless dark below.
pub const WINGS: [Wing; 11] = [
    Wing {
        name: "the ground floor",
        rooms: &[
            "the foyer",
            "the library",
            "the dining hall",
            "the parlour",
            "the cloakroom",
            "the smoking room",
            "the morning room",
            "the study",
        ],
        stone: false,
    },
    Wing {
        name: "the portrait gallery",
        rooms: &[
            "the long gallery",
            "the music room",
            "the conservatory",
            "the trophy room",
            "the billiard room",
            "the sculpture hall",
            "the map room",
            "the card room",
        ],
        stone: false,
    },
    Wing {
        name: "the bedchambers",
        rooms: &[
            "the nursery",
            "the master bedroom",
            "the dressing room",
            "the guest chamber",
            "the sewing room",
            "the sickroom",
            "the bathing room",
            "the linen store",
        ],
        stone: false,
    },
    Wing {
        name: "the servants' quarters",
        rooms: &[
            "the servants' hall",
            "the kitchen",
            "the scullery",
            "the laundry",
            "the butler's pantry",
            "the bell corridor",
            "the larder",
            "the boot room",
        ],
        stone: false,
    },
    Wing {
        name: "the attic",
        rooms: &[
            "the attic stairs",
            "the trunk room",
            "the dollhouse corner",
            "the rafters",
            "the bat roost",
            "the old chapel",
            "the hat boxes",
            "the broken skylight",
        ],
        stone: false,
    },
    Wing {
        name: "the clock tower",
        rooms: &[
            "the winding room",
            "the belfry stair",
            "the gear loft",
            "the pendulum shaft",
            "the clock face",
            "the bell chamber",
            "the weight well",
            "the keeper's cell",
        ],
        stone: false,
    },
    Wing {
        name: "the flooded cellars",
        rooms: &[
            "the cistern",
            "the cold store",
            "the coal cellar",
            "the pump room",
            "the drowned cellar",
            "the wine vault",
            "the ice house",
            "the well head",
        ],
        stone: true,
    },
    Wing {
        name: "the catacombs",
        rooms: &[
            "the ossuary",
            "the bone gallery",
            "the charnel house",
            "the niche passage",
            "the chapel of skulls",
            "the root cellar",
            "the burial shaft",
            "the forgotten stair",
        ],
        stone: true,
    },
    Wing {
        name: "the crypt",
        rooms: &[
            "the crypt stairs",
            "the family tomb",
            "the mausoleum",
            "the sarcophagus hall",
            "the weeping vault",
            "the lantern niche",
            "the sealed tomb",
            "the founder's grave",
        ],
        stone: true,
    },
    Wing {
        name: "the heart of the house",
        rooms: &[
            "the hollow",
            "the black hall",
            "the mirror maze",
            "the endless stair",
            "the room of doors",
            "the cold heart",
            "the last door",
            "the throat of the house",
        ],
        stone: true,
    },
    Wing {
        name: "the endless dark",
        rooms: &[
            "a nameless room",
            "a room with no corners",
            "a room that hums",
            "a room of whispers",
            "a drowned room",
            "a room of bones",
            "a room that breathes",
            "a room with no ceiling",
        ],
        stone: true,
    },
];

/// The wing a floor is in, 0 for the ground floor, 10 for the endless dark.
pub fn wing_index(floor: u32) -> usize {
    ((floor.max(1) - 1) / WING_FLOORS).min(10) as usize
}

pub fn wing(floor: u32) -> &'static Wing {
    &WINGS[wing_index(floor)]
}

/// The first floor of a wing, counting wings from 0.
pub fn wing_first(index: usize) -> u32 {
    index as u32 * WING_FLOORS + 1
}

/// How a floor plays.
#[derive(Clone, Debug, PartialEq)]
pub struct FloorSpec {
    pub number: u32,
    pub name: &'static str,
    /// Points scale with this: the floor number, or the ball's own level.
    pub level: u32,
    pub rooms: usize,
    pub size: i32,
    /// Things in a room that are not the answer.
    pub decoys: usize,
    pub haunt: f64,
    /// Seconds between ghosts, low and high.
    pub ghost_gap: (f64, f64),
    pub ghosts: &'static [GhostKind],
    pub ghost_speed: f32,
    /// Whether "Freeze!" is spoken as the ghost arrives, as well as its cue.
    pub call_freeze: bool,
    /// Haunt seconds lost for touching or offering the wrong thing.
    pub penalty: f64,
    /// Whether H gives hints.
    pub hints: bool,
    /// Haunt seconds a hint costs.
    pub hint_cost: f64,
    /// Things call out softer and less often.
    pub lights_out: bool,
    /// The monthly ball: séance rooms and the ballroom's own names.
    pub seance: bool,
    /// Which puzzles the floor may set.
    pub offerings: bool,
    pub missing: bool,
    /// Things a house séance calls up, 0 for no séances.
    pub seance_len: usize,
    /// Rotten boards in each room after the first.
    pub rotten: usize,
    /// Chance that a room is dark, or mirrored, or walked by the shadow.
    pub dark: f32,
    pub mirror: f32,
    pub shadow: f32,
    /// Seconds between the shadow's steps.
    pub shadow_pace: f64,
    /// Mimics in a room that has them, and the chance a room does.
    pub mimics: usize,
    pub mimic_chance: f32,
    /// Chance a room holds a charm.
    pub charms: f32,
    /// Chance a room has a pendulum, and its seconds per column.
    pub pendulum: f32,
    pub pendulum_pace: f64,
    /// Chance a room is partly flooded.
    pub water: f32,
    /// Chance stones fall in a room, and seconds between them, low and high.
    pub stones: f32,
    pub stone_gap: (f64, f64),
    /// Hits the founder takes before he crumbles, 0 where he does not walk.
    pub founder: u32,
    /// Seconds between his steps, and how long his blow takes to land.
    pub founder_pace: f64,
    pub founder_wind: f64,
    /// Chance a room's exit door moves, and seconds between moves.
    pub shifts: f32,
    pub shift_gap: (f64, f64),
    /// The twists this floor carries, spoken as it begins.
    pub twists: Vec<Twist>,
}

impl FloorSpec {
    /// Every danger this floor can hold, for its briefing.
    pub fn dangers(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.rotten > 0 {
            out.push("rotten boards");
        }
        if self.dark > 0.0 {
            out.push("dark rooms");
        }
        if self.shadow > 0.0 {
            out.push("the shadow");
        }
        if self.ghosts.contains(&GhostKind::Hound) {
            out.push("the blind hound");
        }
        if self.mimics > 0 {
            out.push("mimics");
        }
        if self.mirror > 0.0 {
            out.push("mirrored rooms");
        }
        if self.pendulum > 0.0 {
            out.push("the pendulum");
        }
        if self.water > 0.0 {
            out.push("deep water");
        }
        if self.stones > 0.0 {
            out.push("falling stones");
        }
        if self.founder > 0 {
            out.push("the founder");
        }
        if self.shifts > 0.0 {
            out.push("the shifting house");
        }
        out
    }
}

/// Rounds seconds to the nearest ten.
fn tens(secs: f64) -> f64 {
    (secs / 10.0).round() * 10.0
}

/// A floor of the house, generated from its number: any number from 1 up,
/// past floor 100 into the endless dark. Nightmare has less time, faster
/// and more frequent ghosts, and never calls "Freeze!".
pub fn floor_spec(number: u32, nightmare: bool) -> FloorSpec {
    let mut spec = floor_base(number, nightmare);
    for t in floor_twists(number.max(1)) {
        t.apply(&mut spec);
    }
    spec
}

/// A floor before its own twists are laid on.
pub fn floor_base(number: u32, nightmare: bool) -> FloorSpec {
    use GhostKind::*;
    let n = number.max(1);
    // How far down the house, 0 at the top and 1 at floor 100; the endless
    // dark goes on past 1.
    let t = (n - 1) as f64 / 99.0;
    let tc = t.min(1.0);
    let rooms = (3 + (n - 1) / 18).min(8) as usize;
    let size = (5 + (n - 1) / 20).min(9) as i32;
    let seance_len: usize = match n {
        _ if n < SEANCES_FROM => 0,
        _ if n < 40 => 2,
        _ if n < 75 => 3,
        _ => 4,
    };
    let decoys = (1 + (n - 1) / 12).min(6) as usize;
    let per_room = 30.0 + 9.0 * size as f64 + 12.0 * seance_len.saturating_sub(1) as f64;
    // Deeper floors give less time for each room, down to three quarters,
    // and the endless dark keeps squeezing to just over a half.
    let pressure = if t <= 1.0 {
        1.0 - 0.25 * t
    } else {
        (0.75 - 0.004 * (n - FLOORS) as f64).max(0.55)
    };
    let ghosts: &'static [GhostKind] = match n {
        _ if n < 5 => &[Widow],
        _ if n < 10 => &[Widow, Butler],
        _ if n < HOUND_FROM => &[Widow, Butler, Child],
        _ => &GhostKind::ALL,
    };
    let gap = 1.0 - 0.6 * tc;
    let mut spec = FloorSpec {
        number: n,
        name: wing(n).name,
        level: n,
        rooms,
        size,
        decoys,
        haunt: tens(rooms as f64 * per_room * pressure),
        ghost_gap: (35.0 * gap, 50.0 * gap),
        ghosts,
        ghost_speed: (1.0 + 0.5 * t).min(2.0) as f32,
        call_freeze: n <= 4,
        penalty: match n {
            _ if n < 30 => WRONG_PENALTY,
            _ if n < 70 => 15.0,
            _ => 20.0,
        },
        hints: true,
        hint_cost: match n {
            _ if n < 20 => HINT_COST,
            _ if n < 60 => 10.0,
            _ => 15.0,
        },
        lights_out: false,
        seance: false,
        offerings: n >= OFFERINGS_FROM,
        missing: n >= MISSING_FROM,
        seance_len,
        rotten: if n < ROTTEN_FROM {
            0
        } else {
            (1 + (n - ROTTEN_FROM) / 15).min(6) as usize
        },
        dark: if n < DARK_FROM {
            0.0
        } else {
            (0.2 + 0.3 * t).min(0.6) as f32
        },
        mirror: if n < MIRRORS_FROM {
            0.0
        } else {
            (0.15 + 0.25 * t).min(0.5) as f32
        },
        shadow: if n < SHADOW_FROM {
            0.0
        } else {
            (0.25 + 0.5 * t).min(0.85) as f32
        },
        shadow_pace: (2.2 - 1.2 * t).max(0.9),
        mimics: match n {
            _ if n < MIMICS_FROM => 0,
            _ if n < 80 => 1,
            _ => 2,
        },
        mimic_chance: if n < MIMICS_FROM { 0.0 } else { 0.5 },
        charms: if n < CHARMS_FROM { 0.0 } else { 0.3 },
        pendulum: if n < PENDULUM_FROM {
            0.0
        } else {
            (0.35 + 0.5 * (t - 0.5)).min(0.6) as f32
        },
        pendulum_pace: (0.6 - 0.2 * (t - 0.5).max(0.0)).max(0.4),
        water: if n < WATER_FROM {
            0.0
        } else {
            (0.4 + 0.5 * (t - 0.6)).min(0.6) as f32
        },
        stones: if n < STONES_FROM {
            0.0
        } else {
            (0.4 + 0.5 * (t - 0.7)).min(0.6) as f32
        },
        stone_gap: (12.0 - 3.0 * tc, 18.0 - 3.0 * tc),
        founder: match n {
            _ if n < FOUNDER_FROM => 0,
            _ if n < SHIFTS_FROM => 3,
            _ if n <= FLOORS => 4,
            _ => 5,
        },
        founder_pace: (1.5 - 0.5 * (t - 0.8).max(0.0)).max(1.1),
        founder_wind: (1.4 - 0.5 * (t - 0.8).max(0.0)).max(1.0),
        shifts: if n < SHIFTS_FROM { 0.0 } else { 0.5 },
        shift_gap: (30.0, 42.0),
        twists: Vec::new(),
    };
    if nightmare {
        spec.haunt = (spec.haunt * 0.75).round();
        spec.ghost_gap = (spec.ghost_gap.0 * 0.7, spec.ghost_gap.1 * 0.7);
        spec.ghost_speed *= 1.25;
        spec.call_freeze = false;
        if spec.ghosts.len() < 3 {
            spec.ghosts = &[Widow, Butler, Child];
        }
    }
    spec
}

/// The twists a floor carries: none above floor 11, then one on some
/// floors, one on every floor from 41, two from 71, three in the endless
/// dark. The same for a floor number every time.
pub fn floor_twists(n: u32) -> Vec<Twist> {
    let mut rng = Rng::seeded(0x7715_7000 ^ (n as u64).wrapping_mul(2_654_435_761));
    let count = match n {
        _ if n < TWISTS_FROM => 0,
        _ if n <= 40 => usize::from(rng.chance(0.5)),
        _ if n <= 70 => 1,
        _ if n <= FLOORS => 2,
        _ => 3,
    };
    let mut pool: Vec<Twist> = Twist::ALL
        .into_iter()
        .filter(|t| n >= t.from_floor())
        .collect();
    rng.shuffle(&mut pool);
    pool.truncate(count);
    pool
}

/// The floor number of the monthly ball, a wing of the house that opens
/// only for the monthly challenge.
pub const BALL_FLOOR: u32 = 1000;

/// The monthly ball: five rooms, séances in most of them, every ghost
/// abroad, and a longer clock to match.
pub fn ball_spec() -> FloorSpec {
    use GhostKind::*;
    FloorSpec {
        number: BALL_FLOOR,
        name: "the midnight masquerade",
        level: 6,
        rooms: 5,
        size: 6,
        decoys: 2,
        haunt: 400.0,
        ghost_gap: (24.0, 34.0),
        ghosts: &[Widow, Butler, Child],
        ghost_speed: 1.15,
        call_freeze: true,
        penalty: WRONG_PENALTY,
        hints: true,
        hint_cost: HINT_COST,
        lights_out: false,
        seance: true,
        offerings: true,
        missing: false,
        seance_len: 2,
        rotten: 0,
        dark: 0.0,
        mirror: 0.0,
        shadow: 0.0,
        shadow_pace: 2.0,
        mimics: 0,
        mimic_chance: 0.0,
        charms: 0.3,
        pendulum: 0.0,
        pendulum_pace: 0.6,
        water: 0.0,
        stones: 0.0,
        stone_gap: (12.0, 18.0),
        founder: 0,
        founder_pace: 1.5,
        founder_wind: 1.4,
        shifts: 0.0,
        shift_gap: (30.0, 42.0),
        twists: Vec::new(),
    }
}

/// A rule that changes a floor: some floors carry one, a daily or weekly
/// challenge has one, and a player can take any they have earned as a
/// curse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Twist {
    /// Ghosts come twice as often, every kind of them.
    Restless,
    /// Things call out softer and less often.
    LightsOut,
    /// The haunt clock is 40 percent shorter.
    WitchingHour,
    /// A wrong touch or offering costs double.
    CursedHands,
    /// No hints.
    SilentSpirits,
    /// Every room is dark.
    Darkness,
    /// Every room is mirrored.
    Mirrors,
}

impl Twist {
    pub const ALL: [Twist; 7] = [
        Twist::Restless,
        Twist::LightsOut,
        Twist::WitchingHour,
        Twist::CursedHands,
        Twist::SilentSpirits,
        Twist::Darkness,
        Twist::Mirrors,
    ];

    /// The twists a daily or weekly challenge picks from.
    pub const CHALLENGE: [Twist; 5] = [
        Twist::Restless,
        Twist::LightsOut,
        Twist::WitchingHour,
        Twist::CursedHands,
        Twist::SilentSpirits,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::Restless => "Restless dead",
            Twist::LightsOut => "Lights out",
            Twist::WitchingHour => "Witching hour",
            Twist::CursedHands => "Cursed hands",
            Twist::SilentSpirits => "Silent spirits",
            Twist::Darkness => "Pitch dark",
            Twist::Mirrors => "Hall of mirrors",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Twist::Restless => "ghosts twice as often",
            Twist::LightsOut => "things call softer and less often",
            Twist::WitchingHour => "a shorter clock",
            Twist::CursedHands => "wrong things cost double",
            Twist::SilentSpirits => "no hints",
            Twist::Darkness => "every room dark",
            Twist::Mirrors => "every room mirrored",
        }
    }

    /// The first floor that may carry it.
    pub fn from_floor(self) -> u32 {
        match self {
            Twist::Darkness => DARK_FROM,
            Twist::Mirrors => MIRRORS_FROM,
            _ => TWISTS_FROM,
        }
    }

    /// A stable id for saves and unlocks.
    pub fn id(self) -> &'static str {
        match self {
            Twist::Restless => "restless",
            Twist::LightsOut => "lights-out",
            Twist::WitchingHour => "witching-hour",
            Twist::CursedHands => "cursed-hands",
            Twist::SilentSpirits => "silent-spirits",
            Twist::Darkness => "pitch-dark",
            Twist::Mirrors => "mirrors",
        }
    }

    /// The floor as this twist changes it.
    pub fn apply(self, spec: &mut FloorSpec) {
        match self {
            Twist::Restless => {
                spec.ghost_gap = (spec.ghost_gap.0 * 0.5, spec.ghost_gap.1 * 0.5);
                if spec.ghosts.len() < 3 {
                    spec.ghosts = &[GhostKind::Widow, GhostKind::Butler, GhostKind::Child];
                }
            }
            Twist::LightsOut => spec.lights_out = true,
            Twist::WitchingHour => spec.haunt = (spec.haunt * 0.6).round(),
            Twist::CursedHands => spec.penalty *= 2.0,
            Twist::SilentSpirits => spec.hints = false,
            Twist::Darkness => spec.dark = 1.0,
            Twist::Mirrors => spec.mirror = 1.0,
        }
        if !spec.twists.contains(&self) {
            spec.twists.push(self);
        }
    }
}

/// The curses a player has switched on: twists taken by choice, each worth
/// a quarter more score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Curses(pub u8);

impl Curses {
    fn bit(t: Twist) -> u8 {
        1 << Twist::ALL.iter().position(|&x| x == t).unwrap_or(0)
    }

    pub fn has(self, t: Twist) -> bool {
        self.0 & Self::bit(t) != 0
    }

    pub fn with(self, t: Twist) -> Curses {
        Curses(self.0 | Self::bit(t))
    }

    pub fn list(self) -> Vec<Twist> {
        Twist::ALL.into_iter().filter(|&t| self.has(t)).collect()
    }

    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Score with the curses' bonus: a quarter more for each.
    pub fn bonus(self, points: u64) -> u64 {
        points + points * self.count() as u64 / 4
    }

    pub fn apply(self, spec: &mut FloorSpec) {
        for t in self.list() {
            t.apply(spec);
        }
    }
}

/// A challenge run: which floors, in order, and its twist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub period: Period,
    pub first: u32,
    pub last: u32,
    pub twist: Option<Twist>,
}

impl Event {
    /// The event for a challenge, and the seed its layouts are built from:
    /// the same for every player in the period.
    pub fn from_challenge(ch: &Challenge) -> (Event, u64) {
        let mut rng = ch.rng();
        let event = match ch.period {
            // One floor from the upper house, with a twist.
            Period::Daily => {
                let first = 3 + rng.below(28);
                Event {
                    period: Period::Daily,
                    first,
                    last: first,
                    twist: Some(*rng.pick(&Twist::CHALLENGE)),
                }
            }
            // Three floors in a row, with a twist, courage carried.
            Period::Weekly => {
                let first = 10 + rng.below(31);
                Event {
                    period: Period::Weekly,
                    first,
                    last: first + 2,
                    twist: Some(*rng.pick(&Twist::CHALLENGE)),
                }
            }
            Period::Monthly => Event {
                period: Period::Monthly,
                first: BALL_FLOOR,
                last: BALL_FLOOR,
                twist: None,
            },
        };
        (event, rng.next_u64())
    }

    /// How a floor of this event plays.
    pub fn spec(self, floor: u32) -> FloorSpec {
        if self.period == Period::Monthly {
            return ball_spec();
        }
        let mut spec = floor_spec(floor, false);
        if let Some(t) = self.twist {
            t.apply(&mut spec);
        }
        spec
    }

    /// The weekly's courage lasts the whole night: it does not refill
    /// between floors.
    pub fn carries_courage(self) -> bool {
        self.period == Period::Weekly
    }

    /// How many floors the event climbs.
    pub fn floors(self) -> u32 {
        self.last - self.first + 1
    }
}

const BALL_ROOMS: &[&str] = &[
    "the ballroom doors",
    "the grand ballroom",
    "the masquerade gallery",
    "the moonlit terrace",
    "the banquet hall",
    "the orchestra gallery",
];

/// Cells reachable from `from` without stepping on `blocked`.
fn reachable(w: i32, h: i32, from: (i32, i32), blocked: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut seen = vec![from];
    let mut queue = VecDeque::from([from]);
    while let Some((x, y)) = queue.pop_front() {
        for s in Side::ALL {
            let (dx, dy) = s.delta();
            let c = (x + dx, y + dy);
            if c.0 >= 0
                && c.1 >= 0
                && c.0 < w
                && c.1 < h
                && !blocked.contains(&c)
                && !seen.contains(&c)
            {
                seen.push(c);
                queue.push_back(c);
            }
        }
    }
    seen
}

/// Which kind of puzzle a room sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Find,
    Match,
    Riddle,
    Offering,
    Missing,
    Seance,
}

/// Lays out a floor's rooms: deterministic for a seed.
pub fn plan_floor(spec: &FloorSpec, rng: &mut Rng) -> Vec<Room> {
    let mut names: Vec<&'static str> = if spec.seance {
        BALL_ROOMS.to_vec()
    } else {
        wing(spec.number).rooms.to_vec()
    };
    rng.shuffle(&mut names[1..]);
    let s = spec.size;
    let mut rooms: Vec<Room> = Vec::new();
    let mut targets_used: Vec<Item> = Vec::new();
    // Targets found so far, not yet given to a spirit.
    let mut offerable: Vec<Item> = Vec::new();
    let mut entry = Door {
        side: Side::South,
        at: s / 2,
    };
    for i in 0..spec.rooms {
        let exit_sides: Vec<Side> = Side::ALL
            .iter()
            .copied()
            .filter(|&d| d != entry.side && d != Side::South)
            .collect();
        let exit_side = *rng.pick(&exit_sides);
        let exit = Door {
            side: exit_side,
            at: rng.below(s as u32) as i32,
        };
        let kind = pick_kind(spec, i, &rooms, &offerable, rng);
        let fresh = |used: &Vec<Item>| -> Vec<Item> {
            Item::ALL
                .iter()
                .copied()
                .filter(|it| !used.contains(it))
                .collect()
        };
        let puzzle = match kind {
            Kind::Offering => {
                let at = rng.below(offerable.len() as u32) as usize;
                Puzzle::Offering(offerable.remove(at))
            }
            Kind::Seance => {
                let len = if spec.seance {
                    2
                } else {
                    spec.seance_len.max(2)
                };
                let mut pool = fresh(&targets_used);
                if pool.len() < len {
                    pool = Item::ALL.to_vec();
                }
                rng.shuffle(&mut pool);
                pool.truncate(len);
                targets_used.extend(pool.iter().copied());
                offerable.extend(pool.iter().copied());
                Puzzle::Seance(Seq::new(&pool))
            }
            _ => {
                let mut pool = fresh(&targets_used);
                if pool.is_empty() {
                    pool = Item::ALL.to_vec();
                }
                let item = *rng.pick(&pool);
                targets_used.push(item);
                offerable.push(item);
                match kind {
                    Kind::Find => Puzzle::Find(item),
                    Kind::Match => Puzzle::Match(item),
                    Kind::Riddle => Puzzle::Riddle(item),
                    _ => Puzzle::Missing(item),
                }
            }
        };
        let mut room = Room {
            name: names[i % names.len()],
            w: s,
            h: s,
            entry,
            exit,
            puzzle,
            items: Vec::new(),
            solved: false,
            stage: 0,
            rotten: Vec::new(),
            holes: Vec::new(),
            dark: false,
            mirror: false,
            shadow: None,
            charm: None,
            pendulum: None,
            water: Vec::new(),
            stones: false,
            shifts: false,
        };
        let start = room.inside(entry);
        let door_cell = room.inside(exit);
        if !matches!(puzzle, Puzzle::Offering(_)) {
            let mut things = puzzle.wanted();
            let mut others: Vec<Item> = Item::ALL
                .iter()
                .copied()
                .filter(|it| !things.contains(it))
                .collect();
            rng.shuffle(&mut others);
            let room_for = Item::ALL.len() - things.len();
            let decoys = if matches!(puzzle, Puzzle::Missing(_)) {
                spec.decoys.max(2)
            } else {
                spec.decoys
            };
            things.extend(others.into_iter().take(decoys.min(room_for)));
            let mut cells: Vec<(i32, i32)> = (0..s)
                .flat_map(|x| (0..s).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    (x, y) != door_cell && (x - start.0).abs() + (y - start.1).abs() >= 2
                })
                .collect();
            rng.shuffle(&mut cells);
            for (item, (x, y)) in things.into_iter().zip(cells) {
                room.items.push(Placed {
                    item,
                    x,
                    y,
                    taken: false,
                    mimic: false,
                });
            }
        }
        // The first room of a floor stays gentle: no hazards in it.
        let first = i == 0;
        if !first {
            room.dark = rng.unit() < spec.dark as f64;
            room.mirror = rng.unit() < spec.mirror as f64;
            place_rotten(&mut room, spec.rotten, rng);
            if rng.unit() < spec.shadow as f64 {
                let home = (0..s)
                    .flat_map(|x| (0..s).map(move |y| (x, y)))
                    .filter(|&(x, y)| room.free(x, y))
                    .max_by_key(|&(x, y)| (x - start.0).abs() + (y - start.1).abs())
                    .unwrap_or((s - 1, s - 1));
                room.shadow = Some(Shadow {
                    x: home.0,
                    y: home.1,
                    home,
                    interval: spec.shadow_pace,
                    step_in: spec.shadow_pace * 2.0,
                });
            }
            let mimicked = matches!(
                puzzle,
                Puzzle::Find(_) | Puzzle::Match(_) | Puzzle::Riddle(_) | Puzzle::Missing(_)
            );
            if mimicked && spec.mimics > 0 && rng.unit() < spec.mimic_chance as f64 {
                let item = puzzle.item();
                for _ in 0..spec.mimics {
                    let cells: Vec<(i32, i32)> = (0..s)
                        .flat_map(|x| (0..s).map(move |y| (x, y)))
                        .filter(|&(x, y)| {
                            room.free(x, y) && (x - start.0).abs() + (y - start.1).abs() >= 2
                        })
                        .collect();
                    if cells.is_empty() {
                        break;
                    }
                    let (x, y) = *rng.pick(&cells);
                    room.items.push(Placed {
                        item,
                        x,
                        y,
                        taken: false,
                        mimic: true,
                    });
                }
            }
        }
        rooms.push(room);
        entry = Door {
            side: exit.side.opposite(),
            at: exit.at,
        };
    }
    // Drawn after the layout, so the rooms above stay as they always were.
    for (i, room) in rooms.iter_mut().enumerate() {
        add_deep(spec, i, room, rng);
    }
    rooms
}

/// Whether room `i` gets a danger that comes in with probability `chance`
/// from floor `from`: on its first floor the second room always has it, so
/// the player meets it straight away.
fn debut(spec: &FloorSpec, from: u32, i: usize, chance: f32, rng: &mut Rng) -> bool {
    if chance <= 0.0 || i == 0 {
        return false;
    }
    let roll = rng.unit() < chance as f64;
    roll || (spec.number == from && i == 1 && !spec.seance)
}

/// The deeper dangers, and charms: water, the pendulum, falling stones, the
/// shifting door, then a charm on a free cell.
fn add_deep(spec: &FloorSpec, i: usize, room: &mut Room, rng: &mut Rng) {
    let start = room.inside(room.entry);
    let exit = room.inside(room.exit);
    if debut(spec, WATER_FROM, i, spec.water, rng) {
        // A pool grown from one cell, never on the doors.
        let want = (room.w * room.h / 5) as usize;
        let cells: Vec<(i32, i32)> = (0..room.w)
            .flat_map(|x| (0..room.h).map(move |y| (x, y)))
            .filter(|&c| c != start && c != exit && !room.rotten.contains(&c))
            .collect();
        let mut pool = vec![*rng.pick(&cells)];
        let mut tries = 0;
        while pool.len() < want && tries < 200 {
            tries += 1;
            let (x, y) = *rng.pick(&pool);
            let (dx, dy) = rng.pick(&Side::ALL).delta();
            let c = (x + dx, y + dy);
            if cells.contains(&c) && !pool.contains(&c) {
                pool.push(c);
            }
        }
        room.water = pool;
    }
    if debut(spec, PENDULUM_FROM, i, spec.pendulum, rng) {
        let rows: Vec<i32> = (1..room.h - 1)
            .filter(|&y| y != start.1 && y != exit.1)
            .collect();
        if !rows.is_empty() {
            let row = *rng.pick(&rows);
            room.pendulum = Some(Pendulum {
                row,
                x: 0,
                dir: 1,
                interval: spec.pendulum_pace,
                step_in: spec.pendulum_pace,
            });
        }
    }
    room.stones = debut(spec, STONES_FROM, i, spec.stones, rng);
    room.shifts = debut(spec, SHIFTS_FROM, i, spec.shifts, rng);
    if spec.charms > 0.0 && i > 0 && rng.unit() < spec.charms as f64 {
        let cells: Vec<(i32, i32)> = (0..room.w)
            .flat_map(|x| (0..room.h).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                room.free(x, y)
                    && room.pendulum.is_none_or(|p| p.row != y)
                    && room.shadow.is_none_or(|s| s.home != (x, y))
            })
            .collect();
        if !cells.is_empty() {
            let (x, y) = *rng.pick(&cells);
            let kind = if rng.below(2) == 0 {
                CharmKind::Salt
            } else {
                CharmKind::Stub
            };
            room.charm = Some(Charm {
                kind,
                x,
                y,
                taken: false,
            });
        }
    }
}

/// Floor 1 teaches each kind in turn; later floors mix them, harder kinds
/// more often the deeper you go, with at least one offering, and never an
/// offering with nothing to give. The ball: a match to warm up, then
/// séances, and one offering.
fn pick_kind(
    spec: &FloorSpec,
    i: usize,
    rooms: &[Room],
    offerable: &[Item],
    rng: &mut Rng,
) -> Kind {
    if spec.seance {
        return match i {
            0 => Kind::Match,
            2 if !offerable.is_empty() => Kind::Offering,
            _ => Kind::Seance,
        };
    }
    if spec.number == 1 {
        return [Kind::Find, Kind::Match, Kind::Riddle][i % 3];
    }
    if i == 0 {
        return if rng.below(2) == 0 {
            Kind::Find
        } else {
            Kind::Match
        };
    }
    let has_offering = rooms
        .iter()
        .any(|r| matches!(r.puzzle, Puzzle::Offering(_)));
    if spec.offerings && !offerable.is_empty() && i == spec.rooms - 1 && !has_offering {
        return Kind::Offering;
    }
    let mut pool: Vec<(Kind, u32)> = vec![(Kind::Find, 1), (Kind::Match, 2), (Kind::Riddle, 2)];
    if spec.offerings && !offerable.is_empty() {
        pool.push((Kind::Offering, 2));
    }
    if spec.missing {
        pool.push((Kind::Missing, 3));
    }
    if spec.seance_len >= 2 {
        pool.push((Kind::Seance, 3));
    }
    let total: u32 = pool.iter().map(|p| p.1).sum();
    let mut roll = rng.below(total);
    for (kind, w) in pool {
        if roll < w {
            return kind;
        }
        roll -= w;
    }
    Kind::Find
}

/// Lays rotten boards where they leave every thing and the exit reachable.
fn place_rotten(room: &mut Room, count: usize, rng: &mut Rng) {
    if count == 0 {
        return;
    }
    let start = room.inside(room.entry);
    let exit = room.inside(room.exit);
    let mut cells: Vec<(i32, i32)> = (0..room.w)
        .flat_map(|x| (0..room.h).map(move |y| (x, y)))
        .filter(|&(x, y)| {
            room.item_at(x, y).is_none()
                && (x, y) != exit
                && (x - start.0).abs() + (y - start.1).abs() >= 2
        })
        .collect();
    rng.shuffle(&mut cells);
    let mut must: Vec<(i32, i32)> = room.items.iter().map(|p| (p.x, p.y)).collect();
    must.push(exit);
    for c in cells {
        if room.rotten.len() >= count {
            break;
        }
        let mut trial = room.rotten.clone();
        trial.push(c);
        let open = reachable(room.w, room.h, start, &trial);
        if must.iter().all(|m| open.contains(m)) {
            room.rotten = trial;
        }
    }
}

/// Which haunt is being played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The climb: from a floor, floor after floor, to floor 100 or until
    /// the house stops you.
    Climb(u32),
    /// The whole house from floor 1, harder, with courage carried.
    Nightmare,
    /// The floors below floor 100, without end, with courage carried.
    Endless,
    /// One floor on its own.
    Floor(u32),
    /// A daily, weekly or monthly challenge.
    Challenge(Event),
    /// A run-up floor of Halloween, lighting a candle.
    Stirs(u32),
    /// The Lantern Night, with this many candles lit.
    LanternNight(u32),
}

impl Mode {
    pub fn nightmare(self) -> bool {
        self == Mode::Nightmare
    }

    pub fn first_floor(self) -> u32 {
        match self {
            Mode::Climb(n) | Mode::Floor(n) | Mode::Stirs(n) => n.max(1),
            Mode::Challenge(e) => e.first,
            Mode::LanternNight(_) => 1,
            Mode::Endless => FLOORS + 1,
            Mode::Nightmare => 1,
        }
    }

    pub fn last_floor(self) -> u32 {
        match self {
            Mode::Floor(n) | Mode::Stirs(n) => n.max(1),
            Mode::Challenge(e) => e.last,
            Mode::LanternNight(_) => crate::allhallows::CANDLES,
            Mode::Climb(n) => n.max(FLOORS),
            Mode::Nightmare => FLOORS,
            Mode::Endless => u32::MAX,
        }
    }

    /// The challenge this run is, if any.
    pub fn event(self) -> Option<Event> {
        match self {
            Mode::Challenge(e) => Some(e),
            _ => None,
        }
    }

    /// Whether courage lasts the whole run instead of refilling each floor.
    pub fn carries_courage(self) -> bool {
        match self {
            Mode::Nightmare | Mode::Endless | Mode::LanternNight(_) => true,
            Mode::Challenge(e) => e.carries_courage(),
            _ => false,
        }
    }

    /// Floors between courage given back, when courage is carried.
    pub fn landing_every(self) -> Option<u32> {
        match self {
            Mode::Nightmare => Some(WING_FLOORS),
            Mode::Endless | Mode::LanternNight(_) => Some(5),
            _ => None,
        }
    }

    /// How a floor plays in this mode.
    pub fn spec(self, floor: u32) -> FloorSpec {
        match self {
            Mode::Challenge(e) => e.spec(floor),
            Mode::Stirs(_) => {
                let mut spec = floor_spec(floor, false);
                Twist::Restless.apply(&mut spec);
                spec
            }
            Mode::LanternNight(lit) => {
                let mut spec = floor_spec(floor, false);
                Twist::Restless.apply(&mut spec);
                Twist::LightsOut.apply(&mut spec);
                spec.haunt += lit as f64 * crate::allhallows::CANDLE_SECS;
                spec
            }
            m => floor_spec(floor, m.nightmare()),
        }
    }

    /// The best-score board.
    pub fn board(self) -> String {
        match self {
            Mode::Climb(_) => "climb".to_string(),
            Mode::Nightmare => "nightmare".to_string(),
            Mode::Endless => "endless".to_string(),
            Mode::Floor(n) => format!("floor {n}"),
            Mode::Challenge(e) => format!("{} challenge", e.period.label().to_lowercase()),
            Mode::Stirs(_) => crate::allhallows::STIRS_BOARD.to_string(),
            Mode::LanternNight(_) => crate::allhallows::NIGHT_BOARD.to_string(),
        }
    }
}

/// Where the ghost is on its pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhostPhase {
    Approaching,
    Near,
    /// Past you, and safe: the run lets it go the moment it gets here.
    Leaving,
}

/// A ghost drifting past you along a straight line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ghost {
    pub kind: GhostKind,
    /// The compass bearing it comes from, radians, 0 north, clockwise.
    pub bearing: f32,
    /// Signed metres along its path: positive before it reaches you.
    pub s: f32,
    pub speed: f32,
    pub phase: GhostPhase,
}

impl Ghost {
    /// Where it is relative to you, metres: x east, y north.
    pub fn offset(&self) -> (f32, f32) {
        let (sin, cos) = self.bearing.sin_cos();
        // A little to one side, so it passes rather than goes through.
        let lateral = 0.8;
        (sin * self.s + cos * lateral, cos * self.s - sin * lateral)
    }

    /// The compass direction it comes from, in words.
    pub fn from_name(&self) -> &'static str {
        compass(self.bearing)
    }
}

/// The nearest of eight compass points to a bearing.
pub fn compass(bearing: f32) -> &'static str {
    const NAMES: [&str; 8] = [
        "north",
        "north-east",
        "east",
        "south-east",
        "south",
        "south-west",
        "west",
        "north-west",
    ];
    let i = ((bearing.rem_euclid(TAU) / (TAU / 8.0)).round() as usize) % 8;
    NAMES[i]
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Escaped,
    TimeUp,
    ChasedOut,
}

/// What happened on the floor being played, for unlocks and results.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloorStats {
    pub caught: u32,
    pub falls: u32,
    pub shadow_catches: u32,
    pub hints: u32,
    pub ghosts: Vec<GhostKind>,
    pub rotten: bool,
    pub shadow: bool,
    pub mimic: bool,
    pub mirror: bool,
    pub dark: bool,
    pub spirit: bool,
    pub seance: bool,
    pub pendulum: bool,
    pub water: bool,
    pub stones: bool,
    pub shifted: bool,
    /// The founder came for you on this floor.
    pub founder: bool,
    /// You struck him down.
    pub banished: bool,
    /// Blows of his that landed.
    pub founder_hits: u32,
    pub charms: u32,
}

/// The founder: he follows you from room to room and fights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Founder {
    pub x: i32,
    pub y: i32,
    pub step_in: f64,
    /// Seconds into his blow, while he winds it up.
    pub wind: Option<f64>,
}

/// A stone about to fall where dust trickles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallingStone {
    pub x: i32,
    pub y: i32,
    pub fall_in: f64,
}

/// Something that happened, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// The practice house's last door: the stairs stay barred.
    StairsBarred,
    /// A new floor begins.
    FloorBegins {
        number: u32,
        name: &'static str,
        rooms: usize,
        haunt: f64,
    },
    /// You came into a room (index, of how many).
    EnterRoom {
        index: usize,
        of: usize,
    },
    Step,
    /// Walked into a wall.
    Bump(Side),
    /// Walked into a hole where a board gave way.
    Hole(Side),
    /// A rotten board groans next to you: where, in cells.
    Groan {
        dx: f32,
        dy: f32,
    },
    /// The exit is still locked or sealed.
    DoorLocked {
        sealed: bool,
    },
    /// The door you came in by will not open.
    DoorShut,
    /// You went through the open exit.
    DoorOpened,
    /// You stand where something lies.
    OnItem {
        item: Item,
        spent: bool,
    },
    PickedUp(Item),
    /// One of a séance's things taken: how many so far, of how many.
    SeanceStep {
        next: Item,
        done: usize,
        of: usize,
    },
    /// H when this floor allows no hints.
    NoHints,
    /// Touched a thing that is not the answer.
    WrongItem(Item),
    /// Touched a mimic: it shrieks and hops away.
    Mimic(Item),
    /// The room's puzzle is solved and its door unlocked.
    Solved {
        points: u64,
        door: Side,
    },
    NothingHere,
    /// A thing already left behind in a solved room.
    Leftover(Item),
    /// Standing at the exit: whether it is open.
    AtDoor {
        open: bool,
    },
    /// An offering to the spirit away from its door.
    NotAtDoor(Side),
    OfferAccepted(Item),
    OfferRefused(Item),
    /// Tab with nothing carried.
    EmptyHands,
    /// Chose a carried thing: which, its place, and how many.
    Selected {
        item: Item,
        index: usize,
        of: usize,
    },
    Hint(String),
    /// A thing in the room sounds: where it is relative to you, in cells
    /// (x east, y north).
    Emit {
        item: Item,
        dx: f32,
        dy: f32,
    },
    /// The cold draught from an open exit.
    Draft {
        dx: f32,
        dy: f32,
    },
    /// The spirit sealing the exit whispers.
    Spirit {
        dx: f32,
        dy: f32,
    },
    GhostComing(Ghost),
    GhostNear(Ghost),
    GhostPassed {
        kind: GhostKind,
        points: u64,
    },
    /// A charm calls out softly from where it lies.
    CharmCall {
        dx: f32,
        dy: f32,
    },
    /// You walked onto a charm and took it.
    Charm(CharmKind),
    /// Salt kept this from catching you.
    Warded(Threat),
    /// The pendulum's blade swept into a new column.
    Swing {
        dx: f32,
        dy: f32,
    },
    /// A step into deep water.
    Splash,
    /// Still wading: the step did not happen yet.
    Wading,
    /// Deep water next to you laps.
    Lap {
        dx: f32,
        dy: f32,
    },
    /// Dust trickles over where you stand: a stone is coming.
    Dust,
    /// The stone fell, and missed you: where it landed.
    StoneFell {
        dx: f32,
        dy: f32,
    },
    /// The founder comes through the door behind you.
    FounderEnters {
        dx: f32,
        dy: f32,
    },
    FounderStep {
        dx: f32,
        dy: f32,
    },
    /// He winds up a blow, beside you.
    WindUp {
        dx: f32,
        dy: f32,
    },
    /// Halfway through his wind-up: a blow now lands.
    Opening,
    /// You struck too early, and he blocked.
    Parried,
    /// You struck him: hits he can still take.
    Hit {
        left: u32,
    },
    /// His blow missed because you stepped away.
    FounderMissed,
    /// He crumbles, for the rest of the floor.
    Banished {
        points: u64,
    },
    /// He stands in the way.
    FounderInWay(Side),
    /// The house shifted: the exit door is now in this wall.
    Shift(Side),
    /// Caught by a ghost or the shadow, or fell through a board: courage
    /// left.
    Caught {
        by: Threat,
        courage: u32,
    },
    /// A landing: courage given back in a run that carries it.
    Landing {
        courage: u32,
    },
    Warning(f64),
    FloorCleared {
        number: u32,
        time: f64,
        bonus: u64,
    },
    Ended(Outcome),
}

/// One haunt in progress.
#[derive(Clone, Debug)]
pub struct Run {
    pub mode: Mode,
    pub seed: u64,
    pub curses: Curses,
    pub spec: FloorSpec,
    pub rooms: Vec<Room>,
    pub room: usize,
    pub x: i32,
    pub y: i32,
    pub inventory: Vec<Item>,
    pub selected: usize,
    pub haunt_left: f64,
    pub courage: u32,
    pub score: u64,
    pub floor_time: f64,
    /// Seconds over every floor cleared so far.
    pub total_time: f64,
    /// Each cleared floor's time, in order.
    pub floor_times: Vec<f64>,
    /// The highest floor cleared this run, 0 for none.
    pub deepest: u32,
    pub ghost: Option<Ghost>,
    pub ghost_in: f64,
    pub ghosts_evaded: u32,
    pub times_caught: u32,
    pub hints: u32,
    pub outcome: Option<Outcome>,
    /// Set when a floor is cleared and the next is waiting.
    pub next_floor_waiting: bool,
    /// What happened on this floor so far.
    pub stats: FloorStats,
    /// Seconds since your last step or bump: the shadow hunts while it is
    /// short.
    pub since_step: f64,
    /// Seconds you have stood still while the hound passes.
    pub still: f64,
    /// Seconds since the ghost came near and the stab sounded.
    pub since_stab: f64,
    timers: Vec<f64>,
    hops: Vec<f64>,
    beacon_in: f64,
    warned: usize,
    rng: Rng,
    /// The tutorial's house: nothing costs courage and the clock stands
    /// still.
    pub practice: bool,
    /// Salt carried: each pinch wards off one catch. Kept for the run.
    pub salt: u32,
    /// Seconds still wading.
    pub wade: f64,
    charm_in: f64,
    pub stone: Option<FallingStone>,
    pub stone_in: f64,
    pub founder: Option<Founder>,
    /// Seconds until the founder follows you into this room.
    pub founder_in: f64,
    /// Hits he can still take on this floor; 0 once he crumbles.
    pub founder_health: u32,
    pub shift_in: f64,
}

impl Run {
    pub fn new(mode: Mode, seed: u64) -> Run {
        Run::cursed(mode, seed, Curses::default())
    }

    /// The tutorial's house: these rooms, in practice. Nothing costs
    /// courage, the clock stands still, and no ghost comes unless sent.
    pub fn practice(rooms: Vec<Room>) -> Run {
        let mut run = Run::new(Mode::Floor(1), 31);
        run.rooms = rooms;
        run.spec.rooms = run.rooms.len();
        run.spec.ghost_gap = (1e9, 1e9);
        run.practice = true;
        run.room = 0;
        run.place_at_entry();
        run.reset_room_clocks();
        run
    }

    /// A run with curses on: twists by choice, for more score.
    pub fn cursed(mode: Mode, seed: u64, curses: Curses) -> Run {
        // Challenges are the same for everyone: curses never touch them.
        let curses = if mode.event().is_some() {
            Curses::default()
        } else {
            curses
        };
        let spec = Run::spec_for(mode, curses, mode.first_floor());
        let mut run = Run {
            mode,
            seed,
            curses,
            spec,
            rooms: Vec::new(),
            room: 0,
            x: 0,
            y: 0,
            inventory: Vec::new(),
            selected: 0,
            haunt_left: 0.0,
            courage: COURAGE,
            score: 0,
            floor_time: 0.0,
            total_time: 0.0,
            floor_times: Vec::new(),
            deepest: 0,
            ghost: None,
            ghost_in: 0.0,
            ghosts_evaded: 0,
            times_caught: 0,
            hints: 0,
            outcome: None,
            next_floor_waiting: false,
            stats: FloorStats::default(),
            since_step: 0.0,
            still: 0.0,
            since_stab: 0.0,
            timers: Vec::new(),
            hops: Vec::new(),
            beacon_in: 0.0,
            warned: 0,
            rng: Rng::seeded(seed ^ 0x6057_6057),
            practice: false,
            salt: 0,
            wade: 0.0,
            charm_in: 0.0,
            stone: None,
            stone_in: 0.0,
            founder: None,
            founder_in: 0.0,
            founder_health: 0,
            shift_in: 0.0,
        };
        run.load_floor(mode.first_floor());
        run
    }

    fn spec_for(mode: Mode, curses: Curses, floor: u32) -> FloorSpec {
        let mut spec = mode.spec(floor);
        curses.apply(&mut spec);
        spec
    }

    /// The layout a floor of this run will have.
    pub fn plan(mode: Mode, seed: u64, floor: u32) -> Vec<Room> {
        Run::plan_cursed(mode, seed, floor, Curses::default())
    }

    pub fn plan_cursed(mode: Mode, seed: u64, floor: u32, curses: Curses) -> Vec<Room> {
        let spec = Run::spec_for(mode, curses, floor);
        plan_floor(
            &spec,
            &mut Rng::seeded(seed.wrapping_add(floor as u64 * 7919)),
        )
    }

    fn load_floor(&mut self, floor: u32) {
        self.spec = Run::spec_for(self.mode, self.curses, floor);
        self.rooms = Run::plan_cursed(self.mode, self.seed, floor, self.curses);
        self.room = 0;
        self.inventory.clear();
        self.selected = 0;
        self.haunt_left = self.spec.haunt;
        let carry = self.mode.carries_courage() && floor != self.mode.first_floor();
        if !carry {
            self.courage = COURAGE;
        }
        self.floor_time = 0.0;
        self.warned = 0;
        self.ghost = None;
        self.next_floor_waiting = false;
        self.stats = FloorStats::default();
        self.since_step = 0.0;
        self.founder_health = self.spec.founder;
        self.place_at_entry();
        self.reset_room_clocks();
    }

    /// What the screen speaks as a floor starts.
    pub fn floor_begins(&mut self) -> Vec<Ev> {
        let mut out = vec![
            Ev::FloorBegins {
                number: self.spec.number,
                name: self.spec.name,
                rooms: self.rooms.len(),
                haunt: self.spec.haunt,
            },
            Ev::EnterRoom {
                index: 0,
                of: self.rooms.len(),
            },
        ];
        out.extend(self.groans());
        out
    }

    /// Climbs to the waiting floor.
    pub fn next_floor(&mut self) -> Vec<Ev> {
        if !self.next_floor_waiting {
            return Vec::new();
        }
        let floor = self.spec.number + 1;
        self.load_floor(floor);
        let mut out = Vec::new();
        if let Some(every) = self.mode.landing_every() {
            if (floor - 1).is_multiple_of(every) && self.courage < COURAGE {
                self.courage += 1;
                out.push(Ev::Landing {
                    courage: self.courage,
                });
            }
        }
        let mut begins = self.floor_begins();
        begins.extend(out);
        begins
    }

    fn place_at_entry(&mut self) {
        let room = &self.rooms[self.room];
        let (x, y) = room.inside(room.entry);
        self.x = x;
        self.y = y;
    }

    fn ghosts_here(&self) -> bool {
        !(self.spec.number == 1 && self.room == 0 && !self.mode.nightmare())
    }

    fn reset_room_clocks(&mut self) {
        let n = self.rooms[self.room].items.len();
        self.timers = (0..n).map(|i| 0.4 + i as f64 * 0.45).collect();
        self.hops = (0..n).map(|_| 5.0 + self.rng.unit() * 3.0).collect();
        self.beacon_in = 0.8;
        let (lo, hi) = self.spec.ghost_gap;
        self.ghost_in = lo + self.rng.unit() * (hi - lo);
        let room = &self.rooms[self.room];
        self.stats.rotten |= !room.rotten.is_empty();
        self.stats.shadow |= room.shadow.is_some();
        self.stats.mimic |= room.mimics() > 0;
        self.stats.mirror |= room.mirror;
        self.stats.dark |= room.dark;
        self.stats.spirit |= matches!(room.puzzle, Puzzle::Offering(_));
        self.stats.seance |= matches!(room.puzzle, Puzzle::Seance(_));
        self.stats.pendulum |= room.pendulum.is_some();
        self.stats.water |= !room.water.is_empty();
        self.stats.stones |= room.stones;
        let (stones, shifts) = (room.stones, room.shifts);
        self.wade = 0.0;
        self.charm_in = 1.5;
        self.stone = None;
        if stones {
            let (lo, hi) = self.spec.stone_gap;
            self.stone_in = lo + self.rng.unit() * (hi - lo);
        }
        if shifts {
            let (lo, hi) = self.spec.shift_gap;
            self.shift_in = lo + self.rng.unit() * (hi - lo);
        }
        self.founder = None;
        self.founder_in = if self.founder_active() && self.room > 0 {
            FOUNDER_DELAY
        } else {
            0.0
        };
    }

    /// The founder walks this floor and has not crumbled.
    pub fn founder_active(&self) -> bool {
        self.spec.founder > 0 && self.founder_health > 0
    }

    /// Whether the founder stands next to you, or on you.
    pub fn founder_beside(&self) -> bool {
        self.founder
            .is_some_and(|f| (f.x - self.x).abs() + (f.y - self.y).abs() <= 1)
    }

    pub fn current(&self) -> &Room {
        &self.rooms[self.room]
    }

    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }

    fn playing(&self) -> bool {
        self.outcome.is_none() && !self.next_floor_waiting
    }

    /// The rotten boards next to where you stand, groaning.
    fn groans(&self) -> Vec<Ev> {
        self.current()
            .rotten_near(self.x, self.y)
            .into_iter()
            .map(|(dx, dy)| Ev::Groan {
                dx: dx as f32,
                dy: dy as f32,
            })
            .collect()
    }

    /// One step, or a bump, a door, a fall, or being caught.
    pub fn walk(&mut self, side: Side) -> Vec<Ev> {
        if !self.playing() {
            return Vec::new();
        }
        self.since_step = 0.0;
        self.still = 0.0;
        if let Some(g) = self.ghost {
            if g.phase == GhostPhase::Near
                && !g.kind.hunts_stillness()
                && self.since_stab >= STAB_GRACE
            {
                return self.caught(Threat::Ghost(g.kind));
            }
        }
        if self.wade > 0.0 {
            return vec![Ev::Wading];
        }
        let (dx, dy) = side.delta();
        let (nx, ny) = (self.x + dx, self.y + dy);
        let room = self.current();
        if room.in_grid(nx, ny) {
            if room.holes.contains(&(nx, ny)) {
                return vec![Ev::Hole(side)];
            }
            if self.founder.is_some_and(|f| (f.x, f.y) == (nx, ny)) {
                return vec![Ev::FounderInWay(side)];
            }
            if room.rotten.contains(&(nx, ny)) {
                let room = &mut self.rooms[self.room];
                room.rotten.retain(|&c| c != (nx, ny));
                room.holes.push((nx, ny));
                return self.caught(Threat::Board);
            }
            let wet = room.water.contains(&(nx, ny));
            self.x = nx;
            self.y = ny;
            let mut out = vec![if wet { Ev::Splash } else { Ev::Step }];
            if self.shadow_here() {
                return self.caught(Threat::Shadow);
            }
            if self.blade_here() {
                return self.caught(Threat::Blade);
            }
            if wet {
                self.wade = WADE_TIME;
                if self.ghost.is_none() {
                    self.ghost_in -= SPLASH_CALL;
                }
            }
            if let Some(c) = self.current().charm_at(nx, ny) {
                out.extend(self.take_charm(c));
            }
            let room = self.current();
            if let Some(i) = room.item_at(nx, ny) {
                out.push(Ev::OnItem {
                    item: room.items[i].item,
                    spent: room.solved,
                });
            }
            out.extend(self.groans());
            if !wet {
                out.extend(self.laps());
            }
            return out;
        }
        // Leaving the grid: a door, or a wall.
        let here = (self.x, self.y);
        if side == room.exit.side && room.inside(room.exit) == here {
            // The practice house has no stairs: the lesson ends on the
            // landing, never in a cleared floor's results.
            if room.solved && self.practice && self.room + 1 == self.rooms.len() {
                return vec![Ev::StairsBarred];
            }
            if room.solved {
                return self.go_through();
            }
            return vec![Ev::DoorLocked {
                sealed: matches!(room.puzzle, Puzzle::Offering(_)),
            }];
        }
        if side == room.entry.side && room.inside(room.entry) == here {
            return vec![Ev::DoorShut];
        }
        vec![Ev::Bump(side)]
    }

    fn blade_here(&self) -> bool {
        self.current()
            .pendulum
            .is_some_and(|p| (p.x, p.row) == (self.x, self.y))
    }

    /// Deep water next to you, lapping.
    fn laps(&self) -> Vec<Ev> {
        self.current()
            .water_near(self.x, self.y)
            .into_iter()
            .map(|(dx, dy)| Ev::Lap {
                dx: dx as f32,
                dy: dy as f32,
            })
            .collect()
    }

    fn take_charm(&mut self, c: Charm) -> Vec<Ev> {
        if let Some(ch) = self.rooms[self.room].charm.as_mut() {
            ch.taken = true;
        }
        self.stats.charms += 1;
        match c.kind {
            CharmKind::Salt => self.salt = (self.salt + 1).min(SALT_MAX),
            CharmKind::Stub => self.haunt_left += STUB_SECONDS,
        }
        vec![Ev::Charm(c.kind)]
    }

    fn shadow_here(&self) -> bool {
        self.current()
            .shadow
            .is_some_and(|s| (s.x, s.y) == (self.x, self.y))
    }

    fn go_through(&mut self) -> Vec<Ev> {
        let mut out = vec![Ev::DoorOpened];
        if self.room + 1 < self.rooms.len() {
            self.room += 1;
            self.ghost = None;
            self.place_at_entry();
            self.reset_room_clocks();
            out.push(Ev::EnterRoom {
                index: self.room,
                of: self.rooms.len(),
            });
            out.extend(self.groans());
            return out;
        }
        // The stairs: the floor is cleared.
        self.ghost = None;
        let bonus = self
            .curses
            .bonus((self.haunt_left.max(0.0) as u64) * 5 + self.courage as u64 * 100);
        self.score += bonus;
        self.total_time += self.floor_time;
        self.floor_times.push(self.floor_time);
        self.deepest = self.deepest.max(self.spec.number);
        out.push(Ev::FloorCleared {
            number: self.spec.number,
            time: self.floor_time,
            bonus,
        });
        if self.spec.number >= self.mode.last_floor() {
            self.outcome = Some(Outcome::Escaped);
            out.push(Ev::Ended(Outcome::Escaped));
        } else {
            self.next_floor_waiting = true;
        }
        out
    }

    fn caught(&mut self, by: Threat) -> Vec<Ev> {
        if by.warded_by_salt() && self.salt > 0 {
            self.salt -= 1;
            match by {
                Threat::Ghost(_) => {
                    self.ghost = None;
                    let (lo, hi) = self.spec.ghost_gap;
                    self.ghost_in = lo + self.rng.unit() * (hi - lo);
                }
                Threat::Shadow => {
                    if let Some(s) = self.rooms[self.room].shadow.as_mut() {
                        (s.x, s.y) = s.home;
                        s.step_in = s.interval * 2.0;
                    }
                }
                _ => self.knock_back(),
            }
            return vec![Ev::Warded(by)];
        }
        self.ghost = None;
        self.times_caught += 1;
        self.stats.caught += 1;
        match by {
            Threat::Board => self.stats.falls += 1,
            Threat::Shadow => self.stats.shadow_catches += 1,
            Threat::Founder => self.stats.founder_hits += 1,
            _ => {}
        }
        self.wade = 0.0;
        self.stone = None;
        if self.founder.take().is_some() {
            self.founder_in = FOUNDER_DELAY;
        }
        if !self.practice {
            self.courage = self.courage.saturating_sub(1);
        }
        let mut out = vec![Ev::Caught {
            by,
            courage: self.courage,
        }];
        if self.courage == 0 {
            self.outcome = Some(Outcome::ChasedOut);
            out.push(Ev::Ended(Outcome::ChasedOut));
        } else {
            self.place_at_entry();
            if let Some(s) = self.rooms[self.room].shadow.as_mut() {
                (s.x, s.y) = s.home;
                s.step_in = s.interval * 2.0;
            }
            let (lo, hi) = self.spec.ghost_gap;
            self.ghost_in = lo + self.rng.unit() * (hi - lo);
            out.extend(self.groans());
        }
        out
    }

    fn points(&self) -> u64 {
        self.curses.bonus(100 * self.spec.level as u64)
    }

    /// The main action: pick up, offer, or try the door.
    pub fn interact(&mut self) -> Vec<Ev> {
        if !self.playing() {
            return Vec::new();
        }
        if self.founder_beside() {
            return self.strike();
        }
        let points = self.points();
        let (x, y) = (self.x, self.y);
        let room_index = self.room;
        let room = &self.rooms[room_index];
        let at_exit = room.inside(room.exit) == (x, y);
        if let Puzzle::Offering(wanted) = room.puzzle {
            if room.solved {
                return vec![if at_exit {
                    Ev::AtDoor { open: true }
                } else {
                    Ev::NothingHere
                }];
            }
            if !at_exit {
                return vec![Ev::NotAtDoor(room.exit.side)];
            }
            let Some(&offered) = self.inventory.get(self.selected) else {
                return vec![Ev::EmptyHands];
            };
            if offered == wanted {
                self.inventory.remove(self.selected);
                self.selected = 0;
                let door = room.exit.side;
                self.rooms[room_index].solved = true;
                self.score += points;
                self.beacon_in = 0.6;
                return vec![Ev::OfferAccepted(offered), Ev::Solved { points, door }];
            }
            self.haunt_left -= self.spec.penalty;
            return vec![Ev::OfferRefused(offered)];
        }
        if let Some(i) = room.item_at(x, y) {
            let placed = room.items[i];
            let item = placed.item;
            if room.solved {
                return vec![Ev::Leftover(item)];
            }
            if placed.mimic {
                self.haunt_left -= self.spec.penalty;
                self.hop(i);
                return vec![Ev::Mimic(item)];
            }
            if Some(item) == room.next_wanted() {
                let door = room.exit.side;
                let wanted = room.puzzle.wanted();
                let room = &mut self.rooms[room_index];
                room.items[i].taken = true;
                room.stage += 1;
                self.inventory.push(item);
                self.selected = self.inventory.len() - 1;
                if let Some(&next) = wanted.get(room.stage) {
                    return vec![
                        Ev::PickedUp(item),
                        Ev::SeanceStep {
                            next,
                            done: room.stage,
                            of: wanted.len(),
                        },
                    ];
                }
                room.solved = true;
                // The mimics fall silent with the room.
                for p in room.items.iter_mut().filter(|p| p.mimic) {
                    p.taken = true;
                }
                self.score += points;
                self.beacon_in = 0.6;
                return vec![Ev::PickedUp(item), Ev::Solved { points, door }];
            }
            self.haunt_left -= self.spec.penalty;
            return vec![Ev::WrongItem(item)];
        }
        if at_exit {
            return vec![Ev::AtDoor { open: room.solved }];
        }
        vec![Ev::NothingHere]
    }

    /// The main action with the founder beside you: a blow with the
    /// candlestick. It lands only in the second half of his wind-up.
    fn strike(&mut self) -> Vec<Ev> {
        let Some(f) = self.founder else {
            return Vec::new();
        };
        let late = f.wind.is_some_and(|w| w >= self.spec.founder_wind / 2.0);
        if !late {
            return vec![Ev::Parried];
        }
        self.founder_health = self.founder_health.saturating_sub(1);
        if self.founder_health == 0 {
            self.founder = None;
            self.founder_in = 0.0;
            self.stats.banished = true;
            let points = self.curses.bonus(200 * self.spec.level as u64);
            self.score += points;
            return vec![Ev::Banished { points }];
        }
        self.knock_back();
        vec![Ev::Hit {
            left: self.founder_health,
        }]
    }

    /// The founder staggers back from you, his blow undone.
    fn knock_back(&mut self) {
        let (px, py) = (self.x, self.y);
        let room = self.rooms[self.room].clone();
        let Some(f) = self.founder.as_mut() else {
            return;
        };
        let (mut dx, mut dy) = ((f.x - px).signum(), (f.y - py).signum());
        if (dx, dy) == (0, 0) {
            (dx, dy) = if py + 1 < room.h { (0, 1) } else { (0, -1) };
        }
        if dx != 0 && dy != 0 {
            dy = 0;
        }
        for _ in 0..KNOCKBACK {
            let (nx, ny) = (f.x + dx, f.y + dy);
            if !room.in_grid(nx, ny) || room.holes.contains(&(nx, ny)) {
                break;
            }
            f.x = nx;
            f.y = ny;
        }
        f.wind = None;
        f.step_in = self.spec.founder_pace * 2.0;
    }

    /// A mimic hops to another free cell, away from you.
    fn hop(&mut self, i: usize) {
        let (px, py) = (self.x, self.y);
        let room = &self.rooms[self.room];
        let cells: Vec<(i32, i32)> = (0..room.w)
            .flat_map(|x| (0..room.h).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                room.free(x, y)
                    && (x, y) != (px, py)
                    && room.shadow.is_none_or(|s| (s.x, s.y) != (x, y))
            })
            .collect();
        if cells.is_empty() {
            return;
        }
        let (x, y) = *self.rng.pick(&cells);
        let p = &mut self.rooms[self.room].items[i];
        p.x = x;
        p.y = y;
        self.hops[i] = 5.0 + self.rng.unit() * 3.0;
    }

    /// The second action: the next thing you carry.
    pub fn cycle(&mut self) -> Vec<Ev> {
        if self.inventory.is_empty() {
            return vec![Ev::EmptyHands];
        }
        self.selected = (self.selected + 1) % self.inventory.len();
        vec![Ev::Selected {
            item: self.inventory[self.selected],
            index: self.selected,
            of: self.inventory.len(),
        }]
    }

    /// The thing carried that an offering would give.
    pub fn held(&self) -> Option<Item> {
        self.inventory.get(self.selected).copied()
    }

    /// Where the room wants you to go next, as a cell.
    pub fn goal(&self) -> (i32, i32) {
        let room = self.current();
        match room.target() {
            Some(p) => (p.x, p.y),
            None => room.inside(room.exit),
        }
    }

    /// The way to the exit door, its wall's direction last: "1 step east
    /// and 2 steps north" to a north door, "2 steps east, then north" when
    /// only the wall's way is left, or "north of you" from beside it.
    pub fn exit_words(&self) -> String {
        let room = self.current();
        let (ex, ey) = room.inside(room.exit);
        let (dx, dy) = (ex - self.x, ey - self.y);
        let side = room.exit.side;
        let (across, toward) = match side {
            Side::North | Side::South => (directions(dx, 0), dy),
            Side::East | Side::West => (directions(0, dy), dx),
        };
        let toward = match side {
            Side::North | Side::South => directions(0, toward),
            Side::East | Side::West => directions(toward, 0),
        };
        match (across.as_str(), toward.as_str()) {
            ("right here", "right here") => format!("{} of you", side.name()),
            ("right here", way) => way.to_string(),
            (way, "right here") => format!("{way}, then {}", side.name()),
            (way, last) => format!("{way} and {last}"),
        }
    }

    /// A hint, which costs haunt time.
    pub fn hint(&mut self) -> Vec<Ev> {
        if !self.playing() {
            return Vec::new();
        }
        if !self.spec.hints {
            return vec![Ev::NoHints];
        }
        self.hints += 1;
        self.stats.hints += 1;
        self.haunt_left -= self.spec.hint_cost;
        let room = self.current();
        let (gx, gy) = self.goal();
        let way = directions(gx - self.x, gy - self.y);
        let text = match (room.solved, room.puzzle, room.target()) {
            (false, Puzzle::Offering(item), _) => {
                format!("The spirit wants the {}. Its door is {way}.", item.name())
            }
            (false, _, Some(p)) => format!("The {} is {way}.", p.item.name()),
            _ => format!("The open door is {}.", self.exit_words()),
        };
        vec![Ev::Hint(text)]
    }

    /// Time passes: sounds come round, the ghost and the shadow move, the
    /// mimics hop, the clock runs.
    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        let mut out = Vec::new();
        if !self.playing() || dt <= 0.0 {
            return out;
        }
        self.floor_time += dt;
        if self.practice {
            self.haunt_left = self.haunt_left.max(60.0);
        } else {
            self.haunt_left -= dt;
        }
        self.since_step += dt;
        self.wade = (self.wade - dt).max(0.0);
        self.still += dt;
        // One toll for however many marks were crossed (a penalty can jump
        // several), naming the time truly left.
        let mut crossed = None;
        while self.warned < WARNINGS.len() && self.haunt_left <= WARNINGS[self.warned] {
            crossed = Some(WARNINGS[self.warned]);
            self.warned += 1;
        }
        if let Some(mark) = crossed {
            if self.haunt_left > 0.0 {
                out.push(Ev::Warning(mark.min(self.haunt_left.ceil())));
            }
        }
        if self.haunt_left <= 0.0 {
            self.haunt_left = 0.0;
            self.ghost = None;
            self.outcome = Some(Outcome::TimeUp);
            out.push(Ev::Ended(Outcome::TimeUp));
            return out;
        }

        // Mimics hop now and then; the real thing keeps still.
        if !self.current().solved {
            for i in 0..self.current().items.len() {
                let p = self.current().items[i];
                if !p.mimic || p.taken {
                    continue;
                }
                self.hops[i] -= dt;
                if self.hops[i] <= 0.0 {
                    self.hop(i);
                }
            }
        }

        // The things in the room call out, faster the nearer you are.
        let room = &self.rooms[self.room];
        if !room.solved {
            for (i, p) in room.items.iter().enumerate() {
                if p.taken {
                    continue;
                }
                self.timers[i] -= dt;
                if self.timers[i] <= 0.0 {
                    let dx = (p.x - self.x) as f32;
                    let dy = (p.y - self.y) as f32;
                    let d = dx.hypot(dy);
                    self.timers[i] = if self.spec.lights_out {
                        1.8 + 0.45 * d as f64
                    } else {
                        1.0 + 0.3 * d as f64
                    };
                    if room.dark && d > DARK_REACH {
                        continue;
                    }
                    out.push(Ev::Emit {
                        item: p.item,
                        dx,
                        dy,
                    });
                }
            }
        }
        let offering = matches!(room.puzzle, Puzzle::Offering(_));
        if room.solved || offering {
            self.beacon_in -= dt;
            if self.beacon_in <= 0.0 {
                let (px, py) = room.door_point(room.exit);
                let dx = px - self.x as f32;
                let dy = py - self.y as f32;
                if room.solved {
                    self.beacon_in = 2.0;
                    out.push(Ev::Draft { dx, dy });
                } else {
                    self.beacon_in = 2.5;
                    out.push(Ev::Spirit { dx, dy });
                }
            }
        }

        // A charm calls out softly, untouched by the puzzle.
        let dark = room.dark;
        if let Some(c) = room.charm.filter(|c| !c.taken) {
            self.charm_in -= dt;
            if self.charm_in <= 0.0 {
                let dx = (c.x - self.x) as f32;
                let dy = (c.y - self.y) as f32;
                let d = dx.hypot(dy);
                self.charm_in = 2.5 + 0.3 * d as f64;
                if !(dark && d > DARK_REACH) {
                    out.push(Ev::CharmCall { dx, dy });
                }
            }
        }

        out.extend(self.deep_update(dt));
        if !self.playing() || out.iter().any(|e| matches!(e, Ev::Caught { .. })) {
            return out;
        }

        // The shadow walks, toward your footsteps while it hears them.
        if let Some(mut sh) = self.rooms[self.room].shadow {
            sh.step_in -= dt;
            if sh.step_in <= 0.0 {
                sh.step_in = sh.interval;
                let (px, py) = (self.x, self.y);
                let room = &self.rooms[self.room];
                let open = |x: i32, y: i32| room.in_grid(x, y) && !room.holes.contains(&(x, y));
                let mut moves: Vec<(i32, i32)> = Side::ALL
                    .iter()
                    .map(|s| s.delta())
                    .filter(|&(dx, dy)| open(sh.x + dx, sh.y + dy))
                    .collect();
                if self.since_step < SHADOW_HEARS {
                    let d0 = (px - sh.x).abs() + (py - sh.y).abs();
                    moves.retain(|&(dx, dy)| (px - sh.x - dx).abs() + (py - sh.y - dy).abs() < d0);
                }
                if !moves.is_empty() {
                    let (dx, dy) = *self.rng.pick(&moves);
                    sh.x += dx;
                    sh.y += dy;
                }
                self.rooms[self.room].shadow = Some(sh);
                if (sh.x, sh.y) == (px, py) {
                    out.extend(self.caught(Threat::Shadow));
                    return out;
                }
            } else {
                self.rooms[self.room].shadow = Some(sh);
            }
        }

        // The ghost.
        if let Some(mut g) = self.ghost {
            g.s -= g.speed * dt as f32;
            let was = g.phase;
            g.phase = if g.s > GHOST_NEAR {
                GhostPhase::Approaching
            } else if g.s >= -GHOST_NEAR {
                GhostPhase::Near
            } else {
                GhostPhase::Leaving
            };
            if was != GhostPhase::Near && g.phase == GhostPhase::Near {
                // The hound gives you a moment to start walking.
                self.still = 0.0;
                self.since_stab = 0.0;
            } else {
                self.since_stab += dt;
            }
            if g.phase == GhostPhase::Near && g.kind.hunts_stillness() && self.still >= HOUND_STILL
            {
                out.extend(self.caught(Threat::Ghost(g.kind)));
                return out;
            }
            // Once it is past you it is safe again: it breathes out and
            // drifts off, and the next ghost's wait begins.
            if g.phase == GhostPhase::Leaving {
                self.ghost = None;
                self.ghosts_evaded += 1;
                self.stats.ghosts.push(g.kind);
                let points = self.curses.bonus(25 * self.spec.level as u64);
                self.score += points;
                let (lo, hi) = self.spec.ghost_gap;
                self.ghost_in = lo + self.rng.unit() * (hi - lo);
                out.push(Ev::GhostPassed {
                    kind: g.kind,
                    points,
                });
            } else {
                self.ghost = Some(g);
                if was != GhostPhase::Near && g.phase == GhostPhase::Near {
                    out.push(Ev::GhostNear(g));
                }
            }
        } else if self.ghosts_here() && self.stone.is_none() {
            self.ghost_in -= dt;
            if self.ghost_in <= 0.0 {
                let g = self.summon(None, None);
                out.push(Ev::GhostComing(g));
            }
        }
        out
    }

    /// The pendulum, falling stones, the founder and the shifting door.
    fn deep_update(&mut self, dt: f64) -> Vec<Ev> {
        let mut out = Vec::new();
        let (px, py) = (self.x, self.y);
        // The blade swings one column at a time, back and forth.
        if let Some(mut p) = self.rooms[self.room].pendulum {
            p.step_in -= dt;
            if p.step_in <= 0.0 {
                p.step_in = p.interval;
                let w = self.current().w;
                if p.x + p.dir < 0 || p.x + p.dir >= w {
                    p.dir = -p.dir;
                }
                p.x += p.dir;
                out.push(Ev::Swing {
                    dx: (p.x - px) as f32,
                    dy: (p.row - py) as f32,
                });
            }
            self.rooms[self.room].pendulum = Some(p);
            if self.blade_here() {
                out.extend(self.caught(Threat::Blade));
                return out;
            }
        }

        // Dust, then a stone where the dust fell. Never while a ghost is
        // close, so moving and keeping still never both are asked of you.
        if self.current().stones {
            if let Some(mut st) = self.stone {
                st.fall_in -= dt;
                if st.fall_in <= 0.0 {
                    self.stone = None;
                    let (lo, hi) = self.spec.stone_gap;
                    self.stone_in = lo + self.rng.unit() * (hi - lo);
                    if (st.x, st.y) == (self.x, self.y) {
                        out.extend(self.caught(Threat::Stone));
                        return out;
                    }
                    out.push(Ev::StoneFell {
                        dx: (st.x - px) as f32,
                        dy: (st.y - py) as f32,
                    });
                } else {
                    self.stone = Some(st);
                }
            } else {
                self.stone_in -= dt;
                if self.stone_in <= 0.0 {
                    let clear = self.ghost.is_none_or(|g| {
                        g.phase == GhostPhase::Leaving
                            || g.s > GHOST_NEAR + g.speed * (STONE_DELAY as f32 + 0.5)
                    });
                    if clear {
                        self.stone = Some(FallingStone {
                            x: px,
                            y: py,
                            fall_in: STONE_DELAY,
                        });
                        out.push(Ev::Dust);
                    } else {
                        self.stone_in = 1.0;
                    }
                }
            }
        }

        // The founder: through the door behind you, then after you.
        if self.founder_active() {
            match self.founder {
                None if self.founder_in > 0.0 => {
                    let room = self.current();
                    let (x, y) = room.inside(room.entry);
                    // He waits behind the door while you stand at it.
                    let clear = (x - px).abs() + (y - py).abs() >= 2;
                    self.founder_in = (self.founder_in - dt).max(if clear { f64::MIN } else { dt });
                    if self.founder_in <= 0.0 {
                        self.founder = Some(Founder {
                            x,
                            y,
                            step_in: self.spec.founder_pace,
                            wind: None,
                        });
                        self.stats.founder = true;
                        out.push(Ev::FounderEnters {
                            dx: (x - px) as f32,
                            dy: (y - py) as f32,
                        });
                    }
                }
                Some(mut f) => {
                    let beside = (f.x - px).abs() + (f.y - py).abs() <= 1;
                    match f.wind {
                        Some(w) => {
                            let w = w + dt;
                            if w >= self.spec.founder_wind {
                                f.wind = None;
                                f.step_in = self.spec.founder_pace;
                                self.founder = Some(f);
                                if beside {
                                    out.extend(self.caught(Threat::Founder));
                                    return out;
                                }
                                out.push(Ev::FounderMissed);
                            } else {
                                let half = self.spec.founder_wind / 2.0;
                                if w >= half && w - dt < half {
                                    out.push(Ev::Opening);
                                }
                                f.wind = Some(w);
                                self.founder = Some(f);
                            }
                        }
                        None if beside => {
                            f.wind = Some(0.0);
                            self.founder = Some(f);
                            out.push(Ev::WindUp {
                                dx: (f.x - px) as f32,
                                dy: (f.y - py) as f32,
                            });
                        }
                        None => {
                            f.step_in -= dt;
                            if f.step_in <= 0.0 {
                                f.step_in = self.spec.founder_pace;
                                let room = self.current();
                                let d0 = (px - f.x).abs() + (py - f.y).abs();
                                let best = Side::ALL
                                    .iter()
                                    .map(|s| s.delta())
                                    .map(|(dx, dy)| (f.x + dx, f.y + dy))
                                    .filter(|&(x, y)| {
                                        room.in_grid(x, y) && !room.holes.contains(&(x, y))
                                    })
                                    .filter(|&(x, y)| (px - x).abs() + (py - y).abs() < d0)
                                    .max_by_key(|&(x, y)| {
                                        // Close the longer gap first.
                                        if (px - f.x).abs() >= (py - f.y).abs() {
                                            i32::from(x != f.x)
                                        } else {
                                            i32::from(y != f.y)
                                        }
                                    });
                                if let Some((x, y)) = best {
                                    f.x = x;
                                    f.y = y;
                                    out.push(Ev::FounderStep {
                                        dx: (x - px) as f32,
                                        dy: (y - py) as f32,
                                    });
                                }
                            }
                            self.founder = Some(f);
                        }
                    }
                }
                None => {}
            }
        }

        // The house shifts, and the exit door with it.
        if self.current().shifts {
            self.shift_in -= dt;
            if self.shift_in <= 0.0 {
                let (lo, hi) = self.spec.shift_gap;
                self.shift_in = lo + self.rng.unit() * (hi - lo);
                if let Some(side) = self.shift_exit() {
                    out.push(Ev::Shift(side));
                }
            }
        }
        out
    }

    /// Moves the exit door to another place you can reach.
    fn shift_exit(&mut self) -> Option<Side> {
        let room = self.current();
        let mut blocked = room.rotten.clone();
        blocked.extend(room.holes.iter().copied());
        let open = reachable(room.w, room.h, (self.x, self.y), &blocked);
        let start = room.inside(room.entry);
        let doors: Vec<Door> = Side::ALL
            .iter()
            .filter(|&&s| s != room.entry.side)
            .flat_map(|&side| {
                let len = match side {
                    Side::North | Side::South => room.w,
                    _ => room.h,
                };
                (0..len).map(move |at| Door { side, at })
            })
            .filter(|&d| {
                let c = room.inside(d);
                d != room.exit
                    && c != start
                    && c != (self.x, self.y)
                    && open.contains(&c)
                    && room.item_at(c.0, c.1).is_none()
            })
            .collect();
        if doors.is_empty() {
            return None;
        }
        let door = *self.rng.pick(&doors);
        self.rooms[self.room].exit = door;
        self.stats.shifted = true;
        self.beacon_in = 0.6;
        Some(door.side)
    }

    /// Sends a ghost (tests, and the update loop). `kind` and `bearing`
    /// default to random ones.
    pub fn summon(&mut self, kind: Option<GhostKind>, bearing: Option<f32>) -> Ghost {
        let kind = kind.unwrap_or_else(|| *self.rng.pick(self.spec.ghosts));
        let bearing = bearing.unwrap_or_else(|| self.rng.below(8) as f32 * TAU / 8.0);
        let g = Ghost {
            kind,
            bearing,
            s: GHOST_START,
            speed: kind.speed() * self.spec.ghost_speed,
            phase: GhostPhase::Approaching,
        };
        self.ghost = Some(g);
        g
    }

    /// Where you stand, in words.
    pub fn place_words(&self) -> String {
        let room = self.current();
        format!(
            "column {} of {} from the west, row {} of {} from the south",
            self.x + 1,
            room.w,
            self.y + 1,
            room.h
        )
    }

    /// Where a door is, in words.
    pub fn door_words(&self, door: Door) -> String {
        let room = self.current();
        let along = match door.side {
            Side::North | Side::South => format!("column {} of {}", door.at + 1, room.w),
            Side::East | Side::West => format!("row {} of {}", door.at + 1, room.h),
        };
        format!("the {} wall, at {along}", door.side.name())
    }

    /// The rotten boards next to you, in words: "east and north", or None.
    pub fn groan_words(&self) -> Option<String> {
        let sides: Vec<&str> = self
            .current()
            .rotten_near(self.x, self.y)
            .into_iter()
            .filter_map(|(dx, dy)| Side::of(dx, dy).map(Side::name))
            .collect();
        match sides.as_slice() {
            [] => None,
            [one] => Some(one.to_string()),
            [rest @ .., last] => Some(format!("{} and {last}", rest.join(", "))),
        }
    }

    /// Where the founder is from you, in words, while he is in the room.
    pub fn founder_words(&self) -> Option<String> {
        let f = self.founder?;
        Some(directions(f.x - self.x, f.y - self.y))
    }

    /// Where the pendulum's row is from you, in words.
    pub fn pendulum_words(&self) -> Option<String> {
        let p = self.current().pendulum?;
        let dy = p.row - self.y;
        Some(match dy {
            0 => "in your row".to_string(),
            1 => "1 row north".to_string(),
            -1 => "1 row south".to_string(),
            n if n > 0 => format!("{n} rows north"),
            n => format!("{} rows south", -n),
        })
    }

    /// Where the shadow is from you, in words, if this room has one.
    pub fn shadow_words(&self) -> Option<String> {
        let s = self.current().shadow?;
        Some(directions(s.x - self.x, s.y - self.y))
    }
}

/// "2 steps north and 1 step east", or "right here".
pub fn directions(dx: i32, dy: i32) -> String {
    fn part(n: i32, pos: &str, neg: &str) -> Option<String> {
        match n {
            0 => None,
            1 => Some(format!("1 step {pos}")),
            -1 => Some(format!("1 step {neg}")),
            n if n > 0 => Some(format!("{n} steps {pos}")),
            n => Some(format!("{} steps {neg}", -n)),
        }
    }
    let parts: Vec<String> = [part(dy, "north", "south"), part(dx, "east", "west")]
        .into_iter()
        .flatten()
        .collect();
    if parts.is_empty() {
        "right here".to_string()
    } else {
        parts.join(" and ")
    }
}

/// "2 minutes 5 seconds".
pub fn clock_words(secs: f64) -> String {
    let s = secs.max(0.0).ceil() as u64;
    let (m, s) = (s / 60, s % 60);
    let unit = |n: u64, one: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {one}s")
        }
    };
    match (m, s) {
        (0, s) => unit(s, "second"),
        (m, 0) => unit(m, "minute"),
        (m, s) => format!("{} {}", unit(m, "minute"), unit(s, "second")),
    }
}

/// A time to the tenth, in words, for best times: "1 minute 5.3 seconds".
pub fn time_short(secs: f64) -> String {
    let tenths = (secs.max(0.0) * 10.0).round() as u64;
    let (m, rest) = (tenths / 600, tenths % 600);
    let seconds = format!("{}.{} seconds", rest / 10, rest % 10);
    match m {
        0 => seconds,
        1 => format!("1 minute {seconds}"),
        m => format!("{m} minutes {seconds}"),
    }
}
