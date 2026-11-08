//! The dawn chorus: the special walk of International Dawn Chorus Day,
//! and the weekly challenge. Every bird sings at once from its place
//! around you, and you pick out the one asked for.

use std::collections::HashSet;

use hearken::text::{end_sentence, sentence_start};
use hearken::{Ctx, InputEvent, Key, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{challenges, Challenge, Pages, Parlor, ParlorAction, Period, Records, Season};

use crate::birds::Kind;
use crate::challenge::{self, CHALLENGES_NAME};
use crate::guide::{self, Guide};
use crate::rules::{compass, run_seed, sector_words, turn_words, Chorus, ChorusEv};
use crate::walk::END_PAUSE;
use crate::{days, sound, HELP, ID, TITLE_NAME};

/// The chorus screen's `State::name`.
pub const CHORUS_NAME: &str = "birdsong_trail_chorus";
/// The records board of the dawn chorus: the most birds picked out.
pub const CHORUS_BOARD: &str = "chorus";

pub struct ChorusScreen {
    pub chorus: Chorus,
    pub season: Season,
    challenge: Option<Challenge>,
    taught: HashSet<&'static str>,
    last: String,
    ending: Option<f64>,
    home_asked: bool,
    new_birds: u32,
}

impl ChorusScreen {
    /// The dawn chorus of today's season.
    pub fn today(ctx: &mut Ctx<Parlor>) -> ChorusScreen {
        let season = days::season(challenges::today(ctx));
        ChorusScreen::new(Chorus::of_season(season, run_seed()), season)
    }

    pub fn new(chorus: Chorus, season: Season) -> ChorusScreen {
        ChorusScreen {
            chorus,
            season,
            challenge: None,
            taught: HashSet::new(),
            last: String::new(),
            ending: None,
            home_asked: false,
            new_birds: 0,
        }
    }

    /// The weekly challenge's chorus.
    pub fn challenge(ch: Challenge, chorus: Chorus, season: Season) -> ChorusScreen {
        let mut s = ChorusScreen::new(chorus, season);
        s.challenge = Some(ch);
        s
    }

    fn say(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        ctx.say(text);
    }

    fn name(&self) -> String {
        match &self.challenge {
            Some(ch) => challenge::name(ch.period).to_string(),
            None => "The dawn chorus".to_string(),
        }
    }

    /// "Find the robin: a run of clear, wistful whistles."
    fn target_line(&self) -> String {
        match self.chorus.target_species() {
            Some(t) => format!("Find the {}: {}.", t.name, t.sounds_like),
            None => String::new(),
        }
    }

    pub fn controls(ctx: &Ctx<Parlor>) -> String {
        let k = |a| ctx.control_name(a);
        format!(
            "{} and {} turn to face each place around you. {} picks the bird you face. {} plays the song to find alone, {} says where it is, and {} twice heads home.",
            sentence_start(&k(ParlorAction::Left)),
            k(ParlorAction::Right),
            sentence_start(&k(ParlorAction::Primary)),
            sentence_start(&k(ParlorAction::Repeat)),
            k(ParlorAction::Hint),
            sentence_start(&k(ParlorAction::Secondary)),
        )
    }

    pub fn status(&self) -> String {
        let c = &self.chorus;
        let mut parts = vec![
            self.name(),
            format!("{} of {} found", c.found.len(), c.total),
            format!("facing {}", compass(c.facing)),
        ];
        if let Some(t) = c.target_species() {
            parts.push(format!("find the {}", t.name));
        }
        end_sentence(&parts.join(", "))
    }

    fn hint(&self) -> String {
        let c = &self.chorus;
        match (c.target, c.target_species()) {
            (Some(place), Some(t)) => {
                let s = c.sector_of(place);
                let turn = turn_words(s);
                if turn.is_empty() {
                    format!("The {} is straight ahead. Pick it now.", t.name)
                } else {
                    format!("The {} is {}. {turn}", t.name, sector_words(s))
                }
            }
            _ => "Every bird is found.".to_string(),
        }
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<ChorusEv>) {
        let mut lines: Vec<String> = Vec::new();
        for ev in evs {
            match ev {
                ChorusEv::Note { place, note } => {
                    if let Some(s) = &self.chorus.places[place] {
                        let (x, y) = Chorus::place_pos(place);
                        sound::bird_note(ctx, s.species, note, Some(Vec3::new(x, y, 0.0)));
                    }
                }
                ChorusEv::Solo { note } => {
                    if let Some(t) = self.chorus.target_species() {
                        sound::bird_note(ctx, t, note, None);
                    }
                }
                ChorusEv::Target(_) => lines.push(self.target_line()),
                ChorusEv::Found { species, first_try } => {
                    sound::play(ctx, sound::FOUND);
                    let mut g = Guide::load(ctx);
                    let new = g.hear(species, guide::special_where(Kind::Trail));
                    g.store(ctx);
                    let n = self.chorus.found.len();
                    let total = self.chorus.total;
                    let mut line = if first_try {
                        format!("The {}. Found first time, {n} of {total}.", species.name)
                    } else {
                        format!("The {}. Found, {n} of {total}.", species.name)
                    };
                    if new {
                        self.new_birds += 1;
                        line.push_str(" New in your field guide.");
                    }
                    lines.push(line);
                }
                ChorusEv::Wrong {
                    got,
                    place_of_target: _,
                } => {
                    sound::play(ctx, sound::AGAIN);
                    let line = match got {
                        Some(s) => format!("That is the {}.", s.name),
                        None => "No bird there.".to_string(),
                    };
                    lines.push(line);
                    if self.taught.insert("wrong") {
                        let h = sentence_start(&ctx.control_name(ParlorAction::Hint));
                        lines.push(format!("{h} says where it is."));
                    }
                }
                ChorusEv::Turned(f) => {
                    sound::play_vol(ctx, sound::TURN, 0.5);
                    lines.push(end_sentence(&sentence_start(compass(f))));
                }
                ChorusEv::Arrived { .. } => {}
                ChorusEv::Finished => {
                    sound::play(ctx, sound::ARRIVE);
                    lines.push(format!(
                        "Every bird found, {} on the first try.",
                        self.chorus.first_tries
                    ));
                    self.ending = Some(END_PAUSE);
                }
            }
        }
        if !lines.is_empty() {
            let text = lines.join(" ");
            self.say(ctx, text);
        }
    }

    pub fn summary(&self) -> String {
        let c = &self.chorus;
        format!(
            "{}: {} of {} birds picked out, {} on the first try.",
            self.name(),
            c.found.len(),
            c.total,
            c.first_tries
        )
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
        self.chorus.finished = true;
        let summary = self.summary();
        let found = self.chorus.found.len() as u32;
        let mut pages = vec![summary.clone()];
        if !self.chorus.found.is_empty() {
            let names: Vec<&str> = self.chorus.found.iter().map(|s| s.name).collect();
            pages.push(format!("Picked out: {}.", names.join(", ")));
        }
        if let Some(ch) = self.challenge.take() {
            pages.extend(challenge::finish_chorus(ctx, &ch, &self.chorus));
            let summary = challenge::scored(&summary, challenge::chorus_points(&self.chorus));
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(r) = results.take() {
                    ctx.replace(r);
                }
            }));
            return;
        }
        let mut g = Guide::load(ctx);
        g.choruses += 1;
        let record = found > g.chorus_best;
        g.chorus_best = g.chorus_best.max(found);
        g.store(ctx);
        Records::record(ctx, ID, CHORUS_BOARD, found as u64);
        let best = if record {
            "Your best dawn chorus yet.".to_string()
        } else {
            format!("Your best is {} birds.", g.chorus_best)
        };
        let new = match self.new_birds {
            0 => String::new(),
            1 => " 1 bird is new in your field guide.".to_string(),
            n => format!(" {n} birds are new in your field guide."),
        };
        pages.push(format!("{best}{new}"));
        let mut results = Some(
            Results::new(summary, pages.clone(), Back::Title(TITLE_NAME)).again(
                labels::PLAY_AGAIN,
                "The chorus again, the birds in new places.",
                |ctx| {
                    let s = ChorusScreen::today(ctx);
                    ctx.replace(s)
                },
            ),
        );
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(r) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(r);
            }
        }));
    }

    fn head_home(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.home_asked {
            self.home_asked = false;
            self.finish(ctx);
        } else {
            self.home_asked = true;
            let tab = sentence_start(&ctx.control_name(ParlorAction::Secondary));
            self.say(
                ctx,
                format!("Head home? {tab} again heads home. Any other key keeps listening."),
            );
        }
    }
}

/// The results menu after the weekly challenge.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same chorus, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            challenge::start(ctx, ch, true)
        },
    )
}

impl State<Parlor> for ChorusScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        CHORUS_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::bed(ctx, sound::BED_DAWN);
        let total = self.chorus.total;
        let intro = match &self.challenge {
            Some(_) => format!(
                "{}. {total} birds sing around you at once. Pick each one out as it is asked for.",
                self.name()
            ),
            None => format!(
                "The dawn chorus. It is International Dawn Chorus Day, and every bird of {} is singing around you, {total} in all. Pick each one out as it is asked for.",
                self.season.name()
            ),
        };
        let target = self.target_line();
        self.say(ctx, format!("{intro} {target}"));
        ctx.say_queued(Self::controls(ctx));
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.chorus.finished {
            return;
        }
        sound::bed(ctx, sound::BED_DAWN);
        let s = self.status();
        self.say(ctx, format!("Back in the chorus. {s}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.chorus.finished || self.ending.is_some() || event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
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
        if action.is_some() {
            self.home_asked = false;
        }
        let evs = match action {
            Some(ParlorAction::Left) => self.chorus.turn(-1),
            Some(ParlorAction::Right) => self.chorus.turn(1),
            Some(ParlorAction::Primary) => self.chorus.pick(),
            Some(ParlorAction::Up | ParlorAction::Down) => {
                let k = |a| ctx.control_name(a);
                let line = format!(
                    "You stand still to listen. {} and {} turn, {} picks.",
                    sentence_start(&k(ParlorAction::Left)),
                    k(ParlorAction::Right),
                    k(ParlorAction::Primary)
                );
                self.say(ctx, line);
                Vec::new()
            }
            Some(ParlorAction::Status) => {
                let s = self.status();
                self.say(ctx, s);
                Vec::new()
            }
            Some(ParlorAction::Repeat) => {
                self.chorus.play_solo();
                let line = match self.chorus.target_species() {
                    Some(t) => format!("The {}'s song, alone.", t.name),
                    None => self.last.clone(),
                };
                ctx.say(line);
                Vec::new()
            }
            Some(ParlorAction::Hint) => {
                let h = self.hint();
                self.say(ctx, h);
                Vec::new()
            }
            Some(ParlorAction::Secondary) => Vec::new(),
            None => Vec::new(),
        };
        ctx.audio
            .set_listener(Vec3::ZERO, self.chorus.facing as f32 * 45.0);
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
        let evs = self.chorus.update(dt);
        self.voice(ctx, evs);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let c = &self.chorus;
        let mut out = vec![self.name()];
        out.push(format!("Found: {} of {}", c.found.len(), c.total));
        out.push(format!("Facing: {}", compass(c.facing)));
        if let Some(t) = c.target_species() {
            out.push(format!("Find: {}", t.name));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        format!("{} {}", self.target_line(), Self::controls(ctx))
            .trim()
            .to_string()
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_chorusscreen;
