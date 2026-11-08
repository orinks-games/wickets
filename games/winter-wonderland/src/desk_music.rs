//! The Wish Desk's music, made up as you answer the letters.
//!
//! A slow piece in F major, one bar at a time. Every bar picks a one-bar
//! phrase for each part that is playing, over the bar's chord (rendered by
//! `tools/wish_desk_music.py`), so no two stacks sound the same. A music
//! box and a soft pad start it; the cello joins once a letter is answered,
//! and the clarinet's tune once half the stack is. Later stacks start with
//! more of the band.

use hearken::{Bus, Ctx, Sound};
use parlor_core::{Parlor, Rng};

/// Beats a minute.
pub const BPM: f64 = 76.0;
/// One bar of four beats, seconds.
pub const BAR: f64 = 4.0 * 60.0 / BPM;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    F,
    Dm,
    Bb,
    C,
    Gm,
    Am,
}

impl Chord {
    pub const ALL: [Chord; 6] = [
        Chord::F,
        Chord::Dm,
        Chord::Bb,
        Chord::C,
        Chord::Gm,
        Chord::Am,
    ];

    pub fn file(self) -> &'static str {
        match self {
            Chord::F => "f",
            Chord::Dm => "dm",
            Chord::Bb => "bb",
            Chord::C => "c",
            Chord::Gm => "gm",
            Chord::Am => "am",
        }
    }
}

use Chord::{Am, Bb, Dm, Gm, C, F};

/// Four-bar chord patterns in F. Most end on C, which leads home.
pub const PROGRESSIONS: &[[Chord; 4]] = &[
    [F, Dm, Bb, C],
    [F, Bb, Gm, C],
    [Dm, Bb, F, C],
    [F, Am, Bb, C],
    [Bb, F, Gm, C],
    [F, C, Dm, Bb],
    [Gm, Bb, F, C],
];

/// A part of the band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Pad,
    Keys,
    Bass,
    Tune,
}

impl Part {
    pub const ALL: [Part; 4] = [Part::Pad, Part::Keys, Part::Bass, Part::Tune];

    pub fn file(self) -> &'static str {
        match self {
            Part::Pad => "pad",
            Part::Keys => "keys",
            Part::Bass => "bass",
            Part::Tune => "tune",
        }
    }

    /// Phrases rendered for each chord; `tools/wish_desk_music.py` must match.
    pub fn variants(self) -> u32 {
        match self {
            Part::Pad => 2,
            Part::Keys => 4,
            Part::Bass => 3,
            Part::Tune => 4,
        }
    }

    pub fn volume(self) -> f32 {
        match self {
            Part::Pad => 0.5,
            Part::Keys => 0.7,
            Part::Bass => 0.7,
            Part::Tune => 0.6,
        }
    }

    /// The sound key of variant `n` over `chord`.
    pub fn key(self, chord: Chord, n: u32) -> String {
        format!(
            "winter-wonderland/desk/song/{}/{}_{n}",
            self.file(),
            chord.file()
        )
    }
}

/// The parts playing with `answered` of the stack done (0 to 1) on stack
/// `level`.
pub fn parts_for(answered: f32, level: u32) -> Vec<Part> {
    let mut parts = vec![Part::Pad, Part::Keys];
    if answered > 0.0 || level >= 3 {
        parts.push(Part::Bass);
    }
    if answered >= 0.5 || level >= 5 {
        parts.push(Part::Tune);
    }
    parts
}

/// Plays the desk's music along with a stack.
pub struct Conductor {
    rng: Rng,
    progression: usize,
    bar: u32,
    next_at: f64,
    started: bool,
    playing: Vec<Sound>,
    /// The parts in the bar now playing.
    pub parts: Vec<Part>,
}

impl Conductor {
    pub fn new(seed: u64) -> Conductor {
        let mut rng = Rng::seeded(seed ^ 0xde5c_0001);
        let progression = rng.below(PROGRESSIONS.len() as u32) as usize;
        Conductor {
            rng,
            progression,
            bar: 0,
            next_at: 0.0,
            started: false,
            playing: Vec::new(),
            parts: Vec::new(),
        }
    }

    /// Plays any bar that is due at run time `time`.
    pub fn follow(&mut self, ctx: &mut Ctx<Parlor>, time: f64, answered: f32, level: u32) {
        if !self.started {
            self.started = true;
            self.next_at = time;
        }
        while time >= self.next_at {
            self.play_bar(ctx, answered, level);
            self.next_at += BAR;
        }
    }

    fn play_bar(&mut self, ctx: &mut Ctx<Parlor>, answered: f32, level: u32) {
        let in_pattern = (self.bar % 4) as usize;
        if in_pattern == 0 && self.bar > 0 && self.bar.is_multiple_of(8) {
            let mut next = self.rng.below(PROGRESSIONS.len() as u32) as usize;
            if next == self.progression {
                next = (next + 1) % PROGRESSIONS.len();
            }
            self.progression = next;
        }
        let chord = PROGRESSIONS[self.progression][in_pattern];
        self.parts = parts_for(answered, level);
        // Last bar's sounds ring out on their own; only the newest are kept.
        self.playing.clear();
        for part in self.parts.clone() {
            let n = self.rng.below(part.variants());
            let s = ctx.audio.play(&part.key(chord, n), Bus::MUSIC);
            ctx.audio.set_volume(s, part.volume());
            self.playing.push(s);
        }
        self.bar += 1;
    }

    /// Fades the music out, as the stack ends or the screen closes.
    pub fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.playing.drain(..) {
            ctx.audio.stop(s, 600);
        }
        self.started = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_band_grows_as_the_stack_is_answered() {
        assert_eq!(parts_for(0.0, 1), vec![Part::Pad, Part::Keys]);
        assert_eq!(parts_for(0.25, 1).len(), 3);
        assert_eq!(parts_for(0.5, 1).len(), 4);
        assert_eq!(parts_for(0.0, 5).len(), 4);
    }
}
