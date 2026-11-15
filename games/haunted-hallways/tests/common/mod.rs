//! Shared test helpers: a stand-in launcher, and a twin of the screen's
//! run that tests step alongside the keys, so they always know the layout.

#![allow(dead_code)]

use haunted_hallways::rules::{self, Ev, Event, Mode, Puzzle, Run, Side};
use haunted_hallways::ID;
use hearken::{Ctx, Headless, Key, State};
use parlor_core::{pace, Challenge, Date, Pace, Parlor, Period};

/// The day challenge tests are pinned to.
pub const TODAY: Date = Date::new(2026, 10, 16);

pub const SEED: u64 = 31;

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

/// The game's title on top of the stand-in launcher, with the tutorial
/// read, so the cursor starts on Play. The pace is quick, the original
/// timing, so a twin run stepped by the same frames stays in step, with
/// the voice reported quiet each frame (see [`Twin::frame`]).
/// tests/pacing.rs covers the pace and the speech hold.
pub fn title() -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(TODAY);
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    if pace::stored(&h.app.ctx, ID).is_none() {
        pace::set(&mut h.app.ctx, ID, Pace::Quick);
    }
    h.app.ctx.push(haunted_hallways::title());
    h.frame(0.0);
    h
}

/// The rows of the menu on top.
pub fn rows(h: &Headless<Parlor>) -> Vec<String> {
    parlor_core::menus::rows_of(&h.app.top_lines())
}

/// Moves the cursor to the first row starting with `label`.
pub fn goto(h: &mut Headless<Parlor>, label: &str) {
    let rows = rows(h);
    let at = rows
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row starts with {label:?}: {rows:?}"));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

pub fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

pub fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

pub fn key_for(side: Side) -> Key {
    match side {
        Side::North => Key::Up,
        Side::South => Key::Down,
        Side::East => Key::Right,
        Side::West => Key::Left,
    }
}

/// The straight steps from where the run stands to a cell, east-west
/// first, whatever lies between.
pub fn straight(run: &Run, to: (i32, i32)) -> Vec<Side> {
    let mut out = Vec::new();
    let (dx, dy) = (to.0 - run.x, to.1 - run.y);
    let ew = if dx > 0 { Side::East } else { Side::West };
    let ns = if dy > 0 { Side::North } else { Side::South };
    out.extend(std::iter::repeat_n(ew, dx.unsigned_abs() as usize));
    out.extend(std::iter::repeat_n(ns, dy.unsigned_abs() as usize));
    out
}

/// The shortest safe steps from where the run stands to a cell: around
/// rotten boards, holes and the shadow. East-west first when that is as
/// short, so plain rooms walk as they always have.
pub fn route(run: &Run, to: (i32, i32)) -> Vec<Side> {
    let room = run.current();
    let blocked = |c: (i32, i32)| {
        room.rotten.contains(&c)
            || room.holes.contains(&c)
            || room.shadow.is_some_and(|s| (s.x, s.y) == c)
            || room.pendulum.is_some_and(|p| (p.x, p.row) == c)
            || run.founder.is_some_and(|f| (f.x, f.y) == c)
    };
    let direct = straight(run, to);
    let mut at = (run.x, run.y);
    let clear = direct.iter().all(|s| {
        let (dx, dy) = s.delta();
        at = (at.0 + dx, at.1 + dy);
        !blocked(at)
    });
    if clear {
        return direct;
    }
    let start = (run.x, run.y);
    let mut prev = std::collections::HashMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    prev.insert(start, None);
    while let Some(c) = queue.pop_front() {
        if c == to {
            break;
        }
        for s in [Side::East, Side::West, Side::North, Side::South] {
            let (dx, dy) = s.delta();
            let n = (c.0 + dx, c.1 + dy);
            if n.0 < 0 || n.1 < 0 || n.0 >= room.w || n.1 >= room.h {
                continue;
            }
            if (blocked(n) && n != to) || prev.contains_key(&n) {
                continue;
            }
            prev.insert(n, Some((c, s)));
            queue.push_back(n);
        }
    }
    let mut out = Vec::new();
    let mut c = to;
    while let Some(Some((p, s))) = prev.get(&c) {
        out.push(*s);
        c = *p;
    }
    out.reverse();
    out
}

/// Plays a room to the end with a bare run: walk to the answer and take
/// it (both, in order, for a séance), or choose the offering and give it,
/// then walk out.
pub fn solve_run_room(run: &mut Run) -> Vec<Ev> {
    let mut evs = Vec::new();
    let room = run.current().clone();
    if let Puzzle::Offering(want) = room.puzzle {
        while run.held() != Some(want) {
            evs.extend(run.cycle());
        }
    }
    for _ in 0..room.puzzle.wanted().len() {
        let goal = run.goal();
        for side in route(run, goal) {
            evs.extend(bare_walk(run, side));
        }
        evs.extend(run.interact());
    }
    let door = room.inside(room.exit);
    for side in route(run, door) {
        evs.extend(bare_walk(run, side));
    }
    evs.extend(run.walk(room.exit.side));
    evs
}

/// A step with a bare run, where no time passes: wading ends at once.
pub fn bare_walk(run: &mut Run, side: Side) -> Vec<Ev> {
    let evs = run.walk(side);
    run.wade = 0.0;
    evs
}

/// The screen and a run kept in step with it.
pub struct Twin {
    pub h: Headless<Parlor>,
    pub run: Run,
}

impl Twin {
    /// Into a haunt from the title, with the title's row for it.
    pub fn from_title(mut h: Headless<Parlor>, mode: Mode) -> Twin {
        match mode {
            Mode::Climb(_) => h.press(Key::Return),
            Mode::Nightmare => {
                h.press(Key::N);
                h.press(Key::Return);
            }
            Mode::Endless => {
                h.press(Key::E);
                h.press(Key::Return);
            }
            Mode::Floor(n) => {
                h.press(Key::F); // Floor select
                h.press(Key::Return);
                for _ in 0..(n - 1) / 10 {
                    h.press(Key::Down);
                }
                h.press(Key::Return);
                for _ in 0..(n - 1) % 10 {
                    h.press(Key::Down);
                }
                h.press(Key::Return);
            }
            Mode::Challenge(e) => return Twin::challenge(h, e.period),
            Mode::Stirs(_) | Mode::LanternNight(_) => {
                panic!("Halloween runs open from their own rows")
            }
        }
        assert_eq!(
            h.app.top_name(),
            "haunted_hallways_haunt",
            "{:?}",
            h.spoken()
        );
        Twin {
            h,
            run: Run::new(mode, SEED),
        }
    }

    /// Into a challenge from the title's Challenges menu, through its
    /// briefing, with a run built from the same challenge.
    pub fn challenge(mut h: Headless<Parlor>, period: Period) -> Twin {
        goto(&mut h, "Challenges");
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "haunted_hallways_challenges");
        let row = Period::ALL.iter().position(|&p| p == period).unwrap();
        for _ in 0..row {
            h.press(Key::Down);
        }
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "pages");
        h.press(Key::Return);
        assert_eq!(
            h.app.top_name(),
            "haunted_hallways_haunt",
            "{:?}",
            h.spoken()
        );
        let ch = Challenge::current(&h.app.ctx, ID, period);
        let (event, seed) = Event::from_challenge(&ch);
        Twin {
            h,
            run: Run::new(Mode::Challenge(event), seed),
        }
    }

    pub fn start(mode: Mode) -> Twin {
        Twin::from_title(title(), mode)
    }

    pub fn walk(&mut self, side: Side) -> Vec<Ev> {
        self.h.press(key_for(side));
        let evs = self.run.walk(side);
        if evs.contains(&Ev::Splash) {
            // Wade out of it before the next step, screen and twin alike.
            self.frame(rules::WADE_TIME + 0.01);
        }
        evs
    }

    pub fn walk_to(&mut self, to: (i32, i32)) {
        for side in route(&self.run, to) {
            self.walk(side);
        }
    }

    pub fn interact(&mut self) -> Vec<Ev> {
        self.h.press(Key::Space);
        self.run.interact()
    }

    pub fn cycle(&mut self) -> Vec<Ev> {
        self.h.press(Key::Tab);
        self.run.cycle()
    }

    /// A frame of `dt` with the voice already quiet, so the speech hold
    /// never stops the house and the twin stays in step.
    pub fn frame(&mut self, dt: f64) -> Vec<Ev> {
        self.h.app.ctx.stop_speech();
        self.h.frame(dt);
        self.run.update(dt)
    }

    /// Solves the room the run is in and walks out of it.
    pub fn solve_room(&mut self) {
        let room = self.run.current().clone();
        if let Puzzle::Offering(want) = room.puzzle {
            while self.run.held() != Some(want) {
                self.cycle();
            }
        }
        for _ in 0..room.puzzle.wanted().len() {
            let goal = self.run.goal();
            self.walk_to(goal);
            self.interact();
        }
        assert!(self.run.current().solved);
        let door = room.inside(room.exit);
        self.walk_to(door);
        self.walk(room.exit.side);
    }

    /// Reads pages until the stack is back at `name`, leaving the results
    /// menu by Escape, which does what its Back row does.
    pub fn read_until(&mut self, name: &str) {
        for _ in 0..10 {
            if self.h.app.top_name() == name {
                return;
            }
            if self.h.app.top_name() == "results" {
                self.h.press(Key::Escape);
            } else {
                self.h.press(Key::Return);
            }
        }
        panic!("never reached {name}: {:?}", self.h.spoken());
    }
}
