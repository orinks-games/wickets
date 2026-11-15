//! Tuning a rank of the Echo's pipes: the play screen.

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, ScoreResult};

use crate::challenge::{self, Plan, CHALLENGES_NAME};
use crate::progress::{self, Progress};
use crate::rules::{
    note_name, pipes_words, points_words, run_seed, steps_words, Ev, Layout, Rank, PRACTICE, RANK,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "the_echo_play";

/// Seconds the true note plays alone when asked for.
pub const ALONE: f64 = 2.5;
/// Seconds between the notes of the Echo's tune.
pub const TUNE_GAP: f64 = 0.45;
/// Seconds from the end of the tune to the results.
pub const END_PAUSE: f64 = 2.5;

/// What kind of rank the screen is tuning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The evening's tuning, scored.
    Rank,
    /// The practice rank: hints are free, and nothing is scored.
    Practice,
    /// A challenge rank.
    Challenge(Period),
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Rank => "Evening tuning",
            Mode::Practice => "Practice rank",
            Mode::Challenge(period) => challenge::name(period),
        }
    }
}

pub struct Tuning {
    pub rank: Rank,
    pub mode: Mode,
    challenge: Option<Challenge>,
    /// The pipe's held tone and its true note's, while tuning.
    pipe_tone: Sound,
    true_tone: Sound,
    /// Seconds left of the true note playing alone.
    alone: Option<f64>,
    /// The Echo's tune still to play: seconds to wait, pipe index.
    tune: Vec<(f64, usize)>,
    /// Seconds until the results, once the tune has played.
    ending: Option<f64>,
    /// In the tutorial: a tuned rank reads no results.
    in_lesson: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl Tuning {
    pub fn new(mode: Mode) -> Tuning {
        Tuning::with_seed(mode, run_seed())
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Tuning {
        let layout = match mode {
            Mode::Practice => PRACTICE,
            _ => RANK,
        };
        Tuning::on(mode, Rank::new(layout, seed))
    }

    fn on(mode: Mode, rank: Rank) -> Tuning {
        Tuning {
            rank,
            mode,
            challenge: None,
            pipe_tone: Sound::NONE,
            true_tone: Sound::NONE,
            alone: None,
            tune: Vec::new(),
            ending: None,
            in_lesson: false,
            status_key: Default::default(),
        }
    }

    /// A rank of the challenge's.
    pub fn challenge(ch: Challenge) -> Tuning {
        let plan = Plan::for_challenge(&ch);
        let mut t = Tuning::on(Mode::Challenge(ch.period), plan.rank());
        t.challenge = Some(ch);
        t
    }

    /// The tutorial's rank: these pipes, and no results at the end.
    pub fn lesson_rank(layout: Layout, pipes: &[(i32, i32)]) -> Tuning {
        let mut t = Tuning::on(Mode::Practice, Rank::fixed(layout, pipes));
        t.in_lesson = true;
        t
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// Every control, in a few sentences.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "{} and {} raise and lower the pipe. {} sets it once the wobble is still. {} and {} move down and up the rank. {} plays the true note alone, {} speaks the status, {} names the pipe again, {} says which way it is out, and Escape pauses.",
            sentence_start(&k(ParlorAction::Up)),
            k(ParlorAction::Down),
            sentence_start(&k(ParlorAction::Primary)),
            sentence_start(&k(ParlorAction::Left)),
            k(ParlorAction::Right),
            sentence_start(&k(ParlorAction::Secondary)),
            k(ParlorAction::Status),
            k(ParlorAction::Repeat),
            k(ParlorAction::Hint)
        )
    }

    fn intro(&self) -> String {
        let r = &self.rank;
        let hints = if r.layout.free_hints {
            " Hints are free here."
        } else {
            ""
        };
        format!(
            "{}. A rank of {}, low to high. The slower the wobble, the nearer the pipe is to its note.{hints}",
            self.mode.name(),
            pipes_words(r.pipes.len())
        )
    }

    /// The pipe now being tuned, in words.
    pub fn pipe_words(&self) -> String {
        let r = &self.rank;
        let p = r.pipe();
        let state = if p.tuned { ", in tune" } else { "" };
        format!(
            "Pipe {} of {}, {}{state}.",
            r.cur + 1,
            r.pipes.len(),
            note_name(p.note)
        )
    }

    /// The status key, in layers. First: the pipe, whether it wobbles,
    /// and pipes in tune. A second press: the rank and the score.
    pub fn status_layers(&self) -> Vec<String> {
        let r = &self.rank;
        let p = r.pipe();
        let state = if p.tuned { "in tune" } else { "wobbling" };
        let first = format!(
            "Pipe {} of {}, {state}, {} of {} in tune.",
            r.cur + 1,
            r.pipes.len(),
            r.tuned(),
            r.pipes.len()
        );
        let more = if self.mode == Mode::Practice {
            format!("{}, no score kept.", self.mode.name())
        } else {
            format!("{}, score {}.", self.mode.name(), r.score)
        };
        vec![first, more]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }

    /// The rank is tuned and the Echo has played it back, with a moment
    /// after the last note.
    pub fn played_back(&self) -> bool {
        self.rank.finished()
            && self.tune.is_empty()
            && self.ending.is_none_or(|left| left <= END_PAUSE - 1.0)
    }

    /// Starts or retunes the two held tones for the pipe now chosen. A
    /// pipe in tune holds one tone alone.
    fn sound_pipe(&mut self, ctx: &mut Ctx<Parlor>) {
        let p = *self.rank.pipe();
        if self.pipe_tone == Sound::NONE {
            self.pipe_tone = sound::hold(ctx, p.pitch());
        } else {
            ctx.audio
                .set_pitch_over(self.pipe_tone, p.pitch(), sound::GLIDE_MS);
        }
        if self.true_tone == Sound::NONE {
            self.true_tone = sound::hold(ctx, p.true_pitch());
        } else {
            ctx.audio
                .set_pitch_over(self.true_tone, p.true_pitch(), sound::GLIDE_MS);
        }
        let pipe_volume = if self.alone.is_some() {
            0.0
        } else {
            sound::TONE_VOLUME
        };
        ctx.audio.set_volume(self.pipe_tone, pipe_volume);
        let true_volume = if p.tuned && self.alone.is_none() {
            0.0
        } else {
            sound::TONE_VOLUME
        };
        ctx.audio.set_volume(self.true_tone, true_volume);
    }

    fn stop_tones(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in [self.pipe_tone, self.true_tone] {
            if s != Sound::NONE {
                ctx.audio.stop(s, 150);
            }
        }
        self.pipe_tone = Sound::NONE;
        self.true_tone = Sound::NONE;
        self.alone = None;
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        for ev in evs {
            match ev {
                Ev::Moved { .. } => {
                    sound::play(ctx, sound::SLIDE);
                    self.sound_pipe(ctx);
                }
                Ev::Limit => ctx.say("The pipe will go no further that way."),
                Ev::Chose { .. } => {
                    self.sound_pipe(ctx);
                    let words = self.pipe_words();
                    ctx.say_queued(words);
                }
                Ev::End { first } => {
                    ctx.say(if first {
                        "This is the lowest pipe."
                    } else {
                        "This is the highest pipe."
                    });
                }
                Ev::InTune { points, .. } => {
                    sound::play(ctx, sound::IN_TUNE);
                    if self.mode == Mode::Practice {
                        ctx.say("In tune.");
                    } else {
                        ctx.say(format!(
                            "In tune. {}.",
                            sentence_start(&points_words(points))
                        ));
                    }
                }
                Ev::NotYet { near } => {
                    sound::play(ctx, sound::NOT_YET);
                    let line = if near {
                        "Nearly. The wobble is slow, but it is still there."
                    } else {
                        "Not yet. The pipe still wobbles."
                    };
                    let cost = if self.rank.layout.free_hints {
                        String::new()
                    } else {
                        format!(" That try cost {}.", points_words(crate::rules::TRY_COST))
                    };
                    ctx.say(format!("{line}{cost}"));
                }
                Ev::AlreadyTuned => {
                    let (l, r) = (
                        Self::key(ctx, ParlorAction::Left),
                        Self::key(ctx, ParlorAction::Right),
                    );
                    ctx.say(format!(
                        "This pipe is in tune. {} and {r} move to another.",
                        sentence_start(&l)
                    ));
                }
                Ev::Hint { sharp, steps } => {
                    sound::play(ctx, sound::HINT);
                    let way = if sharp { "Sharp" } else { "Flat" };
                    let fix = if sharp { "lower" } else { "raise" };
                    let cost = if self.rank.layout.free_hints {
                        String::new()
                    } else {
                        format!(" That hint cost {}.", points_words(crate::rules::HINT_COST))
                    };
                    ctx.say(format!(
                        "{way}, {} out. {} it.{cost}",
                        steps_words(steps),
                        sentence_start(fix)
                    ));
                }
                Ev::HintInTune => {
                    sound::play(ctx, sound::HINT);
                    let k = sentence_start(&Self::key(ctx, ParlorAction::Primary));
                    ctx.say(format!("In tune now. {k} sets it."));
                }
                Ev::RankTuned => {
                    self.stop_tones(ctx);
                    ctx.say_queued("Every pipe is in tune. The Echo is playing the rank back.");
                    self.tune = self
                        .rank
                        .tune
                        .iter()
                        .enumerate()
                        .map(|(i, &pipe)| (1.0 + i as f64 * TUNE_GAP, pipe))
                        .collect();
                }
            }
        }
    }

    fn play_tune(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut due = Vec::new();
        self.tune.retain_mut(|(wait, pipe)| {
            *wait -= dt;
            if *wait <= 1e-9 {
                due.push(*pipe);
                false
            } else {
                true
            }
        });
        for pipe in due {
            let pitch = self.rank.pipes[pipe].true_pitch();
            sound::play_pitched(ctx, sound::NOTE, pitch);
        }
        if self.tune.is_empty() {
            sound::play(ctx, sound::RANK_TUNED);
            self.ending = Some(END_PAUSE);
        }
    }

    /// The rank is tuned: record it and read the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.rank);
            let summary = challenge::summary(&ch, &self.rank);
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let (summary, pages) = if self.mode == Mode::Practice {
            progress::save_other(ctx, &self.rank, true);
            (
                "Practice rank tuned.".to_string(),
                vec![format!(
                    "Practice rank tuned. {} in tune.",
                    sentence_start(&pipes_words(self.rank.tuned()))
                )],
            )
        } else {
            let score = progress::save_rank(ctx, &self.rank);
            (result_summary(&self.rank), result_pages(&self.rank, score))
        };
        let mut results = Some(after_rank(self.mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// "Rank tuned: 6 pipes, 4 at the first try."
pub fn tuned_line(rank: &Rank) -> String {
    let perfect = match rank.perfect() {
        n if n > 0 && n == rank.pipes.len() => format!("all {n} at the first try without a hint"),
        0 => "none at the first try without a hint".to_string(),
        n => format!("{n} at the first try without a hint"),
    };
    format!("Rank tuned: {}, {perfect}.", pipes_words(rank.pipes.len()))
}

/// The one-line headline of an evening's tuning, the results menu's title.
pub fn result_summary(rank: &Rank) -> String {
    format!("Rank tuned. {}", sentence_start(&points_words(rank.score)))
}

/// The pages after an evening's tuning.
pub fn result_pages(rank: &Rank, saved: ScoreResult) -> Vec<String> {
    let score = match saved {
        ScoreResult::First => format!("{}, your first score.", points_words(rank.score)),
        ScoreResult::NewBest { previous } => format!(
            "{}, a new best, beating {previous}.",
            points_words(rank.score)
        ),
        ScoreResult::Short { best } => {
            format!("{}. Your best is {best}.", points_words(rank.score))
        }
    };
    vec![tuned_line(rank), sentence_start(&score)]
}

/// The results menu after an evening's tuning or a practice rank.
pub fn after_rank(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let help = match mode {
        Mode::Practice => "Another practice rank, with new notes.",
        _ => "Another rank to tune, with new notes.",
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        help,
        move |ctx| ctx.replace(Tuning::new(mode)),
    )
}

/// The results menu after a challenge rank.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same rank, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Tuning::challenge(ch))
        },
    )
}

impl State<Parlor> for Tuning {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::cellar_bed(ctx);
        self.sound_pipe(ctx);
        let intro = self.intro();
        ctx.say(intro);
        let pipe = self.pipe_words();
        ctx.say_queued(pipe);
        let new_here = !Progress::load(ctx).any();
        if new_here {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_tones(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.rank.finished() {
            return;
        }
        sound::cellar_bed(ctx);
        self.sound_pipe(ctx);
        let pipe = self.pipe_words();
        ctx.say(format!("Back in the cellar. {pipe}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        if self.rank.finished() {
            // The results follow the playback; say so rather than nothing.
            if !self.played_back() && matches!(event.pressed(), Some(Key::Escape | Key::Return)) {
                ctx.say("The Echo is playing the rank back.");
            }
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            self.stop_tones(ctx);
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let evs = match action {
            Some(ParlorAction::Up) => self.rank.nudge(true),
            Some(ParlorAction::Down) => self.rank.nudge(false),
            Some(ParlorAction::Left) => self.rank.choose(false),
            Some(ParlorAction::Right) => self.rank.choose(true),
            Some(ParlorAction::Primary) => self.rank.set(),
            Some(ParlorAction::Hint) => self.rank.hint(),
            Some(ParlorAction::Secondary) => {
                self.alone = Some(ALONE);
                self.sound_pipe(ctx);
                ctx.say("The true note alone.");
                return;
            }
            Some(ParlorAction::Status) => {
                let layers = self.status_layers();
                self.status_key.speak(ctx, layers);
                return;
            }
            Some(ParlorAction::Repeat) => {
                let p = self.pipe_words();
                ctx.say(p);
                return;
            }
            None => return,
        };
        self.voice(ctx, evs);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.ending = None;
                if !self.in_lesson {
                    self.finish(ctx);
                }
            }
            return;
        }
        if !self.tune.is_empty() {
            self.play_tune(ctx, dt);
            return;
        }
        if let Some(left) = self.alone.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.alone = None;
                self.sound_pipe(ctx);
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let r = &self.rank;
        let mut out = vec![
            self.mode.name().to_string(),
            format!("Pipe: {} of {}", r.cur + 1, r.pipes.len()),
            format!("In tune: {} of {}", r.tuned(), r.pipes.len()),
        ];
        if self.mode != Mode::Practice {
            out.push(format!("Score: {}", r.score));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut text = format!("{} {}", self.pipe_words(), self.controls(ctx));
        if let Some(ch) = &self.challenge {
            text.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count the rank.",
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

mod visuals_tuning;
