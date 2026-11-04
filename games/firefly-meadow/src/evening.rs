//! The evening on screen: walking the meadow by ear, catching fireflies
//! in time with their blink, setting the jar free, and the results after.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::words::a_or_an;
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, ScoreResult};

use crate::almanac;
use crate::challenge::{self, CHALLENGES_NAME};
use crate::meadow::{Place, Plan, P2};
use crate::progress::{self, Journal};
use crate::rules::{
    direction_words, duration_words, run_seed, Catch, Dir, Evening, Note, Walk, REACH,
};
use crate::sound;
use crate::species::{jar_notes, jar_species, list_words, Species};
use crate::{HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const EVENING_NAME: &str = "firefly_meadow_evening";

/// What kind of evening it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A walk in a meadow, in the real season.
    Walk(Place),
    /// The practice patch.
    Practice,
    /// A challenge's summer evening.
    Challenge(Period),
}

fn v3(p: P2) -> Vec3 {
    Vec3::new(p.0 as f32, p.1 as f32, 0.0)
}

/// The plan for `mode` today.
pub fn plan_for(ctx: &mut Ctx<Parlor>, mode: Mode) -> Plan {
    let date = today(ctx);
    match mode {
        Mode::Walk(place) => Plan::on(place, date),
        Mode::Practice => Plan::practice(date),
        Mode::Challenge(period) => {
            let ch = Challenge::current(ctx, ID, period);
            Plan::challenge(period, &mut ch.rng(), date)
        }
    }
}

/// The page read before an evening.
pub fn intro_page(ctx: &Ctx<Parlor>, plan: &Plan, mode: Mode) -> String {
    let p = ctx.control_name(ParlorAction::Primary);
    match mode {
        Mode::Practice => format!(
            "The practice patch: a small summer meadow with one Dipper just ahead of you. It flashes once, long and low, then waits in the dark. Walk up to it, and press {p} while it glows. It never runs out."
        ),
        Mode::Challenge(period) => challenge::intro(period, plan),
        Mode::Walk(_) => {
            let mut page = format!(
                "{}. {}",
                sentence_start(&plan.title()),
                almanac::season_words(plan.date)
            );
            if plan.midsummer {
                page.push_str(" It is midsummer: Midsummer night is open on the title.");
            }
            page
        }
    }
}

/// Starts an evening of `mode`: its page, then the meadow. Escape on the
/// page backs out. `replace` swaps out the screen that asked.
pub fn start(ctx: &mut Ctx<Parlor>, mode: Mode, replace: bool) {
    let plan = plan_for(ctx, mode);
    let seed = match mode {
        Mode::Challenge(period) => Challenge::current(ctx, ID, period).seed,
        _ => run_seed(),
    };
    let page = intro_page(ctx, &plan, mode);
    let mut screen = Some(EveningScreen::new(ctx, plan, mode, seed));
    let pages = Pages::briefing([page]).then(move |ctx| {
        if let Some(screen) = screen.take() {
            ctx.replace(screen);
        }
    });
    if replace {
        ctx.replace(pages);
    } else {
        ctx.push(pages);
    }
}

pub struct EveningScreen {
    pub evening: Evening,
    pub mode: Mode,
    challenge: Option<Challenge>,
    loops: Vec<Sound>,
    lit_loop: Option<Sound>,
    last: String,
    taught: HashSet<&'static str>,
    confirm_home: bool,
    ended: bool,
    clock: f64,
    scheduled: Vec<(f64, &'static str)>,
    /// Caught in all when the evening began, for what it opens.
    start_caught: u32,
    new_species: Vec<Species>,
    new_chords: Vec<crate::species::Chord>,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl EveningScreen {
    /// An evening from `plan`, with `seed` for its fireflies.
    pub fn new(ctx: &mut Ctx<Parlor>, plan: Plan, mode: Mode, seed: u64) -> EveningScreen {
        let journal = Journal::load(ctx);
        let challenge = match mode {
            Mode::Challenge(period) => Some(Challenge::current(ctx, ID, period)),
            _ => None,
        };
        EveningScreen {
            status_key: Default::default(),
            evening: Evening::new(plan, seed),
            mode,
            challenge,
            loops: Vec::new(),
            lit_loop: None,
            last: String::new(),
            taught: HashSet::new(),
            confirm_home: false,
            ended: false,
            clock: 0.0,
            scheduled: Vec::new(),
            start_caught: journal.caught,
            new_species: Vec::new(),
            new_chords: Vec::new(),
        }
    }

    fn plan(&self) -> &Plan {
        &self.evening.plan
    }

    fn counts(&self) -> bool {
        !matches!(self.mode, Mode::Practice)
    }

    /// Counts these cues as already put in words, so the screen does not
    /// say them: for a tutorial whose own steps teach them.
    pub fn taught_already(&mut self, cues: &[&'static str]) {
        self.taught.extend(cues.iter().copied());
    }

    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say(text.clone());
        self.last = text;
    }

    /// What is out tonight, in a sentence.
    fn out_words(&self) -> String {
        let e = &self.evening;
        match e.live() {
            0 if self.plan().delay > 0.0 => {
                "Fireflies are a summer thing. Tonight a late one may come out.".to_string()
            }
            0 => "No fireflies are out.".to_string(),
            1 => "1 firefly is out.".to_string(),
            n => format!("{n} fireflies are out."),
        }
    }

    fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let l = ctx.control_name(ParlorAction::Left);
        let r = ctx.control_name(ParlorAction::Right);
        let u = ctx.control_name(ParlorAction::Up);
        let d = ctx.control_name(ParlorAction::Down);
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let h = ctx.control_name(ParlorAction::Hint);
        let rep = ctx.control_name(ParlorAction::Repeat);
        let tab = format!("{} sets the jar free.", sentence_start(&t));
        let catch = format!("{} catches a firefly as it glows.", sentence_start(&p));
        format!(
            "{}, {d}, {l} and {r} walk: {u} is north. {catch} {tab} {s} for status, {h} for the nearest glow, {rep} repeats. Enter heads home, Escape pauses.",
            sentence_start(&u)
        )
    }

    /// The status line, in a fixed order: the clock in a challenge, the
    /// jar, what is near, the time out, the place.
    /// The status key, in layers. First: the time left in a challenge,
    /// your jar and the nearest glow. A second press: what you
    /// caught tonight, the time so far and the meadow.
    pub fn status_layers(&self) -> Vec<String> {
        let e = &self.evening;
        let mut first = Vec::new();
        let mut more = Vec::new();
        if let Some(left) = e.time_left() {
            first.push(format!("{} left", duration_words(left)));
        }
        first.push(format!("{} in the jar of {}", e.jar.len(), self.plan().jar));
        more.push(format!("{} caught tonight", e.caught));
        if !e.species_tonight.is_empty() {
            more.push(match e.species_tonight.len() {
                1 => "1 species".to_string(),
                n => format!("{n} species"),
            });
        }
        match e.nearest() {
            Some((_, d)) if d <= REACH => first.push("a firefly in reach".to_string()),
            Some((f, _)) => first.push(format!("nearest glow {}", direction_words(e.pos, f.pos))),
            None => first.push("nothing glowing".to_string()),
        }
        if e.challenge_free() {
            more.push(format!("{} so far", duration_words(e.time)));
        }
        more.push(self.plan().place.name().to_string());
        vec![
            format!("{}.", sentence_start(&first.join(", "))),
            format!("{}.", sentence_start(&more.join(", "))),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }

    fn start_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
        let bed = ctx
            .audio
            .start_loop(sound::bed(self.plan().season), None, Bus::AMBIENCE);
        ctx.audio.set_volume(bed, 0.45);
        self.loops.push(bed);
        for (key, pos, _) in self.plan().place.landmarks() {
            let s = ctx.audio.start_loop(key, Some(v3(pos)), Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.7);
            self.loops.push(s);
        }
        self.place_listener(ctx);
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.loops.drain(..).chain(self.lit_loop.take()) {
            ctx.audio.stop(s, 400);
        }
        self.scheduled.clear();
    }

    fn place_listener(&self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(v3(self.evening.pos), 0.0);
    }

    fn known(&self, ctx: &mut Ctx<Parlor>, species: Species) -> bool {
        Journal::load(ctx).found(species)
    }

    fn flashes_words(species: Species) -> String {
        match species.pattern().flashes {
            1 => "It flashes once.".to_string(),
            2 => "It flashes twice.".to_string(),
            n => format!("It flashes {n} times."),
        }
    }

    fn play_notes(&mut self, ctx: &mut Ctx<Parlor>) {
        let p = ctx.control_name(ParlorAction::Primary);
        for note in self.evening.drain_notes() {
            match note {
                Note::Glow { species, pos } => {
                    ctx.audio
                        .play_at(sound::glow(species), v3(pos), Bus::EFFECTS);
                }
                Note::Near { .. } => {
                    sound::play(ctx, sound::NEAR);
                    let text = format!(
                        "A firefly is in reach. {} catches it as it glows.",
                        sentence_start(&p)
                    );
                    if self.taught.insert("near") {
                        self.say(ctx, text);
                    }
                }
                Note::Missed { species } => {
                    sound::play(ctx, sound::MISSED);
                    let n = species.pattern().flashes;
                    let text = if self.taught.insert("missed") {
                        format!("It drifted on. Answer all {n} flashes.")
                    } else {
                        "Drifted on.".to_string()
                    };
                    self.say(ctx, text);
                }
                Note::Appeared { species } => {
                    let pos = self
                        .evening
                        .flies
                        .first()
                        .map(|f| f.pos)
                        .unwrap_or(self.evening.pos);
                    ctx.audio
                        .play_at(sound::glow(species), v3(pos), Bus::EFFECTS);
                    let way = direction_words(self.evening.pos, pos);
                    self.say(ctx, format!("A late firefly has come out, {way}."));
                }
                Note::Firework { pos } => {
                    let s = ctx.audio.play_at(sound::FIREWORK, v3(pos), Bus::AMBIENCE);
                    ctx.audio.set_volume(s, 0.6);
                    if self.taught.insert("firework") {
                        ctx.say_queued(
                            "Fireworks, far off on the horizon, for the Fourth of July.",
                        );
                    }
                }
                Note::Owl { pos } => {
                    let s = ctx.audio.play_at(sound::OWL, v3(pos), Bus::AMBIENCE);
                    ctx.audio.set_volume(s, 0.6);
                    if self.taught.insert("owl") {
                        ctx.say_queued("An owl calls from the edge of the meadow.");
                    }
                }
                Note::Leaves { pos } => {
                    let s = ctx.audio.play_at(sound::LEAVES, v3(pos), Bus::AMBIENCE);
                    ctx.audio.set_volume(s, 0.6);
                    if self.taught.insert("leaves") {
                        ctx.say_queued("Dry leaves skitter past on the wind.");
                    }
                }
                Note::Warn => self.say(ctx, "30 seconds left.".to_string()),
                Note::TimeUp => {
                    self.say(ctx, "The last light fades. Time is up.".to_string());
                }
            }
        }
    }

    fn walk(&mut self, ctx: &mut Ctx<Parlor>, dir: Dir, held: bool) {
        match self.evening.walk(dir, held) {
            Walk::Stepped => {
                sound::play(ctx, sound::STEP_GRASS);
                self.place_listener(ctx);
            }
            Walk::Edge(dir) => {
                sound::play(ctx, sound::EDGE);
                if !held {
                    let text = format!("Tall grass: the {} edge of the meadow.", dir.edge_words());
                    self.say(ctx, text);
                }
            }
            Walk::Wait => {}
        }
    }

    fn catch(&mut self, ctx: &mut Ctx<Parlor>) {
        let t = ctx.control_name(ParlorAction::Secondary);
        match self.evening.catch() {
            Catch::Caught {
                species,
                in_jar,
                full,
            } => {
                sound::play(ctx, sound::CATCH);
                let new = if self.counts() {
                    progress::note_catch(ctx, species)
                } else {
                    !self.known(ctx, species)
                };
                let mut text = format!(
                    "Caught {} {}. {in_jar} in the jar.",
                    a_or_an(species.name()),
                    species.name()
                );
                if new && self.counts() {
                    sound::play(ctx, sound::NEW_SPECIES);
                    self.new_species.push(species);
                    text = format!(
                        "Caught {} {}, new in your journal: {}. {in_jar} in the jar.",
                        a_or_an(species.name()),
                        species.name(),
                        species.blink_words()
                    );
                }
                if full {
                    text.push_str(&format!(
                        " The jar is full. {} sets them free.",
                        sentence_start(&t)
                    ));
                }
                self.say(ctx, text);
            }
            Catch::Answered { left, .. } => {
                sound::play(ctx, sound::ANSWER);
                let more = match left {
                    1 => "1 more flash".to_string(),
                    n => format!("{n} more flashes"),
                };
                let text = if self.taught.insert("answered") {
                    format!("One flash answered. Answer {more} of this blink.")
                } else {
                    format!("{}.", sentence_start(&more))
                };
                self.say(ctx, text);
            }
            Catch::Already => {
                sound::play(ctx, sound::EARLY);
                self.say(ctx, "Answered. Wait for the next.".to_string());
            }
            Catch::Early { species } => {
                sound::play(ctx, sound::EARLY);
                let text = if self.taught.insert("early") {
                    format!(
                        "Not yet: it is dark. Wait for its glow. {}",
                        Self::flashes_words(species)
                    )
                } else {
                    "Not yet: wait for its glow.".to_string()
                };
                self.say(ctx, text);
            }
            Catch::Nothing { nearest } => {
                sound::play(ctx, sound::NOTHING);
                let text = match nearest {
                    Some(pos) => format!(
                        "Nothing in reach. Nearest glow: {}.",
                        direction_words(self.evening.pos, pos)
                    ),
                    None => format!("Nothing in reach. {}", self.none_out_words()),
                };
                self.say(ctx, text);
            }
            Catch::Full => {
                sound::play(ctx, sound::FULL);
                self.say(
                    ctx,
                    format!("The jar is full. {} sets them free.", sentence_start(&t)),
                );
            }
            Catch::Over => {}
        }
    }

    /// Why nothing glows, when nothing does.
    fn none_out_words(&self) -> String {
        if self.plan().delay > 0.0 && self.evening.live() == 0 && self.evening.caught == 0 {
            "No fireflies yet.".to_string()
        } else if self.plan().respawn {
            "Resting. More soon.".to_string()
        } else {
            "No fireflies now. Enter heads home.".to_string()
        }
    }

    /// Schedules each species' held note, one after another, `gap` apart.
    fn bloom(&mut self, species: &[Species], gap: f64) {
        for (i, &s) in species.iter().enumerate() {
            self.scheduled
                .push((self.clock + 0.3 + i as f64 * gap, sound::bloom(s)));
        }
    }

    fn release(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(r) = self.evening.release() else {
            sound::play(ctx, sound::NOTHING);
            let p = ctx.control_name(ParlorAction::Primary);
            self.say(
                ctx,
                format!(
                    "The jar is empty. {} catches a firefly as it glows.",
                    sentence_start(&p)
                ),
            );
            return;
        };
        sound::play(ctx, sound::RELEASE);
        let distinct = jar_species(&r.jar);
        self.bloom(&distinct, 0.35);
        let new = if self.counts() {
            progress::note_release(ctx, r.chord)
        } else {
            false
        };
        let n = r.jar.len();
        let set = match n {
            1 => "Set free: 1 firefly".to_string(),
            n => format!("Set free: {n} fireflies"),
        };
        let notes: Vec<&str> = jar_notes(&r.jar).iter().map(|n| n.name()).collect();
        let notes = match notes.len() {
            1 => format!("note {}", notes[0]),
            _ => format!("notes {}", list_words(&notes)),
        };
        let text = match r.chord {
            Some(c) if new => {
                self.new_chords.push(c);
                format!("{set}. A new chord: {}.", c.name())
            }
            Some(c) => format!("{set}. The meadow lights up with {}.", c.name()),
            None => format!("{set}, {notes}. The meadow lights up."),
        };
        if self.lit_loop.is_none() {
            let s = ctx.audio.start_loop(sound::LIT, None, Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.4);
            self.lit_loop = Some(s);
        }
        self.say(ctx, text);
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        let e = &self.evening;
        let Some((fly, d)) = e.nearest() else {
            sound::play(ctx, sound::NOTHING);
            let text = self.none_out_words();
            self.say(ctx, text);
            return;
        };
        let (species, pos) = (fly.species, fly.pos);
        ctx.audio
            .play_at(sound::glow(species), v3(pos), Bus::EFFECTS);
        let p = ctx.control_name(ParlorAction::Primary);
        let text = if d <= REACH {
            format!(
                "In reach. Press {p} as it glows. {}",
                Self::flashes_words(species)
            )
        } else {
            format!(
                "Nearest glow: {}. Walk toward its glow.",
                direction_words(e.pos, pos)
            )
        };
        self.say(ctx, text);
    }

    fn head_home(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.confirm_home {
            self.confirm_home = true;
            sound::play(ctx, sound::NOTHING);
            ctx.say("Head home? Enter again heads home. Any other key keeps walking.");
            return;
        }
        self.evening.head_home();
        self.finish(ctx);
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        self.ended = true;
        self.stop_sounds(ctx);
        ctx.audio.play_music(Some(sound::HOME_MUSIC), false);
        if let (Mode::Challenge(period), Some(ch)) = (self.mode, self.challenge.take()) {
            let pages = challenge::finish(ctx, &ch, &self.evening);
            let summary = format!("{} result: {} points", ch.title(), self.evening.score());
            let mut results = Some(challenge::after(period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let longest = if self.counts() {
            Some(progress::note_evening(ctx, self.evening.time))
        } else {
            None
        };
        let journal = Journal::load(ctx);
        let pages = self.result_pages(&journal, longest);
        let summary = self.summary();
        let mode = self.mode;
        let mut results = Some(after_evening(mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    /// The results menu's title.
    pub fn summary(&self) -> String {
        let e = &self.evening;
        match e.caught {
            1 => "Home again. 1 firefly caught".to_string(),
            n => format!("Home again. {n} fireflies caught"),
        }
    }

    /// The pages after an evening.
    pub fn result_pages(&self, journal: &Journal, longest: Option<ScoreResult>) -> Vec<String> {
        let e = &self.evening;
        let time = duration_words(e.time);
        let caught = match e.caught {
            1 => "1 firefly".to_string(),
            n => format!("{n} fireflies"),
        };
        let species = match e.species_tonight.len() {
            0 => String::new(),
            1 => ", 1 species".to_string(),
            n => format!(", {n} species"),
        };
        let mut first = format!("Home again after {time} in the meadow. {caught} caught{species}.");
        if e.releases > 0 {
            first.push_str(&match e.releases {
                1 => " 1 jar set free.".to_string(),
                n => format!(" {n} jars set free."),
            });
        }
        match longest {
            Some(ScoreResult::First) => first.push_str(" Your first evening here."),
            Some(ScoreResult::NewBest { .. }) => first.push_str(" Your longest evening yet."),
            _ => {}
        }
        let mut second = Vec::new();
        if !self.new_species.is_empty() {
            let names: Vec<&str> = self.new_species.iter().map(|s| s.name()).collect();
            second.push(format!("New in your journal: {}.", list_words(&names)));
        }
        if !self.new_chords.is_empty() {
            let names: Vec<&str> = self.new_chords.iter().map(|c| c.name()).collect();
            let label = if names.len() == 1 {
                "New chord"
            } else {
                "New chords"
            };
            second.push(format!("{label}: {}.", list_words(&names)));
        }
        for place in Place::WALKS {
            if self.start_caught < place.opens_at() && journal.caught >= place.opens_at() {
                second.push(format!(
                    "{} is open: choose it with the Meadow row on the title.",
                    sentence_start(place.name())
                ));
            }
        }
        if self.counts() {
            second.push(format!(
                "Your journal: {} of {} species, {} of 6 chords, {} caught in all.",
                journal.species_found(),
                Species::ALL.len(),
                journal.chords_found(),
                match journal.caught {
                    1 => "1 firefly".to_string(),
                    n => format!("{n} fireflies"),
                }
            ));
        }
        let mut pages = vec![first];
        if !second.is_empty() {
            pages.push(second.join(" "));
        }
        pages
    }
}

impl Evening {
    /// Whether the evening runs without a clock.
    pub fn challenge_free(&self) -> bool {
        self.plan.clock.is_none()
    }
}

/// The results menu after an evening: another evening, the results
/// again, and back to the title.
pub fn after_evening(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let help = match mode {
        Mode::Practice => "Another visit to the practice patch.".to_string(),
        Mode::Walk(place) => format!("Another evening in {}.", place.name()),
        Mode::Challenge(_) => "The same evening again.".to_string(),
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        help,
        move |ctx| start(ctx, mode, true),
    )
}

/// Keys a player presses for their screen reader or as a modifier, which
/// the meadow leaves alone.
fn is_quiet_key(key: Key) -> bool {
    matches!(
        key,
        Key::LCtrl
            | Key::RCtrl
            | Key::LShift
            | Key::RShift
            | Key::LAlt
            | Key::RAlt
            | Key::Insert
            | Key::Other(_)
    )
}

impl State<Parlor> for EveningScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        EVENING_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.start_sounds(ctx);
        let text = self.out_words();
        self.say(ctx, text);
        let c = self.controls(ctx);
        ctx.say_queued(c);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        if self.loops.is_empty() {
            self.start_sounds(ctx);
        }
        let s = self.status_layers().remove(0);
        ctx.say(format!("Back in the meadow. {s}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ended {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if event.is_repeat() {
                return;
            }
            self.confirm_home = false;
            let leave = match self.mode {
                Mode::Challenge(_) => Leave::Challenge(CHALLENGES_NAME),
                _ => Leave::Title(TITLE_NAME),
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let walking = matches!(
            action,
            Some(ParlorAction::Left | ParlorAction::Right | ParlorAction::Up | ParlorAction::Down)
        );
        if event.is_repeat() && !walking {
            return;
        }
        if action.is_none() && matches!(event.pressed(), Some(Key::Return | Key::KpEnter)) {
            self.head_home(ctx);
            return;
        }
        if action.is_some() && !event.is_repeat() {
            self.confirm_home = false;
        }
        let held = event.is_repeat();
        match action {
            Some(ParlorAction::Up) => self.walk(ctx, Dir::North, held),
            Some(ParlorAction::Down) => self.walk(ctx, Dir::South, held),
            Some(ParlorAction::Left) => self.walk(ctx, Dir::West, held),
            Some(ParlorAction::Right) => self.walk(ctx, Dir::East, held),
            Some(ParlorAction::Primary) => self.catch(ctx),
            Some(ParlorAction::Secondary) => self.release(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status_layers();
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                if self.last.is_empty() {
                    let s = self.status_layers().remove(0);
                    ctx.say(s);
                } else {
                    ctx.say(self.last.clone());
                }
            }
            Some(ParlorAction::Hint) => self.hint(ctx),
            None => {
                if let InputEvent::KeyDown {
                    key, repeat: false, ..
                } = event
                {
                    if !is_quiet_key(*key) {
                        self.confirm_home = false;
                        sound::play(ctx, sound::NOTHING);
                        ctx.say("Nothing happens. F1 lists the controls.");
                    }
                }
            }
        }
        self.play_notes(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            return;
        }
        self.clock += dt;
        self.evening.update(dt);
        self.play_notes(ctx);
        let now = self.clock;
        let due: Vec<&'static str> = self
            .scheduled
            .iter()
            .filter(|(t, _)| *t <= now)
            .map(|(_, k)| *k)
            .collect();
        self.scheduled.retain(|(t, _)| *t > now);
        for key in due {
            ctx.audio.play(key, Bus::EFFECTS);
        }
        if self.evening.lit <= 0.0 {
            if let Some(s) = self.lit_loop.take() {
                ctx.audio.stop(s, 1500);
            }
        }
        if self.evening.over {
            self.finish(ctx);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![
            format!("Firefly Meadow: {}", self.plan().title()),
            self.status(),
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut now = format!(
            "{} Walk toward a firefly's tone, and press {} while it glows. A firefly that flashes more than once needs every flash of one blink answered.",
            self.status(),
            ctx.control_name(ParlorAction::Primary)
        );
        if let Mode::Challenge(period) = self.mode {
            now.push_str(&format!(
                " This is the {} challenge. Leaving from the pause menu does not count it.",
                period.label().to_lowercase()
            ));
        }
        format!("{now} {}", self.controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_eveningscreen;
