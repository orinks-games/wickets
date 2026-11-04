//! Shared test helpers: a stand-in launcher, and a twin of the screen's run
//! that tests step alongside the keys, so they always know the next call.

#![allow(dead_code)]

use graveyard_shuffle::play::{plans, Floor, Mode};
use graveyard_shuffle::rules::{Ev, Phase, Run};
use graveyard_shuffle::songs::Monster;
use hearken::{Ctx, Headless, Key, State};
use parlor_core::{Date, Parlor, ParlorData};

pub const SEED: u64 = 42;
/// One frame at 60 frames a second.
pub const DT: f64 = 1.0 / 60.0;
/// A day in Halloween season.
pub const PLAIN_DAY: Date = Date::new(2026, 10, 20);

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

pub fn data(date: Date) -> ParlorData {
    ParlorData {
        today: Some(date),
        ..Default::default()
    }
}

/// The title on the stand-in launcher, tutorial read, so the cursor starts
/// on Play.
pub fn title() -> Headless<Parlor> {
    title_on(PLAIN_DAY)
}

pub fn title_on(date: Date) -> Headless<Parlor> {
    let mut h = Headless::new(data(date), Base);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, graveyard_shuffle::ID);
    h.app.ctx.push(graveyard_shuffle::title());
    h.frame(0.0);
    h
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

pub fn rows(h: &Headless<Parlor>) -> Vec<String> {
    parlor_core::menus::rows_of(&h.app.top_lines())
}

pub fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

pub fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

pub fn played_count(h: &Headless<Parlor>, key: &str) -> usize {
    h.played().iter().filter(|k| *k == key).count()
}

/// The twin of a floor of `mode` from `SEED`.
pub fn twin(_ctx: &mut Ctx<Parlor>, mode: Mode) -> Run {
    Run::new(plans(mode, SEED), matches!(mode, Mode::Practice(_)))
}

/// The title on `date`, then straight onto the floor in `mode`, with its
/// twin.
pub fn floor_on(date: Date, mode: Mode) -> (Headless<Parlor>, Run) {
    let mut h = title_on(date);
    let run = twin(&mut h.app.ctx, mode);
    let f = Floor::with_seed(&mut h.app.ctx, mode, SEED);
    h.app.ctx.push(f);
    h.frame(0.0);
    (h, run)
}

pub fn floor(mode: Mode) -> (Headless<Parlor>, Run) {
    floor_on(PLAIN_DAY, mode)
}

/// The key that dances `step` by default.
pub fn key_for(step: Monster) -> Key {
    match step {
        Monster::Vampire => Key::Left,
        Monster::Werewolf => Key::Right,
        Monster::Mummy => Key::Space,
        Monster::Ghost => Key::Tab,
        Monster::Skeleton => Key::Up,
        Monster::Zombie => Key::Down,
    }
}

/// A step that is not `step`.
pub fn wrong_for(step: Monster) -> Monster {
    if step == Monster::Zombie {
        Monster::Skeleton
    } else {
        Monster::Zombie
    }
}

/// Starts the dance, on the screen and the twin.
pub fn start(h: &mut Headless<Parlor>, twin: &mut Run) {
    h.press(Key::Space);
    twin.start();
}

/// One frame on the screen and the twin. Returns the twin's events.
pub fn frame(h: &mut Headless<Parlor>, twin: &mut Run) -> Vec<graveyard_shuffle::rules::RunEv> {
    h.frame(DT);
    twin.update(DT)
}

/// Frames of time on the screen and the twin.
pub fn wait(
    h: &mut Headless<Parlor>,
    twin: &mut Run,
    secs: f64,
) -> Vec<graveyard_shuffle::rules::RunEv> {
    let mut evs = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        evs.extend(frame(h, twin));
    }
    evs
}

/// Dances `step` by key and on the twin.
pub fn press(h: &mut Headless<Parlor>, twin: &mut Run, step: Monster) -> Vec<Ev> {
    h.press(key_for(step));
    twin.press(step)
}

/// Frames until the next open call's beat, then dances it right.
/// Returns false when the tune has no more calls.
pub fn dance_next(h: &mut Headless<Parlor>, twin: &mut Run) -> bool {
    let Some(i) = twin.dance.next_open() else {
        return false;
    };
    let c = twin.dance.plan.calls[i];
    let at = twin.dance.time_of(c.beat as i64);
    while twin.dance.now() < at && twin.dance.waiting().is_none() {
        frame(h, twin);
    }
    press(h, twin, c.step);
    true
}

/// Dances every call of the run spot on, through every tune, until the
/// screen leaves the floor.
pub fn dance_all(h: &mut Headless<Parlor>, twin: &mut Run) {
    for _ in 0..200_000 {
        if twin.finished() {
            break;
        }
        if twin.phase == Phase::Dancing && dance_next(h, twin) {
            continue;
        }
        frame(h, twin);
    }
    assert!(twin.finished(), "the run never finished");
}

/// Frames until the screen shows something other than the floor.
pub fn wait_off_floor(h: &mut Headless<Parlor>) {
    for _ in 0..1000 {
        if h.app.top_name() != graveyard_shuffle::play::PLAY_NAME {
            return;
        }
        h.frame(DT);
    }
    panic!("still on the floor");
}

/// Presses Enter through every page until another screen is up.
pub fn read_pages(h: &mut Headless<Parlor>) {
    let mut n = 0;
    while h.app.top_name() == "pages" && n < 20 {
        h.press(Key::Return);
        n += 1;
    }
}
