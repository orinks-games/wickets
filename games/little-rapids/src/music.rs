//! The river's song, made up as you sail.
//!
//! The song is in D major at the stream's own beat. Every bar it picks a
//! one-bar phrase for each part that is playing, over the bar's chord
//! (rendered by `tools/rapids_music.py`), so no two runs sound the same.
//! The parts join as the combo grows: a bass from the start, a plucked
//! tune at 10, a pad at 25 and the whole band at 50. A bump drops the
//! combo, and the next bar drops back to the bass. Every catch plays a
//! note from the bar's chord, climbing as the combo climbs.

use hearken::{Bus, Ctx};
use parlor_core::{Parlor, Rng};

use crate::rules::{BAR_BEATS, BEAT};

/// One bar, seconds.
pub const BAR: f64 = BEAT * BAR_BEATS as f64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    D,
    Bm,
    G,
    A,
    Em,
}

impl Chord {
    pub const ALL: [Chord; 5] = [Chord::D, Chord::Bm, Chord::G, Chord::A, Chord::Em];

    pub fn file(self) -> &'static str {
        match self {
            Chord::D => "d",
            Chord::Bm => "bm",
            Chord::G => "g",
            Chord::A => "a",
            Chord::Em => "em",
        }
    }
}

use Chord::{Bm, Em, A, D, G};

/// Four-bar chord patterns, all in D. Most end on A, which leads home.
pub const PROGRESSIONS: &[[Chord; 4]] = &[
    [D, Bm, G, A],
    [D, G, Em, A],
    [Bm, G, D, A],
    [D, A, Bm, G],
    [G, D, Em, A],
    [D, Em, G, A],
    [Bm, Em, G, A],
    [G, A, D, D],
];

/// A part of the band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Bass,
    Pluck,
    Pad,
    Counter,
    Drums,
}

impl Part {
    pub const ALL: [Part; 5] = [Part::Bass, Part::Pluck, Part::Pad, Part::Counter, Part::Drums];

    pub fn file(self) -> &'static str {
        match self {
            Part::Bass => "bass",
            Part::Pluck => "pluck",
            Part::Pad => "pad",
            Part::Counter => "counter",
            Part::Drums => "drums",
        }
    }

    /// Phrases rendered for each chord (drums: in all).
    pub fn variants(self) -> u32 {
        match self {
            Part::Bass => 3,
            Part::Pluck => 4,
            Part::Pad => 2,
            Part::Counter => 3,
            Part::Drums => 3,
        }
    }

    /// The combo that brings the part in.
    pub fn joins_at(self) -> u32 {
        match self {
            Part::Bass => 0,
            Part::Pluck => 10,
            Part::Pad => 25,
            Part::Counter | Part::Drums => 50,
        }
    }

    pub fn volume(self) -> f32 {
        match self {
            Part::Bass => 0.8,
            Part::Pluck => 0.75,
            Part::Pad => 0.55,
            Part::Counter => 0.6,
            Part::Drums => 0.7,
        }
    }

    /// The sound key of variant `n` over `chord`.
    pub fn key(self, chord: Chord, n: u32) -> String {
        match self {
            Part::Drums => format!("little-rapids/song/drums/{n}"),
            _ => format!("little-rapids/song/{}/{}_{n}", self.file(), chord.file()),
        }
    }
}

/// Notes per chord for catches, low to high.
pub const NOTES: u32 = 4;

/// The note a catch plays over `chord` at `combo`.
pub fn note_key(chord: Chord, combo: u32) -> String {
    format!(
        "little-rapids/song/note/{}_{}",
        chord.file(),
        combo.saturating_sub(1) % NOTES
    )
}

/// The parts playing at `combo`. Drift has the whole band from the start.
pub fn parts_for(combo: u32, everything: bool) -> Vec<Part> {
    Part::ALL
        .into_iter()
        .filter(|p| everything || combo >= p.joins_at())
        .collect()
}

/// Plays the song along with a run.
pub struct Conductor {
    rng: Rng,
    progression: usize,
    bar: u32,
    /// When the next bar starts, in run seconds.
    next_at: f64,
    started: bool,
    /// The parts in the bar now playing.
    pub playing: Vec<Part>,
    chord: Chord,
}

impl Conductor {
    pub fn new(seed: u64) -> Conductor {
        let mut rng = Rng::seeded(seed ^ 0x5eed_50ab);
        let progression = rng.below(PROGRESSIONS.len() as u32) as usize;
        Conductor {
            rng,
            progression,
            bar: 0,
            next_at: 0.0,
            started: false,
            playing: Vec::new(),
            chord: PROGRESSIONS[progression][0],
        }
    }

    /// The chord of the bar now playing.
    pub fn chord(&self) -> Chord {
        self.chord
    }

    pub fn bar(&self) -> u32 {
        self.bar
    }

    /// Plays any bar that is due at run time `time`.
    pub fn follow(&mut self, ctx: &mut Ctx<Parlor>, time: f64, combo: u32, everything: bool) {
        if !self.started {
            self.started = true;
            self.next_at = time;
        }
        while time >= self.next_at {
            self.play_bar(ctx, combo, everything);
            self.next_at += BAR;
        }
    }

    fn play_bar(&mut self, ctx: &mut Ctx<Parlor>, combo: u32, everything: bool) {
        let in_pattern = (self.bar % 4) as usize;
        if in_pattern == 0 && self.bar > 0 && self.bar.is_multiple_of(8) {
            // A new chord pattern every eight bars.
            let mut next = self.rng.below(PROGRESSIONS.len() as u32) as usize;
            if next == self.progression {
                next = (next + 1) % PROGRESSIONS.len();
            }
            self.progression = next;
        }
        self.chord = PROGRESSIONS[self.progression][in_pattern];
        self.playing = parts_for(combo, everything);
        let parts = self.playing.clone();
        for part in parts {
            let n = self.rng.below(part.variants());
            let key = part.key(self.chord, n);
            let s = ctx.audio.play(&key, Bus::MUSIC);
            ctx.audio.set_volume(s, part.volume());
        }
        self.bar += 1;
    }

    /// A catch's note, over this bar's chord.
    pub fn catch_note(&self, ctx: &mut Ctx<Parlor>, combo: u32, volume: f32) {
        let s = ctx.audio.play(&note_key(self.chord, combo), Bus::EFFECTS);
        ctx.audio.set_volume(s, volume);
    }
}
