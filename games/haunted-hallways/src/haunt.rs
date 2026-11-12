//! The haunt itself: walking the rooms by ear.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::status::sentences;
use parlor_core::{
    Challenge, Challenges, Mood, Pace, Pages, Parlor, ParlorAction, Period, Records, Rng, RunClock,
    Score, ScoreResult, StatusKey,
};

use crate::challenge::{self, challenge_name};
use crate::progress::{self, FloorSaved, Progress};
use crate::rules::{
    clock_words, floor_spec, time_short, wing, CharmKind, Ev, GhostPhase, Item, Mode, Outcome,
    Puzzle, Run, Side, Threat, Twist, COURAGE, DARK_REACH, FLOORS, STUB_SECONDS,
};
use crate::{sound, unlocks, CHALLENGES_NAME, HELP, TITLE_NAME};

/// The Records key for the cues a save has been taught.
pub fn taught_key() -> String {
    format!("{}/taught", crate::ID)
}

/// How long after the phonograph or spirit begins its clue sound plays.
const CLUE_DELAY: f64 = 0.9;
/// How long thunder and screams wait after a clue sound.
const ATMOS_HUSH: f64 = 4.0;
/// The gap between a séance's or a phonograph's sounds.
const SEANCE_GAP: f64 = 1.6;
/// How loud things call out with the lights out.
const LIGHTS_OUT_VOLUME: f32 = 0.45;
/// A ghost's voice sits under speech, not over it.
const GHOST_VOLUME: f32 = 0.7;
/// The shadow's drone, under speech too.
const SHADOW_VOLUME: f32 = 0.6;

pub struct Haunt {
    pub run: Run,
    /// The run's game clock: its pace, set when the haunt begins, and the
    /// hold that keeps the house still while speech plays.
    pub clock: RunClock,
    last_prompt: String,
    /// The exit's place as the last prompt words it, so a shifting door
    /// can be put right in what R repeats.
    prompt_door: String,
    ghost_loop: Sound,
    shadow_loop: Sound,
    step_left: bool,
    /// Cues already explained in words, kept in the save so each is
    /// explained once (the practice house keeps its own).
    taught: HashSet<String>,
    /// A cue was explained since the save last heard.
    taught_new: bool,
    /// Clue sounds still to play: seconds until each, and the thing.
    clue: Vec<(f64, Item)>,
    atmos_in: f64,
    atmos: Rng,
    /// Seconds the atmosphere holds back after a clue, so thunder never
    /// masks one.
    hush: f64,
    started: bool,
    /// The last floor's summary, kept for the results when the run ends there.
    pending_floor: Option<String>,
    /// Unlocks earned this run, for the results.
    earned: Vec<String>,
    /// Unlocks already read out on a floor's page, not read again.
    announced: Vec<String>,
    /// The challenge this run is, to record its result.
    challenge: Option<Challenge>,
    /// The mansion's music, following the run.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: StatusKey,
    /// The second layer of S has been heard, for the tutorial.
    pub heard_more: bool,
}

impl Haunt {
    pub fn new(mode: Mode) -> Haunt {
        let seed = crate::rules::run_seed();
        Haunt::with_seed(mode, seed)
    }

    /// A haunt with the player's curses on.
    pub fn start(ctx: &mut Ctx<Parlor>, mode: Mode) -> Haunt {
        let curses = Progress::load(ctx).curses();
        let seed = crate::rules::run_seed();
        let mut haunt = Haunt::with_seed(mode, seed);
        haunt.run = Run::cursed(mode, seed, curses);
        haunt
    }

    /// The climb, from the highest open floor.
    pub fn climb(ctx: &mut Ctx<Parlor>) -> Haunt {
        let from = Progress::load(ctx).frontier();
        Haunt::start(ctx, Mode::Climb(from))
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Haunt {
        Haunt {
            run: Run::new(mode, seed),
            clock: RunClock::new(Pace::Normal),
            last_prompt: String::new(),
            prompt_door: String::new(),
            ghost_loop: Sound::NONE,
            shadow_loop: Sound::NONE,
            step_left: false,
            taught: HashSet::new(),
            taught_new: false,
            clue: Vec::new(),
            atmos_in: 40.0,
            hush: 0.0,
            atmos: Rng::seeded(seed ^ 0xA7A7),
            started: false,
            pending_floor: None,
            earned: Vec::new(),
            announced: Vec::new(),
            challenge: None,
            score: Score::new(),
            status_key: StatusKey::new(),
            heard_more: false,
        }
    }

    /// What the score hears: deeper into the floor, and deeper into the
    /// house, builds it; the last half minute of the haunt clock hurries
    /// it; a ghost drifting near or the shadow close by darkens it.
    pub fn mood(&self) -> Mood {
        let run = &self.run;
        let rooms = run.rooms.len().max(1) as f32;
        let through = run.room as f32 / rooms;
        let height = (run.spec.number.saturating_sub(1) as f32 / (FLOORS - 1) as f32).min(1.0);
        let ghost = run
            .ghost
            .is_some_and(|g| matches!(g.phase, GhostPhase::Approaching | GhostPhase::Near));
        let shadow = run
            .current()
            .shadow
            .is_some_and(|s| (s.x - run.x).abs() + (s.y - run.y).abs() <= 2);
        Mood::new(0.2 + 0.55 * through + 0.25 * height)
            .with_flag(HURRY, run.haunt_left <= 30.0)
            .with_flag(DANGER, ghost || shadow)
    }

    /// A challenge run, the same for every player this period.
    pub fn challenge(ch: Challenge) -> Haunt {
        let (event, seed) = crate::rules::Event::from_challenge(&ch);
        let mut haunt = Haunt::with_seed(Mode::Challenge(event), seed);
        haunt.challenge = Some(ch);
        haunt
    }

    /// "Ten seconds", or more on deeper floors and under Cursed hands.
    fn penalty_words(&self) -> String {
        sentence_start(&clock_words(self.run.spec.penalty))
    }

    /// True the first time a cue is met on this save.
    fn first(&mut self, cue: impl Into<String>) -> bool {
        let new = self.taught.insert(cue.into());
        self.taught_new |= new;
        new
    }

    /// Loads the cues this save has already been taught.
    fn load_taught(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.run.practice {
            return;
        }
        let saved: HashSet<String> = Records::load(ctx, &taught_key());
        self.taught.extend(saved);
    }

    /// Saves newly taught cues, outside the practice house.
    fn keep_taught(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.taught_new || self.run.practice {
            return;
        }
        self.taught_new = false;
        let mut cues: Vec<&String> = self.taught.iter().collect();
        cues.sort();
        Records::store(ctx, &taught_key(), &cues);
    }

    /// Plays a sound from a cell offset, flipped east for west in a
    /// mirrored room.
    fn play_at(&self, ctx: &mut Ctx<Parlor>, key: &str, dx: f32, dy: f32) -> Sound {
        let dx = if self.run.current().mirror { -dx } else { dx };
        sound::play_at(ctx, key, dx, dy)
    }

    fn stop_ghost(&mut self, ctx: &mut Ctx<Parlor>) {
        self.fade_ghost(ctx, 400);
    }

    fn fade_ghost(&mut self, ctx: &mut Ctx<Parlor>, ms: u32) {
        if self.ghost_loop != Sound::NONE {
            ctx.audio.stop(self.ghost_loop, ms);
            self.ghost_loop = Sound::NONE;
        }
    }

    fn stop_shadow(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.shadow_loop != Sound::NONE {
            ctx.audio.stop(self.shadow_loop, 400);
            self.shadow_loop = Sound::NONE;
        }
    }

    /// Where the shadow sounds from, around the listener.
    fn shadow_point(&self) -> Option<Vec3> {
        let s = self.run.current().shadow?;
        let dx = (s.x - self.run.x) as f32;
        let dx = if self.run.current().mirror { -dx } else { dx };
        Some(sound::at(dx, (s.y - self.run.y) as f32))
    }

    fn start_shadow(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_shadow(ctx);
        if let Some(at) = self.shadow_point() {
            self.shadow_loop = ctx.audio.start_loop(sound::SHADOW, Some(at), Bus::EFFECTS);
            ctx.audio.set_volume(self.shadow_loop, SHADOW_VOLUME);
        }
    }

    /// The puzzle's instructions, in the player's own controls: in full
    /// the first time a kind of puzzle is met (and in F1), short after.
    fn puzzle_prompt(&self, ctx: &Ctx<Parlor>, full: bool) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let r = sentence_start(&ctx.control_name(ParlorAction::Repeat));
        let t = ctx.control_name(ParlorAction::Secondary);
        let room = self.run.current();
        let real = room.items.iter().filter(|i| !i.mimic).count();
        let decoys = if full && real > room.puzzle.wanted().len() {
            format!(" Wrong things cost {}.", clock_words(self.run.spec.penalty))
        } else {
            String::new()
        };
        match (room.puzzle, full) {
            (Puzzle::Find(item), true) => format!(
                "Find the {}: {}. Press {p} on it.{decoys}",
                item.name(),
                item.sound()
            ),
            (Puzzle::Find(item), false) => {
                format!("Find the {}: {}.", item.name(), item.sound())
            }
            (Puzzle::Match(_), true) => format!(
                "Match the phonograph's sound: press {p} on the thing that makes it. {r} replays it.{decoys}"
            ),
            (Puzzle::Match(_), false) => format!("Match the phonograph. {r} replays it."),
            (Puzzle::Riddle(item), true) => format!(
                "A riddle. {} Press {p} on the answer. {r} repeats it.{decoys}",
                item.riddle()
            ),
            (Puzzle::Riddle(item), false) => {
                format!("A riddle. {} {r} repeats it.", item.riddle())
            }
            (Puzzle::Missing(_), true) => format!(
                "The phonograph plays every sound here but one. Press {p} on the missing one. {r} replays it.{decoys}"
            ),
            (Puzzle::Missing(_), false) => {
                format!("Find the missing sound. {r} replays the phonograph.")
            }
            (Puzzle::Offering(_), true) => format!(
                "A spirit seals the exit, in {}, and plays the sound it wants. {} chooses what you hold, {p} offers it at the door. {r} replays the sound.",
                self.run.door_words(room.exit),
                sentence_start(&t)
            ),
            (Puzzle::Offering(_), false) => format!(
                "A spirit seals the exit, in {}. {r} replays its sound.",
                self.run.door_words(room.exit)
            ),
            (Puzzle::Seance(seq), full) => {
                let n = seq.len();
                let which = if room.stage > 0 {
                    format!(" {} of {n} found.", room.stage)
                } else if full {
                    format!(" Press {p} on each thing in turn. Wrong order costs time.")
                } else {
                    String::new()
                };
                format!("A séance, {n} sounds in order.{which} {r} replays them.{decoys}")
            }
        }
    }

    /// What is strange about this room, taught in full the first time.
    fn hazard_prompt(&mut self, ctx: &Ctx<Parlor>) -> String {
        let room = self.run.current().clone();
        let mut out = String::new();
        if room.dark {
            if self.first("dark") {
                out.push_str(&format!(
                    " A dark room: things call out only within {} steps.",
                    DARK_REACH as u32
                ));
            } else {
                out.push_str(" A dark room.");
            }
        }
        if room.mirror {
            if self.first("mirror") {
                out.push_str(" A mirrored room: east and west sounds swap sides.");
            } else {
                out.push_str(" A mirrored room: east sounds from your left.");
            }
        }
        if !room.rotten.is_empty() {
            let n = room.rotten.len();
            let boards = if n == 1 {
                "1 rotten board".to_string()
            } else {
                format!("{n} rotten boards")
            };
            if self.first("rotten") {
                out.push_str(&format!(
                    " {}. A groan means one is next to you.",
                    sentence_start(&boards)
                ));
            } else {
                out.push_str(&format!(" {}.", sentence_start(&boards)));
            }
        }
        if room.shadow.is_some() {
            if self.first("shadow") {
                let s = ctx.control_name(ParlorAction::Status);
                out.push_str(&format!(
                    " A shadow, the breathing drone, follows your footsteps. {} says where it is.",
                    sentence_start(&s)
                ));
            } else {
                out.push_str(" The shadow walks here.");
            }
        }
        if let Some(p) = room.pendulum {
            if self.first("pendulum") {
                out.push_str(&format!(
                    " A pendulum swings along row {} from the south. Cross just after the swish passes.",
                    p.row + 1
                ));
            } else {
                out.push_str(&format!(" The pendulum swings along row {}.", p.row + 1));
            }
        }
        if !room.water.is_empty() {
            if self.first("water") {
                out.push_str(
                    " Flooded. Lapping means deep water. Wading is slow and draws ghosts.",
                );
            } else {
                out.push_str(" Deep water here.");
            }
        }
        if room.stones {
            if self.first("stones") {
                out.push_str(" Stones fall here. When dust trickles, step off.");
            } else {
                out.push_str(" Stones fall here.");
            }
        }
        if room.shifts {
            if self.first("shifts") {
                out.push_str(" This room shifts: a rumble moves the exit door.");
            } else {
                out.push_str(" This room shifts.");
            }
        }
        if room.charm.is_some_and(|c| !c.taken) && self.first("charm") {
            out.push_str(&format!(
                " A soft chime is a charm: walk onto it. Salt wards off one catch, a candle stub adds {}.",
                clock_words(STUB_SECONDS)
            ));
        }
        if room.mimics() > 0 {
            if self.first("mimic") {
                out.push_str(" Mimics here: copies that hop about. The real thing keeps still.");
            } else if room.mimics() == 1 {
                out.push_str(" A mimic is here.");
            } else {
                out.push_str(&format!(" {} mimics are here.", room.mimics()));
            }
        }
        out
    }

    /// Plays the room's clue, for matching, missing, offering and séance rooms.
    fn start_clue(&mut self, ctx: &mut Ctx<Parlor>) {
        let room = self.run.current();
        if room.solved {
            return;
        }
        match room.puzzle {
            Puzzle::Match(item) => {
                sound::play(ctx, sound::PHONOGRAPH);
                self.clue = vec![(CLUE_DELAY, item)];
            }
            Puzzle::Missing(item) => {
                sound::play(ctx, sound::PHONOGRAPH);
                let mut others: Vec<Item> = room
                    .items
                    .iter()
                    .filter(|p| !p.mimic && p.item != item)
                    .map(|p| p.item)
                    .collect();
                others.dedup();
                self.clue = others
                    .into_iter()
                    .enumerate()
                    .map(|(i, it)| (CLUE_DELAY + SEANCE_GAP * i as f64, it))
                    .collect();
            }
            Puzzle::Offering(item) => {
                let (dx, dy) = room.door_point(room.exit);
                let (x, y) = (self.run.x as f32, self.run.y as f32);
                self.play_at(ctx, sound::SPIRIT, dx - x, dy - y);
                self.clue = vec![(CLUE_DELAY, item)];
            }
            Puzzle::Seance(seq) => {
                sound::play(ctx, sound::SEANCE);
                self.clue = seq
                    .items()
                    .iter()
                    .enumerate()
                    .map(|(i, &it)| (CLUE_DELAY + SEANCE_GAP * i as f64, it))
                    .collect();
            }
            _ => {}
        }
    }

    fn room_prompt(&mut self, ctx: &mut Ctx<Parlor>, index: usize, of: usize) {
        let kind = match self.run.current().puzzle {
            Puzzle::Find(_) => "find",
            Puzzle::Match(_) => "match",
            Puzzle::Riddle(_) => "riddle",
            Puzzle::Missing(_) => "missing",
            Puzzle::Offering(_) => "offering",
            Puzzle::Seance(_) => "seance",
        };
        let full = self.first(format!("puzzle {kind}"));
        let puzzle = self.puzzle_prompt(ctx, full);
        let room = self.run.current();
        let name = sentence_start(room.name);
        self.prompt_door = self.run.door_words(room.exit);
        let door = if matches!(room.puzzle, Puzzle::Offering(_)) {
            String::new()
        } else {
            format!(" The exit is in {}.", self.prompt_door)
        };
        // In the practice house the lesson teaches the room, so the room
        // gives its name, the thing to find if it names one, and its door.
        let text = if self.run.practice {
            let find = match room.puzzle {
                Puzzle::Find(item) => format!(" Find the {}.", item.name()),
                _ => String::new(),
            };
            format!("Room {} of {of}: {name}.{find}{door}", index + 1)
        } else {
            let hazards = self.hazard_prompt(ctx);
            format!(
                "Room {} of {of}: {name}.{hazards} {puzzle}{door}",
                index + 1
            )
        };
        self.last_prompt = text.clone();
        self.clock.reset();
        ctx.say_queued(text);
        self.start_clue(ctx);
        self.start_shadow(ctx);
    }

    /// "Floor 37 of 100", or the challenge's own name.
    fn floor_words(&self) -> String {
        let run = &self.run;
        match run.mode {
            Mode::Floor(n) => format!("Floor {n}"),
            Mode::Challenge(e) if run.spec.seance => {
                format!("{}, the hidden wing", challenge_name(e.period))
            }
            Mode::Challenge(e) => {
                let twist = e
                    .twist
                    .map(|t| format!(", twist {}", t.name()))
                    .unwrap_or_default();
                format!(
                    "{}{twist}, floor {} of {}, floor {}",
                    challenge_name(e.period),
                    run.spec.number - e.first + 1,
                    e.floors(),
                    run.spec.number
                )
            }
            Mode::Endless => format!("Endless night, floor {}", run.spec.number),
            Mode::Nightmare => format!("Nightmare, floor {} of {FLOORS}", run.spec.number),
            Mode::Climb(_) => format!("Floor {} of {FLOORS}", run.spec.number),
            Mode::Stirs(n) => format!("{}, floor {n}", crate::allhallows::STIRS),
            Mode::LanternNight(_) => format!(
                "{}, floor {} of {}",
                crate::allhallows::NIGHT,
                run.spec.number,
                crate::allhallows::CANDLES
            ),
        }
    }

    /// What S says, in two layers, most urgent first. The first press: any
    /// danger, then the clock, courage and the way out. A second press
    /// straight away: where you are, the puzzle and the rest.
    pub fn status(&self) -> Vec<String> {
        let run = &self.run;
        let room = run.current();
        let ghost = run.ghost.map(|g| {
            let name = sentence_start(g.kind.name());
            match (g.phase, g.kind.hunts_stillness()) {
                (GhostPhase::Approaching, _) => {
                    format!("{name} coming from the {}.", g.from_name())
                }
                (GhostPhase::Near, true) => format!("{name} passing. Keep walking!"),
                (GhostPhase::Near, false) => format!("{name} passing. Do not move!"),
                (GhostPhase::Leaving, _) => format!("{name} drifting away."),
            }
        });
        let shadow = run.shadow_words().map(|w| format!("Shadow {w}."));
        let boards = run.groan_words().map(|w| format!("Rotten board {w}."));
        let founder = run.founder.map(|f| {
            let way = run.founder_words().unwrap_or_default();
            if f.wind.is_some() {
                format!("The founder {way}, winding up a blow!")
            } else {
                format!("The founder {way}.")
            }
        });
        let dust = run
            .stone
            .filter(|st| (st.x, st.y) == (run.x, run.y))
            .map(|_| "Dust above you. Move!".to_string());
        let blade = run
            .pendulum_words()
            .filter(|w| w == "in your row")
            .map(|_| "The blade swings in your row!".to_string());
        let way = run.exit_words();
        let door = if room.solved {
            "open"
        } else if matches!(room.puzzle, Puzzle::Offering(_)) {
            "sealed"
        } else {
            "locked"
        };
        let exit = format!("Exit door {door}, {way}.");
        let brief = sentences([
            ghost.unwrap_or_default(),
            founder.unwrap_or_default(),
            dust.unwrap_or_default(),
            blade.unwrap_or_default(),
            shadow.unwrap_or_default(),
            boards.unwrap_or_default(),
            format!(
                "{} left, courage {} of {COURAGE}.",
                sentence_start(&clock_words(run.haunt_left)),
                run.courage
            ),
            exit,
        ]);
        let strange = match (room.dark, room.mirror) {
            (true, true) => "Dark and mirrored.",
            (true, false) => "Dark.",
            (false, true) => "Mirrored.",
            _ => "",
        };
        let puzzle = format!(
            "Puzzle: {}{}.",
            room.puzzle.kind_name(),
            if room.solved { ", solved" } else { "" }
        );
        let held = run
            .inventory
            .get(run.selected)
            .map(|it| format!("Holding the {}.", it.name()))
            .unwrap_or_default();
        let mut extra: Vec<String> = Vec::new();
        if let Some(w) = run.pendulum_words().filter(|w| w != "in your row") {
            extra.push(format!("The pendulum swings {w}."));
        }
        if !room.water.is_empty() {
            extra.push("Part flooded.".to_string());
        }
        if room.stones {
            extra.push("Stones fall here.".to_string());
        }
        if room.shifts {
            extra.push("The exit door shifts.".to_string());
        }
        if run.founder_active() {
            let hits = run.founder_health;
            extra.push(if hits == 1 {
                "1 more hit brings the founder down.".to_string()
            } else {
                format!("{hits} more hits bring the founder down.")
            });
        }
        let salt = match run.salt {
            0 => String::new(),
            1 => "1 pinch of salt.".to_string(),
            n => format!("{n} pinches of salt."),
        };
        let more = sentences([
            format!("{}, {}.", self.floor_words(), run.spec.name),
            format!(
                "{}, room {} of {}.",
                sentence_start(room.name),
                run.room + 1,
                run.rooms.len()
            ),
            strange.to_string(),
            extra.join(" "),
            puzzle,
            format!("You are at {}.", run.place_words()),
            held,
            salt,
            format!("Score {}.", run.score),
        ]);
        vec![brief, more]
    }

    /// The floor's own twists, spoken as it begins. A challenge's twist is
    /// left out, as its briefing says it, and so are the restless ghosts
    /// and dark of Halloween, which the event's opening names.
    fn twist_words(&self) -> String {
        let run = &self.run;
        let said: Vec<Twist> = match run.mode {
            Mode::Stirs(_) => vec![Twist::Restless],
            Mode::LanternNight(_) => vec![Twist::Restless, Twist::LightsOut],
            m => m.event().and_then(|e| e.twist).into_iter().collect(),
        };
        let twists: Vec<Twist> = run
            .spec
            .twists
            .iter()
            .copied()
            .filter(|t| !said.contains(t))
            .collect();
        if twists.is_empty() {
            return String::new();
        }
        let parts: Vec<String> = twists
            .iter()
            .map(|t| {
                let curse = if run.curses.has(*t) {
                    ", your curse"
                } else {
                    ""
                };
                format!("{}{curse}: {}", t.name(), t.describe())
            })
            .collect();
        let word = if parts.len() == 1 { "Twist" } else { "Twists" };
        format!(" {word}: {}.", parts.join(". "))
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        self.voice_events(ctx, evs);
        self.keep_taught(ctx);
    }

    fn voice_events(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        let p = ctx.control_name(ParlorAction::Primary);
        for ev in evs {
            match ev {
                Ev::FloorBegins {
                    number,
                    name,
                    rooms,
                    haunt,
                } => {
                    sound::floor_bed(ctx, number);
                    let mood = self.mood();
                    self.score.start(ctx, &sound::SCORE, mood);
                    let courage = if self.run.courage < COURAGE {
                        format!("Courage still {} of {COURAGE}.", self.run.courage)
                    } else {
                        format!("Courage {COURAGE}.")
                    };
                    let head = if self.run.practice {
                        format!("The practice house, {rooms} rooms. No clock, no courage lost.")
                    } else if self.run.spec.seance {
                        format!(
                            "The hidden wing: {}. {rooms} rooms, {} on the clock. {courage}",
                            sentence_start(name),
                            clock_words(haunt)
                        )
                    } else {
                        let first_of_wing = (number - 1) % crate::rules::WING_FLOORS == 0
                            || number == self.run.mode.first_floor();
                        let wing_line = if first_of_wing {
                            format!(": {name}")
                        } else {
                            String::new()
                        };
                        format!(
                            "Floor {number}{wing_line}. {rooms} rooms, {} on the clock. {courage}",
                            clock_words(haunt)
                        )
                    };
                    let twist = self.twist_words();
                    let opening = match self.run.mode {
                        _ if number != self.run.mode.first_floor() => String::new(),
                        Mode::Stirs(_) => format!("{} ", crate::allhallows::stirs_opening()),
                        Mode::LanternNight(lit) => {
                            format!("{} ", crate::allhallows::night_opening(lit))
                        }
                        _ => String::new(),
                    };
                    ctx.say(format!("{opening}{head}{twist}"));
                }
                Ev::EnterRoom { index, of } => self.room_prompt(ctx, index, of),
                Ev::Step => {
                    let key = if wing(self.run.spec.number).stone && !self.run.spec.seance {
                        sound::STEP_STONE
                    } else if self.step_left {
                        sound::STEP_WOOD_A
                    } else {
                        sound::STEP_WOOD_B
                    };
                    self.step_left = !self.step_left;
                    sound::play(ctx, key);
                }
                Ev::Bump(side) => {
                    sound::play(ctx, sound::BUMP);
                    if self.first("bump") {
                        let st = ctx.control_name(ParlorAction::Status);
                        ctx.say(format!(
                            "{} wall. {} twice says where you stand.",
                            sentence_start(side.name()),
                            sentence_start(&st)
                        ));
                    } else {
                        ctx.say(format!("{} wall.", sentence_start(side.name())));
                    }
                }
                Ev::Hole(side) => {
                    sound::play(ctx, sound::BUMP);
                    ctx.say(format!("A hole in the floor, {}.", side.name()));
                }
                Ev::Groan { dx, dy } => {
                    self.play_at(ctx, sound::GROAN, dx, dy);
                    if !self.run.practice && self.first("groan") {
                        let side = Side::of(dx as i32, dy as i32)
                            .map(Side::name)
                            .unwrap_or("next to you");
                        ctx.say_queued(format!("Rotten board, {side}."));
                    }
                }
                Ev::DoorLocked { sealed } => {
                    sound::play(ctx, sound::DOOR_LOCKED);
                    if sealed {
                        ctx.say(format!(
                            "Sealed. {} offers what you hold.",
                            sentence_start(&p)
                        ));
                    } else {
                        ctx.say("Locked. Solve the room first.");
                    }
                }
                Ev::StairsBarred => {
                    sound::play(ctx, sound::DOOR_LOCKED);
                    ctx.say("Stairs barred in practice.");
                }
                Ev::DoorShut => {
                    sound::play(ctx, sound::DOOR_SHUT);
                    ctx.say("That door is shut. No going back.");
                }
                Ev::DoorOpened => {
                    self.stop_ghost(ctx);
                    self.stop_shadow(ctx);
                    self.clue.clear();
                    sound::play(ctx, sound::DOOR_OPEN);
                    ctx.say("Through the door.");
                }
                Ev::OnItem { item, spent } => {
                    if spent {
                        ctx.say(format!("The {}, silent now.", item.name()));
                    } else if self.first("on item") {
                        ctx.say(format!(
                            "Here: the {}. {} picks it up.",
                            item.name(),
                            sentence_start(&p)
                        ));
                    } else {
                        ctx.say(format!("Here: the {}.", item.name()));
                    }
                }
                Ev::PickedUp(item) => {
                    sound::play(ctx, sound::PICKUP);
                    ctx.say(format!("Took the {}.", item.name()));
                }
                Ev::SeanceStep { done, of, .. } => {
                    sound::play(ctx, sound::SEANCE_STEP);
                    let left = of - done;
                    let line = if left == 1 {
                        format!("{done} of {of}. Last one.")
                    } else {
                        format!("{done} of {of}. {left} to go.")
                    };
                    self.last_prompt = line.clone();
                    ctx.say_queued(line);
                }
                Ev::NoHints => {
                    sound::play(ctx, sound::NOTHING);
                    ctx.say("No hints on this floor.");
                }
                Ev::WrongItem(item) => {
                    sound::play(ctx, sound::WRONG);
                    let order = if self.run.current().puzzle.wanted().contains(&item) {
                        " Wrong order."
                    } else {
                        ""
                    };
                    ctx.say(format!(
                        "Not the {}.{order} {} lost.",
                        item.name(),
                        self.penalty_words()
                    ));
                }
                Ev::Mimic(item) => {
                    sound::play(ctx, sound::MIMIC);
                    ctx.say(format!(
                        "A mimic {}. {} lost.",
                        item.name(),
                        self.penalty_words()
                    ));
                }
                Ev::Solved { points, .. } => {
                    sound::play(ctx, sound::SOLVED);
                    sound::play(ctx, sound::DOOR_UNLOCK);
                    self.clue.clear();
                    let follow = if !self.run.practice && self.first("draft") {
                        " Follow the cold draught."
                    } else {
                        ""
                    };
                    self.prompt_door = self.run.door_words(self.run.current().exit);
                    let line = format!(
                        "Solved. Plus {points}. Exit open in {}.{follow}",
                        self.prompt_door
                    );
                    self.last_prompt = line.clone();
                    // In practice the lesson says what solving did.
                    if !self.run.practice {
                        ctx.say_queued(line);
                    }
                }
                Ev::NothingHere => {
                    sound::play(ctx, sound::NOTHING);
                    ctx.say("Nothing here.");
                }
                Ev::Leftover(item) => {
                    sound::play(ctx, sound::NOTHING);
                    ctx.say(format!("The {} is silent. The door is open.", item.name()));
                }
                Ev::AtDoor { open } => {
                    let side = self.run.current().exit.side;
                    if open {
                        ctx.say(format!("Open. Walk {} to go through.", side.name()));
                    } else {
                        sound::play(ctx, sound::DOOR_LOCKED);
                        ctx.say("Locked. Solve the room first.");
                    }
                }
                Ev::NotAtDoor(side) => {
                    sound::play(ctx, sound::NOTHING);
                    ctx.say(format!("Offer at the sealed door, {} wall.", side.name()));
                }
                Ev::OfferAccepted(item) => {
                    sound::play(ctx, sound::OFFER_OK);
                    sound::play(ctx, sound::item(item));
                    if !self.run.practice {
                        ctx.say(format!("The spirit takes the {}.", item.name()));
                    }
                }
                Ev::OfferRefused(item) => {
                    sound::play(ctx, sound::OFFER_BAD);
                    ctx.say(format!(
                        "Refused, not the {}. {} lost.",
                        item.name(),
                        self.penalty_words()
                    ));
                }
                Ev::EmptyHands => {
                    sound::play(ctx, sound::NOTHING);
                    ctx.say("Your hands are empty.");
                }
                Ev::Selected { item, index, of } => {
                    sound::play(ctx, sound::SELECT);
                    sound::play(ctx, sound::item(item));
                    ctx.say(format!("The {}, {} of {of}.", item.name(), index + 1));
                }
                Ev::Hint(text) => {
                    sound::play(ctx, sound::HINT);
                    ctx.say(format!(
                        "{text} {} spent.",
                        sentence_start(&clock_words(self.run.spec.hint_cost))
                    ));
                }
                Ev::Emit { item, dx, dy } => {
                    let s = self.play_at(ctx, sound::item(item), dx, dy);
                    if self.run.spec.lights_out {
                        ctx.audio.set_volume(s, LIGHTS_OUT_VOLUME);
                    }
                }
                Ev::Draft { dx, dy } => {
                    self.play_at(ctx, sound::DRAFT, dx, dy);
                }
                Ev::Spirit { dx, dy } => {
                    self.play_at(ctx, sound::SPIRIT, dx, dy);
                }
                Ev::GhostComing(g) => {
                    self.stop_ghost(ctx);
                    let (x, y) = g.offset();
                    let x = if self.run.current().mirror { -x } else { x };
                    self.ghost_loop = ctx.audio.start_loop(
                        sound::ghost(g.kind),
                        Some(Vec3::new(x, y, 0.0)),
                        Bus::EFFECTS,
                    );
                    ctx.audio.set_volume(self.ghost_loop, GHOST_VOLUME);
                    // Short enough to finish before even the butler arrives.
                    if self.first(format!("ghost {}", g.kind.name())) {
                        let how = if g.kind.hunts_stillness() {
                            " It hunts stillness: at its howl, keep walking."
                        } else if self.first("ghost") {
                            " At the stab, freeze until it breathes out."
                        } else {
                            ""
                        };
                        self.clock.reset();
                        ctx.say(format!(
                            "A ghost! {}, {}, from the {}.{how}",
                            sentence_start(g.kind.name()),
                            g.kind.voice(),
                            g.from_name()
                        ));
                    } else if self.run.spec.call_freeze {
                        ctx.say(format!(
                            "{} comes from the {}.",
                            sentence_start(g.kind.name()),
                            g.from_name()
                        ));
                    }
                }
                Ev::GhostNear(g) => {
                    if g.kind.hunts_stillness() {
                        sound::play(ctx, sound::HOWL);
                        if self.run.spec.call_freeze || self.first("howl") {
                            self.clock.reset();
                            ctx.say("Keep walking!");
                        }
                    } else {
                        sound::play(ctx, sound::FREEZE);
                        if self.run.spec.call_freeze || self.first("freeze") {
                            self.clock.reset();
                            ctx.say("Freeze!");
                        }
                    }
                }
                Ev::GhostPassed { kind, points } => {
                    self.fade_ghost(ctx, 1500);
                    sound::play(ctx, sound::PASSED);
                    // In the practice house the lesson says it, without points.
                    let say = self.run.spec.call_freeze || self.first("passed");
                    if say && !self.run.practice {
                        ctx.say(format!(
                            "{} gone. Plus {points}.",
                            sentence_start(kind.name())
                        ));
                    }
                }
                Ev::Caught { by, courage } => {
                    self.stop_ghost(ctx);
                    let key = match by {
                        Threat::Board => sound::FALL,
                        Threat::Stone => sound::STONE,
                        _ => sound::CAUGHT,
                    };
                    sound::play(ctx, key);
                    if courage > 0 {
                        let what = match by {
                            Threat::Board => "You fall through a rotten board.".to_string(),
                            Threat::Blade => "The blade cuts you.".to_string(),
                            Threat::Stone => "A stone hits you.".to_string(),
                            Threat::Founder => "The founder's blow lands.".to_string(),
                            Threat::Shadow => "The shadow catches you.".to_string(),
                            Threat::Ghost(g) if g.hunts_stillness() => {
                                format!("{} caught you standing still.", sentence_start(g.name()))
                            }
                            Threat::Ghost(g) => {
                                format!("{} saw you move.", sentence_start(g.name()))
                            }
                        };
                        let what = format!("{what} Back to the door.");
                        if self.run.practice {
                            ctx.say(format!("{what} No courage lost in practice."));
                        } else {
                            ctx.say(format!("{what} Courage {courage} of {COURAGE}."));
                        }
                        // The shadow went home: its drone moves with it.
                        if let (Some(at), true) =
                            (self.shadow_point(), self.shadow_loop != Sound::NONE)
                        {
                            ctx.audio.set_position(self.shadow_loop, Some(at));
                        }
                    }
                }
                Ev::CharmCall { dx, dy } => {
                    self.play_at(ctx, sound::CHARM, dx, dy);
                }
                Ev::Charm(kind) => {
                    sound::play(ctx, sound::CHARM_TAKE);
                    let line = match kind {
                        CharmKind::Salt => {
                            let n = self.run.salt;
                            let held = if n == 1 {
                                "You carry 1 pinch".to_string()
                            } else {
                                format!("You carry {n} pinches")
                            };
                            if self.first("salt") {
                                format!("Salt. {held}. It wards off the next catch.")
                            } else {
                                format!("Salt. {held}.")
                            }
                        }
                        CharmKind::Stub => format!(
                            "Candle stub. Plus {} on the clock.",
                            clock_words(STUB_SECONDS)
                        ),
                    };
                    // In practice the lesson names the charm.
                    if !self.run.practice {
                        ctx.say(line);
                    }
                }
                Ev::Warded(by) => {
                    self.stop_ghost(ctx);
                    sound::play(ctx, sound::SALT);
                    let left = match self.run.salt {
                        0 => "No salt left.".to_string(),
                        1 => "1 pinch left.".to_string(),
                        n => format!("{n} pinches left."),
                    };
                    let what = match by {
                        Threat::Founder => "Salt wards off the founder.".to_string(),
                        _ => format!("Salt wards off {}.", by.name()),
                    };
                    ctx.say(format!("{what} {left}"));
                    if let (Some(at), true) = (self.shadow_point(), self.shadow_loop != Sound::NONE)
                    {
                        ctx.audio.set_position(self.shadow_loop, Some(at));
                    }
                }
                Ev::Swing { dx, dy } => {
                    self.play_at(ctx, sound::SWING, dx, dy);
                }
                Ev::Splash => {
                    sound::play(ctx, sound::SPLASH);
                    if self.first("splash") {
                        ctx.say("Deep water. Slow, and the splash carries.");
                    }
                }
                Ev::Wading => {
                    sound::play(ctx, sound::LAP);
                    ctx.say("Still wading.");
                }
                Ev::Lap { dx, dy } => {
                    self.play_at(ctx, sound::LAP, dx, dy);
                    if self.first("lap") {
                        let side = Side::of(dx as i32, dy as i32)
                            .map(Side::name)
                            .unwrap_or("next to you");
                        ctx.say_queued(format!("Deep water, {side}."));
                    }
                }
                Ev::Dust => {
                    sound::play(ctx, sound::DUST);
                    ctx.say("Dust! Step off.");
                }
                Ev::StoneFell { dx, dy } => {
                    self.play_at(ctx, sound::STONE, dx, dy);
                    if self.first("stone missed") {
                        ctx.say("Missed you.");
                    }
                }
                Ev::FounderEnters { dx, dy } => {
                    sound::play(ctx, sound::DOOR_OPEN);
                    self.play_at(ctx, sound::FOUNDER_STEP, dx, dy);
                    if self.first("founder") {
                        let p = ctx.control_name(ParlorAction::Primary);
                        ctx.say(format!(
                            "The founder follows you, in heavy boots. Freezing will not stop him. At the ringing note, {p} strikes with your candlestick. Too soon and he blocks."
                        ));
                    } else {
                        ctx.say("The founder follows you in.");
                    }
                }
                Ev::FounderStep { dx, dy } => {
                    self.play_at(ctx, sound::FOUNDER_STEP, dx, dy);
                }
                Ev::WindUp { dx, dy } => {
                    self.play_at(ctx, sound::WINDUP, dx, dy);
                }
                Ev::Opening => {
                    sound::play(ctx, sound::OPENING);
                    let debut = self.run.spec.number <= crate::rules::FOUNDER_FROM + 1;
                    if self.run.spec.call_freeze || debut || self.first("opening") {
                        ctx.say("Strike!");
                    }
                }
                Ev::Parried => {
                    sound::play(ctx, sound::PARRY);
                    ctx.say("Too soon. He blocks. Step away!");
                }
                Ev::Hit { left } => {
                    sound::play(ctx, sound::STRIKE_HIT);
                    let more = if left == 1 {
                        "1 more hit".to_string()
                    } else {
                        format!("{left} more hits")
                    };
                    ctx.say(format!("Hit! {}.", sentence_start(&more)));
                }
                Ev::FounderMissed => {
                    sound::play(ctx, sound::BUMP);
                    ctx.say("His blow strikes the floor.");
                }
                Ev::Banished { points } => {
                    sound::play(ctx, sound::CRUMBLE);
                    ctx.say(format!("The founder crumbles. Plus {points}."));
                }
                Ev::FounderInWay(side) => {
                    sound::play(ctx, sound::BUMP);
                    ctx.say(format!("The founder blocks the way, {}.", side.name()));
                }
                Ev::Shift(_) => {
                    sound::play(ctx, sound::SHIFT);
                    let door = self.run.door_words(self.run.current().exit);
                    if !self.prompt_door.is_empty() {
                        self.last_prompt = self.last_prompt.replace(&self.prompt_door, &door);
                    }
                    self.clock.reset();
                    ctx.say(format!("The house shifts. Exit now in {door}."));
                    self.prompt_door = door;
                }
                Ev::Landing { courage } => {
                    sound::play(ctx, sound::LANDING);
                    ctx.say_queued(format!("A landing. Courage {courage} of {COURAGE}."));
                }
                Ev::Warning(secs) => {
                    sound::play(ctx, sound::TOLL);
                    ctx.say(format!("{} left.", sentence_start(&clock_words(secs))));
                }
                Ev::FloorCleared {
                    number,
                    time,
                    bonus,
                } => {
                    self.stop_ghost(ctx);
                    self.stop_shadow(ctx);
                    self.score.stop(ctx);
                    sound::play(ctx, sound::STAIRS);
                    let pace = self.clock.pace();
                    let saved = progress::save_floor(ctx, self.run.mode, pace, number, time);
                    let names = unlocks::on_floor(ctx, &self.run, number);
                    if !names.is_empty() {
                        sound::play(ctx, sound::UNLOCK);
                    }
                    self.earned.extend(names.iter().cloned());
                    self.announced.extend(names.iter().cloned());
                    let mut page = floor_page(number, time, bonus, self.run.score, saved);
                    if matches!(self.run.mode, Mode::Stirs(_)) && self.first("event floor") {
                        page.push_str(" Event floors do not count in Floor select.");
                    }
                    if let Some(line) = unlocks::announce(&names) {
                        page.push(' ');
                        page.push_str(&line);
                    }
                    if self.run.next_floor_waiting {
                        ctx.audio.set_bed(None);
                        ctx.audio.play_music(Some(sound::FLOOR_CLEAR), false);
                        let carry = if self.run.mode.carries_courage() {
                            format!(
                                " Your courage does not refill: {} of {COURAGE}.",
                                self.run.courage
                            )
                        } else {
                            String::new()
                        };
                        let next = number + 1;
                        let new_wing = (next - 1) % crate::rules::WING_FLOORS == 0
                            && self.run.mode.event().is_none();
                        let wing_line = if new_wing {
                            format!(" Below lies {}.", wing(next).name)
                        } else {
                            String::new()
                        };
                        let leave = leave_words(self.run.mode);
                        ctx.push(Pages::new([
                            page,
                            format!(
                                "The stairs creak beneath you.{wing_line}{carry} Floor {next} is next. {leave}"
                            ),
                        ]));
                    } else {
                        self.pending_floor = Some(page);
                    }
                }
                Ev::Ended(outcome) => {
                    self.stop_ghost(ctx);
                    self.stop_shadow(ctx);
                    self.score.stop(ctx);
                    self.clue.clear();
                    self.finish(ctx, outcome);
                }
            }
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>, outcome: Outcome) {
        let pace = self.clock.pace();
        let saved = progress::save_end(ctx, &self.run, pace);
        let won = outcome == Outcome::Escaped;
        let names = unlocks::on_end(ctx, &self.run, won);
        if !names.is_empty() {
            sound::play(ctx, sound::UNLOCK);
        }
        self.earned.extend(names);
        let run = &self.run;
        ctx.audio.set_bed(None);
        ctx.audio
            .play_music(Some(if won { sound::WIN } else { sound::LOSE }), false);
        let mut pages = Vec::new();
        // The floor's own page gives its time, so the results do not.
        let timed = self.pending_floor.is_some();
        if let Some(page) = self.pending_floor.take() {
            pages.push(page);
        }
        let room = run.current().name;
        let where_ = if run.spec.seance {
            "in the hidden wing".to_string()
        } else {
            format!("on floor {}", run.spec.number)
        };
        let first = match outcome {
            Outcome::Escaped => match run.mode {
                // One floor: its page just gave the time.
                Mode::Challenge(e) if timed && e.first == e.last => {
                    format!("{} survived.", challenge_name(e.period))
                }
                Mode::Challenge(e) => format!(
                    "{} survived in {}.",
                    challenge_name(e.period),
                    time_short(run.total_time)
                ),
                Mode::Floor(n) | Mode::Stirs(n) if timed => {
                    format!("You cleared floor {n}, {}.", run.spec.name)
                }
                Mode::Floor(n) => format!(
                    "You cleared floor {n}, {}, in {}.",
                    run.spec.name,
                    time_short(run.total_time)
                ),
                Mode::Nightmare => format!(
                    "You woke from the nightmare: all {FLOORS} floors in {}.",
                    time_short(run.total_time)
                ),
                Mode::Stirs(n) => format!(
                    "You cleared floor {n}, {}, in {}.",
                    run.spec.name,
                    time_short(run.total_time)
                ),
                Mode::LanternNight(_) => format!(
                    "You survived the Lantern Night: all {} floors in {}.",
                    crate::allhallows::CANDLES,
                    time_short(run.total_time)
                ),
                _ => format!(
                    "Every door in the house swings open, and you walk out into the cold night air. All {FLOORS} floors behind you, this climb in {}.",
                    time_short(run.total_time)
                ),
            },
            Outcome::TimeUp => {
                format!("The haunt clock strikes zero {where_}, in {room}. The house keeps you.")
            }
            Outcome::ChasedOut => format!(
                "Out of courage, you run screaming from {room} {where_}, and out of the house."
            ),
        };
        let depth = match run.mode {
            Mode::Climb(from) if run.deepest >= from => {
                format!(" This climb cleared floors {from} to {}.", run.deepest)
            }
            Mode::Nightmare | Mode::Endless if run.deepest > 0 => {
                let record = if saved.deepest {
                    ", your deepest yet"
                } else {
                    ""
                };
                format!(" Deepest floor cleared: {}{record}.", run.deepest)
            }
            _ => String::new(),
        };
        let halloween = halloween_line(ctx, run.mode, won);
        let challenge_line = self.challenge.as_ref().map(|ch| {
            let result = Challenges::submit_at(ctx, ch, pace, run.score);
            let streak = Challenges::streak(ctx, ch.game, ch.period);
            challenge::result_line(ch, run.score, result, streak, pace)
        });
        let score = match saved.score {
            Some(ScoreResult::First) => format!("Score {}: your first on this board.", run.score),
            Some(ScoreResult::NewBest { previous }) => format!(
                "Score {}: a new {}, beating {previous}.",
                run.score,
                pace.best_word()
            ),
            Some(ScoreResult::Short { best }) => {
                format!("Score {}. Your {} is {best}.", run.score, pace.best_word())
            }
            None => "No score this time.".to_string(),
        };
        let score = challenge_line.unwrap_or(score);
        let curses = match run.curses.count() {
            0 => String::new(),
            1 => " 1 curse on, a quarter more score.".to_string(),
            n => format!(" {n} curses on, {n} quarters more score."),
        };
        pages.push(format!(
            "{first}{depth}{halloween} {score}{curses} Ghosts evaded: {}. Caught: {}. Hints: {}.",
            run.ghosts_evaded, run.times_caught, run.hints
        ));
        if run.floor_times.len() > 1 {
            let times: Vec<String> = run
                .floor_times
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    format!(
                        "floor {}, {}",
                        run.mode.first_floor() + i as u32,
                        time_short(*t)
                    )
                })
                .collect();
            pages.push(format!("Floor times: {}.", times.join("; ")));
        }
        let fresh: Vec<String> = self
            .earned
            .iter()
            .filter(|n| !self.announced.contains(n))
            .cloned()
            .collect();
        if let Some(line) = unlocks::announce(&fresh) {
            pages.push(line);
        }
        let summary = format!("{}. Score {}.", first.trim_end_matches('.'), run.score);
        let results = match (self.challenge.as_ref(), run.mode) {
            (Some(ch), _) => after_challenge(ch.period, summary, pages.clone()),
            (None, mode) => after_haunt(mode, won, summary, pages.clone()),
        };
        // The results replace the haunt once its pages are read.
        let mut results = Some(results);
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }
}

/// Halloween's part of a result: a candle lit after a run-up floor, the
/// ember after the Lantern Night. Empty for every other run.
fn halloween_line(ctx: &mut Ctx<Parlor>, mode: Mode, won: bool) -> String {
    use crate::allhallows::{ember_line, lit_line, year_of, Ledger};
    let year = year_of(parlor_core::challenges::today(ctx));
    let mut ledger = Ledger::load(ctx);
    let line = match mode {
        Mode::Stirs(_) if won => lit_line(ledger.light(year)),
        Mode::LanternNight(_) => {
            let first = ledger.night_done(year, won);
            if won {
                ember_line(first)
            } else {
                "Lantern Jack laughs in the dark. The Lantern Night stays open until 1 November."
                    .to_string()
            }
        }
        _ => return String::new(),
    };
    ledger.store(ctx);
    format!(" {line}")
}

/// The results menu after a haunt: the next floor when one was cleared on
/// its own, the same haunt again, the results again, back to the title.
pub fn after_haunt(mode: Mode, won: bool, summary: String, pages: Vec<String>) -> Results {
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if let (true, Mode::Floor(n)) = (won, mode) {
        if n < FLOORS {
            let next = n + 1;
            let spec = floor_spec(next, false);
            results = results.next(
                format!("Next floor: floor {next}, {}", spec.name),
                format!(
                    "{} rooms, {} on the haunt clock.",
                    spec.rooms,
                    clock_words(spec.haunt)
                ),
                move |ctx| {
                    let h = Haunt::start(ctx, Mode::Floor(next));
                    ctx.replace(h)
                },
            );
        }
    }
    let (label, help) = match (won, mode) {
        (true, Mode::Floor(_)) => (
            labels::PLAY_AGAIN,
            "The same floor again, for a better time.",
        ),
        (false, Mode::Floor(_)) => (
            labels::TRY_AGAIN,
            "The same floor again, with full courage and a full clock.",
        ),
        (true, Mode::Stirs(_)) => (labels::PLAY_AGAIN, "The floor that lights the next candle."),
        (true, Mode::Climb(_)) => (
            labels::PLAY_AGAIN,
            "Climb again from your highest open floor.",
        ),
        (false, Mode::Climb(_)) => (
            labels::TRY_AGAIN,
            "Climb again from your highest open floor.",
        ),
        (true, _) => (
            labels::PLAY_AGAIN,
            "The same run again, from its first floor.",
        ),
        (false, _) => (
            labels::TRY_AGAIN,
            "The same run again, from its first floor.",
        ),
    };
    results.again(label, help, move |ctx| {
        let h = match mode {
            Mode::Climb(_) => Haunt::climb(ctx),
            Mode::Stirs(n) => {
                let floor = next_candle_floor(ctx).unwrap_or(n);
                Haunt::start(ctx, Mode::Stirs(floor))
            }
            m => Haunt::start(ctx, m),
        };
        ctx.replace(h)
    })
}

/// The floor that lights the next unlit candle, while the house stirs.
fn next_candle_floor(ctx: &mut Ctx<Parlor>) -> Option<u32> {
    use crate::allhallows::{stirs_row, year_of, Ledger};
    let today = parlor_core::challenges::today(ctx);
    let lit = Ledger::load(ctx).year(year_of(today)).candles;
    stirs_row(today, lit).map(|(_, _, floor)| floor)
}

/// The results menu after a challenge run: its briefing again, the
/// results again, back to the challenges.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same house again, from its briefing. Only your best result is kept.",
        move |ctx| {
            let pages = challenge::briefing(ctx, period);
            ctx.replace(pages)
        },
    )
}

/// What leaving between floors keeps, for the stairs page.
pub fn leave_words(mode: Mode) -> &'static str {
    match mode {
        Mode::Climb(_) | Mode::Floor(_) | Mode::Stirs(_) => {
            "Escape pauses, where you can leave with this floor saved."
        }
        Mode::Nightmare | Mode::Endless => {
            "Escape pauses, where you can leave with your deepest floor saved."
        }
        Mode::LanternNight(_) => {
            "Escape pauses. Leaving ends the night, and the next one starts at floor 1."
        }
        Mode::Challenge(_) => "Escape pauses. Leaving does not count the run.",
    }
}

fn floor_page(number: u32, time: f64, bonus: u64, score: u64, saved: FloorSaved) -> String {
    let mut line = format!(
        "Floor {number} cleared in {}. Time bonus and courage: plus {bonus}. Score {score}.",
        time_short(time)
    );
    // A relaxed time goes on its own list, and says so.
    let relaxed = if saved.relaxed { " relaxed" } else { "" };
    match (saved.record, saved.previous) {
        (true, Some(prev)) => line.push_str(&format!(
            " A new{relaxed} best time for this floor, beating {}.",
            time_short(prev)
        )),
        (true, None) => line.push_str(&format!(" Your first{relaxed} time on this floor.")),
        (false, Some(prev)) => line.push_str(&format!(
            " Your{relaxed} best here is {}.",
            time_short(prev)
        )),
        _ => {}
    }
    if let Some(next) = saved.opened {
        line.push_str(&format!(" Floor {next} is open."));
    }
    if saved.nightmare {
        line.push_str(" Nightmare is open on the title.");
    }
    if saved.endless {
        line.push_str(" Endless night is open on the title.");
    }
    line
}

impl State<Parlor> for Haunt {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "haunted_hallways_haunt"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.started {
            self.clock = RunClock::start(ctx, crate::ID);
        }
        self.started = true;
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        let names = unlocks::on_start(ctx);
        self.earned.extend(names);
        self.load_taught(ctx);
        let evs = self.run.floor_begins();
        self.voice(ctx, evs);
        // The tutorial teaches the keys step by step instead.
        if !self.run.practice && self.first("intro") {
            let s = sentence_start(&ctx.control_name(ParlorAction::Status));
            let h = sentence_start(&ctx.control_name(ParlorAction::Hint));
            let keys = [
                ParlorAction::Up,
                ParlorAction::Down,
                ParlorAction::Left,
                ParlorAction::Right,
            ]
            .map(|a| ctx.control_name(a));
            let arrows = if keys.iter().all(|k| k.contains("arrow")) {
                "Arrows".to_string()
            } else {
                format!(
                    "{}, {}, {} and {}",
                    sentence_start(&keys[0]),
                    keys[1],
                    keys[2],
                    keys[3]
                )
            };
            ctx.say_queued(format!(
                "{arrows} walk; you face north. {s}: clock and exit. {h}: hint."
            ));
        }
        self.keep_taught(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_ghost(ctx);
        self.stop_shadow(ctx);
        self.score.stop(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.clock.reset();
        if self.run.finished() {
            return;
        }
        if self.run.next_floor_waiting {
            let evs = self.run.next_floor();
            self.voice(ctx, evs);
            return;
        }
        let room = self.run.current();
        ctx.say(format!(
            "Back in {}. {} of haunt left.",
            room.name,
            sentence_start(&clock_words(self.run.haunt_left))
        ));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || self.run.finished() {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let evs = match ctx.action_for(event) {
            Some(ParlorAction::Up) => self.run.walk(Side::North),
            Some(ParlorAction::Down) => self.run.walk(Side::South),
            Some(ParlorAction::Left) => self.run.walk(Side::West),
            Some(ParlorAction::Right) => self.run.walk(Side::East),
            Some(ParlorAction::Primary) => self.run.interact(),
            Some(ParlorAction::Secondary) => self.run.cycle(),
            Some(ParlorAction::Hint) => self.run.hint(),
            Some(ParlorAction::Status) => {
                let layers = self.status();
                let more = layers.get(1).cloned();
                let line = self.status_key.speak(ctx, layers);
                self.heard_more |= Some(line) == more;
                return;
            }
            Some(ParlorAction::Repeat) => {
                let text = self.last_prompt.clone();
                ctx.say(text);
                self.start_clue(ctx);
                return;
            }
            None => {
                if event.pressed() == Some(Key::I) {
                    let text = if self.run.inventory.is_empty() {
                        "You carry nothing.".to_string()
                    } else {
                        let names: Vec<&str> =
                            self.run.inventory.iter().map(|i| i.name()).collect();
                        format!("You carry: {}.", names.join(", "))
                    };
                    ctx.say(text);
                }
                return;
            }
        };
        self.voice(ctx, evs);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.keep_taught(ctx);
        if !self.started || self.run.finished() || self.run.next_floor_waiting {
            return;
        }
        // The music, the clue sounds and the storm keep real time, so a
        // clue stays in step with the phonograph that plays it; the house
        // itself runs on game time.
        let game_dt = self.clock.game_dt(ctx, dt);
        let mood = self.mood();
        self.score.follow(ctx, mood, dt);
        let mut due = Vec::new();
        self.clue.retain_mut(|(t, item)| {
            *t -= dt;
            if *t <= 0.0 {
                due.push(*item);
                false
            } else {
                true
            }
        });
        for item in due {
            sound::play(ctx, sound::item(item));
            self.hush = ATMOS_HUSH;
        }
        self.hush -= dt;
        self.atmos_in -= dt;
        // Never over a clue still to come or just heard, or a ghost.
        let busy = self.hush > 0.0 || !self.clue.is_empty() || self.run.ghost.is_some();
        if self.atmos_in <= 0.0 && busy {
            self.atmos_in = 1.0;
        } else if self.atmos_in <= 0.0 {
            let depth = (self.run.spec.number.min(10)) as f64;
            self.atmos_in = 30.0 + self.atmos.unit() * 40.0 - depth * 3.0;
            let key = if self.run.spec.number >= 2 && self.atmos.chance(0.4) {
                sound::SCREAM
            } else {
                sound::THUNDER
            };
            let s = ctx.audio.play(key, Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.5);
            if self.first("atmos") {
                ctx.say_queued(
                    "Only the storm and the house: thunder and distant screams are atmosphere and mean nothing.",
                );
            }
        }
        let evs = self.run.update(game_dt);
        if let Some(g) = self.run.ghost {
            if self.ghost_loop != Sound::NONE {
                let (x, y) = g.offset();
                let x = if self.run.current().mirror { -x } else { x };
                ctx.audio
                    .set_position(self.ghost_loop, Some(Vec3::new(x, y, 0.0)));
            }
        }
        if self.shadow_loop != Sound::NONE {
            if let Some(at) = self.shadow_point() {
                ctx.audio.set_position(self.shadow_loop, Some(at));
            }
        }
        self.voice(ctx, evs);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let run = &self.run;
        let room = run.current();
        let mut out = vec![format!(
            "Floor {}, {}, room {} of {}: {}",
            run.spec.number,
            run.spec.name,
            run.room + 1,
            run.rooms.len(),
            room.name
        )];
        out.push(format!(
            "Haunt {}  Courage {}  Score {}",
            clock_words(run.haunt_left),
            run.courage,
            run.score
        ));
        for y in (0..room.h).rev() {
            let row: String = (0..room.w)
                .map(|x| {
                    if (x, y) == (run.x, run.y) {
                        '@'
                    } else if room.shadow.is_some_and(|s| (s.x, s.y) == (x, y)) {
                        'S'
                    } else if run.founder.is_some_and(|f| (f.x, f.y) == (x, y)) {
                        'F'
                    } else if room.pendulum.is_some_and(|p| (p.x, p.row) == (x, y)) {
                        'P'
                    } else if let Some(i) = room.item_at(x, y) {
                        if room.items[i].mimic {
                            'm'
                        } else {
                            '*'
                        }
                    } else if room.rotten.contains(&(x, y)) {
                        'x'
                    } else if room.holes.contains(&(x, y)) {
                        'o'
                    } else if room.inside(room.exit) == (x, y) {
                        'D'
                    } else if room.charm_at(x, y).is_some() {
                        'c'
                    } else if room.water.contains(&(x, y)) {
                        '~'
                    } else {
                        '.'
                    }
                })
                .collect();
            out.push(row);
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let arrows = format!(
            "{}, {}, {} and {}",
            sentence_start(&ctx.control_name(ParlorAction::Up)),
            ctx.control_name(ParlorAction::Down),
            ctx.control_name(ParlorAction::Left),
            ctx.control_name(ParlorAction::Right)
        );
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let r = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        let ghost = match self.run.ghost {
            Some(g) if g.phase == GhostPhase::Near && g.kind.hunts_stillness() => {
                "The hound is passing you right now: keep walking! ".to_string()
            }
            Some(g) if g.phase == GhostPhase::Near => {
                "A ghost is passing you right now: do not move! ".to_string()
            }
            Some(g) if g.kind.hunts_stillness() => "The hound is coming: when it howls, keep walking until it breathes out and drifts away. ".to_string(),
            Some(_) => "A ghost is coming: when its stab sounds, stop moving until it breathes out and drifts away. ".to_string(),
            None => String::new(),
        };
        let room = self.run.current();
        let now = if room.solved {
            "This room is solved: follow the cold draught to the open door and walk through it."
                .to_string()
        } else {
            self.puzzle_prompt(ctx, true)
        };
        let mut dangers = String::new();
        if room.shadow.is_some() {
            dangers.push_str("The shadow follows your footsteps: keep away from it. ");
        }
        if !room.rotten.is_empty() {
            dangers.push_str("A groan means a rotten board next to you: do not step there. ");
        }
        if room.mimics() > 0 {
            dangers.push_str("Mimics hop about; the real thing keeps still. ");
        }
        if room.mirror {
            dangers.push_str("This room is mirrored: east sounds from your left. ");
        }
        if room.dark {
            dangers.push_str("This room is dark: things call out only when you are near. ");
        }
        if room.pendulum.is_some() {
            dangers.push_str(
                "The swish is the pendulum's blade: cross its row just after it passes. ",
            );
        }
        if !room.water.is_empty() {
            dangers.push_str("Lapping means deep water: wading is slow and brings ghosts sooner. ");
        }
        if room.stones {
            dangers.push_str("When dust trickles over you, step off before the stone falls. ");
        }
        if room.shifts {
            dangers.push_str("A rumble moves the exit door: status says where. ");
        }
        if self.run.founder.is_some() {
            dangers.push_str(&format!(
                "The founder hunts you. At his ringing note, {} strikes him; too soon and he blocks, so step away. ",
                ctx.control_name(ParlorAction::Primary)
            ));
        }
        let challenge = match self.run.mode.event() {
            Some(e) => {
                let twist = match e.twist {
                    Some(t) => format!(" Twist: {}, {}.", t.name(), t.describe()),
                    None => " Séance rooms want their sounds' things in order.".to_string(),
                };
                let carry = if e.carries_courage() {
                    " Courage does not refill between floors."
                } else {
                    ""
                };
                format!(
                    "{}, a {} challenge.{twist}{carry} ",
                    challenge_name(e.period),
                    e.period.label().to_lowercase()
                )
            }
            None => String::new(),
        };
        let hint = if self.run.spec.hints {
            format!(
                "{} gives a hint for {}.",
                sentence_start(&h),
                clock_words(self.run.spec.hint_cost)
            )
        } else {
            "No hints on this floor.".to_string()
        };
        format!(
            "{challenge}{ghost}{dangers}{now} {arrows} walk one step; you face north. {} picks up or offers, {t} chooses what you hold, I lists what you carry. {} says any danger, the clock, your courage and the way to the exit; press it again for the room, the puzzle and where you stand. {} repeats the room's clue. {hint} Escape pauses.",
            sentence_start(&p),
            sentence_start(&s),
            sentence_start(&r),
        )
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_haunt;
