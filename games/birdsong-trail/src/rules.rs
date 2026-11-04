//! The game's rules as plain structs, driven by `dt` on a fixed step so
//! tests can play them exactly: a walk on a trail, the dawn chorus and
//! the migration count. No screens or sound here; each `update` returns
//! what happened, and the screens voice it.

use std::cell::Cell;

use parlor_core::{FixedStep, Rng, Season};

use crate::birds::{self, Note, Species};
use crate::guide::Guide;
use crate::trails::{Ground, Trail};

/// The simulation's step, 60 a second.
pub const TICK: f64 = 1.0 / 60.0;
/// Metres per step.
pub const STEP: f32 = 2.0;
/// A bird this close notices you and sings to you.
pub const NOTICE: f32 = 3.5;
/// A bird you walked away from notices you again once you are this far
/// off and come back.
pub const RELEASE: f32 = 6.0;
/// Seconds from one note of a song to the next.
pub const NOTE_GAP: f64 = 0.38;
/// The trail is open ground this far either side of the path.
pub const HALF_WIDTH: f32 = 9.0;
/// The path itself, this far either side of its middle.
pub const PATH: f32 = 1.5;
/// Close enough to the trail's end to have reached it.
pub const END_REACH: f32 = 2.5;
/// Seconds of quiet before a bird waiting for your answer sings again.
pub const NUDGE: f64 = 9.0;
/// The eight ways to face, a turn apart.
pub const WAYS: u8 = 8;

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Fixes the seed of every new run on this thread, for tests.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new run: the fixed one in tests, else the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}

/// "north", "north-east"...
pub fn compass(facing: u8) -> &'static str {
    match facing % WAYS {
        0 => "north",
        1 => "north-east",
        2 => "east",
        3 => "south-east",
        4 => "south",
        5 => "south-west",
        6 => "west",
        _ => "north-west",
    }
}

/// Where something is, from where you face, in eight sectors: 0 is
/// straight ahead, 2 to your right, 4 behind you, 6 to your left.
pub fn sector(facing: u8, from: (f32, f32), to: (f32, f32)) -> u8 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    if dx.abs() < 1e-3 && dy.abs() < 1e-3 {
        return 0;
    }
    let bearing = dx.atan2(dy).to_degrees();
    let rel = (bearing - facing as f32 * 45.0).rem_euclid(360.0);
    ((rel / 45.0).round() as u8) % WAYS
}

/// "straight ahead", "ahead and to your right"...
pub fn sector_words(sector: u8) -> &'static str {
    match sector % WAYS {
        0 => "straight ahead",
        1 => "ahead and to your right",
        2 => "to your right",
        3 => "behind you and to your right",
        4 => "behind you",
        5 => "behind you and to your left",
        6 => "to your left",
        _ => "ahead and to your left",
    }
}

/// How to turn to face a sector: "Turn right once." or "" when ahead.
pub fn turn_words(sector: u8) -> String {
    let times = |n: u8| match n {
        1 => "once".to_string(),
        2 => "twice".to_string(),
        n => format!("{n} times"),
    };
    match sector % WAYS {
        0 => String::new(),
        s @ 1..=3 => format!("Turn right {}.", times(s)),
        4 => "Turn around, 4 turns either way.".to_string(),
        s => format!("Turn left {}.", times(WAYS - s)),
    }
}

/// "about 8 metres" ("about 26 feet" with US units), "right beside you".
pub fn distance_words(metres: f32) -> String {
    if metres.round() <= 1.0 {
        return "right beside you".to_string();
    }
    format!("about {}", parlor_core::units::length(f64::from(metres)))
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// A song being sung, a note at a time.
#[derive(Clone, Debug, PartialEq)]
struct Song {
    notes: Vec<Note>,
    at: usize,
    clock: f64,
    /// Sung to the player, who answers it when it ends.
    close: bool,
}

impl Song {
    fn new(notes: Vec<Note>, close: bool) -> Song {
        Song {
            notes,
            at: 0,
            clock: 0.0,
            close,
        }
    }

    /// One step: the note that sounds now, if any, and whether the song
    /// is over.
    fn tick(&mut self, step: f64) -> (Option<Note>, bool) {
        self.clock -= step;
        if self.clock > 1e-9 {
            return (None, false);
        }
        if self.at < self.notes.len() {
            let note = self.notes[self.at];
            self.at += 1;
            self.clock += NOTE_GAP;
            (Some(note), false)
        } else {
            (None, true)
        }
    }

    fn seconds(notes: usize) -> f64 {
        notes as f64 * NOTE_GAP + 0.3
    }
}

/// One bird on a walk.
#[derive(Clone, Debug, PartialEq)]
pub struct Bird {
    pub species: &'static Species,
    /// Which verse it sings to you: 0 is its signature song.
    pub verse_no: u32,
    pub verse: Vec<Note>,
    pub pos: (f32, f32),
    pub friend: bool,
    /// Answers that missed.
    pub tries: u32,
    /// You walked away from it; it waits until you are well off.
    pub left: bool,
    wait: f64,
    song: Option<Song>,
}

impl Bird {
    pub fn singing(&self) -> bool {
        self.song.is_some()
    }
}

/// Where the walk is.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Walking,
    /// A bird is singing to you; answer when it ends.
    Listening(usize),
    /// A bird waits for your answer: the notes so far.
    Answering(usize, Vec<Note>),
}

/// What happened on a walk.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// A bird sang a note.
    Note {
        bird: usize,
        note: Note,
    },
    /// A bird noticed you and is singing to you.
    Noticed {
        bird: usize,
    },
    /// It has finished: your turn.
    Ready {
        bird: usize,
    },
    /// Your own note.
    Echo(Note),
    /// Your answer matched.
    Befriended {
        bird: usize,
    },
    /// Your answer did not match; it sings again.
    Missed {
        bird: usize,
        tries: u32,
    },
    /// A note while the bird still sings.
    Busy {
        bird: usize,
    },
    /// You turned or stepped away from a bird before answering.
    LeftBird {
        bird: usize,
    },
    /// A bird that waited a while for your answer sings again.
    Nudge {
        bird: usize,
    },
    Step(Ground),
    /// The edge of the trail, or with `end` its start or far end.
    Blocked {
        end: bool,
    },
    Turned(u8),
    ReachedEnd,
}

/// A walk on a trail.
#[derive(Clone, Debug)]
pub struct Walk {
    pub trail: Trail,
    pub season: Season,
    /// The practice glade: nothing goes in the guide.
    pub practice: bool,
    pub length: f32,
    pub birds: Vec<Bird>,
    pub x: f32,
    pub y: f32,
    pub facing: u8,
    pub phase: Phase,
    pub steps: u32,
    pub finished: bool,
    /// Seconds since your last note, while a bird waits for one.
    idle: f64,
    clock: FixedStep,
    rng: Rng,
}

/// The birds of a walk, with the verse each will sing: those with a song
/// still to learn first, in a shuffled order.
pub fn roster(
    trail: Trail,
    season: Season,
    guide: &Guide,
    rng: &mut Rng,
    n: usize,
) -> Vec<(&'static Species, u32)> {
    let mut all = birds::on_trail(trail.habitat(), season);
    rng.shuffle(&mut all);
    all.sort_by_key(|s| guide.verses_of(s) >= s.verses());
    all.into_iter()
        .take(n)
        .map(|s| (s, guide.next_verse(s)))
        .collect()
}

/// How many birds a walk holds: fewer in autumn, fewest in winter.
pub fn walk_size(season: Season) -> usize {
    match season {
        Season::Spring | Season::Summer => 6,
        Season::Autumn => 5,
        Season::Winter => 4,
    }
}

/// The seconds between a bird's songs, by season.
fn gap(season: Season, rng: &mut Rng) -> f64 {
    let (lo, hi) = if season == Season::Winter {
        (7.0, 13.0)
    } else {
        (3.5, 8.0)
    };
    lo + rng.unit() * (hi - lo)
}

impl Walk {
    /// A walk on `trail` with these birds, placed by the seed.
    pub fn new(
        trail: Trail,
        season: Season,
        roster: Vec<(&'static Species, u32)>,
        seed: u64,
    ) -> Walk {
        let mut rng = Rng::seeded(seed);
        let length = trail.length();
        let n = roster.len().max(1);
        let span = length - 16.0;
        let birds = roster
            .into_iter()
            .enumerate()
            .map(|(i, (species, verse_no))| {
                let y = if n == 1 {
                    length / 2.0
                } else {
                    8.0 + span * i as f32 / (n - 1) as f32
                } + (rng.unit() as f32 - 0.5) * 2.0;
                let side = if rng.chance(0.5) { -1.0 } else { 1.0 };
                let x = side * (3.0 + rng.unit() as f32 * 4.5);
                Bird {
                    species,
                    verse_no,
                    verse: species.verse(verse_no),
                    pos: (x, y),
                    friend: false,
                    tries: 0,
                    left: false,
                    wait: 1.0 + i as f64 * 1.3 + rng.unit() * 2.0,
                    song: None,
                }
            })
            .collect();
        Walk {
            trail,
            season,
            practice: false,
            length,
            birds,
            x: 0.0,
            y: 0.0,
            facing: 0,
            phase: Phase::Walking,
            steps: 0,
            finished: false,
            idle: 0.0,
            clock: FixedStep::new(TICK),
            rng,
        }
    }

    /// A walk on `trail` in `season`, with birds the guide still has songs
    /// to learn from first.
    pub fn on(trail: Trail, season: Season, guide: &Guide, seed: u64) -> Walk {
        let mut rng = Rng::seeded(seed ^ 0xB1_2D);
        let roster = roster(trail, season, guide, &mut rng, walk_size(season));
        Walk::new(trail, season, roster, seed)
    }

    /// The practice glade: a short path, one bird ahead and one to the
    /// right, each with a two-note song.
    pub fn practice() -> Walk {
        let ahead = Species::by_id("chickadee").unwrap_or(&birds::ALL[0]);
        let right = Species::by_id("grey-wagtail").unwrap_or(&birds::ALL[0]);
        let mut walk = Walk::new(
            Trail::Woodland,
            Season::Summer,
            vec![(ahead, 0), (right, 0)],
            7,
        );
        walk.length = 18.0;
        walk.practice = true;
        walk.birds[0].pos = (0.0, 5.0);
        walk.birds[1].pos = (5.0, 10.0);
        walk
    }

    pub fn pos(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    pub fn end_pos(&self) -> (f32, f32) {
        (0.0, self.length)
    }

    pub fn on_path(&self) -> bool {
        self.x.abs() <= PATH
    }

    pub fn ground(&self) -> Ground {
        self.trail.ground(self.on_path(), self.season)
    }

    pub fn friends(&self) -> usize {
        self.birds.iter().filter(|b| b.friend).count()
    }

    /// The bird singing to you or waiting for you, if any.
    pub fn focus(&self) -> Option<usize> {
        match &self.phase {
            Phase::Walking => None,
            Phase::Listening(i) | Phase::Answering(i, _) => Some(*i),
        }
    }

    /// The nearest bird you have not answered yet.
    pub fn nearest_new(&self) -> Option<usize> {
        self.birds
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.friend)
            .min_by(|(_, a), (_, b)| dist(a.pos, self.pos()).total_cmp(&dist(b.pos, self.pos())))
            .map(|(i, _)| i)
    }

    /// Where bird `i` is: its sector and distance.
    pub fn whereabouts(&self, i: usize) -> (u8, f32) {
        let pos = self.birds[i].pos;
        (sector(self.facing, self.pos(), pos), dist(self.pos(), pos))
    }

    /// Where the trail's end is: its sector and distance.
    pub fn end_whereabouts(&self) -> (u8, f32) {
        let end = self.end_pos();
        (sector(self.facing, self.pos(), end), dist(self.pos(), end))
    }

    /// Every bird's distance, nearest first.
    pub fn by_distance(&self) -> Vec<usize> {
        let mut all: Vec<usize> = (0..self.birds.len()).collect();
        all.sort_by(|&a, &b| {
            dist(self.birds[a].pos, self.pos()).total_cmp(&dist(self.birds[b].pos, self.pos()))
        });
        all
    }

    /// Walks away from the bird you are with, if any.
    fn leave_focus(&mut self, evs: &mut Vec<Ev>) {
        if let Some(i) = self.focus() {
            self.birds[i].left = true;
            self.birds[i].song = None;
            self.birds[i].wait = 2.0;
            self.phase = Phase::Walking;
            evs.push(Ev::LeftBird { bird: i });
        }
    }

    /// Turns one way to the right (`1`) or left (`-1`).
    pub fn turn(&mut self, way: i32) -> Vec<Ev> {
        let mut evs = Vec::new();
        if self.finished {
            return evs;
        }
        self.leave_focus(&mut evs);
        self.facing = (self.facing as i32 + way).rem_euclid(WAYS as i32) as u8;
        evs.push(Ev::Turned(self.facing));
        evs
    }

    /// One step forward, or back with `forward` false.
    pub fn step(&mut self, forward: bool) -> Vec<Ev> {
        let mut evs = Vec::new();
        if self.finished {
            return evs;
        }
        self.leave_focus(&mut evs);
        let angle = (self.facing as f32 * 45.0).to_radians();
        let sign = if forward { 1.0 } else { -1.0 };
        let (nx, ny) = (
            self.x + sign * STEP * angle.sin(),
            self.y + sign * STEP * angle.cos(),
        );
        if nx.abs() > HALF_WIDTH || ny < -0.5 || ny > self.length + 1.0 {
            evs.push(Ev::Blocked {
                end: nx.abs() <= HALF_WIDTH,
            });
            return evs;
        }
        self.x = nx;
        self.y = ny;
        self.steps += 1;
        evs.push(Ev::Step(self.ground()));
        let (_, to_end) = self.end_whereabouts();
        if to_end <= END_REACH + 0.5 {
            self.finished = true;
            evs.push(Ev::ReachedEnd);
            return evs;
        }
        self.notice(&mut evs);
        evs
    }

    /// The nearest new bird in reach notices you and sings to you.
    fn notice(&mut self, evs: &mut Vec<Ev>) {
        let here = self.pos();
        for b in &mut self.birds {
            if b.left && dist(b.pos, here) > RELEASE {
                b.left = false;
            }
        }
        let near = self
            .birds
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.friend && !b.left && dist(b.pos, here) <= NOTICE)
            .min_by(|(_, a), (_, b)| dist(a.pos, here).total_cmp(&dist(b.pos, here)))
            .map(|(i, _)| i);
        if let Some(i) = near {
            self.sing_to_you(i);
            evs.push(Ev::Noticed { bird: i });
        }
    }

    fn sing_to_you(&mut self, i: usize) {
        for (j, b) in self.birds.iter_mut().enumerate() {
            if j != i && b.song.as_ref().is_some_and(|s| !s.close) {
                b.song = None;
            }
        }
        let verse = self.birds[i].verse.clone();
        self.birds[i].song = Some(Song::new(verse, true));
        self.phase = Phase::Listening(i);
        self.idle = 0.0;
    }

    /// Holds back the song a bird has just begun to sing to you by `secs`,
    /// so a line can be heard first.
    pub fn hold_song(&mut self, secs: f64) {
        let Some(i) = self.focus() else {
            return;
        };
        if let Some(song) = self.birds[i].song.as_mut() {
            if song.close && song.at == 0 {
                song.clock = secs;
            }
        }
    }

    /// The bird you are with sings its song to you again.
    pub fn again(&mut self) -> bool {
        match self.focus() {
            Some(i) => {
                self.sing_to_you(i);
                true
            }
            None => false,
        }
    }

    /// One of your notes, answering the bird you are with.
    pub fn answer(&mut self, note: Note) -> Vec<Ev> {
        let mut evs = Vec::new();
        match &mut self.phase {
            Phase::Walking => {}
            Phase::Listening(i) => evs.push(Ev::Busy { bird: *i }),
            Phase::Answering(i, got) => {
                let i = *i;
                got.push(note);
                evs.push(Ev::Echo(note));
                self.idle = 0.0;
                let bird = &self.birds[i];
                if got.len() >= bird.verse.len() {
                    if *got == bird.verse {
                        let b = &mut self.birds[i];
                        b.friend = true;
                        b.wait = 2.5;
                        self.phase = Phase::Walking;
                        evs.push(Ev::Befriended { bird: i });
                    } else {
                        self.birds[i].tries += 1;
                        let tries = self.birds[i].tries;
                        evs.push(Ev::Missed { bird: i, tries });
                        self.sing_to_you(i);
                    }
                }
            }
        }
        evs
    }

    /// The notes answered so far, while a bird waits for you.
    pub fn answered(&self) -> usize {
        match &self.phase {
            Phase::Answering(_, got) => got.len(),
            _ => 0,
        }
    }

    /// Lets `dt` seconds pass.
    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        let mut evs = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |step| self.tick(step, &mut evs));
        self.clock = clock;
        evs
    }

    fn tick(&mut self, step: f64, evs: &mut Vec<Ev>) {
        if self.finished {
            return;
        }
        let hushed = self.phase != Phase::Walking;
        let season = self.season;
        for i in 0..self.birds.len() {
            let b = &mut self.birds[i];
            if let Some(song) = &mut b.song {
                let (note, over) = song.tick(step);
                if let Some(note) = note {
                    evs.push(Ev::Note { bird: i, note });
                }
                if over {
                    let close = song.close;
                    b.song = None;
                    b.wait = gap(season, &mut self.rng) * if b.friend { 1.6 } else { 1.0 };
                    if close && self.phase == Phase::Listening(i) {
                        self.phase = Phase::Answering(i, Vec::new());
                        self.idle = 0.0;
                        evs.push(Ev::Ready { bird: i });
                    }
                }
                continue;
            }
            if hushed {
                continue;
            }
            b.wait -= step;
            if b.wait <= 0.0 {
                let notes = if b.friend {
                    b.verse.clone()
                } else {
                    b.species.notes()
                };
                b.song = Some(Song::new(notes, false));
            }
        }
        if let Phase::Answering(i, _) = self.phase {
            self.idle += step;
            if self.idle >= NUDGE {
                self.sing_to_you(i);
                evs.push(Ev::Nudge { bird: i });
            }
        }
    }

    /// A challenge score: 3 for each bird answered first time, 2 after one
    /// miss, 1 after more.
    pub fn score(&self) -> u32 {
        self.birds
            .iter()
            .filter(|b| b.friend)
            .map(|b| match b.tries {
                0 => 3,
                1 => 2,
                _ => 1,
            })
            .sum()
    }
}

// The dawn chorus.

/// Places around you in the chorus, one a turn apart.
pub const CHORUS_PLACES: usize = 8;
/// How far off the chorus birds sing.
pub const CHORUS_RADIUS: f32 = 6.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Singer {
    pub species: &'static Species,
    wait: f64,
    song: Option<Song>,
}

/// What happened in the chorus.
#[derive(Clone, Debug, PartialEq)]
pub enum ChorusEv {
    /// A bird in place `place` sang a note.
    Note {
        place: usize,
        note: Note,
    },
    /// The bird to find sang alone, from no place, so you know its song.
    Solo {
        note: Note,
    },
    /// A new bird to find.
    Target(&'static Species),
    Found {
        species: &'static Species,
        first_try: bool,
    },
    /// You picked a place that holds another bird, or none.
    Wrong {
        got: Option<&'static Species>,
        place_of_target: usize,
    },
    Turned(u8),
    /// A new bird flew in to a place.
    Arrived {
        place: usize,
    },
    Finished,
}

/// The dawn chorus: every bird sings at once from its place around you,
/// and you pick out the one asked for by facing it.
#[derive(Clone, Debug)]
pub struct Chorus {
    pub places: Vec<Option<Singer>>,
    /// Birds waiting to fly in as places empty.
    pub waiting: Vec<&'static Species>,
    pub target: Option<usize>,
    pub facing: u8,
    pub found: Vec<&'static Species>,
    pub first_tries: u32,
    pub total: usize,
    pub finished: bool,
    missed: bool,
    solo: Option<Song>,
    /// Seconds the chorus stays quiet for the solo.
    hush: f64,
    clock: FixedStep,
    rng: Rng,
}

impl Chorus {
    /// A chorus of these birds, the first eight in place.
    pub fn new(mut birds: Vec<&'static Species>, seed: u64) -> Chorus {
        let mut rng = Rng::seeded(seed);
        rng.shuffle(&mut birds);
        let total = birds.len();
        let mut waiting: Vec<&'static Species> = birds.into_iter().rev().collect();
        let mut places = Vec::new();
        for i in 0..CHORUS_PLACES {
            places.push(waiting.pop().map(|species| Singer {
                species,
                wait: 0.4 + i as f64 * 0.35 + rng.unit(),
                song: None,
            }));
        }
        let mut chorus = Chorus {
            places,
            waiting,
            target: None,
            facing: 0,
            found: Vec::new(),
            first_tries: 0,
            total,
            finished: total == 0,
            missed: false,
            solo: None,
            hush: 0.0,
            clock: FixedStep::new(TICK),
            rng,
        };
        chorus.pick_target();
        chorus
    }

    /// The chorus of a season: every singing bird of every trail.
    pub fn of_season(season: Season, seed: u64) -> Chorus {
        Chorus::new(birds::of_season(season), seed)
    }

    fn pick_target(&mut self) {
        let filled: Vec<usize> = (0..CHORUS_PLACES)
            .filter(|&i| self.places[i].is_some())
            .collect();
        if filled.is_empty() {
            self.target = None;
            self.finished = true;
            return;
        }
        self.target = Some(*self.rng.pick(&filled));
        self.missed = false;
        self.play_solo();
    }

    /// The bird to find sings alone, and the chorus hushes for it.
    pub fn play_solo(&mut self) {
        if let Some(t) = self.target_species() {
            let notes = t.notes();
            self.hush = Song::seconds(notes.len()) + 0.4;
            self.solo = Some(Song::new(notes, false));
        }
    }

    pub fn target_species(&self) -> Option<&'static Species> {
        self.target
            .and_then(|t| self.places[t].as_ref())
            .map(|s| s.species)
    }

    /// The direction of place `i` from where you face.
    pub fn sector_of(&self, place: usize) -> u8 {
        (place as u8 + WAYS - self.facing % WAYS) % WAYS
    }

    /// Where place `i` is, for the sound.
    pub fn place_pos(place: usize) -> (f32, f32) {
        let a = (place as f32 * 45.0).to_radians();
        (CHORUS_RADIUS * a.sin(), CHORUS_RADIUS * a.cos())
    }

    pub fn turn(&mut self, way: i32) -> Vec<ChorusEv> {
        self.facing = (self.facing as i32 + way).rem_euclid(WAYS as i32) as u8;
        vec![ChorusEv::Turned(self.facing)]
    }

    /// Picks the bird you face.
    pub fn pick(&mut self) -> Vec<ChorusEv> {
        let mut evs = Vec::new();
        let Some(target) = self.target else {
            return evs;
        };
        let here = self.facing as usize % CHORUS_PLACES;
        if here == target {
            let species = self.places[here].as_ref().map(|s| s.species);
            let Some(species) = species else {
                return evs;
            };
            let first_try = !self.missed;
            if first_try {
                self.first_tries += 1;
            }
            self.found.push(species);
            evs.push(ChorusEv::Found { species, first_try });
            self.places[here] = self.waiting.pop().map(|species| Singer {
                species,
                wait: 1.5,
                song: None,
            });
            if self.places[here].is_some() {
                evs.push(ChorusEv::Arrived { place: here });
            }
            self.pick_target();
            match self.target_species() {
                Some(t) => evs.push(ChorusEv::Target(t)),
                None => evs.push(ChorusEv::Finished),
            }
        } else {
            self.missed = true;
            evs.push(ChorusEv::Wrong {
                got: self.places[here].as_ref().map(|s| s.species),
                place_of_target: target,
            });
        }
        evs
    }

    pub fn update(&mut self, dt: f64) -> Vec<ChorusEv> {
        let mut evs = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |step| self.tick(step, &mut evs));
        self.clock = clock;
        evs
    }

    fn tick(&mut self, step: f64, evs: &mut Vec<ChorusEv>) {
        if self.finished {
            return;
        }
        if let Some(solo) = &mut self.solo {
            let (note, over) = solo.tick(step);
            if let Some(note) = note {
                evs.push(ChorusEv::Solo { note });
            }
            if over {
                self.solo = None;
            }
        }
        if self.hush > 0.0 {
            self.hush -= step;
            return;
        }
        for (place, singer) in self.places.iter_mut().enumerate() {
            let Some(s) = singer else { continue };
            if let Some(song) = &mut s.song {
                let (note, over) = song.tick(step);
                if let Some(note) = note {
                    evs.push(ChorusEv::Note { place, note });
                }
                if over {
                    s.song = None;
                    s.wait = 1.8 + self.rng.unit() * 2.4;
                }
                continue;
            }
            s.wait -= step;
            if s.wait <= 0.0 {
                s.song = Some(Song::new(s.species.notes(), false));
            }
        }
    }

    /// A challenge score: 2 for each bird found first time, 1 otherwise.
    pub fn score(&self) -> u32 {
        self.first_tries * 2 + (self.found.len() as u32 - self.first_tries)
    }
}

// The migration count.

/// The highest count you can give.
pub const MOST: u32 = 30;

/// One flock passing overhead.
#[derive(Clone, Debug, PartialEq)]
pub struct Flock {
    pub species: &'static Species,
    pub size: u32,
    /// When each bird calls, in seconds from the start of the pass.
    pub calls: Vec<f64>,
    /// How far each bird is to the side.
    pub sides: Vec<f32>,
}

impl Flock {
    pub fn new(species: &'static Species, size: u32, rng: &mut Rng) -> Flock {
        let gap = 0.5 + rng.unit() * 0.15;
        let calls = (0..size)
            .map(|i| 0.6 + i as f64 * gap + rng.unit() * 0.12)
            .collect();
        let sides = (0..size).map(|_| (rng.unit() as f32 - 0.5) * 8.0).collect();
        Flock {
            species,
            size,
            calls,
            sides,
        }
    }

    /// How long the pass lasts.
    pub fn seconds(&self) -> f64 {
        self.calls.last().copied().unwrap_or(0.0) + 1.0
    }
}

/// The flocks of a count, smaller first.
pub fn flocks(n: usize, smallest: u32, largest: u32, rng: &mut Rng) -> Vec<Flock> {
    let kinds = birds::flocks();
    (0..n)
        .map(|i| {
            let span = (largest - smallest) as f64;
            let base = smallest as f64 + span * i as f64 / (n.max(2) - 1) as f64;
            let size = (base + rng.unit() * 2.0 - 1.0)
                .round()
                .clamp(smallest as f64, largest as f64) as u32;
            let species = *rng.pick(&kinds);
            Flock::new(species, size, rng)
        })
        .collect()
}

/// Where the count is.
#[derive(Clone, Debug, PartialEq)]
pub enum CountPhase {
    /// Quiet before the next flock.
    Waiting(f64),
    /// A flock overhead, seconds into its pass.
    Passing(f64),
    /// Waiting for your count.
    Asking,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CountEv {
    /// A flock is coming: its wingbeats.
    Coming(&'static Species),
    /// One bird of it called, from here: x to the side, y ahead, z up.
    Call {
        species: &'static Species,
        pos: (f32, f32, f32),
    },
    /// The flock has passed: how many?
    Ask,
    Guess(u32),
    Judged {
        size: u32,
        guess: u32,
        points: u32,
    },
    Finished,
}

/// The migration count: flocks pass, and you count each one.
#[derive(Clone, Debug)]
pub struct Count {
    pub flocks: Vec<Flock>,
    pub at: usize,
    pub phase: CountPhase,
    pub guess: u32,
    /// Your count for each flock so far.
    pub guesses: Vec<u32>,
    /// Heading north: from behind you to ahead. South is the other way.
    pub north: bool,
    pub replays: u32,
    pub finished: bool,
    next_call: usize,
    clock: FixedStep,
}

/// Points for a count: 3 exact, 1 off by one.
pub fn points(size: u32, guess: u32) -> u32 {
    match size.abs_diff(guess) {
        0 => 3,
        1 => 1,
        _ => 0,
    }
}

/// The highest score a count can reach.
pub fn best_possible(flocks: usize) -> u32 {
    flocks as u32 * 3
}

impl Count {
    pub fn new(flocks: Vec<Flock>, north: bool) -> Count {
        Count {
            flocks,
            at: 0,
            phase: CountPhase::Waiting(1.5),
            guess: 1,
            guesses: Vec::new(),
            north,
            replays: 0,
            finished: false,
            next_call: 0,
            clock: FixedStep::new(TICK),
        }
    }

    /// The equinox count: 6 flocks of 3 to 10.
    pub fn equinox(north: bool, seed: u64) -> Count {
        let mut rng = Rng::seeded(seed);
        Count::new(flocks(6, 3, 10, &mut rng), north)
    }

    pub fn flock(&self) -> Option<&Flock> {
        self.flocks.get(self.at)
    }

    pub fn score(&self) -> u32 {
        self.guesses
            .iter()
            .zip(&self.flocks)
            .map(|(g, f)| points(f.size, *g))
            .sum()
    }

    pub fn exact(&self) -> u32 {
        self.guesses
            .iter()
            .zip(&self.flocks)
            .filter(|(g, f)| **g == f.size)
            .count() as u32
    }

    /// Changes the count by `by`, while asking.
    pub fn adjust(&mut self, by: i32) -> Vec<CountEv> {
        if self.phase != CountPhase::Asking {
            return Vec::new();
        }
        self.guess = (self.guess as i32 + by).clamp(1, MOST as i32) as u32;
        vec![CountEv::Guess(self.guess)]
    }

    /// Gives your count for the flock that passed.
    pub fn confirm(&mut self) -> Vec<CountEv> {
        if self.phase != CountPhase::Asking {
            return Vec::new();
        }
        let Some(size) = self.flock().map(|f| f.size) else {
            return Vec::new();
        };
        let guess = self.guess;
        self.guesses.push(guess);
        let mut evs = vec![CountEv::Judged {
            size,
            guess,
            points: points(size, guess),
        }];
        self.at += 1;
        self.guess = 1;
        if self.at >= self.flocks.len() {
            self.finished = true;
            evs.push(CountEv::Finished);
        } else {
            self.phase = CountPhase::Waiting(2.5);
        }
        evs
    }

    /// The flock that just passed flies over again.
    pub fn replay(&mut self) -> Vec<CountEv> {
        if self.phase != CountPhase::Asking {
            return Vec::new();
        }
        self.replays += 1;
        self.phase = CountPhase::Waiting(0.3);
        Vec::new()
    }

    pub fn update(&mut self, dt: f64) -> Vec<CountEv> {
        let mut evs = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |step| self.tick(step, &mut evs));
        self.clock = clock;
        evs
    }

    fn tick(&mut self, step: f64, evs: &mut Vec<CountEv>) {
        if self.finished {
            return;
        }
        match self.phase {
            CountPhase::Asking => {}
            CountPhase::Waiting(left) => {
                let left = left - step;
                if left <= 0.0 {
                    if let Some(f) = self.flock() {
                        evs.push(CountEv::Coming(f.species));
                    }
                    self.next_call = 0;
                    self.phase = CountPhase::Passing(0.0);
                } else {
                    self.phase = CountPhase::Waiting(left);
                }
            }
            CountPhase::Passing(t) => {
                let t = t + step;
                let Some(f) = self.flocks.get(self.at) else {
                    return;
                };
                let total = f.seconds();
                while self.next_call < f.calls.len() && f.calls[self.next_call] <= t + 1e-9 {
                    let i = self.next_call;
                    let along = (f.calls[i] / total) as f32 * 36.0 - 18.0;
                    let y = if self.north { along } else { -along };
                    evs.push(CountEv::Call {
                        species: f.species,
                        pos: (f.sides[i], y, 8.0),
                    });
                    self.next_call += 1;
                }
                if t >= total {
                    self.phase = CountPhase::Asking;
                    evs.push(CountEv::Ask);
                } else {
                    self.phase = CountPhase::Passing(t);
                }
            }
        }
    }
}
