//! An outing to the pools, by ear.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::{
    Challenge, Challenges, Mood, Pages, Parlor, ParlorAction, Period, Records, Rng, Score,
    ScoreResult,
};

use crate::rules::{
    fresh_rng, list_words, minutes_words, Ev, Event, Lean, Mode, Modifier, Outing, Place, Plan,
    Species, Weather,
};
use crate::save::Aquarium;
use crate::{sound, AquariumScreen, CHALLENGES_NAME, HELP, TITLE_NAME};

/// The outing screen's `State::name`.
pub const OUTING_NAME: &str = "tide_pools_outing";

/// What the shore's score hears: it builds as the buckets fill and the day
/// goes on, and thins while you lean in close to listen. The tide's last
/// minute hurries it; once the waves reach the rocks, a low unease joins.
pub fn mood(o: &Outing) -> Mood {
    if o.finished {
        return Mood::new(0.0);
    }
    let found = o.gathered.len() as f32 / o.total().max(1) as f32;
    let mut intensity = 0.15 + 0.55 * found + 0.3 * o.tide_in() as f32;
    if o.lean == Lean::In {
        intensity *= 0.4;
    }
    let timed = o.plan.mode != Mode::Practice;
    Mood::new(intensity)
        .with_flag(HURRY, timed && o.tide_left <= 60.0)
        .with_flag(DANGER, timed && o.tide_left <= 20.0)
}

/// Builds an ordinary trip or the practice pools.
pub fn outing_for(mode: Mode) -> Outing {
    let mut rng = fresh_rng();
    let plan = match mode {
        Mode::Beach(place) => Plan::beach(place, &mut rng),
        Mode::Practice => Plan::practice(),
        Mode::Challenge(period) => Plan::challenge(period, &mut rng),
        Mode::Event(event, place) => Plan::event(event, place, &mut rng),
    };
    Outing::new(plan, rng)
}

/// The challenge of `period` today, as an outing everyone shares.
pub fn challenge_outing(challenge: &Challenge) -> Outing {
    let mut rng = challenge.rng();
    let plan = Plan::challenge(challenge.period, &mut rng);
    Outing::new(plan, rng)
}

/// A period's challenge, by name.
pub fn challenge_name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Tonight's three species",
        Period::Weekly => "This week's rare find",
        Period::Monthly => "The Great Low Tide",
    }
}

/// What a period's challenge asks, in a few words.
pub fn challenge_tag(period: Period, plan: &Plan) -> String {
    match period {
        Period::Daily => format!(
            "{}. {}",
            list_words(&plan.targets.iter().map(|s| s.name()).collect::<Vec<_>>()),
            plan.modifier.name()
        ),
        Period::Weekly => match plan.modifier {
            Modifier::RareHunt(s) => format!("the {}, on the Far Reef", s.name()),
            _ => "a rare creature, on the Far Reef".to_string(),
        },
        Period::Monthly => "gather 8 kinds on the Far Reef".to_string(),
    }
}

fn rare_line(plan: &Plan) -> String {
    if plan.any_rare {
        return " Every rare creature can turn up today.".to_string();
    }
    let rares = plan.conditions.rares();
    if rares.is_empty() || plan.rare_chance <= 0.0 {
        return String::new();
    }
    let names: Vec<&str> = rares.iter().map(|s| s.name()).collect();
    format!(
        " On a day like this, you might find a {}.",
        list_words(&names).replace(" and ", " or a ")
    )
}

/// The pages before an ordinary trip, then the trip.
pub fn start(ctx: &mut Ctx<Parlor>, mode: Mode) {
    ctx.push(trip_briefing(mode));
}

/// The pages before an ordinary trip, which then start it. Escape goes
/// back without setting off.
pub fn trip_briefing(mode: Mode) -> Pages {
    let outing = outing_for(mode);
    let plan = &outing.plan;
    let first = match mode {
        Mode::Practice => "The practice pools: three sheltered pools close to home, the same every time, and no tide to hurry you. The first holds an anemone, the second a shy shore crab, the third a periwinkle shell and a starfish. Gather all four to finish.".to_string(),
        Mode::Event(event, place) => format!(
            "{}. {} {} {}{}",
            sentence_start(place.name()),
            event_line(event),
            plan.conditions.describe(),
            plan.summary(),
            rare_line(plan)
        ),
        _ => format!(
            "{}. {} {}{}",
            sentence_start(plan.place.name()),
            plan.conditions.describe(),
            plan.summary(),
            rare_line(plan)
        ),
    };
    let screen = OutingScreen::new(outing, None);
    briefing_then(vec![first], screen)
}

/// What an event changes, in a sentence, for its briefing and arrival.
pub fn event_line(event: Event) -> &'static str {
    match event {
        Event::KingTide => "King tide. The sea is far out today, so two extra pools open at each end, and curlews call from the wet sand.",
        Event::Moonlit => "The full moon lights the pools. The gulls are asleep, and a night heron calls along the shore.",
    }
}

/// Briefing pages that swap themselves for the outing.
fn briefing_then(pages: Vec<String>, screen: OutingScreen) -> Pages {
    let mut slot = Some(screen);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(screen) = slot.take() {
            ctx.replace(screen);
        }
    })
}

/// The current challenge of `period`: its rule first, then the trip.
/// Escape on these pages goes back without starting.
pub fn challenge_briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let challenge = Challenge::current(ctx, crate::ID, period);
    let outing = challenge_outing(&challenge);
    let plan = &outing.plan;
    let trip = format!(
        "{} {} pools, about {}.",
        plan.conditions.describe(),
        plan.pools,
        minutes_words(plan.tide_secs)
    );
    let page = match period {
        Period::Daily => {
            let names: Vec<String> = plan
                .targets
                .iter()
                .map(|s| format!("a {}", s.name()))
                .collect();
            format!(
                "Tonight's three species: find {} on Rocky Point, worth 100 points more each. {} {trip}",
                list_words(&names.iter().map(String::as_str).collect::<Vec<_>>()),
                plan.modifier.describe()
            )
        }
        Period::Weekly => format!(
            "This week's rare find, on the Far Reef: {} {trip}",
            plan.modifier.describe()
        ),
        Period::Monthly => format!("{} {trip}", plan.modifier.describe()),
    };
    let pages = vec![page];
    let screen = OutingScreen::new(outing, Some(challenge));
    briefing_then(pages, screen)
}

/// The results menu after a trip: the same trip again, the results
/// again, the aquarium, and back to where the trip began.
pub fn after_trip(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let results = match mode {
        Mode::Challenge(period) => Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME))
            .again(
                labels::challenge_again(period),
                "The same shore, straight away. Only your best score is kept.",
                move |ctx| {
                    let challenge = Challenge::current(ctx, crate::ID, period);
                    let outing = challenge_outing(&challenge);
                    ctx.replace(OutingScreen::new(outing, Some(challenge)))
                },
            ),
        _ => Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
            labels::PLAY_AGAIN,
            match mode {
                Mode::Practice => "The practice pools again, with no tide.",
                _ => "The same shore, with new weather and new creatures.",
            },
            move |ctx| {
                // An event closed since the trip began: no event trip, and
                // no event board, off its window.
                if let Mode::Event(event, _) = mode {
                    if !event.is_open(parlor_core::challenges::today(ctx)) {
                        ctx.pop_to_with(TITLE_NAME, event.ended());
                        return;
                    }
                }
                ctx.replace(trip_briefing(mode))
            },
        ),
    };
    results.extra(
        "Your aquarium",
        "Visit everything you have gathered, and listen to it.",
        |ctx| ctx.push(AquariumScreen::new()),
    )
}

/// The beach, heard from the rocks.
pub struct OutingScreen {
    pub outing: Outing,
    challenge: Option<Challenge>,
    waves: Option<Sound>,
    pool: Option<Sound>,
    rain: Option<Sound>,
    atmos: Rng,
    gull_in: f64,
    gust_in: f64,
    fog_in: f64,
    /// A curlew's next call, on a king tide.
    curlew_in: f64,
    /// A night heron's next call, on the moonlit pools.
    heron_in: f64,
    last_prompt: String,
    taught: HashSet<&'static str>,
    named: HashSet<Species>,
    owned: Vec<Species>,
    step_left: bool,
    started: bool,
    done: bool,
    /// Run under the tutorial: finds are practice, and nothing is kept.
    lesson: bool,
    /// The shore's music, following the trip.
    pub score: Score,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl OutingScreen {
    pub fn new(outing: Outing, challenge: Option<Challenge>) -> OutingScreen {
        let atmos = Rng::seeded(outing.plan.pools as u64 ^ 0x5EA5);
        OutingScreen {
            status_key: Default::default(),
            outing,
            challenge,
            waves: None,
            pool: None,
            rain: None,
            atmos,
            gull_in: 6.0,
            gust_in: 14.0,
            fog_in: 20.0,
            curlew_in: 9.0,
            heron_in: 9.0,
            last_prompt: String::new(),
            taught: HashSet::new(),
            named: HashSet::new(),
            owned: Vec::new(),
            step_left: false,
            started: false,
            done: false,
            lesson: false,
            score: Score::new(),
        }
    }

    /// The practice pools under the tutorial: a find is named, with no
    /// points and nothing for the aquarium, since the lesson keeps nothing.
    pub fn in_lesson(mut self) -> OutingScreen {
        self.lesson = true;
        self
    }

    /// True the first time a cue is met this trip.
    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    fn prompt(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        self.last_prompt = text.clone();
        ctx.say(text);
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in [self.waves.take(), self.pool.take(), self.rain.take()]
            .into_iter()
            .flatten()
        {
            ctx.audio.stop(s, 600);
        }
        self.score.stop(ctx);
        ctx.audio.set_bed(None);
    }

    /// The calendar event this trip is for, if any.
    fn event(&self) -> Option<Event> {
        match self.outing.plan.mode {
            Mode::Event(event, _) => Some(event),
            _ => None,
        }
    }

    /// On a king tide the sea starts much further out and quieter, and
    /// still reaches the rocks at the end.
    pub fn waves_pos(&self) -> Vec3 {
        let t = self.outing.tide_in() as f32;
        let far = if self.event() == Some(Event::KingTide) {
            18.0
        } else {
            10.0
        };
        Vec3::new(0.0, far - (far - 3.5) * t, 0.0)
    }

    pub fn waves_volume(&self) -> f32 {
        let t = self.outing.tide_in() as f32;
        if self.event() == Some(Event::KingTide) {
            0.15 + 0.65 * t
        } else {
            0.3 + 0.5 * t
        }
    }

    fn place_pool(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.pool {
            ctx.audio
                .set_position(s, Some(sound::pool_at(0, self.outing.lean)));
        }
    }

    /// Everything S says.
    /// The status key, in layers. First: which pool, how close, and the
    /// tide. A second press: what you have gathered and still seek. A
    /// third: the shore, its weather and any event.
    pub fn status(&self) -> Vec<String> {
        let o = &self.outing;
        let mut first = vec![format!(
            "Pool {} of {}, {}.",
            o.at + 1,
            o.pools.len(),
            o.lean.words()
        )];
        first.push(if o.plan.mode == Mode::Practice {
            "No tide to hurry you here.".to_string()
        } else {
            format!("The tide returns in about {}.", minutes_words(o.tide_left))
        });
        let n = o.gathered.len();
        let mut more = vec![match n {
            0 => "Nothing gathered yet.".to_string(),
            1 => "1 creature gathered.".to_string(),
            n => format!("{n} creatures gathered."),
        }];
        more.push(format!("Score {}.", o.points));
        if !o.plan.targets.is_empty() {
            let parts: Vec<String> = o
                .plan
                .targets
                .iter()
                .map(|t| {
                    if o.gathered.contains(t) {
                        format!("the {}, found", t.name())
                    } else {
                        format!("the {}, still to find", t.name())
                    }
                })
                .collect();
            more.push(format!("Tonight's three: {}.", parts.join("; ")));
        }
        if let Modifier::RareHunt(s) = o.plan.modifier {
            if o.gathered.contains(&s) {
                more.push(format!("You have this week's {}.", s.name()));
            } else {
                more.push(format!("The {} is still out there.", s.name()));
            }
        }
        if o.plan.modifier == Modifier::GreatLowTide {
            more.push(format!("{} of 8 different kinds so far.", o.kinds().len()));
        }
        let mut shore = vec![format!(
            "{}. {}",
            sentence_start(o.plan.place.name()),
            o.plan.conditions.describe()
        )];
        match self.event() {
            Some(Event::KingTide) => shore.push(format!(
                "King tide: pools 1, 2, {} and {} are extra.",
                o.pools.len() - 1,
                o.pools.len()
            )),
            Some(Event::Moonlit) => shore.push("Moonlit pools.".to_string()),
            None => {}
        }
        vec![
            parlor_core::status::sentences(first),
            parlor_core::status::sentences(more),
            parlor_core::status::sentences(shore),
        ]
    }

    fn hint(&mut self, ctx: &Ctx<Parlor>) -> String {
        self.outing.note_hint();
        let o = &self.outing;
        let up = ctx.control_name(ParlorAction::Up);
        let p = ctx.control_name(ParlorAction::Primary);
        if o.pool().left() > 0 {
            if o.lean != Lean::In {
                return format!("Something lives here. Lean in with {up}.");
            }
            if o.hiding_here() {
                return "Something shy is hiding. Keep still for the two rising notes.".to_string();
            }
            return format!("Something is out. Press {p} right after its sound.");
        }
        match o.nearest() {
            Some(off) => {
                let side = if off < 0 { "left" } else { "right" };
                let n = off.unsigned_abs();
                let pools = if n == 1 { "pool" } else { "pools" };
                format!("Next find {n} {pools} to your {side}.")
            }
            None => "Nothing left to find.".to_string(),
        }
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        let p = ctx.control_name(ParlorAction::Primary);
        let up = ctx.control_name(ParlorAction::Up);
        for ev in evs {
            match ev {
                Ev::Arrive { pool, of, cleared } => {
                    let key = if self.step_left {
                        sound::STEP_A
                    } else {
                        sound::STEP_B
                    };
                    self.step_left = !self.step_left;
                    sound::play(ctx, key);
                    self.place_pool(ctx);
                    let mut text = format!("Pool {}", pool + 1);
                    if pool + 1 == of {
                        text.push_str(", the last");
                    }
                    text.push('.');
                    if cleared {
                        text.push_str(" Empty.");
                    }
                    self.prompt(ctx, text);
                }
                Ev::Edge { left } => {
                    sound::play(ctx, sound::EDGE);
                    let side = if left { "left" } else { "right" };
                    self.prompt(ctx, format!("No more pools to the {side}."));
                }
                Ev::Lean { lean, changed } => {
                    if !changed {
                        sound::play(ctx, sound::REACH);
                        ctx.say(match lean {
                            Lean::In => "Already leaning in.",
                            _ => "Already standing back.",
                        });
                        continue;
                    }
                    let key = if lean == Lean::In {
                        sound::LEAN_IN
                    } else {
                        sound::LEAN_BACK
                    };
                    sound::play(ctx, key);
                    self.place_pool(ctx);
                    let mut text = sentence_start(lean.words());
                    text.push('.');
                    match lean {
                        Lean::In if self.first("lean in") => text.push_str(&format!(
                            " Only this pool now. {} reaches in.",
                            sentence_start(&p)
                        )),
                        Lean::Back if self.first("lean back") => {
                            text.push_str(" Two pools either side, fainter.")
                        }
                        _ => {}
                    }
                    self.prompt(ctx, text);
                }
                Ev::OnlyWater => {
                    ctx.say_queued("Only water and weed.");
                }
                Ev::Call {
                    species,
                    offset,
                    lean,
                } => {
                    sound::play_at(ctx, sound::creature(species), sound::pool_at(offset, lean));
                    if offset == 0 {
                        if self.named.insert(species) {
                            let text = format!(
                                "{}: a {}.",
                                sentence_start(species.sound_words()),
                                species.name()
                            );
                            ctx.say_queued(text);
                        }
                    } else if !self.lesson && self.first("neighbour") {
                        // The tutorial's own step teaches the neighbours,
                        // so only a real trip says this, once ever.
                        let key = format!("{}/told_neighbours", crate::ID);
                        if !Records::load::<bool>(ctx, &key) {
                            Records::store(ctx, &key, &true);
                            ctx.say_queued("Side sounds are the next pools.");
                        }
                    }
                }
                Ev::Hide { count } => {
                    sound::play_at(ctx, sound::HIDE, sound::pool_at(0, Lean::In));
                    if self.first("hide") {
                        ctx.say_queued("Something shy hid. Keep still until it creeps out.");
                    } else if count > 1 {
                        ctx.say_queued("Some things hide.");
                    } else {
                        ctx.say_queued("Something hides.");
                    }
                }
                Ev::Emerge { .. } => {
                    sound::play_at(ctx, sound::EMERGE, sound::pool_at(0, self.outing.lean));
                    if self.first("emerge") {
                        ctx.say_queued(format!(
                            "Two rising notes: it crept out. Press {p} after its sound."
                        ));
                    }
                }
                Ev::TooSoon => {
                    sound::play(ctx, sound::HIDE);
                    self.prompt(ctx, "Too soon. It hid again.".to_string());
                }
                Ev::NotYet => {
                    sound::play(ctx, sound::EMPTY);
                    self.prompt(ctx, format!("Only water. Press {p} right after a sound."));
                }
                Ev::EmptyPool => {
                    sound::play(ctx, sound::EMPTY);
                    self.prompt(ctx, "This pool is empty.".to_string());
                }
                Ev::OutOfReach => {
                    sound::play(ctx, sound::REACH);
                    self.prompt(ctx, format!("Out of reach. Lean in with {up}."));
                }
                Ev::Gathered {
                    species,
                    points,
                    target,
                } => {
                    sound::play(ctx, sound::GATHER);
                    sound::play_at(ctx, sound::creature(species), Vec3::new(0.0, 0.4, 0.0));
                    if self.lesson {
                        let text = sentence_start(&format!("a {}.", species.name()));
                        self.prompt(ctx, text);
                        continue;
                    }
                    let mut text = format!(
                        "{}. Plus {points}.",
                        sentence_start(&format!("a {}", species.name()))
                    );
                    if species.rare() {
                        sound::play(ctx, sound::RARE);
                        text.push_str(" A rare find.");
                    }
                    if target {
                        text.push_str(" One of tonight's three.");
                    }
                    if let Modifier::RareHunt(s) = self.outing.plan.modifier {
                        if s == species
                            && self.outing.gathered.iter().filter(|g| **g == s).count() == 1
                        {
                            text.push_str(" This week's rare find.");
                        }
                    }
                    if !self.owned.contains(&species) {
                        self.owned.push(species);
                        sound::play(ctx, sound::NEW);
                        text.push_str(" New for your aquarium.");
                    }
                    let o = &self.outing;
                    if !o.finished && o.pools[o.at].left() == 0 {
                        text.push_str(" This pool is empty.");
                    }
                    self.prompt(ctx, text);
                }
                Ev::TideTurned => {
                    sound::play(ctx, sound::TIDE_TURN);
                    ctx.say_queued("The tide has turned.");
                }
                Ev::TideWarning(secs) => {
                    sound::play(ctx, sound::TIDE_WARN);
                    if secs >= 60 {
                        ctx.say_queued("Tide in about a minute.");
                    } else {
                        ctx.say_queued("Tide in 20 seconds.");
                    }
                }
                Ev::TideIn => {
                    sound::play(ctx, sound::WAVE_CRASH);
                    self.finish(ctx, true);
                }
                Ev::AllGathered => self.finish(ctx, false),
            }
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>, tide: bool) {
        if self.done {
            return;
        }
        self.done = true;
        self.stop_loops(ctx);
        let o = &self.outing;
        let score = o.score();
        let mut aquarium = Aquarium::load(ctx);
        let kinds_before = aquarium.kinds();
        let found = format!(
            "at {} on a {}",
            o.plan.place.name(),
            o.plan.conditions.short()
        );
        let new = aquarium.add(&o.gathered, &found);
        aquarium.outings += 1;
        aquarium.store(ctx);
        let mut pages = Vec::new();
        // A trip ended by its last find lets that find's line finish: the
        // page waits behind it instead of saying it again.
        let opening = if tide {
            "Tide's in."
        } else if o.plan.mode == Mode::Practice {
            "Practice pools done."
        } else {
            "Everything gathered before the tide."
        };
        let mut counts: Vec<(Species, u32)> = Vec::new();
        for s in &o.gathered {
            match counts.iter_mut().find(|(k, _)| k == s) {
                Some((_, n)) => *n += 1,
                None => counts.push((*s, 1)),
            }
        }
        let haul = if counts.is_empty() {
            " Nothing gathered this time.".to_string()
        } else {
            let words: Vec<String> = counts.iter().map(|(s, n)| s.count_words(*n)).collect();
            let words: Vec<&str> = words.iter().map(String::as_str).collect();
            format!(" You gathered {}.", list_words(&words))
        };
        pages.push(format!("{opening}{haul}"));
        let bonus = o.bonus();
        let mut score_line = if bonus > 0 {
            format!(
                "Score {score}: {} for your finds and {bonus} for the challenge.",
                o.points
            )
        } else {
            format!("Score {score}.")
        };
        let mut won = !new.is_empty();
        match (o.plan.mode, self.challenge.clone()) {
            (Mode::Beach(place), _) => {
                match Records::record(ctx, crate::ID, place.board(), score) {
                    ScoreResult::First => {
                        score_line.push_str(&format!(" Your first score at {}.", place.name()))
                    }
                    ScoreResult::NewBest { previous } => score_line.push_str(&format!(
                        " A new best at {}, beating {previous}.",
                        place.name()
                    )),
                    ScoreResult::Short { best } => {
                        score_line.push_str(&format!(" Your best at {} is {best}.", place.name()))
                    }
                }
                pages.push(score_line);
            }
            (Mode::Event(event, _), _) => {
                match Records::record(ctx, crate::ID, event.board(), score) {
                    ScoreResult::First => {
                        score_line.push_str(&format!(" Your first score on {}.", event.the()))
                    }
                    ScoreResult::NewBest { previous } => score_line.push_str(&format!(
                        " A new best on {}, beating {previous}.",
                        event.the()
                    )),
                    ScoreResult::Short { best } => {
                        score_line.push_str(&format!(" Your best on {} is {best}.", event.the()))
                    }
                }
                pages.push(score_line);
            }
            (Mode::Challenge(period), Some(challenge)) => {
                let met = o.goal_met();
                won |= met;
                let goal = match o.plan.modifier {
                    Modifier::RareHunt(s) if met => format!("You found this week's {}.", s.name()),
                    Modifier::RareHunt(s) => format!("The {} stayed hidden this time.", s.name()),
                    Modifier::GreatLowTide if met => format!(
                        "You found {} different kinds, enough for the Great Low Tide.",
                        o.kinds().len()
                    ),
                    Modifier::GreatLowTide => format!(
                        "You found {} different kinds. 8 meets the goal.",
                        o.kinds().len()
                    ),
                    _ if met => "You found all of tonight's three.".to_string(),
                    _ => {
                        let found = o
                            .plan
                            .targets
                            .iter()
                            .filter(|t| o.gathered.contains(t))
                            .count();
                        format!("You found {found} of tonight's three.")
                    }
                };
                let result = Challenges::submit(ctx, &challenge, score);
                let best = match result {
                    ScoreResult::First => "Your first score on this one.".to_string(),
                    ScoreResult::NewBest { previous } => {
                        format!("A new best, beating {previous}.")
                    }
                    ScoreResult::Short { best } => format!("Your best is {best}."),
                };
                let streak = Challenges::streak(ctx, crate::ID, period);
                let streak = match streak {
                    0 | 1 => format!("The first {} of a streak.", period.unit()),
                    n => format!("A {n} {} streak.", period.unit()),
                };
                pages.push(format!(
                    "{}: {goal} {score_line} {best} {streak} {}",
                    challenge.title(),
                    challenge.resets()
                ));
            }
            _ => pages.push(score_line),
        }
        let mut tank = if new.is_empty() {
            format!(
                "Your aquarium holds {} of {} kinds.",
                aquarium.kinds(),
                Species::ALL.len()
            )
        } else {
            let names: Vec<&str> = new.iter().map(|s| s.name()).collect();
            format!(
                "New in your aquarium: {}, {} of {}.",
                list_words(&names),
                aquarium.kinds(),
                Species::ALL.len()
            )
        };
        for place in Place::ALL {
            let need = place.unlock_kinds();
            if kinds_before < need && aquarium.kinds() >= need {
                tank.push_str(&format!(
                    " {} is now open on the title menu.",
                    sentence_start(place.name())
                ));
            }
        }
        pages.push(tank);
        // One page: the haul, the score and the aquarium read together.
        let pages = vec![pages.join(" ")];
        ctx.audio
            .play_music(Some(if won { sound::WIN } else { sound::HOME }), false);
        let summary = match (o.plan.mode, &self.challenge) {
            (Mode::Challenge(_), Some(c)) => format!("{} result: score {score}", c.title()),
            (Mode::Practice, _) => format!("Practice pools done. Score {score}"),
            _ => format!("Home from {}. Score {score}", o.plan.place.name()),
        };
        let results = after_trip(o.plan.mode, summary, pages.clone());
        let mut results = Some(results);
        let pages = if tide {
            Pages::new(pages)
        } else {
            Pages::new(pages).after_speech()
        };
        ctx.replace(pages.then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }

    fn atmosphere(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let weather = self.outing.plan.conditions.weather;
        let event = self.event();
        // Event birds keep clear of the pool you are leaning in to hear.
        let listening = self.outing.lean == Lean::In;
        self.gull_in -= dt;
        if event == Some(Event::Moonlit) {
            // Gulls sleep at night: a night heron calls instead.
            self.gull_in = 30.0;
            self.heron_in -= dt;
            if self.heron_in <= 0.0 && listening {
                self.heron_in = 3.0;
            } else if self.heron_in <= 0.0 {
                self.heron_in = 18.0 + self.atmos.unit() * 12.0;
                let x = (self.atmos.unit() as f32 - 0.5) * 24.0;
                let s = ctx
                    .audio
                    .play_at(sound::HERON, Vec3::new(x, 4.0, 0.0), Bus::AMBIENCE);
                ctx.audio.set_volume(s, 0.5);
                if self.first("heron") {
                    ctx.say_queued("A night heron. Just the night.");
                }
            }
        }
        if event == Some(Event::KingTide) {
            self.curlew_in -= dt;
            if self.curlew_in <= 0.0 && listening {
                self.curlew_in = 3.0;
            } else if self.curlew_in <= 0.0 {
                self.curlew_in = 16.0 + self.atmos.unit() * 12.0;
                let x = (self.atmos.unit() as f32 - 0.5) * 20.0;
                let s = ctx
                    .audio
                    .play_at(sound::CURLEW, Vec3::new(x, 12.0, 0.0), Bus::AMBIENCE);
                ctx.audio.set_volume(s, 0.5);
                if self.first("curlew") {
                    ctx.say_queued("A curlew. Just the seaside.");
                }
            }
        }
        if self.gull_in <= 0.0 {
            self.gull_in = match weather {
                Weather::Sunny => 9.0,
                Weather::Drizzle => 30.0,
                _ => 16.0,
            } + self.atmos.unit() * 12.0;
            let x = (self.atmos.unit() as f32 - 0.5) * 16.0;
            let s = ctx
                .audio
                .play_at(sound::GULL, Vec3::new(x, 5.0, 6.0), Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.5);
        }
        self.gust_in -= dt;
        if self.gust_in <= 0.0 {
            self.gust_in = if weather == Weather::Breezy {
                7.0
            } else {
                22.0
            } + self.atmos.unit() * 10.0;
            let x = if self.atmos.chance(0.5) { -6.0 } else { 6.0 };
            let s = ctx
                .audio
                .play_at(sound::GUST, Vec3::new(x, 2.0, 1.0), Bus::AMBIENCE);
            ctx.audio.set_volume(s, 0.5);
        }
        if weather == Weather::Foggy {
            self.fog_in -= dt;
            if self.fog_in <= 0.0 {
                self.fog_in = 30.0 + self.atmos.unit() * 20.0;
                let s =
                    ctx.audio
                        .play_at(sound::FOGHORN, Vec3::new(-20.0, 30.0, 0.0), Bus::AMBIENCE);
                ctx.audio.set_volume(s, 0.6);
            }
        }
        if let Some(w) = self.waves {
            let pos = self.waves_pos();
            let vol = self.waves_volume();
            ctx.audio.set_position(w, Some(pos));
            ctx.audio.set_volume(w, vol);
        }
    }

    fn controls_line(ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} and {} walk between pools, {} leans in, {} steps back, {} reaches in for what you just heard. {} gives the pool and the tide, again for what you have gathered, a third time for the shore. {} gives a hint, F1 helps, Escape pauses.",
            sentence_start(&ctx.control_name(ParlorAction::Left)),
            ctx.control_name(ParlorAction::Right),
            ctx.control_name(ParlorAction::Up),
            ctx.control_name(ParlorAction::Down),
            ctx.control_name(ParlorAction::Primary),
            sentence_start(&ctx.control_name(ParlorAction::Status)),
            ctx.control_name(ParlorAction::Hint),
        )
    }
}

impl State<Parlor> for OutingScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        OUTING_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.started = true;
        self.owned = Aquarium::load(ctx).species();
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(Some(sound::SHORE));
        let waves = ctx
            .audio
            .start_loop(sound::WAVES, Some(self.waves_pos()), Bus::AMBIENCE);
        ctx.audio.set_volume(waves, self.waves_volume());
        self.waves = Some(waves);
        let pool = ctx.audio.start_loop(
            sound::POOL,
            Some(sound::pool_at(0, self.outing.lean)),
            Bus::AMBIENCE,
        );
        ctx.audio.set_volume(pool, 0.5);
        self.pool = Some(pool);
        let mood = mood(&self.outing);
        self.score.start(ctx, &sound::SCORE, mood);
        if self.outing.plan.conditions.weather == Weather::Drizzle {
            let rain = ctx.audio.start_loop(sound::RAIN, None, Bus::AMBIENCE);
            ctx.audio.set_volume(rain, 0.6);
            self.rain = Some(rain);
        }
        let o = &self.outing;
        let text = format!("Pool 1 of {}, at the water's edge.", o.pools.len());
        let controls = if menus::tutorial_read(ctx, crate::ID) {
            "F1 lists the controls.".to_string()
        } else {
            Self::controls_line(ctx)
        };
        self.last_prompt = format!("{text} {controls}");
        ctx.say(text);
        if let Some(event) = self.event() {
            ctx.say_queued(event_line(event));
        }
        ctx.say_queued(controls);
    }

    /// Left before the tide, from the pause menu: what was gathered still
    /// goes home to the aquarium. The shore's best waits for a finished
    /// trip.
    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        if !self.done && !self.lesson && !self.outing.gathered.is_empty() {
            self.done = true;
            let found = format!(
                "at {} on a {}",
                self.outing.plan.place.name(),
                self.outing.plan.conditions.short()
            );
            let mut aquarium = Aquarium::load(ctx);
            aquarium.add(&self.outing.gathered, &found);
            aquarium.store(ctx);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.done {
            return;
        }
        let o = &self.outing;
        let tide = if o.plan.mode == Mode::Practice {
            String::new()
        } else {
            format!(" The tide returns in about {}.", minutes_words(o.tide_left))
        };
        ctx.say(format!(
            "Back at pool {}, {}.{tide}",
            o.at + 1,
            o.lean.words()
        ));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || self.done {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let evs = match ctx.action_for(event) {
            Some(ParlorAction::Left) => self.outing.walk(-1),
            Some(ParlorAction::Right) => self.outing.walk(1),
            Some(ParlorAction::Up) => self.outing.lean_in(),
            Some(ParlorAction::Down) => self.outing.step_back(),
            Some(ParlorAction::Primary) => self.outing.gather(),
            Some(ParlorAction::Secondary) => {
                let text = match self.outing.last_heard {
                    Some(s) => format!(
                        "That last sound was the {}: {}.{}",
                        s.name(),
                        s.sound_words(),
                        if s.shy().is_some() { " It is shy." } else { "" }
                    ),
                    None => "No sound here yet.".to_string(),
                };
                ctx.say(text);
                return;
            }
            Some(ParlorAction::Status) => {
                let layers = self.status();
                self.status_key.speak(ctx, layers);
                return;
            }
            Some(ParlorAction::Repeat) => {
                let text = self.last_prompt.clone();
                ctx.say(text);
                return;
            }
            Some(ParlorAction::Hint) => {
                let text = self.hint(ctx);
                ctx.say(text);
                return;
            }
            None => {
                if let InputEvent::KeyDown { text: Some(c), .. } = event {
                    if c.is_alphanumeric() {
                        ctx.say("Not a key here. F1 lists them.");
                    }
                }
                return;
            }
        };
        self.voice(ctx, evs);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if !self.started || self.done {
            return;
        }
        self.atmosphere(ctx, dt);
        let evs = self.outing.update(dt);
        self.voice(ctx, evs);
        if !self.done {
            let mood = mood(&self.outing);
            self.score.follow(ctx, mood, dt);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let o = &self.outing;
        let row: String = o
            .pools
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == o.at {
                    '@'
                } else if p.left() > 0 {
                    '*'
                } else {
                    '.'
                }
            })
            .collect();
        vec![
            format!(
                "{}: pool {} of {}, {}",
                o.plan.place.name(),
                o.at + 1,
                o.pools.len(),
                o.lean.words()
            ),
            format!(
                "Tide in {:.0} seconds. Gathered {}. Score {}.",
                o.tide_left,
                o.gathered.len(),
                o.points
            ),
            row,
        ]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let o = &self.outing;
        let up = ctx.control_name(ParlorAction::Up);
        let p = ctx.control_name(ParlorAction::Primary);
        let now = match o.lean {
            Lean::In if o.hiding_here() => {
                "Something shy is hiding in this pool: keep still until you hear two soft rising notes, then wait for its sound.".to_string()
            }
            Lean::In => format!(
                "You are leaning in. When a creature makes its sound in this pool, press {p} right after to gather it."
            ),
            Lean::Edge => format!(
                "You are at the water's edge, hearing this pool and the next either side. Lean in with {up} to reach in."
            ),
            Lean::Back => format!(
                "You are standing back, hearing two pools either side. {} brings you to the edge.",
                sentence_start(&up)
            ),
        };
        format!(
            "{now} {} {} names the last sound you heard here. {} repeats the last message.",
            Self::controls_line(ctx),
            sentence_start(&ctx.control_name(ParlorAction::Secondary)),
            sentence_start(&ctx.control_name(ParlorAction::Repeat)),
        )
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_outingscreen;
