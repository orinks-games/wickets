//! The lake at night, and what comes before and after it: the pages that
//! open a night, the canoe itself, and the words at the end.

use std::cell::Cell;
use std::rc::Rc;

use hearken::screens::ConfirmScreen;
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self as shared, labels, Back, Leave, Results};
use parlor_core::{Challenge, Challenges, Pages, Parlor, ParlorAction, Period, Rng, ScoreResult};

use crate::cues;
use crate::journal::{self, Journal};
use crate::rules::{heading_vec, Held, Keys, Lake, Mode, Note, Plan, Side, P2, SONG_LAYERS};
use crate::HELP;

/// The title's screen name.
pub const TITLE_NAME: &str = "lantern_lake";
/// The lake's screen name.
pub const RUN_NAME: &str = "lantern_lake_run";
/// The challenges menu's screen name. A challenge opened from the launcher
/// sits on a host of the same name, so leaving it leads back to wherever
/// the outing began.
pub const CHALLENGES_NAME: &str = "lantern_lake_challenges";

/// The player's controls by name, as the lake says them.
pub fn keys(ctx: &Ctx<Parlor>) -> Keys {
    Keys {
        left: ctx.control_name(ParlorAction::Left),
        right: ctx.control_name(ParlorAction::Right),
        paddle: ctx.control_name(ParlorAction::Up),
        back: ctx.control_name(ParlorAction::Down),
        look: ctx.control_name(ParlorAction::Primary),
        home: ctx.control_name(ParlorAction::Secondary),
        status: ctx.control_name(ParlorAction::Status),
        hint: ctx.control_name(ParlorAction::Hint),
    }
}

/// The paddling controls in one paragraph, for F1 on the lake.
pub fn controls_text(ctx: &Ctx<Parlor>) -> String {
    let k = keys(ctx);
    format!(
        "On the lake. {} and {} turn the canoe: press briefly for a little, hold to keep turning. {} paddles a stroke; hold it to keep paddling. {} back-paddles to slow down. {} looks around and rings every lantern once. {} heads home and ends the night. {} says your lanterns and where the nearest one is. {} repeats the last thing said. {} says which way to turn. Escape pauses.",
        sentence_start(&k.left),
        k.right,
        sentence_start(&k.paddle),
        sentence_start(&k.back),
        sentence_start(&k.look),
        sentence_start(&k.home),
        sentence_start(&k.status),
        sentence_start(&ctx.control_name(ParlorAction::Repeat)),
        sentence_start(&k.hint),
    )
}

/// What starting an outing needs: the mode, and a seed for tests and
/// challenges (`None` seeds from the clock).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub mode: Mode,
    pub seed: Option<u64>,
}

impl Launch {
    /// The lake this launch makes.
    pub fn build(&self) -> Lake {
        let mut rng = self.seed.map_or_else(Rng::from_time, Rng::seeded);
        let plan = match self.mode {
            Mode::Pond => Plan::pond(),
            Mode::Night(n) => Plan::night(n, &mut rng),
            Mode::Free => Plan::free(&mut rng),
            Mode::Challenge(p) => Plan::challenge(p, &mut rng),
            Mode::Festival => Plan::festival(&mut rng),
        };
        Lake::new(plan, rng)
    }
}

/// The page before an outing.
fn intro_page(mode: Mode) -> String {
    match mode {
        Mode::Pond => "Practice pond: 2 lanterns—one ahead, one to your right. Title Sound check has every sound.".to_string(),
        Mode::Night(n) => format!("Night {n}. The canoe slides out from the south shore."),
        Mode::Free => "Free paddle: a wide lake where a new lantern appears whenever you gather one. Stay as long as you like, and head home when you are ready.".to_string(),
        Mode::Challenge(p) => format!("{} challenge.", p.label()),
        Mode::Festival => format!("Lantern Festival night. {} lanterns float on a wide, clear lake, and every creature is out. A voice joins the song with every second lantern, and temple bells ring in the last.", crate::events::FESTIVAL_LANTERNS),
    }
}

/// The page before an outing, then the lake. Escape on the page backs
/// out. `replace` swaps out the screen that asked (the results' rows).
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
        Period::Daily => "Today's lake: 5 lanterns and a twist.",
        Period::Weekly => "This week's lake: a constellation of 8 lanterns.",
        Period::Monthly => "This month's event: the lantern festival.",
    }
}

/// The challenge of `period` today, and its lake plan.
pub fn challenge_plan(ctx: &Ctx<Parlor>, period: Period) -> (Challenge, Plan) {
    let challenge = Challenge::current(ctx, crate::ID, period);
    let plan = Plan::challenge(period, &mut challenge.rng());
    (challenge, plan)
}

/// The lake of the current `period` challenge, ready to paddle.
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
    let (challenge, plan) = challenge_plan(ctx, period);
    let twist = plan.twist.expect("a challenge has a twist");
    let pages = vec![format!(
        "{}: {}. {}: {} Score: 1 a lantern, 2 a place. {}",
        challenge.title(),
        plan.summary(),
        twist.name(),
        twist.describe(),
        challenge.resets()
    )];
    let mut run = Some(challenge_run(ctx, period));
    Pages::briefing(pages).then(move |ctx| {
        if let Some(run) = run.take() {
            ctx.replace(run);
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the outing is over (or left from the
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
    /// Named as the challenges menu is, so leaving an outing, by its
    /// results or the pause menu, lands here and steps aside.
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
        "Opening the Lantern Lake challenge.".to_string()
    }
}

/// The results menu after an outing: the next night when one opened, the
/// same lake again, the results again, and back to where it began.
pub fn after_outing(
    launch: Launch,
    challenge: Option<Challenge>,
    complete: bool,
    summary: String,
    pages: Vec<String>,
) -> Results {
    if let (Some(challenge), Mode::Challenge(period)) = (challenge, launch.mode) {
        return Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
            labels::challenge_again(period),
            "The same lake, straight away. Only your best score is kept.",
            move |ctx| {
                ctx.replace(RunScreen::new(launch, Some(challenge.clone())));
            },
        );
    }
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if let (true, Mode::Night(n)) = (complete, launch.mode) {
        let next = n + 1;
        results = results.next(
            format!("Next night: night {next}"),
            "A new lake, a little wider, with one more lantern.",
            move |ctx| {
                let launch = Launch {
                    mode: Mode::Night(next),
                    ..launch
                };
                start(ctx, launch, true)
            },
        );
    }
    let help = match launch.mode {
        Mode::Pond => "The practice pond again.",
        Mode::Free => "Free paddle again, on a new lake.",
        Mode::Festival => "Lantern Festival night again, on a new lake.",
        _ => "The same night again, on a new lake.",
    };
    results.again(labels::PLAY_AGAIN, help, move |ctx| {
        // The festival closed since the night began: no festival night,
        // and no festival board, off its window.
        if launch.mode == Mode::Festival
            && !crate::events::festival_open(parlor_core::challenges::today(ctx))
        {
            ctx.pop_to_with(TITLE_NAME, crate::events::FESTIVAL_ENDED);
            return;
        }
        let launch = Launch {
            seed: launch.seed.map(|seed| seed.wrapping_add(1)),
            ..launch
        };
        start(ctx, launch, true)
    })
}

/// The loop for song voice `n`, 1 to [`SONG_LAYERS`]: on Lantern Festival
/// night the last is temple bells, in place of the choir.
pub fn song_key(mode: Mode, n: usize) -> &'static str {
    if mode == Mode::Festival && n == SONG_LAYERS {
        cues::TEMPLE_BELLS
    } else {
        cues::SONG_LAYERS[n.clamp(1, SONG_LAYERS) - 1]
    }
}

/// How loud the water rushes past at `speed`.
pub fn glide_volume(speed: f32) -> f32 {
    (speed.abs() / crate::rules::MAX_SPEED).clamp(0.0, 1.0) * 0.5
}

fn v3(p: P2) -> Vec3 {
    Vec3::new(p.x, p.y, 0.0)
}

/// The lake.
pub struct RunScreen {
    lake: Lake,
    launch: Launch,
    challenge: Option<Challenge>,
    loops: Vec<Sound>,
    glide: Option<Sound>,
    layers: Vec<Sound>,
    pending: Vec<usize>,
    song_clock: f64,
    /// The song loop each voice joined on, kept with the night.
    joins: Vec<u32>,
    home: Rc<Cell<bool>>,
    /// Seconds left to let the whole song ring before the night ends.
    ending: Option<f64>,
    done: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

/// Each song voice's volume: ten together stay under speech.
pub const LAYER_VOLUME: f32 = 0.55;

/// How long the whole song rings after the last lantern, at the least.
/// A voice still waiting for its loop gets to start and play through once
/// first (see [`RunScreen::ending_time`]).
pub const ENDING: f64 = 5.0;

impl RunScreen {
    pub fn new(launch: Launch, challenge: Option<Challenge>) -> RunScreen {
        RunScreen {
            status_key: Default::default(),
            lake: launch.build(),
            launch,
            challenge,
            loops: Vec::new(),
            glide: None,
            layers: Vec::new(),
            pending: Vec::new(),
            song_clock: 0.0,
            joins: Vec::new(),
            home: Rc::new(Cell::new(false)),
            ending: None,
            done: false,
        }
    }

    /// The rules, for tests.
    pub fn lake(&self) -> &Lake {
        &self.lake
    }

    /// The rules, for tests that steer by hand.
    pub fn lake_mut(&mut self) -> &mut Lake {
        &mut self.lake
    }

    /// How many song voices are playing, the base not counted.
    pub fn voices(&self) -> usize {
        self.layers.len()
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        let all = self
            .loops
            .drain(..)
            .chain(self.layers.drain(..))
            .chain(self.glide.take());
        for s in all.collect::<Vec<_>>() {
            ctx.audio.stop(s, 400);
        }
        self.pending.clear();
    }

    fn start_ambience(&mut self, ctx: &mut Ctx<Parlor>) {
        let carry = self.lake.plan.weather.carry();
        let lap = ctx.audio.start_loop(cues::LAP, None, Bus::AMBIENCE);
        ctx.audio.set_volume(lap, 0.35);
        self.loops.push(lap);
        let glide = ctx.audio.start_loop(cues::GLIDE, None, Bus::AMBIENCE);
        ctx.audio.set_volume(glide, 0.0);
        self.glide = Some(glide);
        if let Some(bed) = self.lake.plan.weather.bed() {
            let s = ctx.audio.start_loop(bed, None, Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.4);
            self.loops.push(s);
        }
        let reeds =
            ctx.audio
                .start_loop(cues::REEDS_WIND, Some(v3(self.lake.reeds)), Bus::AMBIENCE);
        ctx.audio.set_volume(reeds, 0.6 * carry);
        self.loops.push(reeds);
        let critters: Vec<_> = self
            .lake
            .critters
            .iter()
            .filter(|c| c.creature.loops())
            .map(|c| (c.creature.key(), c.pos))
            .collect();
        for (key, pos) in critters {
            let s = ctx.audio.start_loop(key, Some(v3(pos)), Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.7 * carry);
            self.loops.push(s);
        }
        let spots: Vec<_> = self
            .lake
            .spots
            .iter()
            .map(|s| (s.feature.key(), s.pos))
            .collect();
        for (key, pos) in spots {
            let s = ctx.audio.start_loop(key, Some(v3(pos)), Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.8 * carry);
            self.loops.push(s);
        }
        let base = ctx.audio.start_loop(cues::SONG_BASE, None, Bus::MUSIC);
        ctx.audio.set_volume(base, 0.7);
        self.loops.push(base);
        self.song_clock = 0.0;
    }

    fn play_notes(&mut self, ctx: &mut Ctx<Parlor>) {
        for note in self.lake.drain_notes() {
            match note {
                Note::Say(text) => ctx.say(text),
                Note::Queue(text) => ctx.say_queued(text),
                Note::Cue(key) => {
                    ctx.audio.play(key, Bus::EFFECTS);
                }
                Note::At(key, pos, vol) => {
                    let s = ctx.audio.play_at(key, v3(pos), Bus::EFFECTS);
                    if vol < 1.0 {
                        ctx.audio.set_volume(s, vol);
                    }
                }
                Note::Stroke { side, back } => {
                    // Just off the canoe's side, a little ahead.
                    let right = heading_vec(self.lake.heading + 90.0);
                    let ahead = heading_vec(self.lake.heading);
                    let p = self.lake.pos + right.scale(side.sign() * 0.8) + ahead.scale(0.4);
                    let key = if back { cues::BACKSTROKE } else { cues::STROKE };
                    let s = ctx.audio.play_at(key, v3(p), Bus::EFFECTS);
                    ctx.audio.set_volume(s, 0.7);
                }
                Note::Layer(n) => {
                    if !self.pending.contains(&n) {
                        self.pending.push(n);
                    }
                }
                Note::Complete => {
                    ctx.audio.play(cues::MUSIC_COMPLETE, Bus::MUSIC);
                    self.ending = Some(ENDING);
                }
            }
        }
        // The last lantern's voice waits for the next loop: let it start
        // and play through once before the night ends.
        if !self.pending.is_empty() {
            let needed = self.ending_time_from(self.song_clock);
            if let Some(left) = self.ending.as_mut() {
                *left = left.max(needed);
            }
        }
        // Discoveries go into the journal at once.
        for f in std::mem::take(&mut self.lake.new_finds) {
            if journal::record_find(ctx, f) {
                ctx.say_queued(format!("New in your journal: {}.", f.name()));
            } else {
                ctx.say_queued(format!(
                    "{} is already in your journal.",
                    sentence_start(f.name())
                ));
            }
        }
    }

    /// The listener rides in the canoe; the glide follows the rules.
    fn sync_audio(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(v3(self.lake.pos), self.lake.heading);
        if let Some(g) = self.glide {
            ctx.audio.set_volume(g, glide_volume(self.lake.speed));
        }
    }

    /// Seconds the whole song should ring from song time `clock`: at least
    /// [`ENDING`], and long enough for a waiting voice to start at the next
    /// loop and be heard through one full loop.
    fn ending_time_from(&self, clock: f64) -> f64 {
        if self.pending.is_empty() {
            return ENDING;
        }
        let into = clock.rem_euclid(cues::SONG_LOOP);
        let to_next = cues::SONG_LOOP - into;
        ENDING.max(to_next + cues::SONG_LOOP)
    }

    /// Keeps tonight's song in the player's nights, for the soundtrack.
    fn keep_night(&self, ctx: &mut Ctx<Parlor>) {
        let today = parlor_core::challenges::today(ctx);
        let mode = self.lake.plan.mode;
        crate::soundtrack::keep_night(
            ctx,
            crate::soundtrack::Night {
                n: 0,
                year: today.year,
                month: today.month,
                day: today.day,
                mode: mode.name(),
                festival: mode == Mode::Festival,
                weather: self.lake.plan.weather.bed().map(str::to_string),
                joins: self.joins.clone(),
                complete: self.lake.complete,
            },
        );
    }

    /// Seconds left before the night ends while the whole song rings.
    pub fn ending_left(&self) -> Option<f64> {
        self.ending
    }

    /// New song voices wait for the start of the next loop, so every voice
    /// stays on the beat.
    fn tick_song(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let before = (self.song_clock / cues::SONG_LOOP).floor();
        self.song_clock += dt;
        let after = (self.song_clock / cues::SONG_LOOP).floor();
        if after > before && !self.pending.is_empty() {
            for n in std::mem::take(&mut self.pending) {
                let key = song_key(self.lake.plan.mode, n);
                let s = ctx.audio.start_loop(key, None, Bus::MUSIC);
                ctx.audio.set_volume(s, LAYER_VOLUME);
                self.layers.push(s);
                self.joins.push(after as u32);
            }
        }
    }

    fn held(ctx: &Ctx<Parlor>) -> Held {
        Held {
            left: ctx.action_held(ParlorAction::Left),
            right: ctx.action_held(ParlorAction::Right),
            up: ctx.action_held(ParlorAction::Up),
            down: ctx.action_held(ParlorAction::Down),
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.done = true;
        let complete = self.lake.complete;
        self.keep_night(ctx);
        self.stop_sounds(ctx);
        if !complete {
            ctx.audio.play(cues::MUSIC_GOODNIGHT, Bus::MUSIC);
        }
        let lake = &self.lake;
        let mut pages = Vec::new();
        let opening = if complete {
            "The song is whole, and you paddle home under the stars.".to_string()
        } else {
            "You turn the canoe toward home, and the song fades softly behind you.".to_string()
        };

        let gathered = match lake.gathered {
            1 => "1 lantern".to_string(),
            n => format!("{n} lanterns"),
        };
        let mut summary = if complete {
            format!("Song complete. {gathered}")
        } else {
            format!("Home with {gathered}")
        };
        if self.challenge.is_some() {
            summary.push_str(&format!(", score {}", lake.score()));
        }
        let mut tally = format!(
            "{} gathered {} lantern{}",
            lake.plan.mode.name(),
            lake.gathered,
            if lake.gathered == 1 { "" } else { "s" }
        );
        let found = lake.found();
        if found > 0 {
            tally.push_str(&format!(
                " and found {found} {}",
                if found == 1 { "place" } else { "places" }
            ));
        }
        tally.push('.');
        pages.push(format!("{opening} {tally}"));
        if let Some(c) = self.challenge.clone() {
            let score = self.lake.score();
            let mut line = format!("{} score: {score}.", c.title());
            match Challenges::submit(ctx, &c, score) {
                ScoreResult::First => line.push_str(" Your first score on this one."),
                ScoreResult::NewBest { previous } => {
                    line.push_str(&format!(" A new best, beating {previous}."))
                }
                ScoreResult::Short { best } => line.push_str(&format!(" Your best stays {best}.")),
            }
            let streak = Challenges::streak(ctx, crate::ID, c.period);
            if streak > 0 {
                line.push_str(&format!(" A {streak} {} streak.", c.period.unit()));
            }
            line.push(' ');
            line.push_str(&c.resets());
            pages.push(line);
        }
        pages.push(journal::save_outing(ctx, &self.lake).join(" "));
        let results = after_outing(
            self.launch,
            self.challenge.clone(),
            complete,
            summary,
            pages.clone(),
        );
        let back_to = if self.challenge.is_some() {
            CHALLENGES_NAME
        } else {
            TITLE_NAME
        };
        let mut results = Some(results);
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(back_to);
                ctx.push(results);
            }
        }));
    }

    fn ask_home(&mut self, ctx: &mut Ctx<Parlor>) {
        let flag = self.home.clone();
        let question = if self.lake.plan.respawn || self.lake.complete {
            "Head home and end the outing?".to_string()
        } else {
            format!(
                "Head home now, with {} of {} lanterns? What you gathered and found is kept.",
                self.lake.gathered, self.lake.plan.lanterns
            )
        };
        // The lake holds still while you decide, so the song's clock and
        // its loops stay together; `resume` brings the sound back.
        ctx.audio.pause_all();
        ctx.push(ConfirmScreen::<Parlor>::new(question, move |_ctx| {
            flag.set(true)
        }));
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
        ctx.audio.play_music(None, false);
        self.lake.keys = keys(ctx);
        // A full moon is heard on ordinary nights and free paddles only.
        if matches!(self.lake.plan.mode, Mode::Night(_) | Mode::Free) {
            let today = parlor_core::challenges::today(ctx);
            self.lake.set_moon(crate::events::full_moon(today));
        }
        self.lake.drain_notes();
        self.start_ambience(ctx);
        self.sync_audio(ctx);
        let intro = self.lake.intro();
        self.lake.last = intro.clone();
        ctx.say(intro);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.lake.keys = keys(ctx);
        ctx.audio.resume_all();
        if self.home.get() {
            self.finish(ctx);
            return;
        }
        let status = self.lake.status_layers().remove(0);
        ctx.say(format!("Back on the lake. {status}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        // A pad's X, Y and bumpers stand for no key, only for actions.
        let action = ctx.action_for(event);
        if event.is_repeat() || (event.pressed().is_none() && action.is_none()) {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            // The song is whole: Escape goes straight home with the night.
            if self.ending.is_some() && !self.done {
                self.finish(ctx);
                return;
            }
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            shared::pause(ctx, HELP, leave);
            return;
        }
        if self.done {
            return;
        }
        // The canoe rests while the whole song rings.
        let ending = self.ending.is_some();
        let lake = &mut self.lake;
        match action {
            Some(
                ParlorAction::Left | ParlorAction::Right | ParlorAction::Up | ParlorAction::Down,
            ) if ending => return,
            Some(ParlorAction::Left) => lake.turn(Side::Left),
            Some(ParlorAction::Right) => lake.turn(Side::Right),
            Some(ParlorAction::Up) => lake.paddle(),
            Some(ParlorAction::Down) => lake.back_paddle(),
            Some(ParlorAction::Primary) => lake.look(),
            Some(ParlorAction::Secondary) => {
                self.ask_home(ctx);
                return;
            }
            Some(ParlorAction::Status) => {
                let layers = lake.status_layers();
                lake.last = self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                if lake.last.is_empty() {
                    ctx.say("Nothing to repeat yet.");
                } else {
                    let s = lake.last.clone();
                    ctx.say(s);
                }
            }
            Some(ParlorAction::Hint) => {
                let s = lake.hint();
                ctx.say(s);
            }
            None => ctx.say("Not a paddling control. F1 lists them."),
        }
        self.play_notes(ctx);
        self.sync_audio(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.done {
            return;
        }
        let held = if self.ending.is_some() {
            Held::default()
        } else {
            RunScreen::held(ctx)
        };
        self.lake.update(dt as f32, held);
        self.play_notes(ctx);
        self.sync_audio(ctx);
        self.tick_song(ctx, dt);
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.finish(ctx);
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![self.lake.status()]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        controls_text(ctx)
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// The journal's summary line for the title.
pub fn journal_line(j: &Journal) -> String {
    format!(
        "Journal, {} of {} places found",
        j.places(),
        crate::rules::Feature::ALL.len()
    )
}

mod visuals_runscreen;
