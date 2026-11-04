//! Shared test helpers: a stand-in launcher, and a twin rank the
//! helpers tune alongside the keys.

#![allow(dead_code)]

use hearken::{Ctx, Headless, Key, State};
use parlor_core::{Parlor, ParlorData};
use the_echo::play::{END_PAUSE, TUNE_GAP};
use the_echo::rules::{self, Rank, PRACTICE, RANK, TUNE_LEN};
use the_echo::{Mode, Tuning};

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
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, the_echo::ID);
    h.app.ctx.push(the_echo::title());
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

/// The title, then straight into a rank of `mode`, with a twin of its
/// rank that the helpers step alongside the keys.
pub fn play(mode: Mode) -> (Headless<Parlor>, Rank) {
    let mut h = title();
    h.app.ctx.push(Tuning::with_seed(mode, SEED));
    h.frame(0.0);
    let twin = match mode {
        Mode::Practice => Rank::new(PRACTICE, SEED),
        _ => Rank::new(RANK, SEED),
    };
    (h, twin)
}

/// Tunes the pipe now chosen, by keys and on the twin, then sets it.
pub fn tune_pipe(h: &mut Headless<Parlor>, twin: &mut Rank) {
    let off = twin.pipe().off;
    let (key, up) = if off > 0 {
        (Key::Down, false)
    } else {
        (Key::Up, true)
    };
    for _ in 0..off.abs() {
        h.press(key);
        twin.nudge(up);
    }
    h.press(Key::Space);
    twin.set();
}

/// Tunes every pipe in the rank.
pub fn tune_all(h: &mut Headless<Parlor>, twin: &mut Rank) {
    for _ in 0..20 {
        if twin.finished() {
            return;
        }
        tune_pipe(h, twin);
    }
    panic!("the rank never got tuned");
}

/// Frames until the tune has played and the results are up.
pub fn hear_tune(h: &mut Headless<Parlor>) {
    wait(h, 1.0 + TUNE_LEN as f64 * TUNE_GAP + END_PAUSE + 0.5);
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
