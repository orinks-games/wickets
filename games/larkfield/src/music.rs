//! The hamlet's tune, made up as the day goes.
//!
//! The tune is in G major at 84 beats a minute, one bar of four beats at a
//! time. A guitar plays under everything: that is you. Every settler has
//! their own instrument, by the order they arrived (fiddle, flute,
//! accordion, harp, cello, clarinet), and plays their part from wherever
//! they are working, so you hear who is where. A cheerful settler plays
//! out, a glum one plays softly and only every other bar, and one stuck
//! playing the same song over and over plays exactly that. Each season
//! has its own chord patterns. The phrases are rendered by
//! `tools/larkfield_music.py`.

use hearken::{Bus, Ctx, Vec3};
use parlor_core::{Parlor, Rng};

use crate::crops::Season;
use crate::people::INSTRUMENTS;

pub const BPM: f64 = 84.0;
/// One bar, seconds.
pub const BAR: f64 = 4.0 * 60.0 / BPM;
/// Phrases rendered for each part over each chord.
pub const VARIANTS: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    G,
    C,
    D,
    Em,
    Am,
}

impl Chord {
    pub const ALL: [Chord; 5] = [Chord::G, Chord::C, Chord::D, Chord::Em, Chord::Am];

    pub fn file(self) -> &'static str {
        match self {
            Chord::G => "g",
            Chord::C => "c",
            Chord::D => "d",
            Chord::Em => "em",
            Chord::Am => "am",
        }
    }
}

use Chord::{Am, Em, C, D, G};

/// Four-bar chord patterns for each season.
pub fn progressions(season: Season) -> &'static [[Chord; 4]] {
    match season {
        Season::Spring => &[[G, C, G, D], [G, Em, C, D], [C, G, Am, D]],
        Season::Summer => &[[G, D, Em, C], [C, D, G, G], [G, C, D, D]],
        Season::Autumn => &[[Em, C, G, D], [Am, C, G, D], [Em, Am, C, D]],
        Season::Winter => &[[Em, C, Am, D], [Am, Em, C, D], [Em, D, C, D]],
    }
}

/// The guitar under everything.
pub fn bed_key(chord: Chord, n: u32) -> String {
    format!("larkfield/band/guitar/{}_{n}", chord.file())
}

/// The phrase of instrument `i` over `chord`.
pub fn part_key(i: usize, chord: Chord, n: u32) -> String {
    format!(
        "larkfield/band/{}/{}_{n}",
        INSTRUMENTS[i % INSTRUMENTS.len()],
        chord.file()
    )
}

/// One settler's part this bar: their instrument, where they are, and
/// how they feel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    pub instrument: usize,
    pub at: Vec3,
    pub volume: f32,
    /// Plays only every other bar.
    pub sparse: bool,
    /// Plays the same phrase every bar.
    pub same_song: bool,
}

/// Plays the tune along with the day.
pub struct Band {
    rng: Rng,
    progression: usize,
    bar: u32,
    next_at: f64,
    time: f64,
    started: bool,
    season: Season,
    /// The parts in the bar now playing, by instrument.
    pub playing: Vec<usize>,
}

impl Band {
    pub fn new(seed: u64) -> Band {
        Band {
            rng: Rng::seeded(seed ^ 0x1a2c_f1e1),
            progression: 0,
            bar: 0,
            next_at: 0.0,
            time: 0.0,
            started: false,
            season: Season::Spring,
            playing: Vec::new(),
        }
    }

    /// Starts again from the top of a pattern, as after a break.
    pub fn restart(&mut self) {
        self.started = false;
        self.bar = 0;
    }

    pub fn bar(&self) -> u32 {
        self.bar
    }

    /// Moves the band on by `dt`, playing any bar that is due.
    pub fn follow(&mut self, ctx: &mut Ctx<Parlor>, dt: f64, season: Season, players: &[Player]) {
        self.time += dt;
        if !self.started {
            self.started = true;
            self.next_at = self.time;
            self.season = season;
            self.progression = self.rng.below(progressions(season).len() as u32) as usize;
        }
        while self.time >= self.next_at {
            self.play_bar(ctx, season, players);
            self.next_at += BAR;
        }
    }

    fn play_bar(&mut self, ctx: &mut Ctx<Parlor>, season: Season, players: &[Player]) {
        let pats = progressions(season);
        let in_pattern = (self.bar % 4) as usize;
        if in_pattern == 0 && (season != self.season || self.bar > 0 && self.bar.is_multiple_of(8))
        {
            self.season = season;
            self.progression = self.rng.below(pats.len() as u32) as usize;
        }
        let chord = pats[self.progression % pats.len()][in_pattern];
        let bed = ctx
            .audio
            .play(&bed_key(chord, self.rng.below(VARIANTS)), Bus::MUSIC);
        ctx.audio.set_volume(bed, 0.55);
        self.playing.clear();
        for p in players {
            if p.sparse && self.bar % 2 == 1 {
                continue;
            }
            let n = if p.same_song {
                0
            } else {
                self.rng.below(VARIANTS)
            };
            let key = part_key(p.instrument, chord, n);
            let s = ctx.audio.play_at(&key, p.at, Bus::MUSIC);
            ctx.audio.set_volume(s, p.volume);
            self.playing.push(p.instrument);
        }
        self.bar += 1;
    }
}
