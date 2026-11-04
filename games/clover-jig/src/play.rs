//! The dance floor: the play screen for every mode.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::challenges::today;
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, Rng, ScoreResult, StatusKey};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::progress::{self, CeiliSaved, Progress, SetSaved};
use crate::rules::{
    beats_words, hide_clover, hide_shamrocks, luck_words, points_words, stars_words, steps_words,
    Ev, Judgment, Luck, Phase, Plan, Run, RunEv, TuneResult, CLOVER_BONUS, STREAK_BONUS, TUNE_BARS,
};
use crate::season;
use crate::tunes::{set_of, Place, Step, CEILI_LEVEL, CEILI_TUNES, FIGURES, SETS, TUNES};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "clover_jig_play";
/// Seconds between the calls of the before-dance hint.
pub const HINT_GAP: f64 = 1.2;
/// Seconds from the end of a run to its results, for the applause.
pub const END_PAUSE: f64 = 2.5;

/// What the floor is dancing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A set of three tunes, by number from 1.
    Set(u32),
    /// The practice room: one tune, by index into `TUNES`, that waits for
    /// each step.
    Practice(usize),
    /// Saint Patrick's night: the set dance of four tunes.
    Ceili,
    Challenge(Period),
}

impl Mode {
    pub fn name(self) -> String {
        match self {
            Mode::Set(n) => SETS[(n as usize - 1).min(SETS.len() - 1)].title(),
            Mode::Practice(_) => "Practice room".to_string(),
            Mode::Ceili => "Ceili night".to_string(),
            Mode::Challenge(period) => challenge::name(period).to_string(),
        }
    }

    pub fn place(self) -> Place {
        match self {
            Mode::Set(n) => SETS[(n as usize - 1).min(SETS.len() - 1)].place,
            Mode::Practice(i) => set_of(i).place,
            Mode::Ceili => Place::Hall,
            Mode::Challenge(_) => Place::Outdoors,
        }
    }
}

/// The tunes a mode dances, from `seed`. Shamrocks hide in sets and on
/// ceili night when `shamrocks` is on; the clover hides in one of the last
/// two tunes of the set dance when `clover` is on.
pub fn plans(mode: Mode, seed: u64, shamrocks: bool, clover: bool) -> Vec<Plan> {
    let mut rng = Rng::seeded(seed);
    match mode {
        Mode::Set(n) => {
            let set = &SETS[(n as usize - 1).min(SETS.len() - 1)];
            set.tunes()
                .into_iter()
                .map(|t| {
                    let mut p = Plan::new(t, &set.level, &mut rng);
                    if shamrocks {
                        hide_shamrocks(&mut p.calls, &mut rng);
                    }
                    p
                })
                .collect()
        }
        Mode::Practice(i) => {
            let i = i.min(TUNES.len() - 1);
            vec![Plan::new(&TUNES[i], &set_of(i).level, &mut rng)]
        }
        Mode::Ceili => {
            let mut out: Vec<Plan> = CEILI_TUNES
                .iter()
                .map(|t| {
                    let mut p = Plan::new(t, &CEILI_LEVEL, &mut rng);
                    if shamrocks {
                        hide_shamrocks(&mut p.calls, &mut rng);
                    }
                    p
                })
                .collect();
            if clover {
                let at = 2 + rng.below(2) as usize;
                hide_clover(&mut out[at].calls, &mut rng);
            }
            out
        }
        Mode::Challenge(period) => challenge::Plan::new(period, Rng::seeded(seed)).plans,
    }
}

/// A seed from the clock, for real play.
pub fn run_seed() -> u64 {
    Rng::from_time().next_u64()
}

pub struct Floor {
    pub run: Run,
    pub mode: Mode,
    challenge: Option<Challenge>,
    /// Cues already put in words this run.
    taught: HashSet<&'static str>,
    /// The parts of the tune still sounding, to hold in a wait or a count.
    parts: Vec<Sound>,
    /// The before-dance hint's calls still to play: seconds to wait, step.
    delayed: Vec<(f64, Step)>,
    ending: Option<f64>,
    practice_unsaved: u32,
    /// Tonight, for the clover's once a night.
    night: String,
    year: i32,
    call_outs: bool,
    shamrocks: bool,
    /// S, in layers: a second press reads the rest.
    status_key: StatusKey,
}

impl Floor {
    /// A run of `mode`, from the clock's seed.
    pub fn open(ctx: &mut Ctx<Parlor>, mode: Mode) -> Floor {
        Floor::with_seed(ctx, mode, run_seed())
    }

    /// A run of `mode` from `seed`, with today's shamrocks and clover.
    pub fn with_seed(ctx: &mut Ctx<Parlor>, mode: Mode, seed: u64) -> Floor {
        let date = today(ctx);
        let night = season::night_key(date);
        let shamrocks = matches!(mode, Mode::Set(_) | Mode::Ceili) && season::shamrocks_on(date);
        let clover = mode == Mode::Ceili && progress::clover_due(ctx, &night);
        let run = Run::new(
            plans(mode, seed, shamrocks, clover),
            matches!(mode, Mode::Practice(_)),
        );
        Floor {
            run,
            mode,
            challenge: None,
            taught: HashSet::new(),
            parts: Vec::new(),
            delayed: Vec::new(),
            ending: None,
            practice_unsaved: 0,
            night,
            year: season::keepsake_year(date),
            call_outs: crate::call_outs(ctx),
            shamrocks,
            status_key: StatusKey::default(),
        }
    }

    /// A run of the challenge's tunes.
    pub fn challenge(ctx: &mut Ctx<Parlor>, ch: Challenge) -> Floor {
        let mut f = Floor::with_seed(ctx, Mode::Challenge(ch.period), ch.seed);
        f.challenge = Some(ch);
        f
    }

    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    fn step_key(ctx: &Ctx<Parlor>, step: Step) -> String {
        ctx.control_name(step.action())
    }

    /// "Hop: Space."
    fn step_line(ctx: &Ctx<Parlor>, step: Step) -> String {
        format!("{}: {}.", step.word(), Self::step_key(ctx, step))
    }

    fn start_prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} starts the dance. {} plays every step.",
            sentence_start(&Self::key(ctx, ParlorAction::Primary)),
            sentence_start(&Self::key(ctx, ParlorAction::Hint))
        )
    }

    fn tune_names(&self) -> String {
        let names: Vec<&str> = self.run.plans.iter().map(|p| p.tune.name).collect();
        names.join(", ")
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let n = self.run.plans.len();
        let what = match self.mode {
            Mode::Set(_) => format!("3 tunes: {}.", self.tune_names()),
            Mode::Practice(_) => format!(
                "{}. The tune waits at each step until you dance it.",
                self.run.plans[0].tune.title()
            ),
            Mode::Ceili => format!(
                "The full band plays the set dance, {n} tunes: {}.",
                self.tune_names()
            ),
            Mode::Challenge(_) => {
                if n == 1 {
                    format!("1 tune, {}.", self.run.plans[0].tune.title())
                } else {
                    format!("{n} tunes: {}.", self.tune_names())
                }
            }
        };
        format!("{}. {what} {}", self.mode.name(), self.start_prompt(ctx))
    }

    /// The second line of the intro: what is new or special this run.
    fn news(&self, ctx: &Ctx<Parlor>) -> Option<String> {
        if self.run.plans.iter().any(|p| p.has_clover()) {
            return Some(
                "A four-leaf clover hides tonight: three sparkling notes. Dance that step."
                    .to_string(),
            );
        }
        if self.shamrocks {
            return Some(
                "Shamrocks hide in the tunes: two high chimes. Dance that step well for luck."
                    .to_string(),
            );
        }
        if let Mode::Set(n) = self.mode {
            let set = &SETS[(n as usize - 1).min(SETS.len() - 1)];
            let new: Vec<String> = set
                .new_steps()
                .iter()
                .map(|s| format!("{}, {}", s.lower(), Self::step_key(ctx, *s)))
                .collect();
            if n > 1 && !new.is_empty() {
                return Some(format!("New in this set: {}.", new.join(", and ")));
            }
        }
        None
    }

    /// Every control, in a few sentences.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        // Only the steps these tunes call.
        let steps = self.steps_in_run();
        let has = |s| steps.contains(&s);
        let mut moves = Vec::new();
        if has(Step::Left) || has(Step::Right) {
            moves.push(format!(
                "{} and {} dance left and right",
                k(ParlorAction::Left),
                k(ParlorAction::Right)
            ));
        }
        if has(Step::Hop) {
            moves.push(format!("{} hops", k(ParlorAction::Primary)));
        }
        if has(Step::Turn) {
            moves.push(format!("{} turns", k(ParlorAction::Secondary)));
        }
        if has(Step::Forward) || has(Step::Back) {
            moves.push(format!(
                "{} and {} dance forward and back",
                k(ParlorAction::Up),
                k(ParlorAction::Down)
            ));
        }
        let moves = match moves.split_last() {
            Some((last, rest)) if !rest.is_empty() => {
                format!("{}, and {last}", rest.join(", "))
            }
            _ => moves.join(""),
        };
        format!(
            "Dance each step on its beat. {}. {} speaks the status, {} repeats the last call, {} names the next step, and Escape pauses.",
            sentence_start(&moves),
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Repeat),
            k(ParlorAction::Hint)
        )
    }

    /// The status line, in a fixed order: the first layer of S.
    pub fn status(&self, ctx: &Ctx<Parlor>) -> String {
        self.status_layers(ctx).remove(0)
    }

    /// S, in layers. In a dance the first press says where you are and the
    /// score; a second press names the tune and how the steps have gone.
    pub fn status_layers(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let run = &self.run;
        let name = self.mode.name();
        let n = run.plans.len();
        match run.phase {
            Phase::Ready => {
                let tunes = if n == 1 {
                    "1 tune".to_string()
                } else {
                    format!("{n} tunes")
                };
                vec![format!(
                    "{name}, {tunes}, not started. {}",
                    self.start_prompt(ctx)
                )]
            }
            Phase::Between(_) => vec![format!(
                "{name}, tune {} of {n} done, score {}.",
                run.at + 1,
                run.score()
            )],
            Phase::Done => vec![format!("{name}, every tune danced, score {}.", run.score())],
            Phase::Dancing => {
                let d = &run.dance;
                let mut parts = Vec::new();
                if let Some(i) = d.waiting() {
                    let step = d.plan.calls[i].step;
                    parts.push(format!(
                        "Waiting for {}, {}",
                        step.lower(),
                        Self::step_key(ctx, step)
                    ));
                }
                parts.push(match d.bar() {
                    0 => "counting in".to_string(),
                    b => format!("bar {b} of {TUNE_BARS}"),
                });
                if run.practice() {
                    parts.push(format!("{} danced", steps_words(d.tally().hits())));
                } else {
                    parts.push(format!("score {}", run.score()));
                    parts.push(format!("streak {}", d.streak));
                    if self.shamrocks {
                        parts.push(luck_words(run.luck()));
                    }
                }
                let first = sentence_start(&format!("{}.", parts.join(", ")));
                let mut more = vec![d.plan.tune.name.to_string()];
                if n > 1 {
                    more.push(format!("tune {} of {n}", run.at + 1));
                }
                more.push(d.tally().words());
                vec![first, format!("{}.", more.join(", "))]
            }
        }
    }

    fn hint_all(&mut self, ctx: &mut Ctx<Parlor>) {
        let steps: Vec<Step> = self.steps_in_run();
        let lines: Vec<String> = steps
            .iter()
            .map(|s| format!("{}, {}.", s.word(), Self::step_key(ctx, *s)))
            .collect();
        ctx.say(lines.join(" "));
        self.delayed = steps
            .into_iter()
            .enumerate()
            .map(|(i, s)| (i as f64 * HINT_GAP, s))
            .collect();
        self.play_due(ctx, 0.0);
    }

    /// The steps the caller uses in this run, in their usual order.
    fn steps_in_run(&self) -> Vec<Step> {
        Step::ALL
            .into_iter()
            .filter(|s| {
                self.run
                    .plans
                    .iter()
                    .any(|p| p.calls.iter().any(|c| c.step == *s))
            })
            .collect()
    }

    fn play_due(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut due = Vec::new();
        self.delayed.retain_mut(|(wait, step)| {
            *wait -= dt;
            if *wait <= 1e-9 {
                due.push(*step);
                false
            } else {
                true
            }
        });
        for step in due {
            sound::play_call(ctx, step);
            sound::play_step(ctx, step, 0.8);
        }
    }

    /// Before the dance, a step key plays its call and step, and names it.
    fn preview(&mut self, ctx: &mut Ctx<Parlor>, step: Step) {
        sound::play_call(ctx, step);
        sound::play_step(ctx, step, 0.8);
        let used = self.steps_in_run().contains(&step);
        let line = Self::step_line(ctx, step);
        if used {
            ctx.say(line);
        } else {
            ctx.say(format!("{line} Not called in these tunes."));
        }
    }

    fn start(&mut self, ctx: &mut Ctx<Parlor>) {
        self.delayed.clear();
        self.run.start();
        ctx.say("Here we go. Four beats in.");
    }

    /// Stops the tune for `secs`, then counts it back in, so a line said
    /// now is heard before the next call.
    pub fn count_in_after(&mut self, ctx: &mut Ctx<Parlor>, secs: f64) {
        self.run.count_in_after((secs * 1000.0).round() as i64);
        if self.run.dance.counting_in() {
            self.hold_parts(ctx);
        }
    }

    fn hold_parts(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in &self.parts {
            ctx.audio.pause(*s);
        }
    }

    fn release_parts(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in &self.parts {
            ctx.audio.resume(*s);
        }
    }

    fn stop_parts(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.parts.drain(..) {
            ctx.audio.stop(s, 300);
        }
    }

    fn on_dance(&mut self, ctx: &mut Ctx<Parlor>, ev: Ev) {
        match ev {
            Ev::Count(_) => {
                sound::play(ctx, sound::COUNT);
            }
            Ev::Part { b, .. } => {
                let key = self.run.dance.plan.tune.part_key(b);
                let s = sound::play_part(ctx, &key);
                let audio = &ctx.audio;
                self.parts.retain(|p| audio.is_playing(*p));
                self.parts.push(s);
            }
            Ev::Call { step, luck, .. } => {
                sound::play_call(ctx, step);
                match luck {
                    Luck::Shamrock => {
                        sound::play(ctx, sound::SHAMROCK);
                    }
                    Luck::Clover => {
                        sound::play(ctx, sound::CLOVER_CALL);
                        let night = self.night.clone();
                        progress::clover_called(ctx, &night);
                    }
                    Luck::None => {}
                }
                if self.call_outs {
                    ctx.say(step.word());
                }
            }
            Ev::Judged { step, judgment, .. } => {
                sound::play_step(ctx, step, 1.0);
                match judgment {
                    Judgment::SpotOn => {
                        sound::play(ctx, sound::SPOT_ON);
                    }
                    Judgment::Close => {
                        sound::play(ctx, sound::CLOSE);
                    }
                    _ => {}
                }
                if self.run.practice() {
                    self.practice_unsaved += 1;
                }
                let cue = match judgment {
                    Judgment::SpotOn => "spot on",
                    Judgment::Good => "good",
                    Judgment::Close => "close",
                    _ => "danced",
                };
                if judgment != Judgment::Waited && self.first(cue) {
                    ctx.say(judgment.word());
                }
            }
            Ev::Missed { step, .. } => {
                sound::play(ctx, sound::STUMBLE);
                if self.first("missed") {
                    ctx.say(format!("Missed. That was {}.", step.lower()));
                }
            }
            Ev::Wrong { want, got, .. } => {
                sound::play(ctx, sound::STUMBLE);
                if self.run.practice() {
                    let key = Self::step_key(ctx, want);
                    ctx.say(format!("Not {}. {}: {key}.", got.lower(), want.word()));
                } else if self.first("wrong") {
                    ctx.say(format!("Wrong step. That was {}.", want.lower()));
                }
            }
            Ev::Stray(step) => {
                sound::play_step(ctx, step, sound::STRAY_VOLUME);
            }
            Ev::Streak { run, bonus } => {
                sound::play(ctx, sound::CLAP);
                if self.first("streak") {
                    ctx.say(format!("{run} in a row. Bonus {bonus}."));
                }
            }
            Ev::Shamrock { luck } => {
                sound::play(ctx, sound::LUCKY);
                if self.first("luck") {
                    ctx.say(format!("Shamrock. {}.", sentence_start(&luck_words(luck))));
                }
            }
            Ev::Clover => {
                sound::play(ctx, sound::CLOVER);
                ctx.say(format!("Four-leaf clover found. {CLOVER_BONUS} points."));
            }
            Ev::Wait { step, .. } => {
                self.hold_parts(ctx);
                let line = Self::step_line(ctx, step);
                ctx.say(line);
            }
            Ev::Go => self.release_parts(ctx),
            Ev::Ended => {
                sound::play(ctx, sound::APPLAUSE);
            }
        }
    }

    fn tune_line(r: &TuneResult) -> String {
        format!(
            "{}: {}, {}.",
            r.tune.name,
            stars_words(r.stars),
            points_words(r.score)
        )
    }

    fn on_run(&mut self, ctx: &mut Ctx<Parlor>, ev: RunEv) {
        match ev {
            RunEv::Dance(ev) => self.on_dance(ctx, ev),
            RunEv::TuneDone(i) => {
                let Some(r) = self.run.results.last().copied() else {
                    return;
                };
                let line = if self.run.practice() {
                    format!("{} danced.", r.tune.name)
                } else {
                    Self::tune_line(&r)
                };
                if let Some(next) = self.run.plans.get(i + 1) {
                    let next = match self.mode {
                        Mode::Ceili => format!(
                            "Next figure, {}: {}.",
                            FIGURES[(i + 1).min(FIGURES.len() - 1)].to_lowercase(),
                            next.tune.title()
                        ),
                        _ => format!("Next, {}.", next.tune.title()),
                    };
                    ctx.say(format!("{line} {next}"));
                } else {
                    ctx.say(line);
                }
                self.parts.clear();
            }
            RunEv::Next(_) => {}
            RunEv::Finished => self.ending = Some(END_PAUSE),
        }
    }

    fn dance(&mut self, ctx: &mut Ctx<Parlor>, step: Step) {
        if !self.run.started() {
            self.preview(ctx, step);
            return;
        }
        let evs = self.run.press(step);
        for ev in evs {
            self.on_dance(ctx, ev);
        }
    }

    fn repeat(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.run.started() {
            let p = self.start_prompt(ctx);
            ctx.say(p);
            return;
        }
        let d = &self.run.dance;
        match (self.run.phase, d.last_call) {
            (Phase::Dancing, Some(i)) => {
                let step = d.plan.calls[i].step;
                sound::play_call(ctx, step);
                let key = Self::step_key(ctx, step);
                ctx.say(format!("Last call: {}, {key}.", step.lower()));
            }
            _ => ctx.say("No call yet in this tune."),
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.run.started() {
            self.hint_all(ctx);
            return;
        }
        if self.run.phase != Phase::Dancing {
            ctx.say("Between tunes. The next one starts soon.");
            return;
        }
        let d = &self.run.dance;
        match d.next_open() {
            Some(i) => {
                let step = d.plan.calls[i].step;
                let key = Self::step_key(ctx, step);
                let when = match d.beats_until(i) {
                    0 => "now".to_string(),
                    n => format!("in {}", beats_words(n)),
                };
                ctx.say(format!("Next: {}, {key}, {when}.", step.lower()));
            }
            None => ctx.say("No more steps in this tune."),
        }
    }

    fn save_practice(&mut self, ctx: &mut Ctx<Parlor>) {
        progress::save_practice(ctx, self.practice_unsaved);
        self.practice_unsaved = 0;
    }

    /// The run is over: record it and read the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_parts(ctx);
        ctx.audio.set_bed(None);
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.run);
            let summary = challenge::summary(&ch, &self.run);
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let (pages, results) = match self.mode {
            Mode::Set(n) => {
                let saved = progress::save_set(ctx, n, &self.run);
                let pages = set_pages(n, &self.run, &saved);
                let summary = set_summary(n, &self.run);
                (pages.clone(), after_set(n, summary, pages, &saved))
            }
            Mode::Practice(i) => {
                self.save_practice(ctx);
                let hits = self.run.tally().hits();
                let summary = format!(
                    "Practice done: {}. {} danced.",
                    TUNES[i].name,
                    sentence_start(&steps_words(hits))
                );
                let pages = vec![summary.clone()];
                (pages.clone(), after_practice(i, summary, pages))
            }
            Mode::Ceili => {
                let saved = progress::save_ceili(ctx, self.year, &self.run);
                let pages = ceili_pages(&self.run, &saved);
                let summary = format!(
                    "Ceili night danced. {}.",
                    sentence_start(&points_words(self.run.score()))
                );
                (pages.clone(), after_ceili(summary, pages))
            }
            Mode::Challenge(_) => return,
        };
        let mut results = Some(results);
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// Keys a player presses for their screen reader or as a modifier, which
/// the floor leaves alone.
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

fn score_words(result: ScoreResult, score: u32) -> String {
    match result {
        ScoreResult::First => format!("{}, your first score here.", points_words(score)),
        ScoreResult::NewBest { previous } => {
            format!("{}, a new best, beating {previous}.", points_words(score))
        }
        ScoreResult::Short { best } => format!("{}. Your best is {best}.", points_words(score)),
    }
}

/// The headline of a set, the results menu's title.
pub fn set_summary(n: u32, run: &Run) -> String {
    let stars: u32 = run.results.iter().map(|r| r.stars).sum();
    format!(
        "Set {n} danced. {}, {}.",
        sentence_start(&points_words(run.score())),
        stars_words(stars)
    )
}

/// The pages after a set.
pub fn set_pages(n: u32, run: &Run, saved: &SetSaved) -> Vec<String> {
    let set = &SETS[(n as usize - 1).min(SETS.len() - 1)];
    let mut first = format!(
        "{} danced. {}",
        set.title(),
        sentence_start(&score_words(saved.score, run.score()))
    );
    if let Some(open) = saved.opened {
        let next = &SETS[(open as usize - 1).min(SETS.len() - 1)];
        first.push_str(&format!(" {} is open.", next.title()));
    }
    let tunes: Vec<String> = run.results.iter().map(Floor::tune_line).collect();
    let mut pages = vec![first, tunes.join(" ")];
    let mut steps = format!("Steps: {}.", run.tally().words());
    if saved.streak_record {
        steps.push_str(&format!(
            " Longest run in time: {}, your best yet.",
            steps_words(saved.best_streak)
        ));
    } else {
        steps.push_str(&format!(
            " Longest run in time this set: {}.",
            steps_words(run.best_streak())
        ));
    }
    pages.push(steps);
    if !saved.better.is_empty() {
        let better: Vec<String> = saved
            .better
            .iter()
            .map(|(name, s)| format!("{name}, {}", stars_words(*s)))
            .collect();
        pages.push(format!("More stars than ever: {}.", better.join("; ")));
    }
    let luck = run.luck();
    let heard: u32 = run.results.iter().map(|r| r.shamrocks).sum();
    if heard > 0 {
        pages.push(format!(
            "Shamrocks: {luck} of {heard} caught, for {}.",
            luck_words(luck)
        ));
    }
    if saved.opened.is_none() && !all_cleared(run) {
        pages.push(
            "A tune earns its first star at half the points it can earn. A star on all three tunes opens the next set."
                .to_string(),
        );
    }
    pages
}

fn all_cleared(run: &Run) -> bool {
    run.results.iter().all(|r| r.stars > 0)
}

/// The pages after ceili night.
pub fn ceili_pages(run: &Run, saved: &CeiliSaved) -> Vec<String> {
    let mut first = format!(
        "Ceili night danced: the set dance of {} tunes. {}",
        run.results.len(),
        sentence_start(&score_words(saved.score, run.score()))
    );
    if run.clover() {
        first.push_str(" You found the four-leaf clover.");
    }
    let tunes: Vec<String> = run.results.iter().map(Floor::tune_line).collect();
    let k = saved.keepsake;
    let nights = match k.nights {
        1 => "danced once".to_string(),
        n => format!("danced {n} times"),
    };
    let clover = if k.clover {
        "four-leaf clover found"
    } else {
        "no clover yet"
    };
    vec![
        first,
        tunes.join(" "),
        format!("Steps: {}.", run.tally().words()),
        format!(
            "Your keepsake for Saint Patrick's Day {}: ceili night {nights}, best {}, {clover}, {}.",
            saved.year,
            k.best,
            luck_words(k.luck)
        ),
    ]
}

/// The results menu after a set.
pub fn after_set(n: u32, summary: String, pages: Vec<String>, saved: &SetSaved) -> Results {
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if let Some(next) = saved.next_open {
        let set = &SETS[(next as usize - 1).min(SETS.len() - 1)];
        results = results.next(
            format!("Next set: {}", set.title().to_lowercase()),
            set.scene,
            move |ctx| {
                let f = Floor::open(ctx, Mode::Set(next));
                ctx.replace(f)
            },
        );
    }
    results.again(
        labels::PLAY_AGAIN,
        "The same set again, with new calls.",
        move |ctx| {
            let f = Floor::open(ctx, Mode::Set(n));
            ctx.replace(f)
        },
    )
}

/// The results menu after practice.
pub fn after_practice(i: usize, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        "Practise again",
        "The same tune again, waiting for each step.",
        move |ctx| {
            let f = Floor::open(ctx, Mode::Practice(i));
            ctx.replace(f)
        },
    )
}

/// The results menu after ceili night.
pub fn after_ceili(summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        "The set dance again. Only your best score is kept.",
        |ctx| {
            let f = Floor::open(ctx, Mode::Ceili);
            ctx.replace(f)
        },
    )
}

/// The results menu after a challenge.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same tunes and calls, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            let f = Floor::challenge(ctx, ch);
            ctx.replace(f)
        },
    )
}

impl State<Parlor> for Floor {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::place_bed(ctx, self.mode.place());
        let intro = self.intro(ctx);
        ctx.say(intro);
        if let Some(news) = self.news(ctx) {
            ctx.say_queued(news);
        } else if !Progress::load(ctx).any() || matches!(self.mode, Mode::Practice(_)) {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.save_practice(ctx);
        self.stop_parts(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.run.finished() {
            return;
        }
        // Settings, from the pause menu, may have changed the call-outs.
        self.call_outs = crate::call_outs(ctx);
        sound::place_bed(ctx, self.mode.place());
        if !self.run.started() {
            let p = self.start_prompt(ctx);
            ctx.say(format!("Back on the floor. {p}"));
            return;
        }
        if let Some(i) = self.run.dance.waiting() {
            let step = self.run.dance.plan.calls[i].step;
            let line = Self::step_line(ctx, step);
            ctx.say(format!("Back on the floor. The tune is waiting. {line}"));
            return;
        }
        self.run.count_in();
        if self.run.dance.counting_in() {
            self.hold_parts(ctx);
            ctx.say("Back on the floor. Three beats in.");
        } else {
            ctx.say("Back on the floor.");
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.run.finished() || event.is_repeat() {
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
            Some(ParlorAction::Primary) if !self.run.started() => self.start(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status_layers(ctx);
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => self.repeat(ctx),
            Some(ParlorAction::Hint) => self.hint(ctx),
            Some(a) => {
                if let Some(step) = Step::for_action(a) {
                    self.dance(ctx, step);
                }
            }
            None => {
                if let InputEvent::KeyDown { key, .. } = event {
                    if !is_quiet_key(*key) {
                        ctx.play_ui(UiSound::Error);
                        ctx.say("Not a dance key. F1 lists the controls.");
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
                self.finish(ctx);
            }
            return;
        }
        self.play_due(ctx, dt);
        let evs = self.run.update(dt);
        for ev in evs {
            self.on_run(ctx, ev);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let run = &self.run;
        let d = &run.dance;
        let mut out = vec![self.mode.name()];
        out.push(format!(
            "Tune {} of {}: {}",
            run.at + 1,
            run.plans.len(),
            d.plan.tune.title()
        ));
        out.push(format!("Bar: {} of {TUNE_BARS}", d.bar()));
        if !run.practice() {
            out.push(format!("Score: {}", run.score()));
            out.push(format!("Streak: {}", d.streak));
        }
        if let Some(i) = d.last_call {
            out.push(format!("Last call: {}", d.plan.calls[i].step.word()));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let run = &self.run;
        let now = if !run.started() {
            self.start_prompt(ctx)
        } else if let Some(i) = run.dance.waiting() {
            let step = run.dance.plan.calls[i].step;
            format!("The tune is waiting for {}.", Self::step_line(ctx, step))
        } else if run.practice() {
            "Listen for the caller: each step falls on the beat after its call.".to_string()
        } else {
            format!(
                "Listen for the caller: each step falls on the beat after its call. Every {} in a row add {STREAK_BONUS} points.",
                crate::rules::STREAK_EVERY
            )
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

    /// F1 in a running dance stops the tune while the help is read, and
    /// counts back in after, so no step is missed while it speaks.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        let live = self.run.phase == Phase::Dancing
            && self.run.dance.music_started()
            && self.run.dance.waiting().is_none();
        if !live {
            parlor_core::explain(ctx, help);
            return;
        }
        self.run.count_in();
        self.hold_parts(ctx);
        let pages = parlor_core::help::pages_of(&format!("The tune is stopped. {help}"));
        ctx.push(Pages::new(pages));
    }
}

mod visuals_floor;
