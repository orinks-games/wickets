//! The pantry at night: the play screen.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pace, Pages, Parlor, ParlorAction, Period, RunClock, ScoreResult};

use crate::challenge::{self, Plan, CHALLENGES_NAME};
use crate::events;
use crate::progress::{self, Progress, Saved};
use crate::rules::{
    crumbs_words, freezes_words, offset_words, points_words, run_seed, seconds_words, steps_words,
    ways_words, whiskers_words, Dir, Ev, Kind, Outcome, Run, Treat, CLEAR_BONUS, CRUMB_POINTS,
    NEXT_PANTRY_MS, SNIFF_CRUMBS, WANDER_CHASE,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "pantry_mouse_play";

/// Seconds between the ticks of a sniff.
pub const SNIFF_GAP: f64 = 0.3;
/// Seconds from the end of a run to its results, so the last sound is
/// heard and a key still held from play does not skip the first page.
pub const END_PAUSE: f64 = 1.5;
/// Seconds between steps while an arrow is held, once the hold is seen.
pub const HOLD_PACE: f64 = 0.22;
/// The shortest wait before a held arrow starts repeating. The real wait is
/// longer when a screen reader's re-sent key would read as held for longer.
pub const HOLD_DELAY: f64 = 0.4;
/// How many times a cat coming close is spoken in a run, before the meow
/// alone tells it.
pub const CLOSE_WARNINGS: u32 = 3;

/// What kind of run the screen is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The endless pantry run.
    Pantry,
    /// The practice pantry: one slow cat, no whiskers lost.
    Practice,
    /// A challenge.
    Challenge(Period),
    /// Thanksgiving's pantry run, with turkey scraps.
    Leftovers,
    /// Bin night's pantry run, with a cheese rind.
    BinNight,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Pantry => "The pantry",
            Mode::Practice => "Practice pantry",
            Mode::Challenge(period) => challenge::name(period),
            Mode::Leftovers => events::LEFTOVERS,
            Mode::BinNight => events::BIN_NIGHT,
        }
    }

    /// The event board this mode writes, if it is an event pantry.
    pub fn event_board(self) -> Option<&'static str> {
        match self {
            Mode::Leftovers => Some(events::LEFTOVERS_BOARD),
            Mode::BinNight => Some(events::BIN_NIGHT_BOARD),
            _ => None,
        }
    }
}

/// A cat's name at the start of a sentence: "Ginger cat".
fn cat_title(name: &str) -> String {
    sentence_start(name)
}

/// A cat's rule, in one sentence.
pub fn cat_rule(kind: Kind) -> String {
    match kind {
        Kind::Chaser => "It heads straight for you.".to_string(),
        Kind::Wanderer => {
            format!("It wanders until it is {WANDER_CHASE} steps away, then chases.")
        }
    }
}

/// Said for any key while the next pantry opens.
pub const NEXT_PANTRY: &str = "Next pantry opening.";

/// "Frozen. 2 freezes left." The freezes, not seconds, are counted.
pub fn frozen_words(left: u32) -> String {
    match left {
        0 => "Frozen. No freezes left.".to_string(),
        1 => "Frozen. 1 freeze left.".to_string(),
        n => format!("Frozen. {n} freezes left."),
    }
}

pub struct Play {
    pub run: Run,
    pub mode: Mode,
    challenge: Option<Challenge>,
    /// Cues already explained in words this run.
    taught: HashSet<&'static str>,
    /// The tutorial says the freeze line itself, so the screen keeps quiet
    /// rather than be cut off by it.
    pub lesson_says_freeze: bool,
    /// Cats already named this run.
    met: HashSet<usize>,
    /// Close warnings spoken this run.
    warnings: u32,
    /// The cats' loops, by cat.
    loops: Vec<Option<Sound>>,
    /// Sniff sounds still to play: seconds to wait, offset from the
    /// mouse, and the sound.
    ticks: Vec<(f64, (i32, i32), &'static str)>,
    /// The arrow held down, and seconds until it steps again.
    held: Option<(Dir, f64)>,
    /// The step now taken comes from a held arrow's repeat.
    repeating: bool,
    /// The last step met a wall, whose open ways were said.
    bumped: bool,
    /// Seconds until the results, once the run is over.
    ending: Option<f64>,
    /// The last line worth repeating.
    last: String,
    /// Practice crumbs eaten and not yet saved.
    practice_unsaved: u32,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// The run's game clock: its pace, set as the run starts, and the hold
    /// that keeps the cats still while a line is spoken.
    pub clock: RunClock,
}

impl Play {
    pub fn new(mode: Mode) -> Play {
        Play::with_seed(mode, run_seed())
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Play {
        let run = match mode {
            Mode::Practice => Run::quiet(seed),
            Mode::Leftovers => events::leftovers_run(seed),
            Mode::BinNight => events::bin_night_run(seed),
            _ => Run::pantry(seed),
        };
        Play {
            status_key: Default::default(),
            run,
            mode,
            challenge: None,
            taught: HashSet::new(),
            lesson_says_freeze: false,
            met: HashSet::new(),
            warnings: 0,
            loops: Vec::new(),
            ticks: Vec::new(),
            held: None,
            repeating: false,
            bumped: false,
            ending: None,
            last: String::new(),
            practice_unsaved: 0,
            clock: RunClock::new(Pace::Normal),
        }
    }

    /// A run of the challenge's mazes.
    pub fn challenge(ch: Challenge) -> Play {
        let plan = Plan::for_challenge(&ch);
        let mut p = Play::with_seed(Mode::Challenge(ch.period), 0);
        p.run = plan.run();
        p.challenge = Some(ch);
        p
    }

    /// True the first time a cue is met this run.
    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// Says a line and keeps it for Repeat.
    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        ctx.say(text);
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let what = match self.mode {
            Mode::Pantry | Mode::Leftovers | Mode::BinNight => format!(
                "Eat every crumb, then the next pantry opens. You have {}.",
                whiskers_words(self.run.whiskers)
            ),
            Mode::Practice => {
                "One slow cat, and a catch costs nothing. Learn the sounds.".to_string()
            }
            Mode::Challenge(_) => {
                let total = self.run.pantries().unwrap_or(1);
                let mazes = if total == 1 {
                    "1 maze".to_string()
                } else {
                    format!("{total} mazes")
                };
                format!("{mazes}. You have {}.", whiskers_words(self.run.whiskers))
            }
        };
        format!("{}. {what} {}", self.mode.name(), self.start_prompt(ctx))
    }

    /// "The cats wake at your first step.", or the one cat's name when
    /// there is only one, as in pantry 1.
    fn wake_words(&self) -> String {
        match self.run.cats.as_slice() {
            [cat] => format!("The {} wakes at your first step.", cat.name),
            _ => "The cats wake at your first step.".to_string(),
        }
    }

    /// What to do before the first step.
    fn start_prompt(&self, _ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} crumbs to find. {} {}",
            self.run.left,
            self.wake_words(),
            self.open_line()
        )
    }

    /// What the event pantry hides, in a sentence, or nothing.
    fn treat_line(&self) -> Option<String> {
        let (treat, n) = self.run.treat?;
        let each = match treat {
            Treat::Turkey => "Thanksgiving leftovers",
            Treat::CheeseRind => "Bin night",
        };
        Some(format!(
            "{each}: {} in every pantry, {} points each. Listen for {}. A sniff plays the nearest one from where it hides.",
            treat.count_words(n),
            treat.points(),
            sound::treat_words(treat)
        ))
    }

    /// "2 turkey scraps left", for the status, when the run hides treats.
    fn treats_left_words(&self) -> Option<String> {
        let (treat, _) = self.run.treat?;
        Some(format!(
            "{} left",
            treat.count_words(self.run.treats_left())
        ))
    }

    /// "Open: left and right."
    fn open_line(&self) -> String {
        let open = self.run.maze.open_dirs(self.run.mouse);
        format!("Open: {}.", ways_words(&open))
    }

    /// Every control, in a few sentences.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "The arrows move one cell, and holding one keeps moving. {} sniffs for the nearest crumbs. {} freezes, so the nearest cat loses you. {} speaks the cats, crumbs and whiskers, twice for the rest, {} repeats the last line, {} points to a safe crumb, and Escape pauses.",
            sentence_start(&k(ParlorAction::Primary)),
            sentence_start(&k(ParlorAction::Secondary)),
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Repeat),
            k(ParlorAction::Hint)
        )
    }

    /// One cat, for the status line: "ginger cat 3 cells left and 1 up".
    fn cat_line(&self, i: usize) -> String {
        let (dx, dy) = self.run.cat_offset(i);
        format!("{} {}", self.run.cats[i].name, offset_words(dx, dy))
    }

    /// The status key, in layers. First: each cat, then crumbs and
    /// whiskers. A second press: freezes, score and the maze.
    pub fn status(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let r = &self.run;
        let mut first: Vec<String> = (0..r.cats.len()).map(|i| self.cat_line(i)).collect();
        if r.frozen_ms() > 0 {
            first.push(format!("frozen for {}", seconds_words(r.frozen_ms())));
        }
        first.push(format!("{} left", crumbs_words(r.left)));
        first.extend(self.treats_left_words());
        if r.rules.whiskers {
            first.push(whiskers_words(r.whiskers));
        }
        let mut more = Vec::new();
        if r.frozen_ms() == 0 {
            more.push(format!("{} left", freezes_words(r.freezes)));
        }
        if !r.rules.whiskers {
            more.push("no whiskers to lose".to_string());
        }
        more.push(format!("score {}", r.score));
        more.push(match r.pantries() {
            Some(total) if total > 1 => format!("maze {} of {total}", r.pantry),
            Some(_) => "one maze".to_string(),
            None => format!("pantry {}", r.pantry),
        });
        let mut more = format!("{}.", sentence_start(&more.join(", ")));
        if r.between() {
            more.push(' ');
            more.push_str(NEXT_PANTRY);
        } else if !r.started() {
            more.push(' ');
            more.push_str(&self.wake_words());
        }
        vec![format!("{}.", sentence_start(&first.join(", "))), more]
    }

    /// Starts each cat's loop, silent until it is within hearing.
    fn start_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        self.loops = (0..self.run.cats.len())
            .map(|i| {
                let (dx, dy) = self.run.cat_offset(i);
                let s = ctx.audio.start_loop(
                    sound::cat_loop(i),
                    Some(sound::place(dx, dy)),
                    Bus::EFFECTS,
                );
                ctx.audio
                    .set_volume(s, if self.run.cat_heard(i) { 1.0 } else { 0.0 });
                Some(s)
            })
            .collect();
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.loops.drain(..).flatten() {
            ctx.audio.stop(s, 150);
        }
    }

    /// Moves every cat's loop to where the cat now is.
    fn place_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for (i, s) in self.loops.iter().enumerate() {
            if let (Some(s), true) = (s, i < self.run.cats.len()) {
                let (dx, dy) = self.run.cat_offset(i);
                ctx.audio.set_position(*s, Some(sound::place(dx, dy)));
                ctx.audio
                    .set_volume(*s, if self.run.cat_heard(i) { 1.0 } else { 0.0 });
            }
        }
    }

    fn step(&mut self, ctx: &mut Ctx<Parlor>, dir: Dir) {
        let evs = self.run.step(dir);
        self.voice(ctx, evs);
        self.place_loops(ctx);
    }

    fn sniff(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.run.finished() {
            return;
        }
        let found = self.run.nearest_crumbs(SNIFF_CRUMBS);
        if found.is_empty() {
            ctx.play_ui(UiSound::Error);
            self.say(ctx, "No crumbs left in this pantry.");
            return;
        }
        let m = self.run.mouse;
        self.ticks = found
            .iter()
            .enumerate()
            .map(|(i, (c, _))| {
                (
                    i as f64 * SNIFF_GAP,
                    (c.x - m.x, c.y - m.y),
                    sound::SNIFF_TICK,
                )
            })
            .collect();
        let mut treat_words = String::new();
        if let (Some((treat, _)), Some((c, _))) = (self.run.treat, self.run.nearest_treat()) {
            let at = (c.x - m.x, c.y - m.y);
            self.ticks
                .push((found.len() as f64 * SNIFF_GAP, at, sound::treat(treat)));
            treat_words = format!(" Nearest {}: {}.", treat.name(), offset_words(at.0, at.1));
        }
        self.play_ticks(ctx, 0.0);
        let left = format!(
            "{} left.{treat_words}",
            sentence_start(&crumbs_words(self.run.left))
        );
        if self.first("sniff") {
            self.say(
                ctx,
                format!("Each tick is a crumb, from its direction, nearest first. {left}"),
            );
        } else {
            self.say(ctx, left);
        }
    }

    fn play_ticks(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut due = Vec::new();
        self.ticks.retain_mut(|(wait, at, key)| {
            *wait -= dt;
            if *wait <= 1e-9 {
                due.push((*at, *key));
                false
            } else {
                true
            }
        });
        for ((dx, dy), key) in due {
            sound::play_at(ctx, key, dx, dy);
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.run.rules.hints {
            ctx.play_ui(UiSound::Error);
            self.say(ctx, "No hints here.");
            return;
        }
        if self.run.between() || self.run.finished() {
            self.say(ctx, NEXT_PANTRY);
            return;
        }
        let mut line = match self.run.hint() {
            Some((way, n)) => sentence_start(&format!("{}, {}.", way.word(), steps_words(n))),
            None => "No safe crumb.".to_string(),
        };
        if let (Some((treat, _)), Some((c, n))) = (self.run.treat, self.run.nearest_treat()) {
            if let Some(way) = self.run.maze.toward(self.run.mouse, c) {
                line.push_str(&format!(
                    " {} {}, {}.",
                    sentence_start(treat.name()),
                    way.word(),
                    steps_words(n)
                ));
            }
        }
        self.say(ctx, line);
    }

    fn freeze(&mut self, ctx: &mut Ctx<Parlor>) {
        let ev = self.run.freeze();
        self.voice(ctx, vec![ev]);
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        for ev in evs {
            match ev {
                Ev::Step { crumb } => {
                    self.bumped = false;
                    sound::play(ctx, sound::STEP);
                    if crumb {
                        sound::play(ctx, sound::NIBBLE);
                        if self.mode == Mode::Practice {
                            self.practice_unsaved += 1;
                        }
                        if self.first("nibble") {
                            self.say(ctx, format!("Crumb, {CRUMB_POINTS} points."));
                        }
                    } else if self.first("step") {
                        self.say(ctx, "No crumb here.");
                    }
                }
                Ev::Treat {
                    treat,
                    points: _,
                    left,
                } => {
                    sound::play(ctx, sound::treat(treat));
                    let line = match left {
                        0 => format!("{}, the last.", sentence_start(treat.name())),
                        n => format!("{}, {n} left.", sentence_start(treat.name())),
                    };
                    self.say(ctx, line);
                }
                Ev::Bump { open } => {
                    sound::play(ctx, sound::WALL);
                    // An arrow held into the same wall thuds; its open ways
                    // were said at the first bump.
                    if !(self.repeating && self.bumped) {
                        self.say(ctx, format!("Wall. Open: {}.", ways_words(&open)));
                    }
                    self.bumped = true;
                }
                Ev::StillFrozen { ms } => {
                    // A held arrow would repeat this over the cat's news.
                    if !self.repeating {
                        self.say(ctx, format!("Frozen, {} left.", seconds_words(ms)));
                    }
                }
                Ev::Waiting => {
                    if self.run.between() {
                        self.say(ctx, NEXT_PANTRY);
                    }
                }
                Ev::CatMoved { .. } => {}
                Ev::CatHeard { cat, heard } => {
                    if heard && self.met.insert(cat) {
                        self.clock.reset();
                        let c = &self.run.cats[cat];
                        let way = self
                            .run
                            .cat_way(cat)
                            .map_or(String::new(), |d| format!(", {}", d.word()));
                        let line = format!(
                            "{}{way}: {}.",
                            cat_title(c.name),
                            sound::cat_sound_words(cat)
                        );
                        self.say(ctx, line);
                    }
                }
                Ev::CatClose { cat } => {
                    let (dx, dy) = self.run.cat_offset(cat);
                    sound::play_at(ctx, sound::CAT_CLOSE, dx, dy);
                    if self.warnings < CLOSE_WARNINGS {
                        self.warnings += 1;
                        self.clock.reset();
                        let way = self
                            .run
                            .cat_way(cat)
                            .map_or(String::new(), |d| format!(", {}", d.word()));
                        let line = format!("{} close{way}.", cat_title(self.run.cats[cat].name));
                        self.say(ctx, line);
                    }
                }
                Ev::Caught {
                    cat,
                    whiskers,
                    lost,
                } => {
                    sound::play(ctx, sound::CAUGHT);
                    self.clock.reset();
                    self.ticks.clear();
                    self.held = None;
                    let by = format!("Caught by the {}.", self.run.cats[cat].name);
                    let line = if !lost {
                        format!("{by} No whisker lost.")
                    } else if whiskers == 0 {
                        // The results say "Out of whiskers" next.
                        by
                    } else {
                        format!("{by} {} left.", sentence_start(&whiskers_words(whiskers)))
                    };
                    self.say(ctx, line);
                }
                Ev::Prowling => {
                    self.clock.reset();
                    self.say(ctx, "Cats moving.");
                }
                Ev::Frozen { cat, left } => {
                    sound::play(ctx, sound::FREEZE);
                    self.held = None;
                    let lost = cat.map_or(String::new(), |i| {
                        format!(" {} lost you.", cat_title(self.run.cats[i].name))
                    });
                    if !self.lesson_says_freeze {
                        let line = format!("{}{lost}", frozen_words(left));
                        self.say(ctx, line);
                    }
                }
                Ev::AlreadyFrozen { ms } => {
                    self.say(ctx, format!("Already frozen, {} left.", seconds_words(ms)));
                }
                Ev::NoFreezes => {
                    ctx.play_ui(UiSound::Error);
                    self.say(ctx, "No freezes left in this pantry.");
                }
                Ev::Thawed => {
                    sound::play(ctx, sound::THAW);
                    if self.first("thaw") {
                        self.say(ctx, "You can move again.");
                    }
                }
                Ev::Clear { pantry, bonus } => {
                    sound::play(ctx, sound::CLEAR);
                    self.held = None;
                    let name = match self.run.pantries() {
                        Some(1) => "Maze".to_string(),
                        Some(_) => format!("Maze {pantry}"),
                        None => format!("Pantry {pantry}"),
                    };
                    self.say(ctx, format!("{name} clear. Bonus {bonus}."));
                    if !self.run.finished() {
                        ctx.say_queued(format!(
                            "The next one opens in {}.",
                            seconds_words(NEXT_PANTRY_MS)
                        ));
                    }
                }
                Ev::NextPantry { pantry } => {
                    let line = match self.run.pantries() {
                        Some(total) => {
                            format!("Maze {pantry} of {total}. {}", self.start_prompt(ctx))
                        }
                        None => {
                            let rules = &self.run.rules;
                            let had = rules.cats_in(pantry - 1);
                            let joins = match self.run.cats.get(had) {
                                Some(c) if rules.cats_in(pantry) > had => {
                                    format!("The {} joins.", c.name)
                                }
                                _ => pace_words(rules, pantry).to_string(),
                            };
                            format!("Pantry {pantry}. {joins} {}", self.start_prompt(ctx))
                        }
                    };
                    self.clock.reset();
                    self.say(ctx, line);
                    self.start_loops(ctx);
                }
                Ev::Over(outcome) => {
                    self.held = None;
                    if outcome == Outcome::Cleared && self.run.pantries().unwrap_or(1) > 1 {
                        ctx.say_queued("Every maze clear.");
                    }
                    self.ending = Some(END_PAUSE);
                }
            }
        }
    }

    fn repeat(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.last.is_empty() {
            let line = self.start_prompt(ctx);
            self.say(ctx, line);
        } else {
            let line = self.last.clone();
            ctx.say(line);
        }
    }

    fn save_practice(&mut self, ctx: &mut Ctx<Parlor>) {
        progress::save_practice(ctx, self.practice_unsaved);
        self.practice_unsaved = 0;
    }

    /// The run is over: record it and read the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
        let pace = self.clock.pace();
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.run, pace);
            let summary = challenge::summary(&ch, &self.run);
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let saved = match self.mode.event_board() {
            Some(board) => progress::save_event_run(ctx, &self.run, board, pace),
            None => progress::save_run(ctx, &self.run, pace),
        };
        let pages = result_pages(&self.run, &saved);
        let summary = result_summary(&self.run);
        let mut results = Some(after_run(self.mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    /// The held arrow steps on at a steady pace.
    fn hold(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Some((dir, wait)) = self.held else {
            return;
        };
        if !ctx.action_held(dir.action()) {
            self.held = None;
            return;
        }
        let wait = wait - dt;
        if wait <= 0.0 {
            self.held = Some((dir, wait + HOLD_PACE));
            self.repeating = true;
            self.step(ctx, dir);
            self.repeating = false;
        } else {
            self.held = Some((dir, wait));
        }
    }
}

/// How long a fresh press must last before it counts as a hold.
fn hold_delay(ctx: &Ctx<Parlor>) -> f64 {
    let pulse = (ctx.held.fresh_pulse_ms() + 50) as f64 / 1000.0;
    pulse.max(HOLD_DELAY)
}

/// What ended a pantry run, in one sentence.
fn ended_words(run: &Run) -> String {
    match run.outcome {
        Some(Outcome::Cleared) => "Every pantry clear.".to_string(),
        _ => format!("Out of whiskers in pantry {}.", run.pantry),
    }
}

/// The one-line headline of a pantry run, the results menu's title.
pub fn result_summary(run: &Run) -> String {
    format!(
        "{} {}",
        ended_words(run),
        sentence_start(&points_words(run.score))
    )
}

/// The pages after a pantry run.
pub fn result_pages(run: &Run, saved: &Saved) -> Vec<String> {
    let relaxed = if saved.pace.separate() {
        "relaxed "
    } else {
        ""
    };
    let best_word = saved.pace.best_word();
    let score = match saved.score {
        ScoreResult::First => format!("{}, your first {relaxed}score.", points_words(run.score)),
        ScoreResult::NewBest { previous } => {
            format!(
                "{}, a new {best_word}, beating {previous}.",
                points_words(run.score)
            )
        }
        ScoreResult::Short { best } => {
            format!("{}. Your {best_word} is {best}.", points_words(run.score))
        }
    };
    let first = format!("{} {}", ended_words(run), sentence_start(&score));
    let cleared = match run.cleared {
        1 => "1 pantry cleared".to_string(),
        n => format!("{n} pantries cleared"),
    };
    let deepest = match saved.deeper {
        _ if saved.event => format!("You reached pantry {}.", run.pantry),
        Some(0) => format!("Deepest pantry: {}, your first.", run.pantry),
        Some(previous) => format!(
            "Deepest pantry: {}, a new record, beating {previous}.",
            run.pantry
        ),
        None => format!("Deepest pantry: {}.", saved.deepest),
    };
    let treats = match run.treat {
        Some((treat, _)) => format!(", {}", treat.count_words(run.treats_eaten)),
        None => String::new(),
    };
    vec![
        first,
        format!(
            "{} eaten{treats}, {cleared}. {deepest}",
            sentence_start(&crumbs_words(run.eaten))
        ),
    ]
}

/// The results menu after a pantry run of `mode`: the pantry or an event
/// pantry.
pub fn after_run(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let help = match mode {
        Mode::Leftovers => "A new Leftovers pantry run, from pantry 1 with 3 whiskers.",
        Mode::BinNight => "A new Bin night run, from pantry 1 with 3 whiskers.",
        _ => "A new pantry run, from pantry 1 with 3 whiskers.",
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        help,
        move |ctx| {
            // An event pantry starts again only while it is open.
            let today = parlor_core::challenges::today(ctx);
            let open = match mode {
                Mode::Leftovers => events::leftovers_open(today),
                Mode::BinNight => events::bin_night_open(today),
                _ => true,
            };
            if open {
                ctx.replace(Play::new(mode))
            } else {
                ctx.say(events::CLOSED)
            }
        },
    )
}

/// The results menu after a challenge.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same mazes, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Play::challenge(ch))
        },
    )
}

impl State<Parlor> for Play {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        // The pace is fixed for the run: a change in Settings applies from
        // the next one.
        self.clock = RunClock::start(ctx, ID);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::pantry_bed(ctx);
        self.start_loops(ctx);
        let intro = self.intro(ctx);
        self.say(ctx, intro);
        if let Some(line) = self.treat_line() {
            ctx.say_queued(line);
        }
        let new_here = !Progress::load(ctx).any();
        if new_here || self.mode == Mode::Practice {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.save_practice(ctx);
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.run.finished() {
            return;
        }
        self.clock.reset();
        sound::pantry_bed(ctx);
        self.start_loops(ctx);
        self.held = None;
        let s = self.status(ctx).remove(0);
        ctx.say("Back in the pantry.");
        ctx.say_queued(s);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ending.is_some() || self.run.finished() || event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            self.save_practice(ctx);
            self.held = None;
            self.stop_loops(ctx);
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        if !matches!(
            event,
            InputEvent::KeyDown { .. } | InputEvent::ControllerButtonDown { .. }
        ) {
            return;
        }
        match action {
            Some(
                a @ (ParlorAction::Left
                | ParlorAction::Right
                | ParlorAction::Up
                | ParlorAction::Down),
            ) => {
                if let Some(dir) = Dir::for_action(a) {
                    self.held = Some((dir, hold_delay(ctx)));
                    self.step(ctx, dir);
                }
            }
            Some(ParlorAction::Primary) => self.sniff(ctx),
            Some(ParlorAction::Secondary) => self.freeze(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status(ctx);
                self.last = self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => self.repeat(ctx),
            Some(ParlorAction::Hint) => self.hint(ctx),
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.ending = None;
                self.finish(ctx);
            }
            return;
        }
        // The cats and every timer run on game time: still while a line is
        // spoken, and at the run's pace. A key pressed meanwhile moves the
        // mouse at once; a held arrow steps on once the pantry runs. The
        // sniff cues keep real time.
        let game_dt = self.clock.game_dt(ctx, dt);
        self.play_ticks(ctx, dt);
        let hold_dt = if self.clock.held() { 0.0 } else { dt };
        self.hold(ctx, hold_dt);
        let evs = self.run.update(game_dt);
        let moved = evs.iter().any(|e| {
            matches!(
                e,
                Ev::CatMoved { .. } | Ev::CatHeard { .. } | Ev::Caught { .. }
            )
        });
        self.voice(ctx, evs);
        if moved {
            self.place_loops(ctx);
        }
    }

    /// The maze as text, for a sighted helper: `#` walls, `.` crumbs, `M`
    /// the mouse, and each cat by its first letter.
    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let r = &self.run;
        let mut out = vec![
            self.mode.name().to_string(),
            format!("Crumbs left: {}", r.left),
            format!("Whiskers: {}", r.whiskers),
            format!("Score: {}", r.score),
        ];
        for y in 0..r.maze.h {
            let row: String = (0..r.maze.w)
                .map(|x| {
                    let c = crate::rules::Cell::new(x, y);
                    if c == r.mouse {
                        'M'
                    } else if let Some(cat) = r.cats.iter().find(|k| k.pos == c) {
                        cat.name.chars().next().unwrap_or('C').to_ascii_uppercase()
                    } else if !r.maze.is_open(c) {
                        '#'
                    } else if r.has_crumb(c) {
                        '.'
                    } else {
                        ' '
                    }
                })
                .collect();
            out.push(row);
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let now = if self.run.between() {
            NEXT_PANTRY.to_string()
        } else if !self.run.started() {
            self.start_prompt(ctx)
        } else {
            format!(
                "Find every crumb. {} left. {}",
                sentence_start(&crumbs_words(self.run.left)),
                self.open_line()
            )
        };
        let mut text = format!(
            "{now} {} Each crumb is {CRUMB_POINTS} points and a cleared pantry {CLEAR_BONUS}.",
            self.controls(ctx)
        );
        for c in &self.run.cats {
            text.push_str(&format!(" The {}: {}", c.name, cat_rule(c.kind)));
        }
        if let Some(line) = self.treat_line() {
            let left = self.treats_left_words().unwrap_or_default();
            text.push_str(&format!(" {line} {}.", sentence_start(&left)));
        }
        if let Some(ch) = &self.challenge {
            text.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count the run.",
                ch.period.label().to_lowercase(),
                challenge::name(ch.period)
            ));
        }
        text
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// What a new pantry with no new cat says about the cats' pace: quicker
/// until the pace reaches its floor.
pub fn pace_words(rules: &crate::rules::Rules, pantry: u32) -> &'static str {
    if rules.pace.at(pantry) < rules.pace.at(pantry.saturating_sub(1)) {
        "The cats are quicker."
    } else {
        "The cats keep their pace."
    }
}

mod visuals_play;
