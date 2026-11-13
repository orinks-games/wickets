//! A night under the stars, as plain rules: the listening point, tracing
//! constellations, wonders, the moon, hints and the guide tone, and the
//! twinkling, all driven by `update(dt)` on a fixed step so a night sounds
//! the same at any frame rate.

use std::collections::{BTreeMap, BTreeSet};

use hearken::text::sentence_start;
use parlor_core::{FixedStep, Rng, Season};

use crate::almanac::FullMoon;
use crate::sky::{
    self, angle, az_delta, brightness, compass, open_tier, season_constellations, season_wonders,
    wrap_az, Body, Sky, CONSTELLATIONS, STEP_DEGREES, WONDERS, ZENITH,
};
use crate::sound;

/// How far from the point a star can be heard twinkling, in degrees.
pub const AUDIBLE: f64 = 30.0;
/// How far from the point an unfound wonder's shimmer reaches, in degrees.
pub const SHIMMER_RANGE: f64 = 12.0;
/// Where the point starts: due south, 40 degrees up.
pub const START: (i32, i32) = (36, 8);
/// Points for each star traced, each constellation finished and each
/// wonder found, and what a hint costs, in challenges.
pub const STAR_POINTS: u64 = 10;
pub const PATTERN_POINTS: u64 = 100;
pub const WONDER_POINTS: u64 = 150;
pub const HINT_COST: u64 = 10;
/// A star hunt: points for each star found, less per hint, never under the floor.
pub const HUNT_POINTS: u64 = 100;
pub const HUNT_HINT_COST: u64 = 25;
pub const HUNT_FLOOR: u64 = 50;

/// What a night asks of the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Goal {
    /// Trace whatever is up, for the journal. No end but heading home.
    Free,
    /// Find these stars, one after another: (constellation, star).
    Stars(Vec<(usize, usize)>),
    /// Trace these constellations.
    Patterns(Vec<usize>),
    /// Trace every constellation of the season and find every wonder.
    Tour,
}

/// A way the point moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// What the night asks the screen to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Cue {
    /// A sound from the sky. `pan` is left to right and `ahead` is front to
    /// back, from the point's view; `pitch` is higher for things above it.
    Sound {
        key: &'static str,
        pan: f32,
        ahead: f32,
        gain: f32,
        pitch: f32,
    },
    /// Said now, interrupting.
    Say(String),
    /// Said after what is speaking.
    Queue(String),
    /// A constellation was traced to its last star.
    Traced(usize),
    /// A wonder was found for the first time tonight.
    Found(usize),
    /// The full moon was greeted.
    Greeted,
    /// Every goal of a challenge is met.
    Complete,
}

/// The star or place a hint or the guide tone points to.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub at: (i32, i32),
    /// "Bellatrix, next in Orion".
    pub words: String,
}

fn plain(key: &'static str, gain: f32, pitch: f32) -> Cue {
    Cue::Sound {
        key,
        pan: 0.0,
        ahead: 1.0,
        gain,
        pitch,
    }
}

/// Where a sound at `at` comes from, heard from `point`: (pan, ahead,
/// pitch). Pan follows the azimuth, and pitch rises gently, a third of an
/// octave for every 30 degrees above the point.
pub fn voice(point: (i32, i32), at: (i32, i32)) -> (f32, f32, f32) {
    let daz = (az_delta(point.0, at.0) * STEP_DEGREES) as f64;
    let (pan, ahead) = if point.1 >= ZENITH {
        (0.0, 1.0)
    } else {
        let r = daz.to_radians();
        (r.sin(), r.cos())
    };
    let pitch = 2f64.powf(((at.1 - point.1) * STEP_DEGREES) as f64 / 90.0);
    (pan as f32, ahead as f32, pitch as f32)
}

/// "3 steps right, 1 step up", the keys' way to a place from `from`.
pub fn steps_words(from: (i32, i32), to: (i32, i32)) -> String {
    let daz = if from.1 >= ZENITH || to.1 >= ZENITH {
        0
    } else {
        az_delta(from.0, to.0)
    };
    let dalt = to.1 - from.1;
    let part = |n: i32, pos: &str, neg: &str| -> Option<String> {
        match n {
            0 => None,
            1 => Some(format!("1 step {pos}")),
            -1 => Some(format!("1 step {neg}")),
            n if n > 0 => Some(format!("{n} steps {pos}")),
            n => Some(format!("{} steps {neg}", -n)),
        }
    };
    let parts: Vec<String> = [part(daz, "right", "left"), part(dalt, "up", "down")]
        .into_iter()
        .flatten()
        .collect();
    if parts.is_empty() {
        "right here".to_string()
    } else {
        parts.join(", ")
    }
}

/// "south, 40 degrees up", or "straight overhead".
pub fn place_words(at: (i32, i32)) -> String {
    if at.1 >= ZENITH {
        "straight overhead".to_string()
    } else if at.1 == 0 {
        format!("{}, on the horizon", compass(at.0))
    } else {
        format!("{}, {} degrees up", compass(at.0), at.1 * STEP_DEGREES)
    }
}

/// A star's name and constellation: "Betelgeuse, in Orion".
pub fn star_words(con: usize, star: usize) -> String {
    let def = &CONSTELLATIONS[con];
    format!("{}, in {}", sentence_start(def.stars[star].name), def.name)
}

/// One night on the hill.
pub struct Night {
    pub sky: Sky,
    pub az: i32,
    pub alt: i32,
    pub goal: Goal,
    /// Constellations that can be traced tonight.
    pub targets: Vec<usize>,
    /// Stars traced so far, by constellation.
    pub traced: BTreeMap<usize, usize>,
    /// Constellations traced tonight, in order.
    pub done: Vec<usize>,
    /// Wonders found tonight.
    pub found: Vec<usize>,
    pub greeted: bool,
    pub moon: Option<FullMoon>,
    /// Constellations in the journal, with tonight's.
    pub logged: BTreeSet<usize>,
    /// Constellations written into the journal for the first time tonight.
    pub newly: Vec<usize>,
    pub guide: bool,
    pub hints: u32,
    pub points: u64,
    /// The star hunt: stars found, and the hints and guide on this one.
    pub hunted: usize,
    hunt_hints: u32,
    hunt_guided: bool,
    pub shimmer_told: bool,
    /// The main action's key name, for spoken lines: "Space".
    pub primary_key: String,
    complete: bool,
    guide_timer: f64,
    shimmer_timer: f64,
    clock: FixedStep,
    rng: Rng,
}

impl Night {
    fn build(
        season: Season,
        up: Vec<usize>,
        goal: Goal,
        logged: BTreeSet<usize>,
        moon: Option<FullMoon>,
        seed: u64,
    ) -> Night {
        let mut rng = Rng::seeded(seed);
        let sky = Sky::new(season, &up, moon.is_some(), &mut rng);
        let targets = match &goal {
            Goal::Free | Goal::Tour => up.clone(),
            Goal::Patterns(list) => list.clone(),
            Goal::Stars(_) => Vec::new(),
        };
        Night {
            sky,
            az: START.0,
            alt: START.1,
            goal,
            targets,
            traced: BTreeMap::new(),
            done: Vec::new(),
            found: Vec::new(),
            greeted: false,
            moon,
            logged,
            newly: Vec::new(),
            guide: false,
            hints: 0,
            points: 0,
            hunted: 0,
            hunt_hints: 0,
            hunt_guided: false,
            shimmer_told: false,
            primary_key: "the main action".to_string(),
            complete: false,
            guide_timer: 0.0,
            shimmer_timer: 1.0,
            clock: FixedStep::hz(60.0),
            rng,
        }
    }

    /// A free night: the season's constellations the journal has opened.
    pub fn free(
        season: Season,
        logged: BTreeSet<usize>,
        moon: Option<FullMoon>,
        seed: u64,
    ) -> Night {
        let tier = open_tier(logged.len());
        let up = season_constellations(season)
            .into_iter()
            .filter(|&c| CONSTELLATIONS[c].tier <= tier)
            .collect();
        Night::build(season, up, Goal::Free, logged, moon, seed)
    }

    /// A challenge night: the season's whole sky, with `goal`.
    pub fn challenge(
        season: Season,
        goal: Goal,
        logged: BTreeSet<usize>,
        moon: Option<FullMoon>,
        seed: u64,
    ) -> Night {
        let up = season_constellations(season);
        Night::build(season, up, goal, logged, moon, seed)
    }

    pub fn season(&self) -> Season {
        self.sky.season
    }

    pub fn point(&self) -> (i32, i32) {
        (self.az, self.alt)
    }

    pub fn is_free(&self) -> bool {
        self.goal == Goal::Free
    }

    /// Whether a challenge's goals are all met.
    pub fn complete(&self) -> bool {
        self.complete
    }

    fn body_here(&self) -> Option<Body> {
        self.sky.object_at(self.point()).map(|o| o.body)
    }

    fn traced_of(&self, con: usize) -> usize {
        self.traced.get(&con).copied().unwrap_or(0)
    }

    fn finished(&self, con: usize) -> bool {
        self.traced_of(con) >= CONSTELLATIONS[con].stars.len()
    }

    /// The star being hunted, in a star hunt.
    pub fn hunt_target(&self) -> Option<(usize, usize)> {
        match &self.goal {
            Goal::Stars(list) => list.get(self.hunted).copied(),
            _ => None,
        }
    }

    /// What resting on the point's step says, if anything is there.
    pub fn rest_words(&self) -> Option<String> {
        let body = self.body_here()?;
        Some(match body {
            Body::Star { con, star } => {
                let name = sentence_start(CONSTELLATIONS[con].stars[star].name);
                let cname = CONSTELLATIONS[con].name;
                if self.hunt_target() == Some((con, star)) {
                    format!("{name}, in {cname}, the star you are looking for.")
                } else if !self.targets.contains(&con) {
                    format!("{name}, in {cname}.")
                } else if self.finished(con) {
                    format!("{name}, in {cname}, traced.")
                } else if self.traced_of(con) == star {
                    format!("{name}, next in {cname}.")
                } else {
                    format!("{name}, in {cname}.")
                }
            }
            Body::Faint => "A faint star.".to_string(),
            Body::Wonder(w) => {
                if self.found.contains(&w) {
                    format!("{}.", sentence_start(WONDERS[w].name))
                } else {
                    "A soft glow.".to_string()
                }
            }
            Body::Moon => match self.moon {
                Some(m) => format!("The full moon, {}.", m.name),
                None => "The moon.".to_string(),
            },
        })
    }

    /// The chime a thing in the sky rings with.
    fn chime(body: Body, mag: f32) -> &'static str {
        match body {
            Body::Star { con, star } if mag >= 1.0 => sound::CHIMES[(con * 7 + star) % 3],
            Body::Star { .. } => sound::BRIGHT,
            Body::Faint => sound::FAINT,
            Body::Wonder(_) => sound::SHIMMER,
            Body::Moon => sound::MOON,
        }
    }

    /// Moves the point one step.
    pub fn step(&mut self, dir: Dir) -> Vec<Cue> {
        let mut out = Vec::new();
        let (az, alt) = match dir {
            Dir::Left => (wrap_az(self.az - 1), self.alt),
            Dir::Right => (wrap_az(self.az + 1), self.alt),
            Dir::Up => (self.az, self.alt + 1),
            Dir::Down => (self.az, self.alt - 1),
        };
        if alt < 0 {
            out.push(plain(sound::EDGE, 0.7, 0.8));
            out.push(Cue::Say(
                "The horizon. The point goes no lower.".to_string(),
            ));
            return out;
        }
        if alt > ZENITH {
            out.push(plain(sound::EDGE, 0.7, 1.2));
            out.push(Cue::Say(
                "Straight overhead. The point goes no higher.".to_string(),
            ));
            return out;
        }
        self.az = az;
        self.alt = alt;
        let pitch = 0.85 + 0.3 * alt as f32 / ZENITH as f32;
        out.push(plain(sound::STEP, 0.5, pitch));
        if let Some(obj) = self.sky.object_at(self.point()) {
            let key = Night::chime(obj.body, obj.mag);
            let gain = brightness(obj.mag).max(0.55);
            out.push(plain(key, gain, 1.0));
        }
        if let Some(words) = self.rest_words() {
            out.push(Cue::Say(words));
        } else if alt == ZENITH && matches!(dir, Dir::Up) {
            out.push(Cue::Say("Straight overhead.".to_string()));
        } else if self.az % 18 == 0 && matches!(dir, Dir::Left | Dir::Right) {
            out.push(Cue::Say(format!("{}.", sentence_start(compass(self.az)))));
        }
        out
    }

    /// The main action on the point's step: trace a star, find a wonder,
    /// greet the moon.
    pub fn primary(&mut self) -> Vec<Cue> {
        let mut out = Vec::new();
        let Some(body) = self.body_here() else {
            out.push(plain(sound::NOTHING, 0.6, 1.0));
            out.push(Cue::Say("Empty sky.".to_string()));
            return out;
        };
        if let Some(target) = self.hunt_target() {
            self.hunt(body, target, &mut out);
            return out;
        }
        match body {
            Body::Star { con, star } => self.trace(con, star, &mut out),
            Body::Faint => {
                out.push(plain(sound::NOT_YET, 0.6, 1.0));
                out.push(Cue::Say("A faint star, no pattern.".to_string()));
            }
            Body::Wonder(w) => self.find(w, &mut out),
            Body::Moon => self.greet(&mut out),
        }
        self.check_complete(&mut out);
        out
    }

    fn hunt(&mut self, body: Body, target: (usize, usize), out: &mut Vec<Cue>) {
        if body
            == (Body::Star {
                con: target.0,
                star: target.1,
            })
        {
            let mut points = HUNT_POINTS
                .saturating_sub(HUNT_HINT_COST * self.hunt_hints as u64)
                .max(HUNT_FLOOR);
            if self.hunt_guided {
                points = points.min(HUNT_FLOOR);
            }
            self.points += points;
            self.hunted += 1;
            self.hunt_hints = 0;
            self.hunt_guided = self.guide;
            out.push(plain(sound::FOUND_STAR, 0.9, 1.0));
            let name = sentence_start(CONSTELLATIONS[target.0].stars[target.1].name);
            out.push(Cue::Say(format!("Found: {name}. {points} points.")));
            match self.hunt_target() {
                Some((c, s)) => out.push(Cue::Queue(format!("Next, find {}.", star_words(c, s)))),
                None => {
                    self.complete = true;
                    out.push(Cue::Complete);
                }
            }
            return;
        }
        out.push(plain(sound::NOT_YET, 0.6, 1.0));
        let here = match body {
            Body::Star { con, star } => format!("That is {}.", star_words(con, star)),
            Body::Faint => "That is a faint star.".to_string(),
            Body::Wonder(_) => "That is a soft glow, not a star.".to_string(),
            Body::Moon => "That is the moon.".to_string(),
        };
        out.push(Cue::Say(format!(
            "{here} You are looking for {}.",
            star_words(target.0, target.1)
        )));
    }

    fn trace(&mut self, con: usize, star: usize, out: &mut Vec<Cue>) {
        let def = &CONSTELLATIONS[con];
        let sname = sentence_start(def.stars[star].name);
        if !self.targets.contains(&con) {
            out.push(plain(sound::NOT_YET, 0.6, 1.0));
            out.push(Cue::Say(format!(
                "{sname} is in {}, which is not part of this challenge.",
                def.name
            )));
            return;
        }
        if self.finished(con) {
            out.push(plain(sound::NOT_YET, 0.6, 1.0));
            out.push(Cue::Say(format!("{} is traced already tonight.", def.name)));
            return;
        }
        let next = self.traced_of(con);
        if star != next {
            out.push(plain(sound::NOT_YET, 0.6, 1.0));
            let next_name = def.stars[next].name;
            let text = if next == 0 {
                format!(
                    "{sname} is in {}. Its trace starts at {next_name}.",
                    def.name
                )
            } else {
                format!("{sname} is in {}. Its next star is {next_name}.", def.name)
            };
            out.push(Cue::Say(text));
            return;
        }
        let n = next + 1;
        self.traced.insert(con, n);
        self.points += STAR_POINTS;
        let len = def.stars.len();
        out.push(plain(sound::TRACE, 0.8, 2f32.powf(n as f32 / 12.0 * 2.0)));
        if n < len {
            out.push(Cue::Say(format!(
                "{sname} traced. {} {n} of {len}. Next, {}.",
                def.name, def.stars[n].name
            )));
            return;
        }
        self.points += PATTERN_POINTS;
        self.done.push(con);
        out.push(Cue::Traced(con));
        let first = self.logged.insert(con);
        if first {
            self.newly.push(con);
            out.push(Cue::Say(format!(
                "{} traced: {}. In your journal.",
                def.name, def.figure
            )));
            out.push(Cue::Queue(def.lore_after_figure().to_string()));
        } else {
            out.push(Cue::Say(format!(
                "{} is traced again: {}.",
                def.name, def.figure
            )));
        }
        if self.is_free() {
            self.rise(out);
        }
    }

    /// Raises constellations the journal has just opened, on a free night.
    fn rise(&mut self, out: &mut Vec<Cue>) {
        let tier = open_tier(self.logged.len());
        let season = self.season();
        let rising: Vec<usize> = season_constellations(season)
            .into_iter()
            .filter(|&c| CONSTELLATIONS[c].tier <= tier && !self.sky.up.contains(&c))
            .collect();
        for &c in &rising {
            self.sky.raise(c, &mut self.rng);
            self.targets.push(c);
        }
        if !rising.is_empty() {
            out.push(plain(sound::RISE, 0.8, 1.0));
            let names: Vec<String> = rising
                .iter()
                .map(|&c| {
                    let def = &CONSTELLATIONS[c];
                    let (az, _) = def.centre(season).unwrap_or((0, 0));
                    format!("{} in the {}", def.name, compass(az))
                })
                .collect();
            let text = match names.len() {
                1 => format!("A new constellation rises: {}.", names[0]),
                n => format!(
                    "New constellations rise: {}, and {}.",
                    names[..n - 1].join(", "),
                    names[n - 1]
                ),
            };
            out.push(Cue::Queue(text));
        } else if self.targets.iter().all(|&c| self.finished(c)) {
            out.push(plain(sound::ALL_TRACED, 0.8, 1.0));
            out.push(Cue::Queue(
                "All traced tonight. Enter twice heads home.".to_string(),
            ));
        }
    }

    fn find(&mut self, w: usize, out: &mut Vec<Cue>) {
        let def = &WONDERS[w];
        if self.found.contains(&w) {
            out.push(plain(sound::NOT_YET, 0.6, 1.0));
            out.push(Cue::Say(format!(
                "{}, found already tonight.",
                sentence_start(def.name)
            )));
            return;
        }
        self.found.push(w);
        self.points += WONDER_POINTS;
        out.push(plain(sound::WONDER_FOUND, 0.9, 1.0));
        out.push(Cue::Found(w));
        out.push(Cue::Say(format!("Found: {}.", def.name)));
        out.push(Cue::Queue(def.line.to_string()));
    }

    fn greet(&mut self, out: &mut Vec<Cue>) {
        let name = self.moon.map(|m| m.name).unwrap_or("the moon");
        if self.greeted {
            out.push(plain(sound::NOT_YET, 0.6, 1.0));
            out.push(Cue::Say(format!(
                "{}, greeted already tonight.",
                sentence_start(name)
            )));
            return;
        }
        self.greeted = true;
        out.push(plain(sound::MOON_GREET, 0.9, 1.0));
        out.push(Cue::Greeted);
        out.push(Cue::Say(format!("You greet {name}.")));
        let line = if self.moon.is_some_and(|m| m.blue) {
            "A second full moon in one calendar month is called a blue moon. It comes about once every two or three years."
        } else {
            "Full moons have old names, one for each month. Greet each one through the year to fill your journal."
        };
        out.push(Cue::Queue(line.to_string()));
    }

    fn check_complete(&mut self, out: &mut Vec<Cue>) {
        if self.complete {
            return;
        }
        let done = match &self.goal {
            Goal::Free | Goal::Stars(_) => false,
            Goal::Patterns(_) => self.targets.iter().all(|&c| self.finished(c)),
            Goal::Tour => {
                self.targets.iter().all(|&c| self.finished(c))
                    && season_wonders(self.season())
                        .iter()
                        .all(|w| self.found.contains(w))
            }
        };
        if done {
            self.complete = true;
            out.push(Cue::Complete);
        }
    }

    /// The next thing worth finding, and its words.
    pub fn target(&self) -> Option<Target> {
        let here = self.point();
        if let Some((c, s)) = self.hunt_target() {
            return Some(Target {
                at: self.sky.star_place(c, s)?,
                words: star_words(c, s),
            });
        }
        // The next star of each unfinished constellation: one already begun
        // first, then the ones not yet in the journal, then the nearest.
        let best = self
            .targets
            .iter()
            .filter(|&&c| !self.finished(c))
            .filter_map(|&c| {
                let next = self.traced_of(c);
                let at = self.sky.star_place(c, next)?;
                let rank = (next == 0, self.logged.contains(&c));
                Some((rank, angle(here, at), c, next, at))
            })
            .min_by(|a, b| {
                a.0.cmp(&b.0)
                    .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            });
        if let Some((_, _, c, next, at)) = best {
            let def = &CONSTELLATIONS[c];
            let words = if next == 0 {
                format!(
                    "{}, where {} begins",
                    sentence_start(def.stars[0].name),
                    def.name
                )
            } else {
                format!(
                    "{}, next in {}",
                    sentence_start(def.stars[next].name),
                    def.name
                )
            };
            return Some(Target { at, words });
        }
        let wonder = season_wonders(self.season())
            .into_iter()
            .filter(|w| !self.found.contains(w))
            .filter_map(|w| WONDERS[w].at(self.season()).map(|at| (angle(here, at), at)))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        if let Some((_, at)) = wonder {
            return Some(Target {
                at,
                words: "Something soft and hidden".to_string(),
            });
        }
        if self.moon.is_some() && !self.greeted {
            return Some(Target {
                at: sky::MOON_AT,
                words: "The full moon".to_string(),
            });
        }
        None
    }

    /// H: where the next thing to find is, in steps.
    pub fn hint(&mut self) -> Vec<Cue> {
        let Some(target) = self.target() else {
            return vec![
                plain(sound::NOTHING, 0.6, 1.0),
                Cue::Say("All traced and found. Enter twice heads home.".to_string()),
            ];
        };
        self.hints += 1;
        if !self.is_free() {
            self.hunt_hints += 1;
        }
        let (pan, ahead, pitch) = voice(self.point(), target.at);
        let mut out = vec![Cue::Sound {
            key: sound::HINT,
            pan,
            ahead,
            gain: 0.8,
            pitch,
        }];
        let way = steps_words(self.point(), target.at);
        out.push(Cue::Say(if target.at == self.point() {
            format!("{}: right here.", target.words)
        } else {
            format!("{}: {way}.", target.words)
        }));
        out
    }

    /// Tab: the guide tone on or off.
    pub fn toggle_guide(&mut self) -> Vec<Cue> {
        if self.guide {
            self.guide = false;
            return vec![
                plain(sound::GUIDE, 0.5, 0.8),
                Cue::Say("Guide tone off.".to_string()),
            ];
        }
        let Some(target) = self.target() else {
            return vec![
                plain(sound::NOTHING, 0.6, 1.0),
                Cue::Say("No guide tone: all traced and found.".to_string()),
            ];
        };
        self.guide = true;
        self.hunt_guided = true;
        self.guide_timer = 0.6;
        vec![
            plain(sound::GUIDE, 0.6, 1.0),
            Cue::Say(format!(
                "Guide tone on, toward {}.",
                lower_first(&target.words)
            )),
        ]
    }

    /// The status line, in a fixed order: where the point is, what it rests
    /// on, the trace or hunt in hand, tonight's tally, then the guide and
    /// the moon.
    /// The status key, in layers. First: where the point is, what it rests
    /// on, and the star or constellation under way. A second press:
    /// tonight's progress, points, the guide tone and the moon.
    pub fn status_layers(&self) -> Vec<String> {
        let mut first = vec![sentence_start(&place_words(self.point()))];
        first.push(match self.body_here() {
            Some(Body::Star { con, star }) => {
                format!("on {}", CONSTELLATIONS[con].stars[star].name)
            }
            Some(Body::Faint) => "on a faint star".to_string(),
            Some(Body::Wonder(w)) if self.found.contains(&w) => format!("on {}", WONDERS[w].name),
            Some(Body::Wonder(_)) => "on a soft glow".to_string(),
            Some(Body::Moon) => "on the full moon".to_string(),
            None => "on empty sky".to_string(),
        });
        let mut more = Vec::new();
        match &self.goal {
            Goal::Stars(list) => {
                if let Some((c, s)) = self.hunt_target() {
                    first.push(format!(
                        "star {} of {}, {}",
                        self.hunted + 1,
                        list.len(),
                        CONSTELLATIONS[c].stars[s].name
                    ));
                }
                more.push(format!("{} points", self.points));
            }
            _ => {
                for &c in &self.targets {
                    let n = self.traced_of(c);
                    let len = CONSTELLATIONS[c].stars.len();
                    if n > 0 && n < len {
                        first.push(format!("{} {n} of {len}", CONSTELLATIONS[c].name));
                    }
                }
                let traced = self.targets.iter().filter(|&&c| self.finished(c)).count();
                more.push(format!(
                    "tonight {traced} of {} constellations traced",
                    self.targets.len()
                ));
                let wonders = season_wonders(self.season()).len();
                more.push(format!("{} of {wonders} wonders found", self.found.len()));
                if !self.is_free() {
                    more.push(format!("{} points", self.points));
                }
            }
        }
        more.push(if self.guide {
            "guide tone on".to_string()
        } else {
            "guide tone off".to_string()
        });
        if let Some(m) = self.moon {
            more.push(format!("{} is up", m.name));
        }
        vec![
            format!("{}.", first.join(", ")),
            format!("{}.", sentence_start(&more.join(", "))),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }

    /// The constellations up tonight, for the start of the night.
    pub fn up_words(&self) -> String {
        let names: Vec<&str> = self
            .targets
            .iter()
            .map(|&c| CONSTELLATIONS[c].name)
            .collect();
        match names.len() {
            0 => String::new(),
            1 => names[0].to_string(),
            n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
        }
    }

    /// The challenge score: points less hints, in a tracing challenge.
    pub fn score(&self) -> u64 {
        match self.goal {
            Goal::Stars(_) => self.points,
            _ => self.points.saturating_sub(HINT_COST * self.hints as u64),
        }
    }

    /// Time passing.
    pub fn update(&mut self, dt: f64) -> Vec<Cue> {
        let mut out = Vec::new();
        let mut clock = self.clock;
        clock.advance(dt, |step| self.tick(step, &mut out));
        self.clock = clock;
        out
    }

    fn tick(&mut self, step: f64, out: &mut Vec<Cue>) {
        let point = self.point();
        let moonlit = self.sky.full_moon;
        for i in 0..self.sky.objects.len() {
            let obj = &mut self.sky.objects[i];
            obj.twinkle -= step;
            if obj.twinkle > 0.0 {
                continue;
            }
            let (body, mag, at) = (obj.body, obj.mag, obj.at());
            obj.twinkle = match body {
                Body::Moon => 4.0,
                Body::Wonder(_) => 5.0,
                _ => 1.2 + mag.max(0.0) as f64 * 0.35 + self.rng.unit() * 2.0,
            };
            if matches!(body, Body::Wonder(_)) {
                continue;
            }
            let d = angle(point, at);
            if d > AUDIBLE {
                continue;
            }
            let near = (1.0 - d / AUDIBLE) as f32;
            let mut gain = brightness(mag) * near.powf(1.5) * 0.7;
            if moonlit && matches!(body, Body::Faint) {
                gain *= 0.5;
            }
            if gain < 0.03 {
                continue;
            }
            let (pan, ahead, pitch) = voice(point, at);
            out.push(Cue::Sound {
                key: Night::chime(body, mag),
                pan,
                ahead,
                gain,
                pitch,
            });
        }
        self.shimmer(step, out);
        self.guide_pulse(step, out);
    }

    fn shimmer(&mut self, step: f64, out: &mut Vec<Cue>) {
        self.shimmer_timer -= step;
        if self.shimmer_timer > 0.0 {
            return;
        }
        self.shimmer_timer = 2.2;
        let point = self.point();
        let season = self.season();
        let near = season_wonders(season)
            .into_iter()
            .filter(|w| !self.found.contains(w))
            .filter_map(|w| WONDERS[w].at(season))
            .map(|at| (angle(point, at), at))
            .filter(|(d, _)| *d <= SHIMMER_RANGE)
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let Some((d, at)) = near else {
            return;
        };
        let (pan, ahead, pitch) = voice(point, at);
        out.push(Cue::Sound {
            key: sound::SHIMMER,
            pan,
            ahead,
            gain: 0.25 + 0.6 * (1.0 - d / SHIMMER_RANGE) as f32,
            pitch,
        });
        if !self.shimmer_told {
            self.shimmer_told = true;
            out.push(Cue::Queue(format!(
                "Something hidden is near. Rest on it, press {}.",
                self.primary_key
            )));
        }
    }

    fn guide_pulse(&mut self, step: f64, out: &mut Vec<Cue>) {
        if !self.guide {
            return;
        }
        self.guide_timer -= step;
        if self.guide_timer > 0.0 {
            return;
        }
        let Some(target) = self.target() else {
            self.guide = false;
            out.push(Cue::Queue(
                "Guide tone off: nothing left to find.".to_string(),
            ));
            return;
        };
        let d = angle(self.point(), target.at);
        self.guide_timer = 0.35 + 1.1 * (d / 90.0).min(1.0);
        let (pan, ahead, pitch) = voice(self.point(), target.at);
        out.push(Cue::Sound {
            key: sound::GUIDE,
            pan,
            ahead,
            gain: 0.55,
            pitch,
        });
    }
}

/// "bellatrix, next in orion" is wrong, so only a leading article is
/// lowered: "The full moon" becomes "the full moon".
fn lower_first(text: &str) -> String {
    for article in ["The ", "Something "] {
        if let Some(rest) = text.strip_prefix(article) {
            return format!("{}{rest}", article.to_lowercase());
        }
    }
    text.to_string()
}
