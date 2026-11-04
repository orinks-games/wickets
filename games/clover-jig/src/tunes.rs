//! The steps, the tunes and the sets they are danced in.
//!
//! Every tune has two dance beats a bar: a jig's bar is two groups of three
//! eighth notes, a reel's two groups of four. A tune is two parts, A and B,
//! of four bars each, played A A B B twice: 32 bars, 64 beats.

use parlor_core::ParlorAction;

/// One dance step the caller can call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    Left,
    Right,
    Hop,
    Turn,
    Forward,
    Back,
}

impl Step {
    pub const ALL: [Step; 6] = [
        Step::Left,
        Step::Right,
        Step::Hop,
        Step::Turn,
        Step::Forward,
        Step::Back,
    ];

    /// The caller's word, capitalised: "Left".
    pub fn word(self) -> &'static str {
        match self {
            Step::Left => "Left",
            Step::Right => "Right",
            Step::Hop => "Hop",
            Step::Turn => "Turn",
            Step::Forward => "Forward",
            Step::Back => "Back",
        }
    }

    /// The word inside a sentence: "left".
    pub fn lower(self) -> &'static str {
        match self {
            Step::Left => "left",
            Step::Right => "right",
            Step::Hop => "hop",
            Step::Turn => "turn",
            Step::Forward => "forward",
            Step::Back => "back",
        }
    }

    /// The shared control that dances it.
    pub fn action(self) -> ParlorAction {
        match self {
            Step::Left => ParlorAction::Left,
            Step::Right => ParlorAction::Right,
            Step::Hop => ParlorAction::Primary,
            Step::Turn => ParlorAction::Secondary,
            Step::Forward => ParlorAction::Up,
            Step::Back => ParlorAction::Down,
        }
    }

    pub fn for_action(action: ParlorAction) -> Option<Step> {
        Step::ALL.into_iter().find(|s| s.action() == action)
    }

    /// What the step sounds like when your foot lands, for the sound check.
    pub fn sound_words(self) -> &'static str {
        match self {
            Step::Left => "a hard shoe tap on the boards, from your left",
            Step::Right => "a hard shoe tap on the boards, from your right",
            Step::Hop => "two quick heel clicks",
            Step::Turn => "a swish of skirts and a tap",
            Step::Forward => "a firm stamp ahead of you",
            Step::Back => "a double shuffle behind you",
        }
    }
}

/// A jig in 6/8 or a reel in 4/4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Jig,
    Reel,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Jig => "jig",
            Kind::Reel => "reel",
        }
    }
}

/// One tune.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tune {
    /// Names its sounds: `clover-jig/tune/<id>/a`.
    pub id: &'static str,
    pub name: &'static str,
    pub kind: Kind,
    /// Dance beats a minute, two to a bar.
    pub bpm: u32,
    /// Played by the full ceili band, on Saint Patrick's Day only.
    pub ceili: bool,
}

impl Tune {
    /// "The Turf Fire, a reel".
    pub fn title(&self) -> String {
        format!("{}, a {}", self.name, self.kind.word())
    }

    /// The sound key of part A (`b == false`) or part B.
    pub fn part_key(&self, b: bool) -> String {
        let folder = if self.ceili { "ceili" } else { "tune" };
        let part = if b { "b" } else { "a" };
        format!("{}/{folder}/{}/{part}", crate::ID, self.id)
    }

    /// Milliseconds a beat lasts.
    pub fn beat_ms(&self) -> f64 {
        60_000.0 / self.bpm as f64
    }
}

const fn tune(id: &'static str, name: &'static str, kind: Kind, bpm: u32) -> Tune {
    Tune {
        id,
        name,
        kind,
        bpm,
        ceili: false,
    }
}

const fn ceili(id: &'static str, name: &'static str, kind: Kind, bpm: u32) -> Tune {
    Tune {
        id,
        name,
        kind,
        bpm,
        ceili: true,
    }
}

/// Every tune of the four sets, in order, three to a set. The ids match
/// `sound-specs/clover-jig.json`.
pub const TUNES: [Tune; 12] = [
    tune("kettle", "The Kettle on the Hob", Kind::Jig, 88),
    tune("turf-fire", "The Turf Fire", Kind::Reel, 92),
    tune("rocking-chair", "Granny's Rocking Chair", Kind::Jig, 96),
    tune("crossroads", "The Crossroads Jig", Kind::Jig, 98),
    tune("mill-road", "The Mill Road", Kind::Reel, 100),
    tune("hazel-gate", "The Hazel Gate", Kind::Jig, 104),
    tune("market-cart", "The Market Cart", Kind::Reel, 106),
    tune("tinkers-purse", "The Tinker's Purse", Kind::Jig, 108),
    tune("fair-day", "The Fair Day", Kind::Reel, 112),
    tune("harbour-lights", "The Harbour Lights", Kind::Jig, 114),
    tune("fishermans-lilt", "The Fisherman's Lilt", Kind::Reel, 118),
    tune("salt-spray", "The Salt Spray", Kind::Jig, 124),
];

/// The set dance of ceili night: four tunes for the full band.
pub const CEILI_TUNES: [Tune; 4] = [
    ceili("green-ribbon", "The Green Ribbon", Kind::Jig, 104),
    ceili("lucky-penny", "The Lucky Penny", Kind::Reel, 100),
    ceili("clover-field", "The Clover Field", Kind::Jig, 110),
    ceili("saints-day", "The Saint's Day", Kind::Reel, 108),
];

/// The figures the caller names between the tunes of the set dance.
pub const FIGURES: [&str; 4] = ["Lead around", "The swing", "Across the floor", "Home again"];

/// How a set of tunes is called: which steps, and how often.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    /// The steps the caller uses.
    pub steps: &'static [Step],
    /// The chance of a step on each bar's first beat.
    pub downbeat: f64,
    /// The chance of a step on each bar's second beat.
    pub offbeat: f64,
}

pub const BASIC: &[Step] = &[Step::Left, Step::Right, Step::Hop];
pub const WITH_TURN: &[Step] = &[Step::Left, Step::Right, Step::Hop, Step::Turn];
pub const EVERY_STEP: &[Step] = &Step::ALL;

/// Where the bed sound of a set comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Kitchen,
    Outdoors,
    Square,
    Harbour,
    Hall,
}

/// One set of three tunes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Set {
    pub number: u32,
    /// "the kitchen".
    pub name: &'static str,
    /// One sentence on the scene, for the set's help.
    pub scene: &'static str,
    pub place: Place,
    pub level: Level,
    /// Indexes into [`TUNES`].
    pub tunes: [usize; 3],
}

impl Set {
    /// "Set 2, the crossroads".
    pub fn title(&self) -> String {
        format!("Set {}, {}", self.number, self.name)
    }

    pub fn tunes(&self) -> [&'static Tune; 3] {
        self.tunes.map(|i| &TUNES[i])
    }

    /// The step this set adds, if any, for the intro.
    pub fn new_steps(&self) -> &'static [Step] {
        match self.number {
            1 => BASIC,
            2 => &[Step::Turn],
            3 => &[Step::Forward, Step::Back],
            _ => &[],
        }
    }
}

pub const SETS: [Set; 4] = [
    Set {
        number: 1,
        name: "the kitchen",
        scene: "A turf fire, a kettle, and room for one dancer on the flagstones.",
        place: Place::Kitchen,
        level: Level {
            steps: BASIC,
            downbeat: 0.8,
            offbeat: 0.0,
        },
        tunes: [0, 1, 2],
    },
    Set {
        number: 2,
        name: "the crossroads",
        scene: "An evening dance on the road outside the village, with the birds going to roost.",
        place: Place::Outdoors,
        level: Level {
            steps: WITH_TURN,
            downbeat: 0.9,
            offbeat: 0.12,
        },
        tunes: [3, 4, 5],
    },
    Set {
        number: 3,
        name: "the market square",
        scene: "Fair day in the square, with stalls all round and a crowd to watch.",
        place: Place::Square,
        level: Level {
            steps: EVERY_STEP,
            downbeat: 1.0,
            offbeat: 0.3,
        },
        tunes: [6, 7, 8],
    },
    Set {
        number: 4,
        name: "the harbour hall",
        scene: "The big hall by the harbour, the sea outside and the fastest band in the county.",
        place: Place::Harbour,
        level: Level {
            steps: EVERY_STEP,
            downbeat: 1.0,
            offbeat: 0.55,
        },
        tunes: [9, 10, 11],
    },
];

/// How the set dance of ceili night is called.
pub const CEILI_LEVEL: Level = Level {
    steps: EVERY_STEP,
    downbeat: 0.9,
    offbeat: 0.2,
};

/// The set a tune belongs to, by its index in [`TUNES`].
pub fn set_of(tune: usize) -> &'static Set {
    &SETS[(tune / 3).min(SETS.len() - 1)]
}

/// A tune by id, from the sets or the ceili.
pub fn by_id(id: &str) -> Option<&'static Tune> {
    TUNES.iter().chain(CEILI_TUNES.iter()).find(|t| t.id == id)
}
