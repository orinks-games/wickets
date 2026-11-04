//! Shared test helpers: a stand-in launcher, the title on a pinned date,
//! a run on a fixed seed, and a bow held and let go by the keys.

#![allow(dead_code)]

use cupids_quiver::field::{self, Field, Launch};
use cupids_quiver::rules::{Mode, Release, Run, Target};
use hearken::{Ctx, Headless, InputEvent, Key, State};
use parlor_core::{Date, Parlor, ParlorData, Rng};

pub const SEED: u64 = 42;

/// An ordinary autumn day, far from Valentine's Day.
pub const AUTUMN: Date = Date::new(2026, 10, 16);
/// In the Valentine's season, 4 days before.
pub const SEASON: Date = Date::new(2027, 2, 10);
/// Valentine's Day.
pub const DAY: Date = Date::new(2027, 2, 14);

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

/// The stand-in launcher on `today`, nothing else open.
pub fn base(today: Date) -> Headless<Parlor> {
    field::fix_seed(Some(SEED));
    let data = ParlorData {
        today: Some(today),
        ..Default::default()
    };
    let mut h = Headless::new(data, Base);
    h.frame(0.0);
    h
}

/// The title on `today`, tutorial read, so the cursor starts on Play.
pub fn title_on(today: Date) -> Headless<Parlor> {
    let mut h = base(today);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, cupids_quiver::ID);
    h.app.ctx.push(cupids_quiver::title());
    h.frame(0.0);
    h
}

pub fn title() -> Headless<Parlor> {
    title_on(AUTUMN)
}

/// The title on `today`, then straight into a run of `mode`.
pub fn field_on(mode: Mode, today: Date) -> Headless<Parlor> {
    let mut h = title_on(today);
    h.app.ctx.push(Field::new(Launch::seeded(mode, SEED)));
    h.frame(0.0);
    h
}

pub fn field(mode: Mode) -> Headless<Parlor> {
    field_on(mode, AUTUMN)
}

/// The cursor row of the menu on top, without its marker.
pub fn cursor_row(h: &Headless<Parlor>) -> String {
    h.app
        .top_lines()
        .iter()
        .find_map(|l| l.strip_prefix("> ").map(str::to_string))
        .unwrap_or_default()
}

/// Moves the cursor of the menu on top to the first row starting with `label`.
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

/// Time passing, in frames of a sixtieth of a second.
pub fn wait(h: &mut Headless<Parlor>, secs: f64) {
    let frames = (secs * 60.0).round() as usize;
    for _ in 0..frames {
        h.frame(1.0 / 60.0);
    }
}

/// Holds the main action for `secs`, then lets go.
pub fn hold(h: &mut Headless<Parlor>, secs: f64) {
    h.send(InputEvent::key(Key::Space));
    wait(h, secs);
    h.send(InputEvent::key_up(Key::Space));
}

/// A full draw and a shot, then the arrow's flight.
pub fn shoot(h: &mut Headless<Parlor>) {
    hold(h, 1.0);
    wait(h, 0.8);
}

/// The status line the screen shows second.
pub fn status_line(h: &Headless<Parlor>) -> String {
    h.app.top_lines()[1].clone()
}

/// Shoots straight ahead until the run is over and its results are up,
/// climbing to each next rooftop on the way. Returns the rounds played.
pub fn play_out(h: &mut Headless<Parlor>) -> u32 {
    let mut rounds = 1;
    for _ in 0..400 {
        match h.app.top_name() {
            "pages" | "results" => return rounds,
            _ => {}
        }
        if status_line(h).contains("between rooftops") {
            h.press(Key::Return);
            rounds += 1;
            wait(h, 0.2);
            continue;
        }
        shoot(h);
    }
    panic!("the run never ended: {:?}", h.app.top_lines());
}

/// Presses Enter through any pages on top.
pub fn read_pages(h: &mut Headless<Parlor>) {
    for _ in 0..30 {
        if h.app.top_name() != "pages" {
            return;
        }
        h.press(Key::Return);
    }
}

/// A run with nothing drifting yet: the lead-in still to come.
pub fn quiet_run(mode: Mode) -> Run {
    let mut run = Run::new(mode, Rng::seeded(SEED));
    run.delay_spawns(1000.0);
    run
}

/// A target placed by hand.
pub fn target(run: &mut Run, target: Target) {
    run.sky.push(target);
}

/// Draws the string to full, in steps of a sixtieth of a second.
pub fn full_draw(run: &mut Run) {
    run.begin_draw().expect("the bow draws");
    for _ in 0..51 {
        run.update(1.0 / 60.0);
    }
    assert_eq!(run.draw, Some(1.0));
}

/// Draws to full and shoots.
pub fn draw_and_shoot(run: &mut Run) -> Release {
    full_draw(run);
    run.release()
}

/// Runs a second on, and returns the events.
pub fn fly(run: &mut Run) -> Vec<cupids_quiver::rules::Event> {
    let mut events = Vec::new();
    for _ in 0..60 {
        events.extend(run.update(1.0 / 60.0));
    }
    events
}
