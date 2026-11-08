//! The hamlet screen: walking the lane, the day's sounds and the band,
//! and the morning and supper in between.

use std::cell::RefCell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, Back, Leave, Results};
use parlor_core::{Challenge, Challenges, Pages, Parlor, ParlorAction, ScoreResult, StatusKey};

use crate::challenge::CHALLENGES_NAME;
use crate::menus::{ArrivalMenu, BargeMenu, BuildMenu, JobBoard, SeedMenu, Shared, SupperMenu};
use crate::music::{Band, Player};
use crate::rules::{At, Event, Hamlet, Kind, Mood, Phase, Place};
use crate::save::Progress;
use crate::{events, sound, HELP};

/// The title's screen name.
pub const TITLE_NAME: &str = "larkfield";
/// The hamlet's screen name.
pub const HAMLET_NAME: &str = "larkfield_hamlet";

/// What the screen waits for while something is open on top of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Waiting {
    Nothing,
    Morning,
    Supper,
    Menu,
}

/// The hamlet, while you are in it.
pub struct HamletScreen {
    pub h: Shared,
    challenge: Option<Challenge>,
    band: Band,
    status_key: StatusKey,
    river: Option<Sound>,
    bed: Option<&'static str>,
    waiting: Waiting,
    band_on: bool,
    work_sounds: bool,
    entered: bool,
    done: bool,
}

impl HamletScreen {
    pub fn new(h: Hamlet, challenge: Option<Challenge>) -> HamletScreen {
        let seed = h.seed;
        HamletScreen {
            h: Rc::new(RefCell::new(h)),
            challenge,
            band: Band::new(seed),
            status_key: StatusKey::default(),
            river: None,
            bed: None,
            waiting: Waiting::Nothing,
            band_on: true,
            work_sounds: true,
            entered: false,
            done: false,
        }
    }

    fn kind(&self) -> Kind {
        self.h.borrow().kind
    }

    fn settings(&mut self, ctx: &mut Ctx<Parlor>) {
        self.h.borrow_mut().keys = keys(ctx);
        let p = Progress::load(ctx);
        self.band_on = p.band;
        self.work_sounds = p.work_sounds;
        let mut h = self.h.borrow_mut();
        h.day_length = p.day_length;
        if h.kind == Kind::Home {
            h.pace = p.pace;
            let today = parlor_core::challenges::today(ctx);
            h.fair_day = events::barge_fair(today);
            h.music_night = events::music_night(today);
            if h.fair_day && h.phase == Phase::Morning && !h.barge {
                h.barge = true;
                h.news.push(
                    "Barge fair today: the barge visits every in-game day and pays an extra coin for each item."
                        .to_string(),
                );
            }
        }
    }

    /// Keeps your own hamlet.
    fn keep(&self, ctx: &mut Ctx<Parlor>) {
        if self.kind() == Kind::Home && !self.done {
            Progress::keep(ctx, &self.h.borrow());
        }
    }

    fn here(&self) -> Place {
        self.h.borrow().at.place()
    }

    fn sync_bed(&mut self, ctx: &mut Ctx<Parlor>) {
        let (weather, winter) = {
            let h = self.h.borrow();
            (h.weather, h.season() == crate::crops::Season::Winter)
        };
        let bed = sound::bed_for(weather, winter);
        if self.bed != Some(bed) {
            self.bed = Some(bed);
            ctx.audio.set_bed(Some(bed));
        }
        let here = self.here();
        sound::listen_at(ctx, here);
    }

    fn start_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        self.bed = None;
        self.sync_bed(ctx);
        if self.river.is_none() {
            let s = ctx.audio.start_loop(
                sound::RIVER,
                Some(sound::place_pos(Place::River)),
                Bus::EFFECTS,
            );
            ctx.audio.set_volume(s, 0.5);
            self.river = Some(s);
        }
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.river.take() {
            ctx.audio.stop(s, 300);
        }
        ctx.audio.set_bed(None);
        self.bed = None;
    }

    /// Stops every sound the screen started, for a lesson that closes.
    pub fn silence(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
        ctx.audio.play_music(None, false);
    }

    /// The morning: the news on pages, then the working day.
    fn morning(&mut self, ctx: &mut Ctx<Parlor>) {
        self.sync_bed(ctx);
        ctx.audio.play_music(None, false);
        self.band.restart();
        let (date, news) = {
            let h = self.h.borrow();
            let date = format!(
                "Day {} of {}, year {}. {}.",
                h.day_of_season(),
                h.season().name(),
                h.year(),
                crate::rules::sentence_case(h.weather.name())
            );
            (date, h.news.clone())
        };
        ctx.audio.play(sound::MORNING, Bus::EFFECTS);
        if news.is_empty() || self.kind() == Kind::Practice {
            self.start_day(ctx, Some(date));
            return;
        }
        let mut pages = Vec::new();
        let mut first = date;
        let mut rest = news.into_iter();
        if let Some(n) = rest.next() {
            first = format!("{first} {n}");
        }
        pages.push(first);
        let rest: Vec<String> = rest.collect();
        for chunk in rest.chunks(2) {
            pages.push(chunk.join(" "));
        }
        self.waiting = Waiting::Morning;
        ctx.push(Pages::new(pages));
    }

    fn start_day(&mut self, ctx: &mut Ctx<Parlor>, lead: Option<String>) {
        self.waiting = Waiting::Nothing;
        self.h.borrow_mut().start_work();
        let line = {
            let h = self.h.borrow();
            let where_ = h.where_line();
            let need = h.needs_you();
            let mut parts: Vec<String> = lead.into_iter().collect();
            parts.push(format!("Work starts. {where_}"));
            if let Some(first) = need.first() {
                parts.push(first.clone());
            }
            parts.join(" ")
        };
        self.h.borrow_mut().last = line.clone();
        ctx.say(line);
        self.flush(ctx);
    }

    /// After a supper's pages: bed, then the next morning or the end.
    fn after_supper(&mut self, ctx: &mut Ctx<Parlor>) {
        self.waiting = Waiting::Nothing;
        if self.h.borrow().phase == Phase::Supper {
            // Supper was skipped: eat anyway, quietly.
            self.h.borrow_mut().supper(None);
        }
        self.keep(ctx);
        self.flush_sounds_only(ctx);
        if self.h.borrow().phase == Phase::Over {
            self.finish(ctx);
            return;
        }
        self.morning(ctx);
    }

    /// Plays the sounds of events and drops their words, for events that
    /// came with pages already said.
    fn flush_sounds_only(&mut self, ctx: &mut Ctx<Parlor>) {
        let events = self.h.borrow_mut().take_events();
        for e in events {
            match e {
                Event::Keepsake(_) => {
                    ctx.audio.play(sound::KEEPSAKE, Bus::EFFECTS);
                }
                Event::GoalMet => {
                    ctx.audio.play(sound::GOAL, Bus::EFFECTS);
                }
                Event::Arrived(_) => {
                    ctx.audio.play(sound::ARRIVED, Bus::EFFECTS);
                }
                _ => {}
            }
        }
    }

    /// Sounds and says what the hamlet did, and opens any menu it asks for.
    fn flush(&mut self, ctx: &mut Ctx<Parlor>) {
        let events = self.h.borrow_mut().take_events();
        if events.is_empty() {
            return;
        }
        let asks = events.iter().any(|e| {
            matches!(
                e,
                Event::AskSeed | Event::AskBuild(_) | Event::AskBarge | Event::AskArrival
            )
        });
        let mut lead: Vec<String> = Vec::new();
        let mut said = false;
        let here = self.here();
        for e in events {
            match e {
                Event::Say(t) => {
                    if asks {
                        lead.push(t);
                    } else if said {
                        ctx.say_queued(t);
                    } else {
                        ctx.say(t);
                        said = true;
                    }
                }
                Event::Cue(id, at) => {
                    if at == here || self.work_sounds {
                        sound::cue(ctx, id, at, if at == here { 0.9 } else { 0.6 });
                    }
                }
                Event::Edge => {
                    ctx.audio.play(sound::EDGE, Bus::EFFECTS);
                }
                Event::Moved => {
                    let key = match self.h.borrow().at {
                        At::Lane(_) => sound::STEP_LANE,
                        At::In(..) => sound::STEP_IN,
                        At::Plot(..) => sound::STEP_FIELD,
                    };
                    ctx.audio.play(key, Bus::EFFECTS);
                    let here = self.here();
                    sound::listen_at(ctx, here);
                }
                Event::Bite => {
                    ctx.audio
                        .play_at(sound::BITE, sound::place_pos(Place::River), Bus::EFFECTS);
                }
                Event::SupperBell => {
                    ctx.audio.play(sound::SUPPER_BELL, Bus::EFFECTS);
                    self.waiting = Waiting::Supper;
                    ctx.push(SupperMenu::new(self.h.clone()));
                }
                Event::Built(b) => {
                    ctx.audio
                        .play_at(sound::BUILT, sound::place_pos(Place::of(b)), Bus::EFFECTS);
                }
                Event::Arrived(_) => {
                    ctx.audio.play(sound::ARRIVED, Bus::EFFECTS);
                }
                Event::Keepsake(_) => {
                    ctx.audio.play(sound::KEEPSAKE, Bus::EFFECTS);
                }
                Event::GoalMet => {
                    ctx.audio.play(sound::GOAL, Bus::EFFECTS);
                }
                Event::AskSeed => {
                    self.waiting = Waiting::Menu;
                    ctx.push(SeedMenu::new(self.h.clone()));
                }
                Event::AskBuild(b) => {
                    self.waiting = Waiting::Menu;
                    ctx.push(BuildMenu::new(self.h.clone(), b));
                }
                Event::AskBarge => {
                    self.waiting = Waiting::Menu;
                    ctx.push(BargeMenu::new(self.h.clone()));
                }
                Event::AskArrival => {
                    self.waiting = Waiting::Menu;
                    let line = lead.join(" ");
                    ctx.push(ArrivalMenu::new(self.h.clone(), line));
                }
            }
        }
    }

    /// The tune for this bar: every settler at work, from where they are.
    fn players(&self) -> Vec<Player> {
        let h = self.h.borrow();
        h.settlers
            .iter()
            .filter(|s| !s.away() && s.ill_days == 0)
            .map(|s| {
                let mood = s.mood();
                Player {
                    instrument: s.instrument,
                    at: sound::place_pos(s.place()),
                    volume: match mood {
                        Mood::Glum => 0.35,
                        Mood::Fine => 0.6,
                        Mood::Cheerful => 0.85,
                    },
                    sparse: mood == Mood::Glum,
                    same_song: s.wobble == Some(crate::rules::Wobble::SameSong),
                }
            })
            .collect()
    }

    /// A challenge is over: the score, then the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.done = true;
        self.stop_sounds(ctx);
        let (score, met, why) = {
            let h = self.h.borrow();
            (h.score(), h.goal_met, h.score_line())
        };
        let Some(ch) = self.challenge.clone() else {
            ctx.pop();
            return;
        };
        let result = Challenges::submit(ctx, &ch, score);
        let outcome = if met {
            "Goal met."
        } else {
            "The days ran out before the goal."
        };
        let title = format!("{outcome} {score} points. {}", short_result(result));
        let lines = vec![
            format!("{} result: {score} points.", ch.title()),
            why,
            match result {
                ScoreResult::First => "Your first score here is saved.".to_string(),
                ScoreResult::NewBest { previous } => {
                    format!("A new best. The old best was {previous}.")
                }
                ScoreResult::Short { best } => format!("Your best is {best}."),
            },
            ch.resets(),
        ];
        let period = ch.period;
        let r = Results::new(title, lines, Back::Challenges(CHALLENGES_NAME)).again(
            menus::labels::challenge_again(period),
            "The same hamlet again. Only your best result is kept.",
            move |ctx| {
                let pages = crate::challenge::briefing(ctx, period);
                ctx.replace(pages);
            },
        );
        ctx.replace(r);
    }

    fn handle_key(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        let action = ctx.action_for(event);
        if let Some(a) = action {
            let mut h = self.h.borrow_mut();
            match a {
                ParlorAction::Left => h.step(-1),
                ParlorAction::Right => h.step(1),
                ParlorAction::Up => h.step_in(true),
                ParlorAction::Down => h.step_in(false),
                ParlorAction::Primary => h.act(),
                ParlorAction::Secondary => h.tend(),
                ParlorAction::Status => {
                    let layers = h.status_layers();
                    drop(h);
                    let line = self.status_key.speak(ctx, layers);
                    if !line.is_empty() {
                        self.h.borrow_mut().last = line;
                    }
                }
                ParlorAction::Hint => {
                    let line = h.hint();
                    h.say(line);
                }
                ParlorAction::Repeat => h.repeat(),
            }
            return;
        }
        let Some(key) = event.pressed() else {
            return;
        };
        match key {
            Key::Return | Key::KpEnter => self.h.borrow_mut().talk(),
            Key::J => {
                self.waiting = Waiting::Menu;
                ctx.push(JobBoard::new(self.h.clone()));
            }
            Key::L => {
                let mut h = self.h.borrow_mut();
                let line = h.list();
                h.say(line);
            }
            Key::P => {
                self.waiting = Waiting::Menu;
                let pages = settler_pages(&self.h.borrow());
                ctx.push(Pages::new(pages));
            }
            Key::LCtrl
            | Key::RCtrl
            | Key::LShift
            | Key::RShift
            | Key::LAlt
            | Key::RAlt
            | Key::Insert
            | Key::Other(_) => {}
            _ => ctx.say("Not a Larkfield control. F1 lists them."),
        }
    }
}

/// The player's own key names.
fn keys(ctx: &Ctx<Parlor>) -> crate::rules::Keys {
    let k = |a| sentence_start(&ctx.control_name(a));
    crate::rules::Keys {
        main: k(ParlorAction::Primary),
        second: k(ParlorAction::Secondary),
        up: k(ParlorAction::Up),
        down: k(ParlorAction::Down),
    }
}

fn short_result(r: ScoreResult) -> String {
    match r {
        ScoreResult::First => "Your first score.".to_string(),
        ScoreResult::NewBest { .. } => "A new best.".to_string(),
        ScoreResult::Short { best } => format!("Best {best}."),
    }
}

/// Everyone who lives here, a page each.
pub fn settler_pages(h: &Hamlet) -> Vec<String> {
    if h.settlers.is_empty() {
        return vec!["Nobody lives in Larkfield yet.".to_string()];
    }
    h.settlers
        .iter()
        .map(|s| {
            let best: Vec<String> = s
                .best_skills()
                .iter()
                .take(3)
                .map(|k| format!("{} {}", k.name(), s.skill(*k)))
                .collect();
            let reasons = s.reasons();
            let why = if reasons.is_empty() {
                String::new()
            } else {
                format!(": {}", crate::rules::list_words(&reasons))
            };
            let story = s.trade.story();
            let dream = match s.story_step {
                0 => format!("Dream: {}", story.hook),
                3 => format!("Dream come true. Keepsake: {}.", story.keepsake),
                n => format!("Dream: {} {} of 3 steps done.", story.hook, n),
            };
            format!(
                "{}, once a {}, plays the {}. {} and {}: {}, and {}. {}{}. Best at {}. First job: {}. {}",
                s.name,
                s.trade.name(),
                s.instrument(),
                crate::rules::sentence_case(s.traits[0].name()),
                s.traits[1].name(),
                s.traits[0].about(),
                s.traits[1].about(),
                crate::rules::sentence_case(s.mood().name()),
                why,
                crate::rules::list_words(&best),
                s.job.name(),
                dream
            )
        })
        .collect()
}

impl State<Parlor> for HamletScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        HAMLET_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.entered {
            return self.resume(ctx);
        }
        self.entered = true;
        self.settings(ctx);
        self.start_sounds(ctx);
        let phase = self.h.borrow().phase;
        match phase {
            Phase::Morning => self.morning(ctx),
            Phase::Supper => {
                self.waiting = Waiting::Supper;
                ctx.push(SupperMenu::new(self.h.clone()));
            }
            Phase::Over => self.finish(ctx),
            Phase::Day => {
                let line = {
                    let h = self.h.borrow();
                    format!(
                        "Back in Larkfield. {} {}",
                        h.where_line(),
                        h.needs_you().first().cloned().unwrap_or_default()
                    )
                };
                self.h.borrow_mut().last = line.clone();
                ctx.say(line.trim().to_string());
            }
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.keep(ctx);
        self.stop_sounds(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.resume_quiet(ctx);
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.resume_all();
        if self.done {
            return;
        }
        let p = Progress::load(ctx);
        self.band_on = p.band;
        self.work_sounds = p.work_sounds;
        self.h.borrow_mut().keys = keys(ctx);
        self.h.borrow_mut().day_length = p.day_length;
        if self.kind() == Kind::Home {
            self.h.borrow_mut().pace = p.pace;
        }
        match std::mem::replace(&mut self.waiting, Waiting::Nothing) {
            Waiting::Morning => self.start_day(ctx, None),
            Waiting::Supper => self.after_supper(ctx),
            Waiting::Menu | Waiting::Nothing => {
                if self.h.borrow().phase == Phase::Day {
                    self.flush(ctx);
                }
            }
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || self.done {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            self.keep(ctx);
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        if self.h.borrow().phase != Phase::Day {
            return;
        }
        self.handle_key(ctx, event);
        self.flush(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.done || self.h.borrow().phase != Phase::Day {
            return;
        }
        self.h.borrow_mut().update(dt as f32);
        let season = self.h.borrow().season();
        if self.band_on && self.h.borrow().phase == Phase::Day {
            let players = self.players();
            self.band.follow(ctx, dt, season, &players);
        }
        self.flush(ctx);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let h = self.h.borrow();
        let mut out = vec![
            format!(
                "day {} ({} {}), hour {:.1}, {:?}, {}",
                h.day,
                h.season().name(),
                h.day_of_season(),
                h.hour,
                h.phase,
                h.weather.name()
            ),
            format!("at {:?}", h.at),
            format!("food days {}, {}", h.food_days(), h.store_line()),
        ];
        for s in &h.settlers {
            out.push(format!(
                "{}: {} {}, at {}, {}",
                s.name,
                s.mood().name(),
                s.instrument(),
                s.place().name(),
                s.doing()
            ));
        }
        out
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
    let k = |a| ctx.control_name(a);
    let (left, right, up, down) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
    );
    let (main, second, status, hint, repeat) = (
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
        k(ParlorAction::Repeat),
    );
    format!(
        "{} and {right} walk the lane, or move inside a place. {} goes into a place, {down} back out. {} does the job in front of you: dig, sow, water, harvest, chop, fish, build or trade. {} pulls weeds and feeds animals. Enter talks to whoever is here. J opens the job board, L lists what is around you, and P tells you about everyone in Larkfield. {} says what needs you, and again for more. {} says what to do next. {} repeats the last line. Escape pauses.",
        sentence_start(&left),
        sentence_start(&up),
        sentence_start(&main),
        sentence_start(&second),
        sentence_start(&status),
        sentence_start(&hint),
        sentence_start(&repeat),
    )
}

mod visuals_hamlet;
