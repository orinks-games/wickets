//! The night: the gun on its rail, the raiders coming down the lanes in
//! real time, and what the player hears of each.
//!
//! The rules tick ten times a second. Each raider has an engine looped at
//! its place in the sky, so a lane with three raiders is three sounds at
//! three distances. A lock tone pulses under the gun while a raider is in
//! its lane, faster as it nears. Everything else the rules raise as an
//! event is a sound where it happened and, when it matters, a few words.

use std::collections::HashMap;

use hearken::text::sentence_start;
use hearken::visuals::{Scene, GOLD, PAPER, RED, TEAL};
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Results};
use parlor_core::unlocks::earned_line;
use parlor_core::{
    Challenge, Challenges, Parlor, ParlorAction, Period, Records, ScoreResult, StatusKey,
};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::colony::{notices, refusal_words, ColonyScreen, Mode, Shared, Watch};
use crate::music::{Conductor, Pressure, Stab};
use crate::progress::{
    nights_phrase, Progress, LONG_WATCH_BOARD, NIGHTS_BOARD, SCORE_BOARD, STORM_BOARD,
};
use crate::rules::lanes::{lane_name, HORIZON, LANES, MID};
use crate::rules::nights::RELIEF_NIGHT;
use crate::rules::raiders::State as RaiderState;
use crate::rules::{
    Building, Event, Hitter, Input, Kind, Phase, Raider, Range, Refusal, Run, Weapon,
    CLEAN_NIGHT_POINTS, TICKS_A_SECOND,
};
use crate::{sound, unlocks, Start};
use crate::{HELP, ID, TITLE_NAME};

pub const NIGHT_NAME: &str = "dustfall_night";

/// Seconds a rules tick lasts.
const TICK_SECS: f64 = 1.0 / TICKS_A_SECOND as f64;
/// Ticks one frame may catch up at most, so a stall does not land half a
/// night at once.
const MAX_TICKS_A_FRAME: u32 = 5;
/// Seconds between lock-tone pulses with the raider at the horizon, and
/// at the dome.
const LOCK_FAR: f64 = 0.7;
const LOCK_NEAR: f64 = 0.12;
/// Seconds between one "Out of range" and the next while the gun hammers
/// a far mothership.
const GLANCE_EVERY: f64 = 3.0;

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

/// "Hulk, right, near, 3 hits, winding up at the centre."
pub fn raider_line(raider: &Raider) -> String {
    let doing = match (raider.state, raider.charge) {
        (_, Some(c)) if c.weapon == Weapon::Siege => ", winding up a siege shot".to_string(),
        (_, Some(c)) => format!(", winding up at the {}", lane_name(c.lane)),
        (RaiderState::Crossing, None) => ", crossing".to_string(),
        (RaiderState::Hovering, None) => ", holding".to_string(),
        (RaiderState::Falling, None) => String::new(),
    };
    format!("{}, {}{doing}.", raider.words(), hits_phrase(raider.hits))
}

/// Where the screen goes once the night's last line has been said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    /// The night was held: the colony phase.
    Colony,
    /// Relief or the fallen dome: the results.
    Finish,
}

pub struct NightScreen {
    run: Shared,
    /// Seconds not yet ticked.
    acc: f64,
    /// Each raider's engine loop, by id.
    engines: HashMap<u32, Sound>,
    /// Seconds until the lock tone pulses again.
    lock_wait: f64,
    /// The raider the scan rests on.
    pub scanned: Option<u32>,
    last: String,
    status_key: StatusKey,
    call_outs: bool,
    left_to_right: bool,
    /// The dome as the last event knew it, to hear a hit reach it.
    dome_seen: u32,
    /// What the night left for the colony phase.
    notices: Vec<String>,
    /// The score, following the sky.
    music: Conductor,
    /// The dust storm's wind, at the lane it covers.
    wind: Option<Sound>,
    /// The night is over: seconds until the screen moves on, and where.
    ending: Option<(f64, Ending)>,
    ended: bool,
    /// For the tutorial: the controls tried.
    pub status_asked: bool,
    pub hint_asked: bool,
    /// Seconds since "Out of range" was last said.
    glanced_ago: f64,
    pub shots: u32,
}

impl NightScreen {
    pub fn new(run: Shared) -> NightScreen {
        let (dome, seed) = {
            let r = run.borrow();
            (r.colony.dome, r.seed() ^ (u64::from(r.night) << 8))
        };
        NightScreen {
            music: Conductor::night(seed),
            wind: None,
            run,
            acc: 0.0,
            engines: HashMap::new(),
            lock_wait: 0.0,
            scanned: None,
            last: String::new(),
            status_key: StatusKey::new(),
            call_outs: true,
            left_to_right: false,
            dome_seen: dome,
            notices: Vec::new(),
            ending: None,
            ended: false,
            status_asked: false,
            hint_asked: false,
            glanced_ago: GLANCE_EVERY,
            shots: 0,
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

    fn settings(&mut self, ctx: &mut Ctx<Parlor>) {
        let p = Progress::load(ctx);
        self.call_outs = p.call_outs;
        self.left_to_right = p.scan_left_to_right;
        if p.call_outs {
            // The quiet bonus is for a whole colony with the call-outs off.
            self.run.borrow_mut().perks.quiet = false;
        }
        let storm = self.run.borrow().storm;
        match (storm, p.storm_wind, self.wind) {
            (Some(storm), true, None) => {
                let s = ctx.audio.start_loop(
                    sound::STORM_WIND,
                    Some(sound::lane_pos(storm.lane)),
                    Bus::AMBIENCE,
                );
                self.wind = Some(s);
            }
            (_, false, Some(s)) | (None, _, Some(s)) => {
                ctx.audio.stop(s, 400);
                self.wind = None;
            }
            _ => {}
        }
    }

    /// Moves the wind to the lane the storm covers.
    fn move_wind(&mut self, ctx: &mut Ctx<Parlor>, lane: u8) {
        if let Some(s) = self.wind {
            ctx.audio.set_position(s, Some(sound::lane_pos(lane)));
        }
    }

    fn stop_wind(&mut self, ctx: &mut Ctx<Parlor>, fade_ms: u32) {
        if let Some(s) = self.wind.take() {
            ctx.audio.stop(s, fade_ms);
        }
    }

    // --- the engines and the lock tone ---------------------------------------

    fn start_engine(&mut self, ctx: &mut Ctx<Parlor>, id: u32) {
        let Some((kind, lane, height)) = self
            .run
            .borrow()
            .raider(id)
            .map(|r| (r.kind, r.lane, r.height))
        else {
            return;
        };
        let s = ctx.audio.start_loop(
            sound::engine(kind),
            Some(sound::sky_pos(lane, height)),
            Bus::EFFECTS,
        );
        self.engines.insert(id, s);
    }

    fn stop_engine(&mut self, ctx: &mut Ctx<Parlor>, id: u32, fade_ms: u32) {
        if let Some(s) = self.engines.remove(&id) {
            ctx.audio.stop(s, fade_ms);
        }
        if self.scanned == Some(id) {
            self.scanned = None;
        }
    }

    fn stop_engines(&mut self, ctx: &mut Ctx<Parlor>, fade_ms: u32) {
        for (_, s) in self.engines.drain() {
            ctx.audio.stop(s, fade_ms);
        }
    }

    /// Puts every engine where its raider is, and silences the ones in
    /// the dust.
    fn place_engines(&mut self, ctx: &mut Ctx<Parlor>) {
        let run = self.run.borrow();
        for raider in &run.raiders {
            let Some(&s) = self.engines.get(&raider.id) else {
                continue;
            };
            ctx.audio
                .set_position(s, Some(sound::sky_pos(raider.lane, raider.height)));
            let volume = if run.in_dust(raider.lane) { 0.0 } else { 1.0 };
            ctx.audio.set_volume(s, volume);
        }
    }

    /// The lock tone pulses while a raider is in the gun's lane, faster
    /// as it nears, and stops the moment the lane is clear.
    fn lock(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let under = {
            let run = self.run.borrow();
            run.under(run.gun)
                .filter(|r| !run.in_dust(r.lane))
                .map(|r| (r.height, run.gun))
        };
        let Some((height, gun)) = under else {
            self.lock_wait = 0.0;
            return;
        };
        self.lock_wait -= dt;
        if self.lock_wait > 0.0 {
            return;
        }
        sound::under(ctx, sound::LOCK, gun);
        let far = height as f64 / HORIZON as f64;
        self.lock_wait = LOCK_NEAR + (LOCK_FAR - LOCK_NEAR) * far;
    }

    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        self.last = text.clone();
        ctx.say(text);
    }

    // --- events ------------------------------------------------------------

    /// Turns the run's events into sound and speech, at once.
    fn drain(&mut self, ctx: &mut Ctx<Parlor>) {
        let events = self.run.borrow_mut().take_events();
        let mut first = true;
        for event in events {
            if let Some(line) = self.hear(ctx, &event) {
                self.last = line.clone();
                if first {
                    ctx.say(line);
                } else {
                    ctx.say_queued(line);
                }
                first = false;
            }
        }
    }

    /// Plays an event's sounds and returns what, if anything, to say.
    fn hear(&mut self, ctx: &mut Ctx<Parlor>, event: &Event) -> Option<String> {
        let call_outs = self.call_outs;
        match event {
            Event::NightOpens { night, .. } => {
                let run = self.run.borrow();
                let mut opening = format!(
                    "Night {night}, gun at the {}. {}.",
                    lane_name(run.gun),
                    sentence_start(&run.forecast_words())
                );
                if run.colony.has(Building::Scanner) || run.perks.forecast_quirks {
                    if let Some(quirks) = run.quirk_words() {
                        opening.push(' ');
                        opening.push_str(&quirks);
                    }
                }
                Some(opening)
            }
            Event::WaveClear { wave, of } => {
                ctx.audio.play(sound::WAVE_CLEAR, Bus::EFFECTS);
                Some(format!("Wave {wave} of {of} down."))
            }
            Event::WaveComing { wave, of, kinds } => {
                ctx.audio.play(sound::WAVE, Bus::EFFECTS);
                if call_outs {
                    Some(format!(
                        "Wave {wave} of {of}: {}.",
                        crate::rules::nights::describe(kinds)
                    ))
                } else {
                    Some(format!("Wave {wave} of {of}."))
                }
            }
            Event::Arrived { id, kind, lane, .. } | Event::Dropped { id, kind, lane, .. } => {
                self.start_engine(ctx, *id);
                let dusted = self.run.borrow().in_dust(*lane);
                if dusted {
                    return None;
                }
                let height = self.run.borrow().raider(*id).map_or(HORIZON, |r| r.height);
                sound::at(ctx, sound::ARRIVE, *lane, height);
                let stab = match kind {
                    Kind::Hulk | Kind::Mothership | Kind::Carrier => Some(Stab::Heavy),
                    Kind::Skitter => Some(Stab::Run),
                    _ => None,
                };
                if let Some(stab) = stab {
                    self.music.stab(&mut ctx.audio, stab);
                }
                if !call_outs {
                    return None;
                }
                let title = self
                    .run
                    .borrow()
                    .raider(*id)
                    .map_or_else(|| kind.title().to_string(), |r| r.title());
                let what = match (event, kind) {
                    (Event::Dropped { .. }, _) => format!("{title} dropped, {}.", lane_name(*lane)),
                    (_, Kind::Mule | Kind::Carrier) => {
                        format!("{title}, crossing from the {}.", lane_name(*lane))
                    }
                    _ => format!("{title}, {}.", lane_name(*lane)),
                };
                Some(what)
            }
            Event::Ranged {
                id, lane, range, ..
            } => {
                if *range != Range::Near || !call_outs || self.run.borrow().in_dust(*lane) {
                    return None;
                }
                let title = self.run.borrow().raider(*id).map(|r| r.title())?;
                Some(format!("{title} near, {}.", lane_name(*lane)))
            }
            Event::Moved { lane, under } => {
                sound::under(ctx, sound::GUN_RAIL, *lane);
                let in_lane = under
                    .map(|(kind, range)| {
                        format!(" {}, {}.", sentence_start(&a(kind)), range.name())
                    })
                    .unwrap_or_default();
                Some(format!("{}.{in_lane}", sentence_start(lane_name(*lane))))
            }
            Event::DryShot { lane } => {
                sound::under(ctx, sound::GUN_FIRE, *lane);
                None
            }
            Event::Glanced { lane, height, .. } => {
                sound::under(ctx, sound::GUN_FIRE, *lane);
                sound::at(ctx, sound::CHARGE_PASS, *lane, *height);
                if self.glanced_ago < GLANCE_EVERY {
                    return None;
                }
                self.glanced_ago = 0.0;
                Some("Out of range.".to_string())
            }
            Event::Hit {
                id,
                kind,
                lane,
                height,
                left,
                by,
            } => {
                self.hitter(ctx, *by, *lane);
                sound::at(ctx, sound::GUN_HIT, *lane, *height);
                if !call_outs || kind.hits() < 3 {
                    return None;
                }
                let title = self.run.borrow().raider(*id).map(|r| r.title())?;
                Some(format!("{title}, {left} left."))
            }
            Event::Down {
                id,
                kind,
                lane,
                height,
                by,
                ..
            } => {
                self.hitter(ctx, *by, *lane);
                sound::at(ctx, sound::GUN_DOWN, *lane, *height);
                self.stop_engine(ctx, *id, 60);
                Some(format!("{} down.", kind.title()))
            }
            Event::Charging { weapon, lane, .. } => {
                sound::under(ctx, sound::CHARGE_WIND, *lane);
                match weapon {
                    Weapon::Siege => Some("Siege shot coming.".to_string()),
                    _ if call_outs => Some(format!("Bolt at the {}.", lane_name(*lane))),
                    _ => None,
                }
            }
            Event::Shot {
                lane,
                dodged,
                shield,
                dome,
                ..
            } => {
                if *dodged {
                    sound::under(ctx, sound::CHARGE_PASS, *lane);
                    return Some("Dodged.".to_string());
                }
                sound::under(ctx, sound::CHARGE_HIT, *lane);
                let line = if *dome < self.dome_seen {
                    format!("Hit. Shield {shield}, dome {dome}.")
                } else {
                    format!("Hit. Shield {shield}.")
                };
                self.dome_seen = *dome;
                Some(line)
            }
            Event::Breach {
                id,
                kind,
                lane,
                shield,
                dome,
                darkened,
            } => {
                let key = if *dome < self.dome_seen {
                    sound::BREACH_DOME
                } else {
                    sound::BREACH_SHIELD
                };
                self.dome_seen = *dome;
                sound::under(ctx, key, *lane);
                self.stop_engine(ctx, *id, 0);
                let mut line = format!(
                    "Breach at the {}, {}. Shield {shield}, dome {dome}.",
                    lane_name(*lane),
                    a(*kind)
                );
                if let Some(building) = darkened {
                    sound::under(ctx, sound::BREACH_BUILDING, *lane);
                    line.push_str(&format!(" {} dark.", building.title()));
                }
                Some(line)
            }
            Event::Weaved { id, to, .. } => {
                let height = self.run.borrow().raider(*id).map_or(MID, |r| r.height);
                sound::at(ctx, sound::WEAVE, *to, height);
                if !call_outs {
                    return None;
                }
                let title = self.run.borrow().raider(*id).map(|r| r.title())?;
                Some(format!("{title} weaves to the {}.", lane_name(*to)))
            }
            Event::Crossed { .. } => None,
            Event::Hovering { lane, .. } => Some(format!(
                "Mothership holds at mid range, {}.",
                lane_name(*lane)
            )),
            Event::Shifted { lane, .. } => {
                call_outs.then(|| format!("Mothership to the {}.", lane_name(*lane)))
            }
            Event::Left { id, kind } => {
                ctx.audio.play(sound::LEAVE, Bus::EFFECTS);
                self.stop_engine(ctx, *id, 300);
                Some(format!("The {} leaves.", kind.name()))
            }
            Event::MineBlew { lane } => {
                sound::at(ctx, sound::MINE, *lane, MID);
                None
            }
            Event::StormMoved { lane } => {
                self.move_wind(ctx, *lane);
                call_outs.then(|| format!("Dust at the {}.", lane_name(*lane)))
            }
            Event::Held { night, take, clean } => {
                ctx.audio.play(sound::HELD, Bus::EFFECTS);
                let run = self.run.borrow();
                let mut line = format!("Night {night} held.");
                let salvage = take.words();
                if !salvage.is_empty() {
                    line.push_str(&format!(" Salvage: {salvage}."));
                }
                if *clean {
                    line.push_str(&format!(" Clean night, {CLEAN_NIGHT_POINTS} bonus."));
                }
                line.push_str(&format!(" Score {}.", run.score));
                if run.endless() && *night == RELIEF_NIGHT {
                    line.push_str(" Relief lands, and the watch goes on.");
                }
                drop(run);
                self.ending = Some((speech_secs(&line) + 0.8, Ending::Colony));
                Some(line)
            }
            Event::Fallen { .. } => {
                ctx.audio.play(sound::BREACH_DOME, Bus::EFFECTS);
                self.stop_engines(ctx, 600);
                self.ending = Some((2.0, Ending::Finish));
                Some("The dome falls.".to_string())
            }
            Event::Relief { .. } => {
                let (hold, line) = if self.run.borrow().relief < RELIEF_NIGHT {
                    // A challenge cut short of night 12 ends on its own result.
                    (1.0, None)
                } else {
                    (2.2, Some("Relief has landed.".to_string()))
                };
                self.ending = Some((hold, Ending::Finish));
                line
            }
            Event::ColonyOpens { .. } | Event::OrderPaid { .. } => {
                let lines = {
                    let run = self.run.borrow();
                    notices(event, &run)
                };
                self.notices.extend(lines);
                None
            }
            Event::Bought { .. }
            | Event::MineLaid { .. }
            | Event::DomeRepaired { .. }
            | Event::BuildingRepaired { .. }
            | Event::MissileMade { .. }
            | Event::OrderTaken { .. } => None,
        }
    }

    /// The sound of what hit a raider.
    fn hitter(&mut self, ctx: &mut Ctx<Parlor>, by: Hitter, lane: u8) {
        match by {
            Hitter::Gun => sound::under(ctx, sound::GUN_FIRE, lane),
            Hitter::Missile => {
                ctx.audio.play(sound::MISSILE, Bus::EFFECTS);
            }
            // The mine's own event plays its blast.
            Hitter::Mine => {}
            Hitter::Flak(post) => sound::under(ctx, sound::FLAK, post),
        }
    }

    // --- the player's actions ----------------------------------------------

    /// Applies an input, or says why not. A shot while the gun reloads is
    /// nothing at all.
    fn act(&mut self, ctx: &mut Ctx<Parlor>, input: Input) {
        if self.ending.is_some() {
            return;
        }
        let outcome = self.run.borrow_mut().apply(input);
        match outcome {
            Ok(()) => {
                if input == Input::Fire {
                    self.shots += 1;
                }
                self.drain(ctx);
            }
            Err(Refusal::Reloading) => {}
            Err(refusal) => {
                let text = match refusal {
                    Refusal::NoTarget => "Nothing to fire at.".to_string(),
                    Refusal::Edge => {
                        format!("The rail ends at the {}.", lane_name(self.run.borrow().gun))
                    }
                    other => refusal_words(other, &self.run.borrow()),
                };
                self.tell(ctx, text);
            }
        }
    }

    /// The raiders in scan order. The ones in the dust are not heard.
    fn order(&self) -> Vec<u32> {
        let run = self.run.borrow();
        let mut raiders = run.heard();
        if self.left_to_right {
            raiders.sort_by_key(|r| (r.lane, r.height, r.id));
        }
        raiders.iter().map(|r| r.id).collect()
    }

    fn scan(&mut self, ctx: &mut Ctx<Parlor>, forward: bool) {
        let order = self.order();
        if order.is_empty() {
            self.scanned = None;
            let text = {
                let run = self.run.borrow();
                if !run.raiders.is_empty() {
                    "Nothing heard outside the dust.".to_string()
                } else if run.to_come() == 0 {
                    "The sky is clear.".to_string()
                } else if let Some(ticks) = run.next_wave_in() {
                    format!(
                        "Nothing in the sky. Wave {} of {} in {}.",
                        run.wave + 1,
                        run.waves_total(),
                        seconds_words(ticks)
                    )
                } else {
                    "Nothing in the sky yet.".to_string()
                }
            };
            self.tell(ctx, text);
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
        let (line, lane, height) = {
            let run = self.run.borrow();
            let raider = run.raider(id).expect("a scanned raider");
            (raider_line(raider), raider.lane, raider.height)
        };
        sound::at(ctx, sound::SCAN, lane, height);
        self.tell(ctx, line);
    }

    /// The sky, lane by lane.
    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hint_asked = true;
        let text = sky_words(&self.run.borrow());
        self.tell(ctx, text);
    }

    fn status_layers(&self) -> Vec<String> {
        let run = self.run.borrow();
        let mut first = String::new();
        for (_, kind, lane) in run.charges() {
            first.push_str(&format!(
                "{} is winding up at the {}. ",
                sentence_start(&a(kind)),
                lane_name(lane)
            ));
        }
        let raiders = run.raiders.len();
        let coming = run.to_come();
        let wave = if run.phase == Phase::Night {
            format!(" Wave {} of {}.", run.wave.max(1), run.waves_total())
        } else {
            String::new()
        };
        first.push_str(&format!(
            "Gun {}. Dome {}, shield {}.{wave} {}, {}.",
            lane_name(run.gun),
            run.colony.dome,
            run.colony.shield,
            match raiders {
                0 => "No raiders in the sky".to_string(),
                1 => "1 raider in the sky".to_string(),
                n => format!("{n} raiders in the sky"),
            },
            match coming {
                0 => "none to come".to_string(),
                1 => "1 to come".to_string(),
                n => format!("{n} to come"),
            }
        ));
        let mut second: Vec<String> = run
            .nearest_first()
            .iter()
            .map(|r| {
                if run.in_dust(r.lane) {
                    format!("{}, in the dust at the {}.", r.title(), lane_name(r.lane))
                } else {
                    raider_line(r)
                }
            })
            .collect();
        if let Some(storm) = run.storm {
            second.push(format!("Dust at the {}.", lane_name(storm.lane)));
        }
        second.push(format!(
            "Night {}, {} seconds in. Missiles {}.",
            run.night,
            run.seconds(),
            run.colony.missiles
        ));
        vec![first, second.join(" ")]
    }

    // --- the night's end ---------------------------------------------------

    fn move_on(&mut self, ctx: &mut Ctx<Parlor>, ending: Ending) {
        match ending {
            Ending::Colony => self.open_colony(ctx),
            Ending::Finish => self.finish(ctx),
        }
    }

    fn open_colony(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        self.music.stop(&mut ctx.audio, 1200);
        self.stop_wind(ctx, 1200);
        self.stop_engines(ctx, 300);
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
        self.music.stop(&mut ctx.audio, 600);
        self.stop_wind(ctx, 600);
        self.stop_engines(ctx, 300);
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
        p.add_run(run, won);
        p.save(ctx);
        let start = Start::of(run);
        let held = nights_phrase(run.nights_held);
        let best = match start {
            Start::Colony => {
                let result = Records::record(ctx, ID, SCORE_BOARD, run.score);
                Records::record(ctx, ID, NIGHTS_BOARD, run.nights_held as u64);
                match result {
                    ScoreResult::First => "Your first colony.".to_string(),
                    ScoreResult::NewBest { .. } => "A new best.".to_string(),
                    ScoreResult::Short { best } => format!("Best {best}."),
                }
            }
            Start::LongWatch => {
                match Records::record(ctx, ID, LONG_WATCH_BOARD, run.nights_held as u64) {
                    ScoreResult::First => "Your first long watch.".to_string(),
                    ScoreResult::NewBest { .. } => "Your longest watch yet.".to_string(),
                    ScoreResult::Short { best } => {
                        format!("Your longest watch is {}.", nights_phrase(best as u8))
                    }
                }
            }
            Start::Storm => match Records::record(ctx, ID, STORM_BOARD, run.score) {
                ScoreResult::First => "Your first dust storm.".to_string(),
                ScoreResult::NewBest { .. } => "A new best in the storm.".to_string(),
                ScoreResult::Short { best } => format!("Best in the storm {best}."),
            },
        };
        let new = unlocks::sync(ctx);
        let unlocked = earned_line(&new)
            .map(|l| format!(" {l}"))
            .unwrap_or_default();
        // The night's last line has said "Relief has landed." or "The
        // dome falls.", so the spoken summary carries on from it; the page
        // read back later says it whole.
        let (summary, page) = if won {
            (
                format!("{} points. {best}{unlocked}", run.score),
                format!("Relief has landed. {} points. {best}{unlocked}", run.score),
            )
        } else if start == Start::LongWatch {
            (
                format!(
                    "Night {}. {} held. {} points. {best}{unlocked}",
                    run.night,
                    sentence_start(&held),
                    run.score
                ),
                format!(
                    "The dome fell on night {}. {} held. {} points. {best}{unlocked}",
                    run.night,
                    sentence_start(&held),
                    run.score
                ),
            )
        } else {
            (
                format!(
                    "Night {}. {} points. {best}{unlocked}",
                    run.night, run.score
                ),
                format!(
                    "The dome fell on night {}. {} points. {best}{unlocked}",
                    run.night, run.score
                ),
            )
        };
        let mut pages = vec![page];
        if start == Start::LongWatch {
            pages.push(format!(
                "{} held. Raiders downed: {}.",
                sentence_start(&held),
                run.downed
            ));
        } else {
            pages.push(format!(
                "{} held of {RELIEF_NIGHT}. Raiders downed: {}.",
                sentence_start(&held),
                run.downed
            ));
        }
        if let Some(bonus) = bonus_words(run) {
            pages.push(bonus);
        }
        pages.push(Self::colony_line(run));
        let again = if won {
            labels::PLAY_AGAIN
        } else {
            labels::TRY_AGAIN
        };
        let again_help = match start {
            Start::Colony => "A new colony, from night 1.",
            Start::LongWatch => "A new long watch, from night 1.",
            Start::Storm => "A new colony in the dust storm, from night 1.",
        };
        ctx.replace(Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
            again,
            again_help,
            move |ctx| crate::begin_as(ctx, start, None, true),
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
        let (summary, page) = if won {
            let line = format!("{name} held. {} points. {best}", run.score);
            (line.clone(), line)
        } else {
            (
                format!("Night {}. {} points. {best}", run.night, run.score),
                format!(
                    "The dome fell on night {}. {} points. {best}",
                    run.night, run.score
                ),
            )
        };
        let streak = Challenges::streak(ctx, ID, period).max(1);
        let held = match run.nights() {
            1 => format!("Raiders downed: {}.", run.downed),
            0 => format!(
                "{} held. Raiders downed: {}.",
                sentence_start(&nights_phrase(run.nights_held)),
                run.downed
            ),
            nights => format!(
                "{} of {} held. Raiders downed: {}.",
                sentence_start(&nights_phrase(run.nights_held)),
                nights_phrase(nights),
                run.downed
            ),
        };
        let pages = vec![
            page,
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

/// "A quarter more for Hard vacuum." or for the call-outs off, or both.
fn bonus_words(run: &Run) -> Option<String> {
    let percent = run.perks.bonus_percent();
    if percent == 0 {
        return None;
    }
    let what = match (run.perks.hard_vacuum, run.perks.quiet) {
        (true, true) => "Hard vacuum and the call-outs off",
        (true, false) => "Hard vacuum",
        _ => "the call-outs off",
    };
    Some(format!("{percent} percent more for {what}, counted in."))
}

/// The nearest raider's height as a row of six, for the score: 1 at the
/// dome, 6 at the horizon.
fn row_of(height: u32) -> u8 {
    (height * 5 / HORIZON) as u8 + 1
}

/// What the score follows: the nearest raider, a pending charge, the
/// shield at 0.
pub fn pressure(run: &Run) -> Pressure {
    Pressure {
        near: run.nearest_first().first().map(|r| row_of(r.height)),
        charge: !run.charges().is_empty(),
        shield_down: run.colony.shield == 0,
    }
}

/// The sky lane by lane, far left to far right, then with a scanner what
/// comes next.
pub fn sky_words(run: &Run) -> String {
    let mut parts = Vec::new();
    for lane in 0..LANES {
        let name = lane_name(lane);
        if run.in_dust(lane) {
            parts.push(format!("{name}, dust"));
            continue;
        }
        let mut in_lane: Vec<&Raider> = run.raiders.iter().filter(|r| r.lane == lane).collect();
        in_lane.sort_by_key(|r| (r.height, r.id));
        let part = match in_lane.as_slice() {
            [] => format!("{name} clear"),
            [one] => format!("{name}, {} {}", one.a(), one.range().name()),
            [near, rest @ ..] => format!(
                "{name}, {} {} and {} behind",
                near.a(),
                near.range().name(),
                rest.len()
            ),
        };
        parts.push(part);
    }
    let mut text = format!("{}.", sentence_start(&parts.join("; ")));
    let lanes_known = run.colony.has(Building::Scanner) || run.perks.lane_forecast;
    if let Some(next) = run.pending.first() {
        let wait = seconds_words(next.at.saturating_sub(run.tick));
        if lanes_known {
            text.push_str(&format!(
                " Next: {} at the {}, in {wait}.",
                a(next.kind),
                lane_name(next.lane)
            ));
        } else {
            text.push_str(&format!(" Next: {}, in {wait}.", a(next.kind)));
        }
    } else if let Some(ticks) = run.next_wave_in() {
        let wave = run.wave as usize;
        let words = run.wave_words();
        let kinds = words.get(wave).cloned().unwrap_or_default();
        text.push_str(&format!(
            " Wave {} of {} in {}: {kinds}.",
            wave + 1,
            run.waves_total(),
            seconds_words(ticks)
        ));
    }
    text
}

/// "1 second", "4 seconds".
fn seconds_words(ticks: u32) -> String {
    let secs = ticks.div_ceil(TICKS_A_SECOND);
    if secs == 1 {
        "1 second".to_string()
    } else {
        format!("{secs} seconds")
    }
}

impl State<Parlor> for NightScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut scene = Scene::from_lines(self.lines(ctx));
        let run = self.run.borrow();
        // The lanes fill the drawing area: the horizon at the top, the
        // dome along the bottom.
        let (x0, y0, lane_w, depth) = (100.0, 30.0, 160.0, 480.0);
        let x_of = |lane: u8| x0 + f32::from(lane) * lane_w;
        let y_of = |height: u32| y0 + depth * (1.0 - height as f32 / HORIZON as f32);
        for lane in 0..=LANES {
            scene.line((x_of(lane), y0), (x_of(lane), y0 + depth), 1.0, PAPER);
        }
        scene.line((x0, y0), (x_of(LANES), y0), 1.0, PAPER);
        let bottom = y0 + depth;
        scene.line((x0, bottom), (x_of(LANES), bottom), 1.0, PAPER);
        for lane in 0..LANES {
            if run.colony.mined(lane) {
                scene.circle(x_of(lane) + lane_w / 2.0, y_of(MID), 8.0, GOLD);
            }
            if let Some(slot) = run.colony.slot(lane) {
                if slot.building == Building::Flak {
                    scene.circle(x_of(lane) + lane_w / 2.0, bottom + 18.0, 10.0, TEAL);
                }
            }
        }
        for raider in &run.raiders {
            let (x, y) = (x_of(raider.lane) + lane_w / 2.0, y_of(raider.height));
            scene.circle(x, y, 12.0 + 4.0 * f32::from(raider.hits), RED);
            scene.label(
                x - 6.0,
                y - 10.0,
                raider.kind.title().chars().take(1).collect::<String>(),
                PAPER,
            );
        }
        // The dome under the sky, the gun on its rail.
        scene.rect(x0, bottom + 6.0, lane_w * f32::from(LANES), 24.0, TEAL);
        scene.rect(x_of(run.gun) + 50.0, bottom - 4.0, 60.0, 16.0, GOLD);
        scene.label(
            x0,
            bottom + 40.0,
            format!(
                "Night {}, {} s. Dome {}, shield {}.",
                run.night,
                run.seconds(),
                run.colony.dome,
                run.colony.shield
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
        let pressure = pressure(&self.run.borrow());
        self.music.set_pressure(pressure);
        self.music.start(&mut ctx.audio);
        self.drain(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.music.stop(&mut ctx.audio, 300);
        self.stop_wind(ctx, 300);
        self.stop_engines(ctx, 300);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.practice() {
            self.settings(ctx);
        }
        // The engines were stopped under the pause; start them again.
        let ids: Vec<u32> = self.run.borrow().raiders.iter().map(|r| r.id).collect();
        for id in ids {
            if !self.engines.contains_key(&id) {
                self.start_engine(ctx, id);
            }
        }
        let layers = self.status_layers();
        let text = format!("Back on watch. {}", layers[0]);
        self.tell(ctx, text);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ended {
            return;
        }
        let action = ctx.action_for(event);
        // Held keys keep sliding and keep firing; the rest act once.
        if event.is_repeat()
            && !matches!(
                action,
                Some(ParlorAction::Left | ParlorAction::Right | ParlorAction::Primary)
            )
        {
            return;
        }
        if action.is_none() {
            if let Some(Key::Escape) = event.pressed() {
                if !self.practice() {
                    let leave = self.mode().leave();
                    menus::pause(ctx, HELP, leave);
                }
            }
            return;
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
        self.glanced_ago += dt;
        if self.ended {
            return;
        }
        let pressure = pressure(&self.run.borrow());
        self.music.set_pressure(pressure);
        self.music.update(&mut ctx.audio, dt);
        if let Some((left, ending)) = self.ending {
            let left = left - dt;
            if left <= 0.0 {
                self.ending = None;
                self.move_on(ctx, ending);
            } else {
                self.ending = Some((left, ending));
            }
            return;
        }
        self.acc += dt;
        let mut ticked = 0;
        while self.acc >= TICK_SECS && ticked < MAX_TICKS_A_FRAME {
            self.run.borrow_mut().tick();
            self.acc -= TICK_SECS;
            ticked += 1;
            self.drain(ctx);
            if self.ending.is_some() {
                self.acc = 0.0;
                return;
            }
        }
        if ticked == MAX_TICKS_A_FRAME {
            self.acc = 0.0;
        }
        self.place_engines(ctx);
        self.lock(ctx, dt);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let run = self.run.borrow();
        let phase = match run.phase {
            Phase::Colony => "colony",
            Phase::Night => "night",
            Phase::Relief => "relief",
            Phase::Fallen => "fallen",
        };
        let mut lines = vec![
            format!(
                "night {}, tick {}, {phase}, gun {}, wave {} of {}, pending {}, cooldown {}",
                run.night,
                run.tick,
                run.gun,
                run.wave,
                run.waves_total(),
                run.to_come(),
                run.cooldown
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
            let state = match (r.state, r.charge) {
                (_, Some(c)) => format!("charging {} in {}", c.lane, c.ticks),
                (RaiderState::Falling, None) => "falling".to_string(),
                (RaiderState::Crossing, None) => "crossing".to_string(),
                (RaiderState::Hovering, None) => "hovering".to_string(),
            };
            lines.push(format!(
                "raider {} {} at {},{} hits {} {state}",
                r.id,
                r.kind.name(),
                r.lane,
                r.height,
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
        "{} and {right} slide the gun a lane. A tone pulses under the gun while a raider is in its lane, faster as it nears. {} fires up the lane, and the gun fires again a moment later. {} fires a missile at the raider nearest the dome. {} and {down} scan the raiders. {} gives the gun, the dome and the shield, and again for every raider. {} reads the sky lane by lane. {} repeats the last line. Escape pauses.",
        sentence_start(&left),
        sentence_start(&fire),
        sentence_start(&missile),
        sentence_start(&up),
        sentence_start(&status),
        sentence_start(&hint),
        sentence_start(&repeat),
    )
}
