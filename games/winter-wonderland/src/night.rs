//! Midnight Countdown's rules: call and response on the beat, driven by `dt`.
//!
//! Ten phrases count the minutes down to midnight, then a final phrase for
//! the full orchestra. In each phrase the band plays a one-bar call, and
//! the player answers it in the next bar: the same instruments on the same
//! beats. Bells start alone; drums join at seven minutes to midnight and
//! horns at four. The party's energy rises with good answers and drains
//! with misses; if it empties, the party fizzles out before midnight.

use parlor_core::Rng;

/// Nights in the game, each a different party.
pub const NIGHTS: u32 = 5;

/// Phrases before the finale: ten minutes to midnight down to one.
pub const PHRASES: usize = 10;

/// The instruments the player answers with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Inst {
    Bell,
    Drum,
    Horn,
}

impl Inst {
    pub const ALL: [Inst; 3] = [Inst::Bell, Inst::Drum, Inst::Horn];

    pub fn name(self) -> &'static str {
        match self {
            Inst::Bell => "bell",
            Inst::Drum => "drum",
            Inst::Horn => "horn",
        }
    }
}

/// A pause in the music while a new part is introduced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intro {
    Bells,
    Drums,
    Horns,
    Orchestra,
}

/// The intro that comes before phrase `phrase`, if any.
pub fn intro_before(phrase: usize) -> Option<Intro> {
    match phrase {
        0 => Some(Intro::Bells),
        3 => Some(Intro::Drums),
        6 => Some(Intro::Horns),
        PHRASES => Some(Intro::Orchestra),
        _ => None,
    }
}

/// The instruments phrase `phrase` uses.
pub fn insts_for(phrase: usize) -> &'static [Inst] {
    match phrase {
        0..=2 => &[Inst::Bell],
        3..=5 => &[Inst::Bell, Inst::Drum],
        _ => &Inst::ALL,
    }
}

/// How many backing layers play in phrase `phrase` (1 to 4).
pub fn layers_for(phrase: usize) -> u8 {
    match phrase {
        0..=2 => 1,
        3..=5 => 2,
        6..=9 => 3,
        _ => 4,
    }
}

/// How one night plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NightSpec {
    pub level: u32,
    pub name: &'static str,
    pub bpm: f64,
    /// Off-beat notes allowed in one call.
    pub offbeats: u32,
    pub min_notes: u32,
    pub max_notes: u32,
    /// Seconds either side of a note for a perfect answer.
    pub perfect: f64,
    /// Seconds either side of a note for any answer at all.
    pub good: f64,
    /// Party energy a miss or a wrong instrument costs.
    pub drain: i32,
}

impl NightSpec {
    /// Night `level`, 1 to [`NIGHTS`].
    pub fn new(level: u32) -> NightSpec {
        let l = level.clamp(1, NIGHTS);
        let (name, bpm, offbeats, min_notes, max_notes, perfect, good) = match l {
            1 => ("the kitchen party", 84.0, 0, 2, 3, 0.08, 0.17),
            2 => ("the skating pond", 92.0, 0, 2, 4, 0.075, 0.16),
            3 => ("the village square", 100.0, 1, 3, 4, 0.07, 0.15),
            4 => ("the snowy rooftop", 108.0, 2, 3, 5, 0.065, 0.14),
            _ => ("the grand ballroom", 116.0, 2, 4, 5, 0.06, 0.13),
        };
        NightSpec {
            level: l,
            name,
            bpm,
            offbeats,
            min_notes,
            max_notes,
            perfect,
            good,
            drain: 8 + 2 * l as i32,
        }
    }

    pub fn beat_seconds(&self) -> f64 {
        60.0 / self.bpm
    }
}

/// One note the band plays, or the player owes, at an absolute beat.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub beat: f64,
    pub inst: Inst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judge {
    Perfect,
    Good,
}

/// Something that happened in the music, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum NightEvent {
    /// The music stops for a new part: the screen explains it.
    Intro(Intro),
    /// A backing beat: its place in the bar (0 to 3) and how full the
    /// backing is (1 to 4).
    Tick {
        beat_in_bar: u8,
        layer: u8,
    },
    /// The band plays a note of the call.
    Call(Inst),
    Hit {
        inst: Inst,
        judge: Judge,
        points: u64,
    },
    Wrong {
        expected: Inst,
        got: Inst,
    },
    Miss(Inst),
    /// A press in the answer bar with no note to answer.
    Stray,
    /// A press while the band is still calling: no harm done.
    Early,
    /// The combo multiplier went up.
    ComboUp(u32),
    /// A phrase was answered; `clean` when every note was hit.
    PhraseDone {
        clean: bool,
        minutes_left: u32,
    },
    /// Energy fell below a third.
    EnergyLow,
    /// Midnight, with the bonus earned.
    Midnight {
        bonus: u64,
    },
    /// Energy ran out.
    Fizzled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Waiting for the player to start the next part.
    Waiting(Intro),
    Playing,
    /// Over: true at midnight, false when the party fizzled.
    Done(bool),
}

struct Expect {
    note: Note,
    judged: bool,
}

/// One night's countdown.
pub struct Night {
    pub spec: NightSpec,
    /// Practice: energy never drains and nothing is saved.
    pub practice: bool,
    /// The monthly gala: every instrument plays from the first call, and
    /// the music never stops to bring one in.
    pub gala: bool,
    pub stage: Stage,
    /// Beats played since the night began (the clock stops while waiting).
    pub beat: f64,
    /// The phrase being played, 0 to [`PHRASES`] (the finale).
    pub phrase: usize,
    pub score: u64,
    pub combo: u32,
    pub best_combo: u32,
    /// Party energy, 0 to 100.
    pub energy: i32,
    pub perfects: u32,
    pub goods: u32,
    pub misses: u32,
    pub wrongs: u32,
    pub strays: u32,
    /// Notes owed so far, judged or not.
    pub notes: u32,
    /// The last call, as (beat in the bar from 0, instrument).
    pub last_call: Vec<(f64, Inst)>,
    phrase_start: f64,
    call_len: f64,
    calls: Vec<Note>,
    expects: Vec<Expect>,
    next_tick: f64,
    origin: f64,
    phrase_errors: u32,
    low_warned: bool,
    rng: Rng,
}

impl Night {
    pub fn new(level: u32, practice: bool, rng: Rng) -> Night {
        Night {
            spec: NightSpec::new(level),
            practice,
            gala: false,
            stage: Stage::Waiting(Intro::Bells),
            beat: 0.0,
            phrase: 0,
            score: 0,
            combo: 0,
            best_combo: 0,
            energy: 60,
            perfects: 0,
            goods: 0,
            misses: 0,
            wrongs: 0,
            strays: 0,
            notes: 0,
            last_call: Vec::new(),
            phrase_start: 0.0,
            call_len: 4.0,
            calls: Vec::new(),
            expects: Vec::new(),
            next_tick: 0.0,
            origin: 0.0,
            phrase_errors: 0,
            low_warned: false,
            rng,
        }
    }

    /// The combo multiplier: 1 to 4, up one every 8 notes in a row.
    pub fn multiplier(&self) -> u32 {
        (1 + self.combo / 8).min(4)
    }

    /// How many minutes to midnight the current phrase stands for.
    pub fn minutes_left(&self) -> u32 {
        (PHRASES - self.phrase.min(PHRASES)) as u32
    }

    /// Answers so far as a share of notes owed, 0 to 1.
    pub fn accuracy(&self) -> f64 {
        let judged = self.perfects + self.goods + self.misses + self.wrongs;
        if judged == 0 {
            return 1.0;
        }
        (self.perfects as f64 + 0.5 * self.goods as f64) / judged as f64
    }

    /// The instruments the player has met so far.
    pub fn known(&self) -> &'static [Inst] {
        if self.gala {
            return &Inst::ALL;
        }
        insts_for(self.phrase.min(PHRASES))
    }

    /// The intro before phrase `phrase` on this night: the gala has no
    /// drum or horn intros, since they play from the start.
    pub fn intro_before(&self, phrase: usize) -> Option<Intro> {
        match intro_before(phrase) {
            Some(Intro::Drums | Intro::Horns) if self.gala => None,
            other => other,
        }
    }

    /// Whether the band is calling (true) or the player answering (false).
    pub fn in_call(&self) -> bool {
        self.beat < self.phrase_start + self.call_len
    }

    /// The notes the player still owes this phrase, as seconds from now.
    pub fn owed(&self) -> Vec<(f64, Inst)> {
        let secs = self.spec.beat_seconds();
        self.expects
            .iter()
            .filter(|e| !e.judged)
            .map(|e| ((e.note.beat - self.beat) * secs, e.note.inst))
            .collect()
    }

    /// Starts the music after an intro, with a one-bar count-in.
    pub fn begin(&mut self) {
        if !matches!(self.stage, Stage::Waiting(_)) {
            return;
        }
        self.stage = Stage::Playing;
        self.origin = self.beat;
        self.next_tick = self.beat;
        let start = self.beat + 4.0;
        self.start_phrase(start);
    }

    fn start_phrase(&mut self, start: f64) {
        self.phrase_start = start;
        self.phrase_errors = 0;
        let finale = self.phrase == PHRASES;
        self.call_len = if finale { 8.0 } else { 4.0 };
        let (mut lo, mut hi) = if finale {
            (self.spec.min_notes + 3, self.spec.max_notes + 3)
        } else {
            (self.spec.min_notes, self.spec.max_notes)
        };
        if self.gala {
            // Room for all three instruments in every call.
            lo = lo.max(3);
            hi = hi.max(lo);
        }
        let n = self.rng.range(lo as i32, hi as i32) as usize;
        let bar = self.call_len as usize;
        let mut on: Vec<f64> = (0..bar).map(|b| b as f64).collect();
        self.rng.shuffle(&mut on);
        let mut off: Vec<f64> = (0..bar - 1).map(|b| b as f64 + 0.5).collect();
        self.rng.shuffle(&mut off);
        let offs = if self.phrase >= 2 {
            self.rng.below(self.spec.offbeats + 1) as usize
        } else {
            0
        };
        let mut beats: Vec<f64> = off.into_iter().take(offs.min(n - 1)).collect();
        let rest = n - beats.len();
        beats.extend(on.into_iter().take(rest));
        beats.sort_by(|a, b| a.partial_cmp(b).expect("finite beats"));
        let insts = if self.gala {
            &Inst::ALL
        } else {
            insts_for(self.phrase)
        };
        let mut call: Vec<(f64, Inst)> =
            beats.iter().map(|&b| (b, *self.rng.pick(insts))).collect();
        // A phrase that brings in an instrument always uses it, and the
        // finale uses every instrument.
        let must: &[Inst] = match self.phrase {
            0 if self.gala => &Inst::ALL,
            3 if !self.gala => &[Inst::Drum],
            6 if !self.gala => &[Inst::Horn],
            PHRASES => &Inst::ALL,
            _ => &[],
        };
        if must.iter().any(|inst| !call.iter().any(|(_, x)| x == inst)) {
            let mut slots: Vec<usize> = (0..call.len()).collect();
            self.rng.shuffle(&mut slots);
            for (k, inst) in must.iter().enumerate() {
                call[slots[k]].1 = *inst;
            }
        }
        self.calls = call
            .iter()
            .map(|&(b, inst)| Note {
                beat: start + b,
                inst,
            })
            .collect();
        self.expects = call
            .iter()
            .map(|&(b, inst)| Expect {
                note: Note {
                    beat: start + self.call_len + b,
                    inst,
                },
                judged: false,
            })
            .collect();
        self.notes += call.len() as u32;
        self.last_call = call;
    }

    /// Plays the current call again after a one-bar count-in, for when
    /// the player comes back from the pause menu mid-phrase. Notes already
    /// answered stay answered, and nothing is counted twice.
    pub fn replay_call(&mut self) {
        if self.stage != Stage::Playing || self.last_call.is_empty() {
            return;
        }
        let judged: Vec<bool> = self.expects.iter().map(|e| e.judged).collect();
        self.origin = self.beat;
        self.next_tick = self.beat;
        let start = self.beat + 4.0;
        self.phrase_start = start;
        self.calls = self
            .last_call
            .iter()
            .map(|&(b, inst)| Note {
                beat: start + b,
                inst,
            })
            .collect();
        self.expects = self
            .last_call
            .iter()
            .enumerate()
            .map(|(i, &(b, inst))| Expect {
                note: Note {
                    beat: start + self.call_len + b,
                    inst,
                },
                judged: judged.get(i).copied().unwrap_or(false),
            })
            .collect();
    }

    fn phrase_end(&self) -> f64 {
        self.phrase_start + 2.0 * self.call_len
    }

    fn window(&self, seconds: f64) -> f64 {
        seconds / self.spec.beat_seconds()
    }

    /// Runs the music for `dt` seconds.
    pub fn update(&mut self, dt: f64) -> Vec<NightEvent> {
        let mut out = Vec::new();
        if self.stage != Stage::Playing {
            return out;
        }
        self.beat += dt / self.spec.beat_seconds();
        let end = self.phrase_end();
        while self.next_tick <= self.beat && self.next_tick < end {
            let beat_in_bar = ((self.next_tick - self.origin).round() as i64).rem_euclid(4) as u8;
            out.push(NightEvent::Tick {
                beat_in_bar,
                layer: if self.gala {
                    layers_for(self.phrase).max(3)
                } else {
                    layers_for(self.phrase)
                },
            });
            self.next_tick += 1.0;
        }
        while let Some(note) = self.calls.first().copied() {
            if note.beat > self.beat {
                break;
            }
            self.calls.remove(0);
            out.push(NightEvent::Call(note.inst));
        }
        let late = self.window(self.spec.good);
        let now = self.beat;
        let overdue: Vec<usize> = self
            .expects
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.judged && now > e.note.beat + late)
            .map(|(i, _)| i)
            .collect();
        for i in overdue {
            self.expects[i].judged = true;
            let inst = self.expects[i].note.inst;
            self.misses += 1;
            self.fault();
            out.push(NightEvent::Miss(inst));
        }
        if self.check_energy(&mut out) {
            return out;
        }
        if self.beat >= end {
            self.finish_phrase(&mut out);
        }
        out
    }

    fn finish_phrase(&mut self, out: &mut Vec<NightEvent>) {
        let end = self.phrase_end();
        let clean = self.phrase_errors == 0;
        self.phrase += 1;
        self.expects.clear();
        self.calls.clear();
        if self.phrase > PHRASES {
            let bonus = self.energy.max(0) as u64 * 10 * self.spec.level as u64
                + (self.accuracy() * 1000.0).round() as u64 * self.spec.level as u64;
            self.score += bonus;
            self.stage = Stage::Done(true);
            out.push(NightEvent::PhraseDone {
                clean,
                minutes_left: 0,
            });
            out.push(NightEvent::Midnight { bonus });
            return;
        }
        out.push(NightEvent::PhraseDone {
            clean,
            minutes_left: self.minutes_left(),
        });
        match self.intro_before(self.phrase) {
            Some(intro) => {
                self.beat = end;
                self.stage = Stage::Waiting(intro);
                out.push(NightEvent::Intro(intro));
            }
            None => self.start_phrase(end),
        }
    }

    /// A miss, a wrong instrument or a stray press: the combo breaks.
    fn fault(&mut self) {
        self.combo = 0;
        self.phrase_errors += 1;
        if !self.practice {
            self.energy -= self.spec.drain;
        }
    }

    /// Ends the night when energy is gone. True once it has.
    fn check_energy(&mut self, out: &mut Vec<NightEvent>) -> bool {
        if self.energy <= 0 {
            self.energy = 0;
            self.stage = Stage::Done(false);
            self.calls.clear();
            self.expects.clear();
            out.push(NightEvent::Fizzled);
            return true;
        }
        if self.energy < 33 && !self.low_warned {
            self.low_warned = true;
            out.push(NightEvent::EnergyLow);
        } else if self.energy >= 50 {
            self.low_warned = false;
        }
        false
    }

    /// The player plays `inst`.
    pub fn press(&mut self, inst: Inst) -> Vec<NightEvent> {
        let mut out = Vec::new();
        if self.stage != Stage::Playing {
            return out;
        }
        let good = self.window(self.spec.good);
        let perfect = self.window(self.spec.perfect);
        let now = self.beat;
        let nearest = self
            .expects
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.judged && (e.note.beat - now).abs() <= good)
            .min_by(|(_, a), (_, b)| {
                let da = (a.note.beat - now).abs();
                let db = (b.note.beat - now).abs();
                da.partial_cmp(&db).expect("finite beats")
            })
            .map(|(i, _)| i);
        match nearest {
            Some(i) => {
                self.expects[i].judged = true;
                let note = self.expects[i].note;
                if note.inst != inst {
                    self.wrongs += 1;
                    self.fault();
                    out.push(NightEvent::Wrong {
                        expected: note.inst,
                        got: inst,
                    });
                } else {
                    let judge = if (note.beat - now).abs() <= perfect {
                        Judge::Perfect
                    } else {
                        Judge::Good
                    };
                    let before = self.multiplier();
                    let base = match judge {
                        Judge::Perfect => {
                            self.perfects += 1;
                            self.energy += 4;
                            100
                        }
                        Judge::Good => {
                            self.goods += 1;
                            self.energy += 2;
                            50
                        }
                    };
                    self.energy = self.energy.min(100);
                    let finale = if self.phrase == PHRASES { 2 } else { 1 };
                    let points = base * before as u64 * finale;
                    self.score += points;
                    self.combo += 1;
                    self.best_combo = self.best_combo.max(self.combo);
                    out.push(NightEvent::Hit {
                        inst,
                        judge,
                        points,
                    });
                    let after = self.multiplier();
                    if after > before {
                        out.push(NightEvent::ComboUp(after));
                    }
                }
            }
            None => {
                let answering = now >= self.phrase_start + self.call_len - good;
                // A second press on a note already answered does no harm.
                let answered = self
                    .expects
                    .iter()
                    .any(|e| e.judged && (e.note.beat - now).abs() <= good);
                if answering && !answered {
                    self.strays += 1;
                    self.combo = 0;
                    self.phrase_errors += 1;
                    if !self.practice {
                        self.energy -= 4;
                    }
                    out.push(NightEvent::Stray);
                } else {
                    out.push(NightEvent::Early);
                }
            }
        }
        self.check_energy(&mut out);
        out
    }
}

/// "bell on beat 1, drum on beat 2 and a half", for hints.
pub fn describe_call(call: &[(f64, Inst)]) -> String {
    let parts: Vec<String> = call
        .iter()
        .map(|&(b, inst)| {
            let whole = b.floor() as u32 + 1;
            if b.fract() > 0.25 {
                format!("{} on beat {whole} and a half", inst.name())
            } else {
                format!("{} on beat {whole}", inst.name())
            }
        })
        .collect();
    match parts.len() {
        0 => "nothing yet".to_string(),
        1 => parts[0].clone(),
        n => format!("{}, then {}", parts[..n - 1].join(", "), parts[n - 1]),
    }
}
