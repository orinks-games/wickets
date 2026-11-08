//! The run and what surrounds it: launch pages, the rail itself, the
//! intermission between sectors and the debrief.

use std::collections::HashMap;

use hearken::menu::{menu_input, Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self as shared, labels, Back, Leave};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::{
    Challenge, Challenges, Date, Mood, Pages, Parlor, ParlorAction, Records, Rng, Score,
    ScoreResult,
};

use crate::cues;
use crate::profile::{debrief, Debrief, Profile};
use crate::rules::{
    Keys, Kind, Lane, Loadout, Mode, Note, Phase, Pressure, Route, Run, Upgrade, MAX_STRESS,
};
use crate::story::{launch_text, LOGS};
use crate::{HELP, ID};

/// How far apart the lanes sound, in metres. Wide and close enough that a
/// lane one over is heard clearly off to the side from the first moment
/// of its approach, not only in its last half second.
pub const LANE_SPACING: f32 = 4.0;
/// How far ahead an arrival is first heard.
pub const APPROACH_DISTANCE: f32 = 6.0;

pub fn lane_x(lane: usize) -> f32 {
    (lane as f32 - 1.0) * LANE_SPACING
}

/// Where a sound in `lane` is, `y` metres ahead.
fn lane_pos(lane: Lane, y: f32) -> Option<Vec3> {
    lane.index().map(|l| Vec3::new(lane_x(l), y, 0.0))
}

/// The title's `State::name`, which runs return to.
pub const TITLE_NAME: &str = "neon_rail";

/// Opens the pause screen over a run, or between its sectors: its leave
/// row goes to the challenges for a challenge, otherwise to the title.
fn pause(ctx: &mut Ctx<Parlor>, launch: &Launch) {
    let leave = if launch.mode.is_challenge() {
        Leave::Challenge(crate::challenge::CHALLENGES_NAME)
    } else {
        Leave::Title(TITLE_NAME)
    };
    shared::pause(ctx, HELP, leave);
}

/// The player's controls by name.
pub fn keys(ctx: &Ctx<Parlor>) -> Keys {
    Keys {
        left: ctx.control_name(ParlorAction::Left),
        right: ctx.control_name(ParlorAction::Right),
        boost: ctx.control_name(ParlorAction::Primary),
        fire: ctx.control_name(ParlorAction::Secondary),
        slowdown: ctx.control_name(ParlorAction::Down),
        priority: ctx.control_name(ParlorAction::Up),
    }
}

/// The run controls, as F1 and the controls pages give them.
pub fn controls_pages(ctx: &Ctx<Parlor>) -> Vec<String> {
    let k = keys(ctx);
    let status = ctx.control_name(ParlorAction::Status);
    let repeat = ctx.control_name(ParlorAction::Repeat);
    let hint = ctx.control_name(ParlorAction::Hint);
    vec![
        format!(
            "{} shifts left. {} shifts right. Sounds are placed by lane around you: a sound straight ahead is in your lane.",
            sentence_start(&k.left),
            sentence_start(&k.right)
        ),
        format!(
            "{} activates boost for hard breaches. {} fires the rail gun against drones and unstable targets in the last second of their approach.",
            sentence_start(&k.boost),
            sentence_start(&k.fire)
        ),
        format!(
            "{} triggers the slowdown pulse, with limited charges and a cooldown. {} activates priority override, with limited charges.",
            sentence_start(&k.slowdown),
            sentence_start(&k.priority)
        ),
        format!(
            "{} speaks the corridor summary. {} repeats the last call-out. {} says what is coming next and what to do about it. Escape opens the pause menu. F1 gives help for the screen you are on.",
            sentence_start(&status),
            sentence_start(&repeat),
            sentence_start(&hint)
        ),
    ]
}

/// What starting a run needs to know.
#[derive(Clone, Copy, Debug)]
pub struct Launch {
    pub mode: Mode,
    /// A fixed seed for tests; `None` seeds from the clock.
    pub seed: Option<u64>,
    /// For a challenge, the day it was opened: which challenge it is.
    pub day: Option<Date>,
}

impl Launch {
    /// The challenge this launch flies, if it is one.
    pub fn challenge(&self) -> Option<Challenge> {
        let period = self.mode.period()?;
        let day = self.day?;
        Some(Challenge::on(day, ID, period))
    }

    fn rng(&self, runs: u64) -> Rng {
        match self.seed {
            // Doubled: `Rng::seeded` folds the lowest bit away.
            Some(seed) => Rng::seeded(seed.wrapping_add(runs).wrapping_mul(2)),
            None => Rng::from_time(),
        }
    }
}

/// The pages before a run, then the run. `replace` swaps out the screen
/// that asked, as the results menu's "Run again" does.
pub fn launch(ctx: &mut Ctx<Parlor>, launch: Launch, replace: bool) {
    let profile = Profile::load(ctx);
    let k = keys(ctx);
    let mut pages: Vec<String> = Vec::new();
    if let Some(ch) = launch.challenge() {
        pages.extend(crate::challenge::briefing_pages(ctx, &ch));
    } else if let Some(text) = launch_text(profile.runs_started + 1, profile.story_stage) {
        pages.push(text);
    }
    match launch.mode {
        m if m.is_challenge() => {
            pages.push(format!(
                "Center lane. {} and {} shift lanes. {} boosts. {} fires the rail gun. {} triggers a slowdown pulse. {} uses priority override. {} speaks lane, hull and stress; press it again for your tools, and again for the sector and the challenge's rule. Escape pauses.",
                sentence_start(&k.left),
                k.right,
                sentence_start(&k.boost),
                sentence_start(&k.fire),
                sentence_start(&k.slowdown),
                sentence_start(&k.priority),
                sentence_start(&ctx.control_name(ParlorAction::Status))
            ));
        }
        Mode::Tutorial => pages.push(
            "Guided run: each new sound is explained as it comes, and the run pauses while you read."
                .to_string(),
        ),
        _ => {
            match launch.mode {
                Mode::BlackSky => pages.push(
                    "A Black Sky Run is five sectors. After each sector you fit one upgrade and choose the route ahead. The fifth sector is the extraction sprint.".to_string(),
                ),
                Mode::Endless => pages.push(
                    "The endless rail never ends. Every sector is harder than the last. Choose an upgrade and a route after each one, and go as deep as you can.".to_string(),
                ),
                Mode::ColdStart => pages.push(
                    "Cold start: a bare skimmer with no rank bonuses or workshop parts, nine sectors with an upgrade and a route choice between them, and one life. It keeps its own board and pays no credits, parts or intel.".to_string(),
                ),
                Mode::FridayNight => pages.push(
                    "A Friday night run is three sectors made from tonight's date, the same for every pilot, on a standard skimmer. Salvage comes as cargo crates, worth double.".to_string(),
                ),
                _ => pages.push(
                    "The challenge corridor: six fixed sectors in two minutes, the same every time. Learn it, then chase the best score.".to_string(),
                ),
            }
            // The keys are taught by the tutorial and the guided run; a
            // pilot who has finished a run only needs to know where they are.
            if profile.runs_completed > 0 {
                pages.push("Rogue courier route open. F1 lists the controls.".to_string());
            } else {
            pages.push(format!(
                "Rogue courier route open. Center lane. {} and {} shift lanes. {} boosts through blockades.",
                sentence_start(&k.left),
                k.right,
                sentence_start(&k.boost)
            ));
            pages.push(format!(
                "{} fires the rail gun. {} triggers a slowdown pulse. {} uses priority override. {} speaks lane, hull and stress; press it again for your tools. Escape pauses.",
                sentence_start(&k.fire),
                sentence_start(&k.slowdown),
                sentence_start(&k.priority),
                sentence_start(&ctx.control_name(ParlorAction::Status))
            ));
            }
        }
    }
    // Ghost cars are explained here, at a standstill: on the rail the
    // first one arrives too soon for the whole line.
    let today = parlor_core::challenges::today(ctx);
    if crate::events::ghost_cars_for(launch.mode, today) {
        pages.push(crate::rules::GHOST_LINE.to_string());
    }
    let pages = Pages::briefing(pages).then(move |ctx| {
        let screen = RunScreen::start(ctx, launch);
        ctx.replace(screen);
    });
    if replace {
        ctx.replace(pages);
    } else {
        ctx.push(pages);
    }
}

/// The text of each tutorial step, the original's with the player's keys.
fn tutorial_text(step: u8, k: &Keys) -> String {
    let text = match step {
        1 => format!("This low loop is your skimmer in motion. {} or {} shifts lanes, with a magnetic snap. Your lane changes where incoming sounds are placed.", sentence_start(&k.left), k.right),
        2 => "Flow vehicle in the center lane. Move into its lane before it arrives to score and reduce stress.".to_string(),
        3 => "Debris in the center lane, your lane. Shift out of its lane before it arrives.".to_string(),
        4 => "Repair pickup on the left. Be in its lane when it arrives to restore hull.".to_string(),
        5 => format!("Drone in the left lane. {} fires the rail gun at a drone or unstable vehicle as it closes in. It locks only in the last second of the approach. Fire too early and you hear No lock.", sentence_start(&k.fire)),
        6 => format!("Heavy blocker on the left. {} boosts. Boosting into a heavy blocker forces it open, and then boost needs time to recharge.", sentence_start(&k.boost)),
        7 => format!("Full blockade next. It fills all three lanes, so you cannot dodge it. Its alarm grows louder as it comes. {} boosts: press it on the bright chime that sounds just before it hits.", sentence_start(&k.boost)),
        8 => format!("Priority traffic on the right. Priority vehicles raise stress if delayed. {} activates priority override when you need to protect the route.", sentence_start(&k.priority)),
        9 => "Salvage in the center lane. Salvage adds to your score now, and your score becomes credits at the debrief. Every debrief pays, even after a failed run.".to_string(),
        _ => format!("Unstable vehicle on the left. Dodge it or boost through it. If stress spikes, {} triggers a slowdown pulse for a recovery window.", k.slowdown),
    };
    format!("Tutorial step {step}. {text}")
}

/// The rail itself.
pub struct RunScreen {
    run: Option<Run>,
    launch: Launch,
    sounds: HashMap<u32, Sound>,
    /// Whether this screen opens a run, rather than continuing one after
    /// an intermission.
    fresh: bool,
    route: Option<Route>,
    /// Story pages earned in the frame the run left the rail: they follow
    /// the debrief, or wait behind whatever screen replaces this one.
    deferred: Vec<String>,
    /// The rail's music, following the run.
    score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// The guided run's tutorial messages so far, so Up can go back over
    /// them while one holds the run.
    tutorial_log: Vec<String>,
}

impl RunScreen {
    /// A new run of `launch.mode`, counted as started.
    pub fn start(ctx: &mut Ctx<Parlor>, launch: Launch) -> RunScreen {
        let mut profile = Profile::load(ctx);
        let loadout = if launch.mode.bare_skimmer() {
            Loadout::default()
        } else {
            profile.loadout()
        };
        let rng = launch.rng(profile.runs_started);
        let mut run = match (launch.mode, launch.challenge()) {
            (_, Some(ch)) => {
                crate::challenge::Spec::for_challenge(&ch).build(&ch, profile.story_stage)
            }
            (Mode::Challenge, _) => Run::challenge(loadout, profile.story_stage),
            (Mode::Tutorial, _) => Run::tutorial(loadout, profile.story_stage),
            (Mode::FridayNight, _) => {
                let today = parlor_core::challenges::today(ctx);
                crate::events::friday_run(launch.day.unwrap_or(today), profile.story_stage)
            }
            (m, _) => Run::roguelite(m, loadout, profile.story_stage, rng),
        };
        let today = parlor_core::challenges::today(ctx);
        if crate::events::ghost_cars_for(run.mode, today) {
            run.ghost_cars = true;
            // The briefing named them.
            run.ghost_named = true;
            run.event_rule = crate::rules::GHOST_RULE.to_string();
        }
        run.callouts = !profile.quiet_callouts;
        run.earned = profile.earned_upgrades();
        profile.runs_started += 1;
        profile.save(ctx);
        RunScreen {
            run: Some(run),
            launch,
            sounds: HashMap::new(),
            fresh: true,
            route: None,
            deferred: Vec::new(),
            score: Score::new(),
            status_key: Default::default(),
            tutorial_log: Vec::new(),
        }
    }

    /// The run going on after an intermission.
    pub fn continuing(run: Run, launch: Launch, route: Route) -> RunScreen {
        RunScreen {
            run: Some(run),
            launch,
            sounds: HashMap::new(),
            fresh: false,
            route: Some(route),
            deferred: Vec::new(),
            score: Score::new(),
            status_key: Default::default(),
            tutorial_log: Vec::new(),
        }
    }

    /// The rules, for tests.
    pub fn run(&self) -> Option<&Run> {
        self.run.as_ref()
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        for (_, sound) in self.sounds.drain() {
            ctx.audio.stop(sound, 60);
        }
        self.score.stop(ctx);
    }

    /// The score's levels for tests: each stem's key and volume.
    pub fn score_levels(&self) -> Vec<(&'static str, f32)> {
        self.score.levels()
    }

    fn place_listener(&self, ctx: &mut Ctx<Parlor>) {
        if let Some(run) = &self.run {
            ctx.audio
                .set_listener(Vec3::new(lane_x(run.lane), 0.0, 0.0), 0.0);
        }
    }

    fn play_notes(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let notes = run.drain_notes();
        let driving = run.phase == Phase::Driving;
        let over = matches!(run.phase, Phase::Over { .. });
        let k = keys(ctx);
        // One moment's speech is heard whole: the first line may cut in on
        // what came before, the rest wait their turn, so a hull report never
        // silences the "High stress" warning raised by the same hit.
        let mut cut_in = notes.iter().any(|n| matches!(n, Note::Say(_)));
        let mut speak = |ctx: &mut Ctx<Parlor>, text: String| {
            if cut_in {
                cut_in = false;
                ctx.say(text);
            } else {
                ctx.say_queued(text);
            }
        };
        // Pages wait until every sound of this moment has started, so the
        // pause holds all of them.
        let mut steps: Vec<String> = Vec::new();
        for note in notes {
            match note {
                Note::Cue(key) => {
                    ctx.audio.play(key, Bus::EFFECTS);
                }
                Note::LaneCue(key, lane) => match lane_pos(lane, 1.0) {
                    Some(pos) => {
                        ctx.audio.play_at(key, pos, Bus::EFFECTS);
                    }
                    None => {
                        ctx.audio.play(key, Bus::EFFECTS);
                    }
                },
                Note::Say(text) | Note::SayQueued(text) => speak(ctx, text),
                Note::Spawn(id) => {
                    let Some(e) = run.events.iter().find(|e| e.id == id) else {
                        continue;
                    };
                    let pos = lane_pos(e.lane, APPROACH_DISTANCE);
                    let sound = ctx
                        .audio
                        .start_loop(run.loop_key(e.kind), pos, Bus::EFFECTS);
                    if e.kind == Kind::Blockade {
                        ctx.audio.set_volume(sound, 0.35);
                    }
                    self.sounds.insert(id, sound);
                }
                Note::Despawn(id) => {
                    if let Some(sound) = self.sounds.remove(&id) {
                        ctx.audio.stop(sound, 80);
                    }
                }
                Note::Story(stage, reason) => {
                    let mut profile = Profile::load(ctx);
                    if profile.unlock_story(stage) {
                        profile.save(ctx);
                        // On the rail, or as it ends, the log waits for
                        // the debrief's count rather than cut into the run.
                        if !driving && !over {
                            let log = &LOGS[stage - 1];
                            ctx.audio.play(cues::UNLOCK, Bus::EFFECTS);
                            self.deferred.extend([
                                format!("Story archive updated. {reason}"),
                                format!("{}. {}", log.title, log.pages.join(" ")),
                            ]);
                        }
                    }
                }
                Note::Tutorial(_) if !driving => {}
                Note::Tutorial(step) => {
                    // The step first, then any story page of the moment.
                    steps.push(tutorial_text(step, &k));
                }
            }
        }
        if !steps.is_empty() {
            // Earlier tutorial messages come first, so Up goes back over
            // them, but the set opens on this moment's step.
            let mut pages = Vec::new();
            let mut at = 0;
            if !steps.is_empty() {
                at = self.tutorial_log.len();
                self.tutorial_log.extend(steps);
                pages = self.tutorial_log.clone();
            }
            ctx.audio.pause_all();
            ctx.push(Pages::new(pages).open_at(at));
        }
    }

    fn move_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(run) = &self.run else {
            return;
        };
        for e in &run.events {
            let Some(&sound) = self.sounds.get(&e.id) else {
                continue;
            };
            let left = ((e.resolve - run.t) / e.approach()).clamp(0.0, 1.0) as f32;
            match lane_pos(e.lane, APPROACH_DISTANCE * left) {
                Some(pos) => ctx.audio.set_position(sound, Some(pos)),
                None => ctx.audio.set_volume(sound, 0.35 + 0.65 * (1.0 - left)),
            }
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
        ctx.audio.set_bed(None);
        let Some(run) = self.run.take() else {
            return;
        };
        let mut profile = Profile::load(ctx);
        // Logs found on the rail are already saved: count them too.
        let before = run.story_start.min(profile.story_stage);
        let report = debrief(&mut profile, &run, before);
        profile.save(ctx);
        let challenge = self.launch.challenge();
        let board = if run.mode == Mode::Tutorial || challenge.is_some() {
            None
        } else {
            Some(Records::record(
                ctx,
                ID,
                run.mode.board(),
                run.score.max(0) as u64,
            ))
        };
        let won = report.won;
        ctx.audio.play_music(
            Some(if won {
                cues::MUSIC_VICTORY
            } else {
                cues::MUSIC_FAILURE
            }),
            false,
        );
        if !report.unlocks.is_empty() {
            ctx.audio.play(cues::UNLOCK, Bus::UI);
        }
        let launch = self.launch;
        let mut pages = report.pages();
        let headline = if won { "Run complete." } else { "Run failed." };
        pages[0] = format!("{headline} {}", pages[0]);
        if let Some(result) = board {
            pages.push(board_text(run.mode, result, run.score.max(0) as u64));
        }
        if let Some(ch) = challenge {
            let score = run.score.max(0) as u64;
            let result = Challenges::submit(ctx, &ch, score);
            let streak = Challenges::streak(ctx, ID, ch.period).max(1);
            pages.push(format!(
                "{} result: {score}. {} A {streak} {} streak. {}",
                ch.title(),
                challenge_result(result),
                ch.period.unit(),
                ch.resets()
            ));
            if run.mode == Mode::Monthly {
                let mut profile = Profile::load(ctx);
                profile.black_sky_events += 1;
                profile.save(ctx);
                if profile.black_sky_events == 1 {
                    pages.push(format!(
                        "Event log {} saved to the story archive.",
                        crate::challenge::EVENT_LOG_TITLE
                    ));
                }
            }
        }
        let mut results = Some(results(launch, &report, pages.clone()));
        // A story page earned at the end is read after the debrief.
        pages.append(&mut self.deferred);
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }
}

/// What a challenge's record said about this run.
pub fn challenge_result(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "Your first recorded result.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best stays {best}."),
    }
}

/// What the best-score board said about this run.
pub fn board_text(mode: Mode, result: ScoreResult, score: u64) -> String {
    let board = mode.name();
    match result {
        ScoreResult::First => format!("First score on the {board} board: {score}."),
        ScoreResult::NewBest { previous } => {
            format!("New best on the {board} board: {score}, beating {previous}.")
        }
        ScoreResult::Short { best } => format!("{board} best stays {best}."),
    }
}

impl State<Parlor> for RunScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "neon_rail_run"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(Some(cues::SKIMMER));
        self.place_listener(ctx);
        let Some(run) = &self.run else {
            return;
        };
        let stems = if run.mode == Mode::Monthly {
            &cues::SCORE_EVENT
        } else {
            &cues::SCORE
        };
        self.score.start(ctx, stems, mood(run));
        let lane = crate::rules::sentence(Lane::from_index(run.lane).name());
        if self.fresh {
            if run.mode == Mode::Tutorial {
                ctx.say("Tutorial route live. Center lane.");
            } else if run.mode.is_event() {
                ctx.audio.play(cues::CHALLENGE, Bus::UI);
                ctx.say(format!(
                    "{} live. {} Center lane.",
                    run.mode.name(),
                    run.event_rule
                ));
            } else if run.mode.is_challenge() {
                ctx.audio.play(cues::CHALLENGE, Bus::UI);
                let rule = if run.event_rule.is_empty() {
                    String::new()
                } else {
                    format!(" {}", run.event_rule)
                };
                ctx.say(format!("{} live.{rule} Center lane.", run.mode.name()));
            } else {
                ctx.say("Run live. Center lane.");
            }
        } else {
            let route = self.route.map_or("Route", |r| r.name());
            ctx.say(format!("{route} locked in. {lane} lane."));
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.resume_all();
        // Settings, from the pause menu, may have changed the call-outs.
        let quiet = Profile::load(ctx).quiet_callouts;
        if let Some(run) = &mut self.run {
            run.callouts = !quiet;
        }
        if let Some(run) = &self.run {
            let lane = crate::rules::sentence(Lane::from_index(run.lane).name());
            ctx.say(format!("{lane} lane."));
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        // A pad's X, Y and bumpers stand for no key, only for actions.
        let Some(key) = event.pressed().or(action.map(|_| Key::Other(0))) else {
            return;
        };
        if key == Key::Escape {
            pause(ctx, &self.launch);
            return;
        }
        let Some(run) = self.run.as_mut() else {
            return;
        };
        // Modifiers and the screen reader's own keys (Control stops speech)
        // are not the player asking the rail for anything.
        if matches!(
            key,
            Key::LCtrl
                | Key::RCtrl
                | Key::LShift
                | Key::RShift
                | Key::LAlt
                | Key::RAlt
                | Key::Insert
                | Key::Other(_)
        ) && ctx.action_for(event).is_none()
        {
            return;
        }
        match ctx.action_for(event) {
            Some(ParlorAction::Left) => run.shift(-1),
            Some(ParlorAction::Right) => run.shift(1),
            Some(ParlorAction::Primary) => run.boost(),
            Some(ParlorAction::Secondary) => run.fire(),
            Some(ParlorAction::Down) => run.slowdown(),
            Some(ParlorAction::Up) => run.priority_override(),
            Some(ParlorAction::Status) => {
                let layers = run.status_layers();
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                if run.last_prompt.is_empty() {
                    ctx.say("Nothing to repeat yet.");
                } else {
                    let text = run.last_prompt.clone();
                    ctx.say(text);
                }
            }
            Some(ParlorAction::Hint) => {
                let hint = run.hint(&keys(ctx));
                ctx.say(hint);
            }
            None => match key {
                // The original's keys still work.
                Key::A => run.shift(-1),
                Key::D => run.shift(1),
                Key::F => run.fire(),
                Key::X => run.slowdown(),
                Key::P => run.priority_override(),
                _ => ctx.say("Not a run control. F1 lists them."),
            },
        }
        self.place_listener(ctx);
        self.play_notes(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        run.update(dt);
        let phase = run.phase;
        if phase == Phase::Driving {
            let mood = mood(run);
            self.score.follow(ctx, mood, dt);
        }
        self.play_notes(ctx);
        self.move_sounds(ctx);
        match phase {
            Phase::Driving => {}
            Phase::Intermission => {
                self.stop_sounds(ctx);
                if let Some(run) = self.run.take() {
                    let launch = self.launch;
                    ctx.replace(UpgradeMenu::new(run, launch));
                }
            }
            Phase::Over { .. } => self.finish(ctx),
        }
        if !self.deferred.is_empty() {
            // Behind the next screen's own announcement, not over it.
            let pages = std::mem::take(&mut self.deferred);
            ctx.push(Pages::new(pages).after_speech());
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        self.run.iter().map(|r| r.status()).collect()
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        controls_pages(ctx).join(" ")
    }

    /// As in the original, F1 on the rail pauses it and reads the controls
    /// a page at a time, so nothing arrives while the player listens.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.run.is_none() {
            return;
        }
        let mut pages = controls_pages(ctx);
        pages[0] = format!("The rail is paused while you read. {}", pages[0]);
        if let Some(ch) = self.launch.challenge() {
            pages.push(format!(
                "{}: {}. {}",
                ch.title(),
                crate::challenge::name(ch.period),
                crate::challenge::rule(&ch)
            ));
        }
        ctx.audio.pause_all();
        ctx.push(Pages::new(pages));
    }
}

/// The music a run plays for its mood.
/// What the score hears: stress and how far along the route you are build
/// it; high pressure drives it, and a hull under a third darkens it.
pub fn mood(run: &Run) -> Mood {
    let stress = (run.stress as f32 / MAX_STRESS as f32).clamp(0.0, 1.0);
    let sectors = run.sectors.len().max(1) as f32;
    let along = (run.sectors_cleared as f32 / sectors).min(1.0);
    Mood::new(0.2 + 0.45 * stress + 0.35 * along)
        .with_flag(HURRY, run.pressure == Pressure::High)
        .with_flag(DANGER, run.hull * 3 < run.loadout.max_hull)
}

/// Speaks the run's status from a menu between sectors.
fn intermission_keys(
    run: Option<&Run>,
    launch: &Launch,
    ctx: &mut Ctx<Parlor>,
    event: &InputEvent,
) -> bool {
    if event.is_repeat() {
        return false;
    }
    match ctx.action_for(event) {
        Some(ParlorAction::Status) => {
            if let Some(run) = run {
                let s = format!(
                    "Hull {} of {}, stress {}. Score {}, multiplier {}. {} cleared.",
                    run.hull,
                    run.loadout.max_hull,
                    run.stress,
                    run.score,
                    run.multiplier,
                    match run.sectors_cleared {
                        1 => "1 sector".to_string(),
                        n => format!("{n} sectors"),
                    },
                );
                ctx.say(s);
            }
            true
        }
        Some(ParlorAction::Hint) => {
            ctx.say("Upgrades last the whole run. Hull patch helps most when damaged. Riskier routes pay more.");
            true
        }
        _ => {
            if event.pressed() == Some(Key::Escape) {
                pause(ctx, launch);
                return true;
            }
            false
        }
    }
}

/// The intermission menus' keys: Left and Right step through the choices
/// as Up and Down do, since no row there has a value to change.
fn intermission_input<S: Menu<Game = Parlor>>(
    menu: &mut S,
    ctx: &mut Ctx<Parlor>,
    event: &InputEvent,
) {
    match event.pressed() {
        Some(Key::Left) => menu.move_by(ctx, -1),
        Some(Key::Right) => menu.move_by(ctx, 1),
        _ => menu_input(menu, ctx, event),
    }
}

/// Between sectors, first: one upgrade of three.
pub struct UpgradeMenu {
    menu: MenuCore<Self, Parlor>,
    run: Option<Run>,
    launch: Launch,
    options: Vec<Upgrade>,
    /// The relay's radio fragment, which R repeats.
    fragment: Option<String>,
}

impl UpgradeMenu {
    pub fn new(mut run: Run, launch: Launch) -> UpgradeMenu {
        let options = run.upgrade_options();
        let title = format!(
            "Sector {} clear. Hull {}. Stress {}. Score {}. Choose one upgrade",
            run.sectors_cleared, run.hull, run.stress, run.score
        );
        UpgradeMenu {
            menu: MenuCore::new(title),
            run: Some(run),
            launch,
            options,
            fragment: None,
        }
    }
}

/// R between sectors: the menu's title again, then the relay's radio
/// fragment if one played. True when the key was R.
fn repeat_key(
    title: &str,
    fragment: Option<&String>,
    ctx: &mut Ctx<Parlor>,
    event: &InputEvent,
) -> bool {
    if event.is_repeat() || ctx.action_for(event) != Some(ParlorAction::Repeat) {
        return false;
    }
    ctx.say(hearken::text::end_sentence(title));
    if let Some(f) = fragment {
        ctx.say_queued(f.clone());
    }
    true
}

impl Menu for UpgradeMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "neon_rail_upgrade"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_INTERMISSION), true);
        hearken::menu::menu_enter(self, ctx);
        // The relay's radio fragment, behind the menu's own announcement.
        if let Some(run) = &self.run {
            let last = run.sectors.get(run.sectors_cleared.wrapping_sub(1));
            if last.is_some_and(|s| s.route == Route::Relay) {
                let flown = run
                    .sectors
                    .iter()
                    .take(run.sectors_cleared)
                    .filter(|s| s.route == Route::Relay)
                    .count() as u32;
                let n = Profile::load(ctx).route_clears(Route::Relay) + flown;
                let fragment = crate::story::relay_fragment(n).to_string();
                ctx.say_queued(fragment.clone());
                self.fragment = Some(fragment);
            }
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        let title = self.menu.title.clone();
        if repeat_key(&title, self.fragment.as_ref(), ctx, event) {
            return;
        }
        if !intermission_keys(self.run.as_ref(), &self.launch, ctx, event) {
            intermission_input(self, ctx, event);
        }
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        pause(ctx, &self.launch);
    }

    /// Waits for the sector clear bonus, spoken as the rail stopped, to
    /// finish before naming the menu.
    fn announce(&mut self, ctx: &mut Ctx<Parlor>) {
        let title = hearken::text::end_sentence(&self.menu().title);
        ctx.say_queued(title);
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let status = sentence_start(&ctx.control_name(ParlorAction::Status));
        let mut items = self
            .options
            .iter()
            .map(|&u| {
                MenuItem::new(
                    format!("{}: {}", u.name(), u.blurb()),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let Some(mut run) = s.run.take() else {
                            return;
                        };
                        run.apply_upgrade(u);
                        ctx.audio.play(cues::UPGRADE, Bus::UI);
                        let launch = s.launch;
                        ctx.replace(RouteMenu::new(run, launch, u.name()));
                    },
                )
                .select_sound(None)
                .help(format!(
                    "{} Enter fits it. {status} speaks your skimmer's state.",
                    u.blurb()
                ))
            })
            .collect::<Vec<_>>();
        // The endless rail has no extraction: a pilot banks the run here.
        if let Some(run) = self.run.as_ref().filter(|r| r.mode == Mode::Endless) {
            let reached = run.sectors_cleared;
            items.push(
                MenuItem::new(
                    crate::rules::END_HERE,
                    |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let Some(mut run) = s.run.take() else {
                            return;
                        };
                        run.end_here();
                        let mut screen = RunScreen::continuing(run, s.launch, Route::Standard);
                        screen.finish(ctx);
                    },
                )
                .help(format!(
                    "Ends the run as a finished run, paid at the debrief, with sector {reached} reached. Quitting from the pause menu counts nothing."
                )),
            );
        }
        items
    }
}

/// Between sectors, second: the route ahead.
pub struct RouteMenu {
    menu: MenuCore<Self, Parlor>,
    run: Option<Run>,
    launch: Launch,
    options: Vec<Route>,
}

impl RouteMenu {
    pub fn new(run: Run, launch: Launch, fitted: &str) -> RouteMenu {
        let title = if run.mode == Mode::FridayNight {
            format!("{fitted} fitted. Tonight's route is fixed. Enter flies on")
        } else if run.mode.is_challenge() {
            format!("{fitted} fitted. This challenge's route is fixed. Enter flies on")
        } else {
            format!("{fitted} fitted. Choose your route")
        };
        RouteMenu {
            options: Vec::new(),
            menu: MenuCore::new(title),
            run: Some(run),
            launch,
        }
    }
}

impl Menu for RouteMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "neon_rail_route"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let routes = if self.launch.mode.bare_skimmer() {
            Route::CHOOSABLE.to_vec()
        } else {
            Profile::load(ctx).routes()
        };
        if let Some(run) = self.run.as_mut() {
            let nudge = if run.mode.has_fixed_route() {
                None
            } else {
                let profile = Profile::load(ctx);
                run.least_cleared(&routes, |r| profile.route_clears(r))
            };
            self.options = run.route_options_with(&routes, nudge);
        }
        hearken::menu::menu_enter(self, ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        let title = self.menu.title.clone();
        if repeat_key(&title, None, ctx, event) {
            return;
        }
        if !intermission_keys(self.run.as_ref(), &self.launch, ctx, event) {
            intermission_input(self, ctx, event);
        }
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        pause(ctx, &self.launch);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mode = self.launch.mode;
        let blurb = |r: Route| match &self.run {
            Some(run) => r.blurb_for(run),
            None => r.blurb_in(mode),
        };
        self.options
            .iter()
            .map(|&r| {
                MenuItem::new(
                    format!("{}: {}", r.name(), blurb(r)),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let Some(mut run) = s.run.take() else {
                            return;
                        };
                        run.choose_route(r);
                        let launch = s.launch;
                        ctx.replace(RunScreen::continuing(run, launch, r));
                    },
                )
                .help(blurb(r))
            })
            .collect()
    }
}

/// After the debrief: what next, on the shared results menu. After the
/// guided run, Next starts a real Black Sky Run.
pub fn results(launch: Launch, report: &Debrief, pages: Vec<String>) -> shared::Results {
    let title = if report.won {
        "Run complete"
    } else {
        "Run over"
    };
    let summary = format!("{title}. Score {}", report.score);
    let back = if launch.mode.is_challenge() {
        Back::Challenges(crate::challenge::CHALLENGES_NAME)
    } else {
        Back::Title(TITLE_NAME)
    };
    let mut results = shared::Results::new(summary, pages, back);
    if launch.mode == Mode::Tutorial {
        let next = Launch {
            mode: Mode::BlackSky,
            ..launch
        };
        results = results.next(
            "Next: a Black Sky Run",
            "Five sectors with an upgrade and a route choice after each, then extraction.",
            move |ctx| crate::screens::launch(ctx, next, true),
        );
    }
    let (again, help) = match launch.mode.period() {
        Some(period) => (
            labels::challenge_again(period),
            "The same rail again. Only a better score replaces your best.",
        ),
        None if report.won => (
            labels::PLAY_AGAIN.to_string(),
            "Fly the same mode again at once.",
        ),
        None => (
            labels::TRY_AGAIN.to_string(),
            "Fly the same mode again at once.",
        ),
    };
    results = results.again(again, help, move |ctx| {
        // The Friday night run is only open on its Friday.
        if launch.mode == Mode::FridayNight {
            let today = parlor_core::challenges::today(ctx);
            if !crate::events::friday_open(today) {
                ctx.audio.play(cues::DENIED, Bus::UI);
                ctx.say(crate::events::FRIDAY_OVER);
                return;
            }
            let launch = Launch {
                day: Some(today),
                ..launch
            };
            return crate::screens::launch(ctx, launch, true);
        }
        crate::screens::launch(ctx, launch, true)
    });
    results
        .extra(
            "Pilot progress",
            "Your rank, credits, best score and unlocked bonuses.",
            |ctx| {
                let p = Profile::load(ctx);
                ctx.push(Pages::new(p.pages()))
            },
        )
        .extra(
            "Archive",
            "The Black Sky City logs you have recovered.",
            |ctx| ctx.push(crate::menus::Archive::new()),
        )
}

/// The help screen with this game's guide.
pub fn help_screen(ctx: &Ctx<Parlor>) -> parlor_core::Help {
    parlor_core::Help::new(parlor_core::seasonal::help(ctx, HELP))
}

mod visuals_runscreen;
