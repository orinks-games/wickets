//! The skate's music, played live: a medley of carols and a waltz, strung
//! together a section at a time, following the night on the ice.
//!
//! Each section is a phrase of one tune, rendered as a lead line on each
//! of its style's leads and an accompaniment in each style
//! (`tools/snowfall_skate_sounds.py`). The medley picks the next tune as
//! it goes, never the one just played, and starts a lead and its
//! accompaniment on the same sample, each section over the last one's
//! ring-out.
//!
//! What it picks follows the skate ([`Mood`]):
//!
//! - On clear ice, the waltz band: bass, pizzicato and harp under a flute,
//!   a clarinet or a celesta.
//! - Once the snow lies deep, held strings under a music box or a celesta,
//!   and the quietest carol comes up more often.
//! - At the Frost Fair, the brass band.
//!
//! The lead follows your line: a soft lead while the line is short, full
//! once it is long, and back down to the accompaniment when it breaks.

use std::collections::VecDeque;

use hearken::audio::Start;
use hearken::{Audio, Bus, Performer, Sound, Variety};
use parlor_core::Rng;

/// How long an old section rings on under a new one before it is stopped.
const TAIL_FADE_MS: u32 = 400;

/// One phrase of a tune: its name, beats and tempo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    pub name: &'static str,
    pub beats: u32,
    pub bpm: f64,
}

impl Section {
    pub fn secs(&self) -> f64 {
        self.beats as f64 * 60.0 / self.bpm
    }
}

const fn sec(name: &'static str, beats: u32, bpm: f64) -> Section {
    Section { name, beats, bpm }
}

/// The tunes, each its sections in order. The first is the game's own
/// waltz; the rest are public-domain carols, never named in the game.
pub const TUNES: [&[Section]; 5] = [
    &[sec("waltz-a", 24, 144.0), sec("waltz-b", 24, 144.0)],
    &[sec("tree-a", 24, 120.0), sec("tree-b", 24, 120.0)],
    &[sec("wish", 24, 150.0)],
    &[sec("king-a", 32, 112.0), sec("king-b", 36, 112.0)],
    &[sec("still-a", 36, 96.0), sec("still-b", 36, 96.0)],
];

/// The quiet carol, more likely once the snow lies deep.
const QUIET: usize = 4;

/// The new year's song: in the medley from the 31st of December to the 1st
/// of January only, and the last thing heard on a skate then.
pub const YEAR: Section = sec("year", 32, 100.0);

/// The accompaniments, and the leads each carries.
pub const STYLES: [(&str, &[&str]); 3] = [
    ("waltz", &["flute", "clarinet", "celesta"]),
    ("hush", &["music_box", "celesta"]),
    ("band", &["trumpet", "horn"]),
];

/// Every section, the new year's song last.
pub fn sections() -> impl Iterator<Item = &'static Section> {
    TUNES
        .iter()
        .flat_map(|t| t.iter())
        .chain(std::iter::once(&YEAR))
}

pub fn lead_key(section: &str, lead: &str) -> String {
    format!("snowfall-skate/music/lead/{section}-{lead}")
}

pub fn bed_key(section: &str, style: &str) -> String {
    format!("snowfall-skate/music/bed/{section}-{style}")
}

/// A harp sweep up, for a landed spin.
pub const SWEEP: &str = "snowfall-skate/music/sweep";

/// Every key the medley can play.
pub fn every_key() -> Vec<String> {
    let mut out = Vec::new();
    for s in sections() {
        for (style, leads) in STYLES {
            out.push(bed_key(s.name, style));
            out.extend(leads.iter().map(|l| lead_key(s.name, l)));
        }
    }
    out
}

/// What the night is like, for the next tune.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mood {
    /// The snow lies deep.
    pub deep: bool,
    /// At the Frost Fair.
    pub fair: bool,
    /// The 31st of December or the 1st of January.
    pub new_year: bool,
}

impl Mood {
    fn style(self) -> usize {
        if self.fair {
            2
        } else if self.deep {
            1
        } else {
            0
        }
    }
}

/// One section to play: its accompaniment and its tune.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub bed: String,
    pub lead: String,
    pub secs: f64,
}

/// The medley as it plays.
pub struct Medley {
    rng: Rng,
    queue: VecDeque<Cue>,
    last_tune: Option<usize>,
    last_lead: Option<&'static str>,
    pub mood: Mood,
    clock: f64,
    now: Option<(f64, Sound, Sound)>,
    tail: Vec<Sound>,
    lead_level: f32,
    paused: bool,
    /// Every cue started, for the tests.
    pub played: Vec<Cue>,
}

impl Medley {
    pub fn new(seed: u64) -> Medley {
        Medley {
            rng: Rng::seeded(seed),
            queue: VecDeque::new(),
            last_tune: None,
            last_lead: None,
            mood: Mood::default(),
            clock: 0.0,
            now: None,
            tail: Vec::new(),
            lead_level: LEAD_LOW,
            paused: false,
            played: Vec::new(),
        }
    }

    pub fn started(&self) -> bool {
        self.now.is_some()
    }

    /// Queues the next tune for the mood now.
    fn plan_tune(&mut self) {
        let (style, leads) = STYLES[self.mood.style()];
        // The new year's song is tune number TUNES.len().
        let count = TUNES.len() + usize::from(self.mood.new_year);
        let tune = loop {
            let t = if self.mood.deep && !self.mood.fair && self.rng.chance(0.5) {
                QUIET
            } else {
                self.rng.below(count as u32) as usize
            };
            if Some(t) != self.last_tune {
                break t;
            }
        };
        self.last_tune = Some(tune);
        let year: &[Section] = &[YEAR];
        let parts = TUNES.get(tune).copied().unwrap_or(year);
        for s in parts {
            let lead = loop {
                let l = *self.rng.pick(leads);
                if Some(l) != self.last_lead || leads.len() == 1 {
                    break l;
                }
            };
            self.last_lead = Some(lead);
            self.queue.push_back(Cue {
                bed: bed_key(s.name, style),
                lead: lead_key(s.name, lead),
                secs: s.secs(),
            });
        }
    }

    /// Sets the mood. A change of style drops what is queued, so the next
    /// tune starts in the new style.
    pub fn set_mood(&mut self, mood: Mood) {
        if mood.style() != self.mood.style() {
            self.queue.clear();
        }
        self.mood = mood;
    }

    fn next_cue(&mut self) -> Cue {
        if self.queue.is_empty() {
            self.plan_tune();
        }
        self.queue.pop_front().expect("a tune was just planned")
    }

    fn play_next(&mut self, audio: &mut Audio) {
        for s in self.tail.drain(..) {
            audio.stop(s, TAIL_FADE_MS);
        }
        if let Some((_, bed, lead)) = self.now.take() {
            self.tail.push(bed);
            self.tail.push(lead);
        }
        let cue = self.next_cue();
        let once = Start::ONCE;
        let s = audio.start_together(&[
            (cue.bed.as_str(), Bus::MUSIC, once),
            (cue.lead.as_str(), Bus::MUSIC, once.volume(self.lead_level)),
        ]);
        self.now = Some((cue.secs, s[0], s[1]));
        self.played.push(cue);
    }

    /// Plays the new year's song straight away, its lead full, for the
    /// end of a skate. Nothing follows it.
    pub fn finale(&mut self, audio: &mut Audio) {
        let (style, leads) = STYLES[self.mood.style()];
        self.queue.clear();
        self.queue.push_back(Cue {
            bed: bed_key(YEAR.name, style),
            lead: lead_key(YEAR.name, leads[0]),
            secs: YEAR.secs() + 600.0,
        });
        self.lead_level = 1.0;
        self.play_next(audio);
    }

    pub fn start(&mut self, audio: &mut Audio) {
        if self.now.is_none() {
            self.clock = 0.0;
            self.play_next(audio);
        }
    }

    pub fn update(&mut self, audio: &mut Audio, dt: f64) {
        if self.paused {
            return;
        }
        let Some((secs, _, _)) = self.now else {
            return;
        };
        self.clock += dt;
        if self.clock >= secs {
            self.clock -= secs;
            self.play_next(audio);
        }
    }

    /// Sets the lead's level, 0 to 1, over a short ramp.
    pub fn set_lead(&mut self, audio: &mut Audio, level: f32) {
        let level = level.clamp(0.0, 1.0);
        if (level - self.lead_level).abs() < 0.01 {
            return;
        }
        let fall = level < self.lead_level;
        self.lead_level = level;
        if let Some((_, _, lead)) = self.now {
            audio.set_volume_over(lead, level, if fall { 600 } else { 1200 });
        }
    }

    pub fn lead_level(&self) -> f32 {
        self.lead_level
    }

    fn sounds(&self) -> Vec<Sound> {
        let mut out = self.tail.clone();
        if let Some((_, bed, lead)) = self.now {
            out.push(bed);
            out.push(lead);
        }
        out
    }

    pub fn pause(&mut self, audio: &mut Audio) {
        if self.paused {
            return;
        }
        self.paused = true;
        for s in self.sounds() {
            audio.pause(s);
        }
    }

    pub fn resume(&mut self, audio: &mut Audio) {
        if !self.paused {
            return;
        }
        self.paused = false;
        for s in self.sounds() {
            audio.resume(s);
        }
    }

    pub fn stop(&mut self, audio: &mut Audio, fade_ms: u32) {
        for s in self.sounds() {
            audio.stop(s, fade_ms);
        }
        self.tail.clear();
        self.now = None;
        self.queue.clear();
        self.paused = false;
    }
}

/// The lead with no line at all: the accompaniment carries the tune.
pub const LEAD_LOW: f32 = 0.15;

/// How loud the lead is with a line of `line` lanterns.
pub fn lead_level(line: u32) -> f32 {
    (LEAD_LOW + 0.17 * line.min(5) as f32).min(1.0)
}

/// The medley on Wicket's soundtrack: three to five tunes, then a
/// ring-out, the lead full throughout.
pub struct Night {
    medley: Medley,
    left: f64,
    ending: Option<f64>,
}

impl Night {
    pub fn new(mut v: Variety) -> Night {
        let mut medley = Medley::new(v.next_u64());
        medley.lead_level = 1.0;
        Night {
            medley,
            left: 60.0 + 15.0 * f64::from(v.below(5)),
            ending: None,
        }
    }
}

impl Performer for Night {
    fn start(&mut self, audio: &mut Audio) {
        self.medley.start(audio);
    }

    fn update(&mut self, audio: &mut Audio, dt: f64) -> bool {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            return *left > 0.0;
        }
        self.left -= dt;
        let ends_section = self.medley.clock + dt >= self.medley.now.as_ref().map_or(0.0, |n| n.0);
        if self.left <= 0.0 && ends_section && self.medley.queue.is_empty() {
            self.ending = Some(2.5);
            return true;
        }
        self.medley.update(audio, dt);
        true
    }

    fn stop(&mut self, audio: &mut Audio) {
        self.medley.stop(audio, 600);
    }
}
