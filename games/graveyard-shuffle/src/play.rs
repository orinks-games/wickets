//! The graveyard dance floor: the play screen for every mode.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, Rng, ScoreResult, StatusKey};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::progress::{self, Progress, SetSaved};
use crate::rules::{
    beats_words, points_words, stars_words, steps_words, Ev, Judgment, Phase, Plan, Run, RunEv,
    SongResult, TUNE_BARS,
};
use crate::songs::{night_of, Monster, Night, Place, NIGHTS, SONGS};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "graveyard_shuffle_play";
/// Seconds between the calls of the before-dance hint.
pub const HINT_GAP: f64 = 1.2;
/// Seconds from the end of a run to its results, for the applause.
pub const END_PAUSE: f64 = 2.5;

/// What the floor is dancing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A night of three songs, by number from 1.
    Set(u32),
    /// The practice room: one song, by index into `SONGS`, that waits for
    /// each monster's move.
    Practice(usize),
    Challenge(Period),
}

impl Mode {
    pub fn name(self) -> String {
        match self {
            Mode::Set(n) => NIGHTS[(n as usize - 1).min(NIGHTS.len() - 1)].title(),
            Mode::Practice(_) => "Practice room".to_string(),
            Mode::Challenge(period) => challenge::name(period).to_string(),
        }
    }

    /// The name for the first layer of S: "Night 1".
    pub fn short_name(self) -> String {
        match self {
            Mode::Set(n) => format!("Night {n}"),
            _ => self.name(),
        }
    }

    pub fn place(self) -> Place {
        match self {
            Mode::Set(n) => NIGHTS[(n as usize - 1).min(NIGHTS.len() - 1)].place,
            Mode::Practice(i) => night_of(i).place,
            Mode::Challenge(_) => Place::Church,
        }
    }
}

/// The songs a mode dances, from `seed`.
pub fn plans(mode: Mode, seed: u64) -> Vec<Plan> {
    let mut rng = Rng::seeded(seed);
    match mode {
        Mode::Set(n) => {
            let set = &NIGHTS[(n as usize - 1).min(NIGHTS.len() - 1)];
            set.songs()
                .into_iter()
                .map(|t| Plan::new(t, &set.level, &mut rng))
                .collect()
        }
        Mode::Practice(i) => {
            let i = i.min(SONGS.len() - 1);
            vec![Plan::new(&SONGS[i], &night_of(i).level, &mut rng)]
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
    /// Monsters already named on this save, which the first call leaves
    /// unsaid.
    met: HashSet<Monster>,
    /// Monsters the intro's "New tonight" line named, which their first
    /// call leaves unsaid.
    announced: HashSet<Monster>,
    status_key: StatusKey,
    /// The parts of the song still sounding, to hold in a wait or a count.
    parts: Vec<Sound>,
    /// The before-dance hint's monsters still to play: seconds to wait, monster.
    delayed: Vec<(f64, Monster)>,
    ending: Option<f64>,
    practice_unsaved: u32,
    call_outs: bool,
}

impl Floor {
    /// A run of `mode`, from the clock's seed.
    pub fn open(ctx: &mut Ctx<Parlor>, mode: Mode) -> Floor {
        Floor::with_seed(ctx, mode, run_seed())
    }

    /// A run of `mode` from `seed`.
    pub fn with_seed(ctx: &mut Ctx<Parlor>, mode: Mode, seed: u64) -> Floor {
        let run = Run::new(plans(mode, seed), matches!(mode, Mode::Practice(_)));
        let met = progress::met(ctx);
        let met = Monster::ALL
            .into_iter()
            .filter(|m| met.contains(m.word()))
            .collect();
        Floor {
            run,
            mode,
            challenge: None,
            taught: HashSet::new(),
            met,
            announced: HashSet::new(),
            status_key: StatusKey::new(),
            parts: Vec::new(),
            delayed: Vec::new(),
            ending: None,
            practice_unsaved: 0,
            call_outs: crate::call_outs(ctx),
        }
    }

    /// A run of the challenge's songs.
    pub fn challenge(ctx: &mut Ctx<Parlor>, ch: Challenge) -> Floor {
        let mut f = Floor::with_seed(ctx, Mode::Challenge(ch.period), ch.seed);
        f.challenge = Some(ch);
        f
    }

    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    /// Whether to name `monster` as it comes up: once per save, and never in
    /// practice, where the wait names its move.
    fn meet(&mut self, ctx: &mut Ctx<Parlor>, monster: Monster) -> bool {
        if self.run.practice() || !self.met.insert(monster) {
            return false;
        }
        progress::save_met(ctx, monster);
        !self.announced.contains(&monster)
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    fn step_key(ctx: &Ctx<Parlor>, step: Monster) -> String {
        ctx.control_name(step.action())
    }

    /// "Zombie, the drag: Down arrow."
    fn step_line(ctx: &Ctx<Parlor>, step: Monster) -> String {
        format!(
            "{}, {}: {}.",
            step.word(),
            step.dance(),
            Self::step_key(ctx, step)
        )
    }

    fn start_prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} starts the dance. {} plays every monster and its move.",
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
            Mode::Set(_) => format!("3 songs: {}.", self.tune_names()),
            Mode::Practice(_) => format!(
                "{}. The song waits for each monster until you dance its move.",
                self.run.plans[0].tune.name
            ),
            Mode::Challenge(_) => {
                if n == 1 {
                    format!("1 song, {}.", self.run.plans[0].tune.name)
                } else {
                    format!("{n} songs: {}.", self.tune_names())
                }
            }
        };
        format!("{}. {what} {}", self.mode.name(), self.start_prompt(ctx))
    }

    /// The second line of the intro: who is new this night and not yet
    /// met on this save. Their first call then leaves them unsaid.
    fn news(&mut self, ctx: &Ctx<Parlor>) -> Option<String> {
        let Mode::Set(n) = self.mode else {
            return None;
        };
        if n == 1 {
            return None;
        }
        let set = &NIGHTS[(n as usize - 1).min(NIGHTS.len() - 1)];
        let new: Vec<Monster> = set
            .new_monsters()
            .iter()
            .copied()
            .filter(|m| !self.met.contains(m))
            .collect();
        if new.is_empty() {
            return None;
        }
        self.announced.extend(new.iter().copied());
        let words: Vec<String> = new
            .iter()
            .map(|m| format!("{}, {}", m.lower(), Self::step_key(ctx, *m)))
            .collect();
        Some(format!("New tonight: {}.", words.join(", and ")))
    }

    /// The monsters this mode can bring up, in their usual order.
    fn monsters(&self) -> Vec<Monster> {
        let night = |n: &Night| n.level.monsters.to_vec();
        match self.mode {
            Mode::Set(n) => night(&NIGHTS[(n as usize - 1).min(NIGHTS.len() - 1)]),
            Mode::Practice(i) => night(night_of(i)),
            Mode::Challenge(_) => self.steps_in_run(),
        }
    }

    /// The beat rule and the moves of the monsters in this mode.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let moves: Vec<String> = self
            .monsters()
            .iter()
            .map(|m| format!("{}, {}", m.word().to_lowercase(), Self::step_key(ctx, *m)))
            .collect();
        format!(
            "Move on the beat after each call. {}.",
            sentence_start(&moves.join("; "))
        )
    }

    /// What F1 adds in a challenge.
    fn challenge_line(&self) -> Option<String> {
        self.challenge.as_ref().map(|ch| {
            format!(
                "This is the {} challenge, {}. Leaving from the pause menu does not count the run.",
                ch.period.label().to_lowercase(),
                challenge::name(ch.period)
            )
        })
    }

    /// The other keys of the floor.
    pub fn keys(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "{} speaks the status, {} repeats the last monster, {} names the next one, and Escape pauses.",
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Repeat),
            k(ParlorAction::Hint)
        )
    }

    /// The status layers, most urgent first: S reads the first, and S
    /// again the next.
    pub fn status(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let line = self.status_line(ctx);
        match self.run.phase {
            Phase::Ready => vec![line, self.start_prompt(ctx)],
            _ => vec![line],
        }
    }

    /// The first layer of the status, in a fixed order.
    fn status_line(&self, ctx: &Ctx<Parlor>) -> String {
        let run = &self.run;
        let name = self.mode.name();
        let n = run.plans.len();
        match run.phase {
            Phase::Ready => {
                let tunes = if n == 1 {
                    "1 song".to_string()
                } else {
                    format!("{n} songs")
                };
                format!("{}, {tunes}, not started.", self.mode.short_name())
            }
            Phase::Between(_) => format!(
                "{name}, song {} of {n} done, score {}.",
                run.at + 1,
                run.score()
            ),
            Phase::Done => format!("{name}, every song danced, score {}.", run.score()),
            Phase::Dancing => {
                let d = &run.dance;
                let mut parts = Vec::new();
                if let Some(i) = d.waiting() {
                    let step = d.plan.calls[i].step;
                    parts.push(format!(
                        "Waiting for {}, {}",
                        step.dance(),
                        Self::step_key(ctx, step)
                    ));
                }
                parts.push(d.plan.tune.name.to_string());
                if n > 1 {
                    parts.push(format!("song {} of {n}", run.at + 1));
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
                }
                format!("{}.", parts.join(", "))
            }
        }
    }

    fn hint_all(&mut self, ctx: &mut Ctx<Parlor>) {
        let steps: Vec<Monster> = self.steps_in_run();
        let lines: Vec<String> = steps.iter().map(|s| Self::step_line(ctx, *s)).collect();
        ctx.say(lines.join(" "));
        self.delayed = steps
            .into_iter()
            .enumerate()
            .map(|(i, s)| (i as f64 * HINT_GAP, s))
            .collect();
        self.play_due(ctx, 0.0);
    }

    /// The monsters that come up in this run, in their usual order.
    fn steps_in_run(&self) -> Vec<Monster> {
        Monster::ALL
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
    fn preview(&mut self, ctx: &mut Ctx<Parlor>, step: Monster) {
        sound::play_call(ctx, step);
        sound::play_step(ctx, step, 0.8);
        let used = self.steps_in_run().contains(&step);
        let line = Self::step_line(ctx, step);
        if used {
            ctx.say(line);
        } else {
            ctx.say(format!("{line} It does not come up in these songs."));
        }
    }

    fn start(&mut self, ctx: &mut Ctx<Parlor>) {
        self.delayed.clear();
        self.run.start();
        ctx.say("Here we go. Four knocks in.");
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
            Ev::Call { step, .. } => {
                sound::play_call(ctx, step);
                if self.call_outs {
                    ctx.say(step.word());
                } else if self.meet(ctx, step) {
                    // The first time each monster comes up, name it and its key.
                    ctx.say(format!("{}, {}.", step.word(), Self::step_key(ctx, step)));
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
                    ctx.say(format!(
                        "Missed. That was {}, {}.",
                        step.lower(),
                        step.dance()
                    ));
                }
            }
            Ev::Wrong { want, got, .. } => {
                sound::play(ctx, sound::STUMBLE);
                if self.run.practice() {
                    let key = Self::step_key(ctx, want);
                    ctx.say(format!(
                        "Not {}. {}, {}: {key}.",
                        got.dance(),
                        want.word(),
                        want.dance()
                    ));
                } else if self.first("wrong") {
                    ctx.say(format!(
                        "Wrong move. That was {}, {}.",
                        want.lower(),
                        want.dance()
                    ));
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

    fn tune_line(r: &SongResult) -> String {
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
                    let next = format!("Next, {}.", next.tune.name);
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

    fn dance(&mut self, ctx: &mut Ctx<Parlor>, step: Monster) {
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
                ctx.say(format!(
                    "Last up: {}, {}, {key}.",
                    step.lower(),
                    step.dance()
                ));
            }
            _ => ctx.say("No monster yet in this song."),
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.run.started() {
            self.hint_all(ctx);
            return;
        }
        if self.run.phase != Phase::Dancing {
            ctx.say("Between songs. The next one starts soon.");
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
                ctx.say(format!(
                    "Next: {}, {}, {key}, {when}.",
                    step.lower(),
                    step.dance()
                ));
            }
            None => ctx.say("No more monsters in this song."),
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
                    SONGS[i].name,
                    sentence_start(&steps_words(hits))
                );
                let pages = vec![summary.clone()];
                (pages.clone(), after_practice(i, summary, pages))
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
        "Night {n} danced. {}, {}.",
        sentence_start(&points_words(run.score())),
        stars_words(stars)
    )
}

/// The pages after a set.
pub fn set_pages(n: u32, run: &Run, saved: &SetSaved) -> Vec<String> {
    let set = &NIGHTS[(n as usize - 1).min(NIGHTS.len() - 1)];
    let mut first = format!(
        "{}, danced. {}",
        set.title(),
        sentence_start(&score_words(saved.score, run.score()))
    );
    if let Some(open) = saved.opened {
        let next = &NIGHTS[(open as usize - 1).min(NIGHTS.len() - 1)];
        first.push_str(&format!(" {} is open.", next.title()));
    }
    if let Some(w) = saved.waiting {
        first.push_str(&format!(" {}", crate::picker::waits_words(w)));
    }
    if saved.kept {
        let kept = if saved.in_season {
            crate::KEPT_LINE
        } else {
            crate::KEPT_LINE_OUT_OF_SEASON
        };
        first.push_str(&format!(" {kept}"));
    }
    let tunes: Vec<String> = run.results.iter().map(Floor::tune_line).collect();
    let mut pages = vec![first, tunes.join(" ")];
    let mut steps = format!("Moves: {}.", run.tally().words());
    if saved.streak_record {
        steps.push_str(&format!(
            " Longest streak: {}, your best.",
            saved.best_streak
        ));
    } else {
        steps.push_str(&format!(" Longest streak: {}.", run.best_streak()));
    }
    pages.push(steps);
    // On a first score every song is better than ever, as page 2 just said.
    if !saved.better.is_empty() && saved.score != ScoreResult::First {
        let better: Vec<String> = saved
            .better
            .iter()
            .map(|(name, s)| format!("{name}, {}", stars_words(*s)))
            .collect();
        pages.push(format!("More stars than ever: {}.", better.join("; ")));
    }
    if saved.opened.is_none() && saved.waiting.is_none() && !all_cleared(run) {
        pages.push(
            "A song earns its first star at half the points it can earn. A star on all three songs opens the next night."
                .to_string(),
        );
    }
    pages
}

fn all_cleared(run: &Run) -> bool {
    run.results.iter().all(|r| r.stars > 0)
}

/// The results menu after a set.
pub fn after_set(n: u32, summary: String, pages: Vec<String>, saved: &SetSaved) -> Results {
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if let Some(next) = saved.next_open {
        let set = &NIGHTS[(next as usize - 1).min(NIGHTS.len() - 1)];
        results = results.next(
            format!("Next night: {}", set.title_in_sentence()),
            set.scene,
            move |ctx| {
                let f = Floor::open(ctx, Mode::Set(next));
                ctx.replace(f)
            },
        );
    }
    results.again(
        labels::PLAY_AGAIN,
        "The same night again, with new monsters coming up.",
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
        "The same song again, waiting for each move.",
        move |ctx| {
            let f = Floor::open(ctx, Mode::Practice(i));
            ctx.replace(f)
        },
    )
}

/// The results menu after a challenge.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same songs and monsters, straight away. Only your best result is kept.",
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
        // Help pauses everything; the pause menu has already resumed it.
        ctx.audio.resume_all();
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
            ctx.say(format!("Back on the floor. The song is waiting. {line}"));
            return;
        }
        self.run.count_in();
        if self.run.dance.counting_in() {
            self.hold_parts(ctx);
            ctx.say("Back on the floor. Three knocks in.");
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
                let layers = self.status(ctx);
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => self.repeat(ctx),
            Some(ParlorAction::Hint) => self.hint(ctx),
            Some(a) => {
                if let Some(step) = Monster::for_action(a) {
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
            "Song {} of {}: {}",
            run.at + 1,
            run.plans.len(),
            d.plan.tune.name
        ));
        out.push(format!("Bar: {} of {TUNE_BARS}", d.bar()));
        if !run.practice() {
            out.push(format!("Score: {}", run.score()));
            out.push(format!("Streak: {}", d.streak));
        }
        if let Some(i) = d.last_call {
            out.push(format!("Last up: {}", d.plan.calls[i].step.word()));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let run = &self.run;
        let controls = self.controls(ctx);
        if !run.started() {
            let mut text = format!("{} {controls} {}", self.start_prompt(ctx), self.keys(ctx));
            if let Some(line) = self.challenge_line() {
                text.push_str(&format!(" {line}"));
            }
            return text;
        }
        match run.dance.waiting() {
            Some(i) => {
                let step = run.dance.plan.calls[i].step;
                format!(
                    "The song is waiting for {}, {}: {}. {controls}",
                    step.lower(),
                    step.dance(),
                    Self::step_key(ctx, step)
                )
            }
            None => controls,
        }
    }

    /// While the song plays, F1 pauses it and reads the help a page at a
    /// time, so no monster comes up while the player listens.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        if !self.run.started() || self.run.finished() || self.ending.is_some() {
            parlor_core::explain(ctx, help);
            return;
        }
        let mut pages = parlor_core::help::pages_of(&help);
        pages.extend(parlor_core::help::pages_of(&self.keys(ctx)));
        pages.extend(self.challenge_line());
        ctx.audio.pause_all();
        ctx.push(Pages::new(pages));
    }
}

mod visuals_floor;
