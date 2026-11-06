//! The rooftop itself, and what comes before and after it: saving the
//! garden, the challenge pages, the seed packet and the results.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use hearken::audio::Start;
use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::screens::ConfirmScreen;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{
    Challenge, Challenges, Pages, Parlor, ParlorAction, Period, Records, Rng, ScoreResult,
};
use serde::{Deserialize, Serialize};

use crate::breeding::{Collection, Keepsake, Variety};
use crate::cues;
use crate::menus::CHALLENGES_NAME;
use crate::rules::{number, sentence, Ask, Garden, Goal, Keys, Kind, Note, Species, Stage, Twist};
use crate::song::{Part, Song};
use crate::{HELP, ID};

/// The title's screen name.
pub const TITLE_NAME: &str = "rooftop_garden";
/// The garden's screen name.
pub const RUN_NAME: &str = "rooftop_garden_run";

/// Where your own garden is saved.
pub const GARDEN_KEY: &str = "rooftop-garden/garden";
/// Where unfinished challenge gardens are saved, by challenge slot.
pub const CHALLENGE_KEY: &str = "rooftop-garden/challenge-gardens";
/// What your gardens have found: varieties, keepsakes and requests. Kept
/// apart from the garden, so starting it over keeps them.
pub const COLLECTION_KEY: &str = "rooftop-garden/collection";
/// Small saved facts: whether the tutorial has been read.
pub const PROGRESS_KEY: &str = "rooftop-garden/progress";

#[derive(Default, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    pub tutorial: bool,
}

/// The player's controls by name.
pub fn keys(ctx: &Ctx<Parlor>) -> Keys {
    let arrows = [
        ParlorAction::Up,
        ParlorAction::Down,
        ParlorAction::Left,
        ParlorAction::Right,
    ]
    .map(|a| ctx.control_name(a));
    Keys {
        arrows: arrows.join(", "),
        water: ctx.control_name(ParlorAction::Primary),
        tend: ctx.control_name(ParlorAction::Secondary),
        status: ctx.control_name(ParlorAction::Status),
        repeat: ctx.control_name(ParlorAction::Repeat),
        hint: ctx.control_name(ParlorAction::Hint),
    }
}

/// The garden's controls in one paragraph, for F1.
pub fn controls_text(ctx: &Ctx<Parlor>) -> String {
    let k = keys(ctx);
    format!(
        "In the garden. {} step between planters: up and down change row. {} waters, or plants in empty soil; watering moist soil waterlogs it. {} picks aphids, harvests or prunes. Delete composts a plant. Enter sits on the bench to pass time faster. L lists every planter. N reads the neighbours' requests. {} says what needs you; press it twice for the day, weather and harmony. {} repeats. {} points to the nearest job. Escape pauses; the garden saves when you leave.",
        sentence(&k.arrows),
        sentence(&k.water),
        sentence(&k.tend),
        sentence(&k.status),
        sentence(&k.repeat),
        sentence(&k.hint),
    )
}

/// What starting a garden needs: a seed for tests (`None` seeds from the clock).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub seed: Option<u64>,
}

impl Launch {
    fn rng(&self) -> Rng {
        self.seed.map_or_else(Rng::from_time, Rng::seeded)
    }
}

/// Your saved garden, or a fresh one.
pub fn load_home(ctx: &mut Ctx<Parlor>, launch: Launch) -> Garden {
    let saved: Option<Garden> = Records::load(ctx, GARDEN_KEY);
    let mut g = match saved {
        Some(mut g) => {
            g.repair();
            g.rng = launch.rng();
            g
        }
        None => Garden::home(launch.rng()),
    };
    g.book = load_collection(ctx);
    g
}

/// What your gardens have found.
pub fn load_collection(ctx: &mut Ctx<Parlor>) -> Collection {
    let mut book: Collection = Records::load(ctx, COLLECTION_KEY);
    book.repair();
    book
}

/// Saves a garden: yours under its own key, a challenge's by its slot.
pub fn save(ctx: &mut Ctx<Parlor>, garden: &Garden) {
    match garden.kind {
        Kind::Home => {
            Records::store(ctx, GARDEN_KEY, &Some(garden.clone()));
            Records::store(ctx, COLLECTION_KEY, &garden.book);
            Records::record(ctx, ID, "varieties", garden.book.found.len() as u64);
            Records::record(ctx, ID, "harvests", garden.harvests as u64);
            Records::record(ctx, ID, "harmony", garden.best_harmony as u64);
            Records::record(ctx, ID, "days", garden.day as u64);
        }
        _ => {
            let mut all: BTreeMap<String, Garden> = Records::load(ctx, CHALLENGE_KEY);
            all.insert(garden.slot.clone(), garden.clone());
            Records::store(ctx, CHALLENGE_KEY, &all);
        }
    }
}

/// Forgets a finished challenge garden, and any from past periods.
fn forget_challenge(ctx: &mut Ctx<Parlor>, slot: &str) {
    let current: Vec<String> = Period::ALL
        .iter()
        .map(|&p| slot_of(&Challenge::current(ctx, ID, p)))
        .collect();
    let mut all: BTreeMap<String, Garden> = Records::load(ctx, CHALLENGE_KEY);
    all.retain(|k, _| k != slot && current.contains(k));
    Records::store(ctx, CHALLENGE_KEY, &all);
}

/// "weekly:2026-W42".
pub fn slot_of(c: &Challenge) -> String {
    format!("{}:{}", c.period.label().to_lowercase(), c.key)
}

/// Opens your own garden.
pub fn start_home(ctx: &mut Ctx<Parlor>, launch: Launch) {
    let garden = load_home(ctx, launch);
    ctx.push(RunScreen::new(garden, None));
}

/// What each period's challenge is about.
pub fn period_text(period: Period) -> &'static str {
    match period {
        Period::Daily => "6 planters, 4 kinds of seed, 4 days to grow a three-part harmony.",
        Period::Weekly => "9 planters, 8 days to harvest 5 tomatoes.",
        Period::Monthly => {
            "A festival garden: 12 planters, 10 days to bring all 6 voices into a full chorus."
        }
    }
}

/// The current challenge of `period`, and its garden: the one saved in
/// progress if there is one, else a fresh one from the challenge seed.
pub fn challenge_garden(ctx: &mut Ctx<Parlor>, period: Period) -> (Challenge, Garden, bool) {
    let challenge = Challenge::current(ctx, ID, period);
    let slot = slot_of(&challenge);
    let mut all: BTreeMap<String, Garden> = Records::load(ctx, CHALLENGE_KEY);
    match all.remove(&slot) {
        Some(mut g) => {
            g.repair();
            // The rest of the season rolls from the challenge's own seed and
            // how far the garden has come, not from the clock.
            let at = g.day as u64 * 100_000 + (g.clock * 10_000.0) as u64;
            g.rng = Rng::seeded(challenge.seed ^ at.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            (challenge, g, true)
        }
        None => {
            let g = Garden::challenge(period, slot, challenge.rng());
            (challenge, g, false)
        }
    }
}

/// The twist of today's `period` challenge.
pub fn challenge_twist(ctx: &Ctx<Parlor>, period: Period) -> Twist {
    let c = Challenge::current(ctx, ID, period);
    Garden::challenge(period, String::new(), c.rng())
        .twist
        .unwrap_or(Twist::Festival)
}

/// The current challenge of `period`: its rule first, then the garden.
/// Escape on these pages goes back without opening it.
pub fn challenge_briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let (challenge, garden, resumed) = challenge_garden(ctx, period);
    let twist = garden.twist.unwrap_or(Twist::Festival);
    let back = if resumed {
        format!(
            "Back to your challenge garden: day {} of {}. ",
            garden.day,
            garden.day_limit.unwrap_or(garden.day)
        )
    } else {
        String::new()
    };
    let name = match period {
        Period::Daily => "Today's garden",
        Period::Weekly => "This week's garden",
        Period::Monthly => "The rooftop festival",
    };
    let plan = period_text(period).trim_start_matches("A festival garden: ");
    let twist_text = match twist {
        Twist::Festival => twist.describe().to_string(),
        _ => {
            let d = twist.describe();
            let mut c = d.chars();
            let first = c
                .next()
                .map(|f| f.to_lowercase().collect::<String>())
                .unwrap_or_default();
            format!("Twist, {}: {first}{}", twist.name(), c.as_str())
        }
    };
    let goal = match garden.goal {
        Some(Goal::Harmony(n)) => format!(
            " A {}-part harmony is {n} different plants in bloom and well watered at once. Score: 100 for the goal, 10 a day to spare, 5 a harvest, 5 a harmony part, 2 a pest.",
            number(n)
        ),
        Some(_) => " Score: 100 for the goal, 10 a day to spare, 5 a harvest, 5 a harmony part, 2 a pest.".to_string(),
        None => String::new(),
    };
    let pages = vec![format!(
        "{back}{name}: {plan} {twist_text}{goal} Leaving from the pause menu keeps the garden until the {} is over.",
        period.unit(),
    )];
    let mut slot = Some((garden, challenge));
    Pages::briefing(pages).then(move |ctx| {
        if let Some((garden, challenge)) = slot.take() {
            ctx.replace(RunScreen::new(garden, Some(challenge)));
        }
    })
}

/// The results menu after a challenge garden closes: the challenge again,
/// the results again, and back to the challenges.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "A fresh garden for the same challenge. Only your best score is kept.",
        move |ctx| {
            let (challenge, garden, _) = challenge_garden(ctx, period);
            ctx.replace(RunScreen::new(garden, Some(challenge)))
        },
    )
}

/// A planter's place on the rooftop, two steps apart; row 1 is ahead.
pub fn plot_pos(garden: &Garden, i: usize) -> Vec3 {
    let (r, c) = garden.cell(i);
    Vec3::new(c as f32 * 2.0, -(r as f32) * 2.0, 0.0)
}

/// The sounds coming from one planter.
#[derive(Default)]
struct Voice {
    /// The species and instrument it plays on.
    species: Option<(Species, u8)>,
    hum: Option<Sound>,
    /// The plant's part of the song, while it sings.
    part: Option<Part>,
    /// What it is singing now: a loop of the garden's own tune, or this
    /// bar's phrase of a new one.
    layer: Option<Sound>,
    /// The last bar's phrase, still ringing out.
    tail: Option<Sound>,
    /// Waiting for the next bar to join the song.
    pending: bool,
    dry: Option<Sound>,
    soggy: Option<Sound>,
    pest: Option<Sound>,
    volume: f32,
    pitch: f32,
}

/// What the screen asked the player, waiting for an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    Nothing,
    Seeds,
    Compost,
}

/// How loud a plant's part plays: softly while growing, fully in bloom.
pub fn layer_volume(stage: Stage) -> f32 {
    if stage >= Stage::Blooming {
        0.8
    } else {
        0.35
    }
}

/// A thirsty plant's part droops this far in pitch.
pub const DROOP: f32 = 0.94;

/// A waterlogged plant's part sinks to this share of its volume.
pub const SOGGY_VOLUME: f32 = 0.4;

/// The rooftop.
pub struct RunScreen {
    garden: Garden,
    challenge: Option<Challenge>,
    asked: Asked,
    seed: Rc<Cell<Option<Variety>>>,
    pumpkin: Rc<Cell<bool>>,
    compost: Rc<Cell<bool>>,
    voices: Vec<Voice>,
    loops: Vec<Sound>,
    wind: Option<Sound>,
    rain: Option<Sound>,
    /// The pad under the song, and its last bar ringing out.
    base: [Option<Sound>; 2],
    song: Song,
    /// The song's seed, and the parts that sang it in the order they
    /// joined, kept with the session for the soundtrack.
    song_seed: u64,
    sung: Vec<Part>,
    verses_heard: u32,
    song_kept: bool,
    fx_rng: Rng,
    bird_in: f64,
    chime_in: f64,
    horn_in: f64,
    done: bool,
    /// False for a garden that is only for learning: it is never saved.
    keep: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl RunScreen {
    pub fn new(garden: Garden, challenge: Option<Challenge>) -> RunScreen {
        let seed = garden.day as u64 * 7919 + garden.harvests as u64;
        RunScreen {
            status_key: Default::default(),
            garden,
            challenge,
            asked: Asked::Nothing,
            seed: Rc::new(Cell::new(None)),
            pumpkin: Rc::new(Cell::new(false)),
            compost: Rc::new(Cell::new(false)),
            voices: Vec::new(),
            loops: Vec::new(),
            wind: None,
            rain: None,
            base: [None, None],
            song: Song::new(seed ^ 0x5eed),
            song_seed: seed ^ 0x5eed,
            sung: Vec::new(),
            verses_heard: 0,
            song_kept: false,
            fx_rng: Rng::seeded(seed),
            bird_in: 6.0,
            chime_in: 14.0,
            horn_in: 25.0,
            done: false,
            keep: true,
        }
    }

    /// A garden to learn on, never saved over yours.
    pub fn practice(garden: Garden) -> RunScreen {
        RunScreen {
            status_key: Default::default(),
            keep: false,
            ..RunScreen::new(garden, None)
        }
    }

    /// The rules, for tests.
    pub fn garden(&self) -> &Garden {
        &self.garden
    }

    /// The rules, for tests that set the scene by hand.
    pub fn garden_mut(&mut self) -> &mut Garden {
        &mut self.garden
    }

    /// How many plants' parts are playing.
    pub fn voices(&self) -> usize {
        self.voices.iter().filter(|v| v.layer.is_some()).count()
    }

    /// Where the song is, for tests.
    pub fn song(&self) -> &Song {
        &self.song
    }

    fn stop_voice(ctx: &mut Ctx<Parlor>, v: &mut Voice) {
        for s in [
            v.hum.take(),
            v.layer.take(),
            v.tail.take(),
            v.dry.take(),
            v.soggy.take(),
            v.pest.take(),
        ]
        .into_iter()
        .flatten()
        {
            ctx.audio.stop(s, 400);
        }
        v.pending = false;
        v.part = None;
        v.species = None;
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        let mut voices = std::mem::take(&mut self.voices);
        for v in voices.iter_mut() {
            Self::stop_voice(ctx, v);
        }
        let all: Vec<Sound> = self
            .loops
            .drain(..)
            .chain(self.wind.take())
            .chain(self.rain.take())
            .chain(self.base.iter_mut().filter_map(Option::take))
            .collect();
        for s in all {
            ctx.audio.stop(s, 400);
        }
    }

    fn start_ambience(&mut self, ctx: &mut Ctx<Parlor>) {
        let city = ctx.audio.start_loop(cues::CITY, None, Bus::AMBIENCE);
        ctx.audio.set_volume(city, 0.4);
        self.loops.push(city);
        let wind = ctx.audio.start_loop(cues::WIND, None, Bus::AMBIENCE);
        ctx.audio.set_volume(wind, self.wind_volume());
        self.wind = Some(wind);
        // The song starts its verse again, with every grown part at once.
        self.song.restart();
        self.sync(ctx);
        self.play_bar(ctx);
    }

    fn wind_volume(&self) -> f32 {
        match self.garden.weather {
            crate::rules::Weather::Breezy => 0.6,
            _ => 0.2,
        }
    }

    /// Brings every planter's loops in line with the rules. A part that
    /// starts singing waits for the next bar.
    fn sync(&mut self, ctx: &mut Ctx<Parlor>) {
        let here = plot_pos(&self.garden, self.garden.here());
        ctx.audio.set_listener(here, 0.0);
        if self.voices.len() != self.garden.plots.len() {
            self.stop_voices(ctx);
            self.voices = (0..self.garden.plots.len())
                .map(|_| Voice::default())
                .collect();
        }
        let raining = self.garden.raining();
        for i in 0..self.voices.len() {
            let pos = plot_pos(&self.garden, i);
            let plant = self.garden.plots[i].clone();
            let v = &mut self.voices[i];
            let who = plant.as_ref().map(|p| (p.species, p.tone()));
            if v.species != who {
                Self::stop_voice(ctx, v);
                v.species = who;
            }
            let Some(p) = plant else {
                continue;
            };
            let hum = p.stage == Stage::Sprout && !p.wilted;
            toggle(ctx, &mut v.hum, hum, cues::HUM, pos, 0.5);
            let dry = p.thirsty() && !raining;
            toggle(ctx, &mut v.dry, dry, cues::DRY, pos, 0.8);
            toggle(ctx, &mut v.soggy, p.waterlogged(), cues::SOGGY, pos, 0.7);
            toggle(ctx, &mut v.pest, p.pest, cues::BUZZ, pos, 0.7);
            let pitch = if p.thirsty() && p.stage >= Stage::Sprout {
                DROOP
            } else {
                1.0
            };
            if let Some(h) = v.hum {
                ctx.audio.set_pitch(h, pitch);
            }
            if p.voiced() {
                let volume = layer_volume(p.stage);
                v.part = Some(Part::of(p.species, p.pumpkin));
                if v.layer.is_none() {
                    v.pending = true;
                }
                for s in [v.layer, v.tail].into_iter().flatten() {
                    if v.volume != volume {
                        ctx.audio.set_volume(s, volume);
                    }
                    if v.pitch != pitch {
                        ctx.audio.set_pitch(s, pitch);
                    }
                }
                v.volume = volume;
                v.pitch = pitch;
            } else {
                v.pending = false;
                v.part = None;
                for s in [v.layer.take(), v.tail.take()].into_iter().flatten() {
                    ctx.audio.stop(s, 600);
                }
            }
        }
        match (raining, self.rain) {
            (true, None) => {
                let s = ctx.audio.start_loop(cues::RAIN, None, Bus::AMBIENCE);
                ctx.audio.set_volume(s, 0.6);
                self.rain = Some(s);
            }
            (false, Some(s)) => {
                ctx.audio.stop(s, 1500);
                self.rain = None;
            }
            _ => {}
        }
    }

    fn stop_voices(&mut self, ctx: &mut Ctx<Parlor>) {
        let mut voices = std::mem::take(&mut self.voices);
        for v in voices.iter_mut() {
            Self::stop_voice(ctx, v);
        }
    }

    /// Moves the song on a bar at a time. New parts wait for the start of
    /// the next bar, so every part stays on the beat.
    fn tick_song(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.song.tick(dt) {
            self.play_bar(ctx);
        }
    }

    /// The top of a bar: each singing part starts what the song gives it
    /// here, and parts waiting to join come in.
    fn play_bar(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.song.bar() == 0 {
            self.verses_heard += 1;
        }
        for v in &self.voices {
            if let Some(part) = v.part {
                if !self.sung.contains(&part) {
                    self.sung.push(part);
                }
            }
        }
        let base_joins = self.base[0].is_none();
        if let Some((key, from)) = self.song.cue(Part::Base, base_joins) {
            let s = start_part(ctx, &key, from, None, 0.6, 1.0);
            let last = self.base[0].replace(s);
            if let Some(old) = std::mem::replace(&mut self.base[1], last) {
                ctx.audio.stop(old, 300);
            }
        }
        for i in 0..self.voices.len() {
            let pos = plot_pos(&self.garden, i);
            let v = &mut self.voices[i];
            let Some(part) = v.part else {
                continue;
            };
            let tone = v.species.map_or(0, |(_, t)| t);
            let joining = v.layer.is_none();
            if let Some((key, from)) = self.song.cue_on(part, tone, joining) {
                let s = start_part(ctx, &key, from, Some(pos), v.volume, v.pitch);
                let last = v.layer.replace(s);
                if let Some(old) = std::mem::replace(&mut v.tail, last) {
                    ctx.audio.stop(old, 300);
                }
            }
            v.pending = false;
        }
    }

    /// Sparrows, a pigeon, the neighbour's chimes and a horn far below.
    fn tick_ambience(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let speed = if self.garden.sitting { 2.0 } else { 1.0 };
        let r = &mut self.fx_rng;
        let around = |r: &mut Rng, dist: f32| {
            let x = (r.unit() as f32 - 0.5) * 2.0 * dist;
            let y = (r.unit() as f32 - 0.3) * dist;
            Vec3::new(x, y, 1.0)
        };
        self.bird_in -= dt * speed;
        if self.bird_in <= 0.0 {
            let morning = self.garden.clock < 0.3;
            let key = if r.chance(0.7) {
                cues::SPARROW
            } else {
                cues::PIGEON
            };
            let pos = around(r, 8.0);
            let s = ctx.audio.play_at(key, pos, Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.5);
            self.bird_in = if morning { 5.0 } else { 12.0 } + r.unit() * 10.0;
        }
        self.chime_in -= dt * speed;
        if self.chime_in <= 0.0 {
            let pos = Vec3::new(-12.0, 10.0, 3.0);
            let s = ctx.audio.play_at(cues::CHIMES, pos, Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.5);
            let breezy = self.garden.weather == crate::rules::Weather::Breezy;
            if breezy && self.garden.book.has(Keepsake::WindChimes) {
                // Your own chimes, by the bench, a little higher.
                let s = ctx
                    .audio
                    .play_at(cues::CHIMES, Vec3::new(-1.0, 1.0, 0.5), Bus::AMBIENCE);
                ctx.audio.set_volume(s, 0.45);
                ctx.audio.set_pitch(s, 1.26);
            }
            self.chime_in = if breezy { 8.0 } else { 20.0 } + r.unit() * 15.0;
        }
        self.horn_in -= dt;
        if self.horn_in <= 0.0 {
            let pos = around(r, 20.0);
            let s = ctx
                .audio
                .play_at(cues::HORN, Vec3::new(pos.x, pos.y, -8.0), Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.3);
            self.horn_in = 30.0 + r.unit() * 40.0;
        }
    }

    fn play_notes(&mut self, ctx: &mut Ctx<Parlor>) {
        for note in self.garden.drain_notes() {
            match note {
                Note::Say(text) => ctx.say(text),
                Note::Queue(text) => ctx.say_queued(text),
                Note::Cue(key) => {
                    ctx.audio.play(key, Bus::EFFECTS);
                }
                Note::At(key, i) => {
                    let pos = plot_pos(&self.garden, i.min(self.garden.plots.len() - 1));
                    ctx.audio.play_at(key, pos, Bus::EFFECTS);
                }
                Note::Beyond(key, dr, dc) => {
                    let here = plot_pos(&self.garden, self.garden.here());
                    let pos = Vec3::new(here.x + dc as f32 * 2.0, here.y - dr as f32 * 2.0, 0.0);
                    ctx.audio.play_at(key, pos, Bus::EFFECTS);
                }
                Note::Stinger(key) => {
                    let s = ctx.audio.play(key, Bus::MUSIC);
                    ctx.audio.set_volume(s, 0.8);
                }
                Note::NewDay => {
                    if self.keep {
                        save(ctx, &self.garden);
                    }
                    if let Some(w) = self.wind {
                        ctx.audio.set_volume(w, self.wind_volume());
                    }
                }
                Note::Expanded => self.stop_voices(ctx),
                Note::GoalMet | Note::SeasonOver => {}
            }
        }
    }

    /// Keeps this session's song in the player's garden songs, once.
    fn keep_song(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.keep || self.song_kept {
            return;
        }
        self.song_kept = true;
        let today = parlor_core::challenges::today(ctx);
        crate::soundtrack::keep_session(
            ctx,
            crate::soundtrack::Session {
                n: 0,
                year: today.year,
                month: today.month,
                day: today.day,
                garden_day: self.garden.day,
                seed: self.song_seed,
                parts: self.sung.iter().map(|p| p.id().to_string()).collect(),
                verses: self.verses_heard,
            },
        );
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.done = true;
        self.keep_song(ctx);
        self.stop_sounds(ctx);
        let g = &self.garden;
        let met = g.goal_met;
        ctx.audio.play(
            if met {
                cues::MUSIC_GOAL
            } else {
                cues::MUSIC_SEASON_END
            },
            Bus::MUSIC,
        );
        let mut lines = Vec::new();
        if met {
            lines.push(format!(
                "Goal met on day {}. {}",
                g.day,
                g.goal.map(|g| g.describe()).unwrap_or_default()
            ));
        } else {
            let (now, target) = g.progress().unwrap_or((0, 0));
            lines.push(format!(
                "The days are up, with {now} of {target} toward the goal."
            ));
        }
        let harvests = match g.harvests {
            1 => "1 harvest".to_string(),
            n => format!("{n} harvests"),
        };
        let pests = match g.pests_picked {
            1 => "1 pest picked".to_string(),
            n => format!("{n} pests picked"),
        };
        let harmony = match g.best_harmony {
            1 => "a fullest harmony of 1 part".to_string(),
            n => format!("a fullest harmony of {n} parts"),
        };
        lines.push(format!("{harvests}, {pests}, and {harmony}."));
        let score = g.score();
        let slot = g.slot.clone();
        if let Some(c) = self.challenge.clone().filter(|_| score == 0) {
            // An empty garden is not a finished challenge: no score, no streak.
            lines.push(format!("Nothing grew, so no score. {}", c.resets()));
        } else if let Some(c) = self.challenge.clone() {
            let mut line = format!("{} score: {score}.", c.title());
            match Challenges::submit(ctx, &c, score) {
                ScoreResult::First => line.push_str(" Your first score on this one."),
                ScoreResult::NewBest { previous } => {
                    line.push_str(&format!(" A new best, beating {previous}."))
                }
                ScoreResult::Short { best } => line.push_str(&format!(" Your best stays {best}.")),
            }
            let streak = Challenges::streak(ctx, ID, c.period);
            if streak > 0 {
                line.push_str(&format!(" A {streak} {} streak.", c.period.unit()));
            }
            line.push(' ');
            line.push_str(&c.resets());
            lines.push(line);
        }
        forget_challenge(ctx, &slot);
        let summary = if met {
            format!("Goal met. Score {score}")
        } else if score == 0 && self.challenge.is_some() {
            "The season is over. Nothing grew".to_string()
        } else {
            format!("The season is over. Score {score}")
        };
        let page = lines.join(" ");
        let mut results = Some(match &self.challenge {
            Some(c) => after_challenge(c.period, summary, [page.clone()].to_vec()),
            None => Results::new(summary, [page.clone()], Back::Title(TITLE_NAME)),
        });
        ctx.replace(Pages::new([page]).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }

    /// Back on the rooftop after a menu: plant the chosen seed, pull up
    /// the plant, or just say where you are.
    fn back(&mut self, ctx: &mut Ctx<Parlor>, speak: bool) {
        self.garden.keys = keys(ctx);
        ctx.audio.resume_all();
        match std::mem::replace(&mut self.asked, Asked::Nothing) {
            Asked::Seeds => match self.seed.take() {
                _ if self.pumpkin.replace(false) => self.garden.sow_pumpkin(),
                Some(s) => self.garden.sow(s),
                None => ctx.say("No seed planted."),
            },
            Asked::Compost => {
                if self.compost.replace(false) {
                    self.garden.compost();
                } else {
                    ctx.say("You leave it growing.");
                }
            }
            Asked::Nothing => {
                if speak {
                    let here = self.garden.describe_here();
                    ctx.say(format!("Back in the garden. {here}"));
                }
            }
        }
        self.play_notes(ctx);
        self.sync(ctx);
    }

    fn handle_ask(&mut self, ctx: &mut Ctx<Parlor>, ask: Ask) {
        match ask {
            Ask::Nothing => {}
            Ask::Seeds => {
                self.asked = Asked::Seeds;
                self.seed.set(None);
                self.pumpkin.set(false);
                ctx.audio.pause_all();
                let place = self.garden.place(self.garden.here());
                let choices = self
                    .garden
                    .seeds
                    .iter()
                    .map(|&s| self.garden.book.varieties(s))
                    .collect();
                let mut menu = SeedMenu::new(choices, place, self.seed.clone());
                if self.garden.kind == Kind::Home {
                    // The season may have closed since the garden opened:
                    // no pumpkin seeds off their window.
                    let today = parlor_core::challenges::today(ctx);
                    self.garden.pumpkin_season = crate::events::pumpkin_open(today);
                }
                if self.garden.pumpkin_season {
                    menu = menu.with_pumpkin(self.pumpkin.clone());
                }
                ctx.push(menu);
            }
            Ask::Compost => {
                self.asked = Asked::Compost;
                self.compost.set(false);
                let name = self
                    .garden
                    .plant_here()
                    .map(|p| p.name())
                    .unwrap_or_else(|| "plant".into());
                let flag = self.compost.clone();
                ctx.audio.pause_all();
                ctx.push(
                    ConfirmScreen::<Parlor>::new(
                        format!("Pull up the {name} and add it to the compost?"),
                        move |_ctx| flag.set(true),
                    )
                    .yes_label("Yes, pull it up")
                    .no_label("No, leave it growing"),
                );
            }
        }
    }
}

/// Starts one bar of a song part, `from` seconds in, on the music bus:
/// at its planter, or from the middle for the pad.
fn start_part(
    ctx: &mut Ctx<Parlor>,
    key: &str,
    from: f64,
    pos: Option<Vec3>,
    volume: f32,
    pitch: f32,
) -> Sound {
    let s = match pos {
        Some(pos) if from == 0.0 => ctx.audio.play_at(key, pos, Bus::MUSIC),
        _ => ctx.audio.start_with(
            key,
            Bus::MUSIC,
            Start {
                volume,
                pitch,
                from_s: from,
                ..Start::ONCE
            },
        ),
    };
    match pos {
        Some(_) if from == 0.0 => {
            ctx.audio.set_volume(s, volume);
            ctx.audio.set_pitch(s, pitch);
        }
        Some(pos) => ctx.audio.set_position(s, Some(pos)),
        None => {}
    }
    s
}

/// Starts or stops one looping sound at a planter.
fn toggle(
    ctx: &mut Ctx<Parlor>,
    slot: &mut Option<Sound>,
    want: bool,
    key: &str,
    pos: Vec3,
    volume: f32,
) {
    match (want, *slot) {
        (true, None) => {
            let s = ctx.audio.start_loop(key, Some(pos), Bus::EFFECTS);
            ctx.audio.set_volume(s, volume);
            *slot = Some(s);
        }
        (false, Some(s)) => {
            ctx.audio.stop(s, 300);
            *slot = None;
        }
        _ => {}
    }
}

impl State<Parlor> for RunScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        RUN_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        self.garden.keys = keys(ctx);
        self.garden.drain_notes();
        if self.garden.kind == Kind::Home {
            // Seeds and planters earned before this version of the garden.
            self.garden.unlocks();
            self.garden.drain_notes();
            // The calendar touches your own garden only.
            let today = parlor_core::challenges::today(ctx);
            self.garden.pumpkin_season = crate::events::pumpkin_open(today);
            self.garden.bells = crate::events::sunday_bells(today);
        }
        self.start_ambience(ctx);
        let intro = self.garden.intro();
        self.garden.last = intro.clone();
        ctx.say(intro);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.keep_song(ctx);
        self.stop_sounds(ctx);
        if !self.done && self.keep {
            save(ctx, &self.garden);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.back(ctx, true);
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        self.back(ctx, false);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        // A pad's X, Y and bumpers stand for no key, only for actions.
        let Some(key) = event
            .pressed()
            .or(ctx.action_for(event).map(|_| Key::Other(0)))
        else {
            return;
        };
        if event.is_repeat() {
            return;
        }
        if key == Key::Escape {
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        if self.done {
            return;
        }
        let g = &mut self.garden;
        let mut ask = Ask::Nothing;
        match ctx.action_for(event) {
            Some(ParlorAction::Up) => g.step(-1, 0),
            Some(ParlorAction::Down) => g.step(1, 0),
            Some(ParlorAction::Left) => g.step(0, -1),
            Some(ParlorAction::Right) => g.step(0, 1),
            Some(ParlorAction::Primary) => ask = g.water(),
            Some(ParlorAction::Secondary) => g.tend(),
            Some(ParlorAction::Status) => {
                let layers = g.status_layers();
                g.last = self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => g.repeat(),
            Some(ParlorAction::Hint) => g.speak_hint(),
            None => match key {
                Key::Return | Key::KpEnter => g.bench(),
                Key::L => g.speak_list(),
                Key::N => g.speak_requests(),
                Key::Delete | Key::Backspace => ask = g.ask_compost(),
                // Modifiers and screen-reader keys: Control silences the
                // screen reader, so the garden must not talk back.
                Key::LCtrl
                | Key::RCtrl
                | Key::LShift
                | Key::RShift
                | Key::LAlt
                | Key::RAlt
                | Key::Insert
                | Key::Other(_) => {}
                _ => ctx.say("Not a garden control. F1 lists them."),
            },
        }
        self.play_notes(ctx);
        self.sync(ctx);
        self.handle_ask(ctx, ask);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.done {
            return;
        }
        self.garden.update(dt as f32);
        self.play_notes(ctx);
        self.sync(ctx);
        self.tick_song(ctx, dt);
        self.tick_ambience(ctx, dt);
        if self.garden.finished {
            self.finish(ctx);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![self.garden.status()]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut s = controls_text(ctx);
        if let Some(goal) = self.garden.goal {
            s.push_str(&format!(" This challenge's goal: {}", goal.describe()));
        }
        s
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// The seed packet: choose what to plant in an empty planter. A species
/// with varieties found opens a list of them.
pub struct SeedMenu {
    menu: MenuCore<Self, Parlor>,
    /// Each unlocked species' varieties, the plain seed first.
    seeds: Vec<Vec<Variety>>,
    chosen: Rc<Cell<Option<Variety>>>,
    /// Set when the pumpkin is chosen, in pumpkin season.
    pumpkin: Option<Rc<Cell<bool>>>,
}

impl SeedMenu {
    pub fn new(
        seeds: Vec<Vec<Variety>>,
        place: String,
        chosen: Rc<Cell<Option<Variety>>>,
    ) -> SeedMenu {
        SeedMenu {
            menu: MenuCore::new(format!("Seeds for {place}")).with_intro_help(
                "Up and Down choose a seed, Enter plants it, Escape plants nothing.",
            ),
            seeds,
            chosen,
            pumpkin: None,
        }
    }

    /// Adds the pumpkin seed, for pumpkin season.
    pub fn with_pumpkin(mut self, chosen: Rc<Cell<bool>>) -> SeedMenu {
        self.pumpkin = Some(chosen);
        self
    }
}

impl Menu for SeedMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "rooftop_garden_seeds"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::shop_music::start(ctx, cues::MUSIC_SHED);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::shop_music::stop(ctx);
    }

    /// Back from a species' varieties: plant what was chosen there.
    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.chosen.get().is_some() {
            ctx.pop_quiet();
        } else {
            self.refresh(ctx, true);
        }
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = self
            .seeds
            .iter()
            .filter(|kinds| !kinds.is_empty())
            .map(|kinds| {
                let s = kinds[0].species;
                if kinds.len() == 1 {
                    let plain = kinds[0];
                    MenuItem::new(
                        format!("{}, sings {}", sentence(s.name()), s.voice()),
                        move |m: &mut Self, ctx: &mut Ctx<Parlor>| {
                            m.chosen.set(Some(plain));
                            ctx.pop_quiet();
                        },
                    )
                    .help(s.tip())
                } else {
                    let list = kinds.clone();
                    MenuItem::new(
                        format!("{}, {} kinds", sentence(s.name()), kinds.len()),
                        move |m: &mut Self, ctx: &mut Ctx<Parlor>| {
                            ctx.push(VarietyMenu::new(list.clone(), m.chosen.clone()));
                        },
                    )
                    .help("Enter lists the plain seed and every variety you have found.")
                }
            })
            .collect();
        if let Some(flag) = self.pumpkin.clone() {
            items.push(
                MenuItem::new(
                    "Pumpkin, sings a low bassoon, until Thanksgiving",
                    move |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                        flag.set(true);
                        ctx.pop_quiet();
                    },
                )
                .help("A seasonal seed. It grows just like a squash and takes the squash's part in the song, as a low bassoon."),
            );
        }
        items.push(
            MenuItem::new("Plant nothing", |_m: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop_quiet()
            })
            .help("Close the seed packet."),
        );
        items
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop_quiet();
    }
}

/// One species' seeds: the plain seed and each variety found.
pub struct VarietyMenu {
    menu: MenuCore<Self, Parlor>,
    kinds: Vec<Variety>,
    chosen: Rc<Cell<Option<Variety>>>,
}

impl VarietyMenu {
    pub fn new(kinds: Vec<Variety>, chosen: Rc<Cell<Option<Variety>>>) -> VarietyMenu {
        let species = kinds.first().map_or("Seeds", |v| v.species.name());
        VarietyMenu {
            menu: MenuCore::new(sentence(species)).with_intro_help(
                "Up and Down choose a variety, Enter plants it, Escape goes back to the packet.",
            ),
            kinds,
            chosen,
        }
    }
}

impl Menu for VarietyMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "rooftop_garden_varieties"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        self.kinds
            .iter()
            .map(|&v| {
                let label = if v.is_plain() {
                    format!("Plain {}, on {}", v.species.name(), v.tone())
                } else {
                    format!("{}, {}", sentence(&v.name()), v.describe())
                };
                MenuItem::new(label, move |m: &mut Self, ctx: &mut Ctx<Parlor>| {
                    m.chosen.set(Some(v));
                    ctx.pop_quiet();
                })
                .help(v.species.tip())
            })
            .collect()
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }
}

mod visuals_runscreen;
