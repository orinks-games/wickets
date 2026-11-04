//! The fireflies of the meadow: each species' blink, its tone, and where
//! it lives, and the chords the jar can make when it is set free.

use serde::{Deserialize, Serialize};

/// A firefly species. The journal names each one once it is caught.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Species {
    Dipper,
    TwinSpark,
    SlowEmber,
    TripleTwinkle,
    OrchardLantern,
    CreekGlimmer,
    HayFlicker,
    BlueGhost,
    LateEmber,
    /// The synchronous firefly, met only on midsummer night.
    MidsummerChorus,
}

/// How a firefly blinks: a burst of `flashes` lights, each `on` seconds
/// long with `gap` seconds between, then `dark` seconds of darkness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pattern {
    pub flashes: u8,
    pub on: f64,
    pub gap: f64,
    pub dark: f64,
}

impl Pattern {
    /// One whole blink, burst and darkness.
    pub fn cycle(&self) -> f64 {
        self.burst() + self.dark
    }

    /// The burst alone, from the first light to the end of the last.
    pub fn burst(&self) -> f64 {
        self.flashes as f64 * self.on + (self.flashes.saturating_sub(1)) as f64 * self.gap
    }

    /// When flash `i` (from 0) starts, from the start of the cycle.
    pub fn flash_start(&self, i: u8) -> f64 {
        i as f64 * (self.on + self.gap)
    }

    /// The flash lit at `phase` seconds into the cycle, if any.
    pub fn lit(&self, phase: f64) -> Option<u8> {
        self.near(phase, 0.0, 0.0)
    }

    /// The flash at or near `phase`, allowing `before` seconds early and
    /// `after` seconds late: how a press in time is judged.
    pub fn near(&self, phase: f64, before: f64, after: f64) -> Option<u8> {
        (0..self.flashes).find(|&i| {
            let start = self.flash_start(i);
            phase >= start - before && phase < start + self.on + after
        })
    }
}

impl Species {
    pub const ALL: [Species; 10] = [
        Species::Dipper,
        Species::TwinSpark,
        Species::SlowEmber,
        Species::TripleTwinkle,
        Species::OrchardLantern,
        Species::CreekGlimmer,
        Species::HayFlicker,
        Species::BlueGhost,
        Species::LateEmber,
        Species::MidsummerChorus,
    ];

    /// The species that can go in the jar: all but the midsummer chorus,
    /// which only blinks in the midsummer show.
    pub const JAR: [Species; 9] = [
        Species::Dipper,
        Species::TwinSpark,
        Species::SlowEmber,
        Species::TripleTwinkle,
        Species::OrchardLantern,
        Species::CreekGlimmer,
        Species::HayFlicker,
        Species::BlueGhost,
        Species::LateEmber,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Species::Dipper => "Dipper",
            Species::TwinSpark => "Twin Spark",
            Species::SlowEmber => "Slow Ember",
            Species::TripleTwinkle => "Triple Twinkle",
            Species::OrchardLantern => "Orchard Lantern",
            Species::CreekGlimmer => "Creek Glimmer",
            Species::HayFlicker => "Hay Flicker",
            Species::BlueGhost => "Blue Ghost",
            Species::LateEmber => "Late Ember",
            Species::MidsummerChorus => "Midsummer Chorus",
        }
    }

    /// The saved name, which never changes.
    pub fn slug(self) -> &'static str {
        match self {
            Species::Dipper => "dipper",
            Species::TwinSpark => "twin_spark",
            Species::SlowEmber => "slow_ember",
            Species::TripleTwinkle => "triple_twinkle",
            Species::OrchardLantern => "orchard_lantern",
            Species::CreekGlimmer => "creek_glimmer",
            Species::HayFlicker => "hay_flicker",
            Species::BlueGhost => "blue_ghost",
            Species::LateEmber => "late_ember",
            Species::MidsummerChorus => "midsummer_chorus",
        }
    }

    pub fn pattern(self) -> Pattern {
        let p = |flashes, on, gap, dark| Pattern {
            flashes,
            on,
            gap,
            dark,
        };
        match self {
            Species::Dipper => p(1, 0.6, 0.0, 3.4),
            Species::TwinSpark => p(2, 0.3, 0.35, 3.0),
            Species::SlowEmber => p(1, 1.2, 0.0, 4.0),
            Species::TripleTwinkle => p(3, 0.25, 0.3, 3.4),
            Species::OrchardLantern => p(1, 0.45, 0.0, 5.0),
            Species::CreekGlimmer => p(2, 0.35, 0.6, 3.2),
            Species::HayFlicker => p(1, 0.2, 0.0, 1.9),
            Species::BlueGhost => p(1, 5.0, 0.0, 4.0),
            Species::LateEmber => p(1, 0.9, 0.0, 5.5),
            Species::MidsummerChorus => p(6, 0.3, 0.3, 2.4),
        }
    }

    /// Its blink in words, for the journal and its first catch.
    pub fn blink_words(self) -> &'static str {
        match self {
            Species::Dipper => "one long flash, low and golden",
            Species::TwinSpark => "two quick flashes",
            Species::SlowEmber => "one slow, deep glow",
            Species::TripleTwinkle => "three quick, high flashes",
            Species::OrchardLantern => "one bright flash with a long dark wait",
            Species::CreekGlimmer => "two flashes with a pause between",
            Species::HayFlicker => "one tiny, quick flash, often",
            Species::BlueGhost => "a steady glow that lasts for seconds",
            Species::LateEmber => "one low, slow flash, far apart",
            Species::MidsummerChorus => "six flashes in a row, all together",
        }
    }

    /// A line on it for the journal.
    pub fn about(self) -> &'static str {
        match self {
            Species::Dipper => "The most common firefly. Its flash swoops upward as it flies, like a letter J drawn in light.",
            Species::TwinSpark => "Small and busy, it flashes twice and moves on. Often the first out in spring.",
            Species::SlowEmber => "A big, unhurried firefly that glows low in the grass like a coal.",
            Species::TripleTwinkle => "It rises as it blinks, three sparks in a row, high above the grass.",
            Species::OrchardLantern => "It hangs in the apple trees and lights up seldom, but brightly.",
            Species::CreekGlimmer => "A green glimmer that follows the water, flash, pause, flash.",
            Species::HayFlicker => "A tiny firefly of the hayfield that flickers fast. Quick hands catch it.",
            Species::BlueGhost => "Rare and quiet. It glows a soft blue near the ground and drifts like a ghost.",
            Species::LateEmber => "The last firefly of the year, out on cool autumn evenings when the others are gone.",
            Species::MidsummerChorus => "On midsummer night thousands of them flash together, the whole meadow in one breath.",
        }
    }

    /// Where the player is likely to meet it, for the journal's unknown rows.
    pub fn where_words(self) -> &'static str {
        match self {
            Species::Dipper => "Found in every meadow, spring and summer.",
            Species::TwinSpark => {
                "In spring, every meadow; in summer, the home meadow and the creek bank."
            }
            Species::SlowEmber => {
                "In summer, the home meadow and the hayfield; in early autumn, every meadow."
            }
            Species::TripleTwinkle => "Found in the orchard and the hayfield, in summer.",
            Species::OrchardLantern => "Found only in the orchard, in summer.",
            Species::CreekGlimmer => "Found only on the creek bank, in summer.",
            Species::HayFlicker => "Found only in the hayfield, in summer.",
            Species::BlueGhost => "Rare: found in the orchard and on the creek bank, in summer.",
            Species::LateEmber => "Found only in autumn.",
            Species::MidsummerChorus => "Found only on midsummer night.",
        }
    }

    /// The note it rings, which becomes its voice in a jar's chord.
    pub fn note(self) -> Note {
        match self {
            Species::Dipper => Note::G,
            Species::TwinSpark => Note::C,
            Species::SlowEmber => Note::C,
            Species::TripleTwinkle => Note::E,
            Species::OrchardLantern => Note::A,
            Species::CreekGlimmer => Note::D,
            Species::HayFlicker => Note::G,
            Species::BlueGhost => Note::E,
            Species::LateEmber => Note::A,
            Species::MidsummerChorus => Note::C,
        }
    }

    pub fn from_slug(slug: &str) -> Option<Species> {
        Species::ALL.into_iter().find(|s| s.slug() == slug)
    }
}

/// The five notes of the meadow's scale, C major pentatonic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Note {
    C,
    D,
    E,
    G,
    A,
}

impl Note {
    pub fn name(self) -> &'static str {
        match self {
            Note::C => "C",
            Note::D => "D",
            Note::E => "E",
            Note::G => "G",
            Note::A => "A",
        }
    }
}

/// A chord the journal names, by the notes in the jar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Chord {
    OpenFifth,
    SummerTriad,
    DuskChord,
    CreekChord,
    PorchSong,
    WholeMeadow,
}

impl Chord {
    pub const ALL: [Chord; 6] = [
        Chord::OpenFifth,
        Chord::SummerTriad,
        Chord::DuskChord,
        Chord::CreekChord,
        Chord::PorchSong,
        Chord::WholeMeadow,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Chord::OpenFifth => "the Open Fifth",
            Chord::SummerTriad => "the Summer Triad",
            Chord::DuskChord => "the Dusk Chord",
            Chord::CreekChord => "the Creek Chord",
            Chord::PorchSong => "the Porch Song",
            Chord::WholeMeadow => "the Whole Meadow",
        }
    }

    /// The notes it needs, exactly.
    pub fn notes(self) -> &'static [Note] {
        match self {
            Chord::OpenFifth => &[Note::C, Note::G],
            Chord::SummerTriad => &[Note::C, Note::E, Note::G],
            Chord::DuskChord => &[Note::C, Note::E, Note::A],
            Chord::CreekChord => &[Note::D, Note::G, Note::A],
            Chord::PorchSong => &[Note::C, Note::E, Note::G, Note::A],
            Chord::WholeMeadow => &[Note::C, Note::D, Note::E, Note::G, Note::A],
        }
    }

    /// How to make it, in words, for the journal's unknown rows.
    pub fn recipe(self) -> &'static str {
        match self {
            Chord::OpenFifth => "Notes C and G, and nothing else.",
            Chord::SummerTriad => "Notes C, E and G.",
            Chord::DuskChord => "Notes C, E and A.",
            Chord::CreekChord => "Notes D, G and A.",
            Chord::PorchSong => "Notes C, E, G and A.",
            Chord::WholeMeadow => "All five notes, C, D, E, G and A.",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Chord::OpenFifth => "open_fifth",
            Chord::SummerTriad => "summer_triad",
            Chord::DuskChord => "dusk_chord",
            Chord::CreekChord => "creek_chord",
            Chord::PorchSong => "porch_song",
            Chord::WholeMeadow => "whole_meadow",
        }
    }
}

/// The distinct notes of a jar, low to high.
pub fn jar_notes(jar: &[Species]) -> Vec<Note> {
    let mut notes: Vec<Note> = jar.iter().map(|s| s.note()).collect();
    notes.sort();
    notes.dedup();
    notes
}

/// The named chord a jar makes, if any.
pub fn chord_of(jar: &[Species]) -> Option<Chord> {
    let notes = jar_notes(jar);
    Chord::ALL
        .into_iter()
        .find(|c| c.notes() == notes.as_slice())
}

/// The distinct species of a jar, in journal order.
pub fn jar_species(jar: &[Species]) -> Vec<Species> {
    let mut all = jar.to_vec();
    all.sort();
    all.dedup();
    all
}

/// "Dipper", "Dipper and Twin Spark", "Dipper, Twin Spark and Slow Ember".
pub fn list_words(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
