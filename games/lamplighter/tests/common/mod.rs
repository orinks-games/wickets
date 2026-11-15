//! Shared test helpers: a stand-in launcher, and a walker that lights a
//! hall by its lamps' places, as a player would.

#![allow(dead_code)]

use hearken::{Ctx, Headless, Key, State};
use lamplighter::rules::{self, Hall, Side, PRACTICE, ROUND};
use lamplighter::{Mode, Round};
use parlor_core::{Parlor, ParlorData};

pub const SEED: u64 = 42;
pub const QUIET_DAY: parlor_core::Date = parlor_core::Date::new(2026, 10, 16);
pub const DT: f64 = 1.0 / 60.0;

/// A screen standing in for the launcher.
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

/// The title on the stand-in launcher, tutorial read.
pub fn title() -> Headless<Parlor> {
    title_with(Default::default())
}

pub fn title_with(data: ParlorData) -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(data, Base);
    if h.app.ctx.game.today.is_none() {
        h.app.ctx.game.today = Some(QUIET_DAY);
    }
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, lamplighter::ID);
    h.app.ctx.push(lamplighter::title());
    h.frame(0.0);
    h
}

pub fn cursor_row(h: &Headless<Parlor>) -> String {
    h.app
        .top_lines()
        .iter()
        .find_map(|l| l.strip_prefix("> ").map(str::to_string))
        .unwrap_or_default()
}

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

pub fn rows(h: &Headless<Parlor>) -> Vec<String> {
    parlor_core::menus::rows_of(&h.app.top_lines())
}

pub fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

pub fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

/// The title, then straight into a round of `mode`, with a twin of its
/// hall that the helpers step alongside the keys.
pub fn play(mode: Mode) -> (Headless<Parlor>, Hall) {
    let mut h = title();
    h.app.ctx.push(Round::with_seed(mode, SEED));
    h.frame(0.0);
    let mut twin = match mode {
        Mode::Practice => Hall::new(PRACTICE, SEED),
        _ => Hall::new(ROUND, SEED),
    };
    if !twin.timed() {
        twin.start();
    }
    (h, twin)
}

/// Lights the taper, on the screen and the twin.
pub fn light_taper(h: &mut Headless<Parlor>, twin: &mut Hall) {
    h.press(Key::Space);
    twin.start();
}

/// Walks to the nearest lamp out and lights it, by keys and on the twin.
/// Returns false when none is out.
pub fn light_nearest(h: &mut Headless<Parlor>, twin: &mut Hall) -> bool {
    let Some((ahead, side)) = twin.nearest_out() else {
        return false;
    };
    let key = if ahead > 0 { Key::Up } else { Key::Down };
    for _ in 0..ahead.abs() {
        h.press(key);
        twin.walk(ahead > 0);
    }
    h.press(match side {
        Side::Left => Key::Left,
        Side::Right => Key::Right,
    });
    twin.reach(side);
    true
}

/// Lights every lamp in the hall.
pub fn light_all(h: &mut Headless<Parlor>, twin: &mut Hall) {
    for _ in 0..200 {
        if twin.finished() || !light_nearest(h, twin) {
            return;
        }
    }
    panic!("the hall never lit");
}

/// Frames of time.
pub fn wait(h: &mut Headless<Parlor>, secs: f64) {
    let frames = (secs / DT).round() as usize;
    for _ in 0..frames {
        h.frame(DT);
    }
}

/// Presses Enter through every page until another screen is up.
pub fn read_pages(h: &mut Headless<Parlor>) {
    let mut n = 0;
    while h.app.top_name() == "pages" && n < 20 {
        h.press(Key::Return);
        n += 1;
    }
}
