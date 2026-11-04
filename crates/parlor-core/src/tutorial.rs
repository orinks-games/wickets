//! Interactive tutorials: a game teaches by having the player do things.
//!
//! A [`Lesson`] is a run of [`Step`]s. Each step asks for one thing ("Left
//! steps you one board left. Step to board 15."), waits for the player to
//! do it, answers every press in the game's own sounds and words, and moves
//! on when the thing is done. Help keeps the full guide as pages; the
//! lesson is where a new player learns.
//!
//! Every game gives one through `GameInfo::tutorial`, and its title's
//! Tutorial row opens it. Story mode opens the same lesson for a game the
//! player has never played, with [`needs_lesson`] deciding.
//!
//! A step can be a drill, where the lesson answers each press itself, or
//! live: the game's own play screen runs inside the lesson (see [`Live`])
//! until the player has done what the step asks, such as hit one heart.
//!
//! In a drill the repeat control says the step again, with its sound; in a
//! live step the game keeps its own keys. F1 says which step this is and
//! what it asks; Escape leaves.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use hearken::text::end_sentence;
use hearken::{Bus, Ctx, InputEvent, Key, State, UiSound};

use crate::pages::PAGE_SOUND;
use crate::{Parlor, ParlorAction, Records};

/// What a step makes of one press.
pub enum Reply {
    /// The step is done. The line is said, then the next step's prompt.
    Done(String),
    /// Heard, and the step goes on. The line is said.
    Say(String),
    /// Not what this step asks for: the step's nudge is said, or its
    /// prompt again.
    Nudge,
}

impl Reply {
    pub fn done(line: impl Into<String>) -> Reply {
        Reply::Done(line.into())
    }
    pub fn say(line: impl Into<String>) -> Reply {
        Reply::Say(line.into())
    }
}

type OnAction = Box<dyn FnMut(&mut Ctx<Parlor>, ParlorAction) -> Reply>;
type OnTick = Box<dyn FnMut(&mut Ctx<Parlor>, f64) -> Option<Reply>>;
type OnStart = Box<dyn FnMut(&mut Ctx<Parlor>)>;
type Done = Box<dyn FnMut(&mut Ctx<Parlor>)>;
type Check = Box<dyn FnMut(&mut Ctx<Parlor>) -> Option<String>>;
/// What a control answers with in [`Step::each`].
pub type Reaction = Box<dyn FnMut(&mut Ctx<Parlor>) -> String>;

/// One thing the player learns by doing it.
pub struct Step {
    prompt: String,
    nudge: Option<String>,
    on_start: Option<OnStart>,
    on_action: OnAction,
    on_tick: Option<OnTick>,
    /// Enter (with no action bound to it) finishes the step.
    on_enter: Option<String>,
    /// The game screen a live step runs, and what ends the step.
    live: Option<(Rc<dyn LiveScreen>, Check)>,
    /// The step answers the repeat control itself, so F1 says the step.
    own_repeat: bool,
}

impl Step {
    /// A step that answers each press with `on_action`. The prompt says
    /// what to do and names the control, built with `ctx.control_name`.
    pub fn new(
        prompt: impl Into<String>,
        on_action: impl FnMut(&mut Ctx<Parlor>, ParlorAction) -> Reply + 'static,
    ) -> Step {
        Step {
            prompt: prompt.into(),
            nudge: None,
            on_start: None,
            on_action: Box::new(on_action),
            on_tick: None,
            on_enter: None,
            live: None,
            own_repeat: false,
        }
    }

    /// Press one control. `react` plays what the game plays for it and
    /// gives the line that ends the step.
    pub fn press(
        prompt: impl Into<String>,
        action: ParlorAction,
        mut react: impl FnMut(&mut Ctx<Parlor>) -> String + 'static,
    ) -> Step {
        Step::new(prompt, move |ctx, a| {
            if a == action {
                Reply::Done(react(ctx))
            } else {
                Reply::Nudge
            }
        })
    }

    /// Press each of these controls once, in any order. Each answers with
    /// its own reaction; the step ends with `done` once all are pressed.
    pub fn each(
        prompt: impl Into<String>,
        reactions: Vec<(ParlorAction, Reaction)>,
        done: impl Into<String>,
    ) -> Step {
        let done = done.into();
        let mut left: Vec<bool> = vec![true; reactions.len()];
        let mut reactions = reactions;
        Step::new(prompt, move |ctx, a| {
            let Some(i) = reactions.iter().position(|(r, _)| *r == a) else {
                return Reply::Nudge;
            };
            let line = (reactions[i].1)(ctx);
            left[i] = false;
            if left.iter().any(|&l| l) {
                Reply::Say(line)
            } else {
                Reply::Done(format!("{} {done}", end_sentence(&line)))
            }
        })
    }

    /// A step that is only words, for a rule there is nothing to press for
    /// yet. Enter, or the main action, goes on. Use few: the player learns
    /// by doing.
    pub fn note(text: impl Into<String>) -> Step {
        let mut step = Step::new(text, |_ctx, a| {
            if a == ParlorAction::Primary {
                Reply::Done(String::new())
            } else {
                Reply::Nudge
            }
        });
        step.on_enter = Some(String::new());
        step
    }

    /// Runs as the step begins, and again when the player asks to hear it
    /// again: play the sound the step is about.
    pub fn start(mut self, f: impl FnMut(&mut Ctx<Parlor>) + 'static) -> Step {
        self.on_start = Some(Box::new(f));
        self
    }

    /// Runs every frame while the step is open, for a step about timing.
    /// Return a reply to speak it, or end the step.
    pub fn tick(mut self, f: impl FnMut(&mut Ctx<Parlor>, f64) -> Option<Reply> + 'static) -> Step {
        self.on_tick = Some(Box::new(f));
        self
    }

    /// What a press that does not fit says, in place of the prompt.
    pub fn nudge(mut self, text: impl Into<String>) -> Step {
        self.nudge = Some(text.into());
        self
    }

    /// The step gives the repeat control its own job, such as reading the
    /// rows rung: help then names F1, not repeat, for hearing the step again.
    pub fn own_repeat(mut self) -> Step {
        self.own_repeat = true;
        self
    }

    /// The step's prompt, as it is spoken.
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

/// A game screen run inside a lesson, shared by the live steps made from
/// it, so one practice field can carry several steps. It is entered when
/// its first step begins and left when the lesson moves past it or closes.
/// What the screen says as it is entered is kept quiet, so the step's
/// prompt is heard alone: write the prompt to say where the player is.
pub struct Live<S> {
    cell: Rc<LiveCell<S>>,
}

/// Looks at a key before the live screen does: a line keeps the key from
/// it, and is said instead.
type Guard<S> = Box<dyn Fn(&S, &mut Ctx<Parlor>, &InputEvent) -> Option<String>>;

struct LiveCell<S> {
    screen: RefCell<S>,
    entered: Cell<bool>,
    guard: RefCell<Option<Guard<S>>>,
}

trait LiveScreen {
    fn begin(&self, ctx: &mut Ctx<Parlor>);
    fn input(&self, ctx: &mut Ctx<Parlor>, event: &InputEvent);
    fn update(&self, ctx: &mut Ctx<Parlor>, dt: f64);
    fn resume(&self, ctx: &mut Ctx<Parlor>);
    fn resume_quiet(&self, ctx: &mut Ctx<Parlor>);
    fn end(&self, ctx: &mut Ctx<Parlor>);
    fn lines(&self, ctx: &Ctx<Parlor>) -> Vec<String>;
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene;
}

impl<S: State<Parlor>> LiveScreen for LiveCell<S> {
    fn begin(&self, ctx: &mut Ctx<Parlor>) {
        if !self.entered.replace(true) {
            // The step's prompt sets the scene, so the screen's own opening
            // (where you are, the controls) is not said over it. Its sounds
            // still start.
            unspoken(ctx, |ctx| self.screen.borrow_mut().enter(ctx));
        }
    }
    fn input(&self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if let Some(guard) = self.guard.borrow().as_ref() {
            if let Some(line) = guard(&self.screen.borrow(), ctx, event) {
                ctx.say(line);
                return;
            }
        }
        self.screen.borrow_mut().handle_input(ctx, event);
    }
    fn update(&self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.screen.borrow_mut().update(ctx, dt);
    }
    fn resume(&self, ctx: &mut Ctx<Parlor>) {
        self.screen.borrow_mut().resume(ctx);
    }
    fn resume_quiet(&self, ctx: &mut Ctx<Parlor>) {
        self.screen.borrow_mut().resume_quiet(ctx);
    }
    fn end(&self, ctx: &mut Ctx<Parlor>) {
        if self.entered.replace(false) {
            self.screen.borrow_mut().exit(ctx);
        }
    }
    fn lines(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        self.screen.borrow().lines(ctx)
    }
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.screen.borrow().visuals(ctx)
    }
}

/// Runs `f` with nothing it says reaching the voice, the transcript or the
/// log.
fn unspoken(ctx: &mut Ctx<Parlor>, f: impl FnOnce(&mut Ctx<Parlor>)) {
    let speech = std::mem::replace(&mut ctx.speech, Box::new(hearken::speech::NullSpeech));
    let (lines, said) = (ctx.transcript.len(), ctx.said);
    let level = log::max_level();
    log::set_max_level(log::LevelFilter::Off);
    f(ctx);
    log::set_max_level(level);
    ctx.transcript.truncate(lines);
    ctx.said = said;
    ctx.speech = speech;
}

impl<S> Clone for Live<S> {
    fn clone(&self) -> Self {
        Live {
            cell: Rc::clone(&self.cell),
        }
    }
}

impl<S: State<Parlor> + 'static> Live<S> {
    /// The screen to run, such as the game's practice field.
    pub fn new(screen: S) -> Live<S> {
        Live {
            cell: Rc::new(LiveCell {
                screen: RefCell::new(screen),
                entered: Cell::new(false),
                guard: RefCell::new(None),
            }),
        }
    }

    /// Keeps keys from the screen that would leave the lesson's ground,
    /// such as one that finishes and scores a run: `guard` sees each key
    /// first, and a line it gives is said instead.
    pub fn guard(
        self,
        guard: impl Fn(&S, &mut Ctx<Parlor>, &InputEvent) -> Option<String> + 'static,
    ) -> Live<S> {
        *self.cell.guard.borrow_mut() = Some(Box::new(guard));
        self
    }

    /// A look at the screen, for a test that plays the lesson.
    pub fn with<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        f(&self.cell.screen.borrow())
    }

    /// A change to the screen from a step's `start`, such as holding a
    /// sound back until the step's prompt has been said, or sending a
    /// ghost as the step begins.
    pub fn with_mut<R>(&self, f: impl FnOnce(&mut S) -> R) -> R {
        f(&mut self.cell.screen.borrow_mut())
    }

    /// A step on this screen. The screen gets every key but Escape, and
    /// after each key and frame `check` looks at it: a line ends the step.
    pub fn step(
        &self,
        prompt: impl Into<String>,
        mut check: impl FnMut(&S, &mut Ctx<Parlor>) -> Option<String> + 'static,
    ) -> Step {
        let cell = Rc::clone(&self.cell);
        let mut step = Step::new(prompt, |_ctx, _a| Reply::Nudge);
        let screen: Rc<dyn LiveScreen> = Rc::clone(&self.cell) as Rc<dyn LiveScreen>;
        step.live = Some((
            screen,
            Box::new(move |ctx| check(&cell.screen.borrow(), ctx)),
        ));
        step
    }
}

/// A game's interactive tutorial. See the module docs.
pub struct Lesson {
    id: &'static str,
    name: String,
    intro: String,
    steps: Vec<Step>,
    outro: String,
    at: usize,
    begun: bool,
    done: Option<Done>,
    cleanup: Option<Done>,
}

impl Lesson {
    /// A lesson for game `id`, called `name` ("Midnight Lanes"). The intro
    /// is one or two sentences on what the game is, said before the first
    /// step.
    pub fn new(id: &'static str, name: &str, intro: impl Into<String>) -> Lesson {
        Lesson {
            id,
            name: name.to_string(),
            intro: intro.into(),
            steps: Vec::new(),
            outro: String::new(),
            at: 0,
            begun: false,
            done: None,
            cleanup: None,
        }
    }

    pub fn step(mut self, step: Step) -> Lesson {
        self.steps.push(step);
        self
    }

    /// Said once the last step is done: where to go next, such as the
    /// practice row.
    pub fn outro(mut self, text: impl Into<String>) -> Lesson {
        self.outro = text.into();
        self
    }

    /// What to do once the lesson is finished, in place of going back to
    /// the screen beneath with the closing line. The closing line is said
    /// first. Story mode uses this to carry on with its programme.
    pub fn then(mut self, done: impl FnMut(&mut Ctx<Parlor>) + 'static) -> Lesson {
        self.done = Some(Box::new(done));
        self
    }

    /// Runs when the lesson closes, finished or not: stop the loops its
    /// steps started.
    pub fn on_exit(mut self, f: impl FnMut(&mut Ctx<Parlor>) + 'static) -> Lesson {
        self.cleanup = Some(Box::new(f));
        self
    }

    /// How many steps the lesson has.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The game this lesson teaches.
    pub fn id(&self) -> &'static str {
        self.id
    }

    /// Every step's prompt, in order, for tests and the text check.
    pub fn prompts(&self) -> Vec<&str> {
        self.steps.iter().map(Step::prompt).collect()
    }

    fn start_step(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(step) = self.steps.get_mut(self.at) {
            if let Some(start) = step.on_start.as_mut() {
                start(ctx);
            }
            if let Some((live, _)) = &step.live {
                live.begin(ctx);
            }
        }
    }

    /// Runs the live step's check, and ends the step on a line.
    fn check_live(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(step) = self.steps.get_mut(self.at) else {
            return;
        };
        let Some((_, check)) = step.live.as_mut() else {
            return;
        };
        if let Some(line) = check(ctx) {
            self.advance(ctx, line);
        }
    }

    fn end_live(&mut self, ctx: &mut Ctx<Parlor>) {
        for step in &self.steps {
            if let Some((live, _)) = &step.live {
                live.end(ctx);
            }
        }
    }

    fn reply(&mut self, ctx: &mut Ctx<Parlor>, reply: Reply) {
        match reply {
            Reply::Say(line) => ctx.say(line),
            Reply::Nudge => {
                let step = &self.steps[self.at];
                let line = step.nudge.clone().unwrap_or_else(|| step.prompt.clone());
                ctx.say(line);
            }
            Reply::Done(line) => self.advance(ctx, line),
        }
    }

    fn advance(&mut self, ctx: &mut Ctx<Parlor>, line: String) {
        let was = self.steps[self.at].live.as_ref().map(|(l, _)| Rc::clone(l));
        self.at += 1;
        let next = self
            .steps
            .get(self.at)
            .and_then(|s| s.live.as_ref())
            .map(|(l, _)| Rc::clone(l));
        if let Some(was) = was {
            if !next.is_some_and(|n| Rc::ptr_eq(&n, &was)) {
                was.end(ctx);
            }
        }
        let said = if line.trim().is_empty() {
            String::new()
        } else {
            format!("{} ", end_sentence(&line))
        };
        if self.at < self.steps.len() {
            ctx.audio.play(PAGE_SOUND, Bus::UI);
            ctx.say(format!("{said}{}", self.steps[self.at].prompt));
            self.start_step(ctx);
        } else {
            self.finish(ctx, said);
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>, said: String) {
        mark_tutorial_done(ctx, self.id);
        ctx.play_ui(UiSound::Notify);
        let closing = format!("{said}Tutorial done. {}", self.outro);
        let closing = closing.trim().to_string();
        match self.done.take() {
            Some(mut done) => {
                ctx.say(closing);
                done(ctx);
            }
            None => ctx.pop_with(closing),
        }
    }
}

impl State<Parlor> for Lesson {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        let mut scene = self
            .steps
            .get(self.at)
            .and_then(|step| step.live.as_ref())
            .map(|(live, _)| live.visuals(ctx))
            .unwrap_or_default();
        if let Some(step) = self.steps.get(self.at) {
            let mut lines = vec![
                format!("Step {} of {}", self.at + 1, self.steps.len()),
                step.prompt.clone(),
            ];
            lines.append(&mut scene.lines);
            scene.lines = lines;
        }
        scene
    }

    fn name(&self) -> &'static str {
        "lesson"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.begun {
            return self.resume(ctx);
        }
        self.begun = true;
        ctx.audio.play(PAGE_SOUND, Bus::UI);
        let intro = end_sentence(&self.intro);
        match self.steps.first() {
            Some(step) => {
                ctx.say(format!("{intro} {}", step.prompt));
                self.start_step(ctx);
            }
            None => {
                ctx.say(intro);
                self.finish(ctx, String::new());
            }
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.end_live(ctx);
        if let Some(cleanup) = self.cleanup.as_mut() {
            cleanup(ctx);
        }
    }

    /// Back from a screen on top. A live game says where the player is, as
    /// it would after its own menus and pages; a drill says its step.
    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(step) = self.steps.get(self.at) else {
            return;
        };
        match &step.live {
            Some((live, _)) => {
                let live = Rc::clone(live);
                live.resume(ctx);
                self.check_live(ctx);
            }
            None => {
                ctx.say(step.prompt.clone());
                self.start_step(ctx);
            }
        }
    }

    /// Back from a screen the live game opened, such as a seed packet: the
    /// game carries on, and the step is not said over it.
    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some((live, _)) = self.steps.get(self.at).and_then(|s| s.live.as_ref()) {
            let live = Rc::clone(live);
            live.resume_quiet(ctx);
            self.check_live(ctx);
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.at >= self.steps.len() {
            return;
        }
        if let Some((live, _)) = &self.steps[self.at].live {
            if event.pressed() == Some(Key::Escape) {
                ctx.play_ui(UiSound::Back);
                ctx.pop_with("Tutorial left.");
                return;
            }
            let live = Rc::clone(live);
            live.input(ctx, event);
            self.check_live(ctx);
            return;
        }
        if event.is_repeat() {
            return;
        }
        if let Some(action) = ctx.action_for(event) {
            let reply = (self.steps[self.at].on_action)(ctx, action);
            if matches!(reply, Reply::Nudge) && action == ParlorAction::Repeat {
                ctx.say(self.steps[self.at].prompt.clone());
                self.start_step(ctx);
                return;
            }
            self.reply(ctx, reply);
            return;
        }
        match event.pressed() {
            Some(Key::Escape) => {
                ctx.play_ui(UiSound::Back);
                ctx.pop_with("Tutorial left.");
            }
            Some(Key::Return | Key::KpEnter) => match self.steps[self.at].on_enter.clone() {
                Some(line) => self.advance(ctx, line),
                None => self.reply(ctx, Reply::Nudge),
            },
            Some(_) => self.reply(ctx, Reply::Nudge),
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Some(step) = self.steps.get_mut(self.at) else {
            return;
        };
        if let Some((live, _)) = &step.live {
            let live = Rc::clone(live);
            live.update(ctx, dt);
            self.check_live(ctx);
            return;
        }
        let Some(tick) = step.on_tick.as_mut() else {
            return;
        };
        if let Some(reply) = tick(ctx, dt) {
            self.reply(ctx, reply);
        }
    }

    fn lines(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        match self.steps.get(self.at) {
            Some(step) => {
                let mut lines = vec![
                    format!("Step {} of {}", self.at + 1, self.steps.len()),
                    step.prompt.clone(),
                ];
                if let Some((live, _)) = &step.live {
                    lines.extend(live.lines(ctx));
                }
                lines
            }
            None => Vec::new(),
        }
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let step = self.steps.get(self.at);
        let prompt = step.map(|s| s.prompt.as_str()).unwrap_or_default();
        let again = if step.is_some_and(|s| s.live.is_some() || s.own_repeat) {
            "F1 says this step again.".to_string()
        } else {
            format!(
                "{} says this step again.",
                hearken::text::sentence_start(&ctx.control_name(ParlorAction::Repeat))
            )
        };
        format!(
            "{} tutorial, step {} of {}. {prompt} {again} Escape leaves the tutorial.",
            self.name,
            (self.at + 1).min(self.steps.len()),
            self.steps.len(),
        )
    }
}

fn tutorial_key(id: &str) -> String {
    // The key from when the tutorial was pages: a player who read those
    // counts as taught.
    format!("{id}/tutorial_read")
}

fn played_key(id: &str) -> String {
    format!("{id}/played")
}

/// Whether the player has finished this game's tutorial.
pub fn tutorial_done(ctx: &mut Ctx<Parlor>, id: &str) -> bool {
    Records::load(ctx, &tutorial_key(id))
}

/// Notes that the player has finished this game's tutorial.
pub fn mark_tutorial_done(ctx: &mut Ctx<Parlor>, id: &str) {
    Records::store(ctx, &tutorial_key(id), &true);
}

/// Whether the player has played this game: finished a run, saved a score
/// or played a challenge, from anywhere in the parlor.
pub fn has_played(ctx: &mut Ctx<Parlor>, id: &str) -> bool {
    if Records::load::<bool>(ctx, &played_key(id)) {
        return true;
    }
    // Saves from before the flag: any score or challenge result counts.
    let scores: std::collections::BTreeMap<String, u64> =
        Records::load(ctx, &format!("{id}/scores"));
    let challenges: std::collections::BTreeMap<String, u64> =
        Records::load(ctx, &format!("{id}/challenges"));
    !scores.is_empty() || !challenges.is_empty()
}

/// Notes that the player has played this game. Scores, challenge results
/// and the shared results screen call it; a game with none of those calls
/// it when a session ends.
pub fn mark_played(ctx: &mut Ctx<Parlor>, id: &str) {
    if ctx.game.story_scene == Some(id) {
        ctx.game.story_done = true;
    }
    note_played(ctx, id);
}

/// Saves that the player has played this game before, without finishing
/// a story scene: for play found in the game's own saves.
pub fn note_played(ctx: &mut Ctx<Parlor>, id: &str) {
    if !Records::load::<bool>(ctx, &played_key(id)) {
        Records::store(ctx, &played_key(id), &true);
    }
}

/// Whether a new player should be taught this game: never played and
/// never taught. Story mode skips the lesson when this is false.
pub fn needs_lesson(ctx: &mut Ctx<Parlor>, id: &str) -> bool {
    !tutorial_done(ctx, id) && !has_played(ctx, id)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::menus::testing::Base;
    use crate::testkit::press_action;
    use hearken::Headless;

    fn lesson() -> Lesson {
        let mut board = 20;
        Lesson::new("dummy", "Dummy", "Welcome to Dummy")
            .step(Step::new(
                "Left steps left. Step to board 18.",
                move |_ctx, a| match a {
                    ParlorAction::Left => {
                        board -= 1;
                        if board == 18 {
                            Reply::done("Board 18")
                        } else {
                            Reply::say(format!("Board {board}"))
                        }
                    }
                    _ => Reply::Nudge,
                },
            ))
            .step(Step::press("Space bowls.", ParlorAction::Primary, |_ctx| {
                "Strike".to_string()
            }))
            .outro("Play is ready")
    }

    fn open(lesson: Lesson) -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(lesson);
        h.frame(0.0);
        h
    }

    #[test]
    fn steps_answer_and_advance() {
        let mut h = open(lesson());
        assert_eq!(
            h.spoken(),
            ["Welcome to Dummy. Left steps left. Step to board 18."]
        );
        h.clear();
        press_action(&mut h, ParlorAction::Up);
        assert_eq!(h.spoken(), ["Left steps left. Step to board 18."]);
        h.clear();
        press_action(&mut h, ParlorAction::Left);
        assert_eq!(h.spoken(), ["Board 19"]);
        h.clear();
        press_action(&mut h, ParlorAction::Left);
        assert_eq!(h.spoken(), ["Board 18. Space bowls."]);
        assert!(needs_lesson(&mut h.app.ctx, "dummy"));
        h.clear();
        press_action(&mut h, ParlorAction::Primary);
        assert_eq!(h.app.top_name(), "base");
        assert!(tutorial_done(&mut h.app.ctx, "dummy"));
        assert!(!needs_lesson(&mut h.app.ctx, "dummy"));
    }

    #[test]
    fn repeat_says_the_step_again_and_escape_leaves() {
        let mut h = open(lesson());
        h.clear();
        press_action(&mut h, ParlorAction::Repeat);
        assert_eq!(h.spoken(), ["Left steps left. Step to board 18."]);
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "base");
        assert!(!tutorial_done(&mut h.app.ctx, "dummy"));
    }

    #[test]
    fn help_names_f1_when_a_step_keeps_repeat_for_itself() {
        let mut h = open(lesson());
        h.clear();
        h.press(Key::F1);
        assert!(
            h.spoken()[0].contains("R says this step again."),
            "{:?}",
            h.spoken()
        );
        let step = Step::new("R reads the log.", |_ctx, _a| Reply::Say("Log".into())).own_repeat();
        let mut h = open(Lesson::new("dummy", "Dummy", "Hi").step(step));
        h.clear();
        h.press(Key::F1);
        assert!(
            h.spoken()[0].contains("F1 says this step again."),
            "{:?}",
            h.spoken()
        );
        assert!(!h.spoken()[0].contains("R says"), "{:?}", h.spoken());
    }

    #[test]
    fn then_runs_in_place_of_going_back() {
        let ran = Rc::new(Cell::new(false));
        let flag = Rc::clone(&ran);
        let mut h = open(lesson().then(move |ctx| {
            flag.set(true);
            ctx.pop();
        }));
        press_action(&mut h, ParlorAction::Left);
        press_action(&mut h, ParlorAction::Left);
        h.clear();
        press_action(&mut h, ParlorAction::Primary);
        assert!(ran.get());
        assert_eq!(h.spoken(), ["Strike. Tutorial done. Play is ready"]);
    }

    #[test]
    fn each_needs_every_control() {
        let mut h = open(Lesson::new("dummy", "Dummy", "Hi").step(Step::each(
            "Left and Right.",
            vec![
                (
                    ParlorAction::Left,
                    Box::new(|_: &mut Ctx<Parlor>| "West".to_string()),
                ),
                (
                    ParlorAction::Right,
                    Box::new(|_: &mut Ctx<Parlor>| "East".to_string()),
                ),
            ],
            "Both ways",
        )));
        press_action(&mut h, ParlorAction::Left);
        press_action(&mut h, ParlorAction::Left);
        h.clear();
        press_action(&mut h, ParlorAction::Right);
        assert_eq!(h.spoken(), ["East. Both ways. Tutorial done."]);
    }

    #[test]
    fn notes_go_on_with_enter() {
        let mut h = open(
            Lesson::new("dummy", "Dummy", "Hi")
                .step(Step::note("A rule."))
                .step(Step::press("Space.", ParlorAction::Primary, |_| {
                    "Done".into()
                })),
        );
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Space."]);
    }

    /// A stand-in play screen: counts Space presses.
    struct Field {
        shots: u32,
        entered: Rc<Cell<u32>>,
    }

    impl State<Parlor> for Field {
        fn name(&self) -> &'static str {
            "field"
        }
        fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
            self.entered.set(self.entered.get() + 1);
            ctx.say("Practice field.");
        }
        fn exit(&mut self, _ctx: &mut Ctx<Parlor>) {
            self.entered.set(self.entered.get() + 100);
        }
        fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
            if ctx.action_for(event) == Some(ParlorAction::Primary) {
                self.shots += 1;
                ctx.say("Shot.");
            }
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    #[test]
    fn live_steps_run_the_game_screen_until_done() {
        let entered = Rc::new(Cell::new(0));
        let field = Live::new(Field {
            shots: 0,
            entered: Rc::clone(&entered),
        });
        let mut h = open(
            Lesson::new("dummy", "Dummy", "Hi")
                .step(field.step("Shoot once.", |f, _| (f.shots >= 1).then(|| "One".into())))
                .step(field.step("Shoot twice more.", |f, _| {
                    (f.shots >= 3).then(|| "Three".into())
                })),
        );
        assert_eq!(h.spoken(), ["Hi. Shoot once."]);
        h.clear();
        press_action(&mut h, ParlorAction::Primary);
        assert_eq!(h.spoken(), ["Shot.", "One. Shoot twice more."]);
        press_action(&mut h, ParlorAction::Primary);
        press_action(&mut h, ParlorAction::Primary);
        assert_eq!(h.app.top_name(), "base");
        assert_eq!(entered.get(), 101);
    }

    #[test]
    fn scores_count_as_played() {
        let mut h = open(lesson());
        assert!(!has_played(&mut h.app.ctx, "other"));
        Records::record(&mut h.app.ctx, "other", "best", 3);
        assert!(has_played(&mut h.app.ctx, "other"));
    }
}
