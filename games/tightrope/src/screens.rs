//! The wire itself and what surrounds a run: the pages before it, the
//! results after it, and challenge runs.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self as shared, labels, Back, Leave, Results};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::{
    Challenge, Challenges, Mood, Pace, Pages, Parlor, ParlorAction, Period, Rng, RunClock, Score,
    ScoreResult,
};

use crate::cues;
use crate::progress;
use crate::rules::{Keys, Mode, Note, Phase, Plan, Run, Side, DANGER_LEAN, RISER, SKYLINE_LEN};
use crate::HELP;

/// The title's screen name.
pub const TITLE_NAME: &str = "tightrope";
/// The wire's screen name.
pub const RUN_NAME: &str = "tightrope_run";
/// The challenges menu's screen name. A challenge opened from the launcher
/// sits on a host of the same name, so leaving it leads back to wherever
/// the run began.
pub const CHALLENGES_NAME: &str = "tightrope_challenges";

/// The player's controls by name.
pub fn keys(ctx: &Ctx<Parlor>) -> Keys {
    Keys {
        left: ctx.control_name(ParlorAction::Left),
        right: ctx.control_name(ParlorAction::Right),
        step: ctx.control_name(ParlorAction::Primary),
        shoo: ctx.control_name(ParlorAction::Secondary),
        crouch: ctx.control_name(ParlorAction::Down),
        stand: ctx.control_name(ParlorAction::Up),
        status: ctx.control_name(ParlorAction::Status),
        hint: ctx.control_name(ParlorAction::Hint),
    }
}

/// The wire controls a page at a time, for F1 on the wire: the keys
/// every step needs first, then gusts and pigeons, then the rest.
pub fn controls_pages(ctx: &Ctx<Parlor>) -> Vec<String> {
    let k = keys(ctx);
    vec![
        format!(
            "{} and {} shift your weight once per press, opposite your lean. Holding adds no further shift; release between presses. {} steps, only when balanced. {} says your lean; again for lives and score.",
            sentence_start(&k.left),
            k.right,
            sentence_start(&k.step),
            sentence_start(&k.status),
        ),
        format!(
            "{} crouches through a gust, {} stands. {} shoos a pigeon within 2 steps.",
            sentence_start(&k.crouch),
            k.stand,
            sentence_start(&k.shoo),
        ),
        format!(
            "{} repeats. {} gives advice. Escape pauses.",
            sentence_start(&ctx.control_name(ParlorAction::Repeat)),
            sentence_start(&k.hint),
        ),
    ]
}

/// What starting a run needs: the mode, and a seed for tests and
/// challenges (`None` seeds from the clock).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub mode: Mode,
    pub seed: Option<u64>,
}

impl Launch {
    /// The run this launch starts. A challenge builds its plan from its
    /// seed, so every player walks the same wires.
    pub fn build(&self) -> Run {
        match self.mode {
            Mode::Challenge(period) => {
                let mut rng = Rng::seeded(self.seed.unwrap_or(0));
                let plan = Plan::challenge(period, &mut rng);
                Run::new(plan, rng)
            }
            mode => {
                let rng = self.seed.map_or_else(Rng::from_time, Rng::seeded);
                Run::of(mode, rng)
            }
        }
    }
}

/// The page before a run.
fn intro_page(mode: Mode) -> String {
    match mode {
        Mode::Training => "The practice wire: a short, low wire over the garden. One gust and one pigeon come along, each explained as it arrives. Falling costs nothing here.".to_string(),
        Mode::Skyline => format!("The skyline run: {SKYLINE_LEN} crossings, rooftop to rooftop, each longer and higher than the last, with 3 lives. Faster crossings and crossings with no falls score more."),
        Mode::Night => "The night skyline: the same 10 crossings in the dark, with a twitchier wire, harder wind and busier pigeons. Every gust is still warned. 3 lives.".to_string(),
        Mode::Single(n) => format!("Crossing {n} on its own, for your best time. 3 lives."),
        Mode::Challenge(p) => format!("{} challenge.", p.label()),
    }
}

/// The page before a run, then the run. Escape on the page backs out.
/// `replace` swaps out the screen that asked (the results' Play again).
pub fn start(ctx: &mut Ctx<Parlor>, launch: Launch, replace: bool) {
    let pages = Pages::briefing([intro_page(launch.mode)]).then(move |ctx| {
        let screen = RunScreen::new(launch, None);
        ctx.replace(screen)
    });
    if replace {
        ctx.replace(pages);
    } else {
        ctx.push(pages);
    }
}

/// What each period's challenge is about.
pub fn period_text(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's wire: one crossing with a twist.",
        Period::Weekly => "This week's run: 3 high crossings in a row, on only 2 lives.",
        Period::Monthly => "This month's event, the Grand Span: one immense wire 60 floors up, a single life, and the wind changes side with every gust.",
    }
}

/// The challenge of `period` on this day, and its plan.
pub fn challenge_plan(ctx: &Ctx<Parlor>, period: Period) -> (Challenge, Plan) {
    let challenge = Challenge::current(ctx, crate::ID, period);
    let plan = Plan::challenge(period, &mut challenge.rng());
    (challenge, plan)
}

/// The wire of the current `period` challenge, ready to walk.
pub fn challenge_run(ctx: &Ctx<Parlor>, period: Period) -> RunScreen {
    let (challenge, _) = challenge_plan(ctx, period);
    let launch = Launch {
        mode: Mode::Challenge(period),
        seed: Some(challenge.seed),
    };
    RunScreen::new(launch, Some(challenge))
}

/// The pages read before a challenge, which then start it. Escape backs
/// out without starting.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let (_, plan) = challenge_plan(ctx, period);
    let name = match period {
        Period::Daily => "Today's wire",
        Period::Weekly => "This week's run",
        Period::Monthly => "The Grand Span",
    };
    let pages = vec![format!(
        "{name}. {} {}",
        plan.modifier.describe(),
        plan.summary()
    )];
    let mut run = Some(challenge_run(ctx, period));
    Pages::briefing(pages).then(move |ctx| {
        if let Some(run) = run.take() {
            ctx.replace(run);
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the run is over (or left from the
/// pause menu) it steps aside, back to the launcher.
pub struct ChallengeHost {
    period: Period,
    opened: bool,
}

impl ChallengeHost {
    pub fn new(period: Period) -> ChallengeHost {
        ChallengeHost {
            period,
            opened: false,
        }
    }
}

impl State<Parlor> for ChallengeHost {
    /// Named as the challenges menu is, so leaving a run, by its results
    /// or the pause menu, lands here and steps aside to the launcher.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            let pages = briefing(ctx, self.period);
            ctx.push(pages);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, _result: &str) {
        ctx.pop();
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        "Opening the Tightrope challenge.".to_string()
    }
}

/// What the wire's waltz hears: it builds step by step across the wire
/// and crossing by crossing, and settles while you fall or rest on a roof.
/// A gust on its way or blowing hurries it; leaning near a fall darkens it.
pub fn mood(run: &Run) -> Mood {
    let route = run.index as f32 / run.levels.len().max(1) as f32;
    match run.phase {
        Phase::Walking => {
            let across = run.pos as f32 / run.level().steps.max(1) as f32;
            Mood::new(0.2 + 0.5 * across + 0.3 * route)
                .with_flag(HURRY, run.gust.is_some())
                .with_flag(DANGER, run.lean.abs() >= DANGER_LEAN)
        }
        Phase::Falling { .. } => Mood::new(0.1),
        Phase::Crossed { .. } => Mood::new(0.15 + 0.3 * route),
        Phase::Over { .. } => Mood::new(0.0),
    }
}

/// The wire.
pub struct RunScreen {
    run: Run,
    /// The run's game clock: its pace, set when the wire begins, and the
    /// hold that keeps the wire still while speech plays.
    pub run_clock: RunClock,
    /// The gust's riser is sounding: it climbs to the moment the gust
    /// hits, so the wire runs through speech until then.
    riser: bool,
    launch: Launch,
    challenge: Option<Challenge>,
    tone: Option<Sound>,
    coo: Option<Sound>,
    street: Option<Sound>,
    wind: Option<Sound>,
    done: bool,
    /// The wire's music, following the walk.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// Walked inside the tutorial, where F1 repeats the step.
    pub in_lesson: bool,
}

/// How loud the street is from `floors` up.
pub fn street_volume(floors: u32) -> f32 {
    (1.0 / (1.0 + floors as f32 / 5.0)).clamp(0.08, 0.8)
}

/// How loud the wind is from `floors` up.
pub fn wind_volume(floors: u32) -> f32 {
    (0.12 + floors as f32 / 90.0).min(0.6)
}

impl RunScreen {
    pub fn new(launch: Launch, challenge: Option<Challenge>) -> RunScreen {
        RunScreen {
            status_key: Default::default(),
            run: launch.build(),
            run_clock: RunClock::new(Pace::Normal),
            riser: false,
            launch,
            challenge,
            tone: None,
            coo: None,
            street: None,
            wind: None,
            done: false,
            score: Score::new(),
            in_lesson: false,
        }
    }

    /// The rules, for tests.
    pub fn run(&self) -> &Run {
        &self.run
    }

    /// The rules, for tests to set up a wire.
    pub fn run_mut(&mut self) -> &mut Run {
        &mut self.run
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        for sound in [
            self.tone.take(),
            self.coo.take(),
            self.street.take(),
            self.wind.take(),
        ]
        .into_iter()
        .flatten()
        {
            ctx.audio.stop(sound, 120);
        }
        self.score.stop(ctx);
    }

    fn play_notes(&mut self, ctx: &mut Ctx<Parlor>) {
        let floors = self.run.level().floors;
        for note in self.run.drain_notes() {
            match note {
                // The screen plays the riser itself, in real time.
                Note::At(key, _) if key == cues::GUST_WARNING => {}
                // A gust, a pigeon, the danger alarm and a new wire come
                // with a line to act on: each gets a fresh hold.
                Note::At(key, x)
                    if key == cues::GUST_ALERT
                        || key == cues::PIGEON_LAND
                        || key == cues::DANGER =>
                {
                    self.run_clock.reset();
                    ctx.audio.play_at(key, Vec3::new(x, 1.0, 0.0), Bus::EFFECTS);
                }
                Note::Say(text) => ctx.say(text),
                Note::Queue(text) => ctx.say_queued(text),
                Note::Cue(key) => {
                    ctx.audio.play(key, Bus::EFFECTS);
                }
                Note::At(key, x) => {
                    ctx.audio.play_at(key, Vec3::new(x, 1.0, 0.0), Bus::EFFECTS);
                }
                Note::Ahead(key, y) => {
                    ctx.audio.play_at(key, Vec3::new(0.0, y, 0.0), Bus::EFFECTS);
                }
                Note::Below(key, x) => {
                    let pos = Vec3::new(x, 3.0, -(floors as f32) * 0.4);
                    let s = ctx.audio.play_at(key, pos, Bus::AMBIENCE);
                    ctx.audio.set_volume(s, street_volume(floors));
                }
                Note::Music(key) => {
                    ctx.audio.play_music(Some(key), false);
                }
                Note::NewWire => {
                    self.run_clock.reset();
                    if let Some(s) = self.street {
                        ctx.audio.set_volume(s, street_volume(floors));
                    }
                    if let Some(s) = self.wind {
                        ctx.audio.set_volume(s, wind_volume(floors));
                    }
                }
            }
        }
    }

    /// Keeps the balance tone and the pigeon's cooing where the rules say.
    fn sync_audio(&mut self, ctx: &mut Ctx<Parlor>) {
        let run = &self.run;
        if let Some(tone) = self.tone {
            if run.phase == Phase::Walking {
                let a = run.lean.abs().min(1.0) as f32;
                ctx.audio.set_volume(tone, 0.3 + 0.45 * a);
                ctx.audio.set_pitch(tone, 1.0 + 0.6 * a);
                ctx.audio
                    .set_position(tone, Some(Vec3::new(run.lean as f32 * 4.0, 1.0, 0.0)));
            } else {
                ctx.audio.set_volume(tone, 0.0);
            }
        }
        match (&run.pigeon, self.coo) {
            (Some(p), coo) => {
                let ahead = p.at.saturating_sub(run.pos) as f32 * 1.5;
                let pos = Vec3::new(p.side.sign() as f32 * 1.0, ahead, 0.0);
                match coo {
                    Some(s) => ctx.audio.set_position(s, Some(pos)),
                    None => {
                        let s = ctx
                            .audio
                            .start_loop(cues::PIGEON_COO, Some(pos), Bus::EFFECTS);
                        ctx.audio.set_volume(s, 0.7);
                        self.coo = Some(s);
                    }
                }
            }
            (None, Some(s)) => {
                ctx.audio.stop(s, 150);
                self.coo = None;
            }
            (None, None) => {}
        }
    }

    /// Starts the gust's riser so it reaches its height as the gust hits.
    /// The riser lasts its own length in real time, so at a slower pace it
    /// starts closer to the gust, and never while the wire is held.
    fn start_riser(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(gust) = self.run.gust.as_ref().filter(|g| !g.blowing()) else {
            self.riser = false;
            return;
        };
        if self.riser
            || self.run_clock.held()
            || gust.warn_left > RISER * self.run_clock.pace().scale()
        {
            return;
        }
        self.riser = true;
        let x = (gust.side.sign() * 4.0) as f32;
        ctx.audio
            .play_at(cues::GUST_WARNING, Vec3::new(x, 1.0, 0.0), Bus::EFFECTS);
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>, won: bool) {
        self.done = true;
        self.stop_sounds(ctx);
        let run = &self.run;
        let pace = self.run_clock.pace();
        let mut lines = vec![self.run.last.clone()];
        if let Some(c) = self.challenge.clone() {
            let score = run.score;
            let mut line = format!("{} score: {score}.", c.title());
            let best = pace.best_word();
            match Challenges::submit_at(ctx, &c, pace, score) {
                ScoreResult::First if pace.separate() => {
                    line.push_str(" Your first relaxed score on this one.")
                }
                ScoreResult::First => line.push_str(" Your first score on this one."),
                ScoreResult::NewBest { previous } => {
                    line.push_str(&format!(" A new {best}, beating {previous}."))
                }
                ScoreResult::Short { best: b } => {
                    line.push_str(&format!(" Your {best} stays {b}."))
                }
            }
            let streak = Challenges::streak(ctx, crate::ID, c.period);
            if streak > 0 {
                line.push_str(&format!(" A {streak} {} streak.", c.period.unit()));
            }
            line.push(' ');
            line.push_str(&c.resets());
            lines.push(line);
        } else {
            lines.extend(progress::save_run(ctx, &self.run, pace));
        }
        let score = self.run.score;
        let launch = self.launch;
        let results = after_run(launch, self.challenge.clone(), won, score, lines.clone());
        let back_to = if self.challenge.is_some() {
            CHALLENGES_NAME
        } else {
            TITLE_NAME
        };
        let mut results = Some(results);
        // The last fall's or the arrival's words are heard first.
        let pages = Pages::new(result_pages(lines))
            .then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.pop_to_quiet(back_to);
                    ctx.push(results);
                }
            })
            .after_speech();
        ctx.replace(pages);
    }
}

impl State<Parlor> for RunScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        RUN_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.run_clock = RunClock::start(ctx, crate::ID);
        ctx.audio.play_music(None, false);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        self.run.keys = keys(ctx);
        // The intro was written with default key names: say it afresh.
        self.run.drain_notes();
        let floors = self.run.level().floors;
        let street = ctx.audio.start_loop(cues::STREET, None, Bus::AMBIENCE);
        ctx.audio.set_volume(street, street_volume(floors));
        self.street = Some(street);
        let wind = ctx.audio.start_loop(cues::WIND, None, Bus::AMBIENCE);
        ctx.audio.set_volume(wind, wind_volume(floors));
        self.wind = Some(wind);
        let tone = ctx
            .audio
            .start_loop(cues::TONE, Some(Vec3::new(0.0, 1.0, 0.0)), Bus::EFFECTS);
        self.tone = Some(tone);
        let mood = mood(&self.run);
        self.score.start(ctx, &cues::SCORE, mood);
        ctx.audio.play(cues::START, Bus::EFFECTS);
        let intro = self.run.intro();
        self.run.last = intro.clone();
        ctx.say(intro);
        self.sync_audio(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.run_clock.reset();
        self.run.keys = keys(ctx);
        ctx.audio.resume_all();
        let status = self.run.status_layers().remove(0);
        ctx.say(format!("Back on the wire. {status}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        // A pad's X, Y and bumpers stand for no key, only for actions.
        let Some(key) = event
            .pressed()
            .or(ctx.action_for(event).map(|_| Key::Other(0)))
        else {
            return;
        };
        if key == Key::Escape {
            if !event.is_repeat() {
                let leave = if self.challenge.is_some() {
                    Leave::Challenge(CHALLENGES_NAME)
                } else {
                    Leave::Title(TITLE_NAME)
                };
                shared::pause(ctx, HELP, leave);
            }
            return;
        }
        if self.done {
            return;
        }
        let action = ctx.action_for(event);
        // One answer per press: a held arrow must not pile shift on shift
        // at the keyboard's repeat rate and throw the walker off.
        if event.is_repeat() {
            return;
        }
        let run = &mut self.run;
        match action {
            Some(ParlorAction::Left) => run.shift(Side::Left),
            Some(ParlorAction::Right) => run.shift(Side::Right),
            Some(ParlorAction::Primary) => run.step(),
            Some(ParlorAction::Secondary) => run.shoo(),
            Some(ParlorAction::Down) => run.crouch(),
            Some(ParlorAction::Up) => run.stand(),
            Some(ParlorAction::Status) => {
                let layers = run.status_layers();
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                if run.last.is_empty() {
                    ctx.say("Nothing to repeat yet.");
                } else {
                    let s = run.last.clone();
                    ctx.say(s);
                }
            }
            Some(ParlorAction::Hint) => {
                let s = run.hint();
                ctx.say(s);
            }
            None => {}
        }
        self.play_notes(ctx);
        self.sync_audio(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.done {
            return;
        }
        // Once the riser sounds, the wire keeps time with it; otherwise
        // speech holds the wire. The music keeps real time.
        if self.riser && self.run.gust.as_ref().is_some_and(|g| !g.blowing()) {
            self.run_clock.bypass();
        }
        let game_dt = self.run_clock.game_dt(ctx, dt);
        self.start_riser(ctx);
        self.run.update(game_dt);
        self.play_notes(ctx);
        self.sync_audio(ctx);
        let mood = mood(&self.run);
        self.score.follow(ctx, mood, dt);
        if let Some(won) = self.run.over() {
            self.finish(ctx, won);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![self.run.status()]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        controls_pages(ctx).join(" ")
    }

    /// F1 on the wire pauses it and reads the controls a page at a time,
    /// so the wire holds still while the player listens.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.done {
            return;
        }
        let mut pages = controls_pages(ctx);
        pages[0] = format!("Paused while you read. {}", pages[0]);
        ctx.audio.pause_all();
        ctx.push(Pages::new(pages));
    }
}

/// The result pages: how the run ended first, then what it set.
pub fn result_pages(lines: Vec<String>) -> Vec<String> {
    let mut lines = lines.into_iter();
    let mut pages: Vec<String> = lines.next().into_iter().collect();
    let rest: Vec<String> = lines.collect();
    if !rest.is_empty() {
        pages.push(rest.join(" "));
    }
    pages
}

/// The results menu after a run: the next crossing when one opened, the
/// same run again, the results again, and back to where the run began.
pub fn after_run(
    launch: Launch,
    challenge: Option<Challenge>,
    won: bool,
    score: u64,
    lines: Vec<String>,
) -> Results {
    let summary = if won {
        format!("{} complete. Score {score}", launch.mode.name())
    } else {
        format!("Run over. Score {score}")
    };
    let pages = result_pages(lines);
    if let (Some(challenge), Mode::Challenge(period)) = (challenge, launch.mode) {
        return Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
            labels::challenge_again(period),
            "The same wires, straight away. Only your best score is kept.",
            move |ctx| {
                ctx.replace(RunScreen::new(launch, Some(challenge.clone())));
            },
        );
    }
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if let (true, Mode::Single(n)) = (won, launch.mode) {
        if n < SKYLINE_LEN {
            let next = n + 1;
            results = results.next(
                format!("Next crossing: crossing {next}"),
                "Walk the next crossing alone, for its best time.",
                move |ctx| {
                    let launch = Launch {
                        mode: Mode::Single(next),
                        ..launch
                    };
                    start(ctx, launch, true)
                },
            );
        }
    }
    let (label, help) = match (launch.mode, won) {
        (Mode::Training, _) => (labels::PLAY_AGAIN, "Walk the practice wire again."),
        (_, true) => (labels::PLAY_AGAIN, "Start the same kind of run again."),
        (_, false) => (labels::TRY_AGAIN, "Start the same kind of run again."),
    };
    results.again(label, help, move |ctx| {
        let launch = Launch {
            seed: launch.seed.map(|seed| seed.wrapping_add(1)),
            ..launch
        };
        // The briefing was heard before the first try: straight onto the wire.
        ctx.replace(RunScreen::new(launch, None))
    })
}

mod visuals_runscreen;
