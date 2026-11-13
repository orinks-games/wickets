//! Bowling on lane nine: the play screen.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, ScoreResult};

use crate::challenge::{self, Plan, CHALLENGES_NAME};
use crate::lane::{best_line, Busy, Ev, Kind, Mark, Roll, Session};
use crate::progress::{self, Progress, Saved};
use crate::rules::{
    aim_words, board_words, pin_list, pinfall, points_words, release_words, run_seed, Frame, Game,
    Line, Rack, SWING_TOP,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "midnight_lanes_play";

/// Seconds from the last ball to the results, so its sounds are heard
/// and a key still held from play does not skip the first page.
pub const END_PAUSE: f64 = 1.5;

/// What kind of run the screen is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// One full game of ten frames.
    Game,
    /// A challenge.
    Challenge(Period),
    /// A full game with pumpkins for pins, around Halloween.
    Pumpkin,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Game => "Full game",
            Mode::Challenge(period) => challenge::name(period),
            Mode::Pumpkin => crate::pumpkin::NAME,
        }
    }
}

pub struct Bowl {
    pub session: Session,
    pub mode: Mode,
    challenge: Option<Challenge>,
    /// Cues already explained in words this run.
    taught: HashSet<&'static str>,
    swing: Option<Sound>,
    roll: Option<Sound>,
    /// Seconds until the results, once the last ball is scored.
    ending: Option<f64>,
    /// The last result line, for Repeat.
    last_line: Option<String>,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl Bowl {
    pub fn new(mode: Mode) -> Bowl {
        Bowl::with_seed(mode, run_seed())
    }

    pub fn with_seed(mode: Mode, seed: u64) -> Bowl {
        Bowl::from_session(mode, Session::single(seed))
    }

    pub fn from_session(mode: Mode, session: Session) -> Bowl {
        Bowl {
            status_key: Default::default(),
            session,
            mode,
            challenge: None,
            taught: HashSet::new(),
            swing: None,
            roll: None,
            ending: None,
            last_line: None,
        }
    }

    /// A run of the challenge.
    pub fn challenge(ch: Challenge) -> Bowl {
        let plan = Plan::for_challenge(&ch);
        let mut b = Bowl::from_session(Mode::Challenge(ch.period), plan.session());
        b.challenge = Some(ch);
        b
    }

    /// True the first time a cue is met this run.
    fn first(&mut self, cue: &'static str) -> bool {
        self.taught.insert(cue)
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    /// What to do with the ball in hand.
    fn roll_prompt(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} starts the swing. Press it again at the top.",
            sentence_start(&Self::key(ctx, ParlorAction::Primary))
        )
    }

    fn intro(&self, ctx: &Ctx<Parlor>) -> String {
        let what = match &self.session.kind {
            Kind::Games(1) => "Ten frames on lane nine.".to_string(),
            Kind::Games(n) => format!("{n} games on lane nine."),
            Kind::Leaves(l) => format!("{} leaves, one ball each.", l.len()),
        };
        format!(
            "{}. {what} {} {}",
            self.mode.name(),
            self.rack_line(),
            self.roll_prompt(ctx)
        )
    }

    /// The pins standing, as a sentence.
    fn rack_line(&self) -> String {
        let rack = self.session.rack;
        if rack == Rack::FULL {
            "All 10 pins standing.".to_string()
        } else {
            format!("Standing: {}.", pin_list(rack))
        }
    }

    /// Every control, in a few sentences.
    pub fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let k = |a| Self::key(ctx, a);
        format!(
            "{} and {} step one board left or right. {} turns the aim right and {} turns it left. {} starts the swing, and again lets go at the top of the swing. {} speaks the frame, pins and your line, twice for the score, {} repeats the last result, {} suggests a line, and Escape pauses.",
            sentence_start(&k(ParlorAction::Left)),
            k(ParlorAction::Right),
            sentence_start(&k(ParlorAction::Up)),
            k(ParlorAction::Down),
            sentence_start(&k(ParlorAction::Primary)),
            sentence_start(&k(ParlorAction::Status)),
            k(ParlorAction::Repeat),
            k(ParlorAction::Hint)
        )
    }

    /// The status line, in a fixed order: where in the game, the pins
    /// standing, the score, then the board and aim.
    pub fn status(&self) -> Vec<String> {
        let s = &self.session;
        let place = match (&s.kind, s.position()) {
            (_, None) => "Game over.".to_string(),
            (Kind::Leaves(l), Some((n, _))) => {
                format!("Leave {n} of {}, {} made.", l.len(), s.made)
            }
            (Kind::Games(1), Some((f, r))) => format!("Frame {f}, ball {r}."),
            (Kind::Games(n), Some((f, r))) => {
                format!("Game {} of {n}, frame {f}, ball {r}.", s.game_number())
            }
        };
        let score = match &s.kind {
            Kind::Leaves(_) => String::new(),
            Kind::Games(1) => match counting_words(&s.game) {
                Some(c) => format!("Score {}, {c} still counting.", s.game.score()),
                None => format!("Score {}.", s.game.score()),
            },
            Kind::Games(_) => match counting_words(&s.game) {
                Some(c) => format!(
                    "Score {}, {c} still counting, series {}.",
                    s.game.score(),
                    s.series_total()
                ),
                None => format!("Score {}, series {}.", s.game.score(), s.series_total()),
            },
        };
        vec![
            format!(
                "{place} {} Board {}, {}.",
                self.rack_line(),
                s.stand,
                aim_words(s.aim)
            ),
            score,
        ]
    }

    /// The suggested line for the pins standing.
    pub fn hint_line(&self) -> String {
        hint_words(self.session.rack, self.session.stand)
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.swing.take() {
            ctx.audio.stop(s, 60);
        }
        if let Some(s) = self.roll.take() {
            ctx.audio.stop(s, 120);
        }
    }

    fn result_line(&self, roll: &Roll) -> String {
        let leaves = self.session.leaves().is_some();
        let standing = roll.standing;
        let left = if standing.is_empty() {
            String::new()
        } else {
            format!(" Left standing: {}.", pin_list(standing))
        };
        let line = match roll.mark {
            Mark::Strike if self.mode == Mode::Pumpkin && roll.streak >= 2 => {
                format!("Strike. A grin carved. {} in a row.", roll.streak)
            }
            Mark::Strike if self.mode == Mode::Pumpkin => "Strike. A grin carved.".to_string(),
            Mark::Strike if roll.streak >= 2 => format!("Strike. {} in a row.", roll.streak),
            Mark::Strike => "Strike.".to_string(),
            Mark::Spare if leaves => "Spare made.".to_string(),
            Mark::Spare => "Spare.".to_string(),
            _ if roll.gutter => format!("Gutter ball. No pins.{left}"),
            _ if roll.knocked.is_empty() => format!("Missed every pin.{left}"),
            Mark::Split => format!("{} down, a split.{left}", roll.knocked.count()),
            Mark::Open => format!("{} down.{left}", roll.knocked.count()),
        };
        if leaves {
            let total = self.session.leaves().map_or(0, <[Rack]>::len);
            format!("{line} {} of {total} made.", self.session.made)
        } else {
            line
        }
    }

    fn voice(&mut self, ctx: &mut Ctx<Parlor>, evs: Vec<Ev>) {
        for ev in evs {
            match ev {
                Ev::Step(board) => {
                    sound::play_at(ctx, sound::STEP, board as f64);
                    ctx.say(board_words(board));
                }
                Ev::StepEdge(board) => {
                    ctx.play_ui(UiSound::Error);
                    ctx.say(format!("Edge of the approach. {}", board_words(board)));
                }
                Ev::Aim(aim) => {
                    let target = self.session.target();
                    sound::play_at(ctx, sound::AIM_TONE, target as f64);
                    let mut line = format!(
                        "{}, board {target} at the pins.",
                        sentence_start(&aim_words(aim))
                    );
                    if self.first("aim") {
                        line.push_str(" The tone sounds from where the ball would cross.");
                    }
                    ctx.say(line);
                }
                Ev::AimEdge(aim) => {
                    ctx.play_ui(UiSound::Error);
                    ctx.say(format!(
                        "The aim turns no further. {}.",
                        sentence_start(&aim_words(aim))
                    ));
                }
                Ev::Swing => {
                    let s = ctx.audio.start_loop(sound::SWING, None, Bus::EFFECTS);
                    ctx.audio.set_pitch(s, sound::SWING_LOW);
                    self.swing = Some(s);
                    if self.first("swing") {
                        ctx.say("Swing. Let go when the tone peaks.");
                    }
                }
                Ev::Released { error, .. } => {
                    if let Some(s) = self.swing.take() {
                        ctx.audio.stop(s, 40);
                    }
                    sound::play_at(ctx, sound::RELEASE, self.session.stand as f64);
                    let s = ctx.audio.start_loop(
                        sound::BALL_ROLL,
                        Some(sound::board_place(self.session.stand as f64)),
                        Bus::EFFECTS,
                    );
                    ctx.audio.set_volume(s, sound::ROLL_NEAR);
                    self.roll = Some(s);
                    ctx.say(release_words(error));
                }
                Ev::Gutter => {
                    let board = self.session.ball_board().unwrap_or(1.0);
                    sound::play_at(ctx, sound::GUTTER, board);
                    ctx.say("Gutter.");
                }
                Ev::Impact { down } => {
                    if let Some(s) = self.roll.take() {
                        ctx.audio.stop(s, 80);
                    }
                    let pumpkins = self.mode == Mode::Pumpkin;
                    if down >= sound::HEAVY {
                        let key = if pumpkins {
                            sound::PUMPKIN_HEAVY
                        } else {
                            sound::PIN_HEAVY
                        };
                        sound::play(ctx, key);
                    } else if down > 0 {
                        let key = if pumpkins {
                            sound::PUMPKIN_LIGHT
                        } else {
                            sound::PIN_LIGHT
                        };
                        sound::play(ctx, key);
                    }
                }
                Ev::Result(roll) => {
                    match roll.mark {
                        Mark::Strike if self.mode == Mode::Pumpkin => {
                            sound::play(ctx, sound::GRIN);
                        }
                        Mark::Strike => {
                            sound::play(ctx, sound::STRIKE);
                        }
                        Mark::Spare => {
                            sound::play(ctx, sound::SPARE);
                        }
                        _ => {}
                    }
                    let line = self.result_line(&roll);
                    ctx.say(line.clone());
                    self.last_line = Some(line);
                }
                Ev::NewRack => {
                    sound::play(ctx, sound::PINSETTER);
                }
                Ev::Ready => {
                    sound::play_at(ctx, sound::BALL_RETURN, self.session.stand as f64);
                    if self.session.leaves().is_some() {
                        let n = self.session.leave + 1;
                        ctx.say_queued(format!("Leave {n}. {}", self.rack_line()));
                    } else if self.first("ready") {
                        ctx.say_queued("Ball back. Your turn.");
                    }
                }
                Ev::GameOver { game, score } => {
                    // A single game's results open with its score: not
                    // said here as well.
                    if self.session.series() > 1 {
                        ctx.say_queued(format!("Game {game} over: {}.", points_words(score)));
                    }
                }
                Ev::Over => self.ending = Some(END_PAUSE),
                Ev::Busy(why) => {
                    let line = match why {
                        Busy::Rolling => "The ball is rolling.".to_string(),
                        Busy::Sweeping => "Ball returning.".to_string(),
                        Busy::Swinging => format!(
                            "Mid swing. {} lets go.",
                            sentence_start(&Self::key(ctx, ParlorAction::Primary))
                        ),
                        Busy::Over => "The last ball is bowled.".to_string(),
                    };
                    ctx.say(line);
                }
            }
        }
    }

    fn repeat(&mut self, ctx: &mut Ctx<Parlor>) {
        match &self.last_line {
            Some(line) => {
                let line = line.clone();
                ctx.say(line);
            }
            None => {
                let line = format!(
                    "No ball yet. {} {}",
                    self.rack_line(),
                    self.roll_prompt(ctx)
                );
                ctx.say(line);
            }
        }
    }

    /// The run is over: record it and read the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.session);
            let summary = challenge::summary(&ch, &self.session);
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let board = match self.mode {
            Mode::Pumpkin => crate::pumpkin::BOARD,
            _ => progress::GAME_BOARD,
        };
        let saved = progress::save_game(ctx, &self.session, board);
        let pages = result_pages(&self.session, &saved);
        let summary = result_summary(&self.session);
        let mut results = Some(after_game(self.mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }
}

/// Strikes or spares still waiting for their bonus balls, as words:
/// "2 strikes", "1 spare". None when every mark is scored. Both at once
/// cannot happen: the two balls after a strike always score it.
pub fn counting_words(game: &Game) -> Option<String> {
    let open: Vec<_> = game
        .frames()
        .into_iter()
        .filter(|f| f.total.is_none())
        .collect();
    let strikes = open.iter().filter(|f| f.strike()).count();
    let spares = open.iter().filter(|f| f.spare()).count();
    match (strikes, spares) {
        (0, 0) => None,
        (1, _) => Some("1 strike".to_string()),
        (n, 0) => Some(format!("{n} strikes")),
        _ => Some("1 spare".to_string()),
    }
}

/// The suggested line for `rack`, from where the bowler stands.
pub fn hint_words(rack: Rack, stand: i32) -> String {
    let (stand, aim) = best_line(rack, stand);
    if rack == Rack::FULL {
        return format!("Angle into the pocket: board {stand}, {}.", aim_words(aim));
    }
    let down = pinfall(rack, &Line::new(stand, aim, 0.0)).count();
    let all = if down == rack.count() {
        if down == 1 {
            "the pin".to_string()
        } else {
            format!("all {down}")
        }
    } else {
        format!("{} of {}", down, rack.count())
    };
    format!(
        "Try board {stand}, {}. A clean release there should take {all}.",
        aim_words(aim)
    )
}

/// A frame as the debrief reads it: "strike", "7, spare", "7 and 2".
pub fn frame_words(frame: &Frame, tenth: bool) -> String {
    // An open tenth is two balls, said like any other frame.
    let open = frame.rolls.len() == 2 && !frame.strike() && !frame.spare();
    if !tenth || open {
        return if frame.strike() {
            "strike".to_string()
        } else if frame.spare() {
            format!("{}, spare", frame.rolls[0])
        } else {
            match frame.rolls.as_slice() {
                [a, b] => format!("{a} and {b}"),
                [a] => a.to_string(),
                _ => String::new(),
            }
        };
    }
    let mut words = Vec::new();
    let mut up = 10u8;
    let mut fresh = true;
    for &r in &frame.rolls {
        if r == up {
            words.push(if fresh { "strike" } else { "spare" }.to_string());
            up = 10;
            fresh = true;
        } else {
            words.push(r.to_string());
            up -= r;
            fresh = false;
        }
    }
    words.join(", ")
}

/// The frame-by-frame debrief: "Frame 1: strike, 20."
pub fn frame_lines(game: &Game) -> Vec<String> {
    game.frames()
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let total = f.total.map(|t| format!(", {t}")).unwrap_or_default();
            format!("Frame {}: {}{total}.", i + 1, frame_words(f, i == 9))
        })
        .collect()
}

/// The one-line headline of a full game, the results menu's title.
pub fn result_summary(session: &Session) -> String {
    let score = session.games.first().map_or(0, Game::score);
    format!("Game over. {}", sentence_start(&points_words(score)))
}

/// The pages after a full game: the headline, the frames, the tallies.
pub fn result_pages(session: &Session, saved: &Saved) -> Vec<String> {
    let game = session.games.first().cloned().unwrap_or_default();
    let total = game.score();
    let record = match saved.score {
        ScoreResult::First => "your first game on the board".to_string(),
        ScoreResult::NewBest { previous } => format!("a new high game, beating {previous}"),
        ScoreResult::Short { best } => format!("your high game is {best}"),
    };
    let mut first = format!(
        "Game over. {}, {record}.",
        sentence_start(&points_words(total))
    );
    if saved.first_perfect {
        first.push_str(" A perfect game, 12 strikes in a row.");
    }
    let lines = frame_lines(&game);
    let mut pages = vec![first];
    pages.push(lines[..5.min(lines.len())].join(" "));
    if lines.len() > 5 {
        pages.push(lines[5..].join(" "));
    }
    let strikes = match session.strikes {
        1 => "1 strike".to_string(),
        n => format!("{n} strikes"),
    };
    let spares = match session.spares {
        1 => "1 spare".to_string(),
        n => format!("{n} spares"),
    };
    let run = match saved.streak_record {
        Some(prev) if prev > 0 => format!(
            " Most strikes in a row: {}, a new record, beating {prev}.",
            session.best_streak
        ),
        Some(_) => format!(
            " Most strikes in a row: {}, your first run on the board.",
            session.best_streak
        ),
        None if session.best_streak > 0 => {
            format!(" Most strikes in a row: {}.", session.best_streak)
        }
        None => String::new(),
    };
    pages.push(format!("{} and {spares}.{run}", sentence_start(&strikes)));
    pages
}

/// The results menu after a full game.
pub fn after_game(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let help = match mode {
        Mode::Pumpkin => "Another game of ten frames with pumpkins for pins.",
        _ => "Another full game of ten frames.",
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        help,
        move |ctx| ctx.replace(Bowl::new(mode)),
    )
}

/// The results menu after a challenge.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same lane, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(Bowl::challenge(ch))
        },
    )
}

impl State<Parlor> for Bowl {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        sound::alley_bed(ctx);
        let intro = self.intro(ctx);
        ctx.say(intro);
        if !menus::tutorial_read(ctx, ID) && !Progress::load(ctx).any() {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.session.over() {
            return;
        }
        sound::alley_bed(ctx);
        if let Some(s) = self.roll {
            ctx.audio.resume(s);
        }
        let line = if self.session.ready() {
            format!("Back on the approach. {}", self.roll_prompt(ctx))
        } else {
            "Back on the approach.".to_string()
        };
        ctx.say(line);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.session.over() {
            return;
        }
        if event.is_repeat() {
            // A held arrow keeps stepping or turning, one board a repeat.
            let evs = match ctx.action_for(event) {
                Some(ParlorAction::Left) => self.session.step(-1),
                Some(ParlorAction::Right) => self.session.step(1),
                Some(ParlorAction::Up) => self.session.turn(1),
                Some(ParlorAction::Down) => self.session.turn(-1),
                _ => return,
            };
            self.voice(ctx, evs);
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if self.session.cancel_swing() {
                if let Some(s) = self.swing.take() {
                    ctx.audio.stop(s, 40);
                }
            }
            if let Some(s) = self.roll {
                ctx.audio.pause(s);
            }
            let leave = if self.challenge.is_some() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let evs = match action {
            Some(ParlorAction::Left) => self.session.step(-1),
            Some(ParlorAction::Right) => self.session.step(1),
            Some(ParlorAction::Up) => self.session.turn(1),
            Some(ParlorAction::Down) => self.session.turn(-1),
            Some(ParlorAction::Primary) => self.session.primary(),
            Some(ParlorAction::Secondary) => {
                ctx.say("No spin. A ball rolls straight along your aim.");
                return;
            }
            Some(ParlorAction::Status) => {
                let layers = self.status();
                self.status_key.speak(ctx, layers);
                return;
            }
            Some(ParlorAction::Repeat) => {
                self.repeat(ctx);
                return;
            }
            Some(ParlorAction::Hint) => {
                let line = self.hint_line();
                ctx.say(line);
                return;
            }
            None => return,
        };
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
        let evs = self.session.update(dt);
        self.voice(ctx, evs);
        if let (Some(s), Some(t)) = (self.swing, self.session.swing_time()) {
            ctx.audio.set_pitch(s, sound::swing_pitch(t, SWING_TOP));
        }
        if let (Some(s), Some(p), Some(board)) = (
            self.roll,
            self.session.ball_progress(),
            self.session.ball_board(),
        ) {
            ctx.audio.set_position(s, Some(sound::board_place(board)));
            ctx.audio.set_volume(s, sound::roll_volume(p));
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let s = &self.session;
        let mut out = vec![self.mode.name().to_string()];
        match s.position() {
            Some((f, _)) if s.leaves().is_some() => out.push(format!("Leave {f}, made {}", s.made)),
            Some((f, r)) => out.push(format!("Frame {f}, ball {r}")),
            None => out.push("Game over".to_string()),
        }
        if s.leaves().is_none() {
            out.push(format!("Score: {}", s.game.score()));
        }
        out.push(format!("Standing: {}", pin_list(s.rack)));
        out.push(format!("Board {}, {}", s.stand, aim_words(s.aim)));
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let s = &self.session;
        // The controls that follow say what the primary key does.
        let now = if s.ready() {
            self.rack_line()
        } else if s.swing_time().is_some() {
            format!(
                "Mid swing. {} lets go at the top of the swing.",
                sentence_start(&Self::key(ctx, ParlorAction::Primary))
            )
        } else {
            "Ball rolling.".to_string()
        };
        let mut text = format!("{now} {}", self.controls(ctx));
        if let Some(ch) = &self.challenge {
            text.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count the run.",
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

mod visuals_bowl;
