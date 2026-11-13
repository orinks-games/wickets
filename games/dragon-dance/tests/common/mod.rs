//! Shared test helpers: a stand-in launcher, the title on a pinned date,
//! and an autopilot that dances a street the way a good player would.

#![allow(dead_code)]

use dragon_dance::rules::{self, Dance, Event, Thing};
use dragon_dance::{sound, Launch, Mode};
use hearken::{Ctx, Headless, Key, State};
use parlor_core::{Date, Parlor, ParlorData};

pub const SEED: u64 = 42;

/// An ordinary autumn day: no festival.
pub const ORDINARY: Date = Date::new(2026, 10, 22);

/// A screen standing in for the launcher, so leaving the game has
/// somewhere to go.
pub struct Base;

impl State<Parlor> for Base {
    fn name(&self) -> &'static str {
        "base"
    }
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.say("Base.");
    }
    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        String::new()
    }
}

/// The launcher stand-in with the date pinned, nothing opened yet.
pub fn base_on(date: Date) -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let data = ParlorData {
        today: Some(date),
        ..Default::default()
    };
    let mut h = Headless::new(data, Base);
    h.frame(0.0);
    h
}

/// The title on `date`, tutorial read, so the cursor starts on Play.
pub fn title_on(date: Date) -> Headless<Parlor> {
    let mut h = base_on(date);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, dragon_dance::ID);
    h.app.ctx.push(dragon_dance::title());
    h.frame(0.0);
    h
}

/// The title on an ordinary day.
pub fn title() -> Headless<Parlor> {
    title_on(ORDINARY)
}

/// The title, then straight into `mode` with the test seed.
pub fn dance_on(date: Date, mode: Mode) -> Headless<Parlor> {
    let mut h = title_on(date);
    dragon_dance::dance::start(&mut h.app.ctx, Launch { mode, seed: SEED });
    h.frame(0.0);
    h
}

pub fn dance(mode: Mode) -> Headless<Parlor> {
    dance_on(ORDINARY, mode)
}

pub fn rows(h: &Headless<Parlor>) -> Vec<String> {
    parlor_core::menus::rows_of(&h.app.top_lines())
}

/// The cursor row of the menu on top, without its marker.
pub fn cursor_row(h: &Headless<Parlor>) -> String {
    h.app
        .top_lines()
        .iter()
        .find_map(|l| l.strip_prefix("> ").map(str::to_string))
        .unwrap_or_default()
}

/// Moves the cursor of the menu on top to the first row starting with
/// `label`, by the arrows.
pub fn goto(h: &mut Headless<Parlor>, label: &str) {
    h.press(Key::Home);
    for _ in 0..40 {
        if cursor_row(h).starts_with(label) {
            return;
        }
        h.press(Key::Down);
    }
    panic!("no row starts with {label:?}: {:?}", h.app.top_lines());
}

pub fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

pub fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

/// Presses Enter on a row that starts a street, then Enter again through
/// the street's briefing.
pub fn start(h: &mut Headless<Parlor>) {
    h.press(Key::Return);
    if h.app.top_name() == parlor_core::briefing::NAME {
        h.press(Key::Return);
    }
}

/// Time passing, in frames of a sixtieth of a second.
pub fn wait(h: &mut Headless<Parlor>, secs: f64) {
    let frames = (secs * 60.0).round() as usize;
    for _ in 0..frames {
        h.frame(1.0 / 60.0);
    }
}

/// The head's lane and the pearl's, from 0, as the dance screen's lines
/// show them: "lane 2 of 3, pearl 2, beat 5".
pub fn lanes(h: &Headless<Parlor>) -> (usize, usize) {
    let line = &h.app.top_lines()[1];
    let num = |after: &str| -> usize {
        line.split(after)
            .nth(1)
            .and_then(|t| t.split([' ', ',']).next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("no {after:?} in {line:?}"))
    };
    (num("lane ") - 1, num("pearl ") - 1)
}

/// Plays the street on top by the keys, as a good player would: follow
/// the pearl, rear on each gong. Returns everything spoken, in order.
/// Stops when the dance screen gives way to another.
pub fn dance_by_keys(h: &mut Headless<Parlor>, max_secs: f64) -> Vec<String> {
    let name = h.app.top_name();
    let mut heard = Vec::new();
    let frames = (max_secs * 60.0) as usize;
    for _ in 0..frames {
        if h.app.top_name() != name {
            break;
        }
        let (lane, pearl) = lanes(h);
        if lane < pearl {
            h.press(Key::Right);
        } else if lane > pearl {
            h.press(Key::Left);
        }
        let gong = played(h, sound::GONG) || played(h, sound::GREAT_GONG);
        if gong && h.app.top_name() == name {
            h.press(Key::Space);
        }
        heard.extend(h.spoken());
        h.clear();
        h.frame(1.0 / 60.0);
    }
    heard.extend(h.spoken());
    heard
}

/// What the autopilot did over a street.
#[derive(Clone, Debug, PartialEq)]
pub struct Danced {
    pub score: u64,
    pub follow: u32,
    pub envelopes: u32,
    pub perfect: u32,
    pub stumbles: u32,
    pub segments: u32,
    pub events: usize,
}

/// Dances a whole street in frames of `dt` with the rules alone: follows
/// the pearl, rears on every gong, and, with `greedy`, steps out for
/// each envelope just before it arrives and back after.
pub fn autopilot(dance: &mut Dance, dt: f64, greedy: bool) -> Danced {
    let mut events = 0;
    let mut target: Option<usize> = None;
    for _ in 0..200_000 {
        if dance.finished {
            break;
        }
        dance.update(dt);
        let mut got = dance.take_events();
        for e in got.clone() {
            match e {
                Event::Gong { .. } => {
                    dance.rear();
                    got.extend(dance.take_events());
                }
                Event::Envelope { .. } | Event::EnvelopeMissed { .. } => target = None,
                Event::Lantern { .. } | Event::LanternMissed { .. } => target = None,
                _ => {}
            }
        }
        events += got.len();
        if greedy {
            if let Some(item) = dance
                .incoming()
                .into_iter()
                .find(|i| i.thing != Thing::Firecracker && i.beats <= 0.5)
            {
                target = Some(item.lane);
            }
        }
        let want = target.unwrap_or(dance.pearl);
        while dance.lane != want {
            dance.step(if dance.lane < want { 1 } else { -1 });
        }
    }
    Danced {
        score: dance.score,
        follow: dance.follow_beats,
        envelopes: dance.envelopes,
        perfect: dance.perfect,
        stumbles: dance.stumbles,
        segments: dance.segments,
        events,
    }
}

/// A street of `mode`'s shape with the test seed.
pub fn rules_dance(mode: Mode) -> Dance {
    Dance::new(mode.street(), SEED)
}
