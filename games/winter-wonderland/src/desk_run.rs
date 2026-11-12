//! The Wish Desk's screen: the briefing, the shelf, the letters, and
//! where a run goes when the stack is answered.
//!
//! Left and Right walk the shelf, Space shakes the box in front of you,
//! and holding Space wraps it: the paper crinkles, the ribbon snaps and
//! the stamp comes down. Let go before the snap and nothing is wrapped.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Sound, State};
use parlor_core::{Pages, Parlor, ParlorAction, Rng};

use crate::challenge::{self, Part};
use crate::desk::{Shake, Stack, StackSpec, Wrapped, STACKS};
use crate::desk_music::Conductor;
use crate::events;
use crate::mode::{story_ending, Mode, WRITES};
use crate::progress::{self, Progress};
use crate::sound::{self, cue, shelf_pos, stinger};
use crate::{fresh_rng, pause, results};

/// Seconds Space is held before the wrapping starts.
pub const HOLD_TO_WRAP: f64 = 0.45;
/// Seconds of wrapping before the ribbon snaps and the parcel is done.
pub const WRAP_TIME: f64 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The stack's briefing; the desk waits for the player.
    Briefing,
    Playing,
}

/// One run at the Wish Desk: a stack, or a few in a row.
pub struct DeskRun {
    mode: Mode,
    /// The stack the run started from, for Play again.
    first: u32,
    level: u32,
    last: u32,
    stack: Stack,
    /// Points from stacks already answered.
    banked: u64,
    phase: Phase,
    rng: Rng,
    /// Seconds Space has been held on this press, while it is held.
    held: Option<f64>,
    /// The wrapping paper's sound, while wrapping.
    paper: Sound,
    /// "It sounds like a squish." waits for the shake to finish.
    pending: Option<(f64, String)>,
    /// The straw has been explained once.
    told_straw: bool,
    /// The first wrap has been explained once.
    told_wrap: bool,
    /// Late post has come this run.
    late: bool,
    /// The last stack answered opened the next one.
    opened: bool,
    /// Bonus for the stack just answered.
    bonus: u64,
    letters: u32,
    wrongs: u32,
    shakes: u32,
    music: Conductor,
    /// Seconds into the run, for the music.
    time: f64,
    status_key: parlor_core::StatusKey,
    /// Run inside the tutorial: one stack, never results.
    pub lesson: bool,
}

impl DeskRun {
    /// A run in `mode`; `start` is the stack for a run from the title.
    pub fn new(mode: Mode, start: u32) -> DeskRun {
        let (first, last) = mode.stacks(start.clamp(1, STACKS));
        let mut rng = match mode.plan() {
            Some(plan) => Rng::seeded(plan.run_seed ^ 0xde5c),
            None => fresh_rng(),
        };
        let stack = Self::make_stack(&mode, first, Rng::seeded(rng.next_u64()));
        let music = Conductor::new(rng.next_u64());
        DeskRun {
            mode,
            first,
            level: first,
            last,
            stack,
            banked: 0,
            phase: Phase::Briefing,
            rng,
            held: None,
            paper: Sound::NONE,
            pending: None,
            told_straw: false,
            told_wrap: false,
            late: false,
            opened: false,
            bonus: 0,
            letters: 0,
            wrongs: 0,
            shakes: 0,
            music,
            time: 0.0,
            status_key: Default::default(),
            lesson: false,
        }
    }

    fn make_stack(mode: &Mode, level: u32, rng: Rng) -> Stack {
        match mode {
            Mode::Challenge { plan, .. } => plan.stack(level, rng),
            Mode::Muddled => {
                let mut spec = StackSpec::new(level);
                spec.straw = true;
                spec.little = true;
                Stack::new(spec, rng)
            }
            _ => Stack::new(StackSpec::new(level), rng),
        }
    }

    /// The stack on the desk, for the tutorial and tests.
    pub fn stack(&self) -> &Stack {
        &self.stack
    }

    /// Whether the letters have started, the briefing over.
    pub fn playing(&self) -> bool {
        self.phase == Phase::Playing
    }

    /// The run's score so far.
    pub fn score(&self) -> u64 {
        self.banked + self.stack.score
    }

    fn name(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// "a squish": what the current letter's wish would sound like.
    fn wish_sound(&self) -> Option<String> {
        self.stack.letter().map(|l| format!("a {}", l.wish.name()))
    }

    /// The current letter, as R reads it.
    pub fn letter_text(&self) -> String {
        let Some(l) = self.stack.letter() else {
            return "Every letter is answered.".to_string();
        };
        let late = if l.late { "Late post. " } else { "" };
        let sound = if self.stack.spec.say_sounds && !l.little {
            format!(" It would {} in its box.", l.wish.name())
        } else {
            String::new()
        };
        format!(
            "{late}{}, age {}, {WRITES}: {}{sound}",
            l.from,
            l.age,
            l.words()
        )
    }

    fn briefing(&self, ctx: &Ctx<Parlor>) -> String {
        let spec = &self.stack.spec;
        let primary = Self::name(ctx, ParlorAction::Primary);
        let of = if self.last > self.first {
            format!(" of {}", self.last)
        } else {
            String::new()
        };
        let mut text = format!(
            "Wish Desk, stack {}{of}. {} letters, {} boxes on the shelf.",
            spec.level, spec.letters, spec.boxes
        );
        if spec.level == 1 || self.mode.is_story() {
            text.push_str(&format!(
                " {} and {} walk the shelf. {} shakes a box. Hold {primary} to wrap it for the letter.",
                sentence_start(&Self::name(ctx, ParlorAction::Left)),
                Self::name(ctx, ParlorAction::Right),
                sentence_start(&primary),
            ));
        }
        if spec.level == 3 && !spec.straw {
            text.push_str(" From now on, the letters do not say how each wish sounds.");
        }
        // A challenge's own twist line says so instead.
        if spec.straw && self.mode.plan().is_none() {
            text.push_str(" Every box is packed in straw: shake it twice.");
        }
        if let Some(plan) = self.mode.plan() {
            if !plan.twists_gifts() {
                text.push(' ');
                text.push_str(&plan.twist_line());
            }
        }
        text.push_str(&format!(
            " {} opens the first letter.",
            sentence_start(&primary)
        ));
        text
    }

    /// The status key, in layers. First: where you stand and the letter.
    /// A second press: the stack and the score.
    fn status(&self) -> Vec<String> {
        let s = &self.stack;
        if self.phase == Phase::Briefing {
            return vec![
                "The letters are waiting for you.".to_string(),
                format!("Stack {}. Score {}.", s.spec.level, self.score()),
            ];
        }
        let here = s.here();
        let place = if here.gone {
            "wrapped and gone"
        } else if here.shakes == 0 {
            "not shaken yet"
        } else {
            "shaken"
        };
        let letter = match s.letter() {
            Some(l) => format!(
                "{}'s letter, {} of {}.",
                l.from,
                s.current + 1,
                s.letters.len()
            ),
            None => "Every letter is answered.".to_string(),
        };
        vec![
            format!("Box {} of {}, {place}. {letter}", s.at + 1, s.shelf.len()),
            format!(
                "Stack {}. {} answered. Score {}.",
                s.spec.level,
                s.answered(),
                self.score()
            ),
        ]
    }

    fn pos(&self) -> hearken::Vec3 {
        shelf_pos(self.stack.at, self.stack.shelf.len())
    }

    fn begin(&mut self, ctx: &mut Ctx<Parlor>) {
        self.phase = Phase::Playing;
        cue(ctx, sound::DESK_LETTER);
        let first = self.letter_text();
        ctx.say(format!("Box 1. {first}"));
    }

    fn step(&mut self, ctx: &mut Ctx<Parlor>, by: i32) {
        self.cancel_wrap(ctx, false);
        if !self.stack.step(by) {
            cue(ctx, sound::DESK_END);
            ctx.say(format!("End of the shelf. Box {}.", self.stack.at + 1));
            return;
        }
        self.jumped(ctx);
    }

    /// To the first or last box.
    fn jump_end(&mut self, ctx: &mut Ctx<Parlor>, last: bool) {
        self.cancel_wrap(ctx, false);
        let to = if last { self.stack.shelf.len() - 1 } else { 0 };
        if self.stack.at == to {
            cue(ctx, sound::DESK_END);
            ctx.say(format!("Already at box {}.", to + 1));
            return;
        }
        self.stack.at = to;
        self.jumped(ctx);
    }

    fn jumped(&mut self, ctx: &mut Ctx<Parlor>) {
        self.pending = None;
        let pos = self.pos();
        if self.stack.here().gone {
            ctx.audio.play_at(sound::DESK_EMPTY, pos, Bus::EFFECTS);
            ctx.say(format!("Box {}, gone.", self.stack.at + 1));
        } else {
            ctx.audio.play_at(sound::DESK_STEP, pos, Bus::EFFECTS);
            ctx.say(format!("Box {}.", self.stack.at + 1));
        }
    }

    fn shake(&mut self, ctx: &mut Ctx<Parlor>) {
        let pos = self.pos();
        match self.stack.shake() {
            Shake::Gift(kind) => {
                ctx.audio.play_at(sound::gift(kind), pos, Bus::EFFECTS);
                if self.stack.spec.level == 1 {
                    self.pending = Some((1.1, format!("It sounds like a {}.", kind.name())));
                }
            }
            Shake::Straw => {
                ctx.audio.play_at(sound::DESK_STRAW, pos, Bus::EFFECTS);
                if !self.told_straw {
                    self.told_straw = true;
                    let p = Self::name(ctx, ParlorAction::Primary);
                    ctx.say(format!(
                        "Only straw. {} again to hear the gift.",
                        sentence_start(&p)
                    ));
                }
            }
            Shake::Empty => {
                ctx.audio.play_at(sound::DESK_EMPTY, pos, Bus::EFFECTS);
                ctx.say("Nothing here. That gift is wrapped and gone.");
            }
        }
    }

    /// Stops wrapping. Says so when `say` and the paper had started.
    fn cancel_wrap(&mut self, ctx: &mut Ctx<Parlor>, say: bool) {
        let wrapping = self.held.is_some_and(|t| t >= HOLD_TO_WRAP);
        self.held = None;
        if self.paper != Sound::NONE {
            ctx.audio.stop(self.paper, 120);
            self.paper = Sound::NONE;
        }
        if wrapping && say {
            ctx.say("Wrapping stopped. Hold to the snap to finish.");
        }
    }

    /// Runs a held Space: the paper starts, then the ribbon snaps.
    fn follow_hold(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Some(t) = self.held else { return };
        if !ctx.action_held(ParlorAction::Primary) {
            self.cancel_wrap(ctx, true);
            return;
        }
        let t = t + dt;
        self.held = Some(t);
        if t >= HOLD_TO_WRAP && self.paper == Sound::NONE {
            if self.stack.here().gone || self.stack.done() {
                self.held = None;
                return;
            }
            self.pending = None;
            ctx.stop_speech();
            self.paper = ctx
                .audio
                .play_at(sound::DESK_WRAP, self.pos(), Bus::EFFECTS);
        }
        if t >= HOLD_TO_WRAP + WRAP_TIME {
            self.paper = Sound::NONE;
            self.held = None;
            self.wrapped(ctx);
        }
    }

    fn wrapped(&mut self, ctx: &mut Ctx<Parlor>) {
        let pos = self.pos();
        ctx.audio.play_at(sound::DESK_SNAP, pos, Bus::EFFECTS);
        match self.stack.wrap() {
            Wrapped::Right { points, from } => {
                ctx.audio.play_at(sound::DESK_STAMP, pos, Bus::EFFECTS);
                if self.stack.done() {
                    self.stack_done(ctx, from, points);
                } else if self.lesson {
                    // The lesson's next step speaks instead of the next letter.
                    ctx.say(format!("Wrapped for {from}. Plus {points}."));
                } else {
                    cue(ctx, sound::DESK_LETTER);
                    let next = self.letter_text();
                    ctx.say(format!("Wrapped for {from}. Plus {points}. {next}"));
                }
            }
            Wrapped::Wrong { from, .. } => {
                ctx.audio.play_at(sound::DESK_WRONG, pos, Bus::EFFECTS);
                let more = if !self.told_wrap {
                    self.told_wrap = true;
                    format!(
                        " That costs some of this letter's points. {} reads the letter again.",
                        sentence_start(&Self::name(ctx, ParlorAction::Repeat))
                    )
                } else {
                    String::new()
                };
                ctx.say(format!(
                    "Not what {from} asked for. It goes back on the shelf.{more}"
                ));
            }
            Wrapped::Empty => {
                ctx.audio.play_at(sound::DESK_EMPTY, pos, Bus::EFFECTS);
                ctx.say("Nothing here to wrap.");
            }
        }
    }

    /// The last letter of a stack is answered.
    fn stack_done(&mut self, ctx: &mut Ctx<Parlor>, from: &str, points: u64) {
        if !self.late
            && self.mode == Mode::Free
            && !self.lesson
            && events::late_post_due(ctx)
            && self.stack.late_post()
        {
            self.late = true;
            events::late_post_came(ctx);
            cue(ctx, sound::DESK_LATE);
            // The first late letter, without its own "Late post." in front.
            let next = self.letter_text();
            let next = next.strip_prefix("Late post. ").unwrap_or(&next);
            ctx.say(format!(
                "Wrapped for {from}. Plus {points}. Late post: two more letters, worth double. {next}"
            ));
            return;
        }
        if self.lesson {
            ctx.say(format!("Wrapped for {from}. Plus {points}."));
            return;
        }
        self.bonus = self.stack.neat_bonus();
        self.stack.score += self.bonus;
        cue(ctx, sound::DESK_CLEAR);
        self.banked += self.stack.score;
        self.tally();
        if self.mode == Mode::Free {
            self.opened = Progress::open_stack(ctx, self.level + 1);
        }
        if self.level >= self.last {
            ctx.say(format!("Wrapped for {from}. Plus {points}."));
            self.finish(ctx);
            return;
        }
        let done = self.level;
        let bonus = self.bonus_words();
        self.level += 1;
        self.stack = Self::make_stack(&self.mode, self.level, Rng::seeded(self.rng.next_u64()));
        self.phase = Phase::Briefing;
        ctx.say(format!(
            "Wrapped for {from}. Plus {points}. Stack {done} answered.{bonus} Score {}.",
            self.score()
        ));
        let brief = self.briefing(ctx);
        ctx.say_queued(brief);
    }

    fn bonus_words(&self) -> String {
        if self.bonus > 0 {
            format!(" No wrong boxes: {} bonus points.", self.bonus)
        } else {
            String::new()
        }
    }

    fn tally(&mut self) {
        self.letters += self.stack.answered() as u32;
        self.wrongs += self.stack.total_wrongs;
        self.shakes += self.stack.total_shakes;
    }

    fn summary(&self) -> String {
        let letters = if self.letters == 1 {
            "letter"
        } else {
            "letters"
        };
        let boxes = if self.wrongs == 1 { "box" } else { "boxes" };
        let shakes = if self.shakes == 1 { "shake" } else { "shakes" };
        format!(
            "{} {letters} answered with {} {shakes}, {} wrong {boxes}.",
            self.letters, self.shakes, self.wrongs
        )
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.music.stop(ctx);
        stinger(ctx, sound::WIN);
        let score = self.banked;
        let level = self.level;
        let bonus = self.bonus_words();
        match self.mode.clone() {
            Mode::Story { blizzard, gifts } => {
                let total = gifts + score;
                let board = if blizzard {
                    progress::BLIZZARD_BOARD
                } else {
                    progress::STORY_BOARD
                };
                let line = progress::record(ctx, board, total);
                let opened = !blizzard && Progress::mark(ctx, |p| &mut p.story);
                let extra = if opened {
                    " The blizzard story is now open on the title."
                } else {
                    ""
                };
                let pages = vec![
                    story_ending(),
                    format!(
                        "Gift Sort {gifts} plus the Wish Desk {score}: story score {total}. {}{bonus} {line}{extra}",
                        self.summary()
                    ),
                ];
                let summary = format!("Every letter answered. Story score {total}");
                results::show(
                    ctx,
                    pages.clone(),
                    results::story(blizzard, true, summary, pages),
                );
            }
            Mode::Challenge { plan, gifts } => {
                let total = gifts + score;
                let parts = if plan.part == Part::Double {
                    format!("Gift Sort {gifts} plus the Wish Desk {score}. ")
                } else {
                    String::new()
                };
                let line = challenge::result_line(ctx, &plan, total);
                let pages = vec![format!(
                    "Challenge cleared. {}{bonus} {parts}{line}",
                    self.summary()
                )];
                let summary = format!("Challenge cleared. Score {total}");
                results::show(
                    ctx,
                    pages.clone(),
                    results::challenge(&plan, summary, pages),
                );
            }
            Mode::Muddled => {
                let line = progress::record(ctx, events::MUDDLED_BOARD, score);
                let pages = vec![format!(
                    "Every little letter is answered. {}{bonus} Score {score}. {line}",
                    self.summary()
                )];
                let summary = format!("Muddled post answered. Score {score}");
                results::show(ctx, pages.clone(), results::muddled(summary, pages));
            }
            _ => {
                let line = progress::record(ctx, progress::DESK_BOARD, score);
                progress::record(ctx, &progress::stack_board(level), score);
                let mut extra = String::new();
                if level < STACKS {
                    if self.opened {
                        extra = format!(" Stack {} is open.", level + 1);
                    }
                } else if Progress::mark(ctx, |p| &mut p.all_stacks) {
                    extra = " You have answered every stack at the desk.".to_string();
                }
                let pages = vec![format!(
                    "Stack {level} answered. {}{bonus} Score {score}. {line}{extra}",
                    self.summary()
                )];
                let summary = format!("Stack {level} answered. Score {score}");
                results::show(
                    ctx,
                    pages.clone(),
                    results::desk(self.first, summary, pages),
                );
            }
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(letter) = self.stack.letter().cloned() else {
            ctx.say("Every letter is answered.");
            return;
        };
        let Some(answer) = self.stack.answer() else {
            ctx.say("Every letter is answered.");
            return;
        };
        let first = !self.stack.hinted;
        self.stack.take_hint();
        cue(ctx, sound::HINT);
        let sound = self.wish_sound().unwrap_or_default();
        let named = self.stack.spec.say_sounds && !letter.little;
        let text = if first && !named {
            format!(
                "{}'s wish would sound like {sound} in its box.",
                letter.from
            )
        } else {
            let way = match answer.cmp(&self.stack.at) {
                std::cmp::Ordering::Less => format!(
                    "Walk {} to box {}.",
                    Self::name(ctx, ParlorAction::Left),
                    answer + 1
                ),
                std::cmp::Ordering::Greater => format!(
                    "Walk {} to box {}.",
                    Self::name(ctx, ParlorAction::Right),
                    answer + 1
                ),
                std::cmp::Ordering::Equal => "It is the box in front of you.".to_string(),
            };
            format!("{}'s gift sounds like {sound}. {way}", letter.from)
        };
        let half = if first {
            " Half points for this letter."
        } else {
            ""
        };
        ctx.say(format!("{text}{half}"));
    }

    fn reminder(&self, ctx: &Ctx<Parlor>) -> String {
        let p = Self::name(ctx, ParlorAction::Primary);
        match self.phase {
            Phase::Briefing => format!(
                "{} opens the first letter, {} repeats the briefing.",
                sentence_start(&p),
                Self::name(ctx, ParlorAction::Repeat)
            ),
            Phase::Playing => format!(
                "{} and {} walk the shelf. {} shakes, hold it to wrap. F1 explains the keys.",
                sentence_start(&Self::name(ctx, ParlorAction::Left)),
                Self::name(ctx, ParlorAction::Right),
                sentence_start(&p)
            ),
        }
    }
}

impl State<Parlor> for DeskRun {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "winter_wonderland_wish_desk"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(Some(sound::FIRE));
        if !self.lesson {
            // The lesson's step prompt stands in for the briefing.
            let brief = self.briefing(ctx);
            ctx.say(brief);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.cancel_wrap(ctx, false);
        self.music.stop(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.cancel_wrap(ctx, false);
        match self.phase {
            Phase::Briefing => {
                let brief = self.briefing(ctx);
                ctx.say(brief);
            }
            Phase::Playing => {
                let status = self.status().remove(0);
                ctx.say(format!("Back at the desk. {status}"));
            }
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        let Some(action) = action else {
            match event.pressed() {
                Some(hearken::Key::Escape) => {
                    self.cancel_wrap(ctx, false);
                    pause(ctx, &self.mode);
                }
                Some(_) => {
                    let r = self.reminder(ctx);
                    ctx.say(r);
                }
                None => {}
            }
            return;
        };
        if event.pressed().is_none() {
            return;
        }
        match (self.phase, action) {
            (_, ParlorAction::Status) => {
                let layers = self.status();
                self.status_key.speak(ctx, layers);
            }
            (Phase::Briefing, ParlorAction::Primary) => self.begin(ctx),
            (Phase::Briefing, ParlorAction::Repeat | ParlorAction::Secondary) => {
                let b = self.briefing(ctx);
                ctx.say(b);
            }
            (Phase::Briefing, ParlorAction::Hint) => {
                let p = Self::name(ctx, ParlorAction::Primary);
                ctx.say(format!(
                    "Each letter wishes for one thing. Think about how it would sound in a box. Press {p} when you are ready."
                ));
            }
            (Phase::Briefing, _) => {
                let r = self.reminder(ctx);
                ctx.say(r);
            }
            (Phase::Playing, ParlorAction::Primary) => {
                self.shake(ctx);
                if !self.stack.here().gone {
                    self.held = Some(0.0);
                }
            }
            (Phase::Playing, ParlorAction::Left) => self.step(ctx, -1),
            (Phase::Playing, ParlorAction::Right) => self.step(ctx, 1),
            (Phase::Playing, ParlorAction::Up) => self.jump_end(ctx, false),
            (Phase::Playing, ParlorAction::Down) => self.jump_end(ctx, true),
            (Phase::Playing, ParlorAction::Repeat | ParlorAction::Secondary) => {
                self.cancel_wrap(ctx, false);
                cue(ctx, sound::DESK_LETTER);
                let l = self.letter_text();
                ctx.say(l);
            }
            (Phase::Playing, ParlorAction::Hint) => {
                self.cancel_wrap(ctx, false);
                self.hint(ctx);
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.phase != Phase::Playing {
            return;
        }
        self.time += dt;
        let answered = self.stack.answered() as f32 / self.stack.letters.len().max(1) as f32;
        self.music
            .follow(ctx, self.time, answered, self.stack.spec.level);
        if let Some((left, text)) = self.pending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                let text = std::mem::take(text);
                self.pending = None;
                ctx.say(text);
            }
        }
        self.follow_hold(ctx, dt);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let s = &self.stack;
        let mut out = vec![format!(
            "Wish Desk, stack {} ({}), letter {} of {}, box {} of {}, score {}",
            s.spec.level,
            if self.phase == Phase::Briefing {
                "briefing"
            } else {
                "playing"
            },
            (s.current + 1).min(s.letters.len()),
            s.letters.len(),
            s.at + 1,
            s.shelf.len(),
            self.score()
        )];
        if let Some(l) = s.letter() {
            out.push(format!(
                "Letter: {} wants {} ({})",
                l.from,
                l.item,
                l.wish.name()
            ));
        }
        for (i, p) in s.shelf.iter().enumerate() {
            let what = if p.gone {
                "gone".to_string()
            } else if p.shakes == 0 {
                "not shaken".to_string()
            } else if s.spec.straw && p.shakes == 1 {
                "straw".to_string()
            } else {
                p.kind.name().to_string()
            };
            out.push(format!("Box {}: {what}", i + 1));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let n = |a| ctx.control_name(a);
        match self.phase {
            Phase::Briefing => format!(
                "Wish Desk briefing. {} opens the first letter. {} repeats the briefing, {} speaks the stack, Escape pauses.",
                sentence_start(&n(ParlorAction::Primary)),
                sentence_start(&n(ParlorAction::Repeat)),
                n(ParlorAction::Status)
            ),
            Phase::Playing => format!(
                "Wish Desk. {} and {} walk the shelf, {} and {} jump to its ends. {} shakes the box in front of you. Hold {} to wrap it for the letter, and let go early to stop. {} reads the letter again, {} says where you are, {} gives a hint at half points. Escape pauses.",
                sentence_start(&n(ParlorAction::Left)),
                n(ParlorAction::Right),
                n(ParlorAction::Up),
                n(ParlorAction::Down),
                sentence_start(&n(ParlorAction::Primary)),
                n(ParlorAction::Primary),
                sentence_start(&n(ParlorAction::Repeat)),
                sentence_start(&n(ParlorAction::Status)),
                sentence_start(&n(ParlorAction::Hint))
            ),
        }
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        self.cancel_wrap(ctx, false);
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// The story's bridge, then its stack at the desk.
pub fn story_desk(blizzard: bool, gifts: u64, pages: Vec<String>) -> Pages {
    Pages::new(pages).then(move |ctx| {
        ctx.replace(DeskRun::new(
            Mode::Story { blizzard, gifts },
            Mode::story_stack(blizzard),
        ))
    })
}

mod visuals_deskrun;
