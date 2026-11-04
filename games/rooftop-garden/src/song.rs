//! The garden's song: its own tune, and new tunes made up as it plays.
//!
//! The garden's own tune is four bars (G, E minor, C, D), one loop per part.
//! Between plays of it the garden makes up new tunes: a chord pattern, and
//! for every bar and every part one of that part's one-bar phrases over the
//! bar's chord (rendered by `tools/garden_music.py`). Each new tune plays
//! twice, so the ear can catch it, then the next is made up. The order goes
//! round: the garden's own tune, a new tune twice, another new tune twice.

use crate::cues;
use crate::rules::Species;
use parlor_core::Rng;

/// One bar, seconds: four beats at 80 beats a minute.
pub const BAR: f64 = 3.0;
/// Bars in a verse.
pub const BARS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    G,
    Em,
    C,
    D,
    Am,
}

impl Chord {
    pub const ALL: [Chord; 5] = [Chord::G, Chord::Em, Chord::C, Chord::D, Chord::Am];

    fn file(self) -> &'static str {
        match self {
            Chord::G => "g",
            Chord::Em => "em",
            Chord::C => "c",
            Chord::D => "d",
            Chord::Am => "am",
        }
    }
}

use Chord::{Am, Em, C, D, G};

/// The chord patterns a new tune can take, all in G major. Most end on D,
/// which leads back to G.
pub const PROGRESSIONS: &[[Chord; BARS]] = &[
    [G, Em, C, D],
    [G, C, Am, D],
    [Em, C, G, D],
    [G, Am, C, D],
    [C, G, Am, D],
    [G, C, G, D],
    [Em, Am, C, D],
    [C, Am, G, D],
    [G, Em, Am, D],
    [Am, C, G, D],
    [G, D, Em, C],
    [C, D, G, Em],
];

/// One voice in the song: the pad under it, or a plant's part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Base,
    Tomato,
    Squash,
    Pumpkin,
    Bean,
    Sunflower,
    Lavender,
    Mint,
}

impl Part {
    pub const ALL: [Part; 8] = [
        Part::Base,
        Part::Tomato,
        Part::Squash,
        Part::Pumpkin,
        Part::Bean,
        Part::Sunflower,
        Part::Lavender,
        Part::Mint,
    ];

    /// The part a plant sings.
    pub fn of(species: Species, pumpkin: bool) -> Part {
        match species {
            Species::Squash if pumpkin => Part::Pumpkin,
            Species::Tomato => Part::Tomato,
            Species::Squash => Part::Squash,
            Species::Bean => Part::Bean,
            Species::Sunflower => Part::Sunflower,
            Species::Lavender => Part::Lavender,
            Species::Mint => Part::Mint,
        }
    }

    fn index(self) -> usize {
        Part::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }

    pub fn id(self) -> &'static str {
        self.file()
    }

    pub fn from_id(id: &str) -> Option<Part> {
        Part::ALL.into_iter().find(|part| part.file() == id)
    }

    fn file(self) -> &'static str {
        match self {
            Part::Base => "base",
            Part::Tomato => "tomato",
            Part::Squash => "squash",
            Part::Pumpkin => "pumpkin",
            Part::Bean => "bean",
            Part::Sunflower => "sunflower",
            Part::Lavender => "lavender",
            Part::Mint => "mint",
        }
    }

    /// How many phrases the part has over each chord; must match
    /// `tools/garden_music.py`.
    pub fn variants(self) -> u8 {
        match self {
            Part::Tomato | Part::Sunflower => 4,
            Part::Squash | Part::Pumpkin | Part::Bean | Part::Lavender => 3,
            Part::Base | Part::Mint => 1,
        }
    }

    /// The part's loop in the garden's own tune.
    pub fn own(self) -> &'static str {
        match self {
            Part::Base => cues::SONG_BASE,
            Part::Pumpkin => cues::PUMPKIN,
            Part::Tomato => cues::song(Species::Tomato),
            Part::Squash => cues::song(Species::Squash),
            Part::Bean => cues::song(Species::Bean),
            Part::Sunflower => cues::song(Species::Sunflower),
            Part::Lavender => cues::song(Species::Lavender),
            Part::Mint => cues::song(Species::Mint),
        }
    }

    /// Whether `key` is this part singing: its loop or one of its phrases.
    pub fn sings(self, key: &str) -> bool {
        key == self.own()
            || key
                .strip_prefix("rooftop-garden/song/made/")
                .and_then(|rest| rest.strip_prefix(self.file()))
                .is_some_and(|rest| rest.starts_with('/'))
    }

    /// The key of one of the part's phrases.
    pub fn phrase(self, chord: Chord, n: u8) -> String {
        format!(
            "rooftop-garden/song/made/{}/{}-{}",
            self.file(),
            chord.file(),
            n
        )
    }
}

/// A made-up tune: four chords, and each part's phrase in each bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tune {
    pub chords: [Chord; BARS],
    picks: [[u8; 8]; BARS],
}

impl Tune {
    pub fn compose(rng: &mut Rng) -> Tune {
        let chords = *rng.pick(PROGRESSIONS);
        let mut picks = [[0u8; 8]; BARS];
        for (bar, row) in picks.iter_mut().enumerate() {
            for part in Part::ALL {
                let n = part.variants() as u32;
                // The flute's phrase 0 is one long note: it ends the tune,
                // and nowhere else.
                row[part.index()] = match part {
                    Part::Sunflower if bar == BARS - 1 => 0,
                    Part::Sunflower => 1 + rng.below(n - 1) as u8,
                    _ => rng.below(n) as u8,
                };
            }
        }
        Tune { chords, picks }
    }

    /// The key `part` plays in `bar`.
    pub fn key(&self, part: Part, bar: usize) -> String {
        part.phrase(self.chords[bar], self.picks[bar][part.index()])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verse {
    /// The garden's own tune.
    Own,
    New(Tune),
}

/// Verses before the order goes round: own, new, new again, newer, newer again.
const ROUND: usize = 5;

/// Where the song is, and what comes next.
pub struct Song {
    rng: Rng,
    verse: Verse,
    /// Which verse of the round this is.
    turn: usize,
    bar: usize,
    clock: f64,
}

impl Song {
    pub fn new(seed: u64) -> Song {
        Song {
            rng: Rng::seeded(seed),
            verse: Verse::Own,
            turn: 0,
            bar: 0,
            clock: 0.0,
        }
    }

    pub fn verse(&self) -> &Verse {
        &self.verse
    }

    pub fn bar(&self) -> usize {
        self.bar
    }

    /// Back to the first bar of this verse, as the garden opens.
    pub fn restart(&mut self) {
        self.bar = 0;
        self.clock = 0.0;
    }

    /// Moves the song on; true when a new bar has started.
    pub fn tick(&mut self, dt: f64) -> bool {
        self.clock += dt;
        if self.clock < BAR {
            return false;
        }
        self.clock -= BAR;
        self.bar += 1;
        if self.bar == BARS {
            self.bar = 0;
            self.turn = (self.turn + 1) % ROUND;
            self.verse = match (self.turn, &self.verse) {
                (0, _) => Verse::Own,
                // A new tune plays twice.
                (2 | 4, Verse::New(tune)) => Verse::New(tune.clone()),
                _ => Verse::New(Tune::compose(&mut self.rng)),
            };
        }
        true
    }

    /// What `part` should start at the top of this bar, and how many
    /// seconds into it, or None to let what it is playing carry on. A loop
    /// of the garden's own tune lasts the whole verse; a part that joins
    /// partway through it starts from the bar it joins on. A made-up tune
    /// plays a new phrase every bar.
    pub fn cue(&self, part: Part, joining: bool) -> Option<(String, f64)> {
        match &self.verse {
            Verse::Own if self.bar == 0 => Some((part.own().to_string(), 0.0)),
            Verse::Own if joining => Some((part.own().to_string(), self.bar as f64 * BAR)),
            Verse::Own => None,
            Verse::New(tune) => Some((tune.key(part, self.bar), 0.0)),
        }
    }
}
