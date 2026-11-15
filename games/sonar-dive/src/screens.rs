//! The dive itself, the boat between dives, and the end of an expedition.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, InputEvent, Key, Sound, State, UiSound, Vec3};
use parlor_core::challenges::today;
use parlor_core::menus::{self as shared, labels, Back, Leave, Results};
use parlor_core::score::{DANGER, HURRY};
use parlor_core::{
    pace, Challenge, Challenges, Mood, Pace, Pages, Parlor, ParlorAction, Period, Records,
    RunClock, Score, ScoreResult,
};

use crate::cues;
use crate::rules::{
    clock, clock_of, compass, throttle_name, AirWarning, DiveEnd, Event, Expedition, ExpeditionEnd,
    Kind, Return, MAX_THROTTLE, MINE_HEARING, MIN_THROTTLE, V2,
};
use crate::save::{Saved, DEEP_UNLOCK};
use crate::{HELP, ID};

/// The title's screen name, which "Back to the title" returns to.
pub const TITLE_NAME: &str = "sonar_dive";

/// The challenges menu's screen name. A challenge opened from the launcher
/// sits on a host of the same name, so leaving it leads back to wherever
/// the run began.
pub const CHALLENGES_NAME: &str = "sonar_dive_challenges";

/// World metres to audio units: the audio's distance model fades out over
/// 30 units, so 60 metres of sonar range spans it.
pub const AUDIO_SCALE: f32 = 0.5;

/// Opens the pause screen, whose leave row goes where this run began.
fn pause(ctx: &mut Ctx<Parlor>, challenge: &Option<Challenge>) {
    let leave = if challenge.is_some() {
        Leave::Challenge(CHALLENGES_NAME)
    } else {
        Leave::Title(TITLE_NAME)
    };
    shared::pause(ctx, HELP, leave);
}

fn at(p: V2) -> Vec3 {
    Vec3::new(p.x * AUDIO_SCALE, p.y * AUDIO_SCALE, 0.0)
}

/// What kind of run this is, for scoring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Expedition,
    Deep,
    Practice,
    Challenge(Period),
    /// The glow dive, on new moon nights.
    Glow,
}

impl Mode {
    /// The best-score board this mode records to.
    pub fn board(self) -> Option<&'static str> {
        match self {
            Mode::Expedition => Some("best"),
            Mode::Deep => Some("deep"),
            Mode::Glow => Some(crate::events::GLOW_BOARD),
            _ => None,
        }
    }
}

/// "straight ahead", "behind you", "at 3 o'clock".
pub fn clock_phrase(c: u32) -> String {
    match c {
        12 => "straight ahead".to_string(),
        6 => "behind you".to_string(),
        c => format!("at {c} o'clock"),
    }
}

fn metres(d: f32) -> String {
    let n = parlor_core::units::from_metres(f64::from(d))
        .round()
        .max(1.0);
    parlor_core::units::count(n as i64)
}

/// A dive's depth: "30 metres", or "100 feet" with US units.
pub fn depth(m: f64) -> String {
    parlor_core::units::length_rounded(m, 20.0)
}

/// Where the bell is from the sub.
fn bell_phrase(d: &crate::rules::Dive) -> String {
    if d.bell_distance() < crate::rules::BELL_REACH {
        "You are at the bell.".to_string()
    } else {
        format!(
            "The bell is {} {}.",
            metres(d.bell_distance()),
            clock_phrase(d.bell_clock())
        )
    }
}

/// The spoken readout of a ping.
pub fn readout(returns: &[Return]) -> String {
    if returns.is_empty() {
        return "No echoes in range.".to_string();
    }
    let parts: Vec<String> = returns
        .iter()
        .take(4)
        .map(|r| {
            let name = match (r.empty, r.kind) {
                (true, Kind::Galleon) => "empty galleon",
                (true, _) => "empty wreck",
                (false, kind) => kind.name(),
            };
            format!("{name} {} {}", metres(r.dist), clock_phrase(r.clock))
        })
        .collect();
    format!("Echoes: {}.", parts.join("; "))
}

/// Seconds after the last turn before the heading is spoken.
pub const HEADING_PAUSE: f64 = 0.3;

/// The keys a dive uses, by their current names.
pub struct Keys {
    pub ping: String,
    pub left: String,
    pub right: String,
    pub up: String,
    pub down: String,
    pub buoy: String,
    pub status: String,
    pub repeat: String,
    pub hint: String,
}

impl Keys {
    pub fn of(ctx: &Ctx<Parlor>) -> Keys {
        Keys {
            ping: ctx.control_name(ParlorAction::Primary),
            left: ctx.control_name(ParlorAction::Left),
            right: ctx.control_name(ParlorAction::Right),
            up: ctx.control_name(ParlorAction::Up),
            down: ctx.control_name(ParlorAction::Down),
            buoy: ctx.control_name(ParlorAction::Secondary),
            status: ctx.control_name(ParlorAction::Status),
            repeat: ctx.control_name(ParlorAction::Repeat),
            hint: ctx.control_name(ParlorAction::Hint),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{} pings sonar. {} and {} turn while held. {} and {} set the throttle. {} drops a marker buoy. {} speaks air, hull and the bell, twice for the rest, {} repeats the last message, {} gives a hint. Escape pauses.",
            sentence_start(&self.ping),
            sentence_start(&self.left),
            self.right,
            sentence_start(&self.up),
            self.down,
            sentence_start(&self.buoy),
            sentence_start(&self.status),
            self.repeat,
            self.hint
        )
    }

    /// The keys a page at a time, for F1 during a dive: moving first,
    /// then the rest.
    pub fn pages(&self) -> Vec<String> {
        vec![
            format!(
                "{} pings sonar. {} and {} turn while held. {} and {} set the throttle.",
                sentence_start(&self.ping),
                sentence_start(&self.left),
                self.right,
                sentence_start(&self.up),
                self.down,
            ),
            format!(
                "{} speaks air, hull and the bell, twice for the rest. {} gives a hint. {} drops a marker buoy. {} repeats. Escape pauses.",
                sentence_start(&self.status),
                sentence_start(&self.hint),
                sentence_start(&self.buoy),
                sentence_start(&self.repeat),
            ),
        ]
    }
}

/// The dive: the sub, the sonar and the sea.
pub struct DiveScreen {
    exp: Option<Expedition>,
    mode: Mode,
    challenge: Option<Challenge>,
    engine: Sound,
    current: Sound,
    readout: bool,
    last: String,
    was_turning: bool,
    /// Seconds until the heading is spoken after the last turn, so a run
    /// of taps says it once.
    heading_in: Option<f64>,
    tick_sector: i32,
    mine_taught: bool,
    scrape_taught: bool,
    /// The deep's music, following the dive.
    score: Score,
    /// The equinox's whale road: far song to the north, as a landmark.
    pub whale_road: bool,
    road_in: f64,
    /// The far song now sounding, stopped the moment a ping goes out.
    road: Sound,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// The expedition's pace, fixed on its first dive and carried to the
    /// next through the boat.
    pace: Option<Pace>,
    /// The sea's clock: it holds for speech, at the expedition's pace.
    pub clock: RunClock,
}

impl DiveScreen {
    pub fn new(exp: Expedition, mode: Mode, challenge: Option<Challenge>) -> DiveScreen {
        DiveScreen {
            status_key: Default::default(),
            exp: Some(exp),
            mode,
            challenge,
            engine: Sound::NONE,
            current: Sound::NONE,
            readout: false,
            last: String::new(),
            was_turning: false,
            heading_in: None,
            tick_sector: 0,
            mine_taught: false,
            scrape_taught: false,
            score: Score::new(),
            whale_road: false,
            road_in: crate::events::ROAD_FIRST,
            road: Sound::NONE,
            pace: None,
            clock: RunClock::default(),
        }
    }

    /// The same dive at `pace`, as the expedition's later dives keep the
    /// pace of its first.
    pub fn at_pace(mut self, pace: Option<Pace>) -> DiveScreen {
        self.pace = pace;
        self
    }

    /// Where the whale road runs: due north, beyond the site's edge.
    fn road_at(&self) -> V2 {
        let r = self.exp.as_ref().map_or(60.0, |e| e.dive.layout.radius);
        V2::from_heading(0.0).scale(r + crate::events::ROAD_BEYOND)
    }

    /// The far whale song, now and then, never over a ping's echoes: it
    /// starts only while no echoes are on their way back, and a ping stops it.
    fn sing_the_road(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if !self.whale_road {
            return;
        }
        self.road_in -= dt;
        let busy = self.exp.as_ref().is_none_or(|e| e.dive.echoes_pending());
        if self.road_in > 0.0 || busy {
            return;
        }
        self.road_in = crate::events::ROAD_EVERY;
        let s = ctx
            .audio
            .play_at(cues::WHALE_ROAD, at(self.road_at()), Bus::AMBIENCE);
        ctx.audio.set_volume(s, 0.5);
        self.road = s;
    }

    /// Silences the far song, so a ping's echoes come back over nothing.
    fn hush_the_road(&mut self, ctx: &mut Ctx<Parlor>, fade_ms: u32) {
        ctx.audio.stop(self.road, fade_ms);
        self.road = Sound::NONE;
    }

    /// The expedition under way, for tests.
    pub fn expedition(&self) -> Option<&Expedition> {
        self.exp.as_ref()
    }

    /// The expedition under way, to change for tests.
    pub fn expedition_mut(&mut self) -> Option<&mut Expedition> {
        self.exp.as_mut()
    }

    fn tell(&mut self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        let text = text.into();
        self.last = text.clone();
        ctx.say(text);
    }

    fn intro(&self, ctx: &mut Ctx<Parlor>) -> String {
        let Some(exp) = &self.exp else {
            return String::new();
        };
        let d = &exp.dive;
        let l = &d.layout;
        let mut text = format!("Dive {}, {} down.", l.number, depth(f64::from(l.depth_m)));
        let m = exp.config.modifier;
        if m != crate::rules::Modifier::None {
            text.push_str(&format!(" {}: {}.", m.name(), m.describe()));
        }
        if exp.config.galleon {
            text.push_str(" The galleon lies near the edge of the site, ringed by mines.");
        }
        if exp.config.pearl {
            if exp.dives_begun() == 1 {
                text.push_str(" The new moon makes the sea glow, and a glow pearl lies on every dive tonight.");
            } else {
                text.push_str(" A glow pearl lies somewhere on this dive.");
            }
        }
        if self.whale_road && exp.dives_begun() == 1 {
            text.push_str(" Whale song from the far north: a landmark only.");
        }
        if d.practice {
            text.push_str(" Practice: endless air, harmless mines.");
        } else {
            text.push_str(&format!(
                " {} seconds of air, hull {} of {}.",
                d.oxygen.round(),
                d.hull,
                d.max_hull
            ));
        }
        // After an expedition or the tutorial, the keys are known: F1 has them.
        let known = Saved::peek(ctx).expeditions > 0 || shared::tutorial_read(ctx, ID);
        if exp.dives_begun() == 1 && known {
            text.push_str(" You start at the bell, the way up.");
        } else if exp.dives_begun() == 1 {
            text.push_str(&format!(
                " {} You start at the bell, the way up.",
                Keys::of(ctx).summary()
            ));
        } else {
            text.push_str(&format!(
                " {} mines.",
                d.objects
                    .iter()
                    .filter(|o| o.kind == Kind::Mine && o.active)
                    .count()
            ));
        }
        text
    }

    /// The S key, in layers. First: air, hull, any current and where the
    /// bell is. A second press: heading, throttle, coins and the dive.
    pub fn status_layers(&self) -> Vec<String> {
        let Some(exp) = &self.exp else {
            return Vec::new();
        };
        let d = &exp.dive;
        let mut first = Vec::new();
        if d.practice {
            first.push(format!("Unlimited air, hull {} of {}.", d.hull, d.max_hull));
        } else {
            first.push(format!(
                "Air {} seconds, hull {} of {}.",
                d.oxygen.ceil(),
                d.hull,
                d.max_hull
            ));
        }
        if d.in_current.is_some() {
            first.push("In a current.".to_string());
        }
        first.push(bell_phrase(d));
        let mut more = vec![format!(
            "Heading {} degrees, {}, {}.",
            d.heading.round() as i32 % 360,
            compass(d.heading),
            throttle_name(d.throttle)
        )];
        more.push(format!(
            "Carrying {} coins, {} banked.",
            d.carried, exp.banked
        ));
        if d.salvage.is_some() {
            more.push("Salvaging.".to_string());
        }
        more.push(format!(
            "Dive {}, {}.",
            d.layout.number,
            depth(f64::from(d.layout.depth_m))
        ));
        if d.practice {
            more.push("Practice.".to_string());
        }
        if self.whale_road {
            more.push("Whales to the north.".to_string());
        }
        vec![
            parlor_core::status::sentences(first),
            parlor_core::status::sentences(more),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        parlor_core::status::sentences(self.status_layers())
    }

    /// The H key.
    pub fn hint(&self) -> String {
        let Some(exp) = &self.exp else {
            return String::new();
        };
        let d = &exp.dive;
        let bell = bell_phrase(d);
        if !d.practice && d.left_bell && d.oxygen < d.air_home() + 10.0 {
            return format!(
                "Head home now: about {} seconds away, {} of air. {bell}",
                d.air_home().round(),
                d.oxygen.round()
            );
        }
        let mut s = match d.nearest_treasure() {
            Some(i) => {
                let o = &d.objects[i];
                let mut s = format!(
                    "The nearest treasure is a {} {} {}.",
                    o.kind.name(),
                    metres(o.pos.dist(d.pos)),
                    clock_phrase(clock(d.heading, d.pos, o.pos))
                );
                if matches!(o.kind, Kind::Wreck | Kind::Galleon) {
                    s.push_str(" Stop beside it to salvage.");
                } else {
                    s.push_str(" Steer into it to collect it.");
                }
                s
            }
            None => format!("Picked clean. Back to the bell. {bell}"),
        };
        if self.whale_road {
            s.push_str(" Whale song is due north.");
        }
        if let Some((i, dist)) = d.nearest_mine() {
            if dist < 20.0 {
                s.push_str(&format!(
                    " Careful: a mine is {} {}.",
                    metres(dist),
                    clock_phrase(clock(d.heading, d.pos, d.objects[i].pos))
                ));
            }
        }
        s
    }

    /// What to do now, for F1.
    fn situation(&self) -> &'static str {
        let Some(d) = self.exp.as_ref().map(|e| &e.dive) else {
            return "";
        };
        if d.salvage.is_some() {
            "You are salvaging a wreck: stay at all stop until it finishes."
        } else if d.in_current.is_some() {
            "You are in a current: throttle up and steer to fight it, or let it carry you."
        } else if !d.left_bell {
            "You are at the boat's bell. Find treasure by sonar, then come back here to surface."
        } else {
            "Steer toward treasure, and come back to the bell before your air runs out."
        }
    }

    fn sound_event(&mut self, ctx: &mut Ctx<Parlor>, event: Event) {
        match event {
            Event::Ping => {
                self.hush_the_road(ctx, 60);
                ctx.audio.play(cues::PING, Bus::EFFECTS);
            }
            Event::PingNotReady => {
                ctx.audio.play(cues::NOT_READY, Bus::EFFECTS);
                ctx.say("Sonar recharging.");
            }
            Event::Echo { kind, pos, empty } => {
                ctx.audio
                    .play_at(cues::echo(kind, empty), at(pos), Bus::EFFECTS);
            }
            Event::PingDone { returns } => {
                if self.readout || returns.is_empty() {
                    let text = readout(&returns);
                    if self.exp.as_ref().is_some_and(|e| e.dive.lesson) {
                        self.last = text.clone();
                        ctx.say_queued(text);
                    } else {
                        self.tell(ctx, text);
                    }
                }
            }
            Event::Chest { value, carried } => {
                ctx.audio.play(cues::CHEST, Bus::EFFECTS);
                self.tell(
                    ctx,
                    format!("Treasure chest, {value} coins. Carrying {carried}."),
                );
            }
            Event::Pearl { value, carried } => {
                ctx.audio.play(cues::PEARL, Bus::EFFECTS);
                self.tell(
                    ctx,
                    format!("Glow pearl, {value} coins. Carrying {carried}."),
                );
            }
            Event::SalvageStart { galleon } => {
                self.clock.reset();
                let what = if galleon { "the galleon" } else { "the wreck" };
                self.tell(ctx, format!("Salvaging {what}. Stay stopped."));
            }
            Event::SalvageClank { pos } => {
                ctx.audio.play_at(cues::CLANK, at(pos), Bus::EFFECTS);
            }
            Event::SalvageDone { value, carried, .. } => {
                ctx.audio.play(cues::SALVAGED, Bus::EFFECTS);
                self.tell(ctx, format!("Salvaged {value} coins. Carrying {carried}."));
            }
            Event::SalvageBroken => {
                self.tell(ctx, "Salvage interrupted.");
            }
            Event::MineNear { .. } => {
                self.clock.reset();
                if self.mine_taught {
                    ctx.say("Mine.");
                } else {
                    self.mine_taught = true;
                    self.tell(ctx, "Mine! Steer away.");
                }
            }
            Event::MineTick { pos } => {
                ctx.audio.play_at(cues::MINE_TICK, at(pos), Bus::EFFECTS);
            }
            Event::MineHit { hull, .. } => {
                self.clock.reset();
                ctx.audio.play(cues::MINE_BLAST, Bus::EFFECTS);
                let practice = self.exp.as_ref().is_some_and(|e| e.dive.practice);
                if practice {
                    self.tell(ctx, "Mine. No harm in practice.");
                } else if hull > 0 {
                    let max = self.exp.as_ref().map_or(hull, |e| e.dive.max_hull);
                    self.tell(ctx, format!("Mine! Hull {hull} of {max}."));
                }
            }
            Event::Scrape { pos, wall } => {
                self.clock.reset();
                ctx.audio.play_at(cues::SCRAPE, at(pos), Bus::EFFECTS);
                let what = if wall { "the rock wall" } else { "a rock" };
                if self.scrape_taught {
                    ctx.say(format!("Scraped {what}. All stop."));
                } else {
                    self.scrape_taught = true;
                    let practice = self.exp.as_ref().is_some_and(|e| e.dive.practice);
                    let air = if practice {
                        ""
                    } else {
                        ", 3 seconds of air lost"
                    };
                    self.tell(
                        ctx,
                        format!("Scraped {what}. All stop{air}. Turn and throttle up."),
                    );
                }
            }
            Event::WhaleSong { pos } => {
                ctx.audio.play_at(cues::WHALE_SONG, at(pos), Bus::EFFECTS);
            }
            Event::WhaleAir { secs } => {
                ctx.audio.play(cues::WHALE_AIR, Bus::EFFECTS);
                let text = if secs < 0.5 {
                    "The whale swims beside you.".to_string()
                } else {
                    format!("Whale breath: plus {} seconds of air.", secs.round())
                };
                self.tell(ctx, text);
            }
            Event::CurrentIn { flow } => {
                self.clock.reset();
                ctx.audio.stop(self.current, 100);
                self.current = ctx.audio.start_loop(cues::CURRENT, None, Bus::EFFECTS);
                ctx.audio.set_volume(self.current, 0.6);
                let heading = self.exp.as_ref().map_or(0.0, |e| e.dive.heading);
                let toward = clock_of(flow.heading() - heading);
                self.tell(
                    ctx,
                    format!(
                        "Current, toward {}.",
                        match toward {
                            12 => "straight ahead".to_string(),
                            6 => "straight behind you".to_string(),
                            c => format!("{c} o'clock"),
                        }
                    ),
                );
            }
            Event::CurrentOut => {
                ctx.audio.stop(self.current, 400);
                self.current = Sound::NONE;
                ctx.say("Out of the current.");
            }
            Event::Air(w) => {
                self.clock.reset();
                ctx.audio.play(cues::AIR_WARNING, Bus::EFFECTS);
                let left = self.exp.as_ref().map_or(0.0, |e| e.dive.oxygen).round();
                let at_bell = self
                    .exp
                    .as_ref()
                    .is_some_and(|e| e.dive.bell_distance() < crate::rules::BELL_REACH);
                let text = match w {
                    AirWarning::Half => format!("Half air, {left} seconds."),
                    AirWarning::Quarter => format!("Quarter air, {left} seconds."),
                    // Sitting at the bell, "get to the bell" would be wrong.
                    AirWarning::Low if at_bell => format!("Air critical, {left} seconds!"),
                    AirWarning::Low => format!("Air critical, {left} seconds! Get to the bell."),
                };
                self.tell(ctx, text);
            }
            Event::Bell => {
                ctx.audio.play_at(cues::BELL, at(V2::ZERO), Bus::EFFECTS);
            }
            Event::BuoyBeep { pos, .. } => {
                ctx.audio.play_at(cues::BUOY_BEEP, at(pos), Bus::EFFECTS);
            }
            Event::End(_) => {}
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some(mut exp) = self.exp.take() else {
            return;
        };
        self.score.stop(ctx);
        self.hush_the_road(ctx, 300);
        ctx.audio.stop(self.engine, 300);
        ctx.audio.stop(self.current, 300);
        self.engine = Sound::NONE;
        self.current = Sound::NONE;
        match exp.dive.outcome {
            Some(DiveEnd::Surfaced) => {
                ctx.audio.play(cues::SURFACE, Bus::EFFECTS);
            }
            Some(DiveEnd::Drowned) => {
                ctx.audio.play(cues::DROWN, Bus::EFFECTS);
            }
            _ => {}
        }
        exp.settle();
        if exp.end.is_none() {
            ctx.replace(SurfaceMenu::new(exp, self.mode, self.challenge.take()).at_pace(self.pace));
        } else {
            let pace = self.clock.pace();
            conclude(ctx, exp, self.mode, self.challenge.take(), pace);
        }
    }
}

impl State<Parlor> for DiveScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        "sonar_dive_dive"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.readout = Saved::load(ctx).readout;
        let pace = *self.pace.get_or_insert_with(|| pace::get(ctx, ID));
        self.clock = RunClock::new(pace);
        let today = parlor_core::challenges::today(ctx);
        self.whale_road = crate::events::whale_road(self.mode, today);
        ctx.audio.play_music(None, false);
        let Some(exp) = &self.exp else {
            return;
        };
        let n = exp.dive_number();
        let (pos, heading) = (exp.dive.pos, exp.dive.heading);
        self.tick_sector = (heading / 30.0).floor() as i32;
        ctx.audio.set_listener(at(pos), heading);
        ctx.audio.spatial = cues::spatial();
        ctx.audio.set_bed(Some(cues::ambience(n)));
        self.engine = ctx.audio.start_loop(cues::ENGINE, None, Bus::EFFECTS);
        ctx.audio.set_volume(self.engine, 0.25);
        let mood = mood(exp);
        self.score.start(ctx, &cues::SCORE, mood);
        let intro = self.intro(ctx);
        self.tell(ctx, intro);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.score.stop(ctx);
        self.hush_the_road(ctx, 300);
        ctx.audio.stop(self.engine, 300);
        ctx.audio.stop(self.current, 300);
        ctx.audio.set_bed(None);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        ctx.audio.spatial = hearken::SpatialConfig::default();
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.resume_all();
        self.clock.reset();
        // Settings, from the pause menu, may have changed the call-outs.
        self.readout = Saved::load(ctx).readout;
        let Some(exp) = &self.exp else {
            return;
        };
        let d = &exp.dive;
        let air = if d.practice {
            String::new()
        } else {
            format!(" {} seconds of air.", d.oxygen.ceil())
        };
        ctx.say(format!("Back to dive {}.{air}", d.layout.number));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() || self.exp.is_none() {
            return;
        }
        if event.pressed() == Some(Key::Escape) {
            pause(ctx, &self.challenge);
            return;
        }
        let Some(action) = ctx.action_for(event) else {
            return;
        };
        self.clock.asked_by(Some(action));
        match action {
            ParlorAction::Primary => {
                let ev = self.exp.as_mut().map(|e| e.dive.ping());
                if let Some(ev) = ev {
                    self.sound_event(ctx, ev);
                }
            }
            ParlorAction::Left | ParlorAction::Right => {
                ctx.audio.play(cues::TURN_TICK, Bus::EFFECTS);
            }
            ParlorAction::Up | ParlorAction::Down => {
                let Some(exp) = self.exp.as_mut() else {
                    return;
                };
                let up = action == ParlorAction::Up;
                let changed = if up {
                    exp.dive.throttle_up()
                } else {
                    exp.dive.throttle_down()
                };
                match changed {
                    Some(t) => {
                        let key = if up {
                            cues::THROTTLE_UP
                        } else {
                            cues::THROTTLE_DOWN
                        };
                        ctx.audio.play(key, Bus::EFFECTS);
                        ctx.say(sentence_start(throttle_name(t)) + ".");
                    }
                    None => {
                        let t = if up { MAX_THROTTLE } else { MIN_THROTTLE };
                        ctx.audio.play(cues::NOT_READY, Bus::EFFECTS);
                        ctx.say(format!("Already {}.", throttle_name(t)));
                    }
                }
            }
            ParlorAction::Secondary => {
                let Some(exp) = self.exp.as_mut() else {
                    return;
                };
                let n = exp.dive.drop_buoy();
                ctx.audio.play(cues::BUOY_DROP, Bus::EFFECTS);
                let note = if n as usize > crate::rules::MAX_BUOYS {
                    " Your oldest buoy is gone."
                } else {
                    ""
                };
                self.tell(ctx, format!("Buoy {n} dropped here.{note}"));
            }
            ParlorAction::Status => {
                let layers = self.status_layers();
                self.status_key.speak(ctx, layers);
            }
            ParlorAction::Repeat => {
                let text = if self.last.is_empty() {
                    "Nothing to repeat yet.".to_string()
                } else {
                    self.last.clone()
                };
                ctx.say(text);
            }
            ParlorAction::Hint => {
                let h = self.hint();
                self.tell(ctx, h);
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let left = ctx.action_held(ParlorAction::Left);
        let right = ctx.action_held(ParlorAction::Right);
        let turn = right as i32 - left as i32;
        let Some(exp) = self.exp.as_mut() else {
            return;
        };
        // A held turn answers through speech, and a ping's echoes keep
        // their real delays: the delay is the distance.
        if turn != 0 || exp.dive.echoes_pending() {
            self.clock.bypass();
        }
        let game_dt = self.clock.game_dt(ctx, dt);
        let events = exp.dive.update(game_dt, turn);
        let mood = mood(exp);
        self.score.follow(ctx, mood, dt);
        let heading = exp.dive.heading;
        let pos = exp.dive.pos;
        let throttle = exp.dive.throttle;
        let ended = exp.dive.outcome.is_some();
        ctx.audio.set_listener(at(pos), heading);
        ctx.audio
            .set_pitch(self.engine, 0.75 + 0.15 * (throttle - MIN_THROTTLE) as f32);
        ctx.audio
            .set_volume(self.engine, 0.2 + 0.08 * throttle.unsigned_abs() as f32);
        let sector = (heading / 30.0).floor() as i32;
        if turn != 0 && sector != self.tick_sector {
            ctx.audio.play(cues::TURN_TICK, Bus::EFFECTS);
        }
        self.tick_sector = sector;
        if turn != 0 {
            self.heading_in = None;
        } else if self.was_turning {
            self.heading_in = Some(HEADING_PAUSE);
        } else if let Some(left) = self.heading_in {
            let left = left - dt;
            if left <= 0.0 {
                self.heading_in = None;
                // Queued, so it never cuts off a line said as the turn ended,
                // such as the tutorial's next step.
                ctx.say_queued(format!(
                    "Heading {} degrees, {}.",
                    heading.round() as i32 % 360,
                    compass(heading)
                ));
            } else {
                self.heading_in = Some(left);
            }
        }
        self.was_turning = turn != 0;
        for e in events {
            self.sound_event(ctx, e);
        }
        if !ended {
            self.sing_the_road(ctx, dt);
        }
        if ended {
            self.finish(ctx);
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        vec![self.status()]
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut pages = Keys::of(ctx).pages();
        pages.insert(0, self.situation().to_string());
        pages.join(" ")
    }

    /// F1 during a dive pauses it, air and all, and reads where you are
    /// and the keys a page at a time; the dive goes on when they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.exp.is_none() {
            return;
        }
        let mut pages = Keys::of(ctx).pages();
        pages.insert(0, format!("Paused while you read. {}", self.situation()));
        ctx.audio.pause_all();
        ctx.push(Pages::new(pages));
    }
}

/// Between dives, at the boat.
pub struct SurfaceMenu {
    menu: MenuCore<Self, Parlor>,
    exp: Option<Expedition>,
    mode: Mode,
    challenge: Option<Challenge>,
    pace: Option<Pace>,
}

impl SurfaceMenu {
    pub fn new(exp: Expedition, mode: Mode, challenge: Option<Challenge>) -> SurfaceMenu {
        let title = format!(
            "Surfaced. {} coins this dive, {} banked. Hull {} of {}",
            exp.last_haul, exp.banked, exp.dive.hull, exp.config.hull
        );
        SurfaceMenu {
            menu: MenuCore::new(title).with_intro_help(
                "Choose to dive again, deeper, or end the expedition to keep what you have banked.",
            ),
            exp: Some(exp),
            mode,
            challenge,
            pace: None,
        }
    }

    /// The menu for an expedition diving at `pace`.
    pub fn at_pace(mut self, pace: Option<Pace>) -> SurfaceMenu {
        self.pace = pace;
        self
    }
}

impl Menu for SurfaceMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "sonar_dive_surface"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(Some(cues::MUSIC_SURFACED), false);
        menu_enter(self, ctx);
    }

    /// Once the surfacing tune ends, the boat's own loop takes over.
    fn update(&mut self, ctx: &mut Ctx<Parlor>, _dt: f64) {
        if ctx.audio.music().is_none() {
            parlor_core::shop_music::start(ctx, cues::MUSIC_SURFACE);
        }
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let Some(exp) = &self.exp else {
            return Vec::new();
        };
        let next = exp.dive_number() + 1;
        let last = exp
            .config
            .max_dives
            .is_some_and(|m| exp.dives_begun() + 1 == m);
        let last = if last { ", the last dive" } else { "" };
        vec![
            MenuItem::new(
                format!("Dive {next}, {}{last}", depth(f64::from(30 * next))),
                |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    if let Some(mut exp) = s.exp.take() {
                        exp.next_dive();
                        ctx.replace(
                            DiveScreen::new(exp, s.mode, s.challenge.take()).at_pace(s.pace),
                        );
                    }
                },
            )
            .help("Richer treasure, more mines and currents, less air."),
            MenuItem::new(
                format!("Stop and keep {} coins", exp.banked),
                |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    if let Some(mut exp) = s.exp.take() {
                        exp.stop();
                        let pace = s.pace.unwrap_or_else(|| pace::get(ctx, ID));
                        conclude(ctx, exp, s.mode, s.challenge.take(), pace);
                    }
                },
            )
            .help("Your banked treasure becomes your score."),
        ]
    }

    fn on_escape(&mut self, ctx: &mut Ctx<Parlor>) {
        pause(ctx, &self.challenge);
    }
}

fn score_line(result: ScoreResult, noun: &str, pace: Pace) -> String {
    let best = pace.best_word();
    match result {
        ScoreResult::First if pace.separate() => format!("Your first relaxed {noun}."),
        ScoreResult::First => format!("Your first recorded {noun}."),
        ScoreResult::NewBest { previous } => {
            format!("A new {best} {noun}. The old {best} was {previous}.")
        }
        ScoreResult::Short { best: b } => format!("Your {best} {noun} is {b}."),
    }
}

/// Ends an expedition: records it, plays its stinger and reads the result.
pub fn conclude(
    ctx: &mut Ctx<Parlor>,
    exp: Expedition,
    mode: Mode,
    challenge: Option<Challenge>,
    pace: Pace,
) {
    let end = exp.end.unwrap_or(ExpeditionEnd::Banked);
    let n = exp.dive_number();
    let score = exp.score();
    let mut pages = Vec::new();
    let lost = exp.dive.carried;
    let head = match end {
        ExpeditionEnd::Banked if score == 0 => {
            format!(
                "Expedition over after {} {}, no coins.",
                exp.surfaced,
                if exp.surfaced == 1 { "dive" } else { "dives" }
            )
        }
        ExpeditionEnd::Banked => format!(
            "Expedition over. Surfaced {} {}, {score} coins banked.",
            exp.surfaced,
            if exp.surfaced == 1 { "time" } else { "times" }
        ),
        ExpeditionEnd::Drowned => {
            format!("Out of air on dive {n}. {lost} coins aboard lost, {score} banked.")
        }
        ExpeditionEnd::Wrecked => {
            format!("Hull breached on dive {n}. {lost} coins aboard lost, {score} banked.")
        }
    };
    pages.push(head);
    let summary = match end {
        ExpeditionEnd::Banked => format!("Expedition over. {score} coins banked"),
        ExpeditionEnd::Drowned => format!("Out of air on dive {n}. {score} coins banked"),
        ExpeditionEnd::Wrecked => format!("Hull breached on dive {n}. {score} coins banked"),
    };
    ctx.audio.play_music(
        Some(if end == ExpeditionEnd::Banked {
            cues::MUSIC_WIN
        } else {
            cues::MUSIC_LOSE
        }),
        false,
    );
    // Challenges start deep on their own sea: they unlock nothing.
    if !matches!(mode, Mode::Practice | Mode::Challenge(_)) {
        let mut saved = Saved::load(ctx);
        let was = saved.deep_unlocked();
        saved.deepest = saved.deepest.max(n);
        saved.expeditions += 1;
        saved.store(ctx);
        if !was && saved.deep_unlocked() {
            pages.push(format!(
                "You reached dive {DEEP_UNLOCK}. Deep start is unlocked: the title menu can now begin an expedition at dive {DEEP_UNLOCK}."
            ));
        }
    }
    match (mode, &challenge) {
        (Mode::Challenge(_), Some(ch)) => {
            let result = Challenges::submit_at(ctx, ch, pace, score as u64);
            let streak = Challenges::streak(ctx, ID, ch.period);
            let streak = format!("A {} {} streak.", streak.max(1), ch.period.unit());
            pages.push(format!(
                "{} result: {score} coins. {} {streak} {}",
                ch.title(),
                score_line(result, "result", pace),
                ch.resets()
            ));
        }
        (Mode::Glow, _) if !crate::events::glow_open(today(ctx)) => pages.push(format!(
            "Haul: {score} coins. {}, so it stays off the glow dive board.",
            crate::events::GLOW_OVER
        )),
        _ => match mode.board() {
            // An empty haul is no record: "your first recorded haul" for
            // nothing, then "the old best was 0", reads as a mistake.
            Some(_) if score == 0 => {
                pages.push("Haul: 0 coins, so nothing goes on the board.".to_string())
            }
            Some(board) => {
                let result = Records::record(ctx, ID, &pace.board(board), score as u64);
                pages.push(format!(
                    "Haul: {score} coins. {}",
                    score_line(result, "haul", pace)
                ));
            }
            None => pages.push("Practice dives are not scored.".to_string()),
        },
    }
    let won = end == ExpeditionEnd::Banked;
    let results = after_run(mode, won, summary, pages.clone());
    let back_to = match mode {
        Mode::Challenge(_) => CHALLENGES_NAME,
        _ => TITLE_NAME,
    };
    let mut results = Some(results);
    ctx.replace(Pages::new(pages).then(move |ctx| {
        if let Some(results) = results.take() {
            ctx.pop_to_quiet(back_to);
            ctx.push(results);
        }
    }));
}

/// The results menu after an expedition: the same mode again, the results
/// again, and back to where the run began.
pub fn after_run(mode: Mode, won: bool, summary: String, pages: Vec<String>) -> Results {
    match mode {
        Mode::Challenge(period) => Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME))
            .again(
                labels::challenge_again(period),
                "The same sea, straight away. Only your best result is kept.",
                move |ctx| {
                    let ch = Challenge::current(ctx, ID, period);
                    ctx.replace(challenge_dive(&ch));
                },
            ),
        _ => {
            let (label, help) = match mode {
                Mode::Practice => (labels::PLAY_AGAIN, "Another practice dive, on a new sea."),
                Mode::Deep => (
                    if won {
                        labels::PLAY_AGAIN
                    } else {
                        labels::TRY_AGAIN
                    },
                    "A new deep start expedition, from dive 4, on a new sea.",
                ),
                Mode::Glow => (
                    if won {
                        labels::PLAY_AGAIN
                    } else {
                        labels::TRY_AGAIN
                    },
                    "Tonight's glowing sea again, from dive 1.",
                ),
                _ => (
                    if won {
                        labels::PLAY_AGAIN
                    } else {
                        labels::TRY_AGAIN
                    },
                    "A new expedition from dive 1, on a new sea.",
                ),
            };
            Results::new(summary, pages, Back::Title(TITLE_NAME)).again(label, help, move |ctx| {
                if mode == Mode::Glow && !crate::events::glow_open(today(ctx)) {
                    ctx.play_ui(UiSound::Back);
                    ctx.pop_to_with(TITLE_NAME, crate::events::GLOW_OVER);
                    return;
                }
                let dive = crate::dive_on(ctx, mode);
                ctx.replace(dive)
            })
        }
    }
}

/// A dive screen for `ch`, from its first dive.
pub fn challenge_dive(ch: &Challenge) -> DiveScreen {
    DiveScreen::new(
        Expedition::new(challenge_config(ch), ch.rng()),
        Mode::Challenge(ch.period),
        Some(ch.clone()),
    )
}

/// Stands in for the title when the launcher opens a challenge directly:
/// it opens the briefing, and once the run is over (or left from the
/// pause menu) it steps aside, back to the launcher.
pub struct ChallengeHost {
    period: Period,
    opened: bool,
}

impl ChallengeHost {
    pub fn new(period: Period) -> ChallengeHost {
        ChallengeHost {
            period,
            opened: false,
        }
    }
}

impl State<Parlor> for ChallengeHost {
    /// Named as the challenges menu is, so leaving a run, by its results
    /// or the pause menu, lands here and steps aside to the launcher.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            let pages = briefing(ctx, self.period);
            ctx.push(pages);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, _result: &str) {
        ctx.pop();
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        "Opening the Sonar Dive challenge.".to_string()
    }
}

/// The config a challenge runs on.
pub fn challenge_config(ch: &Challenge) -> crate::rules::Config {
    use crate::rules::Config;
    match ch.period {
        Period::Daily => Config::daily(ch.seed),
        Period::Weekly => Config::weekly(ch.seed),
        Period::Monthly => Config::monthly(),
    }
}

/// The challenge's own name.
pub fn challenge_name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Sprint dive",
        Period::Weekly => "Long haul",
        Period::Monthly => "The sunken galleon",
    }
}

/// A few words on what the challenge is, for its row in the list.
pub fn challenge_tag(ch: &Challenge) -> String {
    let c = challenge_config(ch);
    match ch.period {
        Period::Daily => format!("one dive, {}", c.modifier.name()),
        Period::Weekly => format!("3 dives, no repairs, {}", c.modifier.name()),
        Period::Monthly => "a 1000-coin galleon each dive, one hull point".to_string(),
    }
}

/// What the challenge asks, in a sentence or two.
pub fn challenge_rule(ch: &Challenge) -> String {
    let c = challenge_config(ch);
    let m = c.modifier;
    match ch.period {
        Period::Daily => format!(
            "One dive, starting at dive {}. Today's twist, {}: {}. Surface with as much treasure as you can.",
            c.start_dive,
            m.name(),
            m.describe()
        ),
        Period::Weekly => format!(
            "Up to 3 dives from dive 3, and surfacing does not repair your hull. This week's twist, {}: {}.",
            m.name(),
            m.describe()
        ),
        Period::Monthly => "Each dive, a galleon holding 1000 coins lies near the edge of the site, ringed by 6 mines. You have 2 dives from dive 4 and a single hull point: one mine ends the run. Salvaging the galleon takes 6 seconds of stillness.".to_string(),
    }
}

/// The challenge in one short page, for its briefing.
pub fn challenge_brief(ch: &Challenge) -> String {
    let c = challenge_config(ch);
    let twist = match c.modifier.describe() {
        "" => String::new(),
        d => format!(" Twist: {}.", sentence_start(d)),
    };
    let name = challenge_name(ch.period);
    match ch.period {
        Period::Daily => format!("{name}: one dive.{twist} Most treasure wins."),
        Period::Weekly => format!("{name}: 3 dives, no hull repairs.{twist} Most treasure wins."),
        Period::Monthly => format!(
            "{name}: a 1000-coin galleon each dive, ringed by mines. 2 dives, one hull point. Most treasure wins."
        ),
    }
}

/// The pages read before a challenge run, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{} {}", challenge_brief(&ch), ch.resets())];
    let mut dive = Some(challenge_dive(&ch));
    Pages::briefing(pages).then(move |ctx| {
        if let Some(dive) = dive.take() {
            ctx.replace(dive);
        }
    })
}

/// What the score hears: gold aboard, the dive's depth and the air used
/// build it; under a third of the air left hurries it; a mine within
/// earshot darkens it.
pub fn mood(exp: &Expedition) -> Mood {
    let d = &exp.dive;
    let air = if d.max_oxygen > 0.0 {
        (d.oxygen / d.max_oxygen).clamp(0.0, 1.0) as f32
    } else {
        1.0
    };
    let gold = (d.carried as f32 / 10.0).min(1.0);
    let deep = (d.layout.number.saturating_sub(1) as f32 / 4.0).min(1.0);
    let mine = d
        .nearest_mine()
        .is_some_and(|(_, dist)| dist <= MINE_HEARING);
    Mood::new(0.15 + 0.3 * gold + 0.25 * deep + 0.3 * (1.0 - air))
        .with_flag(HURRY, !d.practice && air < 1.0 / 3.0)
        .with_flag(DANGER, mine)
}

mod visuals_divescreen;
