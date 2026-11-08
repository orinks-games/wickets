//! Flying a lander down: the play screen.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{
    units, Challenge, Date, Pages, Parlor, ParlorAction, Period, ScoreResult, StatusKey,
};

use crate::challenge::{self, Job, Plan, CHALLENGES_NAME};
use crate::progress::{self, Progress, Saved};
use crate::rules::{
    advice, next_call_every, seconds_words, shown_call_every, speed_words, survey_moon, tone_pitch,
    Advice, Ev, Flight, Moon, Outcome, MOONS,
};
use crate::{events, moon_summary, sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "soft_landing_flight";

/// Seconds from touchdown to what comes next, so the thud or the crunch is
/// heard and a key still held from flight does not skip the first page.
pub const END_PAUSE: f64 = 2.0;

/// What kind of flight the screen is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Survey moon `n`, from 1.
    Moon(u32),
    /// The practice pad: endless fuel, nothing to lose.
    Practice,
    /// A challenge run.
    Challenge(Period),
    /// Tonight's full moon pad.
    FullMoon,
    /// The Tranquility pad, on moon landing day.
    Tranquility,
}

impl Mode {
    /// The best-score board of an event row.
    pub fn event_board(self) -> Option<&'static str> {
        match self {
            Mode::FullMoon => Some(events::FULL_MOON_BOARD),
            Mode::Tranquility => Some(events::TRANQUILITY_BOARD),
            _ => None,
        }
    }

    /// Whether this mode can be flown on `date`: an event row only while
    /// its window is open.
    pub fn open_on(self, date: Date) -> bool {
        match self {
            Mode::FullMoon => events::full_moon_pad(date).is_some(),
            Mode::Tranquility => events::tranquility_open(date),
            _ => true,
        }
    }

    /// What Play again says once an event row has closed.
    pub fn closed_words(self) -> &'static str {
        match self {
            Mode::Tranquility => events::TRANQUILITY_OVER,
            _ => events::FULL_MOON_OVER,
        }
    }
}

/// Seconds into a descent in which a Too fast line waits for Descent
/// begun instead of cutting it off.
const START_GRACE: f64 = 0.5;

pub struct Descent {
    pub flight: Flight,
    pub mode: Mode,
    job: Option<Job>,
    challenge: Option<Challenge>,
    /// Cues already explained in words this run.
    taught: HashSet<&'static str>,
    /// Lines the tutorial says itself on this step, which the screen keeps
    /// quiet so the two do not cut each other off: "hint", "landed".
    /// "prompts": the lesson speaks its steps, which Too fast and the
    /// call-outs must not cut off.
    pub lesson_says: HashSet<&'static str>,
    status_key: StatusKey,
    /// The last call-out, for Repeat.
    last_call: String,
    /// The engine key that started the descent is still held: it does not
    /// fire the engine until it is let go.
    await_release: bool,
    engine_loop: Sound,
    tone_loop: Sound,
    /// Seconds until the results, once down.
    ending: Option<f64>,
    practice_unsaved: u32,
    /// Hints asked for, so the tutorial hears the hint key.
    pub hints: u32,
}

impl Descent {
    pub fn new(mode: Mode) -> Descent {
        let flight = match mode {
            Mode::Moon(n) => Flight::new(survey_moon(n)),
            Mode::Tranquility => Flight::new(events::tranquility()),
            _ => Flight::practice(),
        };
        Descent {
            flight,
            mode,
            job: None,
            challenge: None,
            taught: HashSet::new(),
            lesson_says: HashSet::new(),
            status_key: StatusKey::new(),
            last_call: String::new(),
            await_release: false,
            engine_loop: Sound::NONE,
            tone_loop: Sound::NONE,
            ending: None,
            practice_unsaved: 0,
            hints: 0,
        }
    }

    /// An event row's flight onto `moon`.
    pub fn event(mode: Mode, moon: Moon) -> Descent {
        let mut d = Descent::new(mode);
        d.flight = Flight::new(moon);
        d
    }

    /// A run of the challenge's moons, from the first.
    pub fn challenge(ch: Challenge) -> Descent {
        let job = Job::new(Plan::for_challenge(&ch));
        let mut d = Descent::new(Mode::Challenge(ch.period));
        d.flight = job.flight();
        d.job = Some(job);
        d.challenge = Some(ch);
        d
    }

    /// True the first time a cue is met on this save. Saved, so a
    /// teaching line is heard once, not on every moon.
    fn first(&mut self, ctx: &mut Ctx<Parlor>, cue: &'static str) -> bool {
        self.taught.insert(cue) && progress::learn(ctx, cue)
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    fn hints_allowed(&self) -> bool {
        self.job.as_ref().is_none_or(|j| j.plan.hints)
    }

    /// "Thistle, moon 1 of 3" or "Quill, moon 2 of 3 of the weekly survey".
    fn where_words(&self) -> String {
        let name = self.flight.moon.name;
        match (self.mode, &self.job) {
            (Mode::Moon(n), _) => format!("{name}, moon {n} of {MOONS}"),
            (Mode::Practice, _) => format!("Practice pad on {name}"),
            (Mode::Challenge(period), Some(job)) if job.moons() > 1 => format!(
                "{name}, moon {} of {}, {}",
                job.index + 1,
                job.moons(),
                challenge::name(period)
            ),
            (Mode::Challenge(period), _) => format!("{name}, {}", challenge::name(period)),
            (Mode::FullMoon, _) => format!("Full moon pad on {name}"),
            (Mode::Tranquility, _) => format!("{name}, on moon landing day"),
        }
    }

    fn start_prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} starts the descent.",
            sentence_start(&Self::key(ctx, ParlorAction::Primary))
        )
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let fuel = if self.flight.unlimited {
            "endless fuel".to_string()
        } else {
            format!(
                "fuel for {}",
                seconds_words(self.flight.fuel.floor() as u32)
            )
        };
        format!(
            "{}. {}, {fuel}. {}",
            self.where_words(),
            sentence_start(&moon_summary(&self.flight.moon)),
            self.start_prompt(ctx)
        )
    }

    /// Every control, in a few sentences.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "Hold {} to fire the main engine, and let go to fall. {} asks for a call-out. {} and {} change how often altitude is called. {} speaks the status, {} repeats the last call-out, {} gives a hint, and Escape pauses.",
            k(ParlorAction::Primary),
            sentence_start(&k(ParlorAction::Secondary)),
            sentence_start(&k(ParlorAction::Up)),
            k(ParlorAction::Down),
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Repeat),
            k(ParlorAction::Hint)
        )
    }

    fn fuel_words(&self) -> String {
        if self.flight.unlimited {
            "fuel endless".to_string()
        } else {
            format!("fuel {} percent", self.flight.fuel_percent())
        }
    }

    /// The status key's layers. First: height, speed and fuel, the
    /// numbers a pilot tracks. Second: where, the safe speed and the hint.
    pub fn status(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let f = &self.flight;
        if let Some(outcome) = f.outcome {
            return vec![self.down_words(outcome)];
        }
        let first = if !f.started() {
            format!(
                "{}, waiting at {}, {}. {}",
                self.where_words(),
                units::length(f.altitude),
                self.fuel_words(),
                self.start_prompt(ctx)
            )
        } else {
            let s = f.speed();
            let motion = if s < -0.2 {
                format!("climbing {}", speed_words(-s))
            } else {
                format!("falling {}", speed_words(s))
            };
            format!(
                "{}, {motion}, {}.",
                units::length(f.altitude),
                self.fuel_words()
            )
        };
        let hint = if self.hints_allowed() && f.started() {
            self.hint_words(ctx)
        } else {
            String::new()
        };
        let second = format!(
            "{}. Safe speed {}. {hint}",
            self.where_words(),
            speed_words(f.limit())
        );
        vec![first, second.trim().to_string()]
    }

    /// "falling 9 metres a second" or "climbing 2 metres a second".
    fn velocity_words(&self) -> String {
        let s = self.flight.speed();
        if s < 0.0 {
            format!("climbing {}", speed_words(-s))
        } else {
            format!("falling {}", speed_words(s))
        }
    }

    fn down_words(&self, outcome: Outcome) -> String {
        let name = self.flight.moon.name;
        match outcome {
            Outcome::Landed { speed } => {
                format!("Down safely on {name} at {}.", speed_words(speed))
            }
            Outcome::Crashed { speed } => format!("Crashed at {}.", speed_words(speed)),
        }
    }

    /// Mission control's radio before a call-out: the click, with crickets
    /// behind it under a full moon, or the Quindar tone at Tranquility.
    /// Returns what to say the first time it is heard.
    fn radio(&mut self, ctx: &mut Ctx<Parlor>) -> Option<&'static str> {
        match self.mode {
            Mode::FullMoon => {
                sound::play(ctx, sound::RADIO_CLICK);
                sound::play_vol(ctx, sound::CRICKETS, sound::CRICKETS_VOLUME);
                self.first(ctx, "crickets")
                    .then_some("Crickets on the radio. Only atmosphere.")
            }
            Mode::Tranquility => {
                sound::play(ctx, sound::QUINDAR);
                self.first(ctx, "quindar")
                    .then_some("The Quindar beep: same as the radio click.")
            }
            _ => {
                sound::play(ctx, sound::RADIO_CLICK);
                None
            }
        }
    }

    /// Mission control's call-out on request: altitude, speed, fuel.
    fn call_now(&mut self, ctx: &mut Ctx<Parlor>) {
        let f = &self.flight;
        if f.finished() {
            let line = self.down_words(f.outcome.expect("finished"));
            ctx.say(line);
            return;
        }
        let line = format!(
            "Altitude {}, {}, {}.",
            units::length(f.altitude),
            self.velocity_words(),
            self.fuel_words()
        );
        let first = self.radio(ctx);
        ctx.say(line.clone());
        if let Some(first) = first {
            ctx.say_queued(first);
        }
        self.last_call = line;
    }

    /// What the hint key says now.
    pub fn hint_words(&self, ctx: &Ctx<Parlor>) -> String {
        let p = Self::key(ctx, ParlorAction::Primary);
        match advice(&self.flight) {
            Advice::Ready => format!(
                "{} Then fall freely, and burn near the pad.",
                self.start_prompt(ctx)
            ),
            Advice::Climbing => format!("You are climbing. Let go of {p}."),
            Advice::Steady => "Speed is safe. Burn briefly only if the tone rises.".to_string(),
            Advice::Coast { secs } => format!("Fall for about {}, then burn.", seconds_words(secs)),
            Advice::Burn { secs: 0 } => "Burn now with the last of the fuel.".to_string(),
            Advice::Burn { secs } => {
                format!("Burn now and hold for about {}.", seconds_words(secs))
            }
            Advice::NoFuel => "Fuel is out. Nothing to burn: brace for touchdown.".to_string(),
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hints += 1;
        if !self.hints_allowed() {
            ctx.play_ui(UiSound::Error);
            ctx.say(format!(
                "No hints on this challenge. {} still reads your speed.",
                sentence_start(&Self::key(ctx, ParlorAction::Status))
            ));
            return;
        }
        if self.lesson_says.contains("hint") {
            return;
        }
        let line = self.hint_words(ctx);
        ctx.say(line);
    }

    fn repeat(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.last_call.is_empty() {
            let line = if self.flight.started() {
                format!(
                    "No call-out yet. {} asks for one.",
                    sentence_start(&Self::key(ctx, ParlorAction::Secondary))
                )
            } else {
                self.start_prompt(ctx)
            };
            ctx.say(line);
        } else {
            ctx.say(self.last_call.clone());
        }
    }

    /// Up calls less often, Down more often.
    fn change_interval(&mut self, ctx: &mut Ctx<Parlor>, more: bool) {
        let now = self.flight.call_every;
        match next_call_every(now, more) {
            Some(every) => {
                self.flight.set_call_every(every);
                if self.mode != Mode::Practice {
                    progress::save_call_every(ctx, every);
                }
                ctx.play_ui(UiSound::Move);
                ctx.say(format!(
                    "Call-outs every {}.",
                    units::count(shown_call_every(every, self.flight.feet).into())
                ));
            }
            None => {
                ctx.play_ui(UiSound::Error);
                let end = if more {
                    "the most often"
                } else {
                    "the least often"
                };
                ctx.say(format!(
                    "Call-outs already every {}, {end}.",
                    units::count(shown_call_every(now, self.flight.feet).into())
                ));
            }
        }
    }

    fn start(&mut self, ctx: &mut Ctx<Parlor>) {
        self.flight.start();
        self.await_release = true;
        if self.first(ctx, "descent") {
            ctx.say("Descent begun. Listen to the tone.");
        } else {
            ctx.say("Descent begun.");
        }
        self.sync_loops(ctx);
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in [&mut self.engine_loop, &mut self.tone_loop] {
            if *s != Sound::NONE {
                ctx.audio.stop(*s, 150);
                *s = Sound::NONE;
            }
        }
    }

    /// The engine loop while it burns, and the descent tone at the pitch of
    /// the fall, while flying.
    fn sync_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        let f = &self.flight;
        if !f.started() || f.finished() {
            self.stop_loops(ctx);
            return;
        }
        if self.tone_loop == Sound::NONE {
            self.tone_loop = sound::start_loop(ctx, sound::DESCENT_TONE, sound::TONE_VOLUME);
        }
        ctx.audio
            .set_pitch(self.tone_loop, tone_pitch(f.speed(), f.limit()));
        match (f.engine, self.engine_loop == Sound::NONE) {
            (true, true) => {
                self.engine_loop = sound::start_loop(ctx, sound::ENGINE, sound::ENGINE_VOLUME)
            }
            (false, false) => {
                ctx.audio.stop(self.engine_loop, 120);
                self.engine_loop = Sound::NONE;
            }
            _ => {}
        }
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        for ev in evs {
            match ev {
                Ev::EngineOn => {
                    if self.first(ctx, "engine") {
                        ctx.say("Engine on.");
                    }
                }
                Ev::EngineOff => {}
                Ev::FuelOut => {
                    ctx.play_ui(UiSound::Error);
                    ctx.say("Fuel out. The engine is dead.");
                }
                Ev::FuelLow(pct) => {
                    sound::play(ctx, sound::FUEL_LOW);
                    ctx.say(format!("Fuel {pct} percent."));
                }
                Ev::TooFast => {
                    let line = if self.first(ctx, "tone") {
                        "Too fast. The tone rises with speed."
                    } else {
                        "Too fast."
                    };
                    // At the start of a descent, after Descent begun, not
                    // over it; in the tutorial, after the lesson's line.
                    if self.flight.time < START_GRACE || self.lesson_says.contains("prompts") {
                        ctx.say_queued(line);
                    } else {
                        ctx.say(line);
                    }
                }
                Ev::SafeSpeed => ctx.say("Safe speed."),
                Ev::CallOut { altitude, speed } => {
                    let first = self.radio(ctx);
                    let line = if speed < 0.0 {
                        format!("{}, climbing.", units::count(altitude.into()))
                    } else {
                        format!(
                            "{}, falling {:.0}.",
                            units::count(altitude.into()),
                            units::from_metres(speed)
                        )
                    };
                    if self.lesson_says.contains("prompts") {
                        // In the tutorial, after the lesson's line.
                        ctx.say_queued(line.clone());
                    } else {
                        ctx.say(line.clone());
                    }
                    self.last_call = line;
                    if let Some(first) = first {
                        ctx.say_queued(first);
                    }
                    if self.first(ctx, "callout") {
                        ctx.say_queued("Call-outs give your height and falling speed.");
                    }
                }
                Ev::Beacon => {
                    sound::play_vol(ctx, sound::PAD_BEACON, sound::BEACON_VOLUME);
                    if self.first(ctx, "beacon") {
                        ctx.say_queued("Pad beacon: faster as you near it.");
                    }
                }
                Ev::Touchdown(outcome) => {
                    self.stop_loops(ctx);
                    sound::play(
                        ctx,
                        if outcome.landed() {
                            sound::TOUCHDOWN
                        } else {
                            sound::CRASH
                        },
                    );
                    if !(outcome.landed() && self.lesson_says.contains("landed")) {
                        let line = self.down_words(outcome);
                        ctx.say(line);
                    }
                    if self.mode == Mode::Tranquility && outcome.landed() {
                        ctx.say_queued("Tranquility Base here. The Eagle has landed.");
                    }
                    self.ending = Some(END_PAUSE);
                }
            }
        }
    }

    fn save_practice(&mut self, ctx: &mut Ctx<Parlor>) {
        progress::save_practice(ctx, self.practice_unsaved);
        self.practice_unsaved = 0;
    }

    /// After touchdown: the results, the next challenge moon, or another
    /// practice descent.
    fn after_touchdown(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(outcome) = self.flight.outcome else {
            return;
        };
        match self.mode {
            Mode::Practice => {
                if outcome.landed() {
                    self.practice_unsaved += 1;
                }
                let every = self.flight.call_every;
                self.flight = Flight::practice();
                self.flight.set_call_every(every);
                self.taught.clear();
                self.last_call.clear();
                let p = self.start_prompt(ctx);
                ctx.say(format!("Back to the top. {p}"));
            }
            Mode::Moon(n) => {
                ctx.audio.set_bed(None);
                let saved = progress::save_flight(ctx, n, &self.flight);
                let pages = result_pages(&self.flight, &saved);
                let summary = result_summary(&self.flight);
                let mut results = Some(after_moon(n, outcome.landed(), summary, pages.clone()));
                ctx.push(Pages::new(pages).then(move |ctx| {
                    if let Some(results) = results.take() {
                        ctx.pop_to_quiet(TITLE_NAME);
                        ctx.push(results);
                    }
                }));
            }
            Mode::Challenge(_) => self.next_challenge_moon(ctx),
            Mode::FullMoon | Mode::Tranquility => {
                ctx.audio.set_bed(None);
                let board = self.mode.event_board().expect("an event row");
                let open = self.mode.open_on(today(ctx));
                let saved = if open {
                    progress::save_event(ctx, board, &self.flight)
                } else {
                    Saved::nothing()
                };
                let mut pages = result_pages(&self.flight, &saved);
                if !open {
                    pages.push(format!(
                        "{}, so this flight stays off its board.",
                        self.mode.closed_words()
                    ));
                }
                let summary = result_summary(&self.flight);
                let mode = self.mode;
                let moon = self.flight.moon;
                let mut results = Some(after_event(
                    mode,
                    moon,
                    outcome.landed(),
                    summary,
                    pages.clone(),
                ));
                ctx.push(Pages::new(pages).then(move |ctx| {
                    if let Some(results) = results.take() {
                        ctx.pop_to_quiet(TITLE_NAME);
                        ctx.push(results);
                    }
                }));
            }
        }
    }

    fn next_challenge_moon(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(job) = self.job.as_mut() else {
            return;
        };
        if let Some(next) = job.advance(&self.flight) {
            let banked = job.banked;
            let name = self.flight.moon.name;
            let every = self.flight.call_every;
            self.flight = next;
            self.flight.set_call_every(every);
            self.last_call.clear();
            ctx.say(format!(
                "Down on {name}. {banked} points banked. Next, {}, fuel {} percent.",
                challenge::moon_words(&self.flight.moon),
                self.flight.fuel_percent()
            ));
            let p = self.start_prompt(ctx);
            ctx.say_queued(p);
            return;
        }
        let (Some(job), Some(ch)) = (self.job.take(), self.challenge.take()) else {
            return;
        };
        ctx.audio.set_bed(None);
        let pages = challenge::finish(ctx, &ch, &job);
        let summary = challenge::summary(&ch, &job);
        let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }

    fn leave(&self) -> Leave {
        if self.challenge.is_some() {
            Leave::Challenge(CHALLENGES_NAME)
        } else {
            Leave::Title(TITLE_NAME)
        }
    }
}

/// Keys a player presses for their screen reader or as a modifier, which
/// the lander leaves alone.
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

/// "1 minute 5 seconds", "40 seconds".
pub fn time_words(secs: f64) -> String {
    let total = secs.round().max(0.0) as u32;
    let (m, s) = (total / 60, total % 60);
    let mins = match m {
        0 => String::new(),
        1 => "1 minute".to_string(),
        m => format!("{m} minutes"),
    };
    match (m, s) {
        (0, s) => seconds_words(s),
        (_, 0) => mins,
        (_, s) => format!("{mins} {}", seconds_words(s)),
    }
}

/// The one-line headline of a flight, the results menu's title.
pub fn result_summary(flight: &Flight) -> String {
    let name = flight.moon.name;
    match flight.outcome {
        Some(o @ Outcome::Landed { .. }) => {
            format!("{}, {} points", sentence_start(name), flight.score(o))
        }
        Some(Outcome::Crashed { speed }) => {
            format!("Crashed on {name} at {}", speed_words(speed))
        }
        None => name.to_string(),
    }
}

/// The pages after a survey flight.
pub fn result_pages(flight: &Flight, saved: &Saved) -> Vec<String> {
    let moon = &flight.moon;
    let name = moon.name;
    let Some(outcome) = flight.outcome else {
        return vec![name.to_string()];
    };
    let mut pages = Vec::new();
    match outcome {
        Outcome::Landed { speed: _ } => {
            let points = flight.score(outcome);
            let score = match saved.score {
                Some(ScoreResult::First) => format!("{points} points, your first on {name}."),
                Some(ScoreResult::NewBest { previous }) => {
                    format!("{points} points, a new best, beating {previous}.")
                }
                Some(ScoreResult::Short { best }) => {
                    format!("{points} points. Your best is {best}.")
                }
                None => format!("{points} points."),
            };
            let soft = if saved.softest {
                " Your softest touchdown yet."
            } else {
                ""
            };
            // Touchdown already said where and how fast: the page leads with
            // the score, and a moon this landing opened.
            let opened = saved
                .opened
                .map(|next| format!(" {} is open: moon {next}.", survey_moon(next).name))
                .unwrap_or_default();
            pages.push(format!("{score}{soft}{opened}"));
            pages.push(format!(
                "Fuel left: {} percent. Flight time: {}.",
                flight.fuel_percent(),
                time_words(flight.time)
            ));
        }
        Outcome::Crashed { speed } => {
            // A hair over came from letting go at the pad, not a late burn.
            let tip = if speed < moon.safe + 1.0 {
                "Near the pad, hold the engine until Safe speed."
            } else {
                "Burn earlier."
            };
            pages.push(format!(
                "Safe speed on {name} is {}. {tip}",
                speed_words(moon.safe)
            ));
            pages.push(format!(
                "Fuel left: {} percent. Flight time: {}.",
                flight.fuel_percent(),
                time_words(flight.time)
            ));
        }
    }
    if saved.survey_done {
        pages.push(
            "You have landed on every survey moon. Choose Moon select to fly any of them again.".to_string(),
        );
    }
    pages
}

/// The results menu after a survey flight.
pub fn after_moon(n: u32, landed: bool, summary: String, pages: Vec<String>) -> Results {
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if landed && n < MOONS {
        let next = n + 1;
        let moon: Moon = survey_moon(next);
        results = results.next(
            format!("Next moon: {}", moon.name),
            format!("{}.", sentence_start(&moon_summary(&moon))),
            move |ctx| ctx.replace(Descent::new(Mode::Moon(next))),
        );
    }
    let again = if landed {
        labels::PLAY_AGAIN
    } else {
        labels::TRY_AGAIN
    };
    results.again(
        again,
        format!(
            "{} again, from the top with a full tank.",
            survey_moon(n).name
        ),
        move |ctx| ctx.replace(Descent::new(Mode::Moon(n))),
    )
}

/// The results menu after an event row's flight.
pub fn after_event(
    mode: Mode,
    moon: Moon,
    landed: bool,
    summary: String,
    pages: Vec<String>,
) -> Results {
    let again = if landed {
        labels::PLAY_AGAIN
    } else {
        labels::TRY_AGAIN
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        again,
        format!("{} again, from the top with a full tank.", moon.name),
        move |ctx| {
            if mode.open_on(today(ctx)) {
                ctx.replace(Descent::event(mode, moon));
            } else {
                ctx.play_ui(UiSound::Back);
                ctx.pop_to_with(TITLE_NAME, mode.closed_words());
            }
        },
    )
}

/// The results menu after a challenge run.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same moons, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Descent::challenge(ch))
        },
    )
}

impl State<Parlor> for Descent {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::space_bed(ctx);
        let p = Progress::load(ctx);
        if self.mode != Mode::Practice {
            self.flight.set_call_every(p.call_every());
        }
        let intro = self.intro(ctx);
        ctx.say(intro);
        let teach = !menus::tutorial_read(ctx, ID)
            && (!p.any() || matches!(self.mode, Mode::Practice | Mode::Moon(1)));
        if teach {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        self.save_practice(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.flight.finished() {
            return;
        }
        // The pause menu's Settings may have changed the units.
        if self.flight.feet != units::us() {
            self.flight.set_feet(units::us());
        }
        sound::space_bed(ctx);
        let f = &self.flight;
        let line = if f.started() {
            format!(
                "Back in flight at {}, {}.",
                units::length(f.altitude),
                self.velocity_words()
            )
        } else {
            format!("Back on the lander. {}", self.start_prompt(ctx))
        };
        ctx.say(line);
        // A key held into the pause does not fire the engine until let go.
        self.await_release = true;
        self.sync_loops(ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        let escape = action.is_none() && event.pressed() == Some(Key::Escape);
        if self.ending.is_some() && (action.is_some() || escape) {
            // Down already: every key hears how it ended.
            let line = self.down_words(self.flight.outcome.expect("down"));
            ctx.say(line);
            return;
        }
        if escape {
            self.stop_loops(ctx);
            self.save_practice(ctx);
            menus::pause(ctx, HELP, self.leave());
            return;
        }
        match action {
            Some(ParlorAction::Primary) => {
                if !self.flight.started() {
                    self.start(ctx);
                } else if !self.flight.unlimited && self.flight.fuel <= 0.0 {
                    ctx.play_ui(UiSound::Error);
                    ctx.say("Fuel is out.");
                }
                // While flying, the engine follows the held key in update.
            }
            Some(ParlorAction::Left | ParlorAction::Right) => {
                ctx.play_ui(UiSound::Error);
                ctx.say("No side thrusters. The pad is straight below.");
            }
            Some(ParlorAction::Up) => self.change_interval(ctx, false),
            Some(ParlorAction::Down) => self.change_interval(ctx, true),
            Some(ParlorAction::Secondary) => self.call_now(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status(ctx);
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => self.repeat(ctx),
            Some(ParlorAction::Hint) => self.hint(ctx),
            None => {
                if let InputEvent::KeyDown { key, .. } = event {
                    if !is_quiet_key(*key) {
                        ctx.play_ui(UiSound::Error);
                        ctx.say("Not a lander key. F1 lists the controls.");
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.ending = None;
                self.after_touchdown(ctx);
            }
            return;
        }
        let held = ctx.action_held(ParlorAction::Primary);
        if !held {
            self.await_release = false;
        }
        let evs = self.flight.update(dt, held && !self.await_release);
        self.voice(ctx, evs);
        if self.ending.is_none() {
            self.sync_loops(ctx);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let f = &self.flight;
        vec![
            self.where_words(),
            format!(
                "Altitude: {:.1} {}",
                units::from_metres(f.altitude),
                units::plural()
            ),
            format!("Speed: {}", self.velocity_words()),
            format!("Fuel: {}", self.fuel_words()),
            format!("Engine: {}", if f.engine { "on" } else { "off" }),
            format!(
                "Call-outs every {}",
                units::count(shown_call_every(f.call_every, f.feet).into())
            ),
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let now = if !self.flight.started() {
            self.start_prompt(ctx)
        } else if self.hints_allowed() {
            self.hint_words(ctx)
        } else {
            "Hold the engine when the tone rises, and let go when it drops.".to_string()
        };
        let mut text = format!("{now} {}", self.controls(ctx));
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

mod visuals_descent;
