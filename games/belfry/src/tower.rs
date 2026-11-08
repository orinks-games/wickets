//! The tower: building rows, ringing them and hearing the answers.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, ScoreResult, StatusKey};

use crate::challenge::{self, Job, CHALLENGES_NAME};
use crate::progress::{self, Saved};
use crate::rules::{
    bell_words, points, row_words, run_seed, Answer, Cue, Mode, Outcome, Peal, Refusal, Ringer,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const TOWER_NAME: &str = "belfry_tower";

/// How long after the called place's bell its shimmer follows, once
/// the bell's strike has spoken, so the two never blur.
pub const MOON_AFTER: f64 = 0.6;
/// How fast a shimmer still sounding fades as the player moves on.
const MOON_FADE_MS: u32 = 30;

/// Cues explained in words the first time they play.
const TAUGHT_KNOCK: &str = "knock";
const TAUGHT_TAP: &str = "tap";

pub struct Tower {
    pub peal: Peal,
    mode: Mode,
    pub ringer: Ringer,
    /// A challenge, peal after peal, and the challenge it counts for.
    job: Option<Job>,
    challenge: Option<Challenge>,
    taught: HashSet<&'static str>,
    last_prompt: String,
    /// S, layered: the row first, then the rest.
    status_key: StatusKey,
    /// The results are on their way: the peal is over.
    ended: bool,
    /// On a Practice night peal, the place the captain calls, from 0.
    pub lit: Option<usize>,
    /// Seconds until the shimmer follows the called place's bell.
    moon_wait: Option<f64>,
    /// The shimmer, while it may still be sounding.
    moon: Sound,
}

impl Tower {
    pub fn new(mode: Mode) -> Tower {
        Tower::with_seed(mode, run_seed())
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Tower {
        let mut peal = Peal::new(mode.rules(), seed);
        let lit = (mode == Mode::PracticeNight).then(|| (seed % peal.rules.places as u64) as usize);
        if let Some(place) = lit {
            peal.row[place] = peal.code[place];
        }
        peal.unsolve_start(lit);
        Tower {
            peal,
            mode,
            ringer: Ringer::new(),
            job: None,
            challenge: None,
            taught: HashSet::new(),
            last_prompt: String::new(),
            status_key: StatusKey::new(),
            ended: false,
            lit,
            moon_wait: None,
            moon: Sound::NONE,
        }
    }

    /// The challenge's peals, from its seed.
    pub fn challenge(ch: Challenge) -> Tower {
        let job = Job::new(&ch);
        Tower {
            peal: job.peal(),
            mode: Mode::Village,
            ringer: Ringer::new(),
            job: Some(job),
            challenge: Some(ch),
            taught: HashSet::new(),
            last_prompt: String::new(),
            status_key: StatusKey::new(),
            ended: false,
            lit: None,
            moon_wait: None,
            moon: Sound::NONE,
        }
    }

    pub fn is_challenge(&self) -> bool {
        self.job.is_some()
    }

    fn bells(&self) -> u8 {
        self.peal.rules.bells
    }

    /// "Place 2: bell 4, middle low.", and "Place 3, called: ..." on the
    /// place the captain calls.
    fn place_words(&self) -> String {
        let moon = if self.on_lit() { ", called" } else { "" };
        format!(
            "Place {}{moon}: {}.",
            self.peal.place + 1,
            bell_words(self.peal.bell(), self.bells()).to_lowercase()
        )
    }

    /// Hushes the shimmer as the player acts: one still waiting never
    /// plays, and one still sounding fades out at once.
    fn hush_moon(&mut self, ctx: &mut Ctx<Parlor>) {
        self.moon_wait = None;
        if self.moon != Sound::NONE {
            ctx.audio.stop(self.moon, MOON_FADE_MS);
            self.moon = Sound::NONE;
        }
    }

    /// Whether the cursor is on the place the captain called.
    fn on_lit(&self) -> bool {
        self.lit == Some(self.peal.place)
    }

    /// "The captain calls place 3: bell 4, middle low. ..."
    fn moon_words(&self) -> Option<String> {
        let place = self.lit?;
        Some(format!(
            "The captain calls place {}: {}, already set. A shimmer marks it.",
            place + 1,
            bell_words(self.peal.code[place], self.bells()).to_lowercase()
        ))
    }

    fn rows_left_words(&self) -> String {
        match self.peal.rows_left() {
            1 => "1 row left".to_string(),
            n => format!("{n} rows left"),
        }
    }

    /// What the tower is, as play starts.
    fn intro(&self) -> String {
        let rules = self.peal.rules.describe();
        match &self.job {
            Some(job) if job.peals() > 1 => format!(
                "{}. {}",
                challenge::name(job.period),
                challenge::next_words(job)
            ),
            Some(job) => format!("{}. Find the peal: {rules}.", challenge::name(job.period)),
            None => format!("{}. Find the peal: {rules}.", self.mode.name()),
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
        let rep = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        let hint = if self.peal.rules.hints {
            format!(", {h} a hint")
        } else {
            String::new()
        };
        format!(
            "{} and {r} move between places. {} and {d} change the bell. {p} rings the row, {t} plays it back. {s} gives the status, {rep} the row log{hint}. F1 helps, Escape pauses.",
            sentence_start(&l),
            sentence_start(&u)
        )
    }

    /// The status, in two layers for S. First the row: its number, the
    /// place, the row being built and the last answer. Then the rest: rows
    /// left, the captain's call, hints, and the challenge's peal.
    pub fn status_layers(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let p = &self.peal;
        let mut first = format!(
            "Row {} of {}, place {}, your row {}.",
            (p.rows_used() + 1).min(p.rules.rows),
            p.rules.rows,
            p.place + 1,
            row_words(&p.row)
        );
        if let Some(answer) = p.last_answer() {
            first.push_str(&format!(" Last: {}.", answer.words_for(p.rules.places)));
        }
        let mut parts = vec![self.rows_left_words()];
        if let Some(place) = self.lit {
            parts.push(format!(
                "the captain called place {}, bell {}",
                place + 1,
                p.code[place]
            ));
        }
        match p.hints_used {
            0 if p.rules.hints && p.rows_left() >= 2 => parts.push(format!(
                "{} trades a row for a hint",
                ctx.control_name(ParlorAction::Hint)
            )),
            0 => {}
            1 => parts.push("1 hint used".to_string()),
            n => parts.push(format!("{n} hints used")),
        }
        if let Some(job) = &self.job {
            if job.peals() > 1 {
                parts.push(format!(
                    "peal {} of {}, {} banked",
                    job.index + 1,
                    job.peals(),
                    points(job.banked)
                ));
            }
        }
        vec![first, format!("{}.", sentence_start(&parts.join(", ")))]
    }

    /// The first layer of the status: where the row stands.
    pub fn status(&self, ctx: &Ctx<Parlor>) -> String {
        self.status_layers(ctx).remove(0)
    }

    fn prompt(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say_queued(text.clone());
        self.last_prompt = text;
    }

    /// The words after a rung row: the answer, then the rows left, and the
    /// meaning of knocks and taps the first time each is heard.
    fn answer_words(&mut self, answer: Answer) -> String {
        let n = self.peal.rows_used();
        let mut text = format!(
            "Row {n}: {}. {}.",
            answer.words(),
            sentence_start(&self.rows_left_words())
        );
        if answer.in_place > 0 && self.taught.insert(TAUGHT_KNOCK) {
            text.push_str(" A knock is a right bell in its place.");
        }
        if answer.misplaced > 0 && self.taught.insert(TAUGHT_TAP) {
            text.push_str(" A tap is a right bell in the wrong place.");
        }
        text
    }

    fn play_cues(&mut self, ctx: &mut Ctx<Parlor>, cues: Vec<Cue>) {
        for cue in cues {
            match cue {
                Cue::Bell(b) => {
                    sound::play(ctx, sound::bell(b));
                }
                Cue::Knock => {
                    sound::play(ctx, sound::KNOCK);
                }
                Cue::Tap => {
                    sound::play(ctx, sound::TAP);
                }
                Cue::Speak(text) => {
                    ctx.say(text.clone());
                    self.last_prompt = text;
                }
            }
        }
    }

    /// A key while a row rings: the rest of it is skipped and its answer
    /// is spoken at once.
    fn skip_ringing(&mut self, ctx: &mut Ctx<Parlor>) {
        let spoken: Vec<Cue> = self
            .ringer
            .flush()
            .into_iter()
            .filter(|c| matches!(c, Cue::Speak(_)))
            .collect();
        if spoken.is_empty() && !self.peal.finished() {
            sound::play(ctx, sound::NOPE);
            ctx.say("Skipped.");
        } else {
            self.play_cues(ctx, spoken);
        }
        self.check_end(ctx);
    }

    fn ring(&mut self, ctx: &mut Ctx<Parlor>) {
        let bells = self.peal.row.clone();
        match self.peal.ring() {
            Ok(answer) => {
                sound::play(ctx, sound::ROPE);
                let speak = (!self.peal.finished()).then(|| self.answer_words(answer));
                self.ringer.row(&bells, Some(answer), speak);
                let due = self.ringer.due();
                self.play_cues(ctx, due);
            }
            Err(_) => {
                sound::play(ctx, sound::NOPE);
                ctx.say("The peal is over.");
            }
        }
    }

    fn play_back(&mut self, ctx: &mut Ctx<Parlor>) {
        let bells = self.peal.row.clone();
        let text = format!("Your row: {}.", row_words(&bells));
        self.ringer.row(&bells, None, Some(text));
        let due = self.ringer.due();
        self.play_cues(ctx, due);
    }

    fn repeat(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.peal.log.is_empty() {
            let p = ctx.control_name(ParlorAction::Primary);
            ctx.say(format!(
                "No rows rung yet. {} rings your row.",
                sentence_start(&p)
            ));
            return;
        }
        let log = self.peal.log_lines().join(" ");
        match self.peal.last_answer() {
            Some(answer) if answer.in_place + answer.misplaced > 0 => {
                self.ringer.answer(answer, Some(log));
                let due = self.ringer.due();
                self.play_cues(ctx, due);
            }
            _ => ctx.say(log),
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.peal.hint() {
            Ok(hint) => {
                sound::play(ctx, sound::HINT);
                let text = format!(
                    "Hint: {}. It used a row: {}.",
                    hint.words().to_lowercase(),
                    self.rows_left_words()
                );
                ctx.say(text.clone());
                self.last_prompt = text;
            }
            Err(why) => {
                sound::play(ctx, sound::NOPE);
                ctx.say(match why {
                    Refusal::HintsOff => "No hints in the cathedral.",
                    Refusal::HintNeedsRows => "No hint: it would use your last row.",
                    Refusal::NothingToHint => "No hint: every bell has had its hint.",
                    Refusal::Finished => "The peal is over.",
                });
            }
        }
    }

    /// Once a finished peal's last row has rung out, the results follow.
    fn check_end(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended || !self.peal.finished() || self.ringer.busy() {
            return;
        }
        if self.job.is_some() {
            self.finish_job_peal(ctx);
        } else {
            self.finish(ctx);
        }
    }

    fn stinger(&self, ctx: &mut Ctx<Parlor>, solved: bool) {
        ctx.audio.set_bed(None);
        let key = if solved { sound::SOLVED } else { sound::LOST };
        ctx.audio.play_music(Some(key), false);
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        let solved = self.peal.solved();
        self.stinger(ctx, solved);
        let saved = progress::save_end(ctx, self.mode, &self.peal);
        let pages = result_pages(&self.peal, &saved);
        let summary = result_summary(&self.peal);
        let mut results = Some(after_peal(self.mode, solved, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    /// A challenge peal ended: on to the next, or the challenge is over.
    fn finish_job_peal(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(job) = self.job.as_mut() else {
            return;
        };
        if let Some(next) = job.advance(&self.peal) {
            let done = challenge::peal_words(&self.peal);
            let banked = job.banked;
            // After the first peal the bank is just that score: say it once.
            let first = banked == self.peal.score();
            let intro = challenge::next_words(job);
            self.peal = next;
            sound::play(ctx, sound::NEXT_PEAL);
            if first {
                ctx.say(done);
            } else {
                ctx.say(format!("{done} {} banked.", points(banked)));
            }
            ctx.say_queued(intro);
            let p = self.place_words();
            self.prompt(ctx, p);
            return;
        }
        self.ended = true;
        let (Some(job), Some(ch)) = (self.job.take(), self.challenge.take()) else {
            return;
        };
        self.stinger(ctx, self.peal.solved());
        let pages = challenge::finish_job(ctx, &ch, &job, &self.peal);
        let summary = format!("{} result: {}", ch.title(), points(job.score()));
        let mut results = Some(after_job(ch.period, summary, pages.clone()));
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }
}

/// Keys a player presses for their screen reader or as a modifier, which
/// the tower leaves alone.
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

/// The results menu's title.
pub fn result_summary(peal: &Peal) -> String {
    match peal.outcome {
        Some(Outcome::Solved) => {
            let rows = peal.rows_used();
            format!(
                "Solved in {rows} {}. {}",
                if rows == 1 { "row" } else { "rows" },
                points(peal.score())
            )
        }
        _ => format!("Out of rows. The peal was {}", row_words(&peal.code)),
    }
}

/// The pages after a peal ends.
pub fn result_pages(peal: &Peal, saved: &Saved) -> Vec<String> {
    let rows = peal.rows_used();
    let mut first = if peal.solved() {
        format!(
            "Solved in {rows} {}. The peal is {}, and the bells ring it out.",
            if rows == 1 { "row" } else { "rows" },
            row_words(&peal.code)
        )
    } else {
        format!(
            "Out of rows. The peal was {}. Your last row had {}.",
            row_words(&peal.code),
            peal.last_answer()
                .unwrap_or_default()
                .words()
                .to_lowercase()
        )
    };
    match saved.score {
        Some(ScoreResult::First) => first.push_str(&format!(
            " {}: your first score here.",
            points(peal.score())
        )),
        Some(ScoreResult::NewBest { previous }) => first.push_str(&format!(
            " {}: a new best, beating {previous}.",
            points(peal.score())
        )),
        Some(ScoreResult::Short { best }) => {
            first.push_str(&format!(" {}. Your best is {best}.", points(peal.score())))
        }
        None => {}
    }
    let mut second = Vec::new();
    if peal.solved() {
        match (saved.fewest_record, saved.previous_fewest) {
            (true, Some(prev)) => second.push(format!("Fewest rows: {rows}, beating {prev}.")),
            (true, None) => second.push(format!("Fewest rows: {rows}, your first solve.")),
            (false, Some(prev)) => second.push(format!("Your fewest rows is {prev}.")),
            _ => {}
        }
        second.push(match saved.streak {
            1 => "Streak: 1 peal solved.".to_string(),
            n => format!("Streak: {n} peals solved in a row."),
        });
    } else {
        second.push("Your solve streak starts again.".to_string());
    }
    if peal.hints_used > 0 {
        second.push(match peal.hints_used {
            1 => "1 hint used.".to_string(),
            n => format!("{n} hints used."),
        });
    }
    vec![first, second.join(" ")]
}

/// The results menu after a peal: the same mode again, the results
/// again, and back to the title.
pub fn after_peal(mode: Mode, solved: bool, summary: String, pages: Vec<String>) -> Results {
    let again = if solved {
        labels::PLAY_AGAIN
    } else {
        labels::TRY_AGAIN
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        again,
        mode.again_help(),
        move |ctx| ctx.replace(Tower::new(mode)),
    )
}

/// The results menu after a challenge: the challenge again, the results
/// again, and back to the challenges.
pub fn after_job(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same peals, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Tower::challenge(ch))
        },
    )
}

impl State<Parlor> for Tower {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        TOWER_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::tower_bed(ctx);
        let intro = self.intro();
        ctx.say(intro);
        if let Some(moon) = self.moon_words() {
            self.moon = sound::play(ctx, sound::MOON);
            ctx.say_queued(moon);
        }
        // The full controls the first time; after a finished peal, F1 has them.
        if progress::Progress::load(ctx).any_played() {
            // Knocks and taps were taught on the first peal.
            self.taught.insert(TAUGHT_KNOCK);
            self.taught.insert(TAUGHT_TAP);
            ctx.say_queued("F1 lists the controls.");
        } else {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        }
        let p = self.place_words();
        self.prompt(ctx, p);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush_moon(ctx);
        ctx.audio.set_bed(None);
        // A peal left after ringing a row counts as not solved.
        if self.job.is_none() && !self.ended && !self.peal.finished() && self.peal.rows_used() > 0 {
            self.ended = true;
            progress::save_abandoned(ctx, self.mode);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        sound::tower_bed(ctx);
        // Paused mid-ring: the answer is still to come.
        if self.ringer.busy() {
            ctx.say("Back in the tower. The row rings on.");
            return;
        }
        let s = self.status(ctx);
        ctx.say(format!("Back in the tower. {s}"));
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
            let leave = if self.is_challenge() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            self.hush_moon(ctx);
            menus::pause(ctx, HELP, leave);
            return;
        }
        let stepping = matches!(
            action,
            Some(ParlorAction::Left | ParlorAction::Right | ParlorAction::Up | ParlorAction::Down)
        );
        if event.is_repeat() && !stepping {
            return;
        }
        if action.is_some() {
            self.hush_moon(ctx);
        }
        // Any key skips a ringing row, as help says, not only bound ones.
        let any_key = action.is_some()
            || matches!(event, InputEvent::KeyDown { key, .. } if !is_quiet_key(*key));
        if any_key && self.ringer.busy() {
            if !event.is_repeat() {
                self.skip_ringing(ctx);
            }
            return;
        }
        if self.peal.finished() {
            return;
        }
        match action {
            Some(ParlorAction::Left | ParlorAction::Right) => {
                self.peal.move_place(action == Some(ParlorAction::Right));
                sound::play(ctx, sound::bell(self.peal.bell()));
                if self.on_lit() {
                    self.moon_wait = Some(MOON_AFTER);
                }
                let text = self.place_words();
                ctx.say(text);
            }
            Some(ParlorAction::Up | ParlorAction::Down) => {
                let b = self.peal.change_bell(action == Some(ParlorAction::Up));
                sound::play(ctx, sound::bell(b));
                ctx.say(format!("{}.", bell_words(b, self.bells())));
            }
            Some(ParlorAction::Primary) => self.ring(ctx),
            Some(ParlorAction::Secondary) => self.play_back(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status_layers(ctx);
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => self.repeat(ctx),
            Some(ParlorAction::Hint) => self.hint(ctx),
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            self.hush_moon(ctx);
            return;
        }
        if let Some(wait) = self.moon_wait.as_mut() {
            *wait -= dt;
            if *wait <= 0.0 {
                self.moon_wait = None;
                self.moon = sound::play(ctx, sound::MOON);
            }
        }
        self.peal.update(dt);
        let cues = self.ringer.update(dt);
        self.play_cues(ctx, cues);
        self.check_end(ctx);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let p = &self.peal;
        let mut out = vec![match &self.job {
            Some(job) => format!(
                "{}, peal {} of {}",
                challenge::name(job.period),
                job.index + 1,
                job.peals()
            ),
            None => format!("Belfry: {}", self.mode.name()),
        }];
        out.push(format!(
            "Row {} of {}, place {}: {}",
            (p.rows_used() + 1).min(p.rules.rows),
            p.rules.rows,
            p.place + 1,
            row_words(&p.row)
        ));
        out.extend(p.log_lines());
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let mut now = format!(
            "{} Build a row of {} bells, then {p} rings it. Knocks are right bells in their place, taps are right bells in the wrong place.",
            self.status_layers(ctx).join(" "),
            self.peal.rules.places
        );
        if let Some(moon) = self.moon_words() {
            now.push(' ');
            now.push_str(&moon);
        }
        if let Some(job) = &self.job {
            now.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count it.",
                job.period.label().to_lowercase(),
                challenge::name(job.period)
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

mod visuals_tower;
