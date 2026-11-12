//! The kitchen: a meal cooked by ear, station by station.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, Leave};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::{Mood, Pace, Parlor, ParlorAction, RunClock, Score};

use crate::challenge::{self, ChallengePlan, ChallengeRun};
use crate::progress::{self, Progress};
use crate::results;
use crate::rules::{
    self, course, helping, meal_rng, sentence, Course, Event, Grade, Meal, Stage, Station,
};
use crate::sound;
use crate::thanksgiving;
use crate::{CHALLENGES_NAME, HELP, TITLE_NAME};

/// What a meal is part of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A campaign course, 1 to 8.
    Course(u32),
    /// A second-helpings round, with the points of the rounds before it.
    Helpings { round: u32, total: u64 },
    /// A daily, weekly or monthly challenge meal.
    Challenge(ChallengeRun),
    /// The family's Thanksgiving dinner, cooked on the day in `year`.
    Thanksgiving { year: i32 },
}

impl Mode {
    /// "Course 3, The side dishes", "Second helpings, round 2".
    pub fn title(&self) -> String {
        match self {
            Mode::Course(n) => format!("Course {n}, {}", course(*n).name),
            Mode::Helpings { round, .. } => format!("Second helpings, round {round}"),
            Mode::Challenge(run) => run.title(),
            Mode::Thanksgiving { .. } => thanksgiving::NAME.to_string(),
        }
    }
}

/// How the meal is going.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// The menu has been read; waiting for the player to start cooking.
    Ready,
    Cooking,
    /// Everything is served; the results follow in this many seconds.
    Served(f64),
}

/// How long the dinner bell rings before the results.
const SERVED_PAUSE: f64 = 2.5;

pub struct Kitchen {
    pub mode: Mode,
    pub course: Course,
    pub meal: Meal,
    pub phase: Phase,
    pub focus: Station,
    pub hints: u32,
    callouts: bool,
    /// The meal's game clock: its pace, fixed when the meal starts, and
    /// the hold that keeps every dish still while a line is spoken.
    pub clock: RunClock,
    /// The pace carried from the meal before, for a run of several meals
    /// (second helpings, a challenge's courses): the whole run keeps the
    /// pace it started at.
    carried: Option<Pace>,
    /// Call-outs were off from the start and never switched on: the meal
    /// earns the by-ear bonus.
    by_ear: bool,
    /// The challenge this meal belongs to, if any.
    plan: Option<ChallengePlan>,
    /// Golden plates served this banquet.
    plates: u32,
    loops: [(Sound, Option<&'static str>, f32); 5],
    last_prompt: String,
    /// The kitchen's music, following the meal.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// Cooking inside the tutorial: the lesson's prompts lead, the kitchen's
    /// own lines wait behind them, and no helper steps in.
    lesson: bool,
    /// Seconds the meal's clock stands still, so a lesson's step is heard
    /// before the kitchen judges timing.
    pub hold: f64,
}

impl Kitchen {
    pub fn new(mode: Mode) -> Kitchen {
        let course = match &mode {
            Mode::Course(n) => course(*n),
            Mode::Helpings { round, .. } => helping(*round, &mut meal_rng()),
            Mode::Challenge(run) => run.plan().course(run.stage),
            Mode::Thanksgiving { .. } => thanksgiving::dinner(),
        };
        Kitchen::with_course(mode, course)
    }

    /// The kitchen inside the tutorial.
    pub fn in_lesson(mut self) -> Kitchen {
        self.lesson = true;
        self.meal.patient = true;
        self
    }

    /// A meal of exactly this course.
    pub fn with_course(mode: Mode, course: Course) -> Kitchen {
        let mut meal = Meal::new(&course);
        let (plan, plates) = match &mode {
            Mode::Challenge(run) => {
                let plan = run.plan();
                plan.apply(&mut meal);
                (Some(plan), run.plates)
            }
            _ => (None, 0),
        };
        let focus = meal.stations().first().copied().unwrap_or(Station::Oven);
        Kitchen {
            status_key: Default::default(),
            mode,
            course,
            meal,
            phase: Phase::Ready,
            focus,
            hints: 0,
            callouts: true,
            clock: RunClock::default(),
            carried: None,
            by_ear: false,
            plan,
            plates,
            loops: [(Sound::NONE, None, 0.0); 5],
            last_prompt: String::new(),
            score: Score::new(),
            lesson: false,
            hold: 0.0,
        }
    }

    /// What the score hears: more dishes on the stove and more of the meal
    /// served build it up; a dish that is due hurries it, and one that is
    /// burning darkens it.
    pub fn mood(&self) -> Mood {
        if self.phase != Phase::Cooking {
            return Mood::new(0.0);
        }
        let dishes = &self.meal.dishes;
        let stations = self.meal.stations().len().max(1) as f32;
        let busy = dishes.iter().filter(|d| d.started() && !d.done()).count() as f32;
        let served = self.meal.served() as f32 / dishes.len().max(1) as f32;
        Mood::new(0.2 + 0.5 * (busy / stations).min(1.0) + 0.3 * served)
            .with_flag(HURRY, dishes.iter().any(|d| d.stage == Stage::Due))
            .with_flag(DANGER, dishes.iter().any(|d| d.stage == Stage::Burning))
    }

    fn primary(ctx: &Ctx<Parlor>) -> String {
        ctx.control_name(ParlorAction::Primary)
    }

    /// The next meal of the same run, at this run's pace.
    pub fn at_pace(mut self, pace: Pace) -> Kitchen {
        self.carried = Some(pace);
        self
    }

    fn menu_line(&self) -> String {
        let dishes: Vec<String> = self
            .meal
            .dishes
            .iter()
            .map(|d| format!("{} {}", d.name(), d.station().on()))
            .collect();
        format!("On the menu: {}.", join(&dishes))
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let p = Self::primary(ctx);
        let head = match &self.mode {
            Mode::Course(_) => format!("{}. {}", self.mode.title(), self.course.blurb),
            Mode::Helpings { total, .. } => {
                let carried = if *total > 0 {
                    format!(" {total} points so far.")
                } else {
                    String::new()
                };
                format!("{}.{carried}", self.mode.title())
            }
            Mode::Challenge(run) => {
                let carried = if run.total > 0 {
                    format!(" {} points so far.", run.total)
                } else {
                    String::new()
                };
                let rule = self.plan.as_ref().map(|p| p.reminder()).unwrap_or_default();
                format!("{}.{carried} {rule}", self.mode.title())
            }
            Mode::Thanksgiving { year } => format!(
                "Thanksgiving dinner, {year}. {} {}",
                self.course.blurb,
                thanksgiving::family(*year).line()
            ),
        };
        // The first course teaches the loop; later ones only say the key.
        let then = if matches!(self.mode, Mode::Course(1)) {
            format!(" Then listen for the recipe, go to the station it names, and press {p}.")
        } else {
            String::new()
        };
        format!(
            "{head} {} {} starts cooking.{then}",
            self.menu_line(),
            sentence_start(&p),
        )
    }

    /// What R says when nothing has been called yet.
    fn prompt(&self, ctx: &Ctx<Parlor>) -> String {
        match self.phase {
            Phase::Ready => self.intro(ctx),
            Phase::Served(_) => "Everything is served. The results are coming.".to_string(),
            Phase::Cooking if self.last_prompt.is_empty() => {
                "Cooking. Listen for the recipe to call the first dish.".to_string()
            }
            Phase::Cooking => self.last_prompt.clone(),
        }
    }

    /// "Left burner, far left: gravy, cooking, next: whisk."
    fn station_line(&self, station: Station) -> String {
        format!(
            "{}, {}: {}.",
            station.title(),
            station.place(),
            rules::describe_station(&self.meal, station)
        )
    }

    /// The status key, in layers. First: any station that needs you now,
    /// where you stand and how many dishes are served. A second press:
    /// every other station and the clock. A third: the course, quality and
    /// the challenge's rule.
    fn status(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let n = self.meal.dishes.len();
        let urgent = |s: rules::Station| {
            self.meal.active_at(s).is_some_and(|i| {
                matches!(
                    self.meal.dishes[i].stage,
                    rules::Stage::Due | rules::Stage::Burning
                )
            })
        };
        let line = |s: rules::Station| {
            format!("{}: {}.", s.title(), rules::describe_station(&self.meal, s))
        };
        let stations = self.meal.stations();
        let mut first = Vec::new();
        let mut more = Vec::new();
        if self.phase == Phase::Ready {
            first.push(format!(
                "Not started; {} starts cooking.",
                Self::primary(ctx)
            ));
        } else {
            first.extend(stations.iter().copied().filter(|&s| urgent(s)).map(line));
        }
        first.push(format!("You are at the {}.", self.focus.name()));
        if self.phase != Phase::Ready {
            first.push(format!("{} of {n} dishes served.", self.meal.served()));
            more.push(format!(
                "{} in.",
                sentence_start(&rules::seconds(self.meal.elapsed))
            ));
        }
        more.extend(stations.iter().copied().filter(|&s| !urgent(s)).map(line));
        let outcome = self.meal.outcome();
        let mut course = vec![format!("{}.", self.mode.title())];
        if self.meal.dishes.iter().any(|d| !d.scores.is_empty()) {
            course.push(format!("Average quality so far {}.", outcome.average));
        }
        if let Some(p) = self.plan.as_ref() {
            course.push(p.reminder());
        }
        vec![
            parlor_core::status::sentences(first),
            parlor_core::status::sentences(more),
            parlor_core::status::sentences(course),
        ]
    }

    fn recipe_card(&self) -> String {
        let lines: Vec<String> = self
            .meal
            .dishes
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let state = if d.done() {
                    format!("served, quality {}", d.quality())
                } else if d.stage == Stage::Queued {
                    let first = d.after.map(|j| self.meal.dishes[j].name()).unwrap_or("");
                    format!("after the {first}")
                } else if !d.started() {
                    match d.stage {
                        Stage::Due | Stage::Burning => format!("called: start {} now", d.kind.it()),
                        _ => "not called yet".to_string(),
                    }
                } else {
                    match d.stage {
                        Stage::Due | Stage::Burning => "needs you now".to_string(),
                        _ => "cooking".to_string(),
                    }
                };
                format!(
                    "{}, {} {}: {state}.",
                    i + 1,
                    d.kind.start_phrase(),
                    d.station().on()
                )
            })
            .collect();
        format!("Recipe card. {}", lines.join(" "))
    }

    /// "2 stations to your left", "right here".
    fn direction(&self, to: Station) -> String {
        let used = self.meal.stations();
        let here = used.iter().position(|s| *s == self.focus).unwrap_or(0) as i32;
        let there = used.iter().position(|s| *s == to).unwrap_or(0) as i32;
        let d = there - here;
        match d {
            0 => "in front of you".to_string(),
            d if d < 0 => format!("{} to your left, key {}", -d, to.number()),
            d => format!("{d} to your right, key {}", to.number()),
        }
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play(sound::HINT, Bus::UI);
        self.hints += 1;
        let line = if self.phase == Phase::Ready {
            format!(
                "{} starts cooking. Then wait for the recipe to call the first dish.",
                sentence_start(&Self::primary(ctx))
            )
        } else if let Some(i) = self.meal.most_urgent() {
            let d = &self.meal.dishes[i];
            let what = if d.started() {
                d.current()
                    .map_or("act".to_string(), |s| s.action.verb(d.kind.plural()))
            } else {
                "start".to_string()
            };
            format!(
                "{}: {what} now, {}.",
                sentence_start(d.name()),
                self.direction(d.station())
            )
        } else if let Some((i, secs)) = self.meal.next_up() {
            let d = &self.meal.dishes[i];
            let what = if d.started() {
                d.current()
                    .map_or(String::new(), |s| s.action.verb(d.kind.plural()))
            } else {
                "start it".to_string()
            };
            format!(
                "Nothing needs you right now. Next, the {} {}: {what} in about {}.",
                d.name(),
                d.station().on(),
                rules::seconds(secs)
            )
        } else {
            "Nothing needs you right now.".to_string()
        };
        ctx.say(line);
    }

    /// What the focused station sounds like now, and what that means.
    fn listen(&self, ctx: &mut Ctx<Parlor>) {
        let st = self.focus;
        let line = match self.meal.active_at(st).map(|i| &self.meal.dishes[i]) {
            None => format!("The {} is quiet: nothing on it.", st.name()),
            Some(d) if !d.started() => match d.stage {
                Stage::Due => format!("Double knock, {}: start the {}.", st.name(), d.name()),
                Stage::Burning => {
                    format!("Triple knock, {}: start the {} now!", st.name(), d.name())
                }
                _ => format!(
                    "The {} is quiet: the {} {} not called for yet.",
                    st.name(),
                    d.name(),
                    d.kind.is()
                ),
            },
            Some(d) => match d.stage {
                Stage::Due => format!(
                    "You hear {}: the {} {} {}.",
                    st.due_sound(),
                    d.name(),
                    d.kind.needs(),
                    d.current()
                        .map_or("you".to_string(), |s| s.action.gerund(d.kind.plural()))
                ),
                Stage::Burning => format!(
                    "You hear {}: the {} {} burning. Act now.",
                    st.burning_sound(),
                    d.name(),
                    d.kind.is()
                ),
                _ => format!(
                    "You hear {}: the {} {} cooking happily.",
                    st.cooking_sound(),
                    d.name(),
                    d.kind.is()
                ),
            },
        };
        ctx.say(line);
    }

    fn set_focus(&mut self, ctx: &mut Ctx<Parlor>, station: Station) {
        self.focus = station;
        sound::at(ctx, sound::FOCUS, station);
        let line = self.station_line(station);
        ctx.say(line);
        self.sync_loops(ctx);
    }

    fn step_focus(&mut self, ctx: &mut Ctx<Parlor>, delta: i32) {
        let used = self.meal.stations();
        let here = used.iter().position(|s| *s == self.focus).unwrap_or(0) as i32;
        let there = here + delta;
        if there < 0 || there >= used.len() as i32 {
            sound::at(ctx, sound::EDGE, self.focus);
            let side = if delta < 0 { "left" } else { "right" };
            ctx.say(format!(
                "The {} is the last station on the {side} this course.",
                self.focus.name()
            ));
            return;
        }
        self.set_focus(ctx, used[there as usize]);
    }

    fn jump(&mut self, ctx: &mut Ctx<Parlor>, station: Station) {
        if !self.meal.uses(station) {
            sound::at(ctx, sound::EDGE, station);
            ctx.say(format!(
                "The {}, key {}, is not in use this course.",
                station.name(),
                station.number()
            ));
            return;
        }
        self.set_focus(ctx, station);
    }

    fn begin(&mut self, ctx: &mut Ctx<Parlor>) {
        self.phase = Phase::Cooking;
        // The recipe cue waits for the first call; the lesson's own prompt
        // says what comes next.
        if !self.lesson {
            ctx.say("Cooking. Listen for the recipe.");
        }
        let events = self.meal.update(0.0);
        self.voice(ctx, events);
    }

    fn act(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.lesson {
            if let Some(i) = self.meal.active_at(self.focus) {
                let d = &self.meal.dishes[i];
                if let Some(s) = d.current() {
                    if d.clock < s.wait - rules::EARLY_GRACE {
                        sound::at(ctx, sound::EARLY, d.station());
                        let line = if s.action == rules::Action::Start {
                            format!("Not yet: the {} {} not called.", d.name(), d.kind.is())
                        } else {
                            format!("Not yet: the {} {} time.", d.name(), d.kind.needs())
                        };
                        ctx.say(line);
                        return;
                    }
                }
            }
        }
        let events = self.meal.act(self.focus);
        self.voice(ctx, events);
    }

    /// Plays and speaks what happened.
    fn voice(&mut self, ctx: &mut Ctx<Parlor>, events: Vec<Event>) {
        let mut lines: Vec<String> = Vec::new();
        for event in events {
            match event {
                Event::Called(i) => {
                    let d = &self.meal.dishes[i];
                    ctx.audio.play(sound::RECIPE, Bus::UI);
                    let line = rules::called_line(d.kind);
                    self.last_prompt = line.clone();
                    lines.push(line);
                }
                Event::Due(i) => {
                    if self.callouts {
                        let line = rules::due_line(&self.meal.dishes[i]);
                        self.last_prompt = line.clone();
                        lines.push(line);
                    }
                }
                Event::Burning(i) => {
                    if self.callouts {
                        let line = rules::burning_line(&self.meal.dishes[i]);
                        self.last_prompt = line.clone();
                        lines.push(line);
                    }
                }
                Event::Ruined { dish, action } => {
                    let d = &self.meal.dishes[dish];
                    sound::at(ctx, sound::RUINED, d.station());
                    lines.push(rules::ruined_line(d, action));
                }
                Event::Done {
                    dish,
                    action,
                    score,
                } => {
                    let d = &self.meal.dishes[dish];
                    let grade = Grade::for_score(score);
                    sound::at(ctx, sound::action(d.station(), action), d.station());
                    ctx.audio.play(sound::grade(grade), Bus::UI);
                    if action == rules::Action::Start {
                        // In the lesson, the step's own line says it.
                        if !self.lesson {
                            lines.push(format!("{} started. {}", sentence(d.name()), grade.word()));
                        }
                    } else if self.lesson && d.done() {
                        // The lesson's closing line names the finished dish.
                    } else {
                        lines.push(grade.word().to_string());
                    }
                }
                Event::Stroke { dish, done, of } => {
                    if self.clock.pace() == Pace::Relaxed {
                        lines.push(format!("{done} of {of} strokes."));
                    }
                    let d = &self.meal.dishes[dish];
                    let action = d.current().map_or(rules::Action::Chop, |s| s.action);
                    sound::at(ctx, sound::action(d.station(), action), d.station());
                }
                Event::Fumble { dish, action } => {
                    let d = &self.meal.dishes[dish];
                    sound::at(ctx, sound::EARLY, d.station());
                    if action == rules::Action::Start {
                        lines.push(format!(
                            "Not yet: the {} {} not called. Minus {}.",
                            d.name(),
                            d.kind.is(),
                            d.fumble_cost
                        ));
                    } else {
                        lines.push(format!(
                            "Not yet: the {} {} time. Minus {}.",
                            d.name(),
                            d.kind.needs(),
                            d.fumble_cost
                        ));
                    }
                }
                // The lesson closes as its dish is done, before the meal is
                // served and the dinner bell rings.
                Event::Served(_) | Event::MealOver if self.lesson => {}
                Event::Served(i) => {
                    let d = &self.meal.dishes[i];
                    sound::at(ctx, sound::PLATE, d.station());
                    lines.push(format!(
                        "{} served, quality {}.",
                        sentence(d.name()),
                        d.quality()
                    ));
                    if let Some(plan) = &self.plan {
                        let golden =
                            plan.twist.is_none() && d.quality() >= challenge::GOLDEN_QUALITY;
                        if golden {
                            self.plates += 1;
                            ctx.audio.play(sound::GOLDEN, Bus::UI);
                        }
                        if let Some(note) =
                            challenge::served_note(plan, d.kind, d.quality(), self.plates)
                        {
                            lines.push(note);
                        }
                    }
                    if let Some(j) = self.meal.active_at(d.station()).filter(|&j| j != i) {
                        lines.push(format!(
                            "The {} is free for the {}.",
                            d.station().name(),
                            self.meal.dishes[j].name()
                        ));
                    }
                }
                Event::Empty(station) => {
                    sound::at(ctx, sound::EMPTY, station);
                    // Moving here already said the station is all served.
                    if self.meal.uses(station) && self.meal.active_at(station).is_none() {
                        lines.push("Nothing to do here.".to_string());
                    } else {
                        lines.push(format!(
                            "Nothing to do here: {}.",
                            rules::describe_station(&self.meal, station)
                        ));
                    }
                }
                Event::MealOver => {
                    ctx.audio.play(sound::BELL, Bus::EFFECTS);
                    lines.push("Everything is served. The dinner bell rings.".to_string());
                    self.phase = Phase::Served(SERVED_PAUSE);
                    self.score.stop(ctx);
                }
            }
        }
        let mut lines = lines.into_iter();
        if let Some(first) = lines.next() {
            // In the lesson the prompts come first: the kitchen waits.
            if self.lesson {
                ctx.say_queued(first);
            } else {
                ctx.say(first);
            }
        }
        for line in lines {
            ctx.say_queued(line);
        }
        self.sync_loops(ctx);
    }

    /// The loop a station should be playing, and how loud.
    fn desired(&self, station: Station) -> Option<(&'static str, f32)> {
        if self.phase == Phase::Ready {
            return None;
        }
        let i = self.meal.active_at(station)?;
        let d = &self.meal.dishes[i];
        let (key, volume) = match (d.started(), d.stage) {
            (false, Stage::Due) => (sound::WAITING, 0.8),
            (false, Stage::Burning) => (sound::WAITING_LATE, 1.0),
            (false, _) => return None,
            (true, Stage::Cooking) => (sound::cooking(station)?, 0.5),
            (true, Stage::Due) => (sound::due(station), 1.0),
            (true, Stage::Burning) => (sound::burning(station), 1.0),
            _ => return None,
        };
        let focus = if station == self.focus { 1.0 } else { 0.6 };
        Some((key, volume * focus))
    }

    /// Brings every station's loop in line with the meal.
    fn sync_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for station in Station::ALL {
            let want = self.desired(station);
            let slot = &mut self.loops[station.index()];
            let key = want.map(|w| w.0);
            if slot.1 != key {
                if slot.1.is_some() {
                    ctx.audio.stop(slot.0, 150);
                }
                *slot = (Sound::NONE, None, 0.0);
                if let Some((key, volume)) = want {
                    let s = ctx
                        .audio
                        .start_loop(key, Some(sound::place(station)), Bus::EFFECTS);
                    ctx.audio.set_volume(s, volume);
                    *slot = (s, Some(key), volume);
                }
            } else if let Some((_, volume)) = want {
                if (slot.2 - volume).abs() > 0.01 {
                    ctx.audio.set_volume(slot.0, volume);
                    slot.2 = volume;
                }
            }
        }
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for slot in self.loops.iter_mut() {
            if slot.1.is_some() {
                ctx.audio.stop(slot.0, 200);
            }
            *slot = (Sound::NONE, None, 0.0);
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        let outcome = self.meal.outcome();
        if let Mode::Challenge(run) = &self.mode {
            challenge::finish(ctx, run.clone(), outcome, self.by_ear, self.clock.pace());
            return;
        }
        let results = results::score_paced(
            self.mode.clone(),
            outcome,
            self.by_ear,
            self.hints,
            self.clock.pace(),
            ctx,
        );
        ctx.replace(results);
    }
}

fn join(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        n => format!("{}, and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

/// Keys a player presses for their screen reader or as a modifier.
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

fn station_key(event: &InputEvent) -> Option<Station> {
    let n = match event.pressed()? {
        Key::Num1 | Key::Kp1 => 1,
        Key::Num2 | Key::Kp2 => 2,
        Key::Num3 | Key::Kp3 => 3,
        Key::Num4 | Key::Kp4 => 4,
        Key::Num5 | Key::Kp5 => 5,
        _ => return None,
    };
    Station::from_number(n)
}

impl State<Parlor> for Kitchen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "harvest_feast_kitchen"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let progress = Progress::load(ctx);
        self.callouts = progress.callouts;
        progress::migrate_pace(ctx);
        self.clock = RunClock::new(self.carried.unwrap_or_else(|| progress::pace(ctx)));
        if self.plan.as_ref().is_some_and(ChallengePlan::quiet) {
            self.callouts = false;
        }
        self.by_ear = !self.callouts;
        sound::kitchen(ctx);
        let mood = self.mood();
        self.score.start(ctx, &sound::SCORE, mood);
        if self.plan.is_some() {
            ctx.audio.play(sound::TWIST, Bus::UI);
        }
        if matches!(self.mode, Mode::Thanksgiving { .. }) {
            ctx.audio.play(sound::FAMILY, Bus::UI);
        }
        // In the lesson, the lesson's own intro and prompt say it.
        if !self.lesson {
            let intro = self.intro(ctx);
            ctx.say(intro);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        self.score.stop(ctx);
    }

    /// Back from pause or help: the kitchen picks up where it was.
    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        // Settings, from the pause menu, may have changed the call-outs.
        // Switching them on ends the by-ear bonus for this meal; switching
        // them off starts it only from the next one.
        if !self.plan.as_ref().is_some_and(ChallengePlan::quiet) {
            self.callouts = Progress::load(ctx).callouts;
            if self.callouts {
                self.by_ear = false;
            }
        }
        sound::kitchen(ctx);
        let line = match self.phase {
            Phase::Ready => self.intro(ctx),
            _ => format!(
                "{}. You are at the {}. {}",
                self.mode.title(),
                self.focus.name(),
                self.prompt(ctx)
            ),
        };
        ctx.say(line);
        self.sync_loops(ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        // A controller's X, Y and shoulder buttons stand for no key, so the
        // action is read before giving up on a press.
        let action = ctx.action_for(event);
        if event.is_repeat() || (action.is_none() && event.pressed().is_none()) {
            return;
        }
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            let leave = if matches!(self.mode, Mode::Challenge(_)) {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        // Modifiers and screen-reader keys (Control silences speech) are
        // not presses of the game's.
        if action.is_none() && event.pressed().is_some_and(is_quiet_key) {
            return;
        }
        if let Phase::Served(_) = self.phase {
            ctx.say("Everything is served. The results are coming.");
            return;
        }
        if action.is_none() {
            if let Some(station) = station_key(event) {
                self.jump(ctx, station);
                return;
            }
        }
        match action {
            Some(ParlorAction::Primary) => match self.phase {
                Phase::Ready => self.begin(ctx),
                // Lesson speech holds the meal clock; ignore Space so early
                // presses do not stack fumbles over the prompt.
                _ if self.hold > 0.0 => {}
                _ => self.act(ctx),
            },
            Some(ParlorAction::Left) => self.step_focus(ctx, -1),
            Some(ParlorAction::Right) => self.step_focus(ctx, 1),
            Some(ParlorAction::Up) => {
                let line = rules::detail(&self.meal, self.focus);
                ctx.say(line);
            }
            Some(ParlorAction::Down) => {
                let card = self.recipe_card();
                ctx.say(card);
            }
            Some(ParlorAction::Secondary) => self.listen(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status(ctx);
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                let p = self.prompt(ctx);
                ctx.say(p);
            }
            Some(ParlorAction::Hint) => self.hint(ctx),
            None => match event.pressed() {
                Some(Key::Return | Key::KpEnter) if self.phase == Phase::Ready => self.begin(ctx),
                Some(Key::Return | Key::KpEnter) => ctx.say(format!(
                    "{} acts at the station in front of you.",
                    sentence_start(&Self::primary(ctx))
                )),
                _ => {}
            },
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        match self.phase {
            Phase::Ready => {}
            Phase::Cooking if self.hold > 0.0 => self.hold -= dt,
            Phase::Cooking => {
                // Every dish's clock holds while a line is being spoken,
                // and runs at the meal's pace.
                let game_dt = self.clock.game_dt(ctx, dt);
                let events = self.meal.update(game_dt);
                if !events.is_empty() {
                    self.voice(ctx, events);
                }
                if self.phase == Phase::Cooking {
                    let mood = self.mood();
                    self.score.follow(ctx, mood, dt);
                }
            }
            Phase::Served(left) => {
                let left = left - dt;
                if left <= 0.0 {
                    self.finish(ctx);
                } else {
                    self.phase = Phase::Served(left);
                }
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let mut out = vec![self.mode.title(), format!("{:?}", self.phase)];
        for s in self.meal.stations() {
            let marker = if s == self.focus { "> " } else { "  " };
            out.push(format!(
                "{marker}{}: {}",
                s.title(),
                rules::describe_station(&self.meal, s)
            ));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let p = sentence_start(&Self::primary(ctx));
        let left = ctx.control_name(ParlorAction::Left);
        let right = ctx.control_name(ParlorAction::Right);
        let up = ctx.control_name(ParlorAction::Up);
        let down = ctx.control_name(ParlorAction::Down);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let r = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        let stations: Vec<String> = self
            .meal
            .stations()
            .into_iter()
            .map(|s| format!("{} {}", s.number(), s.name()))
            .collect();
        let rule = self
            .plan
            .as_ref()
            .map(|p| format!(" {}", p.reminder()))
            .unwrap_or_default();
        format!(
            "{}.{rule} Stations this meal: {}. {left} and {right} move between them, or press a station's number. {p} starts, stirs, flips, bastes or pulls out the dish in front of you: wait for its sound to change, then act at once. {up} describes the dish in front of you, {down} reads the recipe card, {t} explains what you are hearing. {s} says what needs you now, again for every station, {r} repeats the last call, {h} says what needs you most. Escape pauses.",
            self.mode.title(),
            join(&stations),
            left = sentence_start(&left),
            up = sentence_start(&up),
        )
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_kitchen;
