//! Lighting the Long Hall: the play screen.

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, State, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, ScoreResult};

use crate::challenge::{self, Plan, CHALLENGES_NAME};
use crate::progress::{self, Progress, Saved};
use crate::rules::{
    alcove_words, lamps_words, points_words, run_seed, time_words, where_words, Alcove, Ev, Hall,
    Layout, Side, PRACTICE, ROUND, TAPER_LOW,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "lamplighter_play";

/// Seconds between the pulses of the lamps that are out.
pub const PULSE: f64 = 1.6;
/// Seconds after a step before the lamps nearby pulse.
pub const PULSE_AFTER_STEP: f64 = 0.35;
/// How many steps away a lamp that is out can be heard.
pub const HEAR: i32 = 4;
/// Seconds from the last lamp to the results, so the chime is heard and a
/// key still held does not skip the first page.
pub const END_PAUSE: f64 = 2.0;

/// What kind of hall the screen is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The evening round, with a taper.
    Round,
    /// The practice hall: short, with no taper.
    Practice,
    /// A challenge hall.
    Challenge(Period),
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Round => "Evening round",
            Mode::Practice => "Practice hall",
            Mode::Challenge(period) => challenge::name(period),
        }
    }
}

pub struct Round {
    pub hall: Hall,
    pub mode: Mode,
    challenge: Option<Challenge>,
    /// Seconds until the lamps that are out pulse again.
    pulse: f64,
    /// Which footstep plays next.
    left_foot: bool,
    /// Seconds until the results, once the hall is lit.
    ending: Option<f64>,
    /// In the tutorial: a lit hall reads no results.
    in_lesson: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl Round {
    pub fn new(mode: Mode) -> Round {
        Round::with_seed(mode, run_seed())
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Round {
        let mut hall = match mode {
            Mode::Practice => Hall::new(PRACTICE, seed),
            _ => Hall::new(ROUND, seed),
        };
        if !hall.timed() {
            hall.start();
        }
        Round {
            hall,
            mode,
            challenge: None,
            pulse: PULSE_AFTER_STEP,
            left_foot: true,
            ending: None,
            in_lesson: false,
            status_key: Default::default(),
        }
    }

    /// The tutorial's hall: short, no taper, and no results at the end.
    pub fn lesson_hall(layout: Layout, seed: u64) -> Round {
        let mut r = Round::with_seed(Mode::Practice, seed);
        r.hall = Hall::new(layout, seed);
        r.hall.start();
        r.in_lesson = true;
        r
    }

    /// A round of the challenge's hall.
    pub fn challenge(ch: Challenge) -> Round {
        let plan = Plan::for_challenge(&ch);
        let mut r = Round::with_seed(Mode::Challenge(ch.period), 0);
        r.hall = plan.hall();
        r.challenge = Some(ch);
        r
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// What to do before the taper is lit.
    fn light_prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} lights your taper and starts the clock.",
            sentence_start(&Self::key(ctx, ParlorAction::Primary))
        )
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let h = &self.hall;
        let what = format!(
            "{} out along {} steps.",
            sentence_start(&lamps_words(h.out())),
            h.length()
        );
        if h.started() {
            format!("{}. {what} No taper, so take your time.", self.mode.name())
        } else {
            format!(
                "{}. {what} The taper burns for {}. {}",
                self.mode.name(),
                time_words(h.taper_secs().unwrap_or(0)),
                self.light_prompt(ctx)
            )
        }
    }

    /// Every control, in a few sentences.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "{} and {} walk the hall, toward the far wall and back toward the lobby door. {} and {} reach to that wall and light a lamp that is out. {} speaks the status, {} says what is at this step, {} finds the nearest lamp out, and Escape pauses.",
            sentence_start(&k(ParlorAction::Up)),
            k(ParlorAction::Down),
            sentence_start(&k(ParlorAction::Left)),
            k(ParlorAction::Right),
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Repeat),
            k(ParlorAction::Hint)
        )
    }

    /// S in two layers: the step, lamps out and the taper first; then
    /// the hall's length with the score, or what starts the clock.
    pub fn status_layers(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let h = &self.hall;
        // Lamps first, and words, so it is not heard as "step 1 of 3".
        let first = format!("{} out, at step {}", lamps_words(h.out()), h.pos + 1);
        let length = format!("Step {} of {}", h.pos + 1, h.length());
        if !h.started() {
            return vec![
                format!("{first}, taper not lit."),
                format!("{length}. {}", self.light_prompt(ctx)),
            ];
        }
        match h.taper_secs() {
            Some(t) => vec![
                format!("{first}, taper {} left.", time_words(t)),
                format!("{length}, score {}.", h.score),
            ],
            None => vec![format!("{first}, no taper."), format!("{length}.")],
        }
    }

    /// What is at the player's step, left wall then right.
    pub fn here_words(&self) -> String {
        let [l, r] = self.hall.here();
        format!(
            "Step {} of {}. Left: {}. Right: {}.",
            self.hall.pos + 1,
            self.hall.length(),
            alcove_words(l),
            alcove_words(r)
        )
    }

    /// The nearest lamp out, in words.
    pub fn hint_words(&self) -> String {
        match self.hall.nearest_out() {
            Some((ahead, side)) => {
                format!("Nearest lamp out: {}.", where_words(ahead, side))
            }
            None => "Every lamp is burning.".to_string(),
        }
    }

    /// The lamps that are out within hearing, each from its wall.
    fn pulse_lamps(&self, ctx: &mut Ctx<Parlor>) {
        let pos = self.hall.pos as i32;
        for (step, side) in self.hall.out_lamps() {
            let ahead = step as i32 - pos;
            if ahead.abs() <= HEAR {
                sound::play_at(ctx, sound::LAMP_OUT, ahead, side);
            }
        }
    }

    /// The burning lamps at this step crackle softly from their walls.
    fn pass_burning(&self, ctx: &mut Ctx<Parlor>) {
        for side in Side::BOTH {
            if self.hall.alcove(self.hall.pos, side) == Alcove::Burning {
                sound::play_at_vol(ctx, sound::LAMP_BURNING, 0, side, sound::BURNING_VOLUME);
            }
        }
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        // A draught in the same moment as a light changes the count, so the
        // count waits for the draught's line.
        let lit = evs.iter().any(|e| matches!(e, Ev::Lit { .. }));
        let draught = evs.iter().any(|e| matches!(e, Ev::Draught { .. }));
        for ev in evs {
            match ev {
                Ev::Step { to } => {
                    let key = if self.left_foot {
                        sound::STEP_A
                    } else {
                        sound::STEP_B
                    };
                    self.left_foot = !self.left_foot;
                    sound::play(ctx, key);
                    self.pass_burning(ctx);
                    self.pulse = PULSE_AFTER_STEP;
                    ctx.say(format!("{}.", to + 1));
                }
                Ev::LobbyDoor => {
                    ctx.audio.play_at(
                        sound::LOBBY_DOOR,
                        Vec3::new(0.0, -sound::STEP, 0.0),
                        hearken::Bus::EFFECTS,
                    );
                    ctx.say("The lobby door. The hall runs ahead of you.");
                }
                Ev::FarWall => {
                    ctx.audio.play_at(
                        sound::FAR_WALL,
                        Vec3::new(0.0, sound::STEP, 0.0),
                        hearken::Bus::EFFECTS,
                    );
                    ctx.say("The far wall. The hall runs back behind you.");
                }
                Ev::Lit { side } => {
                    sound::play_at(ctx, sound::LIGHT, 0, side);
                    let left = self.hall.out();
                    if draught {
                        ctx.say("Lit.");
                    } else if left > 0 {
                        ctx.say(format!("Lit. {left} to go."));
                    }
                }
                Ev::Bare { side } => {
                    sound::play_at(ctx, sound::BARE_WALL, 0, side);
                    ctx.say(format!("Bare wall on your {}.", side.word()));
                }
                Ev::AlreadyBurning { side } => {
                    sound::play_at(ctx, sound::ALREADY_BURNING, 0, side);
                    ctx.say(format!("Already burning on your {}.", side.word()));
                }
                Ev::TaperLow => {
                    sound::play(ctx, sound::TAPER_LOW);
                    ctx.say(format!("Taper low. {} left.", time_words(TAPER_LOW as u32)));
                }
                Ev::TaperOut => {
                    sound::play(ctx, sound::TAPER_OUT);
                    sound::play(ctx, sound::NEW_TAPER);
                    ctx.say("Taper out. Mabel lights a fresh one. Taper bonus gone.");
                }
                Ev::Draught { step, side } => {
                    sound::play(ctx, sound::DRAUGHT);
                    let ahead = step as i32 - self.hall.pos as i32;
                    let count = if lit {
                        format!(" {} to go.", self.hall.out())
                    } else {
                        String::new()
                    };
                    ctx.say_queued(format!(
                        "A draught. A lamp went out, {}.{count}",
                        where_words(ahead, side)
                    ));
                }
                Ev::HallLit { .. } => {
                    sound::play(ctx, sound::HALL_LIT);
                    ctx.say("Every lamp is burning.");
                    self.ending = Some(END_PAUSE);
                }
                Ev::NotStarted => {
                    let p = self.light_prompt(ctx);
                    ctx.say(format!("Your taper is not lit. {p}"));
                }
            }
        }
    }

    fn light_taper(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hall.start();
        sound::play(ctx, sound::NEW_TAPER);
        self.pulse = PULSE_AFTER_STEP;
        ctx.say("Taper lit. Listen for the lamps that are out.");
    }

    /// The hall is lit: record it and read the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.hall);
            let summary = challenge::summary(&ch, &self.hall);
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let (summary, pages) = if self.mode == Mode::Practice {
            progress::save_lamps(ctx, self.hall.lit, true);
            (
                "Practice hall lit.".to_string(),
                vec![format!(
                    "Practice hall lit. {} lit.",
                    sentence_start(&lamps_words(self.hall.lit as usize))
                )],
            )
        } else {
            let saved = progress::save_round(ctx, &self.hall);
            (result_summary(&self.hall), result_pages(&self.hall, &saved))
        };
        let mut results = Some(after_round(self.mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// Keys a player presses for their screen reader or as a modifier, which
/// the hall leaves alone.
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

/// "Hall lit. 8 lamps in 1 minute 40 seconds."
pub fn lit_line(hall: &Hall) -> String {
    let tapers = match hall.tapers {
        0 => String::new(),
        1 => " Mabel lit you 1 fresh taper.".to_string(),
        n => format!(" Mabel lit you {n} fresh tapers."),
    };
    // Relights after a draught are counted apart from the lamps that were out.
    let relit = match hall.relit().min(hall.lit) {
        0 => String::new(),
        n => format!(" and {n} relit"),
    };
    format!(
        "Hall lit. {}{relit} in {}.{tapers}",
        lamps_words((hall.lit - hall.relit().min(hall.lit)) as usize),
        time_words(progress::secs(hall))
    )
}

/// The one-line headline of an evening round, the results menu's title.
pub fn result_summary(hall: &Hall) -> String {
    sentence_start(&points_words(hall.score))
}

/// The pages after an evening round.
pub fn result_pages(hall: &Hall, saved: &Saved) -> Vec<String> {
    let lamps = hall.score - hall.bonus;
    let parts = if hall.bonus > 0 {
        format!(
            "{} for the lamps and a taper bonus of {}.",
            sentence_start(&points_words(lamps)),
            hall.bonus
        )
    } else {
        "No taper bonus: it went with the first taper.".to_string()
    };
    let score = match saved.score {
        ScoreResult::First => format!("{}, your first score.", points_words(hall.score)),
        ScoreResult::NewBest { previous } => format!(
            "{}, a new best, beating {previous}.",
            points_words(hall.score)
        ),
        ScoreResult::Short { best } => {
            format!("{}. Your best is {best}.", points_words(hall.score))
        }
    };
    let t = progress::secs(hall);
    let fastest = match saved.fastest_record {
        Some(None) => format!("Fastest hall: {}, your first.", time_words(t)),
        Some(Some(prev)) => format!(
            "Fastest hall: {}, a new record, beating {}.",
            time_words(t),
            time_words(prev)
        ),
        None => format!("Fastest hall: {}.", time_words(saved.fastest.unwrap_or(t))),
    };
    vec![
        lit_line(hall),
        format!("{parts} {} {fastest}", sentence_start(&score)),
    ]
}

/// The results menu after an evening round or a practice hall.
pub fn after_round(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let help = match mode {
        Mode::Practice => "Another practice hall, with its lamps in new places.",
        _ => "Another evening round, with its lamps in new places.",
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        help,
        move |ctx| ctx.replace(Round::new(mode)),
    )
}

/// The results menu after a challenge hall.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same hall, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Round::challenge(ch))
        },
    )
}

impl State<Parlor> for Round {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::hall_bed(ctx);
        let intro = self.intro(ctx);
        ctx.say(intro);
        let new_here = !Progress::load(ctx).any();
        if new_here || self.mode == Mode::Practice {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.hall.finished() {
            return;
        }
        sound::hall_bed(ctx);
        let here = self.here_words();
        ctx.say(format!("Back in the hall. {here}"));
        self.pulse = PULSE_AFTER_STEP;
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.hall.finished() || event.is_repeat() {
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
        let evs = match action {
            Some(ParlorAction::Primary) if !self.hall.started() => {
                self.light_taper(ctx);
                return;
            }
            Some(ParlorAction::Primary) => {
                ctx.say("Taper lit. Reach left or right.");
                return;
            }
            Some(ParlorAction::Up) => self.hall.walk(true),
            Some(ParlorAction::Down) => self.hall.walk(false),
            Some(ParlorAction::Left) => self.hall.reach(Side::Left),
            Some(ParlorAction::Right) => self.hall.reach(Side::Right),
            Some(ParlorAction::Secondary) => {
                ctx.say("Nothing to hold in the hall.");
                return;
            }
            Some(ParlorAction::Status) => {
                let layers = self.status_layers(ctx);
                self.status_key.speak(ctx, layers);
                return;
            }
            Some(ParlorAction::Repeat) => {
                let here = self.here_words();
                ctx.say(here);
                return;
            }
            Some(ParlorAction::Hint) => {
                if let Some((ahead, side)) = self.hall.nearest_out() {
                    sound::play_at(ctx, sound::LAMP_OUT, ahead, side);
                }
                let hint = self.hint_words();
                ctx.say(hint);
                return;
            }
            None => {
                if let InputEvent::KeyDown { key, .. } = event {
                    if !is_quiet_key(*key) {
                        ctx.play_ui(hearken::UiSound::Error);
                        ctx.say("Not a Lamplighter key. F1 lists the controls.");
                    }
                }
                return;
            }
        };
        self.voice(ctx, evs);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                self.ending = None;
                if !self.in_lesson {
                    self.finish(ctx);
                }
            }
            return;
        }
        if !self.hall.started() {
            return;
        }
        self.pulse -= dt;
        if self.pulse <= 0.0 {
            self.pulse = PULSE;
            self.pulse_lamps(ctx);
        }
        let evs = self.hall.update(dt);
        self.voice(ctx, evs);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let h = &self.hall;
        let mut out = vec![
            self.mode.name().to_string(),
            format!("Step: {} of {}", h.pos + 1, h.length()),
            format!("Lamps out: {}", h.out()),
        ];
        if let Some(t) = h.taper_secs() {
            out.push(format!("Taper: {t} seconds"));
            out.push(format!("Score: {}", h.score));
        }
        let [l, r] = h.here();
        out.push(format!("Left: {}", alcove_words(l)));
        out.push(format!("Right: {}", alcove_words(r)));
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let now = if !self.hall.started() {
            self.light_prompt(ctx)
        } else {
            self.hint_words()
        };
        let mut text = format!("{now} {}", self.controls(ctx));
        if let Some(ch) = &self.challenge {
            text.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count the hall.",
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

mod visuals_round;
