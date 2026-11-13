//! The play screen: the head of the dragon in the parade, the drum, the
//! pearl, and the results when the street ends.

use hearken::text::{end_sentence, sentence_start};
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::words::streak_sentence;
use parlor_core::{
    challenges, Challenge, Challenges, Parlor, ParlorAction, Period, Records, ScoreResult,
};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::festival;
use crate::progress::Progress;
use crate::rules::{self, Dance, Event, Grade, Move, Street, Thing, STAR_PERCENT, WARN};
use crate::sound::{self, at_lane, coming_pos, pearl_pos, shop_pos};
use crate::streets::{self, STREETS};
use crate::{HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const DANCE_NAME: &str = "dragon_dance_street";

/// What is being danced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// A street of the main parade, from 1.
    Street(usize),
    Practice,
    /// The New Year parade of a lunar year.
    NewYear(i32),
    /// The Lantern walk of a lunar year.
    Lantern(i32),
    Challenge(Period),
}

impl Mode {
    pub fn street(self) -> Street {
        match self {
            Mode::Street(n) => streets::street(n),
            Mode::Practice => streets::PRACTICE,
            Mode::NewYear(_) => streets::NEW_YEAR,
            Mode::Lantern(_) => streets::LANTERN_WALK,
            Mode::Challenge(period) => challenge::street(period),
        }
    }

    /// The records board its scores go on, if any.
    pub fn board(self) -> Option<String> {
        match self {
            Mode::Street(n) => Some(format!("street {n}")),
            Mode::NewYear(_) => Some("new year".to_string()),
            Mode::Lantern(_) => Some("lantern walk".to_string()),
            Mode::Practice | Mode::Challenge(_) => None,
        }
    }

    /// "Street 3: Silk Street", "The New Year parade".
    pub fn title(self) -> String {
        match self {
            Mode::Street(n) => format!("Street {n}: {}", streets::street(n).name),
            Mode::Practice => "The practice street".to_string(),
            Mode::NewYear(year) => format!(
                "The New Year parade, {}",
                festival::year_of(parlor_core::calendar::zodiac(year))
            ),
            Mode::Lantern(_) => "The Lantern walk".to_string(),
            Mode::Challenge(period) => challenge::name(period).to_string(),
        }
    }

    pub fn period(self) -> Option<Period> {
        match self {
            Mode::Challenge(p) => Some(p),
            _ => None,
        }
    }
}

/// A street to start: what and with which seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Launch {
    pub mode: Mode,
    pub seed: u64,
}

impl Launch {
    pub fn new(mode: Mode) -> Launch {
        Launch {
            mode,
            seed: rules::run_seed(),
        }
    }
}

/// Opens the dance for `launch` on top of the screen there now.
pub fn start(ctx: &mut Ctx<Parlor>, launch: Launch) {
    ctx.push(DanceScreen::new(launch));
}

pub struct DanceScreen {
    pub dance: Dance,
    pub launch: Launch,
    challenge: Option<Challenge>,
    progress: Progress,
    callouts: bool,
    pearl_loop: Option<Sound>,
    loops: Vec<(usize, Sound)>,
    last: String,
    done: bool,
    started: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// Run by the tutorial: the street starts over instead of ending, so
    /// the lesson never reaches the results.
    pub lesson: bool,
    /// Lane changes this street, so the tutorial can ask for one.
    pub steps: u32,
    /// A shop's cheer has been said in words; later cheers are sound only.
    cheer_said: bool,
}

impl DanceScreen {
    pub fn new(launch: Launch) -> DanceScreen {
        DanceScreen {
            status_key: Default::default(),
            lesson: false,
            steps: 0,
            cheer_said: false,
            dance: Dance::new(launch.mode.street(), launch.seed),
            launch,
            challenge: None,
            progress: Progress::default(),
            callouts: true,
            pearl_loop: None,
            loops: Vec::new(),
            last: String::new(),
            done: false,
            started: false,
        }
    }

    /// The current `period` challenge's street, from its seed.
    pub fn challenge(ch: Challenge) -> DanceScreen {
        let launch = Launch {
            mode: Mode::Challenge(ch.period),
            seed: ch.seed,
        };
        let mut screen = DanceScreen::new(launch);
        screen.challenge = Some(ch);
        screen
    }

    fn lanes(&self) -> usize {
        self.dance.lanes()
    }

    /// Says `text` now, and keeps it for the repeat key.
    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        ctx.say(text.clone());
        self.last = text;
    }

    /// Says `text` after what is speaking, and keeps it for the repeat key.
    fn tell_queued(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        ctx.say_queued(text.clone());
        self.last = text;
    }

    /// The first time only: says an explanation and remembers it was heard.
    fn teach(&mut self, ctx: &mut Ctx<Parlor>, what: &str, text: String) -> bool {
        if self.progress.teach(what) {
            self.progress.store(ctx);
            self.tell(ctx, text);
            true
        } else {
            false
        }
    }

    fn key(ctx: &Ctx<Parlor>, action: ParlorAction) -> String {
        ctx.control_name(action)
    }

    fn intro(&self) -> String {
        let street = self.dance.street;
        let mut text = format!(
            "{}. {}. Follow the pearl. The drums count in 8 beats.",
            self.launch.mode.title(),
            sentence_start(&street.describe())
        );
        if self.dance.run_up {
            text.push_str(" Lanterns are up: red envelopes count double.");
        }
        if self.dance.forgiving {
            text.push_str(" Practice: nothing is saved, and stumbles cost nothing.");
        }
        text
    }

    fn controls(ctx: &Ctx<Parlor>) -> String {
        let l = Self::key(ctx, ParlorAction::Left);
        let r = Self::key(ctx, ParlorAction::Right);
        let p = Self::key(ctx, ParlorAction::Primary);
        let t = Self::key(ctx, ParlorAction::Secondary);
        let s = Self::key(ctx, ParlorAction::Status);
        let u = Self::key(ctx, ParlorAction::Up);
        let rep = Self::key(ctx, ParlorAction::Repeat);
        let h = Self::key(ctx, ParlorAction::Hint);
        format!(
            "{} and {r} change lanes. {p} rears up on a gong. {t} finds the pearl, {u} what is coming, {s} the status, twice for more, {rep} repeats, {h} a hint. F1 helps, Escape pauses.",
            sentence_start(&l)
        )
    }

    fn pearl_line(&self) -> String {
        if self.dance.lane == self.dance.pearl {
            format!("Pearl, lane {}, here.", self.dance.pearl + 1)
        } else {
            format!(
                "Pearl, lane {}, {}.",
                self.dance.pearl + 1,
                rules::relative(self.dance.lane, self.dance.pearl)
            )
        }
    }

    fn ahead_line(&self) -> String {
        let mut parts: Vec<String> = self
            .dance
            .incoming()
            .iter()
            .map(Dance::incoming_words)
            .collect();
        if let Some(n) = self.dance.pearl_moves_in() {
            parts.push(format!("the pearl moves in {}", rules::beats_words(n)));
        }
        if let Some(n) = self.dance.next_gong().filter(|&n| n <= 8) {
            parts.push(format!("a gong in {}", rules::beats_words(n)));
        }
        if parts.is_empty() {
            "Nothing coming. Follow the pearl.".to_string()
        } else {
            format!("Coming: {}.", parts.join(", "))
        }
    }

    fn dragon_line(&self) -> String {
        let d = &self.dance;
        let caught = if d.street.calm() {
            match d.lanterns.len() {
                1 => "1 lantern caught".to_string(),
                n => format!("{n} lanterns caught"),
            }
        } else {
            match d.envelopes {
                1 => "1 red envelope caught".to_string(),
                n => format!("{n} red envelopes caught"),
            }
        };
        format!(
            "Your dragon is {} segments long, {caught}. Combo {}, best {}.",
            d.segments, d.combo, d.best_combo
        )
    }

    fn hint(&self, ctx: &Ctx<Parlor>) -> String {
        let d = &self.dance;
        let l = Self::key(ctx, ParlorAction::Left);
        let r = Self::key(ctx, ParlorAction::Right);
        let p = Self::key(ctx, ParlorAction::Primary);
        let way = |to: usize| if to < d.lane { l.clone() } else { r.clone() };
        if d.counting_in() {
            return format!(
                "Counting in. The pearl is in lane {}, {}.",
                d.pearl + 1,
                rules::relative(d.lane, d.pearl)
            );
        }
        let incoming = d.incoming();
        if let Some(f) = incoming
            .iter()
            .find(|i| i.thing == Thing::Firecracker && i.lane == d.lane)
        {
            let to = d.pearl;
            if to != d.lane {
                return format!(
                    "Firecrackers in your lane! Step to the pearl in lane {}: {} {}.",
                    to + 1,
                    way(to),
                    rules::relative(d.lane, to)
                );
            }
            return format!(
                "Firecrackers in lane {}, your lane. Step aside, then come back.",
                f.lane + 1
            );
        }
        if let Some(n) = d.next_gong().filter(|&n| n <= 2) {
            return format!(
                "A gong in {}. {} as it strikes.",
                rules::beats_words(n),
                sentence_start(&p)
            );
        }
        if d.lane != d.pearl {
            return format!(
                "The pearl is in lane {}, {}. {} moves you toward it.",
                d.pearl + 1,
                rules::relative(d.lane, d.pearl),
                sentence_start(&way(d.pearl))
            );
        }
        if let Some(e) = incoming
            .iter()
            .find(|i| i.thing != Thing::Firecracker && i.lane != d.lane)
        {
            return format!(
                "A {} in lane {}. Catch it, then back to the pearl.",
                e.thing.name(),
                e.lane + 1
            );
        }
        "On the pearl. Stay with it.".to_string()
    }

    fn stop_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        for (_, s) in self.loops.drain(..) {
            ctx.audio.stop(s, 60);
        }
        if let Some(s) = self.pearl_loop.take() {
            ctx.audio.stop(s, 200);
        }
    }

    fn sync_loops(&mut self, ctx: &mut Ctx<Parlor>) {
        let lanes = self.lanes();
        let live = self.dance.incoming();
        let mut kept = Vec::new();
        for (id, s) in self.loops.drain(..) {
            if live.iter().any(|i| i.id == id) {
                kept.push((id, s));
            } else {
                ctx.audio.stop(s, 60);
            }
        }
        self.loops = kept;
        for item in &live {
            let pos = coming_pos(item.lane, lanes, item.beats);
            let s = match self.loops.iter().find(|(id, _)| *id == item.id) {
                Some((_, s)) => *s,
                None => {
                    let key = match item.thing {
                        Thing::Envelope => sound::ENVELOPE,
                        Thing::Lantern => sound::LANTERN,
                        Thing::Firecracker => sound::CRACKLE,
                    };
                    let s = ctx.audio.start_loop(key, Some(pos), Bus::EFFECTS);
                    self.loops.push((item.id, s));
                    s
                }
            };
            ctx.audio.set_position(s, Some(pos));
            let near = 1.0 - (item.beats / WARN as f64).clamp(0.0, 1.0) as f32;
            ctx.audio.set_volume(s, 0.45 + 0.55 * near);
        }
        if let Some(s) = self.pearl_loop {
            ctx.audio
                .set_position(s, Some(pearl_pos(self.dance.pearl, lanes)));
        }
        sound::listen_from(ctx, self.dance.lane, lanes);
    }

    fn on_event(&mut self, ctx: &mut Ctx<Parlor>, event: Event) {
        let lanes = self.lanes();
        match event {
            Event::CountIn { .. } => {
                sound::play(ctx, sound::COUNT);
            }
            Event::Start => {
                self.started = true;
                let line = self.pearl_line();
                // Queued, so the intro is heard to its end.
                if self.progress.teach("pearl") {
                    self.progress.store(ctx);
                    self.tell_queued(
                        ctx,
                        format!("Dance. Keep your head in the pearl's lane. {line}"),
                    );
                } else {
                    self.tell_queued(ctx, "Dance.");
                }
            }
            Event::Beat { beat, down } => {
                sound::play(ctx, if down { sound::DRUM_DOWN } else { sound::DRUM });
                if self.dance.run_up && beat % 2 == 1 {
                    ctx.audio.play(sound::LION, Bus::EFFECTS);
                }
            }
            Event::PearlMoved { to, .. } => {
                sound::play_at(ctx, sound::PEARL_MOVE, pearl_pos(to, lanes));
                let line = self.pearl_line();
                if !self.teach(ctx, "pearl-move", format!("The pearl moved. {line}"))
                    && self.callouts
                {
                    self.tell(ctx, line);
                }
            }
            Event::Warn { thing, lane, .. } => {
                let taught = match thing {
                    Thing::Envelope => self.teach(
                        ctx,
                        "envelope",
                        format!(
                            "A red envelope, lane {}. Be in its lane to catch it.",
                            lane + 1
                        ),
                    ),
                    Thing::Lantern => self.teach(
                        ctx,
                        "lantern",
                        format!("A lantern, lane {}. Be in its lane to catch it.", lane + 1),
                    ),
                    Thing::Firecracker => self.teach(
                        ctx,
                        "firecracker",
                        format!("Firecrackers, lane {}. Keep out of that lane.", lane + 1),
                    ),
                };
                if !taught && self.callouts {
                    let text = match thing {
                        Thing::Envelope => format!("Envelope, lane {}.", lane + 1),
                        Thing::Lantern => format!("Lantern, lane {}.", lane + 1),
                        Thing::Firecracker => format!("Firecrackers, lane {}.", lane + 1),
                    };
                    if thing == Thing::Firecracker {
                        self.tell(ctx, text);
                    } else {
                        self.tell_queued(ctx, text);
                    }
                }
            }
            Event::Roll { .. } => {
                sound::play(ctx, sound::ROLL);
                let p = sentence_start(&Self::key(ctx, ParlorAction::Primary));
                self.teach(
                    ctx,
                    "gong",
                    format!("A gong in 2 beats. {p} on the gong rears up."),
                );
            }
            Event::Gong { great } => {
                sound::play(
                    ctx,
                    if great {
                        sound::GREAT_GONG
                    } else {
                        sound::GONG
                    },
                );
            }
            Event::Midnight => {
                ctx.audio.set_bed(None);
                ctx.audio.play_music(Some(sound::MIDNIGHT), false);
                let animal = parlor_core::calendar::zodiac(match self.launch.mode {
                    Mode::NewYear(y) => y,
                    _ => festival::lunar_year(challenges::today(ctx)),
                });
                self.tell(
                    ctx,
                    format!(
                        "Midnight. Happy New Year, and welcome to {}.",
                        festival::year_of(animal)
                    ),
                );
            }
            Event::Rear {
                grade,
                points,
                great,
            } => {
                let key = match grade {
                    Grade::Perfect => sound::REAR_PERFECT,
                    Grade::Good => sound::REAR_GOOD,
                };
                sound::play(ctx, key);
                let word = match grade {
                    Grade::Perfect => "Perfect rear",
                    Grade::Good => "Good rear",
                };
                let tail = if great { ", for the new year" } else { "" };
                self.tell(ctx, format!("{word}, {points} points{tail}."));
            }
            Event::GongMissed { .. } => {
                sound::play(ctx, sound::GONG_MISSED);
                if self.callouts {
                    ctx.say_queued("Gong missed.");
                }
            }
            Event::Stray { broke } => {
                sound::play(ctx, sound::REAR_STRAY);
                if broke {
                    self.tell(ctx, "No gong. The dance breaks step: combo lost.");
                } else {
                    self.tell(ctx, "No gong to rear on.");
                }
            }
            Event::Envelope {
                lane,
                points,
                segments,
            } => {
                sound::play_at(ctx, sound::ENVELOPE_GET, at_lane(lane, lanes));
                self.tell(
                    ctx,
                    format!("Red envelope, {points} points. Dragon {segments} segments long."),
                );
            }
            Event::EnvelopeMissed { lane } => {
                sound::play_at(ctx, sound::ENVELOPE_MISS, at_lane(lane, lanes));
            }
            Event::Lantern { lane, kind, points } => {
                sound::play_at(ctx, sound::LANTERN_GET, at_lane(lane, lanes));
                let new = !self.progress.lanterns.contains(&kind);
                let name = festival::lantern_name(kind);
                let text = if new {
                    format!("A {name}, new for your collection. {points} points.")
                } else {
                    format!("A {name}, {points} points.")
                };
                self.tell(ctx, text);
            }
            Event::LanternMissed { lane, .. } => {
                sound::play_at(ctx, sound::ENVELOPE_MISS, at_lane(lane, lanes));
            }
            Event::Stumble { lane, lost } => {
                sound::play_at(ctx, sound::STUMBLE, at_lane(lane, lanes));
                let text = if self.dance.forgiving {
                    "Firecrackers! Free in practice.".to_string()
                } else {
                    format!("Firecrackers! {lost} points lost.")
                };
                self.tell(ctx, text);
            }
            Event::Crackled { lane } => {
                sound::play_at(ctx, sound::BURST, at_lane(lane, lanes));
            }
            Event::Shop {
                side, name, cheer, ..
            } => {
                let pos = shop_pos(side == rules::Side::Left, lanes);
                sound::play_at(ctx, if cheer { sound::CHEER } else { sound::CLAP }, pos);
                if self.dance.run_up {
                    sound::play_at(ctx, sound::LANTERNS_UP, pos);
                }
                if self.callouts && cheer && !self.cheer_said {
                    self.cheer_said = true;
                    let text = if self.dance.run_up {
                        format!(
                            "{} cheers under its lanterns, {}.",
                            sentence_start(name),
                            side.words()
                        )
                    } else {
                        format!("{} cheers, {}.", sentence_start(name), side.words())
                    };
                    ctx.say_queued(text);
                }
            }
            Event::Combo { combo, bonus } => {
                sound::play(ctx, sound::COMBO);
                self.tell_queued(ctx, format!("Combo {combo}, {bonus} points."));
            }
            Event::LostPearl { combo } => {
                sound::play(ctx, sound::OFF_PEARL);
                let line = self.pearl_line();
                // The pearl's lane once: not again if it was just said.
                let text = if self.last.ends_with(&line) {
                    format!("Off the pearl, combo {combo} lost.")
                } else {
                    format!("Off the pearl, combo {combo} lost. {line}")
                };
                self.tell(ctx, text);
            }
            Event::Countdown { n } => {
                sound::play(ctx, sound::COUNTDOWN);
                ctx.say(format!("{n}."));
            }
            Event::Finished => {}
        }
    }

    /// Saves the street, plays its ending and opens the results.
    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.done = true;
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
        let d = &self.dance;
        let mode = self.launch.mode;
        let stars = d.stars();
        let best = d.best_score();
        let mut progress = Progress::load(ctx);
        progress.taught = self.progress.taught.clone();
        let mut lines: Vec<String> = Vec::new();
        let mut opened: Option<usize> = None;
        let headline = match mode {
            Mode::Street(n) => {
                let stars_words = stars_words(stars);
                format!("Street {n} danced: {stars_words}, {} points", d.score)
            }
            Mode::Practice => format!("Practice street danced: {} points", d.score),
            Mode::NewYear(_) => format!("The New Year parade is danced: {} points", d.score),
            Mode::Lantern(_) => format!("The Lantern walk is done: {} points", d.score),
            Mode::Challenge(period) => {
                format!("{} danced: {} points", challenge::name(period), d.score)
            }
        };
        let mut first = end_sentence(&headline);
        if mode != Mode::Practice {
            progress.danced += 1;
            progress.longest = progress.longest.max(d.segments);
            progress.envelopes += d.envelopes;
        }
        if let Mode::Street(n) = mode {
            let was_open = progress.opened(n + 1);
            let old = progress.set_stars(n, stars);
            if stars > old && old > 0 {
                first.push_str(&format!(" Up from {} on this street.", stars_words(old)));
            }
            if !was_open && progress.opened(n + 1) && n < STREETS.len() {
                opened = Some(n + 1);
            }
        }
        if let Some(board) = mode.board() {
            let r = Records::record(ctx, ID, &board, d.score);
            first.push_str(&format!(" {}", score_line(r)));
        } else if mode == Mode::Practice {
            first.push_str(" Practice scores are not saved.");
        }
        lines.push(first);
        let mut second = vec![format!(
            "On the pearl for {} of {} beats. Best combo {}.",
            d.follow_beats, d.beats_played, d.best_combo
        )];
        if d.street.calm() {
            second.push(match d.lanterns.len() {
                1 => "1 lantern caught.".to_string(),
                n => format!("{n} lanterns caught."),
            });
        } else {
            second.push(format!(
                "{} caught, the dragon {} segments long.",
                sentence_start(&match d.envelopes {
                    1 => "1 red envelope".to_string(),
                    n => format!("{n} red envelopes"),
                }),
                d.segments
            ));
            second.push(format!(
                "Gongs: {} perfect and {} good, of {}.",
                d.perfect, d.good, d.gongs
            ));
            second.push(match d.stumbles {
                0 => "No stumbles.".to_string(),
                1 => "1 stumble.".to_string(),
                n => format!("{n} stumbles."),
            });
        }
        lines.push(second.join(" "));
        match mode {
            Mode::Street(n) => {
                if let Some(next) = opened {
                    lines.push(format!(
                        "Street {next} opens: {}.",
                        streets::street(next).name
                    ));
                } else if stars == 0 {
                    let need = best * STAR_PERCENT[0] / 100;
                    let mut line = format!(
                        "A star needs {need} points here, {} percent of the best this street allows.",
                        STAR_PERCENT[0]
                    );
                    if n < STREETS.len() {
                        line.push_str(&format!(" 1 star opens street {}.", n + 1));
                    }
                    lines.push(line);
                } else if stars < 3 {
                    let need = best * STAR_PERCENT[stars as usize] / 100;
                    lines.push(format!("The next star needs {need} points."));
                } else if n == STREETS.len() && progress.total_stars() == 3 * STREETS.len() as u32 {
                    lines.push("Every street of the parade has 3 stars.".to_string());
                }
            }
            Mode::NewYear(year) => {
                let animal = parlor_core::calendar::zodiac(year);
                progress.parades.insert(year);
                if progress.add_keepsake(animal, year) {
                    ctx.audio.play(sound::KEEPSAKE, Bus::EFFECTS);
                    lines.push(format!(
                        "Keepsake for {}: {}. {} of 12 animals collected.",
                        festival::year_of(animal),
                        festival::keepsake(animal),
                        progress.keepsakes.len()
                    ));
                } else {
                    lines.push(format!(
                        "You already hold the keepsake for {}: {}.",
                        festival::year_of(animal),
                        festival::keepsake(animal)
                    ));
                }
            }
            Mode::Lantern(year) => {
                progress.walks.insert(year);
                let before = progress.lanterns.len();
                progress.lanterns.extend(d.lanterns.iter().copied());
                let new = progress.lanterns.len() - before;
                lines.push(format!(
                    "{} new to your collection: {} of {} lanterns found.",
                    match new {
                        1 => "1 lantern".to_string(),
                        n => format!("{n} lanterns"),
                    },
                    progress.lanterns.len(),
                    festival::LANTERNS.len()
                ));
            }
            _ => {}
        }
        progress.store(ctx);
        if let Some(ch) = self.challenge.clone() {
            let r = Challenges::submit(ctx, &ch, d.score);
            let streak = Challenges::streak(ctx, ID, ch.period).max(1);
            lines.push(format!(
                "{} result: {} points. {} {} {}",
                ch.title(),
                d.score,
                challenge_line(r),
                streak_sentence(streak, ch.period.unit()),
                ch.resets()
            ));
        }
        if !matches!(mode, Mode::NewYear(_)) {
            let key = if stars > 0 || d.street.calm() {
                sound::CLEAR
            } else {
                sound::FINISH
            };
            ctx.audio.play_music(Some(key), false);
        }
        let results = results(self.launch, stars, opened, headline, lines);
        ctx.replace(results);
    }
}

/// "no stars", "1 star", "3 stars".
pub fn stars_words(n: u8) -> String {
    match n {
        0 => "no stars".to_string(),
        1 => "1 star".to_string(),
        n => format!("{n} stars"),
    }
}

/// What saving a score meant.
pub fn score_line(r: ScoreResult) -> String {
    match r {
        ScoreResult::First => "Your first score here.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

fn challenge_line(r: ScoreResult) -> String {
    match r {
        ScoreResult::First => "Your first result on this challenge.".to_string(),
        ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// After a street: the next street when one opened, the same again, the
/// results again and back.
pub fn results(
    launch: Launch,
    stars: u8,
    opened: Option<usize>,
    headline: String,
    lines: Vec<String>,
) -> Results {
    let back = match launch.mode.period() {
        Some(_) => Back::Challenges(CHALLENGES_NAME),
        None => Back::Title(TITLE_NAME),
    };
    let mut r = Results::new(headline, lines, back);
    if let Some(next) = opened {
        r = r.next(
            format!("Next street: street {next}, {}", streets::street(next).name),
            format!("{}.", sentence_start(&streets::street(next).describe())),
            move |ctx| ctx.replace(DanceScreen::new(Launch::new(Mode::Street(next)))),
        );
    }
    let (label, help) = match launch.mode {
        Mode::Challenge(period) => (
            labels::challenge_again(period),
            "The same street again. Only your best result is kept.".to_string(),
        ),
        Mode::Street(_) if stars == 0 => (
            labels::TRY_AGAIN.to_string(),
            "The same street, with a new plan of envelopes and firecrackers.".to_string(),
        ),
        _ => (
            labels::PLAY_AGAIN.to_string(),
            "The same street, with a new plan of envelopes and firecrackers.".to_string(),
        ),
    };
    r.again(label, help, move |ctx| {
        let screen = match launch.mode {
            Mode::Challenge(period) => DanceScreen::challenge(Challenge::current(ctx, ID, period)),
            mode => DanceScreen::new(Launch::new(mode)),
        };
        ctx.replace(screen)
    })
}

impl State<Parlor> for DanceScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        DANCE_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.progress = Progress::load(ctx);
        self.callouts = !self.progress.quiet || self.launch.mode == Mode::Practice;
        self.dance.forgiving = self.launch.mode == Mode::Practice;
        let today = challenges::today(ctx);
        self.dance.run_up = !self.dance.street.calm() && festival::run_up(today);
        let lanes = self.lanes();
        sound::street_beds(ctx, self.dance.street.calm());
        sound::listen_from(ctx, self.dance.lane, lanes);
        self.pearl_loop = Some(ctx.audio.start_loop(
            sound::PEARL,
            Some(pearl_pos(self.dance.pearl, lanes)),
            Bus::EFFECTS,
        ));
        let intro = self.intro();
        self.tell(ctx, intro);
        if let Mode::NewYear(year) = self.launch.mode {
            sound::play(ctx, sound::GREETING);
            let line = festival::greeting(parlor_core::calendar::zodiac(year)).to_string();
            self.tell_queued(ctx, line);
        } else {
            ctx.say_queued(Self::controls(ctx));
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_loops(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.done {
            return;
        }
        // Settings, from the pause menu, may have changed the call-outs. The
        // run's own copy of the progress keeps the new value when it saves.
        self.progress.quiet = Progress::load(ctx).quiet;
        self.callouts = !self.progress.quiet || self.launch.mode == Mode::Practice;
        let status = self.dance.status_layers().remove(0);
        ctx.say(format!("Back in the parade. {status}"));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.done || event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            let leave = match self.launch.mode.period() {
                Some(_) => Leave::Challenge(CHALLENGES_NAME),
                None => Leave::Title(TITLE_NAME),
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let lanes = self.lanes();
        match action {
            Some(ParlorAction::Left | ParlorAction::Right) => {
                let delta = if action == Some(ParlorAction::Left) {
                    -1
                } else {
                    1
                };
                match self.dance.step(delta) {
                    Move::Moved { lane } => {
                        self.steps += 1;
                        sound::play(ctx, sound::STEP);
                        sound::listen_from(ctx, lane, lanes);
                        if lane == self.dance.pearl {
                            sound::play_at(ctx, sound::PEARL_HERE, pearl_pos(lane, lanes));
                            ctx.say(format!("Lane {}, the pearl.", lane + 1));
                        } else {
                            ctx.say(format!("Lane {}.", lane + 1));
                        }
                    }
                    Move::Edge { left } => {
                        sound::play(ctx, sound::EDGE);
                        let side = if left { "left" } else { "right" };
                        ctx.say(format!(
                            "The {side} edge of the street. Lane {}.",
                            self.dance.lane + 1
                        ));
                    }
                }
            }
            Some(ParlorAction::Primary) => {
                self.dance.rear();
                for e in self.dance.take_events() {
                    self.on_event(ctx, e);
                }
            }
            Some(ParlorAction::Secondary) => {
                sound::play_at(ctx, sound::PEARL_MOVE, pearl_pos(self.dance.pearl, lanes));
                let line = self.pearl_line();
                ctx.say(line);
            }
            Some(ParlorAction::Status) => {
                let layers = self.dance.status_layers();
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                if self.last.is_empty() {
                    ctx.say("Nothing to repeat yet.");
                } else {
                    let text = self.last.clone();
                    ctx.say(text);
                }
            }
            Some(ParlorAction::Hint) => {
                let text = self.hint(ctx);
                ctx.say(text);
            }
            Some(ParlorAction::Up) => {
                let text = self.ahead_line();
                ctx.say(text);
            }
            Some(ParlorAction::Down) => {
                let text = self.dragon_line();
                ctx.say(text);
            }
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.done {
            return;
        }
        self.dance.update(dt);
        for e in self.dance.take_events() {
            self.on_event(ctx, e);
        }
        if self.dance.finished && self.lesson {
            self.dance = Dance::new(self.dance.street, self.launch.seed);
            ctx.say("Back to the start of the street.");
        } else if self.dance.finished {
            self.finish(ctx);
            return;
        }
        self.sync_loops(ctx);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let d = &self.dance;
        let mut out = vec![
            format!("Dragon Dance: {}", self.launch.mode.title()),
            format!(
                "lane {} of {}, pearl {}, beat {}",
                d.lane + 1,
                d.lanes(),
                d.pearl + 1,
                d.beat()
            ),
            format!(
                "score {}, combo {}, segments {}",
                d.score, d.combo, d.segments
            ),
        ];
        for item in d.incoming() {
            out.push(format!(
                "coming: {} lane {} {:.2}",
                item.thing.name(),
                item.lane + 1,
                item.beats
            ));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut now = format!(
            "{} Keep your head in the pearl's lane on every beat. Catch red envelopes, keep out of firecrackers, and rear up on each gong.",
            self.dance.status()
        );
        if let Some(period) = self.launch.mode.period() {
            now.push_str(&format!(
                " This is the {} challenge. Leaving from the pause menu does not count it.",
                period.label().to_lowercase()
            ));
        }
        format!("{now} {}", Self::controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_dancescreen;
