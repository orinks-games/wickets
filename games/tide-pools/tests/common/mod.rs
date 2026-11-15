//! Shared test helpers: a stand-in launcher, and a twin of the screen's
//! outing that tests step alongside the keys, so they know the shore.

#![allow(dead_code)]

use hearken::{Ctx, Headless, Key, State};
use parlor_core::{Date, Parlor};
use tide_pools::rules::{self, Ev, Lean, Outing, Species};

pub const SEED: u64 = 11;
pub const TODAY: Date = Date::new(2026, 10, 16);

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

/// The stand-in launcher alone, with the date pinned.
pub fn base() -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(TODAY);
    // Quick, with the voice reported quiet each frame (see `Twin::frame`),
    // keeps the original timings, so the twin keeps step with the screen.
    parlor_core::pace::set(&mut h.app.ctx, tide_pools::ID, parlor_core::Pace::Quick);
    h.frame(0.0);
    h
}

/// The game's title on top of the stand-in launcher.
pub fn title() -> Headless<Parlor> {
    let mut h = base();
    h.app.ctx.push(tide_pools::title());
    h.frame(0.0);
    h
}

pub fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

pub fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

/// Moves the title's cursor to the row starting with `label` and presses
/// Enter.
pub fn choose(h: &mut Headless<Parlor>, label: &str) {
    for _ in 0..20 {
        let lines = h.app.top_lines();
        if lines
            .iter()
            .any(|l| l.starts_with("> ") && l[2..].starts_with(label))
        {
            h.press(Key::Return);
            return;
        }
        h.press(Key::Down);
    }
    panic!("no row starting {label:?}: {:?}", h.app.top_lines());
}

/// Presses Enter until the top screen is not pages.
pub fn read_pages(h: &mut Headless<Parlor>) {
    for _ in 0..12 {
        if h.app.top_name() != "pages" {
            return;
        }
        h.press(Key::Return);
    }
}

/// The screen's outing, mirrored: every key and frame is applied to both.
pub struct Twin {
    pub h: Headless<Parlor>,
    pub o: Outing,
    /// What the twin's last step produced.
    pub evs: Vec<Ev>,
}

impl Twin {
    pub fn new(h: Headless<Parlor>, o: Outing) -> Twin {
        Twin {
            h,
            o,
            evs: Vec::new(),
        }
    }

    pub fn press(&mut self, key: Key) {
        self.h.press(key);
        let mut evs = match key {
            Key::Left => self.o.walk(-1),
            Key::Right => self.o.walk(1),
            Key::Up => self.o.lean_in(),
            Key::Down => self.o.step_back(),
            Key::Space => self.o.gather(),
            _ => Vec::new(),
        };
        evs.extend(self.o.update(0.0));
        self.evs = evs;
    }

    pub fn frame(&mut self, dt: f64) {
        // The voice quiet, so the shore runs as it did before it held for
        // speech: `pacing.rs` tests the hold itself.
        self.h.app.ctx.stop_speech();
        self.h.frame(dt);
        self.evs = self.o.update(dt);
    }

    pub fn walk_to(&mut self, pool: usize) {
        while self.o.at < pool {
            self.press(Key::Right);
        }
        while self.o.at > pool {
            self.press(Key::Left);
        }
    }

    pub fn lean_in(&mut self) {
        while self.o.lean != Lean::In {
            self.press(Key::Up);
        }
    }

    /// Waits, a frame at a time, for `species` to sound in this pool, then
    /// reaches in. Returns what the gather did.
    pub fn gather(&mut self, species: Species) -> Vec<Ev> {
        for _ in 0..2000 {
            self.frame(0.05);
            let heard = self.evs.iter().rev().find_map(|e| match e {
                Ev::Call {
                    species: s,
                    offset: 0,
                    ..
                } => Some(*s),
                _ => None,
            });
            if heard == Some(species) {
                self.press(Key::Space);
                return self.evs.clone();
            }
            if self.o.finished {
                break;
            }
        }
        panic!("never heard the {}", species.name());
    }

    /// Walks to the pool holding `species`, leans in and gathers it.
    pub fn fetch(&mut self, species: Species) -> Vec<Ev> {
        let pool = self
            .o
            .pools
            .iter()
            .position(|p| p.finds.iter().any(|f| f.species == species && !f.gathered))
            .unwrap_or_else(|| panic!("no {} on this shore", species.name()));
        self.walk_to(pool);
        self.lean_in();
        self.gather(species)
    }

    /// Lets the tide come in.
    pub fn wait_for_tide(&mut self) {
        for _ in 0..2000 {
            if self.o.finished {
                return;
            }
            self.frame(0.5);
        }
        panic!("the tide never came in");
    }
}

/// Lets an outing run until `species` sounds in the pool in front, then
/// gathers it. For the bare rules.
pub fn listen_and_gather(o: &mut Outing, species: Species) -> Vec<Ev> {
    for _ in 0..4000 {
        let evs = o.update(0.05);
        let heard = evs.iter().rev().find_map(|e| match e {
            Ev::Call {
                species: s,
                offset: 0,
                ..
            } => Some(*s),
            _ => None,
        });
        if heard == Some(species) {
            return o.gather();
        }
    }
    panic!("never heard the {}", species.name());
}
