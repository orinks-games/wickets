//! Walking a trail: the play screen for trail walks, the practice glade
//! and the daily challenge.

use std::collections::HashSet;

use hearken::text::{end_sentence, sentence_start};
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{challenges, Challenge, Pages, Parlor, ParlorAction, Period, Season};

use crate::birds::{notes_words, Note};
use crate::challenge::{self, CHALLENGES_NAME};
use crate::guide::{self, Guide, Learned};
use crate::rules::{
    compass, distance_words, run_seed, sector, sector_words, turn_words, Ev, Phase, Walk,
};
use crate::trails::Trail;
use crate::{days, sound, HELP, ID, TITLE_NAME};

/// The walk screen's `State::name`.
pub const WALK_NAME: &str = "birdsong_trail_walk";
/// Seconds from the trail's end to the results, so the arrival is heard.
pub const END_PAUSE: f64 = 2.0;
/// Seconds between steps while an arrow is held.
pub const STEP_REPEAT: f64 = 0.4;

/// What kind of walk the screen is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Trail(Trail),
    /// The practice glade: nothing to keep, one bird at a time.
    Practice,
    Challenge(Period),
}

impl Mode {
    pub fn name(self) -> String {
        match self {
            Mode::Trail(t) => sentence_start(t.name()),
            Mode::Practice => "Practice glade".to_string(),
            Mode::Challenge(p) => challenge::name(p).to_string(),
        }
    }
}

/// Seconds the walk keys wait after a bird starts singing to you.
pub const SETTLE: f64 = 0.5;

pub struct WalkScreen {
    pub walk: Walk,
    pub mode: Mode,
    challenge: Option<Challenge>,
    /// Cues already explained in words this walk.
    taught: HashSet<&'static str>,
    /// The last thing said, for Repeat.
    last: String,
    ending: Option<f64>,
    end_loop: Option<Sound>,
    /// Tab was pressed once: a second press heads home.
    home_asked: bool,
    /// Run by the tutorial: the trail's end does not end the walk, so the
    /// lesson finishes at the chickadee.
    pub lesson: bool,
    /// What each answer added to the guide, by bird.
    learned: Vec<(usize, Learned)>,
    /// Kinds of bird in the guide when the walk began.
    birds_before: u32,
    since_step: f64,
    /// Seconds left in which Up and Down, the walk keys, are ignored after
    /// a bird starts singing, so the step that reached it is not an answer.
    settle: f64,
    was_on_path: bool,
    call_outs: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

/// Which trails the guide opens.
fn open_trails(guide: &Guide) -> usize {
    Trail::ALL.iter().filter(|t| guide.opens(**t)).count()
}

impl WalkScreen {
    fn build(walk: Walk, mode: Mode) -> WalkScreen {
        WalkScreen {
            status_key: Default::default(),
            walk,
            mode,
            challenge: None,
            taught: HashSet::new(),
            last: String::new(),
            ending: None,
            end_loop: None,
            home_asked: false,
            lesson: false,
            learned: Vec::new(),
            birds_before: 0,
            since_step: STEP_REPEAT,
            settle: 0.0,
            was_on_path: true,
            call_outs: false,
        }
    }

    /// A walk on `trail` in today's season, birds chosen from the guide.
    pub fn trail(ctx: &mut Ctx<Parlor>, trail: Trail) -> WalkScreen {
        let guide = Guide::load(ctx);
        let season = days::season(challenges::today(ctx));
        WalkScreen::build(
            Walk::on(trail, season, &guide, run_seed()),
            Mode::Trail(trail),
        )
    }

    pub fn practice() -> WalkScreen {
        WalkScreen::build(Walk::practice(), Mode::Practice)
    }

    /// A walk of the daily challenge.
    pub fn challenge(ch: Challenge, plan: &challenge::Plan) -> WalkScreen {
        let mut s = WalkScreen::build(plan.walk(), Mode::Challenge(ch.period));
        s.challenge = Some(ch);
        s
    }

    /// A screen for a walk built by hand, for tests.
    pub fn with_walk(walk: Walk, mode: Mode) -> WalkScreen {
        WalkScreen::build(walk, mode)
    }

    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        self.last = text.clone();
        ctx.say(text);
    }

    fn key(ctx: &Ctx<Parlor>, a: ParlorAction) -> String {
        ctx.control_name(a)
    }

    fn bird_name(&self, i: usize) -> &'static str {
        self.walk.birds[i].species.name
    }

    /// The answer keys, in a sentence.
    pub fn answer_keys(ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} is low, {} is middle, {} is high.",
            sentence_start(&Self::key(ctx, ParlorAction::Down)),
            Self::key(ctx, ParlorAction::Primary),
            Self::key(ctx, ParlorAction::Up)
        )
    }

    /// Every control, in a few sentences.
    pub fn controls(ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "{} and {} turn. {} steps forward and {} steps back. When a bird sings to you, answer it note for note: {} is low, {} is middle, {} is high. {} names the birds around you, {} speaks the status and {} gives a hint. {} twice heads home.",
            sentence_start(&k(ParlorAction::Left)),
            k(ParlorAction::Right),
            sentence_start(&k(ParlorAction::Up)),
            k(ParlorAction::Down),
            k(ParlorAction::Down),
            k(ParlorAction::Primary),
            k(ParlorAction::Up),
            sentence_start(&k(ParlorAction::Primary)),
            k(ParlorAction::Status),
            k(ParlorAction::Hint),
            sentence_start(&k(ParlorAction::Secondary)),
        )
    }

    fn intro(&self) -> String {
        let n = self.walk.birds.len();
        let birds = if n == 1 {
            "1 bird is".to_string()
        } else {
            format!("{n} birds are")
        };
        match self.mode {
            Mode::Practice => "Practice glade. A chickadee sings just ahead of you, and a wagtail off to the right. Walk toward the chickadee.".to_string(),
            _ => format!(
                "{}, {}. {} {birds} singing. {} is at the trail's end, straight ahead.",
                sentence_start(self.walk.trail.name()),
                self.walk.season.name(),
                self.walk.trail.opening(self.walk.season),
                sentence_start(self.walk.trail.end_name()),
            ),
        }
    }

    /// What has come and gone with the season, for the walk's start.
    fn season_news(&self) -> Option<String> {
        if !matches!(self.mode, Mode::Trail(_)) {
            return None;
        }
        let habitat = self.walk.trail.habitat();
        let season = self.walk.season;
        let here: Vec<&str> = self
            .walk
            .birds
            .iter()
            .map(|b| b.species)
            .filter(|s| crate::birds::arrivals(habitat, season).contains(s))
            .map(|s| s.name)
            .collect();
        if let Some(name) = here.first() {
            let line = match season {
                Season::Winter | Season::Autumn => {
                    format!("A winter visitor is here: the {name}.")
                }
                _ => format!("Back for {}: the {name}.", season.name()),
            };
            return Some(line);
        }
        let gone = crate::birds::departures(habitat, season);
        gone.first().map(|s| {
            if season == Season::Spring {
                format!("The {} has gone north for the summer.", s.name)
            } else {
                format!("The {} has gone south for the winter.", s.name)
            }
        })
    }

    /// The status line, in a fixed order: the bird you are with, the
    /// walk, the friends made, where you face, the trail's end.
    /// The status key, in layers. First: the bird you are with, which way
    /// you face and where the trail's end is. A second press: the trail,
    /// the season and the birds befriended.
    pub fn status(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let w = &self.walk;
        let mut parts = Vec::new();
        match &w.phase {
            Phase::Listening(i) => {
                parts.push(format!("the {} is singing to you", self.bird_name(*i)))
            }
            Phase::Answering(i, got) => parts.push(format!(
                "answering the {}, {} of {} notes",
                self.bird_name(*i),
                got.len(),
                w.birds[*i].verse.len()
            )),
            Phase::Walking => {}
        }
        parts.push(format!("facing {}", compass(w.facing)));
        let (sector, d) = w.end_whereabouts();
        parts.push(format!(
            "the trail's end {}, {}",
            sector_words(sector),
            distance_words(d)
        ));
        let place = match self.mode {
            Mode::Trail(_) | Mode::Challenge(_) => {
                format!("{}, {}", w.trail.name(), w.season.name())
            }
            Mode::Practice => "the practice glade".to_string(),
        };
        let more = format!("{place}, {} of {} befriended", w.friends(), w.birds.len());
        let _ = ctx;
        vec![
            end_sentence(&sentence_start(&parts.join(", "))),
            end_sentence(&sentence_start(&more)),
        ]
    }

    /// Where the nearest new bird is, and how to turn to it.
    pub fn hint(&mut self, ctx: &mut Ctx<Parlor>) -> String {
        if let Some(i) = self.walk.focus() {
            let name = self.bird_name(i);
            let words = notes_words(&self.walk.birds[i].verse);
            self.walk.again();
            return format!("The {name}'s song: {words}. It sings once more.");
        }
        match self.walk.nearest_new() {
            Some(i) => {
                let (s, d) = self.walk.whereabouts(i);
                let name = sentence_start(&self.walk.birds[i].species.a_name());
                let turn = turn_words(s);
                let walk = if turn.is_empty() {
                    "Walk toward it.".to_string()
                } else {
                    format!("{turn} Then walk toward it.")
                };
                let _ = ctx;
                format!(
                    "{name} is {}, {}. {walk}",
                    sector_words(s),
                    distance_words(d)
                )
            }
            None => {
                let (s, d) = self.walk.end_whereabouts();
                format!(
                    "Every bird here is a friend. The trail's end is {}, {}. {}",
                    sector_words(s),
                    distance_words(d),
                    turn_words(s)
                )
                .trim_end()
                .to_string()
            }
        }
    }

    /// What you hear around you, nearest first, and each bird sings.
    fn listen(&mut self) -> String {
        let mut lines = Vec::new();
        for i in self.walk.by_distance().into_iter().take(3) {
            let (s, d) = self.walk.whereabouts(i);
            let b = &self.walk.birds[i];
            let who = if b.friend {
                format!("your friend the {}", b.species.name)
            } else {
                b.species.a_name()
            };
            lines.push(format!("{who} {}, {}", sector_words(s), distance_words(d)));
        }
        let (s, d) = self.walk.end_whereabouts();
        lines.push(format!(
            "{} {}, {}",
            self.walk.trail.end_name(),
            sector_words(s),
            distance_words(d)
        ));
        format!("Nearby: {}.", lines.join("; "))
    }

    fn key_lines(&mut self, ctx: &mut Ctx<Parlor>, i: usize, missed: bool) -> String {
        let b = &self.walk.birds[i];
        let words = notes_words(&b.verse);
        let spell = self.call_outs || b.tries >= 2;
        let mut line = if missed {
            format!("Not quite. The {} sings again.", b.species.name)
        } else {
            String::new()
        };
        if spell {
            line = format!("{line} Its song: {words}.");
        }
        if self.first("keys") {
            line = format!("{line} {}", Self::answer_keys(ctx));
        }
        line.trim().to_string()
    }

    fn learn(&mut self, ctx: &mut Ctx<Parlor>, i: usize) -> Vec<String> {
        let b = &self.walk.birds[i];
        let species = b.species;
        let mut lines = vec![format!("Befriended: the {}.", species.name)];
        if self.walk.practice {
            return lines;
        }
        let mut g = Guide::load(ctx);
        let where_when = guide::where_when(self.walk.trail, self.walk.season);
        let before = open_trails(&g);
        let learned = g.learn(species, b.verse_no, &where_when);
        g.store(ctx);
        self.learned.push((i, learned));
        if learned.new_bird {
            sound::play(ctx, sound::NEW_SONG);
            lines.push("New in your field guide.".to_string());
        } else if learned.new_verse {
            sound::play(ctx, sound::NEW_SONG);
            lines.push(format!(
                "A new song for your guide, {} of {}.",
                learned.verses,
                species.verses()
            ));
        }
        if open_trails(&g) > before {
            if let Some(t) = Trail::ALL.iter().find(|t| t.needs() == g.birds()) {
                lines.push(format!("The {} is open.", t.name()));
            }
        }
        lines
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        let mut lines: Vec<String> = Vec::new();
        let mut queued: Vec<String> = Vec::new();
        for ev in evs {
            match ev {
                Ev::Note { bird, note } => {
                    let b = &self.walk.birds[bird];
                    sound::bird_note(ctx, b.species, note, Some(sound::v3(b.pos)));
                }
                // The lesson's own page says this, and holds the song for it.
                Ev::Noticed { .. } if self.lesson => {}
                Ev::Noticed { bird } => {
                    self.settle = SETTLE;
                    let name = sentence_start(&self.walk.birds[bird].species.a_name());
                    if self.first("noticed") {
                        lines.push(format!(
                            "{name} sings to you, close by. Answer when it ends."
                        ));
                    } else {
                        lines.push(format!("{name} sings to you."));
                    }
                }
                Ev::Ready { bird } => {
                    let extra = self.key_lines(ctx, bird, false);
                    lines.push(format!("Your turn. {extra}").trim().to_string());
                }
                Ev::Echo(note) => {
                    sound::your_note(ctx, note);
                }
                Ev::Befriended { bird } => {
                    sound::play(ctx, sound::BEFRIEND);
                    lines.extend(self.learn(ctx, bird));
                    let about = self.walk.birds[bird].species.about;
                    if self
                        .learned
                        .last()
                        .is_some_and(|(i, l)| *i == bird && l.new_bird)
                    {
                        queued.push(about.to_string());
                    }
                    if self.walk.friends() == self.walk.birds.len() {
                        queued.push(format!(
                            "Every bird here is a friend. Walk on to {}.",
                            self.walk.trail.end_name()
                        ));
                    }
                }
                Ev::Missed { bird, .. } => {
                    sound::play(ctx, sound::AGAIN);
                    lines.push(self.key_lines(ctx, bird, true));
                }
                Ev::Busy { bird } => {
                    lines.push(format!(
                        "The {} is still singing. Answer when it ends.",
                        self.bird_name(bird)
                    ));
                }
                Ev::LeftBird { bird } => {
                    lines.push(format!("You let the {} be.", self.bird_name(bird)));
                }
                Ev::Nudge { bird } => {
                    if self.first("nudge") {
                        lines.push(format!(
                            "The {} sings again.",
                            self.bird_name(bird)
                        ));
                    }
                }
                Ev::Step(ground) => {
                    let key = sound::step_key(ground);
                    sound::play_vol(ctx, &key, 0.6);
                    let on = self.walk.on_path();
                    if on != self.was_on_path {
                        self.was_on_path = on;
                        if on {
                            lines.push("Back on the path.".to_string());
                        } else {
                            lines.push("Off the path.".to_string());
                        }
                    }
                }
                Ev::Blocked { end } => {
                    sound::play(ctx, sound::BUMP);
                    if end && self.walk.y > self.walk.length / 2.0 {
                        lines.push("The trail ends here.".to_string());
                    } else if end {
                        lines.push("The trail begins here.".to_string());
                    } else {
                        // Which way the path lies, from where you face.
                        let w = &self.walk;
                        let path = sector_words(sector(w.facing, (w.x, w.y), (0.0, w.y)));
                        lines.push(format!("{} The path is {path}.", w.trail.edge()));
                    }
                }
                Ev::Turned(f) => {
                    sound::play_vol(ctx, sound::TURN, 0.5);
                    lines.push(end_sentence(&sentence_start(compass(f))));
                }
                Ev::ReachedEnd if self.lesson => {
                    self.walk.finished = false;
                    lines.push(
                        "This is the trail's end, but the chickadee is still waiting. Turn back toward its song."
                            .to_string(),
                    );
                }
                Ev::ReachedEnd => {
                    sound::play(ctx, sound::ARRIVE);
                    lines.push(format!(
                        "Trail's end: {}. {} of {} birds befriended.",
                        self.walk.trail.end_name(),
                        self.walk.friends(),
                        self.walk.birds.len()
                    ));
                    self.ending = Some(END_PAUSE);
                }
            }
        }
        if !lines.is_empty() {
            let text = lines.join(" ");
            self.say(ctx, text);
        }
        for q in queued {
            ctx.say_queued(q);
        }
    }

    fn update_listener(&self, ctx: &mut Ctx<Parlor>) {
        ctx.audio
            .set_listener(sound::v3(self.walk.pos()), self.walk.facing as f32 * 45.0);
    }

    fn start_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        self.update_listener(ctx);
        let bed = sound::bed_key(self.walk.trail, self.walk.season);
        sound::bed(ctx, &bed);
        if self.end_loop.is_none() {
            let key = sound::end_key(self.walk.trail);
            let pos = self.walk.end_pos();
            let s = ctx
                .audio
                .start_loop(&key, Some(Vec3::new(pos.0, pos.1, 0.0)), Bus::AMBIENCE);
            self.end_loop = Some(s);
        }
    }

    fn stop_sounds(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.end_loop.take() {
            ctx.audio.stop(s, 400);
        }
        ctx.audio.set_bed(None);
    }

    /// The walk is over, at the trail's end or by heading home.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
        let home = !self.walk.finished;
        self.walk.finished = true;
        if self.mode == Mode::Practice {
            let mut g = Guide::load(ctx);
            g.practice += 1;
            g.store(ctx);
            ctx.pop_to_with(
                TITLE_NAME,
                format!(
                    "Practice done, {} of 2 birds befriended",
                    self.walk.friends()
                ),
            );
            return;
        }
        let mut g = Guide::load(ctx);
        g.walks += 1;
        g.steps += self.walk.steps;
        g.store(ctx);
        let summary = self.summary(home);
        let mut pages = vec![summary.clone()];
        pages.extend(self.detail_pages(&g));
        if let Some(ch) = self.challenge.take() {
            let more = challenge::finish_walk(ctx, &ch, &self.walk);
            pages.extend(more);
            let summary = challenge::scored(&summary, challenge::walk_points(&self.walk));
            let results = after_challenge(ch.period, summary, pages.clone());
            let mut results = Some(results);
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(r) = results.take() {
                    ctx.replace(r);
                }
            }));
            return;
        }
        let mode = self.mode;
        let mut results = Some(after_walk(mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(r) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(r);
            }
        }));
    }

    /// The results headline.
    pub fn summary(&self, home: bool) -> String {
        let w = &self.walk;
        let new = self.learned.iter().filter(|(_, l)| l.new_bird).count();
        let new = match new {
            0 => String::new(),
            1 => ", 1 new in your field guide".to_string(),
            n => format!(", {n} new in your field guide"),
        };
        let place = w.trail.name().to_string();
        let done = if home {
            format!("Home from {place}")
        } else {
            format!("Walk done: {place}")
        };
        format!(
            "{done}. {} of {} birds befriended{new}.",
            w.friends(),
            w.birds.len()
        )
    }

    fn detail_pages(&self, g: &Guide) -> Vec<String> {
        let mut pages = Vec::new();
        let friends: Vec<String> = self
            .walk
            .birds
            .iter()
            .filter(|b| b.friend)
            .map(|b| b.species.name.to_string())
            .collect();
        let songs = self.learned.iter().filter(|(_, l)| l.new_verse).count();
        if !friends.is_empty() {
            let songs = match songs {
                0 => "No new songs this time.".to_string(),
                1 => "1 new song for your guide.".to_string(),
                n => format!("{n} new songs for your guide."),
            };
            pages.push(format!("Befriended: {}. {songs}", friends.join(", ")));
        }
        let open: Vec<&str> = Trail::ALL
            .iter()
            .filter(|t| t.needs() > self.birds_before && t.needs() <= g.birds())
            .map(|t| t.name())
            .collect();
        let opened = match open.as_slice() {
            [] => String::new(),
            [one] => format!(" The {one} is now open."),
            many => format!(" Now open: {}.", many.join(", ")),
        };
        pages.push(format!(
            "Your field guide: {} of {} birds, {} of {} songs.{opened}",
            g.birds(),
            crate::birds::ALL.len(),
            g.songs(),
            guide::total_songs()
        ));
        pages
    }

    fn head_home(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.home_asked {
            self.home_asked = false;
            self.say(ctx, "Heading home.");
            self.finish(ctx);
        } else {
            self.home_asked = true;
            let tab = sentence_start(&Self::key(ctx, ParlorAction::Secondary));
            self.say(
                ctx,
                format!("Head home? {tab} again heads home. Any other key keeps walking."),
            );
        }
    }

    fn note_for(a: ParlorAction) -> Option<Note> {
        match a {
            ParlorAction::Down => Some(Note::Low),
            ParlorAction::Primary => Some(Note::Mid),
            ParlorAction::Up => Some(Note::High),
            _ => None,
        }
    }
}

/// The results menu after a walk.
pub fn after_walk(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let again_help = match mode {
        Mode::Trail(t) => format!("Another walk on the {}, with new birds.", t.name()),
        Mode::Practice | Mode::Challenge(_) => "Another walk.".to_string(),
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME))
        .again(labels::PLAY_AGAIN, again_help, move |ctx| {
            let s = match mode {
                Mode::Trail(t) => WalkScreen::trail(ctx, t),
                Mode::Practice => WalkScreen::practice(),
                Mode::Challenge(_) => return,
            };
            ctx.replace(s)
        })
        .extra(
            "Field guide",
            "Every bird you have met, with its song in words.",
            |ctx| ctx.push(crate::field::FieldGuide::new()),
        )
}

/// The results menu after a challenge walk.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same walk, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            challenge::start(ctx, ch, true)
        },
    )
}

impl State<Parlor> for WalkScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        WALK_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let g = Guide::load(ctx);
        self.call_outs = g.call_outs;
        self.birds_before = g.birds();
        self.start_sounds(ctx);
        let intro = self.intro();
        self.say(ctx, intro);
        if let Some(news) = self.season_news() {
            ctx.say_queued(news);
        }
        let known = g.any() || menus::tutorial_read(ctx, crate::ID);
        if !known || self.mode == Mode::Practice {
            ctx.say_queued(Self::controls(ctx));
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_sounds(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.walk.finished {
            return;
        }
        // Settings, from the pause menu, may have changed the call-outs.
        self.call_outs = Guide::load(ctx).call_outs;
        self.start_sounds(ctx);
        let status = self.status(ctx).remove(0);
        self.say(ctx, format!("Back on the trail. {status}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.walk.finished || self.ending.is_some() {
            return;
        }
        let action = ctx.action_for(event);
        if event.is_repeat() {
            let walking = self.walk.phase == Phase::Walking;
            let moving = matches!(
                action,
                Some(
                    ParlorAction::Up
                        | ParlorAction::Down
                        | ParlorAction::Left
                        | ParlorAction::Right
                )
            );
            if !(walking && moving && self.since_step >= STEP_REPEAT) {
                return;
            }
        }
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        if action == Some(ParlorAction::Secondary) {
            self.head_home(ctx);
            return;
        }
        let listening = matches!(self.walk.phase, Phase::Listening(_));
        if listening
            && self.settle > 0.0
            && matches!(action, Some(ParlorAction::Up | ParlorAction::Down))
        {
            return;
        }
        if action.is_some() {
            self.home_asked = false;
        }
        let focus = self.walk.focus().is_some();
        let evs = match action {
            Some(ParlorAction::Left) => {
                self.since_step = 0.0;
                self.walk.turn(-1)
            }
            Some(ParlorAction::Right) => {
                self.since_step = 0.0;
                self.walk.turn(1)
            }
            Some(a @ (ParlorAction::Up | ParlorAction::Down | ParlorAction::Primary)) if focus => {
                match Self::note_for(a) {
                    Some(n) => self.walk.answer(n),
                    None => Vec::new(),
                }
            }
            Some(ParlorAction::Up) => {
                self.since_step = 0.0;
                self.walk.step(true)
            }
            Some(ParlorAction::Down) => {
                self.since_step = 0.0;
                self.walk.step(false)
            }
            Some(ParlorAction::Primary) => {
                let line = self.listen();
                for i in self.walk.by_distance().into_iter().take(3) {
                    let b = &self.walk.birds[i];
                    let note = b.species.notes()[0];
                    sound::bird_note(ctx, b.species, note, Some(sound::v3(b.pos)));
                }
                self.say(ctx, line);
                Vec::new()
            }
            Some(ParlorAction::Status) => {
                let layers = self.status(ctx);
                self.last = self.status_key.speak(ctx, layers);
                Vec::new()
            }
            Some(ParlorAction::Repeat) => {
                if let Some(i) = self.walk.focus() {
                    self.walk.again();
                    let name = self.bird_name(i);
                    self.say(ctx, format!("The {name} sings again."));
                } else if self.last.is_empty() {
                    let s = self.status(ctx).remove(0);
                    self.say(ctx, s);
                } else {
                    ctx.say(self.last.clone());
                }
                Vec::new()
            }
            Some(ParlorAction::Hint) => {
                let h = self.hint(ctx);
                self.say(ctx, h);
                Vec::new()
            }
            Some(ParlorAction::Secondary) => Vec::new(),
            None => Vec::new(),
        };
        self.update_listener(ctx);
        self.voice(ctx, evs);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.ending = None;
                self.finish(ctx);
            }
            return;
        }
        if self.walk.finished {
            return;
        }
        self.since_step += dt;
        self.settle = (self.settle - dt).max(0.0);
        let evs = self.walk.update(dt);
        self.voice(ctx, evs);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let w = &self.walk;
        let mut out = vec![self.mode.name()];
        out.push(format!("Befriended: {} of {}", w.friends(), w.birds.len()));
        out.push(format!("Facing: {}", compass(w.facing)));
        out.push(format!(
            "{} walked up the trail: {:.0}",
            hearken::text::sentence_start(parlor_core::units::plural()),
            parlor_core::units::from_metres(f64::from(w.y.max(0.0)))
        ));
        if let Some(i) = w.focus() {
            out.push(format!("With: {}", w.birds[i].species.name));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let now = match &self.walk.phase {
            Phase::Listening(i) => format!(
                "The {} is singing to you. Answer when it ends.",
                self.bird_name(*i)
            ),
            Phase::Answering(i, _) => format!(
                "The {} waits for your answer. {}",
                self.bird_name(*i),
                Self::answer_keys(ctx)
            ),
            Phase::Walking => "Walk toward a song to meet its bird.".to_string(),
        };
        let mut text = format!("{now} {}", Self::controls(ctx));
        if let Some(ch) = &self.challenge {
            text.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count the walk.",
                ch.period.label().to_lowercase(),
                challenge::name(ch.period)
            ));
        }
        text
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_walkscreen;
