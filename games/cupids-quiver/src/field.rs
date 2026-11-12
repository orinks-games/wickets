//! The sky over the rooftops during play, and the results after a run.

use std::cell::Cell;
use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Rng, ScoreResult};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::notes::NOTES;
use crate::progress::{self, Progress, Saved};
use crate::rules::{
    aim_words, arrows_words, hearts_words, offset_words, turn_words, Event, Kind, Mode, Phase,
    Range, Refusal, Release, Run, Target, Twist, AIM_LIMIT, AIM_STEP, GOLDEN_POINTS, ROSE_BONUS,
};
use crate::sound::{self, sky_pos};
use crate::valentine;
use crate::{HELP, TITLE_NAME};

/// The play screen's `State::name`.
pub const FIELD_NAME: &str = "cupids_quiver_sky";

/// A passing line waits at most this long behind an important one.
const CHATTER_WAIT: f64 = 0.8;

/// In the tutorial, a heart's partner waits this long once the sky is
/// empty, so the hit that ends the lesson is the last thing said.
const LESSON_PARTNER_GAP: f64 = 1.5;

/// Cues the tutorial teaches in its own prompts. A player who has done the
/// tutorial or finished a run is not taught them again in play, and inside
/// the tutorial the field leaves them to the lesson.
const BASICS: [&str; 5] = ["heart", "full", "sight", "miss", "slack"];

/// Cues met later in play, each explained the first time only.
const LATER: [&str; 5] = ["rose", "golden", "letter", "pair", "sweetheart"];

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Makes every run started from the menus use `seed`, for tests. `None`
/// goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new run: the fixed one in tests, else the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(Cell::get)
        .unwrap_or_else(|| Rng::from_time().next_u64())
}

/// About how long a line takes to say.
pub fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// The controls in one line, naming the player's own keys.
pub fn controls_line(ctx: &Ctx<Parlor>) -> String {
    let l = ctx.control_name(ParlorAction::Left);
    let r = ctx.control_name(ParlorAction::Right);
    let p = ctx.control_name(ParlorAction::Primary);
    let t = ctx.control_name(ParlorAction::Secondary);
    let s = ctx.control_name(ParlorAction::Status);
    let h = ctx.control_name(ParlorAction::Hint);
    format!(
        "{} and {r} aim. Hold {p} past the full draw cue, then let go at the sights cue. {} lists the sky, {s} the status, {h} a hint.",
        sentence_start(&l),
        sentence_start(&t)
    )
}

/// The full controls, for F1.
pub fn controls_full(ctx: &Ctx<Parlor>) -> String {
    let u = ctx.control_name(ParlorAction::Up);
    let d = ctx.control_name(ParlorAction::Down);
    let rep = ctx.control_name(ParlorAction::Repeat);
    format!(
        "{} {} faces straight ahead. {} eases the string, or counts your arrows. {} repeats the last announcement. Escape pauses.",
        controls_line(ctx),
        sentence_start(&u),
        sentence_start(&d),
        sentence_start(&rep)
    )
}

/// "drifting left" or "drifting right".
fn drift_words(t: &Target) -> &'static str {
    if t.speed > 0.0 {
        "drifting right"
    } else {
        "drifting left"
    }
}

/// Where to meet a target: its angle when an arrow would reach it, to the
/// nearest step of the bow.
pub fn meet_angle(t: &Target) -> f64 {
    let at = t.angle_after(t.range.flight());
    ((at / AIM_STEP).round() * AIM_STEP).clamp(-AIM_LIMIT, AIM_LIMIT)
}

/// The sky right now: one entry per target, left to right.
pub fn sky_words(run: &Run) -> String {
    if run.sky.is_empty() {
        return "The sky is clear.".to_string();
    }
    let mut sky: Vec<&Target> = run.sky.iter().collect();
    sky.sort_by(|a, b| a.angle.total_cmp(&b.angle));
    let n = sky.len();
    let items: Vec<String> = sky
        .iter()
        .map(|t| {
            format!(
                "{}, {}, {}, {}",
                sentence_start(&t.kind.name()),
                t.range.name(),
                offset_words(run.aim, t.angle),
                drift_words(t)
            )
        })
        .collect();
    format!("{n} in the sky. {}.", items.join(". "))
}

/// Which run to start, and its seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Launch {
    pub mode: Mode,
    pub seed: u64,
    pub twist: Option<Twist>,
}

impl Launch {
    pub fn new(mode: Mode) -> Launch {
        Launch {
            mode,
            seed: run_seed(),
            twist: None,
        }
    }

    pub fn seeded(mode: Mode, seed: u64) -> Launch {
        Launch {
            mode,
            seed,
            twist: None,
        }
    }
}

/// The sky during a run.
pub struct Field {
    pub run: Run,
    launch: Launch,
    challenge: Option<Challenge>,
    loops: Vec<(u32, Sound)>,
    creak: Option<Sound>,
    last: String,
    taught: HashSet<&'static str>,
    clock: f64,
    busy_until: f64,
    ended: bool,
    /// Target call-outs off.
    quiet: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// Aim call-outs off.
    quiet_aim: bool,
    /// Running inside the tutorial, whose prompts do the teaching.
    lesson: bool,
}

impl Field {
    pub fn new(launch: Launch) -> Field {
        Field {
            status_key: Default::default(),
            run: Run::with_twist(launch.mode, launch.twist, Rng::seeded(launch.seed)),
            launch,
            challenge: None,
            loops: Vec::new(),
            creak: None,
            last: String::new(),
            taught: HashSet::new(),
            clock: 0.0,
            busy_until: 0.0,
            ended: false,
            quiet: false,
            quiet_aim: false,
            lesson: false,
        }
    }

    /// The field inside the tutorial. The lesson's prompts teach the
    /// basics and the field's own lines wait behind them, so nothing cuts
    /// a prompt off. Nothing is saved.
    pub fn in_lesson(mut self) -> Field {
        self.lesson = true;
        self.run.partner_gap = LESSON_PARTNER_GAP;
        self
    }

    /// A run of `mode` from the menus.
    pub fn mode(mode: Mode) -> Field {
        Field::new(Launch::new(mode))
    }

    /// A challenge's run, the same for every player.
    pub fn challenge(ch: Challenge) -> Field {
        let launch = challenge::launch(&ch);
        let mut field = Field::new(launch);
        field.challenge = Some(ch);
        field
    }

    pub fn is_challenge(&self) -> bool {
        self.challenge.is_some()
    }

    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        if self.lesson {
            return self.tell_queued(ctx, text);
        }
        let text = text.into();
        self.busy_until = self.clock + speech_secs(&text);
        ctx.say(text.clone());
        self.last = text;
    }

    fn tell_queued(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        self.busy_until = self.busy_until.max(self.clock) + speech_secs(&text);
        ctx.say_queued(text.clone());
        self.last = text;
    }

    /// A passing line: it never cuts off an important one.
    fn chatter(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        let wait = self.busy_until - self.clock;
        if wait <= 0.0 {
            ctx.say(text);
        } else if wait <= CHATTER_WAIT {
            ctx.say_queued(text);
        }
    }

    /// A passing line about something in the sky, not about a key: in the
    /// tutorial it waits for the lesson's prompt to finish.
    fn sky_chatter(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        if self.lesson {
            ctx.say_queued(text);
        } else {
            self.chatter(ctx, text);
        }
    }

    /// Teaches a cue the first time the player meets it, in this run or
    /// any before. Returns whether it did.
    fn teach(&mut self, ctx: &mut Ctx<Parlor>, what: &'static str, text: String) -> bool {
        if self.lesson && BASICS.contains(&what) {
            return false;
        }
        if !self.taught.insert(what) {
            return false;
        }
        if !self.lesson {
            let mut progress = Progress::load(ctx);
            if progress.taught.insert(what.to_string()) {
                progress.store(ctx);
            }
        }
        self.tell(ctx, text);
        true
    }

    /// The cues this player has already been taught, from earlier runs and
    /// the tutorial.
    fn load_taught(&mut self, ctx: &mut Ctx<Parlor>, progress: &Progress) {
        for what in BASICS
            .iter()
            .chain(LATER.iter())
            .copied()
            .filter(|w| progress.taught.contains(*w))
        {
            self.taught.insert(what);
        }
        if menus::tutorial_read(ctx, crate::ID) || progress.runs > 0 {
            self.taught.extend(BASICS);
        }
    }

    /// "Round 2 of 6" or "Round 4".
    fn round_words(&self) -> String {
        match self.run.rounds() {
            Some(n) => format!("round {} of {n}", self.run.round),
            None => format!("round {}", self.run.round),
        }
    }

    fn goal_words(&self) -> String {
        let p = &self.run.plan;
        if self.run.endless_arrows() {
            return "Endless arrows, nothing to lose.".to_string();
        }
        if p.goal == 0 {
            return format!("{}.", sentence_start(&arrows_words(p.arrows)));
        }
        format!(
            "{}. Hit {} to go on.",
            sentence_start(&arrows_words(p.arrows)),
            hearts_words(p.goal)
        )
    }

    /// What the round is, as it starts.
    fn round_intro(&self) -> String {
        let (place, line) = self.run.round_place();
        let mut text = if self.run.mode == Mode::Practice {
            format!("{place}. {line}")
        } else {
            format!(
                "{}, {}: {place}. {line} {}",
                self.run.mode.name(),
                self.round_words(),
                self.goal_words()
            )
        };
        if self.run.round == 1 {
            if let Some(twist) = self.run.twist {
                text.push_str(&format!(
                    " Today's twist, {}: {}.",
                    twist.name(),
                    twist.describe()
                ));
            }
            if self.run.plan.golden > 0.0 {
                text.push_str(&format!(
                    " Golden hearts drift tonight, worth {GOLDEN_POINTS}."
                ));
            }
            if self.run.plan.roses {
                text.push_str(&format!(
                    " Roses drift among the hearts, worth {ROSE_BONUS} more."
                ));
            }
        }
        if let Some(seats) = &self.run.seats {
            text.push(' ');
            text.push_str(&seats.turn_line());
        }
        text
    }

    /// The status line, in a fixed order: whose turn, the round, arrows,
    /// hearts, the aim, points, then the sky.
    /// The status key, in layers. First: whose turn, your aim and draw,
    /// arrows left and hearts. A second press: the round, points and the
    /// sky.
    pub fn status_layers(&self) -> Vec<String> {
        let r = &self.run;
        let mut first = Vec::new();
        if let Some(seats) = &r.seats {
            first.push(format!("{}'s turn", seats.current_name()));
        }
        first.push(format!("aim {}", aim_words(r.aim)));
        if let Some(d) = r.draw {
            first.push(format!("drawn {} percent", (d * 100.0).round() as u32));
        }
        let mut more = Vec::new();
        if r.mode == Mode::Practice {
            first.push(format!("{} hit", hearts_words(r.tally.hits)));
            more.push("Practice field".to_string());
        } else {
            first.push(format!("{} left", arrows_words(r.arrows_left)));
            if r.plan.goal > 0 {
                first.push(format!("{} of {} hearts", r.round_hits, r.plan.goal));
            } else {
                first.push(format!("{} this round", hearts_words(r.round_hits)));
            }
            more.push(sentence_start(&self.round_words()));
            more.push(format!("{} points", r.score));
        }
        more.push(format!("{} in the sky", r.sky.len()));
        if r.phase == Phase::Between {
            more.push("between rooftops".to_string());
        }
        vec![
            format!("{}.", sentence_start(&first.join(", "))),
            format!("{}.", sentence_start(&more.join(", "))),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        parlor_core::status::sentences(self.status_layers())
    }

    /// What to do now.
    pub fn hint_words(&self, ctx: &Ctx<Parlor>) -> String {
        let r = &self.run;
        let p = ctx.control_name(ParlorAction::Primary);
        if r.phase == Phase::Between {
            return format!("{} climbs to the next rooftop.", sentence_start(&p));
        }
        if let Some(t) = r.sweetheart_target() {
            if let Kind::Heart(c) = t.kind {
                return format!(
                    "Other {} heart: {}, for a sweetheart pair.",
                    c.name(),
                    turn_words(r.aim, meet_angle(t))
                );
            }
        }
        if r.partner_pending() {
            return "Its pair is coming. Hit it next.".to_string();
        }
        let nearest = r.sky.iter().min_by(|a, b| {
            (meet_angle(a) - r.aim)
                .abs()
                .total_cmp(&(meet_angle(b) - r.aim).abs())
        });
        match nearest {
            Some(t) => format!(
                "{}: {}.",
                sentence_start(&t.kind.name()),
                turn_words(r.aim, meet_angle(t))
            ),
            None => "The sky is clear.".to_string(),
        }
    }

    fn stop_creak(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.creak.take() {
            ctx.audio.stop(s, 40);
        }
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for (_, s) in self.loops.drain(..) {
            ctx.audio.stop(s, 80);
        }
        self.stop_creak(ctx);
    }

    fn stop_loop(&mut self, ctx: &mut Ctx<Parlor>, id: u32) {
        if let Some(at) = self.loops.iter().position(|(i, _)| *i == id) {
            let (_, s) = self.loops.remove(at);
            ctx.audio.stop(s, 60);
        }
    }

    fn sync_sound(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::face(ctx, self.run.aim);
        for t in &self.run.sky {
            let pos = sky_pos(t.angle, t.range);
            match self.loops.iter().find(|(id, _)| *id == t.id) {
                Some((_, s)) => ctx.audio.set_position(*s, Some(pos)),
                None => {
                    let s =
                        ctx.audio
                            .start_loop(sound::target_loop(t.kind), Some(pos), Bus::EFFECTS);
                    ctx.audio.set_pitch(s, sound::target_pitch(t.kind));
                    ctx.audio.set_volume(s, sound::range_volume(t.range));
                    self.loops.push((t.id, s));
                }
            }
        }
        if let (Some(s), Some(d)) = (self.creak, self.run.draw) {
            ctx.audio.set_pitch(s, 0.8 + 0.6 * d as f32);
        }
    }

    fn start_round(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::play(ctx, sound::ROUND_START);
        let intro = self.round_intro();
        self.tell(ctx, intro);
    }

    fn draw(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.run.begin_draw() {
            Ok(()) => {
                self.stop_creak(ctx);
                let s = ctx.audio.start_loop(sound::CREAK, None, Bus::EFFECTS);
                ctx.audio.set_pitch(s, 0.8);
                self.creak = Some(s);
            }
            Err(Refusal::Empty) => {
                sound::play(ctx, sound::EMPTY);
                ctx.say("The quiver is empty.");
            }
            Err(Refusal::Drawing) => {
                sound::play(ctx, sound::CREAK);
            }
            Err(Refusal::NotPlaying) => {
                sound::play(ctx, sound::NOPE);
                ctx.say("The round is over.");
            }
        }
    }

    fn let_go(&mut self, ctx: &mut Ctx<Parlor>) {
        let release = self.run.release();
        self.stop_creak(ctx);
        match release {
            Release::Shot => {
                sound::play(ctx, sound::LOOSE);
                if let Some(seats) = &self.run.seats {
                    let line = seats.turn_line();
                    sound::play(ctx, sound::TURN);
                    self.sky_chatter(ctx, line);
                }
            }
            Release::TooSoft(d) => {
                sound::play(ctx, sound::SLACK);
                let p = ctx.control_name(ParlorAction::Primary);
                let pct = (d * 100.0).round() as u32;
                if !self.teach(
                    ctx,
                    "slack",
                    format!("Too soon, {pct} percent. Hold {p} until the click."),
                ) {
                    ctx.say(format!("Too soon, {pct} percent."));
                }
            }
            Release::NotDrawing => {}
        }
    }

    fn on_event(&mut self, ctx: &mut Ctx<Parlor>, event: Event) {
        match event {
            Event::Spawn {
                kind,
                from_left,
                partner,
                ..
            } => self.on_spawn(ctx, kind, from_left, partner),
            Event::Gone { id, .. } => self.stop_loop(ctx, id),
            Event::FullDraw => {
                sound::play(ctx, sound::FULL);
                self.teach(ctx, "full", "Full draw. Let go to shoot.".to_string());
            }
            Event::Sights(Some(id)) => {
                if let Some(t) = self.run.target(id) {
                    // From the target's side, but as near as the nearest
                    // range, so a far heart's ping is as loud as a near one's.
                    let pos = sky_pos(t.angle, Range::Near);
                    ctx.audio.play_at(sound::SIGHT, pos, Bus::EFFECTS);
                    let name = t.kind.name();
                    self.teach(
                        ctx,
                        "sight",
                        format!("In your sights: the {name}. Let go now."),
                    );
                }
            }
            Event::Sights(None) => {}
            Event::Hit {
                id,
                kind,
                range,
                angle,
                points,
                sweetheart,
                shooter,
                ..
            } => self.on_hit(ctx, id, kind, range, angle, points, sweetheart, shooter),
            Event::Miss { aim, .. } => {
                ctx.audio
                    .play_at(sound::MISS, sky_pos(aim, Range::Far), Bus::EFFECTS);
                if !self.teach(
                    ctx,
                    "miss",
                    "Miss. Aim a little ahead, more for far hearts.".to_string(),
                ) {
                    self.sky_chatter(ctx, "Miss.".to_string());
                }
            }
            Event::RoundOver {
                round,
                hits,
                goal,
                more,
            } => self.on_round_over(ctx, round, hits, goal, more),
        }
    }

    fn on_spawn(&mut self, ctx: &mut Ctx<Parlor>, kind: Kind, from_left: bool, partner: bool) {
        let side = if from_left { "the left" } else { "the right" };
        let taught = match kind {
            Kind::Rose => self.teach(
                ctx,
                "rose",
                format!("A rose, from {side}, worth {ROSE_BONUS} more."),
            ),
            Kind::Golden => self.teach(
                ctx,
                "golden",
                format!("A golden heart, from {side}, fast, worth {GOLDEN_POINTS}."),
            ),
            Kind::Letter => self.teach(
                ctx,
                "letter",
                format!("A love note, from {side}. Hit it to keep it."),
            ),
            Kind::Heart(c) if partner => self.teach(
                ctx,
                "pair",
                format!(
                    "Second {} heart, from {side}. Hit both in a row for a pair.",
                    c.name()
                ),
            ),
            Kind::Heart(_) => self.teach(
                ctx,
                "heart",
                format!("A paper heart, from {side}. Turn until its hum is just ahead."),
            ),
        };
        if !taught && !self.quiet {
            let name = match kind {
                Kind::Heart(c) if partner => format!("Second {} heart", c.name()),
                _ => sentence_start(&kind.name()),
            };
            // A new target's name waits for a longer line rather than being
            // dropped, so a heart that drifts in under the round's opening
            // is still named.
            let line = format!("{name}, from {side}.");
            if self.lesson || self.busy_until > self.clock {
                ctx.say_queued(line);
            } else {
                ctx.say(line);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_hit(
        &mut self,
        ctx: &mut Ctx<Parlor>,
        id: u32,
        kind: Kind,
        range: Range,
        angle: f64,
        points: u64,
        sweetheart: bool,
        shooter: usize,
    ) {
        self.stop_loop(ctx, id);
        let pos = sky_pos(angle, range);
        ctx.audio.play_at(sound::POP, pos, Bus::EFFECTS);
        let key = match kind {
            Kind::Heart(_) if sweetheart => sound::SWEETHEART,
            Kind::Heart(_) => sound::CHIME,
            Kind::Rose => sound::ROSE_HIT,
            Kind::Golden => sound::GOLDEN_HIT,
            Kind::Letter => sound::LETTER_HIT,
        };
        let s = ctx.audio.play_at(key, pos, Bus::EFFECTS);
        ctx.audio.set_pitch(s, sound::target_pitch(kind));
        let who = match &self.run.seats {
            Some(seats) => format!("{}: ", seats.name(shooter)),
            None => String::new(),
        };
        if kind == Kind::Letter {
            self.found_note(ctx, &who, points);
            return;
        }
        if sweetheart {
            if let Kind::Heart(c) = kind {
                let first = format!(
                    "{who}Sweetheart pair, both {} hearts. {points} points, with the pair bonus.",
                    c.name()
                );
                if !self.teach(ctx, "sweetheart", first) {
                    self.tell(ctx, format!("{who}Sweetheart pair. {points}."));
                }
                return;
            }
        }
        let line = format!("{who}{}, {points}.", sentence_start(&kind.name()));
        ctx.say(line);
    }

    fn found_note(&mut self, ctx: &mut Ctx<Parlor>, who: &str, points: u64) {
        if self.is_challenge() {
            self.tell(ctx, format!("{who}A love note. {points}."));
            return;
        }
        let mut progress = Progress::load(ctx);
        match progress.find_note() {
            Some(i) => {
                progress.store(ctx);
                sound::play(ctx, sound::KEEPSAKE);
                let n = progress.notes_found();
                self.tell(
                    ctx,
                    format!("{who}A love note, {n} of {}. {points}.", NOTES.len()),
                );
                self.tell_queued(ctx, NOTES[i as usize]);
            }
            None => self.tell(
                ctx,
                format!("{who}A love note, one you have kept already. {points}."),
            ),
        }
    }

    fn on_round_over(
        &mut self,
        ctx: &mut Ctx<Parlor>,
        round: u32,
        hits: u32,
        goal: u32,
        more: bool,
    ) {
        self.stop_loops(ctx);
        sound::play(ctx, sound::ROUND_END);
        if !more {
            self.finish(ctx);
            return;
        }
        let r = &self.run;
        let goal_part = if goal > 0 {
            format!("{} of {goal} needed", hearts_words(hits))
        } else {
            hearts_words(hits)
        };
        let summary = format!(
            "Round {round} done: {goal_part}, {} points this round, {} in all.",
            r.round_points, r.score
        );
        let mut peek = self.run.clone();
        peek.next_round();
        let (place, _) = peek.round_place();
        let p = ctx.control_name(ParlorAction::Primary);
        self.tell(ctx, summary);
        self.tell_queued(
            ctx,
            format!(
                "{} climbs to the next rooftop: {place}.",
                sentence_start(&p)
            ),
        );
    }

    fn next_round(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.run.next_round() {
            self.start_round(ctx);
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        self.stop_loops(ctx);
        let cleared = self.run.cleared() || !self.run.mode.needs_goal();
        ctx.audio.set_bed(None);
        ctx.audio.play_music(
            Some(if cleared {
                sound::WIN
            } else {
                sound::EVENING_ENDS
            }),
            false,
        );
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.run);
            let summary = format!("{} result: {} points", ch.title(), self.run.score);
            let mut results = Some(challenge::after(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let saved = progress::save_end(ctx, &self.run);
        if saved.keepsake.is_some() {
            sound::play(ctx, sound::KEEPSAKE);
        }
        let pages = result_pages(&self.run, &saved);
        let summary = result_summary(&self.run);
        let mut results = Some(after_run(self.launch.mode, cleared, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// The results menu's title.
pub fn result_summary(run: &Run) -> String {
    match run.mode {
        Mode::Rooftops if run.cleared() => {
            format!("All {} rooftops cleared. {} points", run.round, run.score)
        }
        Mode::Rooftops | Mode::Moonlight => {
            let (place, _) = run.round_place();
            format!("The evening ends at {place}. {} points", run.score)
        }
        _ => format!("{} done. {} points", run.mode.name(), run.score),
    }
}

fn score_words(result: Option<ScoreResult>) -> String {
    match result {
        Some(ScoreResult::First) => " Your first score here.".to_string(),
        Some(ScoreResult::NewBest { previous }) => format!(" A new best, beating {previous}."),
        Some(ScoreResult::Short { best }) => format!(" Your best is {best}."),
        None => String::new(),
    }
}

/// The pages after a run.
pub fn result_pages(run: &Run, saved: &Saved) -> Vec<String> {
    let (place, _) = run.round_place();
    let mut first = match run.mode {
        Mode::Rooftops if run.cleared() => format!(
            "Evening done: all {} rooftops cleared. {} points.",
            run.round, run.score
        ),
        Mode::Rooftops | Mode::Moonlight => format!(
            "The evening ends at {place}: {} of {} needed. {} points.",
            hearts_words(run.round_hits),
            run.plan.goal,
            run.score
        ),
        _ => format!("{} done. {} points.", run.mode.name(), run.score),
    };
    first.push_str(&score_words(saved.score));
    let t = &run.tally;
    let mut second = vec![format!(
        "{} from {}, {} percent.",
        sentence_start(&hearts_words(t.hits)),
        arrows_words(t.shots),
        run.accuracy()
    )];
    second.push(match t.pairs {
        0 => "No sweetheart pairs.".to_string(),
        1 => "1 sweetheart pair.".to_string(),
        n => format!("{n} sweetheart pairs."),
    });
    if t.roses > 0 {
        second.push(match t.roses {
            1 => "1 rose.".to_string(),
            n => format!("{n} roses."),
        });
    }
    if t.golden > 0 {
        second.push(match t.golden {
            1 => "1 golden heart.".to_string(),
            n => format!("{n} golden hearts."),
        });
    }
    if t.letters > 0 {
        second.push(match t.letters {
            1 => "1 love note.".to_string(),
            n => format!("{n} love notes."),
        });
    }
    second.push(format!("Longest run of hits: {}.", t.best_streak));
    if run.mode == Mode::ForTwo {
        second.push(format!(
            "Player 1 hit {}, Player 2 hit {}.",
            t.by_seat[0], t.by_seat[1]
        ));
    }
    let mut pages = vec![first, second.join(" ")];
    if run.mode == Mode::Rooftops {
        if saved.roofs_record {
            pages.push(format!(
                "Furthest yet: {} of {} rooftops.",
                saved.roofs,
                crate::rules::ROOFTOPS.len()
            ));
        }
        if saved.opened_moonlight {
            pages.push(
                "Moonlight is open, on the title: endless rounds under the moon, faster with each one."
                    .to_string(),
            );
        }
    }
    if let Some(year) = saved.keepsake {
        let item = valentine::keepsake_item(year);
        pages.push(if saved.first_of_year {
            format!("Your keepsake for Valentine's Day {year}: {item}. It is in the keepsake box on the title.")
        } else {
            format!("Your keepsake for {year}, {item}, is already in the box. Tonight's score is kept with it.")
        });
    }
    pages
}

/// The results menu after a run.
pub fn after_run(mode: Mode, cleared: bool, summary: String, pages: Vec<String>) -> Results {
    let again = if cleared {
        labels::PLAY_AGAIN
    } else {
        labels::TRY_AGAIN
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        again,
        format!("{} again, from the first round.", mode.name()),
        move |ctx| ctx.replace(Field::mode(mode)),
    )
}

impl State<Parlor> for Field {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        FIELD_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let today = parlor_core::challenges::today(ctx);
        if !self.run.mode.is_challenge() {
            self.run.set_roses(valentine::roses_on(today));
        }
        let progress = Progress::load(ctx);
        self.quiet = progress.quiet;
        self.quiet_aim = progress.quiet_aim;
        self.load_taught(ctx, &progress);
        sound::sky_bed(ctx, self.run.mode.is_valentine());
        sound::face(ctx, self.run.aim);
        if self.lesson {
            // The lesson's intro and prompt say where the player is.
            sound::play(ctx, sound::ROUND_START);
            return;
        }
        self.start_round(ctx);
        let c = controls_line(ctx);
        ctx.say_queued(c);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        // The pause menu's Settings may have changed the call-outs.
        let progress = Progress::load(ctx);
        self.quiet = progress.quiet;
        self.quiet_aim = progress.quiet_aim;
        sound::sky_bed(ctx, self.run.mode.is_valentine());
        ctx.say(format!(
            "Back on the roof. {}",
            self.status_layers().remove(0)
        ));
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
            if self.run.cancel_draw() {
                self.stop_creak(ctx);
            }
            let leave = if self.is_challenge() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let turning = matches!(action, Some(ParlorAction::Left | ParlorAction::Right));
        if event.is_repeat() && !turning {
            return;
        }
        if self.run.phase == Phase::Between
            && (action == Some(ParlorAction::Primary)
                || (action.is_none() && event.pressed() == Some(Key::Return)))
        {
            self.next_round(ctx);
            return;
        }
        match action {
            Some(ParlorAction::Left | ParlorAction::Right) => {
                let right = action == Some(ParlorAction::Right);
                if self.run.turn(right) {
                    sound::play(ctx, sound::TICK);
                    if !self.quiet_aim {
                        // The answer to the key, so it always speaks: a
                        // longer line it cuts off is still on the repeat key.
                        let words = sentence_start(&aim_words(self.run.aim));
                        ctx.say(format!("{words}."));
                    }
                } else {
                    sound::play(ctx, sound::EDGE);
                    ctx.say(format!(
                        "As far as the bow turns, {}.",
                        aim_words(self.run.aim)
                    ));
                }
                sound::face(ctx, self.run.aim);
            }
            Some(ParlorAction::Up) => {
                self.run.centre();
                sound::play(ctx, sound::TICK);
                sound::face(ctx, 0.0);
                ctx.say("Straight ahead.");
            }
            Some(ParlorAction::Down) => {
                if self.run.cancel_draw() {
                    self.stop_creak(ctx);
                    sound::play(ctx, sound::SLACK);
                    ctx.say("String eased.");
                } else if self.run.endless_arrows() {
                    ctx.say("Endless arrows on the practice field.");
                } else {
                    ctx.say(format!(
                        "{} in the quiver.",
                        sentence_start(&arrows_words(self.run.arrows_left))
                    ));
                }
            }
            Some(ParlorAction::Primary) => self.draw(ctx),
            Some(ParlorAction::Secondary) => {
                let words = sky_words(&self.run);
                ctx.say(words);
            }
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
            Some(ParlorAction::Hint) => {
                sound::play(ctx, sound::HINT);
                let text = self.hint_words(ctx);
                ctx.say(text);
            }
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            return;
        }
        self.clock += dt;
        // The run steps first, so a draw that reaches full this frame counts.
        let let_go = self.run.drawing() && !ctx.action_held(ParlorAction::Primary);
        for e in self.run.update(dt) {
            self.on_event(ctx, e);
            if self.ended {
                return;
            }
        }
        if let_go {
            self.let_go(ctx);
        }
        self.sync_sound(ctx);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let r = &self.run;
        let mut out = vec![format!("Cupid's Quiver: {}", r.mode.name()), self.status()];
        let mut sky: Vec<&Target> = r.sky.iter().collect();
        sky.sort_by(|a, b| a.angle.total_cmp(&b.angle));
        for t in sky {
            out.push(format!(
                "{} at {:.0}, {}",
                t.kind.name(),
                t.angle,
                t.range.name()
            ));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut now = "Aim a little ahead of a drifting heart.".to_string();
        if let Some(ch) = &self.challenge {
            now.push_str(&format!(
                " This is the {} challenge. Leaving from the pause menu does not count it.",
                ch.period.label().to_lowercase()
            ));
        }
        format!("{now} {}", controls_full(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_field;
