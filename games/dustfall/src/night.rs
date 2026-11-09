//! The night: the gun on its rail, the raiders coming down, and the beats
//! that tell the player what each one did.
//!
//! The rules run the raiders' whole phase at once. The screen turns those
//! events into beats, each a sound where it happened and a short line, and
//! plays them one at a time so the player can follow.

use std::collections::VecDeque;

use hearken::text::sentence_start;
use hearken::visuals::{Scene, GOLD, PAPER, RED, TEAL};
use hearken::{Bus, Ctx, InputEvent, Key, State, Vec3};
use parlor_core::menus::{self, labels, Back, Results};
use parlor_core::{
    Challenge, Challenges, Parlor, ParlorAction, Period, Records, ScoreResult, StatusKey,
};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::colony::{notices, refusal_words, ColonyScreen, Mode, Shared, Watch};
use crate::progress::{nights_phrase, Progress, NIGHTS_BOARD, SCORE_BOARD};
use crate::rules::grid::{column_name, Work, COLS, NEAR_ROW, ROWS};
use crate::rules::nights::RELIEF_NIGHT;
use crate::rules::raiders::State as RaiderState;
use crate::rules::{
    Building, Event, Hitter, Input, Kind, Phase, Raider, Refusal, Run, Sector, Weapon,
    CLEAN_NIGHT_POINTS,
};
use crate::sound;
use crate::{HELP, ID, TITLE_NAME};

pub const NIGHT_NAME: &str = "dustfall_night";

/// About how long a line takes to say.
pub fn speech_secs(text: &str) -> f64 {
    0.3 + text.chars().count() as f64 / 16.0
}

/// "a hulk", "the carrier".
pub fn a(kind: Kind) -> String {
    match kind {
        Kind::Carrier => "the carrier".to_string(),
        kind => format!("a {}", kind.name()),
    }
}

/// "1 hit", "3 hits".
fn hits_phrase(n: u8) -> String {
    if n == 1 {
        "1 hit".to_string()
    } else {
        format!("{n} hits")
    }
}

/// "1 action", "2 actions".
pub fn actions_phrase(n: u8) -> String {
    if n == 1 {
        "1 action".to_string()
    } else {
        format!("{n} actions")
    }
}

/// "Hulk, right, row 4, 3 hits, charging at the centre."
pub fn raider_line(raider: &Raider) -> String {
    let state = match raider.state {
        RaiderState::Approaching if raider.kind.crosses() => ", crossing".to_string(),
        RaiderState::Approaching => String::new(),
        RaiderState::Charging { col, .. } => format!(", charging at the {}", column_name(col)),
        RaiderState::Bashing { .. } => ", bashing a wall".to_string(),
        RaiderState::Holding => ", holding".to_string(),
    };
    format!("{}, {}{state}.", raider.words(), hits_phrase(raider.hits))
}

/// One thing that happened, as the player hears it.
struct Beat {
    sounds: Vec<(&'static str, Option<Vec3>)>,
    line: Option<String>,
    /// Seconds before the next beat, at least.
    hold: f64,
    /// Plays the moment it is made: the player's own action.
    now: bool,
    /// Holds the player's actions until it has played: the raiders' phase.
    /// The night's opening and the turn chime do not, so an eager player
    /// can act over them.
    blocking: bool,
}

impl Beat {
    fn new(hold: f64) -> Beat {
        Beat {
            sounds: Vec::new(),
            line: None,
            hold,
            now: false,
            blocking: true,
        }
    }

    fn open(mut self) -> Beat {
        self.blocking = false;
        self
    }

    fn at(mut self, key: &'static str, at: Sector) -> Beat {
        self.sounds.push((key, Some(sound::sector_pos(at))));
        self
    }

    fn under(mut self, key: &'static str, col: u8) -> Beat {
        self.sounds.push((key, Some(sound::column_pos(col))));
        self
    }

    fn plain(mut self, key: &'static str) -> Beat {
        self.sounds.push((key, None));
        self
    }

    fn say(mut self, line: impl Into<String>) -> Beat {
        self.line = Some(line.into());
        self
    }

    fn now(mut self) -> Beat {
        self.now = true;
        self
    }

    fn play(&self, ctx: &mut Ctx<Parlor>) {
        for (key, pos) in &self.sounds {
            match pos {
                Some(pos) => ctx.audio.play_at(key, *pos, Bus::EFFECTS),
                None => ctx.audio.play(key, Bus::EFFECTS),
            };
        }
    }

    fn secs(&self) -> f64 {
        self.line
            .as_deref()
            .map_or(self.hold, |l| self.hold.max(speech_secs(l)))
    }
}

pub struct NightScreen {
    run: Shared,
    queue: VecDeque<Beat>,
    wait: f64,
    /// The raider the scan rests on.
    pub scanned: Option<u32>,
    last: String,
    status_key: StatusKey,
    call_outs: bool,
    confirm_end: bool,
    left_to_right: bool,
    /// Enter was pressed once with actions left.
    confirming: bool,
    /// The player has been told the raiders are moving.
    told_wait: bool,
    /// The dome as the last beat knew it, to hear a breach reach it.
    dome_seen: u32,
    /// What the night left for the colony phase.
    notices: Vec<String>,
    ended: bool,
    /// For the tutorial: the controls tried.
    pub status_asked: bool,
    pub hint_asked: bool,
    pub turns_ended: u32,
}

impl NightScreen {
    pub fn new(run: Shared) -> NightScreen {
        let dome = run.borrow().colony.dome;
        NightScreen {
            run,
            queue: VecDeque::new(),
            wait: 0.0,
            scanned: None,
            last: String::new(),
            status_key: StatusKey::new(),
            call_outs: true,
            confirm_end: false,
            left_to_right: false,
            confirming: false,
            told_wait: false,
            dome_seen: dome,
            notices: Vec::new(),
            ended: false,
            status_asked: false,
            hint_asked: false,
            turns_ended: 0,
        }
    }

    pub fn run(&self) -> std::cell::Ref<'_, Watch> {
        self.run.borrow()
    }

    fn mode(&self) -> Mode {
        self.run.borrow().mode
    }

    /// The tutorial's night never leaves for the colony or the results.
    fn practice(&self) -> bool {
        self.mode() == Mode::Practice
    }

    /// Whether the raiders' beats are still playing.
    pub fn busy(&self) -> bool {
        self.queue.iter().any(|b| b.blocking)
    }

    /// Drops beats the player need not wait for, such as the arrivals,
    /// when the player acts over them.
    fn flush(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.queue.is_empty() {
            return;
        }
        self.queue.clear();
        self.wait = 0.0;
        ctx.stop_speech();
    }

    fn settings(&mut self, ctx: &mut Ctx<Parlor>) {
        let p = Progress::load(ctx);
        self.call_outs = p.call_outs;
        self.confirm_end = p.confirm_end;
        self.left_to_right = p.scan_left_to_right;
    }

    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        self.last = text.clone();
        ctx.say(text);
    }

    // --- events to beats ---------------------------------------------------

    /// Turns the run's events into beats. The player's own beats play at
    /// once; the raiders' queue up.
    fn drain(&mut self, ctx: &mut Ctx<Parlor>) {
        let events = self.run.borrow_mut().take_events();
        let mut first = true;
        for event in events {
            let Some(beat) = self.beat(&event) else {
                continue;
            };
            if beat.now && self.queue.is_empty() {
                beat.play(ctx);
                if let Some(line) = &beat.line {
                    self.last = line.clone();
                    if first {
                        ctx.say(line.clone());
                    } else {
                        ctx.say_queued(line.clone());
                    }
                    first = false;
                }
            } else {
                self.queue.push_back(beat);
            }
        }
    }

    fn beat(&mut self, event: &Event) -> Option<Beat> {
        let run = self.run.borrow();
        let beat = match event {
            Event::NightOpens { night, arrivals } => {
                let mut beats = vec![Beat::new(0.5).open().say(format!(
                    "Night {night}. {}.",
                    sentence_start(&run.forecast_words())
                ))];
                for (_, kind, at) in arrivals {
                    let mut b = Beat::new(0.5).open().at(sound::ARRIVE, *at);
                    if self.call_outs {
                        let crossing = if kind.crosses() { ", crossing" } else { "" };
                        b = b.say(format!(
                            "{} at the {}{crossing}.",
                            sentence_start(&a(*kind)),
                            column_name(at.col)
                        ));
                    }
                    beats.push(b);
                }
                drop(run);
                let last = beats.pop();
                self.queue.extend(beats);
                return last;
            }
            Event::YourTurn { turn, actions } => {
                let mut line = String::new();
                for (_, kind, col) in run.charges() {
                    line.push_str(&format!(
                        "{} is charging at the {}. ",
                        sentence_start(&a(kind)),
                        column_name(col)
                    ));
                }
                if *turn == 1 || run.colony.hungry() {
                    line.push_str(&format!("Your turn, {}.", actions_phrase(*actions)));
                } else {
                    line.push_str("Your turn.");
                }
                Beat::new(0.4).open().plain(sound::TURN).say(line)
            }
            Event::Moved { col } => {
                let in_column = run
                    .nearest_first()
                    .into_iter()
                    .find(|r| r.at.col == *col)
                    .map(|r| format!(", {} at row {}", a(r.kind), r.at.row))
                    .unwrap_or_default();
                Beat::new(0.3)
                    .under(sound::GUN_RAIL, *col)
                    .say(format!("{}{in_column}.", sentence_start(column_name(*col))))
                    .now()
            }
            Event::DryShot { col } => Beat::new(0.5)
                .under(sound::GUN_FIRE, *col)
                .say(format!("Nothing at the {}.", column_name(*col)))
                .now(),
            Event::Hit {
                kind, at, left, by, ..
            } => {
                let (beat, word) = hitter(Beat::new(0.7), *by, *at);
                beat.at(sound::GUN_HIT, *at).say(format!(
                    "{word}. {}, {} left.",
                    kind.title(),
                    left
                ))
            }
            Event::Down {
                kind, at, take, by, ..
            } => {
                let (beat, word) = hitter(Beat::new(1.0), *by, *at);
                let salvage = take.words();
                let salvage = if salvage.is_empty() {
                    String::new()
                } else {
                    format!(" {}.", sentence_start(&salvage))
                };
                let line = if *by == Hitter::Gun {
                    format!("{} down.{salvage}", kind.title())
                } else {
                    format!("{word}. {} down.{salvage}", kind.title())
                };
                beat.at(sound::GUN_DOWN, *at).say(line)
            }
            Event::TurnEnded { charged } => {
                let line = if *charged > 0 {
                    format!("Turn over. Shield up {charged}, to {}.", run.colony.shield)
                } else {
                    "Turn over.".to_string()
                };
                Beat::new(0.6).plain(sound::TURN_OVER).say(line).now()
            }
            Event::Shot {
                kind,
                weapon,
                col,
                dodged,
                shield,
                dome,
                ..
            } => {
                self.dome_seen = *dome;
                if *dodged {
                    Beat::new(0.8).under(sound::CHARGE_PASS, *col).say(format!(
                        "{} fires at the {}. Dodged.",
                        kind.title(),
                        column_name(*col)
                    ))
                } else {
                    let what = if *weapon == Weapon::Siege {
                        "Siege shot hits".to_string()
                    } else {
                        format!("{} hits", kind.title())
                    };
                    Beat::new(1.0)
                        .under(sound::CHARGE_HIT, *col)
                        .say(format!("{what}. Shield {shield}, dome {dome}."))
                }
            }
            Event::Breach {
                kind,
                col,
                shield,
                dome,
                darkened,
                ..
            } => {
                let key = if *dome < self.dome_seen {
                    sound::BREACH_DOME
                } else {
                    sound::BREACH_SHIELD
                };
                self.dome_seen = *dome;
                let mut beat = Beat::new(1.2).under(key, *col);
                let mut line = format!(
                    "Breach at the {}: {}. Shield {shield}, dome {dome}.",
                    column_name(*col),
                    a(*kind)
                );
                if let Some(building) = darkened {
                    beat = beat.under(sound::BREACH_BUILDING, *col);
                    line.push_str(&format!(" {} dark.", building.title()));
                }
                beat.say(line)
            }
            Event::Stepped { kind, to, .. } => {
                let mut beat = Beat::new(0.45).at(sound::raider(*kind), *to);
                if to.row == NEAR_ROW {
                    beat = beat.say(format!(
                        "{} at the dome, {}.",
                        kind.title(),
                        column_name(to.col)
                    ));
                } else if self.call_outs {
                    beat = beat.say(format!("{}, {}.", kind.title(), to.words()));
                }
                beat
            }
            Event::Bashed {
                kind, wall, down, ..
            } => {
                if *down {
                    Beat::new(0.8)
                        .at(sound::WALL_DOWN, *wall)
                        .say(format!("Wall at {} down.", wall.words()))
                } else {
                    Beat::new(0.6).at(sound::WALL_CRACK, *wall).say(format!(
                        "{} bashes the wall at {}.",
                        kind.title(),
                        wall.words()
                    ))
                }
            }
            Event::Charging {
                kind, weapon, col, ..
            } => {
                let line = if *weapon == Weapon::Siege {
                    format!(
                        "{} winds up a siege shot at the {}. It cannot be dodged.",
                        kind.title(),
                        column_name(*col)
                    )
                } else {
                    format!("{} charging at the {}.", kind.title(), column_name(*col))
                };
                Beat::new(0.8).under(sound::CHARGE_WIND, *col).say(line)
            }
            Event::Dropped { at, .. } => Beat::new(0.6)
                .at(sound::ARRIVE, *at)
                .say(format!("A skitter drops at {}.", at.words())),
            Event::Left { kind, .. } => Beat::new(0.6)
                .plain(sound::LEAVE)
                .say(format!("The {} leaves.", kind.name())),
            Event::Held { night, take, clean } => {
                let mut line = format!("Night {night} held.");
                let salvage = take.words();
                if !salvage.is_empty() {
                    line.push_str(&format!(" Salvage: {salvage}."));
                }
                if *clean {
                    line.push_str(&format!(" Clean night, {CLEAN_NIGHT_POINTS} bonus."));
                }
                line.push_str(&format!(" Score {}.", run.score));
                Beat::new(1.5).plain(sound::HELD).say(line)
            }
            Event::Fallen { .. } => Beat::new(1.5)
                .plain(sound::BREACH_DOME)
                .say("The dome falls."),
            Event::Relief { .. } => Beat::new(0.5).say("Relief has landed."),
            Event::ColonyOpens { .. } => {
                let lines = notices(event, &run);
                drop(run);
                self.notices = lines;
                return None;
            }
            Event::Bought { .. }
            | Event::Placed { .. }
            | Event::DomeRepaired { .. }
            | Event::BuildingRepaired { .. }
            | Event::MissileMade { .. } => return None,
        };
        Some(beat)
    }

    // --- the player's actions ----------------------------------------------

    /// Applies an input, or says why not.
    fn act(&mut self, ctx: &mut Ctx<Parlor>, input: Input) {
        if self.busy() {
            self.wait_please(ctx);
            return;
        }
        self.confirming = false;
        let outcome = self.run.borrow_mut().apply(input);
        match outcome {
            Ok(()) => {
                self.flush(ctx);
                ctx.stop_speech();
                self.drain(ctx);
            }
            Err(refusal) => {
                let text = match refusal {
                    Refusal::NoTarget => "No raiders to hit.".to_string(),
                    Refusal::Edge => {
                        format!(
                            "The rail ends at the {}.",
                            column_name(self.run.borrow().gun)
                        )
                    }
                    other => refusal_words(other, &self.run.borrow()),
                };
                self.tell(ctx, text);
            }
        }
    }

    fn wait_please(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.told_wait {
            self.told_wait = true;
            ctx.say("The raiders are moving.");
        }
    }

    fn end_turn(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.busy() {
            self.wait_please(ctx);
            return;
        }
        let actions = match self.run.borrow().phase {
            Phase::Turn { actions } => actions,
            _ => 0,
        };
        if self.confirm_end && !self.confirming {
            self.confirming = true;
            let text = format!(
                "End the turn with {} left? Enter again ends it.",
                actions_phrase(actions)
            );
            self.tell(ctx, text);
            return;
        }
        self.turns_ended += 1;
        self.act(ctx, Input::EndTurn);
    }

    /// The raiders in scan order.
    fn order(&self) -> Vec<u32> {
        let run = self.run.borrow();
        if self.left_to_right {
            let mut raiders: Vec<&Raider> = run.raiders.iter().collect();
            raiders.sort_by_key(|r| (r.at.col, r.at.row, r.id));
            raiders.iter().map(|r| r.id).collect()
        } else {
            run.nearest_first().iter().map(|r| r.id).collect()
        }
    }

    fn scan(&mut self, ctx: &mut Ctx<Parlor>, forward: bool) {
        let order = self.order();
        if order.is_empty() {
            self.scanned = None;
            self.tell(ctx, "No raiders on the grid.".to_string());
            return;
        }
        let at = self
            .scanned
            .and_then(|id| order.iter().position(|o| *o == id));
        let next = match (at, forward) {
            (None, true) => 0,
            (None, false) => order.len() - 1,
            (Some(i), true) => (i + 1) % order.len(),
            (Some(i), false) => (i + order.len() - 1) % order.len(),
        };
        let id = order[next];
        self.scanned = Some(id);
        let (line, key, at) = {
            let run = self.run.borrow();
            let raider = run.raider(id).expect("a scanned raider");
            (raider_line(raider), sound::raider(raider.kind), raider.at)
        };
        sound::at(ctx, key, at);
        self.tell(ctx, line);
    }

    /// Where the scanned raider, or the nearest, is heading.
    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hint_asked = true;
        let text = {
            let run = self.run.borrow();
            let target = self
                .scanned
                .and_then(|id| run.raider(id))
                .or_else(|| run.nearest_first().first().copied());
            match target {
                None => "No raiders on the grid.".to_string(),
                Some(r) => route_words(&run, r),
            }
        };
        self.tell(ctx, text);
    }

    fn status_layers(&self) -> Vec<String> {
        let run = self.run.borrow();
        let actions = match run.phase {
            Phase::Turn { actions } => actions,
            _ => 0,
        };
        let mut first = String::new();
        for (_, kind, col) in run.charges() {
            first.push_str(&format!(
                "{} is charging at the {}. ",
                sentence_start(&a(kind)),
                column_name(col)
            ));
        }
        let raiders = run.raiders.len();
        first.push_str(&format!(
            "Gun {}. Dome {}, shield {}. {} left. {}.",
            column_name(run.gun),
            run.colony.dome,
            run.colony.shield,
            actions_phrase(actions),
            match raiders {
                0 => "No raiders".to_string(),
                1 => "1 raider".to_string(),
                n => format!("{n} raiders"),
            }
        ));
        let mut second: Vec<String> = run.nearest_first().iter().map(|r| raider_line(r)).collect();
        second.push(format!(
            "Night {}, turn {}. Missiles {}.",
            run.night, run.turn, run.colony.missiles
        ));
        vec![first, second.join(" ")]
    }

    // --- the night's end ---------------------------------------------------

    fn open_colony(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        if self.practice() {
            ctx.say_queued("The tutorial night is over.");
            return;
        }
        let notices = std::mem::take(&mut self.notices);
        let run = std::rc::Rc::clone(&self.run);
        ctx.replace(ColonyScreen::new(run).with_notices(notices));
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        let mode = self.mode();
        let run = self.run.borrow().run.clone();
        let won = run.phase == Phase::Relief;
        match mode {
            Mode::Practice => {}
            Mode::Colony => self.finish_colony(ctx, &run, won),
            Mode::Challenge(period) => self.finish_challenge(ctx, &run, won, period),
        }
    }

    fn end_music(ctx: &mut Ctx<Parlor>, won: bool) {
        ctx.audio.play_music(
            Some(if won {
                sound::RELIEF_MUSIC
            } else {
                sound::FALLEN_MUSIC
            }),
            false,
        );
    }

    /// "Dome 80, shield 20. Power 9, metal 4, food 6."
    fn colony_line(run: &Run) -> String {
        format!(
            "Dome {}, shield {}. {}.",
            run.colony.dome,
            run.colony.shield,
            run.colony.resources.words()
        )
    }

    fn finish_colony(&mut self, ctx: &mut Ctx<Parlor>, run: &Run, won: bool) {
        Self::end_music(ctx, won);
        let mut p = Progress::load(ctx);
        p.saved = None;
        if won {
            p.reliefs += 1;
        }
        p.save(ctx);
        let result = Records::record(ctx, ID, SCORE_BOARD, run.score);
        Records::record(ctx, ID, NIGHTS_BOARD, run.nights_held as u64);
        let best = match result {
            ScoreResult::First => "Your first colony.".to_string(),
            ScoreResult::NewBest { .. } => "A new best.".to_string(),
            ScoreResult::Short { best } => format!("Best {best}."),
        };
        let summary = if won {
            format!("Relief has landed. {} points. {best}", run.score)
        } else {
            format!(
                "The dome fell on night {}. {} points. {best}",
                run.night, run.score
            )
        };
        let pages = vec![
            summary.clone(),
            format!(
                "{} held of {RELIEF_NIGHT}. Raiders downed: {}.",
                sentence_start(&nights_phrase(run.nights_held)),
                run.downed
            ),
            Self::colony_line(run),
        ];
        let again = if won {
            labels::PLAY_AGAIN
        } else {
            labels::TRY_AGAIN
        };
        ctx.replace(Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
            again,
            "A new colony, from night 1.",
            |ctx| crate::begin(ctx, None, true),
        ));
    }

    /// A challenge's end: the score goes on its board, and the results
    /// lead back to the challenges.
    fn finish_challenge(&mut self, ctx: &mut Ctx<Parlor>, run: &Run, won: bool, period: Period) {
        Self::end_music(ctx, won);
        let ch = Challenge::current(ctx, ID, period);
        let result = Challenges::submit(ctx, &ch, run.score);
        let best = match result {
            ScoreResult::First => "Your first result on this challenge.".to_string(),
            ScoreResult::NewBest { previous } => format!("A new best, beating {previous}."),
            ScoreResult::Short { best } => format!("Your best is {best}."),
        };
        let name = challenge::name(ctx, period);
        let summary = if won {
            format!("{name} held. {} points. {best}", run.score)
        } else {
            format!(
                "The dome fell on night {}. {} points. {best}",
                run.night, run.score
            )
        };
        let streak = Challenges::streak(ctx, ID, period).max(1);
        let held = if run.nights() == 1 {
            format!("Raiders downed: {}.", run.downed)
        } else {
            format!(
                "{} of {} held. Raiders downed: {}.",
                sentence_start(&nights_phrase(run.nights_held)),
                nights_phrase(run.nights()),
                run.downed
            )
        };
        let pages = vec![
            summary.clone(),
            held,
            Self::colony_line(run),
            format!(
                "{} result: {} points. A {streak} {} streak. {}",
                ch.title(),
                run.score,
                period.unit(),
                ch.resets()
            ),
        ];
        ctx.replace(
            Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
                labels::challenge_again(period),
                "The same sky, straight away. Only your best result is kept.",
                move |ctx| challenge::start(ctx, period, true),
            ),
        );
    }
}

/// The sounds of what hit a raider, and the word for it.
fn hitter(beat: Beat, by: Hitter, at: Sector) -> (Beat, &'static str) {
    match by {
        Hitter::Gun => (beat.under(sound::GUN_FIRE, at.col), "Hit"),
        Hitter::Missile => (beat.plain(sound::MISSILE), "Missile hit"),
        Hitter::Mine => (beat.at(sound::MINE, at), "Mine"),
        Hitter::Flak(post) => (beat.at(sound::FLAK, post), "Flak"),
    }
}

/// The route finder speaking: the next step, or with a scanner the whole
/// route and when it lands.
pub fn route_words(run: &Run, raider: &Raider) -> String {
    match raider.state {
        RaiderState::Charging { col, .. } => {
            return format!(
                "{} is charging at the {}. Move the gun off that column, or fire first.",
                sentence_start(&a(raider.kind)),
                column_name(col)
            );
        }
        RaiderState::Holding => {
            return format!(
                "The {} holds at {}, {}. Only a missile reaches it.",
                raider.kind.name(),
                raider.at.words(),
                hits_phrase(raider.hits)
            );
        }
        RaiderState::Bashing { wall } => {
            return format!(
                "{} is bashing the wall at {}.",
                sentence_start(&a(raider.kind)),
                wall.words()
            );
        }
        RaiderState::Approaching => {}
    }
    if raider.kind.crosses() {
        let way = if raider.heading < 0 { "left" } else { "right" };
        return format!(
            "The {} crosses row {} toward the {way}, {}.",
            raider.kind.name(),
            raider.at.row,
            hits_phrase(raider.hits)
        );
    }
    let route = run.route_of(raider.id).unwrap_or_default();
    let Some(next) = route.first() else {
        return format!(
            "{} is at the dome, {}. It breaches next phase.",
            sentence_start(&a(raider.kind)),
            column_name(raider.at.col)
        );
    };
    if run.colony.has(Building::Scanner) {
        let steps: Vec<String> = route.iter().map(|s| s.words()).collect();
        let phases = run.phases_to_land(raider.id).unwrap_or(0);
        format!(
            "{}: {}. Lands in {}.",
            raider.kind.title(),
            steps.join("; "),
            if phases == 1 {
                "1 phase".to_string()
            } else {
                format!("{phases} phases")
            }
        )
    } else {
        format!("{} heads to {}.", raider.kind.title(), next.words())
    }
}

impl State<Parlor> for NightScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut scene = Scene::from_lines(self.lines(ctx));
        let run = self.run.borrow();
        // The grid fills the drawing area: the horizon at the top, the
        // dome along the bottom.
        let (x0, y0, cell_w, cell_h) = (100.0, 30.0, 160.0, 80.0);
        let x_of = |col: u8| x0 + f32::from(col) * cell_w;
        let y_of = |row: u8| y0 + f32::from(ROWS - row) * cell_h;
        for col in 0..COLS {
            for row in NEAR_ROW..=ROWS {
                scene.line(
                    (x_of(col), y_of(row)),
                    (x_of(col) + cell_w, y_of(row)),
                    1.0,
                    PAPER,
                );
                scene.line(
                    (x_of(col), y_of(row)),
                    (x_of(col), y_of(row) + cell_h),
                    1.0,
                    PAPER,
                );
            }
        }
        let right = x_of(COLS);
        let bottom = y_of(NEAR_ROW) + cell_h;
        scene.line((right, y_of(ROWS)), (right, bottom), 1.0, PAPER);
        scene.line((x0, bottom), (right, bottom), 1.0, PAPER);
        for (at, work) in run.grid.works() {
            let (x, y) = (x_of(at.col), y_of(at.row));
            match work {
                Work::Wall { .. } => scene.rect(x + 10.0, y + 30.0, cell_w - 20.0, 20.0, PAPER),
                Work::Flak { .. } => scene.circle(x + cell_w / 2.0, y + cell_h / 2.0, 14.0, TEAL),
                Work::Mine => scene.circle(x + cell_w / 2.0, y + cell_h / 2.0, 8.0, GOLD),
            }
        }
        for raider in &run.raiders {
            let (x, y) = (
                x_of(raider.at.col) + cell_w / 2.0,
                y_of(raider.at.row) + cell_h / 2.0,
            );
            scene.circle(x, y, 12.0 + 4.0 * f32::from(raider.hits), RED);
            scene.label(
                x - 6.0,
                y - 10.0,
                raider.kind.title().chars().take(1).collect::<String>(),
                PAPER,
            );
        }
        // The dome under the grid, the gun on its rail.
        scene.rect(x0, bottom + 6.0, cell_w * f32::from(COLS), 24.0, TEAL);
        scene.rect(x_of(run.gun) + 50.0, bottom - 4.0, 60.0, 16.0, GOLD);
        scene.label(
            x0,
            bottom + 40.0,
            format!(
                "Night {}, turn {}. Dome {}, shield {}.",
                run.night, run.turn, run.colony.dome, run.colony.shield
            ),
            PAPER,
        );
        scene
    }

    fn name(&self) -> &'static str {
        NIGHT_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::on_watch(ctx);
        if !self.practice() {
            self.settings(ctx);
        }
        self.drain(ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.practice() {
            self.settings(ctx);
        }
        let layers = self.status_layers();
        let text = format!("Back on watch. {}", layers[0]);
        self.tell(ctx, text);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || self.ended {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() {
            match event.pressed() {
                Some(Key::Escape) if !self.practice() => {
                    let leave = self.mode().leave();
                    menus::pause(ctx, HELP, leave);
                }
                Some(Key::Return | Key::KpEnter) => self.end_turn(ctx),
                _ => {}
            }
            return;
        }
        if action != Some(ParlorAction::Repeat) {
            self.confirming = false;
        }
        match action {
            Some(ParlorAction::Left) => self.act(ctx, Input::Left),
            Some(ParlorAction::Right) => self.act(ctx, Input::Right),
            Some(ParlorAction::Primary) => self.act(ctx, Input::Fire),
            Some(ParlorAction::Secondary) => self.act(ctx, Input::Missile),
            Some(ParlorAction::Up) => self.scan(ctx, false),
            Some(ParlorAction::Down) => self.scan(ctx, true),
            Some(ParlorAction::Status) => {
                self.status_asked = true;
                let layers = self.status_layers();
                self.last = self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Hint) => self.hint(ctx),
            Some(ParlorAction::Repeat) => {
                let text = self.last.clone();
                ctx.say(text);
            }
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            return;
        }
        if let Some(front) = self.queue.front() {
            self.wait -= dt;
            if self.wait > 0.0 {
                return;
            }
            let _ = front;
            let beat = self.queue.pop_front().expect("a beat");
            beat.play(ctx);
            if let Some(line) = &beat.line {
                self.last = line.clone();
                ctx.say_queued(line.clone());
            }
            self.wait = beat.secs();
            if !self.queue.is_empty() {
                return;
            }
        }
        self.told_wait = false;
        let phase = self.run.borrow().phase;
        match phase {
            Phase::Turn { .. } => {}
            Phase::Colony => self.open_colony(ctx),
            Phase::Relief | Phase::Fallen => self.finish(ctx),
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let run = self.run.borrow();
        let (phase, actions) = match run.phase {
            Phase::Colony => ("colony", 0),
            Phase::Turn { actions } => ("turn", actions),
            Phase::Relief => ("relief", 0),
            Phase::Fallen => ("fallen", 0),
        };
        let mut lines = vec![
            format!(
                "night {}, turn {}, {phase}, gun {}, actions {actions}, queue {}",
                run.night,
                run.turn,
                run.gun,
                self.queue.len()
            ),
            format!(
                "dome {}, shield {}, missiles {}, score {}, scanned {}",
                run.colony.dome,
                run.colony.shield,
                run.colony.missiles,
                run.score,
                self.scanned.map_or("none".to_string(), |id| id.to_string())
            ),
        ];
        for r in run.nearest_first() {
            let state = match r.state {
                RaiderState::Approaching => "approaching".to_string(),
                RaiderState::Charging { col, phases } => format!("charging {col} in {phases}"),
                RaiderState::Bashing { wall } => format!("bashing {},{}", wall.col, wall.row),
                RaiderState::Holding => "holding".to_string(),
            };
            lines.push(format!(
                "raider {} {} at {},{} hits {} {state}",
                r.id,
                r.kind.name(),
                r.at.col,
                r.at.row,
                r.hits
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

/// The night's controls in a paragraph, naming the player's own keys.
pub fn controls_text(ctx: &Ctx<Parlor>) -> String {
    let left = ctx.control_name(ParlorAction::Left);
    let right = ctx.control_name(ParlorAction::Right);
    let fire = ctx.control_name(ParlorAction::Primary);
    let missile = ctx.control_name(ParlorAction::Secondary);
    let up = ctx.control_name(ParlorAction::Up);
    let down = ctx.control_name(ParlorAction::Down);
    let status = ctx.control_name(ParlorAction::Status);
    let hint = ctx.control_name(ParlorAction::Hint);
    let repeat = ctx.control_name(ParlorAction::Repeat);
    format!(
        "{} and {right} slide the gun a column, {fire} fires at the nearest raider in its column, {missile} fires a missile at the nearest raider anywhere. Each costs an action. Enter ends the turn, and every action left charges the shield. {} and {down} scan the raiders without spending anything. {} gives the gun, the dome and the shield, and again for every raider. {} says where a raider is heading. {} repeats the last line. Escape pauses.",
        sentence_start(&left),
        sentence_start(&up),
        sentence_start(&status),
        sentence_start(&hint),
        sentence_start(&repeat),
    )
}
