//! Dustfall's score, built live: one bar at a time, following the grid.
//!
//! The piece is in D minor at 72 beats a minute, four beats a bar, over
//! an eight-bar chord cycle. Each bar the [`Conductor`] starts one phrase
//! for every part that is playing, all on the same sample, and the last
//! bar rings out under the new one. The phrases come from
//! `tools/dustfall_music.py`, several for each part and chord, and the
//! conductor never plays the same one twice running, so no two nights
//! sound the same.
//!
//! What plays follows the night ([`Pressure`]):
//!
//! - The bass holds from the first turn.
//! - The pad joins while a raider is at row 4 or nearer.
//! - The plucked line joins while a raider is at row 2 or nearer.
//! - The danger layer, a high held note with a slow pulse, plays only
//!   while a charge is pending, or while the shield is at 0 with a raider
//!   at row 2 or nearer.
//! - A brass stab lands as a hulk, a mothership or the carrier steps, and a
//!   marimba run as a skitter pair steps ([`Stab`]).
//!
//! The colony phase has its own bed at half tempo from the same chords:
//! the bass and the pad, and the bass alone while the colony is hungry.

use hearken::audio::Start;
use hearken::{Audio, Bus, Performer, Sound, Variety};
use parlor_core::Rng;

/// Beats a minute on the night screen.
pub const BPM: f64 = 72.0;
/// Beats a minute in the colony phase.
pub const COLONY_BPM: f64 = 36.0;
/// Beats a bar.
pub const BEATS: f64 = 4.0;

/// How long the bar before last rings on before it is stopped.
const TAIL_FADE_MS: u32 = 400;

/// The chord cycle, a bar each. Must match `tools/dustfall_music.py`.
pub const CYCLE: [&str; 8] = ["Dm", "Bb", "F", "C", "Dm", "Bb", "Gm", "A"];

/// Every chord the cycle uses, once each.
pub const CHORDS: [&str; 6] = ["A", "Bb", "C", "Dm", "F", "Gm"];

/// The parts the conductor can play in a bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Bass,
    Pad,
    Pluck,
    Danger,
}

impl Part {
    pub const ALL: [Part; 4] = [Part::Bass, Part::Pad, Part::Pluck, Part::Danger];

    pub fn name(self) -> &'static str {
        match self {
            Part::Bass => "bass",
            Part::Pad => "pad",
            Part::Pluck => "pluck",
            Part::Danger => "danger",
        }
    }

    /// How many phrases the generator renders for the part and a chord.
    /// Must match the generator.
    pub fn variants(self) -> usize {
        match self {
            Part::Bass => 3,
            Part::Pad => 2,
            Part::Pluck => 3,
            Part::Danger => 1,
        }
    }

    fn slot(self) -> usize {
        match self {
            Part::Bass => 0,
            Part::Pad => 1,
            Part::Pluck => 2,
            Part::Danger => 3,
        }
    }
}

/// A one-shot the conductor plays on a raider's step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stab {
    /// A hulk, a mothership or the carrier steps: brass.
    Heavy,
    /// A skitter pair steps: a marimba run.
    Run,
}

impl Stab {
    pub fn name(self) -> &'static str {
        match self {
            Stab::Heavy => "heavy",
            Stab::Run => "run",
        }
    }
}

/// A night phrase: `dustfall/music/score/<part>-<chord>-<variant>`. The
/// danger layer fits every chord, so it is one phrase.
pub fn score_key(part: Part, chord: &str, variant: usize) -> String {
    match part {
        Part::Danger => "dustfall/music/score/danger".to_string(),
        part => format!("dustfall/music/score/{}-{chord}-{variant}", part.name()),
    }
}

/// A colony phrase: `dustfall/music/colony/<part>-<chord>`, bass or pad.
pub fn colony_key(part: Part, chord: &str) -> String {
    format!("dustfall/music/colony/{}-{chord}", part.name())
}

pub fn stab_key(stab: Stab, chord: &str) -> String {
    format!("dustfall/music/score/{}-{chord}", stab.name())
}

/// Every key the conductor can play.
pub fn every_key() -> Vec<String> {
    let mut out = Vec::new();
    for chord in CHORDS {
        for part in [Part::Bass, Part::Pad, Part::Pluck] {
            for v in 0..part.variants() {
                out.push(score_key(part, chord, v));
            }
            if part != Part::Pluck {
                out.push(colony_key(part, chord));
            }
        }
        out.push(stab_key(Stab::Heavy, chord));
        out.push(stab_key(Stab::Run, chord));
    }
    out.push(score_key(Part::Danger, "", 0));
    out
}

/// How close the night is, for the parts that play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pressure {
    /// The nearest raider's row, if any is on the grid.
    pub near: Option<u8>,
    /// A charge is pending.
    pub charge: bool,
    /// The shield is at 0: with a raider at row 2 or nearer, the dome
    /// itself is about to take the breach.
    pub shield_down: bool,
}

/// The pad joins when a raider is this near.
pub const PAD_ROW: u8 = 4;
/// The plucked line joins when a raider is this near.
pub const PLUCK_ROW: u8 = 2;

impl Pressure {
    pub fn parts(self) -> Vec<Part> {
        let mut out = vec![Part::Bass];
        if self.near.is_some_and(|row| row <= PAD_ROW) {
            out.push(Part::Pad);
        }
        if self.near.is_some_and(|row| row <= PLUCK_ROW) {
            out.push(Part::Pluck);
        }
        let exposed = self.shield_down && self.near.is_some_and(|row| row <= PLUCK_ROW);
        if self.charge || exposed {
            out.push(Part::Danger);
        }
        out
    }
}

/// Which tempo the conductor keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tempo {
    Night,
    Colony,
}

/// The score as it plays.
#[derive(Clone, Debug)]
pub struct Conductor {
    rng: Rng,
    tempo: Tempo,
    /// The bar of the cycle playing now.
    bar: usize,
    clock: f64,
    pressure: Pressure,
    hungry: bool,
    playing: bool,
    now: Vec<Sound>,
    tail: Vec<Sound>,
    /// The variant each part played last, so it is not played twice running.
    last: [Option<usize>; 4],
    /// The keys started each bar, for the tests.
    pub played: Vec<Vec<String>>,
}

impl Conductor {
    fn new(seed: u64, tempo: Tempo) -> Conductor {
        Conductor {
            rng: Rng::seeded(seed),
            tempo,
            bar: 0,
            clock: 0.0,
            pressure: Pressure::default(),
            hungry: false,
            playing: false,
            now: Vec::new(),
            tail: Vec::new(),
            last: [None; 4],
            played: Vec::new(),
        }
    }

    /// The night's score.
    pub fn night(seed: u64) -> Conductor {
        Conductor::new(seed, Tempo::Night)
    }

    /// The colony phase's bed, at half tempo.
    pub fn colony(seed: u64) -> Conductor {
        Conductor::new(seed, Tempo::Colony)
    }

    pub fn tempo(&self) -> Tempo {
        self.tempo
    }

    /// Seconds in a bar.
    pub fn bar_secs(&self) -> f64 {
        let bpm = match self.tempo {
            Tempo::Night => BPM,
            Tempo::Colony => COLONY_BPM,
        };
        BEATS * 60.0 / bpm
    }

    /// The chord of the bar playing now.
    pub fn chord(&self) -> &'static str {
        CYCLE[self.bar % CYCLE.len()]
    }

    /// The parts the next bar plays.
    pub fn parts(&self) -> Vec<Part> {
        match self.tempo {
            Tempo::Night => self.pressure.parts(),
            Tempo::Colony if self.hungry => vec![Part::Bass],
            Tempo::Colony => vec![Part::Bass, Part::Pad],
        }
    }

    /// Sets what the night is like. A part that joins or leaves does so at
    /// the next bar.
    pub fn set_pressure(&mut self, pressure: Pressure) {
        self.pressure = pressure;
    }

    /// A hungry colony's bed is the bass alone.
    pub fn set_hungry(&mut self, hungry: bool) {
        self.hungry = hungry;
    }

    pub fn started(&self) -> bool {
        self.playing
    }

    /// Bars played so far.
    pub fn bars(&self) -> usize {
        self.played.len()
    }

    /// How far into the bar, in beats.
    pub fn beat(&self) -> f64 {
        self.clock / self.bar_secs() * BEATS
    }

    fn pick(&mut self, part: Part, chord: &str) -> String {
        if self.tempo == Tempo::Colony {
            return colony_key(part, chord);
        }
        let count = part.variants();
        let slot = part.slot();
        let variant = if count <= 1 {
            0
        } else {
            loop {
                let v = self.rng.below(count as u32) as usize;
                if Some(v) != self.last[slot] {
                    break v;
                }
            }
        };
        self.last[slot] = Some(variant);
        score_key(part, chord, variant)
    }

    fn play_bar(&mut self, audio: &mut Audio) {
        for s in self.tail.drain(..) {
            audio.stop(s, TAIL_FADE_MS);
        }
        self.tail = std::mem::take(&mut self.now);
        let chord = self.chord();
        let keys: Vec<String> = self
            .parts()
            .into_iter()
            .map(|part| self.pick(part, chord))
            .collect();
        let starts: Vec<(&str, Bus, Start)> = keys
            .iter()
            .map(|k| (k.as_str(), Bus::MUSIC, Start::ONCE))
            .collect();
        self.now = audio.start_together(&starts);
        self.played.push(keys);
    }

    /// Starts the score from the top of the cycle.
    pub fn start(&mut self, audio: &mut Audio) {
        if self.playing {
            return;
        }
        self.playing = true;
        self.clock = 0.0;
        self.bar = 0;
        self.play_bar(audio);
    }

    /// `dt` seconds on: the next bar when this one ends.
    pub fn update(&mut self, audio: &mut Audio, dt: f64) {
        if !self.playing {
            return;
        }
        self.clock += dt;
        let secs = self.bar_secs();
        if self.clock >= secs {
            self.clock -= secs;
            self.bar = (self.bar + 1) % CYCLE.len();
            self.play_bar(audio);
        }
    }

    /// A stab on the bar's chord, straight away.
    pub fn stab(&mut self, audio: &mut Audio, stab: Stab) {
        if !self.playing {
            return;
        }
        let key = stab_key(stab, self.chord());
        audio.play(&key, Bus::MUSIC);
    }

    pub fn stop(&mut self, audio: &mut Audio, fade_ms: u32) {
        for s in self.tail.drain(..).chain(self.now.drain(..)) {
            audio.stop(s, fade_ms);
        }
        self.playing = false;
    }
}

/// How many bars a soundtrack performance runs, at least.
const PERFORMANCE_BARS: u32 = 28;

/// The score on Wicket's soundtrack: a night imagined, the raiders
/// coming nearer bar by bar, a charge or two, then the bed thins out and
/// the piece rings off.
pub struct Performance {
    conductor: Conductor,
    rng: Rng,
    bars: u32,
    /// Pressure by bar, planned up front.
    plan: Vec<Pressure>,
    /// The beat a stab last fell on, so one falls at most once a beat.
    last_stab_beat: Option<(usize, u32)>,
    ending: Option<f64>,
}

impl Performance {
    pub fn new(mut v: Variety) -> Performance {
        let seed = v.next_u64();
        let bars = PERFORMANCE_BARS + v.below(9);
        let mut rng = Rng::seeded(seed ^ 0x5c0e);
        let mut plan = Vec::with_capacity(bars as usize);
        let mut near: u8 = 6;
        for bar in 0..bars {
            // The raiders come nearer, then fall back once the night turns.
            if bar >= 4 && bar % 2 == 0 && near > 1 && bar < bars * 2 / 3 {
                near -= 1;
            }
            if bar >= bars * 2 / 3 && bar % 3 == 0 && near < 5 {
                near += 1;
            }
            let charge = bar >= 10 && bar < bars - 4 && rng.chance(0.3);
            plan.push(Pressure {
                near: (bar >= 2).then_some(near),
                charge,
                shield_down: false,
            });
        }
        Performance {
            conductor: Conductor::night(seed),
            rng,
            bars,
            plan,
            last_stab_beat: None,
            ending: None,
        }
    }

    fn pressure_now(&self) -> Pressure {
        let bar = self.conductor.bars().saturating_sub(1);
        self.plan.get(bar).copied().unwrap_or_default()
    }
}

impl Performer for Performance {
    fn start(&mut self, audio: &mut Audio) {
        self.conductor.set_pressure(self.plan[0]);
        self.conductor.start(audio);
    }

    fn update(&mut self, audio: &mut Audio, dt: f64) -> bool {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            return *left > 0.0;
        }
        self.conductor.update(audio, dt);
        if self.conductor.bars() as u32 >= self.bars {
            self.conductor.stop(audio, 2500);
            self.ending = Some(2.5);
            return true;
        }
        let pressure = self.pressure_now();
        self.conductor.set_pressure(pressure);
        // Stabs on the off beats once the raiders are close.
        let beat = self.conductor.beat().floor() as u32;
        let at = (self.conductor.bars(), beat);
        if pressure.near.is_some_and(|row| row <= PAD_ROW)
            && (beat == 1 || beat == 3)
            && self.last_stab_beat != Some(at)
        {
            self.last_stab_beat = Some(at);
            if self.rng.chance(0.35) {
                let stab = if self.rng.chance(0.5) {
                    Stab::Heavy
                } else {
                    Stab::Run
                };
                self.conductor.stab(audio, stab);
            }
        }
        true
    }

    fn stop(&mut self, audio: &mut Audio) {
        self.conductor.stop(audio, 600);
    }
}
