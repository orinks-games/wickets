//! Cracking a safe by ear: the play screen.

use parlor_core::status::sentences;
use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::{Challenge, Mood, Pages, Parlor, ParlorAction, Period, Score, ScoreResult};

use crate::challenge::{self, Job, Plan};

use crate::heist::CHALLENGES_NAME;
use crate::progress::{self, Saved};
use crate::rules::{
    count_word, run_seed, sequence_words, time_words, Detent, Ev, GuardPhase, Mode, Outcome, Run,
    Turn, Wrong, SAFES, STRIKES, SUSPICION,
};
use crate::{safe_summary, sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const CRACK_NAME: &str = "safecracker_crack";

/// How long after a click its tumbler sound follows, so the two never blur.
const AFTER_CLICK: f64 = 0.08;
/// How long after a spin's ratchet its tumbler sound follows.
const AFTER_SPIN: f64 = 0.3;
/// False gates play this much quieter than the safe's true clunk, which
/// they are louder than at full volume, so a decoy never outshouts it.
pub const GATE_VOLUME: f32 = 0.7;
/// Building noises are this loud, under the dial.
const NOISE_VOLUME: f32 = 0.55;
/// How long after the clunk at 13 the unlucky safe's cat yowls, so the
/// two never blur. A turn before then hushes her, so she is never over a
/// click.
const AFTER_CLUNK: f64 = 0.35;
/// The cat's yowl is soft, under the clunk.
const YOWL_VOLUME: f32 = 0.5;
/// A yowl still sounding when the dial turns again fades out this fast,
/// before the click, so she is never over it.
const YOWL_FADE_MS: u32 = 30;

pub struct Crack {
    pub run: Run,
    last_prompt: String,
    steps: Sound,
    /// The cat's yowl, while it may still be sounding.
    yowl: Sound,
    /// Sounds waiting to play: seconds to wait, key, volume.
    delayed: Vec<(f64, &'static str, f32)>,
    /// Cues already explained in words this safe.
    taught: HashSet<&'static str>,
    started: bool,
    /// Practice wheels not yet saved.
    practice_unsaved: u32,
    /// A challenge job, safe after safe, and the challenge it counts for.
    job: Option<Job>,
    challenge: Option<Challenge>,
    /// The safe's music, following the run.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl Crack {
    pub fn new(mode: Mode) -> Crack {
        Crack::with_seed(mode, run_seed())
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Crack {
        Crack {
            status_key: Default::default(),
            run: Run::new(mode, seed),
            last_prompt: String::new(),
            steps: Sound::NONE,
            yowl: Sound::NONE,
            delayed: Vec::new(),
            taught: HashSet::new(),
            started: false,
            practice_unsaved: 0,
            job: None,
            challenge: None,
            score: Score::new(),
        }
    }

    /// What the score hears: each wheel found builds it; the last half
    /// minute on a clock hurries it; the guard on his way darkens it; and
    /// while he stands listening at the door it drops to the bare strings.
    pub fn mood(&self) -> Mood {
        let run = &self.run;
        let phase = run.guard.map(|g| g.phase);
        if phase == Some(GuardPhase::Near) {
            return Mood::new(0.0);
        }
        let found = run.wheel as f32 / run.spec.wheels.max(1) as f32;
        Mood::new(0.2 + 0.75 * found)
            .with_flag(HURRY, run.mode.timed() && run.time_left <= 30.0)
            .with_flag(DANGER, phase == Some(GuardPhase::Coming))
    }

    /// A run of the challenge's job, from its first safe.
    pub fn challenge(ch: Challenge) -> Crack {
        let job = Job::new(Plan::for_challenge(&ch));
        let mut crack = Crack::with_seed(job.plan.mode(), 0);
        crack.run = job.run();
        if let Some(clock) = job.plan.clock() {
            crack.run.carry_clock(clock);
        }
        crack.job = Some(job);
        crack.challenge = Some(ch);
        crack
    }

    /// True the first time a cue is met on this safe.
    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    /// Lessons in words belong to the early safes and practice.
    fn teaching(&self, upto: u32) -> bool {
        match self.run.mode {
            Mode::Practice => true,
            Mode::Safe(n) => n <= upto,
            Mode::TimeAttack(_) | Mode::Challenge { .. } | Mode::Unlucky | Mode::Blackout => false,
        }
    }

    fn stop_steps(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.steps != Sound::NONE {
            ctx.audio.stop(self.steps, 250);
            self.steps = Sound::NONE;
        }
    }

    fn start_steps(&mut self, ctx: &mut Ctx<Parlor>, volume: f32) {
        self.stop_steps(ctx);
        self.steps = ctx
            .audio
            .start_loop(sound::STEPS, Some(sound::DOOR), Bus::EFFECTS);
        ctx.audio.set_volume(self.steps, volume);
    }

    fn later(&mut self, wait: f64, key: &'static str, volume: f32) {
        self.delayed.push((wait, key, volume));
    }

    /// "Wheel 2 of 3: turn left and listen for the clunk."
    fn wheel_prompt(&self) -> String {
        let run = &self.run;
        if run.practice() {
            return "Turn the dial either way to the clunk, then set it.".to_string();
        }
        let way = match run.required() {
            Some(t) => format!("turn {} to the clunk", t.word()),
            None => "turn either way to the clunk".to_string(),
        };
        format!("Wheel {} of {}: {way}.", run.wheel + 1, run.spec.wheels)
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let spec = &self.run.spec;
        match self.run.mode {
            Mode::Practice => {
                let l = ctx.control_name(ParlorAction::Left);
                let r = ctx.control_name(ParlorAction::Right);
                let p = ctx.control_name(ParlorAction::Primary);
                format!(
                    "The practice safe. One wheel at a time, forever, and nothing to lose. {} and {r} turn the dial. At the clunk, {p} sets the wheel and a new number is waiting. Escape pauses, and you leave from there.",
                    sentence_start(&l)
                )
            }
            Mode::Safe(n) => {
                let rule = if spec.alternate {
                    format!(", found {}", sequence_words(spec))
                } else {
                    String::new()
                };
                let news = spec
                    .news
                    .replace("{status}", &ctx.control_name(ParlorAction::Status));
                format!(
                    "Safe {n} of {SAFES}: {}. {} wheels{rule}. {news}",
                    spec.name,
                    sentence_start(count_word(spec.wheels)),
                )
                .trim_end()
                .to_string()
            }
            Mode::TimeAttack(n) => format!(
                "Time attack, safe {n}: {}. {} on the clock. Hints cost five seconds.{}",
                spec.name,
                sentence_start(&time_words(spec.limit)),
                if spec.guard.is_some() {
                    " The clock stops while the guard listens at the door."
                } else {
                    ""
                }
            ),
            Mode::Challenge { period, .. } => {
                let Some(job) = &self.job else {
                    return String::new();
                };
                self.safe_intro(
                    job,
                    &format!("{}: {}.", period.label(), challenge::name(period)),
                )
            }
            Mode::Unlucky | Mode::Blackout => format!(
                "{}, a bonus safe. {} wheels.",
                sentence_start(spec.name),
                sentence_start(count_word(spec.wheels)),
            ),
        }
    }

    /// What is special about an event safe, queued after its intro.
    fn event_words(&self, ctx: &Ctx<Parlor>) -> Option<String> {
        self.run.mode.is_event().then(|| {
            self.run
                .spec
                .news
                .replace("{secondary}", &ctx.control_name(ParlorAction::Secondary))
        })
    }

    /// The bed under the dial: rain in the blackout, the room hum otherwise.
    fn bed(&self, ctx: &mut Ctx<Parlor>) {
        if self.run.mode == Mode::Blackout {
            ctx.audio.play_music(None, false);
            ctx.audio.set_bed(Some(sound::BLACKOUT_BED));
        } else {
            sound::room_bed(ctx);
        }
    }

    /// Hushes the cat as the dial turns again: a yowl still waiting never
    /// plays, and one still sounding fades out before the click.
    fn hush_yowl(&mut self, ctx: &mut Ctx<Parlor>) {
        self.delayed.retain(|(_, key, _)| *key != sound::YOWL);
        if self.yowl != Sound::NONE {
            ctx.audio.stop(self.yowl, YOWL_FADE_MS);
            self.yowl = Sound::NONE;
        }
    }

    /// A challenge safe's opening words: which safe, its wheels, and what
    /// the job shares across safes.
    fn safe_intro(&self, job: &Job, head: &str) -> String {
        let run = &self.run;
        let spec = &run.spec;
        let mut out = head.to_string();
        if job.safes() > 1 {
            out.push_str(&format!(" Safe {} of {}:", job.index + 1, job.safes()));
        }
        // After "Safe 1 of 2:" the name runs on; alone, it starts a sentence.
        let name = if job.safes() > 1 {
            spec.name.to_string()
        } else {
            sentence_start(spec.name)
        };
        out.push_str(&format!(
            " {}. {} wheels",
            name,
            sentence_start(count_word(spec.wheels))
        ));
        if spec.alternate {
            out.push_str(&format!(", found {}", sequence_words(spec)));
        }
        if spec.decoys > 0 {
            out.push_str(&format!(
                ", {} false gate{} a wheel",
                spec.decoys,
                if spec.decoys == 1 { "" } else { "s" }
            ));
        }
        out.push('.');
        if spec.guard.is_some() {
            out.push_str(if spec.call_guard {
                " A guard walks the corridor, and you are told when he comes."
            } else {
                " A guard walks the corridor, unannounced: listen for his footsteps."
            });
        }
        if let Some(t) = job.plan.twist {
            out.push_str(&format!(" Twist: {}.", t.name()));
        }
        if job.plan.pooled() {
            out.push_str(&format!(
                " Vault strikes {} of {STRIKES}, {} left.",
                run.strikes,
                hints_words(run.hints_left)
            ));
        }
        if run.mode.timed() {
            out.push_str(&format!(
                " {} left on the clock.",
                sentence_start(&time_words(run.time_left))
            ));
        }
        out
    }

    fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let l = ctx.control_name(ParlorAction::Left);
        let r = ctx.control_name(ParlorAction::Right);
        let u = ctx.control_name(ParlorAction::Up);
        let d = ctx.control_name(ParlorAction::Down);
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let rep = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        format!(
            "{} and {r} turn the dial one number down or up. {} spins it ten numbers up and {d} ten down. {} sets the wheel. {} says the number, {s} the dial, wheel and guard, twice for strikes, hints and time, {rep} repeats, {h} gives a hint. F1 helps, Escape pauses.",
            sentence_start(&l),
            sentence_start(&u),
            sentence_start(&p),
            sentence_start(&t)
        )
    }

    fn guard_words(&self) -> &'static str {
        match self.run.guard_phase() {
            None => "",
            Some(GuardPhase::Away) => "The corridor is quiet.",
            Some(GuardPhase::Coming) => "Footsteps coming. When they stop, keep still.",
            Some(GuardPhase::Near) => "The guard is at the door: keep still!",
            Some(GuardPhase::Leaving) => "The guard is walking away: carry on.",
        }
    }

    /// The S key, in layers. First: the guard if he is coming or here,
    /// then the dial and the wheel. A second press: strikes, hints, the
    /// clock and the job.
    pub fn status(&self) -> Vec<String> {
        let run = &self.run;
        if run.practice() {
            return vec![format!(
                "Dial at {}. {} set this session.",
                run.dial, run.practice_sets
            )];
        }
        let guard = self.guard_words();
        let urgent = matches!(
            run.guard_phase(),
            Some(GuardPhase::Coming | GuardPhase::Near | GuardPhase::Leaving)
        );
        let mut first = Vec::new();
        if urgent {
            first.push(guard.to_string());
        }
        first.push(format!(
            "Dial at {}, wheel {} of {}.",
            run.dial,
            run.wheel + 1,
            run.spec.wheels
        ));
        if let Some(t) = run.required() {
            first.push(match run.last_turn {
                Some(last) => format!(
                    "Found turning {}; you last turned {}.",
                    t.word(),
                    last.word()
                ),
                None => format!("Found turning {}; you have not turned yet.", t.word()),
            });
        }
        let mut more = vec![
            format!("Strikes: {} of {STRIKES}.", run.strikes),
            match run.hints_left {
                0 => "No hints left.".to_string(),
                1 => "1 hint left.".to_string(),
                n => format!("{n} hints left."),
            },
        ];
        if run.mode.timed() {
            more.push(format!(
                "{} left on the clock.",
                sentence_start(&time_words(run.time_left))
            ));
        } else {
            more.push(format!(
                "{} on this safe.",
                sentence_start(&time_words(run.time))
            ));
        }
        if !urgent && !guard.is_empty() {
            more.push(guard.to_string());
        }
        if !guard.is_empty() && run.suspicion > 0 {
            more.push("He is suspicious.".to_string());
        }
        match run.mode {
            Mode::Unlucky => {
                more.push("One wheel is 13, and the cat yowls after its clunk.".to_string())
            }
            Mode::Blackout => {
                more.push("The power is cut: the tens click plain, so count from zero.".to_string())
            }
            _ => {}
        }
        if let Some(job) = &self.job {
            if job.safes() > 1 {
                more.push(format!("Safe {} of {}.", job.index + 1, job.safes()));
            }
            more.push(format!("{} points banked.", job.banked));
            if let Some(t) = job.plan.twist {
                more.push(format!("Twist: {}.", t.name()));
            }
        }
        vec![sentences(first), sentences(more)]
    }

    fn prompt(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say_queued(text.clone());
        self.last_prompt = text;
    }

    /// What to do after a spin clunks under the direction rule, where
    /// stepping back the other way is silent.
    fn spin_back_words(&self, ctx: &Ctx<Parlor>) -> Option<String> {
        let way = self.run.required()?;
        let (back, step) = match way {
            Turn::Right => (ParlorAction::Down, ParlorAction::Right),
            Turn::Left => (ParlorAction::Up, ParlorAction::Left),
        };
        Some(format!(
            "If a spin clunks, the number is in the ten you just passed, but stepping back the other way is silent under the direction rule. {} spins back those ten. Then step {} with {} one number at a time until it clunks.",
            sentence_start(&ctx.control_name(back)),
            way.word(),
            ctx.control_name(step)
        ))
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        let mut wait = AFTER_CLICK;
        let mut spun = false;
        for ev in evs {
            match ev {
                Ev::Click { detent, .. } => {
                    self.hush_yowl(ctx);
                    let detent = match (self.run.mode, detent) {
                        (Mode::Blackout, Detent::Ten) => Detent::Plain,
                        (_, d) => d,
                    };
                    sound::play(ctx, sound::detent(detent));
                    wait = AFTER_CLICK;
                }
                Ev::Spin { .. } => {
                    self.hush_yowl(ctx);
                    sound::play(ctx, sound::SPIN);
                    wait = AFTER_SPIN;
                    spun = true;
                }
                Ev::Clunk => {
                    self.later(wait, sound::CLUNK, self.run.spec.clunk);
                    if self.run.unlucky_wheel == Some(self.run.wheel) {
                        self.later(wait + AFTER_CLUNK, sound::YOWL, YOWL_VOLUME);
                    }
                    wait += 0.25;
                    if self.teaching(1) && self.first("clunk") {
                        let p = ctx.control_name(ParlorAction::Primary);
                        ctx.say(format!(
                            "Clunk: the tumbler. After one step, {p} sets it. After a spin, step back one at a time to find it."
                        ));
                    } else if spun && self.teaching(5) && self.first("spin rule") {
                        if let Some(words) = self.spin_back_words(ctx) {
                            ctx.say(format!("Clunk, on a spin. {words}"));
                        }
                    }
                }
                Ev::FalseGate => {
                    self.later(wait, sound::FALSE_GATE, self.run.spec.clunk * GATE_VOLUME);
                    wait += 0.25;
                    if self.teaching(3) && self.first("false") {
                        ctx.say("A hollow rattle: a false gate. Ignore it.");
                    }
                }
                Ev::Set { wheel, of } => {
                    sound::play(ctx, sound::SET);
                    ctx.say(format!("Wheel {wheel} set. {} to go.", of - wheel));
                    let p = self.wheel_prompt();
                    self.prompt(ctx, p);
                }
                Ev::PracticeSet { number, sets } => {
                    sound::play(ctx, sound::SET);
                    self.practice_unsaved += 1;
                    ctx.say(format!("Set, {number}. {sets} this session. New number."));
                    let p = self.wheel_prompt();
                    self.last_prompt = p;
                }
                Ev::Wrong { why, strikes } => {
                    sound::play(ctx, sound::SPRING);
                    let text = if self.run.practice() {
                        match why {
                            Wrong::Number => "Wrong number. Set on the deep clunk.".to_string(),
                            Wrong::Direction => "Wrong way.".to_string(),
                        }
                    } else {
                        let what = match why {
                            Wrong::Number => "Wrong number.".to_string(),
                            Wrong::Direction => format!(
                                "Right number, wrong way: turn {} for wheel {}.",
                                self.run.required().map(Turn::word).unwrap_or("right"),
                                self.run.wheel + 1,
                            ),
                        };
                        if strikes >= STRIKES {
                            what
                        } else {
                            format!("{what} Strike {strikes} of {STRIKES}.")
                        }
                    };
                    ctx.say(text);
                }
                Ev::Opened { .. } => {
                    sound::play(ctx, sound::OPEN);
                }
                Ev::Hint(hint) => {
                    sound::play(ctx, sound::HINT);
                    ctx.say(hint.words());
                }
                Ev::NoHints => {
                    sound::play(ctx, sound::NOPE);
                    ctx.say("No hints left for this safe.");
                }
                Ev::GuardComing => {
                    self.start_steps(ctx, 0.15);
                    if self.run.spec.call_guard {
                        ctx.say("Footsteps! The guard is coming.");
                    }
                }
                Ev::GuardNear => {
                    self.stop_steps(ctx);
                    ctx.audio.play_at(sound::STOP, sound::DOOR, Bus::EFFECTS);
                    if self.run.suspicion > 0 {
                        // He heard you before: one more noise is the end.
                        ctx.say("He is back. One more noise and you are caught.");
                    } else if self.run.spec.call_guard {
                        ctx.say("He stops at the door. Keep still!");
                    }
                }
                Ev::GuardLeaving => {
                    self.start_steps(ctx, 1.0);
                    if self.run.spec.call_guard {
                        ctx.say("He walks away. Carry on.");
                    }
                }
                Ev::GuardGone => {
                    self.stop_steps(ctx);
                }
                Ev::Heard { suspicion } => {
                    if suspicion < SUSPICION {
                        ctx.audio.play_at(sound::HEARD, sound::DOOR, Bus::EFFECTS);
                        ctx.say("He heard something! One more noise and you are caught.");
                    } else {
                        ctx.audio.play_at(sound::CAUGHT, sound::DOOR, Bus::EFFECTS);
                    }
                }
                Ev::TimeWarning(secs) => {
                    sound::play(ctx, sound::TICK);
                    ctx.say(format!(
                        "{} left.",
                        sentence_start(&time_words(secs as f64))
                    ));
                }
                Ev::Noise { kind, x } => {
                    let s =
                        ctx.audio
                            .play_at(sound::noise(kind), Vec3::new(x, 1.0, 0.0), Bus::EFFECTS);
                    ctx.audio.set_volume(s, NOISE_VOLUME);
                }
                Ev::Ended(outcome) => {
                    if outcome == Outcome::Jammed {
                        sound::play(ctx, sound::ALARM);
                    }
                    self.finish(ctx, outcome);
                }
            }
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>, outcome: Outcome) {
        self.stop_steps(ctx);
        self.hush_yowl(ctx);
        self.delayed.clear();
        if self.job.is_some() {
            self.finish_safe(ctx);
            return;
        }
        self.score.stop(ctx);
        let saved = progress::save_end(ctx, &self.run);
        ctx.audio.set_bed(None);
        ctx.audio.play_music(
            Some(if outcome.won() {
                sound::WIN
            } else {
                sound::LOSE
            }),
            false,
        );
        let pages = result_pages(&self.run, outcome, &saved);
        let summary = result_summary(&self.run, outcome);
        let mode = self.run.mode;
        let won = outcome.won();
        let mut results = Some(after_safe(mode, won, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    /// A challenge safe ended: on to the next, or the job is over.
    fn finish_safe(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(job) = self.job.as_mut() else {
            return;
        };
        let before = job.banked;
        if let Some(next) = job.advance(&self.run) {
            let gained = job.banked - before;
            let banked = job.banked;
            let name = self.run.spec.name;
            self.run = next;
            // The next safe's clock waits for its first turn too.
            self.run.clock_running = false;
            self.later(1.0, sound::ONWARD, 1.0);
            ctx.say(format!(
                "Cracked. {} is open: {gained} points, {banked} banked.",
                sentence_start(name)
            ));
            if let Some(job) = &self.job {
                let intro = self.safe_intro(job, "On to the next.");
                ctx.say_queued(intro);
            }
            let p = self.wheel_prompt();
            self.prompt(ctx, p);
            return;
        }
        self.score.stop(ctx);
        let (Some(job), Some(ch)) = (self.job.take(), self.challenge.take()) else {
            return;
        };
        ctx.audio.set_bed(None);
        ctx.audio.play_music(
            Some(if job.won() { sound::WIN } else { sound::LOSE }),
            false,
        );
        let pages = crate::heist::finish_job(ctx, &ch, &job, &self.run);
        let summary = format!("{} result: {} points", ch.title(), job.score());
        let mut results = Some(after_job(ch.period, summary, pages.clone()));
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }

    fn save_practice(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.practice_unsaved > 0 {
            progress::save_practice(ctx, self.practice_unsaved);
            self.practice_unsaved = 0;
        }
    }
}

/// "2 hints", "1 hint", "no hints".
fn hints_words(n: u32) -> String {
    match n {
        0 => "no hints".to_string(),
        1 => "1 hint".to_string(),
        n => format!("{n} hints"),
    }
}

/// The one-line headline of a safe's results, the results menu's title.
pub fn result_summary(run: &Run, outcome: Outcome) -> String {
    let name = run.spec.name;
    match outcome {
        Outcome::Opened => format!("Cracked in {}. Score {}", time_words(run.time), run.score),
        Outcome::Jammed => format!("Jammed. {} stays shut", sentence_start(name)),
        Outcome::Caught => format!("Caught at {name}"),
        Outcome::TimeUp => format!("Time's up. {} stays shut", sentence_start(name)),
    }
}

/// The results menu after a safe: the next safe when this one opened, the
/// same safe again, the results again, and back to the title.
pub fn after_safe(mode: Mode, won: bool, summary: String, pages: Vec<String>) -> Results {
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if let (true, Mode::Safe(n)) = (won, mode) {
        if n < SAFES {
            let next = n + 1;
            results = results.next(
                format!(
                    "Next safe: safe {next}, {}",
                    crate::rules::safe_spec(next).name
                ),
                format!("{}.", safe_summary(next)),
                move |ctx| ctx.replace(Crack::new(Mode::Safe(next))),
            );
        }
    }
    let again = if won {
        labels::PLAY_AGAIN
    } else {
        labels::TRY_AGAIN
    };
    results.again(
        again,
        format!("The same safe with new numbers. {STRIKES} strikes allowed, as always."),
        move |ctx| ctx.replace(Crack::new(mode)),
    )
}

/// The results menu after a challenge job: the job again, the results
/// again, and back to the challenges.
pub fn after_job(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same job, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Crack::challenge(ch))
        },
    )
}

/// The pages after a safe ends.
pub fn result_pages(run: &Run, outcome: Outcome, saved: &Saved) -> Vec<String> {
    let name = run.spec.name;
    let mut first = match outcome {
        Outcome::Opened => format!(
            "Cracked. The handle turns and {name} swings open, in {}.",
            time_words(run.time)
        ),
        Outcome::Jammed => format!(
            "Three wrong sets. {} jams shut and the alarm rings.",
            sentence_start(name)
        ),
        Outcome::Caught => format!("Caught by the guard at {name}."),
        Outcome::TimeUp => format!("Time's up. {} stays shut.", sentence_start(name)),
    };
    if outcome.won() {
        first.push_str(&match saved.score {
            Some(ScoreResult::First) => format!(" Score {}: your first on this safe.", run.score),
            Some(ScoreResult::NewBest { previous }) => {
                format!(" Score {}: a new best, beating {previous}.", run.score)
            }
            Some(ScoreResult::Short { best }) => {
                format!(" Score {}. Your best is {best}.", run.score)
            }
            None => String::new(),
        });
        match (saved.record, saved.previous) {
            (true, Some(prev)) => {
                first.push_str(&format!(" A new best time, beating {}.", time_words(prev)))
            }
            // A first score has said it is the first.
            (true, None) if !matches!(saved.score, Some(ScoreResult::First)) => {
                first.push_str(" Your first time on the board.")
            }
            (false, Some(prev)) => {
                first.push_str(&format!(" Your best time is {}.", time_words(prev)))
            }
            _ => {}
        }
    } else {
        first.push_str(&format!(
            " You set {} of {} wheels.",
            run.wheel, run.spec.wheels
        ));
    }
    let mut pages = vec![first];
    let mut tally = Vec::new();
    // A jam has said its three strikes already.
    if run.strikes > 0 && outcome != Outcome::Jammed {
        tally.push(format!("Strikes: {}.", run.strikes));
    }
    if run.hints_used > 0 {
        tally.push(format!("Hints used: {}.", run.hints_used));
    }
    if run.guard_visits > 0 {
        tally.push(format!("Guard visits slipped past: {}.", run.guard_visits));
    }
    if !tally.is_empty() {
        pages.push(tally.join(" "));
    }
    let mut news = Vec::new();
    if let Some(next) = saved.opened {
        news.push(format!(
            "Safe {next}, {}, is open.",
            crate::rules::safe_spec(next).name
        ));
    } else if outcome.won() && run.mode == Mode::Safe(SAFES) && saved.previous.is_none() {
        news.push(
            "You have cracked every safe in the city. Time attack waits for faster hands."
                .to_string(),
        );
    }
    if saved.attack {
        news.push("Time attack is open on the title.".to_string());
    }
    if !news.is_empty() {
        pages.push(news.join(" "));
    }
    pages
}

impl State<Parlor> for Crack {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        CRACK_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.started = true;
        // The clock waits for the first turn, not the intro.
        self.run.clock_running = false;
        // A player who has cracked a safe knows the clunk.
        if progress::Progress::load(ctx).cracked > 0 {
            self.taught.insert("clunk");
        }
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        self.bed(ctx);
        let mood = self.mood();
        self.score.start(ctx, &sound::SCORE, mood);
        let intro = self.intro(ctx);
        ctx.say(intro);
        if let Some(words) = self.event_words(ctx) {
            ctx.say_queued(words);
        }
        // The control list is for a first safe before the tutorial: the
        // tutorial has taught them.
        let full = matches!(self.run.mode, Mode::Practice | Mode::Safe(1))
            && !parlor_core::tutorial::tutorial_done(ctx, ID);
        if full {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
        let p = self.wheel_prompt();
        self.prompt(ctx, p);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_steps(ctx);
        self.score.stop(ctx);
        self.save_practice(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.run.finished() {
            return;
        }
        let mut text = format!("Back at the dial, at {}.", self.run.dial);
        if self.run.guard_listening() {
            text.push_str(" The guard is still at the door: keep still!");
        }
        ctx.say(text);
        let p = self.wheel_prompt();
        self.prompt(ctx, p);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.run.finished() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if event.is_repeat() {
                return;
            }
            self.save_practice(ctx);
            let leave = if self.job.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        // Turning may repeat while held; everything else wants a fresh press.
        let turning = matches!(
            action,
            Some(ParlorAction::Left | ParlorAction::Right | ParlorAction::Up | ParlorAction::Down)
        );
        if event.is_repeat() && !turning {
            return;
        }
        // A held key at the door counts once: after he hears you, only a
        // fresh press can make the second noise, so a key held as the
        // footsteps stop never gets you caught in a blink.
        if event.is_repeat() && self.run.guard_listening() && self.run.suspicion > 0 {
            return;
        }
        if matches!(
            action,
            Some(
                ParlorAction::Left
                    | ParlorAction::Right
                    | ParlorAction::Up
                    | ParlorAction::Down
                    | ParlorAction::Primary
            )
        ) {
            self.run.clock_running = true;
        }
        let evs = match action {
            Some(ParlorAction::Left) => self.run.turn(Turn::Left),
            Some(ParlorAction::Right) => self.run.turn(Turn::Right),
            Some(ParlorAction::Up) => self.run.spin(Turn::Right),
            Some(ParlorAction::Down) => self.run.spin(Turn::Left),
            Some(ParlorAction::Primary) => self.run.set(),
            Some(ParlorAction::Hint) => self.run.hint(),
            Some(ParlorAction::Secondary) => {
                ctx.say(format!("{}.", self.run.dial));
                return;
            }
            Some(ParlorAction::Status) => {
                let layers = self.status();
                self.status_key.speak(ctx, layers);
                return;
            }
            Some(ParlorAction::Repeat) => {
                let text = if self.last_prompt.is_empty() {
                    self.wheel_prompt()
                } else {
                    self.last_prompt.clone()
                };
                ctx.say(text);
                return;
            }
            None => return,
        };
        self.voice(ctx, evs);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if !self.started || self.run.finished() {
            return;
        }
        let mood = self.mood();
        if mood.intensity == 0.0 && self.score.mood().intensity > 0.0 {
            // He is at the door: the band falls quiet at once.
            self.score.jump(ctx, mood);
        } else {
            self.score.follow(ctx, mood, dt);
        }
        let mut due = Vec::new();
        self.delayed.retain_mut(|(wait, key, vol)| {
            *wait -= dt;
            if *wait <= 0.0 {
                due.push((*key, *vol));
                false
            } else {
                true
            }
        });
        for (key, vol) in due {
            let s = sound::play_vol(ctx, key, vol);
            if key == sound::YOWL {
                self.yowl = s;
            }
        }
        let evs = self.run.update(dt);
        if self.steps != Sound::NONE {
            if let Some(g) = self.run.guard {
                let p = g.progress() as f32;
                let vol = match g.phase {
                    GuardPhase::Coming => 0.15 + 0.85 * p,
                    GuardPhase::Leaving => (1.0 - p * 0.9).max(0.1),
                    _ => 0.0,
                };
                ctx.audio.set_volume(self.steps, vol);
            }
        }
        self.voice(ctx, evs);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let run = &self.run;
        let mut out = vec![if run.practice() {
            "Practice safe".to_string()
        } else if let Some(job) = &self.job {
            format!(
                "{}, safe {} of {}: {}",
                challenge::name(job.plan.period),
                job.index + 1,
                job.safes(),
                run.spec.name
            )
        } else if run.mode.is_event() {
            sentence_start(run.spec.name)
        } else {
            format!("Safe {}: {}", run.spec.number, run.spec.name)
        }];
        out.push(format!("Dial: {}", run.dial));
        if !run.practice() {
            out.push(format!(
                "Wheel {} of {}  Strikes {}/{}  Hints {}",
                (run.wheel + 1).min(run.spec.wheels),
                run.spec.wheels,
                run.strikes,
                STRIKES,
                run.hints_left
            ));
            let clock = if run.mode.timed() {
                format!("Time left {}", time_words(run.time_left))
            } else {
                format!("Time {}", time_words(run.time))
            };
            out.push(clock);
        }
        let guard = self.guard_words();
        if !guard.is_empty() {
            out.push(guard.to_string());
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let run = &self.run;
        let now = if run.guard_listening() {
            "The guard is at the door, listening: keep still! Do not turn or set until his footsteps walk away. Status and hints are silent and safe.".to_string()
        } else if run.practice() {
            "Practice: turn to the clunk, then set. Nothing here is scored."
                .to_string()
        } else {
            let mut s = self.wheel_prompt();
            if let Some(words) = self.spin_back_words(ctx) {
                s.push(' ');
                s.push_str(&words);
            }
            if run.spec.decoys > 0 {
                s.push_str(" Hollow rattles are false gates: ignore them.");
            }
            if let Some(words) = self.event_words(ctx) {
                s.push(' ');
                s.push_str(&words);
            }
            if run.spec.guard.is_some() {
                s.push_str(
                    " When the guard's footsteps stop at the door, keep still until he leaves.",
                );
            }
            if let Some(job) = &self.job {
                s.push_str(&format!(
                    " This is the {} challenge, {}: safe {} of {}.",
                    job.plan.period.label().to_lowercase(),
                    challenge::name(job.plan.period),
                    job.index + 1,
                    job.safes()
                ));
                if let Some(t) = job.plan.twist {
                    s.push_str(&format!(" Twist, {}: {}.", t.name(), t.describe()));
                }
                if job.plan.pooled() {
                    s.push_str(" Strikes and hints are shared across the vault.");
                }
                if job.plan.clock().is_some() {
                    s.push_str(" One clock runs for the whole heist.");
                }
                s.push_str(" Leaving from the pause menu does not count the job.");
            }
            s
        };
        format!("{now} {}", self.controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_crack;
