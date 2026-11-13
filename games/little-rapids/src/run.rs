//! A run down a stream, and the results after it.

use hearken::text::{end_sentence, sentence_start};
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Challenges, Pages, Parlor, ParlorAction, Records, Rng, ScoreResult};

use crate::challenge::{briefing, CHALLENGES_NAME};
use crate::music::Conductor;
use crate::progress::{ribbons_of, Progress, RIBBON_SCRAPS};
use crate::rules::{
    stars_phrase, Boat, Course, Event, Kind, Mode, Phase, Run, Stream, LEAD_BEATS, MILL_LEAD_BEATS,
    SOGGY_SINKS,
};
use crate::sound::{self, thing_pos};
use crate::{events, shelf, HELP, ID};

/// The title's screen name.
pub const TITLE_NAME: &str = "little_rapids";
/// The stream's screen name.
pub const RUN_NAME: &str = "little_rapids_run";
/// The opening line gets at least this long before the boat sets off.
pub const MIN_LEAD_IN: f64 = 2.5;
pub const MAX_LEAD_IN: f64 = 9.0;

/// About how long a line takes to say.
pub fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// What starting a run needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub mode: Mode,
    pub seed: Option<u64>,
}

/// The course for `mode`, with today's calendar and the player's settings.
pub fn course_for(mode: Mode, progress: &Progress, today: parlor_core::Date) -> Course {
    let mut course = match mode {
        Mode::Stream(s) => Course::stream(s),
        Mode::Endless => Course::endless(&progress.open_streams()),
        Mode::Drift => Course::drift(),
        Mode::Practice => Course::practice(),
        m => Course::challenge(m),
    };
    if matches!(mode, Mode::Stream(_) | Mode::Endless) {
        course.parade = events::duck_parade(today);
        course.lanterns = events::lantern_float(today);
        if progress.gentle {
            course.start_level *= 0.5;
            course.end_level *= 0.6;
        }
    }
    course
}

/// The boat a run sails: the week's own in the weekly challenge.
pub fn boat_for(mode: Mode, progress: &Progress, seed: u64) -> Boat {
    match mode {
        Mode::Weekly => Boat::ALL[(seed % Boat::ALL.len() as u64) as usize],
        Mode::Daily | Mode::Monthly => Boat::Paper,
        _ => progress.boat(),
    }
}

/// The page explaining a kind the first time it comes down a stream.
pub fn introduction(kind: Kind) -> &'static str {
    match kind {
        Kind::Petal => {
            "A petal, floating toward you. Steer so it is straight ahead as it reaches you."
        }
        Kind::Leaf => "A leaf. Catch it like a petal.",
        Kind::Scrap => "A paper scrap. Scraps fold into new boats.",
        Kind::Trinket => "A trinket. They are rare. Catch it for your shelf.",
        Kind::Lantern => "A floating lantern. Worth five petals.",
        Kind::Stone => "A stone. Steer clear, or the bump costs your combo.",
        Kind::Gate => {
            "A gate between two stones. Go through the middle for points. Going round is fine."
        }
        Kind::Falls => {
            "A waterfall through a gap in the rocks. Steer into the gap, or you bump the rocks."
        }
        Kind::Mill => "A mill wheel on one side. Be on the other side of the stream as it passes.",
        Kind::Duck => "A duck, paddling across. Steer clear of it.",
    }
}

/// Opens a run, with the briefing first for a challenge.
pub fn start(ctx: &mut Ctx<Parlor>, launch: Launch, replace: bool) {
    if let Some(period) = launch.mode.period() {
        let pages = briefing(ctx, period);
        if replace {
            ctx.replace(pages);
        } else {
            ctx.push(pages);
        }
        return;
    }
    let screen = RunScreen::new(launch, None);
    if replace {
        ctx.replace(screen);
    } else {
        ctx.push(screen);
    }
}

/// The stream during a run.
pub struct RunScreen {
    pub run: Run,
    launch: Launch,
    challenge: Option<Challenge>,
    loops: Vec<(u32, Sound)>,
    banks: Option<(Sound, Sound)>,
    current: Option<Sound>,
    conductor: Conductor,
    last: String,
    status_key: parlor_core::StatusKey,
    intro_pending: Vec<Kind>,
    frozen: bool,
    trinket_names: bool,
    bank_lapping: bool,
    done: bool,
    built: bool,
    /// Seconds to the next paddle stroke while steering, and which take.
    stroke_in: f64,
    stroke: usize,
    /// Left Shift and Right Shift last frame, so a hard stroke fires once a press.
    shifts: (bool, bool),
}

impl RunScreen {
    pub fn new(launch: Launch, challenge: Option<Challenge>) -> RunScreen {
        let seed = launch.seed.unwrap_or_else(|| Rng::from_time().next_u64());
        let course = match launch.mode {
            Mode::Stream(s) => Course::stream(s),
            Mode::Practice => Course::practice(),
            Mode::Drift => Course::drift(),
            Mode::Endless => Course::endless(&[Stream::GardenBrook]),
            m => Course::challenge(m),
        };
        RunScreen {
            run: Run::new(course, Boat::Paper, Rng::seeded(seed)),
            launch: Launch {
                seed: Some(seed),
                ..launch
            },
            challenge,
            loops: Vec::new(),
            banks: None,
            current: None,
            conductor: Conductor::new(seed),
            last: String::new(),
            status_key: Default::default(),
            intro_pending: Vec::new(),
            frozen: false,
            trinket_names: true,
            bank_lapping: true,
            done: false,
            built: false,
            stroke_in: 0.0,
            stroke: 0,
            shifts: (true, true),
        }
    }

    /// Builds the real run from the saves, once the screen can read them.
    fn build(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.built {
            return;
        }
        self.built = true;
        let mut progress = Progress::load(ctx);
        if let Mode::Stream(s) = self.launch.mode {
            progress.last_stream = Some(s.id().to_string());
            progress.save(ctx);
        }
        let today = parlor_core::challenges::today(ctx);
        let seed = self.launch.seed.unwrap_or(1);
        let course = course_for(self.launch.mode, &progress, today);
        let boat = boat_for(self.launch.mode, &progress, seed);
        let mut run = Run::new(course, boat, Rng::seeded(seed));
        run.clean_width = progress.clean_width();
        run.shelf = progress.trinkets.clone();
        run.introduce = self.launch.mode != Mode::Practice;
        for kind in Kind::ALL {
            if progress.has_met(kind) {
                run.met.push(kind);
            }
        }
        if let Some(board) = self.launch.mode.board() {
            run.best = Records::best(ctx, ID, board);
        }
        self.trinket_names = progress.trinket_names;
        self.bank_lapping = progress.bank_lapping;
        self.run = run;
    }

    /// An important line: said now and kept for the repeat key.
    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        ctx.say(text.clone());
        self.last = text;
    }

    fn opening_line(&self, ctx: &mut Ctx<Parlor>) -> String {
        let mut parts = vec![end_sentence(&sentence_start(&self.run.course.mode.label()))];
        if self.run.course.mode.scored() {
            if let Some(best) = self.run.best {
                parts.push(format!("Best {best}."));
            }
        }
        if self.run.boat != Boat::Paper || self.launch.mode == Mode::Weekly {
            parts.push(format!("{}.", self.run.boat.name()));
        }
        if matches!(self.launch.mode, Mode::Stream(_) | Mode::Endless) {
            let today = parlor_core::challenges::today(ctx);
            if let Some(touch) = events::opening(today) {
                parts.push(touch.to_string());
            }
        }
        parts.push(format!(
            "{} starts.",
            sentence_start(&ctx.control_name(ParlorAction::Primary))
        ));
        let mut text = parts.join(" ");
        if !text.ends_with('.') {
            text.push('.');
        }
        text.replace("..", ".")
    }

    fn start_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        let left = ctx.audio.start_loop(
            sound::BANK,
            Some(hearken::Vec3::new(-(crate::rules::HALF as f32), 0.0, 0.0)),
            Bus::EFFECTS,
        );
        let right = ctx.audio.start_loop(
            sound::BANK,
            Some(hearken::Vec3::new(crate::rules::HALF as f32, 0.0, 0.0)),
            Bus::EFFECTS,
        );
        self.banks = Some((left, right));
        let c = ctx.audio.start_loop(
            sound::CURRENT,
            Some(thing_pos(self.run.current(), 1.0)),
            Bus::EFFECTS,
        );
        self.current = Some(c);
        self.sync_scene(ctx);
    }

    /// Stops every sound the run started, for a lesson that closes.
    pub fn silence(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for (_, s) in self.loops.drain(..) {
            ctx.audio.stop(s, 60);
        }
        if let Some((l, r)) = self.banks.take() {
            ctx.audio.stop(l, 200);
            ctx.audio.stop(r, 200);
        }
        if let Some(c) = self.current.take() {
            ctx.audio.stop(c, 200);
        }
    }

    /// Moves every loop to where its thing is now.
    fn sync_scene(&mut self, ctx: &mut Ctx<Parlor>) {
        let pos = self.run.beat_pos();
        let live: Vec<u32> = self.run.things.iter().map(|t| t.id).collect();
        let mut kept = Vec::new();
        for (id, s) in self.loops.drain(..) {
            if live.contains(&id) {
                kept.push((id, s));
            } else {
                ctx.audio.stop(s, 80);
            }
        }
        self.loops = kept;
        for t in &self.run.things {
            let left = t.beats_left(pos);
            let lead = if t.kind == Kind::Mill {
                MILL_LEAD_BEATS
            } else {
                LEAD_BEATS
            } as f64;
            let at = thing_pos(t.x_at(pos), left);
            let s = match self.loops.iter().find(|(id, _)| *id == t.id) {
                Some((_, s)) => *s,
                None => {
                    let s = ctx
                        .audio
                        .start_loop(sound::floating(t.kind), Some(at), Bus::EFFECTS);
                    self.loops.push((t.id, s));
                    s
                }
            };
            ctx.audio.set_position(s, Some(at));
            if t.kind.is_catch() {
                ctx.audio.set_pitch(s, sound::float_pitch(left, lead));
            }
            ctx.audio.set_volume(s, sound::float_volume(left, lead));
        }
        let x = self.run.x;
        if let Some((l, r)) = self.banks {
            let (vl, vr) = if self.bank_lapping {
                (sound::bank_volume(x, -1.0), sound::bank_volume(x, 1.0))
            } else {
                (0.15, 0.15)
            };
            ctx.audio.set_volume(l, vl);
            ctx.audio.set_volume(r, vr);
        }
        if let Some(c) = self.current {
            ctx.audio
                .set_position(c, Some(thing_pos(self.run.current_at(pos + 1.0), 1.0)));
            ctx.audio
                .set_volume(c, if self.run.riding { 0.9 } else { 0.6 });
        }
        sound::listen_from(ctx, x);
    }

    /// A paddle stroke on the side you steer toward, the moment you start
    /// and then every stroke while you hold.
    fn paddle(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.run.steer == 0 || !self.run.is_playing() {
            self.stroke_in = 0.0;
            return;
        }
        self.stroke_in -= dt;
        if self.stroke_in <= 0.0 {
            let side = f64::from(self.run.steer) * 0.6;
            let key = sound::PADDLE[self.stroke % sound::PADDLE.len()];
            ctx.audio
                .play_at(key, thing_pos(self.run.x + side, 0.0), Bus::EFFECTS);
            self.stroke += 1;
            self.stroke_in = sound::STROKE_SECS;
        }
    }

    /// Left Shift and Right Shift: one hard stroke a press. The engine keeps
    /// modifier keys from screens, so they are read as held keys.
    fn hard_strokes(&mut self, ctx: &mut Ctx<Parlor>) {
        let now = (
            ctx.held.physically_down(Key::LShift),
            ctx.held.physically_down(Key::RShift),
        );
        let dir = match (now.0 && !self.shifts.0, now.1 && !self.shifts.1) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
        self.shifts = now;
        if self.run.hard_stroke(dir) {
            let side = f64::from(dir) * 0.6;
            ctx.audio.play_at(
                sound::PADDLE_HARD,
                thing_pos(self.run.x + side, 0.0),
                Bus::EFFECTS,
            );
            // The next ordinary stroke waits its turn after the hard one.
            self.stroke_in = sound::STROKE_SECS;
        }
    }

    fn on_event(&mut self, ctx: &mut Ctx<Parlor>, event: Event) {
        let at = |x: f64| thing_pos(x, 0.0);
        match event {
            Event::Go => {
                ctx.audio.play(sound::LAUNCH, Bus::EFFECTS);
            }
            Event::Heard { .. } => {}
            Event::Introduce(kind) => {
                self.intro_pending.push(kind);
                let mut p = Progress::load(ctx);
                p.meet(kind);
                p.save(ctx);
            }
            Event::Caught {
                kind,
                x,
                clean,
                combo,
                trinket,
                ..
            } => {
                self.conductor
                    .catch_note(ctx, combo, if clean { 1.0 } else { 0.8 });
                if clean {
                    ctx.audio.play_at(sound::CLEAN, at(x), Bus::EFFECTS);
                }
                match kind {
                    Kind::Scrap => {
                        ctx.audio.play_at(sound::SCRAP, at(x), Bus::EFFECTS);
                    }
                    Kind::Trinket | Kind::Lantern => {
                        ctx.audio.play_at(sound::TRINKET, at(x), Bus::EFFECTS);
                    }
                    _ => {}
                }
                if let Some(id) = trinket {
                    if self.trinket_names {
                        if let Some(t) = shelf::get(id) {
                            self.tell(ctx, format!("{}.", t.name));
                        }
                    }
                }
            }
            Event::Missed { .. } => {}
            Event::Through { kind, x, .. } => {
                let key = if kind == Kind::Falls {
                    sound::FALLS
                } else {
                    sound::GATE
                };
                ctx.audio.play_at(key, at(x), Bus::EFFECTS);
            }
            Event::Passed { .. } | Event::Around { .. } => {}
            Event::Bump {
                hazard, x, soggy, ..
            } => {
                let key = match hazard {
                    None => sound::BUMP_BANK,
                    Some(Kind::Mill) => sound::BUMP_MILL,
                    Some(Kind::Duck) => sound::BUMP_DUCK,
                    Some(Kind::Falls) => sound::BUMP_FALLS,
                    Some(_) => sound::BUMP_STONE,
                };
                ctx.audio.play_at(key, at(x), Bus::EFFECTS);
                if self.run.course.mode == Mode::Endless && soggy < SOGGY_SINKS {
                    self.tell(ctx, format!("Soggy, {soggy} of {SOGGY_SINKS}."));
                }
            }
            Event::Multiplier(_) => {
                ctx.audio.play(sound::MULTIPLIER, Bus::EFFECTS);
            }
            Event::NewBest => {
                ctx.audio.play(sound::NEW_BEST, Bus::EFFECTS);
                self.tell(ctx, "New best.");
            }
            Event::Current(on) => {
                let key = if on {
                    sound::CURRENT_IN
                } else {
                    sound::CURRENT_OUT
                };
                ctx.audio.play(key, Bus::EFFECTS);
            }
            Event::Dried { soggy } => {
                self.tell(ctx, format!("Drying out. Soggy, {soggy} of {SOGGY_SINKS}."));
            }
            Event::Mouth => {
                ctx.audio.play(sound::MOUTH, Bus::EFFECTS);
                self.tell(ctx, "River mouth.");
            }
            Event::Sank => {
                ctx.audio.play(sound::SANK, Bus::EFFECTS);
                self.tell(ctx, "The boat sank.");
            }
        }
    }

    /// Saves the run and opens the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.done = true;
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
        let mouth = self.run.reached_mouth();
        ctx.audio.play_music(
            Some(if mouth {
                sound::MOUTH_MUSIC
            } else {
                sound::SANK_MUSIC
            }),
            false,
        );
        let (title, lines) = self.results_text(ctx);
        ctx.replace(results(self.launch, title, lines));
    }

    /// Keeps what the run earned, and says it.
    fn results_text(&mut self, ctx: &mut Ctx<Parlor>) -> (String, Vec<String>) {
        let r = &self.run;
        let mode = r.course.mode;
        let mut progress = Progress::load(ctx);
        let mut title = if mode == Mode::Endless {
            format!("The boat sank at {} metres.", r.metres())
        } else {
            String::new()
        };
        let mut lines = Vec::new();
        let caught = format!(
            "{} of {} things caught, {} clean. Longest combo {}.",
            r.caught, r.offered, r.clean, r.best_combo
        );
        // Score and record.
        if let Some(challenge) = self.challenge.clone() {
            let result = Challenges::submit(ctx, &challenge, r.score);
            let period = challenge.period;
            let streak = Challenges::streak(ctx, ID, period);
            title = format!("{title} {} points. {}", r.score, short_result(result));
            lines.push(format!("{} result: {} points.", challenge.title(), r.score));
            lines.push(score_line(result));
            lines.push(format!(
                "{} streak: {streak} {}{}. {}",
                period.label(),
                period.unit(),
                if streak == 1 { "" } else { "s" },
                challenge.resets()
            ));
        } else if let Some(board) = mode.board() {
            let score = if mode == Mode::Endless {
                r.metres()
            } else {
                r.score
            };
            let result = Records::record(ctx, ID, board, score);
            if mode == Mode::Endless {
                match result {
                    ScoreResult::First => title.push_str(" Your first run here."),
                    ScoreResult::NewBest { .. } => title.push_str(" A new best."),
                    ScoreResult::Short { best } => title.push_str(&format!(" Best {best} metres.")),
                }
                lines.push(format!("{} points on the way.", r.score));
                Records::record(ctx, ID, "endless_score", r.score);
            } else {
                title = format!("{title} {}", short_score(r.score, result));
                lines.push(score_line(result));
            }
        } else {
            title = format!(
                "{} of {} things caught. Nothing is scored here.",
                r.caught, r.offered
            );
        }
        lines.push(caught);
        // Stars and what they open.
        if let Mode::Stream(stream) = mode {
            let stars = r.stars();
            let before: Vec<Stream> = progress.open_streams();
            let endless_before = progress.endless_open();
            let better = progress.keep_stars(stream, stars);
            let mut line = format!("{} on {}.", stars_phrase(stars), stream.name());
            if better {
                line.push_str(" Your best there.");
            }
            title.push_str(&format!(
                " {} on {}.",
                sentence_start(&stars_phrase(stars)),
                stream.name()
            ));
            lines.push(line);
            for s in progress.open_streams() {
                if !before.contains(&s) {
                    lines.push(format!("{} is open.", s.name()));
                    title.push_str(&format!(" {} is open.", s.name()));
                }
            }
            if !endless_before && progress.endless_open() {
                lines.push("The endless river is open.".to_string());
                title.push_str(" The endless river is open.");
            }
            for ribbon in ribbons_of(stream) {
                if !progress.has_ribbon(ribbon.id) && ribbon.earned(r) {
                    progress.ribbons.push(ribbon.id.to_string());
                    progress.scraps += RIBBON_SCRAPS;
                    lines.push(format!(
                        "Ribbon: {}. {RIBBON_SCRAPS} paper scraps.",
                        ribbon.name
                    ));
                    title.push_str(&format!(" Ribbon: {}.", ribbon.name.to_lowercase()));
                    ctx.audio.play(sound::RIBBON, Bus::EFFECTS);
                }
            }
        }
        // Scraps and trinkets are kept from every mode but practice.
        if mode != Mode::Practice {
            if r.scraps > 0 {
                progress.scraps += r.scraps;
                lines.push(format!(
                    "{} paper {} picked up. {} in all.",
                    r.scraps,
                    if r.scraps == 1 { "scrap" } else { "scraps" },
                    progress.scraps
                ));
            }
            let before = progress.trinkets.clone();
            let mut new = Vec::new();
            for &id in &r.trinkets {
                if !progress.trinkets.iter().any(|t| t == id) {
                    progress.trinkets.push(id.to_string());
                    if let Some(t) = shelf::get(id) {
                        new.push(t.name);
                    }
                }
            }
            match new.len() {
                0 => {}
                1 => {
                    lines.push(format!("New on your shelf: {}.", new[0].to_lowercase()));
                    title.push_str(" A new trinket.");
                }
                n => {
                    lines.push(format!(
                        "New on your shelf: {}.",
                        new.iter()
                            .map(|n| n.to_lowercase())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                    title.push_str(&format!(" {n} new trinkets."));
                }
            }
            for set in shelf::sets_completed(&before, &progress.trinkets) {
                lines.push(shelf::set_story(set).to_string());
            }
            for boat in Boat::ALL {
                if !progress.has_boat(boat)
                    && progress.scraps >= boat.scraps()
                    && progress.scraps - r.scraps < boat.scraps()
                {
                    lines.push(format!(
                        "Enough scraps to fold a {}: the boatyard is on the title.",
                        boat.name().to_lowercase()
                    ));
                }
            }
        }
        progress.save(ctx);
        (title.trim().to_string(), lines)
    }
}

/// "a new best", "best 900".
fn short_result(r: ScoreResult) -> String {
    match r {
        ScoreResult::First => "Your first score.".to_string(),
        ScoreResult::NewBest { .. } => "A new best.".to_string(),
        ScoreResult::Short { best } => format!("Best {best}."),
    }
}

/// "4820 points, a new best."
pub fn short_score(score: u64, r: ScoreResult) -> String {
    match r {
        ScoreResult::First => format!("{score} points, your first score here."),
        ScoreResult::NewBest { .. } => format!("{score} points, a new best."),
        ScoreResult::Short { best } => format!("{score} points, best {best}."),
    }
}

/// What saving a score meant.
pub fn score_line(r: ScoreResult) -> String {
    match r {
        ScoreResult::First => "Your first score here is saved.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best. The old best was {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
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
        self.build(ctx);
        sound::on_the_water(ctx);
        let line = self.opening_line(ctx);
        self.run
            .set_lead_in(speech_secs(&line).clamp(MIN_LEAD_IN, MAX_LEAD_IN));
        self.tell(ctx, line);
        self.start_loops(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        let p = Progress::load(ctx);
        self.trinket_names = p.trinket_names;
        self.bank_lapping = p.bank_lapping;
        self.run.clean_width = p.clean_width();
        if self.frozen {
            self.frozen = false;
            ctx.audio.resume_all();
            return;
        }
        let text = format!("Back on the water. Combo {}.", self.run.combo);
        ctx.say(text);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || self.done {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            let leave = if self.launch.mode.period().is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let Some(action) = ctx.action_for(event) else {
            return;
        };
        match action {
            ParlorAction::Primary => {
                if self.run.start_now() {
                    // The Go event plays the launch.
                } else {
                    let l = ctx.control_name(ParlorAction::Left);
                    let r = ctx.control_name(ParlorAction::Right);
                    ctx.say(format!("{} and {r} paddle.", sentence_start(&l)));
                }
            }
            ParlorAction::Status => {
                let layers = self.run.status_layers();
                self.status_key.speak(ctx, layers);
            }
            ParlorAction::Hint => {
                let text = self.run.hint();
                ctx.say(text);
            }
            ParlorAction::Repeat => {
                if self.last.is_empty() {
                    ctx.say("Nothing to repeat yet.");
                } else {
                    let text = self.last.clone();
                    ctx.say(text);
                }
            }
            ParlorAction::Up | ParlorAction::Down | ParlorAction::Secondary => {
                let l = ctx.control_name(ParlorAction::Left);
                let r = ctx.control_name(ParlorAction::Right);
                ctx.say(format!(
                    "The stream carries you. {} and {r} paddle.",
                    sentence_start(&l)
                ));
            }
            ParlorAction::Left | ParlorAction::Right => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.done || self.frozen {
            return;
        }
        if !self.intro_pending.is_empty() {
            let kind = self.intro_pending.remove(0);
            self.frozen = true;
            ctx.audio.pause_all();
            self.last = introduction(kind).to_string();
            ctx.push(PlayPage::new(introduction(kind)));
            return;
        }
        let left = ctx.action_held(ParlorAction::Left);
        let right = ctx.action_held(ParlorAction::Right);
        self.run.steer = match (left, right) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
        self.paddle(ctx, dt);
        self.hard_strokes(ctx);
        self.run.update(dt);
        for event in self.run.take_events() {
            self.on_event(ctx, event);
        }
        if self.run.is_over() {
            self.finish(ctx);
            return;
        }
        if self.run.is_playing() {
            let everything = self.run.course.mode == Mode::Drift;
            let time = self.run.time;
            let combo = self.run.combo;
            self.conductor.follow(ctx, time, combo, everything);
        }
        self.sync_scene(ctx);
        if !self.intro_pending.is_empty() {
            let kind = self.intro_pending.remove(0);
            self.frozen = true;
            ctx.audio.pause_all();
            self.last = introduction(kind).to_string();
            ctx.push(PlayPage::new(introduction(kind)));
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let r = &self.run;
        let phase = match r.phase {
            Phase::LeadIn(_) => "ready",
            Phase::Playing => "playing",
            Phase::Ending(_) => "mouth",
            Phase::Over => "over",
        };
        let mut lines = vec![
            format!("{}, {phase}, beat {}", r.course.mode.label(), r.beat),
            format!(
                "boat {:.2}, current {:.2}, riding {}",
                r.x,
                r.current(),
                r.riding
            ),
            format!(
                "score {}, combo {}, caught {} of {}, bumps {}",
                r.score, r.combo, r.caught, r.offered, r.bumps
            ),
        ];
        let pos = r.beat_pos();
        for t in &r.things {
            lines.push(format!(
                "coming: {} at {:.2} in {:.1} beats",
                t.kind.id(),
                t.x_at(pos),
                t.beats_left(pos)
            ));
        }
        lines
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        controls_text(ctx)
    }

    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// The controls in a paragraph, naming the player's own keys.
pub fn controls_text(ctx: &Ctx<Parlor>) -> String {
    let left = ctx.control_name(ParlorAction::Left);
    let right = ctx.control_name(ParlorAction::Right);
    let status = ctx.control_name(ParlorAction::Status);
    let hint = ctx.control_name(ParlorAction::Hint);
    let repeat = ctx.control_name(ParlorAction::Repeat);
    format!(
        "Hold {left} or {right} to paddle that way, and Left Shift or Right Shift for one hard stroke. Catch things as they reach you, steer clear of stones and ducks, and ride the current for points. {} gives your combo, and again for your score. {} says where the current is and what comes next. {} repeats the last line. Escape pauses.",
        sentence_start(&status),
        sentence_start(&hint),
        sentence_start(&repeat),
    )
}

/// A page that opens in the middle of a run. Only Enter, Escape and the
/// repeat key work, so a steering key held a moment late cannot skip it.
pub struct PlayPage {
    inner: Pages,
}

impl PlayPage {
    pub fn new(text: impl Into<String>) -> PlayPage {
        PlayPage {
            inner: Pages::new([text.into()]),
        }
    }
}

impl State<Parlor> for PlayPage {
    fn name(&self) -> &'static str {
        "pages"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.inner.enter(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.inner.resume(ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let passes = matches!(
            event.pressed(),
            Some(Key::Return | Key::KpEnter | Key::Escape)
        ) || ctx.action_for(event) == Some(ParlorAction::Repeat);
        if passes {
            self.inner.handle_input(ctx, event);
        }
    }

    fn lines(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        self.inner.lines(ctx)
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        self.inner.help(ctx)
    }
}

/// After a run: what happened, and what next.
pub fn results(launch: Launch, title: String, lines: Vec<String>) -> Results {
    let back = match launch.mode.period() {
        Some(_) => Back::Challenges(CHALLENGES_NAME),
        None => Back::Title(TITLE_NAME),
    };
    let (again, help) = match launch.mode.period() {
        Some(period) => (
            labels::challenge_again(period),
            "The same stream again. Only your best result is kept.",
        ),
        None => (
            "Sail again".to_string(),
            "The same stream again, with new things floating down.",
        ),
    };
    Results::new(title, lines, back).again(again, help, move |ctx| {
        let launch = Launch {
            seed: if launch.mode.period().is_some() {
                launch.seed
            } else {
                launch.seed.map(|s| s.wrapping_add(1))
            },
            ..launch
        };
        start(ctx, launch, true)
    })
}

mod visuals_runscreen;
