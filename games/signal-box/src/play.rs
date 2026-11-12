//! Working the signal box: the play screen.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, ScoreResult};

use crate::challenge::{self, Plan, CHALLENGES_NAME};
use crate::events::{self, THIRTEENTH};
use crate::progress::{self, Progress, Saved};
use crate::rules::{
    answer_words, derailed_words, points_words, run_seed, trains_words, window_words, Call, Ev,
    Phase, Shift,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "signal_box_play";

/// Seconds between the earcons of the before-shift hint.
pub const HINT_GAP: f64 = 1.6;
/// Seconds from the end of a run to its results, so the brake screech is
/// heard and a key still held from play does not skip the first page.
pub const END_PAUSE: f64 = 1.5;

/// What kind of run the screen is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The endless night shift.
    Shift,
    /// The practice room: no clock, nothing to lose.
    Practice,
    /// A challenge timetable.
    Challenge(Period),
    /// A steam gala night shift, with the steam special.
    SteamGala,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Shift => "Night shift",
            Mode::Practice => "Practice room",
            Mode::Challenge(period) => challenge::name(period),
            Mode::SteamGala => events::GALA_NAME,
        }
    }
}

/// How long a passing train's event sound fades out. The fade starts
/// this long and a frame before the next call is due, so it has ended
/// before the call's earcon starts.
const PASS_FADE: u32 = 120;
/// The margin before the next call at which the fade starts, in seconds:
/// the fade and one long frame.
const PASS_FADE_LEAD: f64 = PASS_FADE as f64 / 1000.0 + 0.05;

pub struct Signal {
    pub shift: Shift,
    pub mode: Mode,
    challenge: Option<Challenge>,
    /// Cues already explained in words this run.
    taught: HashSet<&'static str>,
    /// The before-shift hint's earcons still to play: seconds to wait, call.
    delayed: Vec<(f64, Call)>,
    /// Seconds until the results, once the run is over.
    ending: Option<f64>,
    /// Practice calls answered and not yet saved.
    practice_unsaved: u32,
    /// The tutorial's practice line: its trains are not practice calls.
    pub lesson: bool,
    /// Friday the 13th on the night shift: the thirteenth train runs.
    pub thirteenth: bool,
    /// The thirteenth train went by this run.
    pub ghost_seen: bool,
    /// The ghost train or the special passing now, cut as the next call comes.
    passing: Sound,
}

impl Signal {
    pub fn new(mode: Mode) -> Signal {
        Signal::with_seed(mode, run_seed())
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Signal {
        let shift = match mode {
            Mode::Practice => Shift::practice(seed),
            Mode::SteamGala => Shift::steam_gala(seed),
            _ => Shift::endless(seed),
        };
        Signal {
            shift,
            mode,
            challenge: None,
            taught: HashSet::new(),
            delayed: Vec::new(),
            ending: None,
            practice_unsaved: 0,
            lesson: false,
            thirteenth: false,
            ghost_seen: false,
            passing: Sound::NONE,
        }
    }

    /// The event on this run, in a sentence, for S, H and F1.
    fn event_words(&self) -> Option<&'static str> {
        if self.mode == Mode::SteamGala {
            Some(events::SPECIAL_WORDS)
        } else if self.thirteenth {
            Some(events::THIRTEENTH_WORDS)
        } else {
            None
        }
    }

    /// `text`, then the event's sentence when one is on.
    fn with_event(&self, text: String) -> String {
        match self.event_words() {
            Some(words) => format!("{text} {words}"),
            None => text,
        }
    }

    /// Fades out the event sound of a train passing.
    fn stop_passing(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_passing_over(ctx, PASS_FADE);
    }

    fn stop_passing_over(&mut self, ctx: &mut Ctx<Parlor>, fade_ms: u32) {
        if self.passing != Sound::NONE {
            ctx.audio.stop(self.passing, fade_ms);
            self.passing = Sound::NONE;
        }
    }

    /// Fades the passing train's event sound out before the next call is
    /// due, so the fade is over by the time the call's earcon starts.
    fn fade_before_call(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Phase::Passing { left } = self.shift.phase {
            if left - dt <= PASS_FADE_LEAD {
                self.stop_passing(ctx);
            }
        }
    }

    /// A run of the challenge's timetable.
    pub fn challenge(ch: Challenge) -> Signal {
        let plan = Plan::for_challenge(&ch);
        let mut s = Signal::with_seed(Mode::Challenge(ch.period), 0);
        s.shift = plan.shift();
        s.challenge = Some(ch);
        s
    }

    /// True the first time a cue is met this run.
    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// What to do before the line opens.
    fn open_prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} opens the line. {} plays every call with its key.",
            sentence_start(&Self::key(ctx, ParlorAction::Primary)),
            sentence_start(&Self::key(ctx, ParlorAction::Hint))
        )
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let what = match self.mode {
            Mode::Shift | Mode::SteamGala => "Calls come until a train derails.".to_string(),
            Mode::Practice => "Calls with no clock and nothing to lose.".to_string(),
            Mode::Challenge(_) => format!(
                "A timetable of {}.",
                trains_words(self.shift.trains().unwrap_or(0))
            ),
        };
        format!("{}. {what} {}", self.mode.name(), self.open_prompt(ctx))
    }

    /// Every control, in a few sentences.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "{} throws the points left and {} the points right. {} raises the signal and {} lowers it. {} rings the bell back. {} speaks the status, {} repeats the call, and Escape pauses.",
            sentence_start(&k(ParlorAction::Left)),
            k(ParlorAction::Right),
            sentence_start(&k(ParlorAction::Up)),
            k(ParlorAction::Down),
            sentence_start(&k(ParlorAction::Primary)),
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Repeat)
        )
    }

    /// The status line, in a fixed order: the run, trains passed, the
    /// window, the score.
    pub fn status(&self, ctx: &Ctx<Parlor>) -> String {
        let s = &self.shift;
        let name = self.mode.name();
        if !s.started() {
            return self.with_event(format!(
                "{name}, line not open, window {}. {}",
                window_words(s.window),
                self.open_prompt(ctx)
            ));
        }
        if !s.timed() {
            let n = s.passed;
            let calls = if n == 1 {
                "1 call answered".to_string()
            } else {
                format!("{n} calls answered")
            };
            return format!("{name}, {calls}, no clock.");
        }
        let passed = match s.trains() {
            Some(total) => format!("{} of {total} trains passed", s.passed),
            None => format!("{} passed", trains_words(s.passed)),
        };
        self.with_event(format!(
            "{name}, {passed}, window {}, score {}.",
            window_words(s.window),
            s.score
        ))
    }

    fn say_call(&mut self, ctx: &mut Ctx<Parlor>, call: Call) {
        sound::play_call(ctx, call);
        ctx.say(format!("{}.", call.word()));
    }

    /// Before the line opens, an answer key plays its call and names it.
    fn preview(&mut self, ctx: &mut Ctx<Parlor>, call: Call) {
        sound::play_call(ctx, call);
        let key = Self::key(ctx, call.action());
        ctx.say(format!("{}. {}", call.word(), answer_words(&key, call)));
    }

    /// The before-shift hint: every call with its key, each earcon in turn.
    fn hint_all(&mut self, ctx: &mut Ctx<Parlor>) {
        let lines: Vec<String> = Call::ALL
            .into_iter()
            .map(|c| format!("{}, {}.", c.word(), Self::key(ctx, c.action())))
            .collect();
        ctx.say(self.with_event(lines.join(" ")));
        self.delayed = Call::ALL
            .into_iter()
            .enumerate()
            .map(|(i, c)| (i as f64 * HINT_GAP, c))
            .collect();
        self.play_due(ctx, 0.0);
    }

    fn play_due(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut due = Vec::new();
        self.delayed.retain_mut(|(wait, call)| {
            *wait -= dt;
            if *wait <= 1e-9 {
                due.push(*call);
                false
            } else {
                true
            }
        });
        for call in due {
            sound::play_call(ctx, call);
        }
    }

    fn open_line(&mut self, ctx: &mut Ctx<Parlor>) {
        self.delayed.clear();
        self.shift.start();
        if self.shift.timed() {
            ctx.say("Line open. The tick keeps the pace.");
        } else {
            ctx.say("Line open. Take your time.");
        }
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        for ev in evs {
            match ev {
                Ev::Call(call) => {
                    // Already faded in the gap; on a long frame, cut it.
                    self.stop_passing_over(ctx, 0);
                    self.say_call(ctx, call)
                }
                Ev::Tick => {
                    sound::play_vol(ctx, sound::TICK, sound::TICK_VOLUME);
                }
                Ev::Passed { train, points } => {
                    if self.thirteenth && train == THIRTEENTH {
                        self.stop_passing(ctx);
                        self.passing = sound::play(ctx, sound::GHOST_TRAIN);
                        self.ghost_seen = true;
                        progress::save_thirteenth(ctx);
                    } else if points > 1 {
                        self.stop_passing(ctx);
                        self.passing = sound::play(ctx, sound::SPECIAL);
                    } else {
                        sound::play(ctx, sound::TRAIN_PASS);
                    }
                    if !self.shift.timed() {
                        self.practice_unsaved += 1;
                    }
                    if self.first("pass") {
                        ctx.say("Train through.");
                    }
                }
                Ev::Streak { trains, bonus } => {
                    sound::play(ctx, sound::STREAK);
                    ctx.say(format!(
                        "{} in a row. Bonus {}.",
                        sentence_start(&trains_words(trains)),
                        bonus
                    ));
                }
                Ev::Faster { .. } => {
                    if self.first("faster") {
                        ctx.say_queued("Faster.");
                    }
                }
                Ev::TopSpeed => ctx.say_queued("Top speed."),
                Ev::NoCall => ctx.say("No call yet."),
                Ev::Wrong { call, got } => {
                    ctx.play_ui(UiSound::Error);
                    let key = Self::key(ctx, call.action());
                    ctx.say(format!(
                        "Not {}. {}: {}",
                        got.word().to_lowercase(),
                        call.word(),
                        answer_words(&key, call)
                    ));
                }
                Ev::Derailed(miss) => {
                    sound::play(ctx, sound::BRAKE);
                    ctx.say(format!(
                        "{} {}",
                        derailed_words(self.shift.passed),
                        miss.words()
                    ));
                    self.ending = Some(END_PAUSE);
                }
                Ev::Cleared => {
                    sound::play(ctx, sound::STREAK);
                    ctx.say(format!(
                        "Timetable cleared. All {} through.",
                        trains_words(self.shift.passed)
                    ));
                    self.ending = Some(END_PAUSE);
                }
            }
        }
    }

    fn answer(&mut self, ctx: &mut Ctx<Parlor>, call: Call) {
        if !self.shift.started() {
            self.preview(ctx, call);
            return;
        }
        let evs = self.shift.answer(call);
        self.voice(ctx, evs);
    }

    fn repeat(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.shift.started() {
            let p = self.open_prompt(ctx);
            ctx.say(p);
        } else if let Some(call) = self.shift.current() {
            self.say_call(ctx, call);
        } else {
            ctx.say("No call yet. Wait for the next one.");
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.shift.started() {
            self.hint_all(ctx);
        } else if let (false, Some(call)) = (self.shift.timed(), self.shift.current()) {
            let key = Self::key(ctx, call.action());
            ctx.say(format!("{}: {}", call.word(), answer_words(&key, call)));
        } else if !self.shift.timed() {
            ctx.say("No call yet. Wait for the next one.");
        } else {
            let text = self.with_event("No hints once the line is open.".to_string());
            ctx.say(text);
        }
    }

    fn save_practice(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.lesson {
            self.practice_unsaved = 0;
            return;
        }
        progress::save_practice(ctx, self.practice_unsaved);
        self.practice_unsaved = 0;
    }

    /// The run is over: record it and read the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.shift);
            let summary = challenge::summary(&ch, &self.shift);
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let pages = if self.mode == Mode::SteamGala {
            let score = progress::save_gala(ctx, &self.shift);
            gala_pages(&self.shift, score)
        } else {
            let saved = progress::save_shift(ctx, &self.shift);
            let mut pages = result_pages(&self.shift, &saved);
            if self.ghost_seen {
                if let Some(last) = pages.last_mut() {
                    last.push_str(" The thirteenth train went by.");
                }
            }
            pages
        };
        let summary = result_summary(&self.shift);
        let mut results = Some(after_shift(self.mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// The one-line headline of a night shift, the results menu's title.
pub fn result_summary(shift: &Shift) -> String {
    format!(
        "{} {}",
        derailed_words(shift.passed),
        sentence_start(&points_words(shift.score))
    )
}

/// The pages after a night shift.
pub fn result_pages(shift: &Shift, saved: &Saved) -> Vec<String> {
    let score = match saved.score {
        ScoreResult::First => format!("{}, your first score.", points_words(shift.score)),
        ScoreResult::NewBest { previous } => {
            format!(
                "{}, a new best, beating {previous}.",
                points_words(shift.score)
            )
        }
        ScoreResult::Short { best } => {
            format!("{}. Your best is {best}.", points_words(shift.score))
        }
    };
    // The derailment was said as it happened: the pages lead with the
    // score.
    let first = sentence_start(&score);
    let run = match saved.run_record {
        Some(0) => format!(
            "Longest run: {}, your first on the board.",
            trains_words(shift.passed)
        ),
        Some(previous) => format!(
            "Longest run: {}, a new record, beating {previous}.",
            trains_words(shift.passed)
        ),
        None => format!("Longest run: {}.", trains_words(saved.best_run)),
    };
    let window = if saved.window_record {
        format!(
            "Window reached: {}, your shortest yet.",
            window_words(shift.window)
        )
    } else {
        format!("Window reached: {}.", window_words(shift.window))
    };
    vec![first, format!("{run} {window}")]
}

/// The pages after a steam gala shift: its score on its own board, and the
/// specials through.
pub fn gala_pages(shift: &Shift, saved: ScoreResult) -> Vec<String> {
    let score = match saved {
        ScoreResult::First => format!(
            "{}, your first steam gala score.",
            points_words(shift.score)
        ),
        ScoreResult::NewBest { previous } => format!(
            "{}, a new steam gala best, beating {previous}.",
            points_words(shift.score)
        ),
        ScoreResult::Short { best } => format!(
            "{}. Your steam gala best is {best}.",
            points_words(shift.score)
        ),
    };
    let specials = match shift.specials {
        1 => "1 steam special".to_string(),
        n => format!("{n} steam specials"),
    };
    vec![
        sentence_start(&score),
        format!("{} through.", sentence_start(&specials)),
    ]
}

/// The results menu after a night shift or a steam gala shift.
pub fn after_shift(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let help = match mode {
        Mode::SteamGala => "Another steam gala shift, from the slow start.",
        _ => "Another night shift, from the slow start.",
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        help,
        move |ctx| {
            let today = parlor_core::challenges::today(ctx);
            if mode == Mode::SteamGala && !events::GALA.is_open(today) {
                ctx.say(events::gala_locked_line(today))
            } else {
                ctx.replace(Signal::new(mode))
            }
        },
    )
}

/// The results menu after a challenge timetable.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same timetable, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Signal::challenge(ch))
        },
    )
}

impl State<Parlor> for Signal {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::box_bed(ctx);
        let today = parlor_core::challenges::today(ctx);
        self.thirteenth = self.mode == Mode::Shift && events::is_thirteenth(today);
        let intro = self.intro(ctx);
        ctx.say(intro);
        if let Some(words) = self.event_words() {
            ctx.say_queued(words);
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
        self.stop_passing(ctx);
        self.save_practice(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.shift.finished() {
            return;
        }
        sound::box_bed(ctx);
        if let Some(call) = self.shift.recall() {
            ctx.say("Back in the box. The call again, with a fresh window.");
            sound::play_call(ctx, call);
            ctx.say_queued(format!("{}.", call.word()));
        } else if self.shift.started() {
            ctx.say("Back in the box. Wait for the next call.");
        } else {
            let p = self.open_prompt(ctx);
            ctx.say(format!("Back in the box. {p}"));
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.shift.finished() || event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            self.save_practice(ctx);
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        match action {
            Some(ParlorAction::Primary) if !self.shift.started() => self.open_line(ctx),
            Some(
                a @ (ParlorAction::Left
                | ParlorAction::Right
                | ParlorAction::Up
                | ParlorAction::Down
                | ParlorAction::Primary),
            ) => {
                if let Some(call) = Call::for_action(a) {
                    self.answer(ctx, call);
                }
            }
            Some(ParlorAction::Secondary) => {
                ctx.say("Nothing to hold. This line has no hold calls.");
            }
            Some(ParlorAction::Status) => {
                let s = self.status(ctx);
                ctx.say(s);
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
        self.play_due(ctx, dt);
        self.fade_before_call(ctx, dt);
        let evs = self.shift.update(dt);
        self.voice(ctx, evs);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let s = &self.shift;
        let mut out = vec![self.mode.name().to_string()];
        out.push(match s.trains() {
            Some(total) => format!("Trains passed: {} of {total}", s.passed),
            None => format!("Trains passed: {}", s.passed),
        });
        if s.timed() {
            out.push(format!("Window: {}", window_words(s.window)));
            out.push(format!("Score: {}", s.score));
        }
        if let Some(call) = s.current() {
            out.push(format!("Call: {}", call.word()));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let s = &self.shift;
        let now = if !s.started() {
            self.open_prompt(ctx)
        } else if let Some(call) = s.current() {
            let key = Self::key(ctx, call.action());
            if s.timed() {
                format!("The call is {}. Answer now.", call.word().to_lowercase())
            } else {
                format!("{}: {}", call.word(), answer_words(&key, call))
            }
        } else {
            "Wait for the next call.".to_string()
        };
        let mut text = self.with_event(format!("{now} {}", self.controls(ctx)));
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

mod visuals;
