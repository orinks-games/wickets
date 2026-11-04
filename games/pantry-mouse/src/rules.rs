//! The rules of the pantry, with nothing spoken or played.
//!
//! A [`Maze`] is a grid of open cells and walls. A [`Run`] puts the mouse,
//! the crumbs and the cats in it. The mouse moves one cell per step, when
//! the player asks; the cats move on their own clock, one cell every
//! `pace` milliseconds. The clock runs on a fixed substep, so the same play
//! gives the same result at any frame rate. Every random choice comes from
//! the run's seeded [`Rng`].

use std::cell::Cell as StdCell;
use std::collections::VecDeque;

use parlor_core::{FixedStep, ParlorAction, Rng};

/// Substeps a second of the run's clock: one substep is 10 milliseconds.
pub const STEP_HZ: f64 = 100.0;
/// Milliseconds in one substep.
pub const STEP_MS: u32 = 10;
/// Whiskers at the start of a run. Each catch costs one.
pub const WHISKERS: u32 = 3;
/// Points for each crumb.
pub const CRUMB_POINTS: u32 = 10;
/// Points for clearing a pantry.
pub const CLEAR_BONUS: u32 = 250;
/// Freezes for each pantry.
pub const FREEZES: u32 = 3;
/// How long a freeze holds the mouse still, in milliseconds.
pub const FREEZE_MS: u32 = 3000;
/// How long the frozen-out cat stays lost after the freeze ends.
pub const LOST_AFTER_MS: u32 = 1000;
/// How long the cats wait after a catch, in milliseconds.
pub const GRACE_MS: u32 = 2000;
/// The pause between a cleared pantry and the next, in milliseconds.
pub const NEXT_PANTRY_MS: u32 = 3000;
/// A cat this many steps away, or nearer, is close.
pub const CLOSE: u32 = 2;
/// A cat this many steps away, or nearer, can be heard.
pub const HEAR_RANGE: u32 = 5;
/// A wanderer chases once it is this many steps away, or nearer.
pub const WANDER_CHASE: u32 = 4;
/// How many crumbs a sniff finds.
pub const SNIFF_CRUMBS: usize = 3;

/// The four ways to move. [`Dir::ALL`] is also the order ties are broken in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    Up,
    Left,
    Down,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Down, Dir::Right];

    /// The direction as spoken: "up".
    pub fn word(self) -> &'static str {
        match self {
            Dir::Up => "up",
            Dir::Left => "left",
            Dir::Down => "down",
            Dir::Right => "right",
        }
    }

    /// One cell's move: up is towards row 0.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Left => (-1, 0),
            Dir::Down => (0, 1),
            Dir::Right => (1, 0),
        }
    }

    pub fn reverse(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Left => Dir::Right,
            Dir::Down => Dir::Up,
            Dir::Right => Dir::Left,
        }
    }

    /// The action that moves this way.
    pub fn action(self) -> ParlorAction {
        match self {
            Dir::Up => ParlorAction::Up,
            Dir::Left => ParlorAction::Left,
            Dir::Down => ParlorAction::Down,
            Dir::Right => ParlorAction::Right,
        }
    }

    /// The direction an action moves, if it moves.
    pub fn for_action(action: ParlorAction) -> Option<Dir> {
        Dir::ALL.into_iter().find(|d| d.action() == action)
    }
}

/// A cell of the maze: `x` from the left, `y` from the top.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
}

impl Cell {
    pub const fn new(x: i32, y: i32) -> Cell {
        Cell { x, y }
    }

    pub fn step(self, dir: Dir) -> Cell {
        let (dx, dy) = dir.delta();
        Cell::new(self.x + dx, self.y + dy)
    }
}

/// The first pantry: 9 by 9, loops everywhere and no dead ends. `#` is a
/// wall, `o` open floor, `M` the mouse's start, and the digits the cats'
/// dens in order.
pub const PANTRY: [&str; 9] = [
    "1ooo#ooo3",
    "o##o#o##o",
    "ooooooooo",
    "o#o###o#o",
    "o#ooMoo#o",
    "o#o###o#o",
    "ooooooooo",
    "o##o#o##o",
    "oooo#ooo2",
];

/// A grid of open cells and walls, with the mouse's start and the dens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Maze {
    pub w: i32,
    pub h: i32,
    open: Vec<bool>,
    /// Where the mouse starts. It holds no crumb.
    pub mouse: Cell,
    /// Where each cat starts, in the order the cats join.
    pub dens: Vec<Cell>,
}

impl Maze {
    /// A maze from rows of text: `#` a wall, `M` the mouse, a digit a den,
    /// anything else open floor.
    pub fn parse(rows: &[&str]) -> Maze {
        let h = rows.len() as i32;
        let w = rows.first().map_or(0, |r| r.len()) as i32;
        let mut open = Vec::new();
        let mut mouse = Cell::new(0, 0);
        let mut dens: Vec<(char, Cell)> = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len() as i32, w, "every row the same width");
            for (x, c) in row.chars().enumerate() {
                let cell = Cell::new(x as i32, y as i32);
                open.push(c != '#');
                if c == 'M' {
                    mouse = cell;
                } else if c.is_ascii_digit() {
                    dens.push((c, cell));
                }
            }
        }
        dens.sort();
        Maze {
            w,
            h,
            open,
            mouse,
            dens: dens.into_iter().map(|(_, c)| c).collect(),
        }
    }

    /// The fixed first pantry.
    pub fn pantry() -> Maze {
        Maze::parse(&PANTRY)
    }

    /// A seeded maze, `w` by `h` (both odd, at least 5). Rooms sit on even
    /// cells and a random spanning tree joins them, so every room is
    /// reachable. Then every dead end gets a second way out, so a cat can
    /// always be dodged. The mouse starts in the middle, the dens are the
    /// corners: top left, bottom right, top right.
    pub fn generate(rng: &mut Rng, w: i32, h: i32) -> Maze {
        assert!(w >= 5 && h >= 5 && w % 2 == 1 && h % 2 == 1, "odd sizes");
        let mut open = vec![false; (w * h) as usize];
        let idx = |c: Cell| (c.y * w + c.x) as usize;
        let room = |c: Cell| c.x >= 0 && c.y >= 0 && c.x < w && c.y < h;
        let jump = |c: Cell, d: Dir| {
            let (dx, dy) = d.delta();
            Cell::new(c.x + 2 * dx, c.y + 2 * dy)
        };
        // A depth-first spanning tree over the rooms.
        let start = Cell::new(0, 0);
        open[idx(start)] = true;
        let mut stack = vec![start];
        while let Some(&here) = stack.last() {
            let mut next: Vec<Dir> = Dir::ALL
                .into_iter()
                .filter(|d| room(jump(here, *d)) && !open[idx(jump(here, *d))])
                .collect();
            if next.is_empty() {
                stack.pop();
                continue;
            }
            rng.shuffle(&mut next);
            let d = next[0];
            open[idx(here.step(d))] = true;
            open[idx(jump(here, d))] = true;
            stack.push(jump(here, d));
        }
        // Braid: give each dead-end room another way out.
        for y in (0..h).step_by(2) {
            for x in (0..w).step_by(2) {
                let c = Cell::new(x, y);
                let ways = Dir::ALL
                    .into_iter()
                    .filter(|d| room(c.step(*d)) && open[idx(c.step(*d))])
                    .count();
                if ways > 1 {
                    continue;
                }
                let mut walls: Vec<Dir> = Dir::ALL
                    .into_iter()
                    .filter(|d| room(jump(c, *d)) && !open[idx(c.step(*d))])
                    .collect();
                if walls.is_empty() {
                    continue;
                }
                rng.shuffle(&mut walls);
                open[idx(c.step(walls[0]))] = true;
            }
        }
        Maze {
            w,
            h,
            open,
            mouse: Cell::new(w / 2 - (w / 2) % 2, h / 2 - (h / 2) % 2),
            dens: vec![
                Cell::new(0, 0),
                Cell::new(w - 1, h - 1),
                Cell::new(w - 1, 0),
            ],
        }
    }

    pub fn inside(&self, c: Cell) -> bool {
        c.x >= 0 && c.y >= 0 && c.x < self.w && c.y < self.h
    }

    pub fn is_open(&self, c: Cell) -> bool {
        self.inside(c) && self.open[self.index(c)]
    }

    pub fn index(&self, c: Cell) -> usize {
        (c.y * self.w + c.x) as usize
    }

    /// Every open cell, row by row from the top.
    pub fn cells(&self) -> Vec<Cell> {
        (0..self.h)
            .flat_map(|y| (0..self.w).map(move |x| Cell::new(x, y)))
            .filter(|c| self.is_open(*c))
            .collect()
    }

    /// The ways out of `c`, in [`Dir::ALL`] order.
    pub fn open_dirs(&self, c: Cell) -> Vec<Dir> {
        Dir::ALL
            .into_iter()
            .filter(|d| self.is_open(c.step(*d)))
            .collect()
    }

    /// Steps from `from` to every cell, `None` for walls and cells out of
    /// reach.
    pub fn distances(&self, from: Cell) -> Vec<Option<u32>> {
        let mut dist = vec![None; (self.w * self.h) as usize];
        if !self.is_open(from) {
            return dist;
        }
        dist[self.index(from)] = Some(0);
        let mut queue = VecDeque::from([from]);
        while let Some(c) = queue.pop_front() {
            let d = dist[self.index(c)].unwrap_or(0);
            for dir in Dir::ALL {
                let n = c.step(dir);
                if self.is_open(n) && dist[self.index(n)].is_none() {
                    dist[self.index(n)] = Some(d + 1);
                    queue.push_back(n);
                }
            }
        }
        dist
    }

    /// Steps from `a` to `b`, if `b` can be reached.
    pub fn distance(&self, a: Cell, b: Cell) -> Option<u32> {
        if !self.is_open(b) {
            return None;
        }
        self.distances(a)[self.index(b)]
    }

    /// The first step of a shortest way from `from` to `to`, ties broken in
    /// [`Dir::ALL`] order. `None` when they are the same cell or apart.
    pub fn toward(&self, from: Cell, to: Cell) -> Option<Dir> {
        if from == to {
            return None;
        }
        let dist = self.distances(to);
        let here = dist[self.index(from)]?;
        Dir::ALL.into_iter().find(|d| {
            let n = from.step(*d);
            self.is_open(n) && dist[self.index(n)] == Some(here.saturating_sub(1))
        })
    }
}

/// How a cat chooses its way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Heads straight for the mouse, by the shortest way.
    Chaser,
    /// Wanders at random until within [`WANDER_CHASE`] steps, then chases.
    Wanderer,
}

/// The cats a run can hold, in the order they join.
pub const CATS: [(Kind, &str); 3] = [
    (Kind::Chaser, "ginger cat"),
    (Kind::Wanderer, "tabby cat"),
    (Kind::Wanderer, "grey cat"),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Cat {
    pub kind: Kind,
    /// The cat's name, lower case: "ginger cat".
    pub name: &'static str,
    pub pos: Cell,
    pub den: Cell,
    /// The way it last moved, so a wanderer does not turn back.
    pub heading: Option<Dir>,
    /// Substeps it stays lost after a freeze: it wanders and cannot chase.
    pub lost: u32,
}

/// The way a cat moves next, or `None` when it is boxed in. A chasing cat
/// takes the shortest way to the mouse, ties broken in [`Dir::ALL`] order.
/// A wandering cat picks at random among the ways out, never turning back
/// unless it must.
pub fn next_dir(
    maze: &Maze,
    kind: Kind,
    pos: Cell,
    heading: Option<Dir>,
    mouse: Cell,
    lost: bool,
    rng: &mut Rng,
) -> Option<Dir> {
    let ways = maze.open_dirs(pos);
    if ways.is_empty() {
        return None;
    }
    let to_mouse = maze.distances(mouse);
    let near = to_mouse[maze.index(pos)].is_some_and(|d| d <= WANDER_CHASE);
    let chasing = !lost && (kind == Kind::Chaser || near);
    if chasing {
        return ways
            .iter()
            .copied()
            .min_by_key(|d| to_mouse[maze.index(pos.step(*d))].unwrap_or(u32::MAX));
    }
    let forward: Vec<Dir> = ways
        .iter()
        .copied()
        .filter(|d| Some(d.reverse()) != heading)
        .collect();
    let pick = if forward.is_empty() { &ways } else { &forward };
    Some(*rng.pick(pick))
}

/// How fast the cats move, pantry by pantry, in milliseconds a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pace {
    pub start: u32,
    pub step: u32,
    pub floor: u32,
}

impl Pace {
    /// The pantry run.
    pub const PANTRY: Pace = Pace {
        start: 700,
        step: 60,
        floor: 340,
    };
    /// The practice pantry: one slow cat.
    pub const QUIET: Pace = Pace {
        start: 1200,
        step: 0,
        floor: 1200,
    };

    /// Milliseconds a cat step in pantry `n`, from 1.
    pub fn at(self, n: u32) -> u32 {
        self.start
            .saturating_sub(self.step * n.saturating_sub(1))
            .max(self.floor)
    }
}

/// A treat an event pantry hides among the crumbs, worth more than a
/// crumb. The crumb in its cell is eaten too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Treat {
    /// Thanksgiving's leftovers.
    Turkey,
    /// Bin night's cheese rind.
    CheeseRind,
}

impl Treat {
    /// The treat's name, lower case: "turkey scrap".
    pub fn name(self) -> &'static str {
        match self {
            Treat::Turkey => "turkey scrap",
            Treat::CheeseRind => "cheese rind",
        }
    }

    /// The name with a count: "1 turkey scrap", "3 turkey scraps".
    pub fn count_words(self, n: u32) -> String {
        match (self, n) {
            (t, 1) => format!("1 {}", t.name()),
            (Treat::Turkey, n) => format!("{n} turkey scraps"),
            (Treat::CheeseRind, n) => format!("{n} cheese rinds"),
        }
    }

    /// Points for eating one, on top of its cell's crumb.
    pub fn points(self) -> u32 {
        match self {
            Treat::Turkey => 50,
            Treat::CheeseRind => 100,
        }
    }
}

/// Treats are never placed nearer the mouse's start than this, in steps.
pub const TREAT_SPACE: u32 = 3;

/// What a run is.
#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    /// How many cats, from [`CATS`].
    pub cats: usize,
    /// How many cats the first pantry holds, so pantry 1 is a gentler
    /// start; from the second pantry on, all of [`Rules::cats`].
    pub first_cats: usize,
    pub pace: Pace,
    /// A catch costs a whisker. Off in the practice pantry.
    pub whiskers: bool,
    /// Hint works.
    pub hints: bool,
}

impl Rules {
    pub const PANTRY: Rules = Rules {
        cats: 2,
        first_cats: 1,
        pace: Pace::PANTRY,
        whiskers: true,
        hints: true,
    };
    pub const QUIET: Rules = Rules {
        cats: 1,
        first_cats: 1,
        pace: Pace::QUIET,
        whiskers: false,
        hints: true,
    };
}

impl Rules {
    /// How many cats pantry `n`, from 1, holds.
    pub fn cats_in(&self, n: u32) -> usize {
        if n <= 1 {
            self.first_cats
        } else {
            self.cats
        }
    }
}

/// Where the pantries come from.
#[derive(Clone, Debug, PartialEq)]
pub enum Course {
    /// The same maze again and again, the cats quicker each time.
    Endless(Maze),
    /// These mazes in order; clearing the last ends the run.
    Mazes(Vec<Maze>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The last whisker went.
    OutOfWhiskers,
    /// Every pantry of the course cleared.
    Cleared,
}

/// Something that happened, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// The mouse moved one cell, and ate a crumb there or not.
    Step { crumb: bool },
    /// A wall: the step was refused. `open` lists the ways out.
    Bump { open: Vec<Dir> },
    /// The mouse is frozen and cannot step.
    StillFrozen { ms: u32 },
    /// The run is between pantries or over: nothing moves.
    Waiting,
    /// A cat moved.
    CatMoved { cat: usize },
    /// A cat came within [`CLOSE`] steps.
    CatClose { cat: usize },
    /// A cat came within hearing, or left it.
    CatHeard { cat: usize, heard: bool },
    /// A cat caught the mouse. `whiskers` is what is left; `lost` is false
    /// in the practice pantry, where a catch costs nothing.
    Caught {
        cat: usize,
        whiskers: u32,
        lost: bool,
    },
    /// The cats are moving again after a catch.
    Prowling,
    /// The mouse froze and `cat` lost track of it.
    Frozen { cat: Option<usize>, left: u32 },
    /// A freeze while frozen.
    AlreadyFrozen { ms: u32 },
    /// No freezes left in this pantry.
    NoFreezes,
    /// The freeze ended.
    Thawed,
    /// The mouse ate a treat, worth `points`; `left` are still hidden in
    /// this pantry.
    Treat {
        treat: Treat,
        points: u32,
        left: u32,
    },
    /// The pantry is clear.
    Clear { pantry: u32, bonus: u32 },
    /// The next pantry has begun.
    NextPantry { pantry: u32 },
    /// The run is over.
    Over(Outcome),
}

/// A run of pantries: the mouse, the crumbs, the cats and the clock.
#[derive(Clone, Debug)]
pub struct Run {
    pub course: Course,
    pub rules: Rules,
    pub maze: Maze,
    /// The pantry being played, from 1.
    pub pantry: u32,
    crumbs: Vec<bool>,
    /// Crumbs left in this pantry.
    pub left: u32,
    pub mouse: Cell,
    pub cats: Vec<Cat>,
    pub whiskers: u32,
    pub score: u32,
    /// Crumbs eaten in the whole run.
    pub eaten: u32,
    /// Freezes left in this pantry.
    pub freezes: u32,
    /// Pantries cleared in the run.
    pub cleared: u32,
    pub outcome: Option<Outcome>,
    started: bool,
    frozen: u32,
    grace: u32,
    cat_timer: u32,
    next_pantry: u32,
    close: Vec<bool>,
    heard: Vec<bool>,
    clock: FixedStep,
    rng: Rng,
    /// The treat this run hides, and how many in each pantry.
    pub treat: Option<(Treat, u32)>,
    /// The cells still holding a treat in this pantry.
    pub treats: Vec<Cell>,
    /// Treats eaten in the whole run.
    pub treats_eaten: u32,
    /// Places the treats, apart from the cats' own dice.
    treat_rng: Rng,
}

fn steps(ms: u32) -> u32 {
    ms.div_ceil(STEP_MS)
}

impl Run {
    pub fn new(course: Course, rules: Rules, seed: u64) -> Run {
        let maze = match &course {
            Course::Endless(m) => m.clone(),
            Course::Mazes(ms) => ms.first().cloned().expect("at least one maze"),
        };
        let mut run = Run {
            course,
            rules,
            maze: maze.clone(),
            pantry: 1,
            crumbs: Vec::new(),
            left: 0,
            mouse: maze.mouse,
            cats: Vec::new(),
            whiskers: WHISKERS,
            score: 0,
            eaten: 0,
            freezes: FREEZES,
            cleared: 0,
            outcome: None,
            started: false,
            frozen: 0,
            grace: 0,
            cat_timer: 0,
            next_pantry: 0,
            close: Vec::new(),
            heard: Vec::new(),
            clock: FixedStep::hz(STEP_HZ),
            rng: Rng::seeded(seed),
            treat: None,
            treats: Vec::new(),
            treats_eaten: 0,
            treat_rng: Rng::seeded(seed ^ 0x7472_6561_7473),
        };
        run.set_up(maze);
        run
    }

    /// The endless pantry run on the fixed maze.
    pub fn pantry(seed: u64) -> Run {
        Run::new(Course::Endless(Maze::pantry()), Rules::PANTRY, seed)
    }

    /// The practice pantry: one slow cat, no whiskers lost.
    pub fn quiet(seed: u64) -> Run {
        Run::new(Course::Endless(Maze::pantry()), Rules::QUIET, seed)
    }

    /// The same run with `n` of `treat` hidden in every pantry.
    pub fn with_treats(mut self, treat: Treat, n: u32) -> Run {
        self.treat = Some((treat, n));
        self.place_treats();
        self
    }

    /// Hides this pantry's treats on seeded cells: never the mouse's start,
    /// a den, or within [`TREAT_SPACE`] steps of the start.
    fn place_treats(&mut self) {
        self.treats.clear();
        let Some((_, n)) = self.treat else {
            return;
        };
        let dist = self.maze.distances(self.maze.mouse);
        let mut spots: Vec<Cell> = self
            .maze
            .cells()
            .into_iter()
            .filter(|c| !self.maze.dens.contains(c))
            .filter(|c| dist[self.maze.index(*c)].is_some_and(|d| d > TREAT_SPACE))
            .collect();
        self.treat_rng.shuffle(&mut spots);
        spots.truncate(n as usize);
        spots.sort();
        self.treats = spots;
    }

    /// Treats still hidden in this pantry.
    pub fn treats_left(&self) -> u32 {
        self.treats.len() as u32
    }

    /// The nearest treat by steps: its cell and the steps to it.
    pub fn nearest_treat(&self) -> Option<(Cell, u32)> {
        let dist = self.maze.distances(self.mouse);
        self.treats
            .iter()
            .filter_map(|c| dist[self.maze.index(*c)].map(|d| (d, c.y, c.x, *c)))
            .min()
            .map(|(d, _, _, c)| (c, d))
    }

    fn set_up(&mut self, maze: Maze) {
        self.crumbs =
            maze.cells()
                .iter()
                .fold(vec![false; (maze.w * maze.h) as usize], |mut v, c| {
                    v[maze.index(*c)] = *c != maze.mouse;
                    v
                });
        self.left = self.crumbs.iter().filter(|c| **c).count() as u32;
        let n = self
            .rules
            .cats_in(self.pantry)
            .min(maze.dens.len())
            .min(CATS.len());
        self.cats = (0..n)
            .map(|i| Cat {
                kind: CATS[i].0,
                name: CATS[i].1,
                pos: maze.dens[i],
                den: maze.dens[i],
                heading: None,
                lost: 0,
            })
            .collect();
        self.maze = maze;
        self.freezes = FREEZES;
        self.reset_places();
    }

    /// Everyone back to the start, the cats waiting.
    fn reset_places(&mut self) {
        self.mouse = self.maze.mouse;
        for cat in &mut self.cats {
            cat.pos = cat.den;
            cat.heading = None;
            cat.lost = 0;
        }
        self.frozen = 0;
        self.cat_timer = 0;
        self.close = vec![false; self.cats.len()];
        self.heard = vec![false; self.cats.len()];
    }

    /// The cats have woken: the mouse has taken its first step.
    pub fn started(&self) -> bool {
        self.started
    }

    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }

    /// Between a cleared pantry and the next.
    pub fn between(&self) -> bool {
        self.next_pantry > 0
    }

    /// Milliseconds of freeze left.
    pub fn frozen_ms(&self) -> u32 {
        self.frozen * STEP_MS
    }

    /// The cats are waiting after a catch.
    pub fn in_grace(&self) -> bool {
        self.grace > 0
    }

    /// Milliseconds a cat step in this pantry.
    pub fn pace_ms(&self) -> u32 {
        self.rules.pace.at(self.pantry)
    }

    /// How many pantries the course holds, `None` when endless.
    pub fn pantries(&self) -> Option<u32> {
        match &self.course {
            Course::Endless(_) => None,
            Course::Mazes(ms) => Some(ms.len() as u32),
        }
    }

    pub fn has_crumb(&self, c: Cell) -> bool {
        self.maze.inside(c) && self.crumbs[self.maze.index(c)]
    }

    /// Steps from the mouse to cat `i`.
    pub fn cat_distance(&self, i: usize) -> Option<u32> {
        self.maze.distance(self.mouse, self.cats[i].pos)
    }

    /// Where cat `i` is from the mouse, in cells: right and down are
    /// positive.
    pub fn cat_offset(&self, i: usize) -> (i32, i32) {
        let c = self.cats[i].pos;
        (c.x - self.mouse.x, c.y - self.mouse.y)
    }

    /// The way from the mouse towards cat `i`.
    pub fn cat_way(&self, i: usize) -> Option<Dir> {
        self.maze.toward(self.mouse, self.cats[i].pos)
    }

    /// Whether cat `i` is within hearing.
    pub fn cat_heard(&self, i: usize) -> bool {
        self.heard.get(i).copied().unwrap_or(false)
    }

    /// The nearest crumbs by steps, nearest first, up to `n`; ties in row
    /// order from the top.
    pub fn nearest_crumbs(&self, n: usize) -> Vec<(Cell, u32)> {
        let dist = self.maze.distances(self.mouse);
        let mut found: Vec<(u32, Cell)> = self
            .maze
            .cells()
            .into_iter()
            .filter(|c| self.has_crumb(*c))
            .filter_map(|c| dist[self.maze.index(c)].map(|d| (d, c)))
            .collect();
        found.sort_by_key(|(d, c)| (*d, c.y, c.x));
        found.into_iter().take(n).map(|(d, c)| (c, d)).collect()
    }

    /// The nearest crumb that no cat is within [`CLOSE`] steps of: the way
    /// to it and how many steps. `None` when there is no such crumb.
    pub fn hint(&self) -> Option<(Dir, u32)> {
        let from_cats: Vec<Vec<Option<u32>>> = self
            .cats
            .iter()
            .map(|c| self.maze.distances(c.pos))
            .collect();
        let dist = self.maze.distances(self.mouse);
        let safe = |c: Cell| {
            from_cats
                .iter()
                .all(|d| d[self.maze.index(c)].is_none_or(|d| d > CLOSE))
        };
        let target = self
            .maze
            .cells()
            .into_iter()
            .filter(|c| self.has_crumb(*c) && safe(*c))
            .filter_map(|c| dist[self.maze.index(c)].map(|d| (d, c.y, c.x, c)))
            .min()?;
        let way = self.maze.toward(self.mouse, target.3)?;
        Some((way, target.0))
    }

    /// The mouse tries a step. The first step wakes the cats.
    pub fn step(&mut self, dir: Dir) -> Vec<Ev> {
        if self.finished() || self.between() {
            return vec![Ev::Waiting];
        }
        if self.frozen > 0 {
            return vec![Ev::StillFrozen {
                ms: self.frozen_ms(),
            }];
        }
        let to = self.mouse.step(dir);
        if !self.maze.is_open(to) {
            return vec![Ev::Bump {
                open: self.maze.open_dirs(self.mouse),
            }];
        }
        self.started = true;
        self.mouse = to;
        let i = self.maze.index(to);
        let crumb = self.crumbs[i];
        let mut evs = Vec::new();
        if crumb {
            self.crumbs[i] = false;
            self.left -= 1;
            self.eaten += 1;
            self.score += CRUMB_POINTS;
        }
        evs.push(Ev::Step { crumb });
        if let (Some((treat, _)), Some(t)) = (self.treat, self.treats.iter().position(|c| *c == to))
        {
            self.treats.remove(t);
            self.treats_eaten += 1;
            self.score += treat.points();
            evs.push(Ev::Treat {
                treat,
                points: treat.points(),
                left: self.treats_left(),
            });
        }
        if let Some(cat) = self.cats.iter().position(|c| c.pos == to) {
            self.caught(cat, &mut evs);
            return evs;
        }
        if self.left == 0 {
            self.clear(&mut evs);
            return evs;
        }
        self.sense(&mut evs);
        evs
    }

    /// The mouse freezes: it cannot step for [`FREEZE_MS`], and the
    /// nearest cat loses track of it until a second after.
    pub fn freeze(&mut self) -> Ev {
        if self.finished() || self.between() {
            return Ev::Waiting;
        }
        if self.frozen > 0 {
            return Ev::AlreadyFrozen {
                ms: self.frozen_ms(),
            };
        }
        if self.freezes == 0 {
            return Ev::NoFreezes;
        }
        self.freezes -= 1;
        self.frozen = steps(FREEZE_MS);
        // Before the first step the cats sleep: there is none to lose.
        let nearest = (0..self.cats.len())
            .filter(|_| self.started)
            .filter_map(|i| self.cat_distance(i).map(|d| (d, i)))
            .min()
            .map(|(_, i)| i);
        if let Some(i) = nearest {
            self.cats[i].lost = steps(FREEZE_MS + LOST_AFTER_MS);
            // A lost cat wanders on without turning back, so face it away
            // from the mouse: otherwise one coming down a corridor walks
            // straight on into the mouse it has just lost.
            let to_mouse = self.maze.distances(self.mouse);
            let pos = self.cats[i].pos;
            if let Some(toward) = self
                .maze
                .open_dirs(pos)
                .into_iter()
                .min_by_key(|d| to_mouse[self.maze.index(pos.step(*d))].unwrap_or(u32::MAX))
            {
                self.cats[i].heading = Some(toward.reverse());
            }
        }
        Ev::Frozen {
            cat: nearest,
            left: self.freezes,
        }
    }

    /// Runs the clock for one frame of `dt` seconds.
    pub fn update(&mut self, dt: f64) -> Vec<Ev> {
        let n = self.clock.advance(dt, |_| {});
        let mut evs = Vec::new();
        for _ in 0..n {
            self.tick(&mut evs);
        }
        evs
    }

    fn tick(&mut self, evs: &mut Vec<Ev>) {
        if self.finished() {
            return;
        }
        if self.next_pantry > 0 {
            self.next_pantry -= 1;
            if self.next_pantry == 0 {
                self.begin_next(evs);
            }
            return;
        }
        // A freeze thaws even before the first step, or the mouse could
        // never take one.
        if self.frozen > 0 {
            self.frozen -= 1;
            if self.frozen == 0 {
                evs.push(Ev::Thawed);
            }
        }
        if !self.started {
            return;
        }
        for cat in &mut self.cats {
            cat.lost = cat.lost.saturating_sub(1);
        }
        if self.grace > 0 {
            self.grace -= 1;
            if self.grace == 0 {
                evs.push(Ev::Prowling);
            }
            return;
        }
        self.cat_timer += 1;
        if self.cat_timer < steps(self.pace_ms()) {
            return;
        }
        self.cat_timer = 0;
        for i in 0..self.cats.len() {
            let cat = &self.cats[i];
            let dir = next_dir(
                &self.maze,
                cat.kind,
                cat.pos,
                cat.heading,
                self.mouse,
                cat.lost > 0,
                &mut self.rng,
            );
            if let Some(dir) = dir {
                let cat = &mut self.cats[i];
                cat.pos = cat.pos.step(dir);
                cat.heading = Some(dir);
                evs.push(Ev::CatMoved { cat: i });
            }
            if self.cats[i].pos == self.mouse {
                self.caught(i, evs);
                return;
            }
        }
        self.sense(evs);
    }

    /// Notes cats coming close or into hearing.
    fn sense(&mut self, evs: &mut Vec<Ev>) {
        let dist = self.maze.distances(self.mouse);
        for i in 0..self.cats.len() {
            let d = dist[self.maze.index(self.cats[i].pos)];
            let close = d.is_some_and(|d| d <= CLOSE);
            if close && !self.close[i] {
                evs.push(Ev::CatClose { cat: i });
            }
            self.close[i] = close;
            let heard = d.is_some_and(|d| d <= HEAR_RANGE);
            if heard != self.heard[i] {
                evs.push(Ev::CatHeard { cat: i, heard });
            }
            self.heard[i] = heard;
        }
    }

    fn caught(&mut self, cat: usize, evs: &mut Vec<Ev>) {
        let lost = self.rules.whiskers;
        if lost {
            self.whiskers = self.whiskers.saturating_sub(1);
        }
        evs.push(Ev::Caught {
            cat,
            whiskers: self.whiskers,
            lost,
        });
        for (i, heard) in self.heard.iter().enumerate() {
            if *heard {
                evs.push(Ev::CatHeard {
                    cat: i,
                    heard: false,
                });
            }
        }
        self.reset_places();
        self.grace = steps(GRACE_MS);
        if self.whiskers == 0 {
            self.outcome = Some(Outcome::OutOfWhiskers);
            evs.push(Ev::Over(Outcome::OutOfWhiskers));
        }
    }

    fn clear(&mut self, evs: &mut Vec<Ev>) {
        self.score += CLEAR_BONUS;
        self.cleared += 1;
        evs.push(Ev::Clear {
            pantry: self.pantry,
            bonus: CLEAR_BONUS,
        });
        for (i, heard) in self.heard.iter().enumerate() {
            if *heard {
                evs.push(Ev::CatHeard {
                    cat: i,
                    heard: false,
                });
            }
        }
        let more = match &self.course {
            Course::Endless(_) => true,
            Course::Mazes(ms) => (self.pantry as usize) < ms.len(),
        };
        if more {
            self.next_pantry = steps(NEXT_PANTRY_MS);
        } else {
            self.outcome = Some(Outcome::Cleared);
            evs.push(Ev::Over(Outcome::Cleared));
        }
    }

    fn begin_next(&mut self, evs: &mut Vec<Ev>) {
        self.pantry += 1;
        let maze = match &self.course {
            Course::Endless(m) => m.clone(),
            Course::Mazes(ms) => ms[(self.pantry - 1) as usize].clone(),
        };
        self.set_up(maze);
        self.place_treats();
        self.started = false;
        self.grace = 0;
        evs.push(Ev::NextPantry {
            pantry: self.pantry,
        });
    }

    /// A short fingerprint of the run's state, for frame-rate tests.
    pub fn snapshot(&self) -> (Cell, Vec<Cell>, u32, u32, u32, u64) {
        (
            self.mouse,
            self.cats.iter().map(|c| c.pos).collect(),
            self.whiskers,
            self.score,
            self.left,
            self.clock.steps(),
        )
    }
}

/// "1 crumb", "12 crumbs".
pub fn crumbs_words(n: u32) -> String {
    match n {
        1 => "1 crumb".to_string(),
        n => format!("{n} crumbs"),
    }
}

/// "1 whisker", "2 whiskers".
pub fn whiskers_words(n: u32) -> String {
    match n {
        1 => "1 whisker".to_string(),
        n => format!("{n} whiskers"),
    }
}

/// "1 step", "4 steps".
pub fn steps_words(n: u32) -> String {
    match n {
        1 => "1 step".to_string(),
        n => format!("{n} steps"),
    }
}

/// "1 freeze", "3 freezes".
pub fn freezes_words(n: u32) -> String {
    match n {
        1 => "1 freeze".to_string(),
        n => format!("{n} freezes"),
    }
}

/// "1 point", "560 points".
pub fn points_words(n: u32) -> String {
    match n {
        1 => "1 point".to_string(),
        n => format!("{n} points"),
    }
}

/// Where something is from the mouse, in cells: "3 cells left and 1 up",
/// "2 cells right", "on your cell".
pub fn offset_words(dx: i32, dy: i32) -> String {
    let cells = |n: i32| {
        if n == 1 {
            "1 cell".to_string()
        } else {
            format!("{n} cells")
        }
    };
    let h = if dx < 0 { "left" } else { "right" };
    let v = if dy < 0 { "up" } else { "down" };
    match (dx.abs(), dy.abs()) {
        (0, 0) => "on your cell".to_string(),
        (a, 0) => format!("{} {h}", cells(a)),
        (0, b) => format!("{} {v}", cells(b)),
        (a, b) => format!("{} {h} and {b} {v}", cells(a)),
    }
}

/// The ways out, spoken: "up, left and right".
pub fn ways_words(dirs: &[Dir]) -> String {
    let words: Vec<&str> = dirs.iter().map(|d| d.word()).collect();
    match words.as_slice() {
        [] => "none".to_string(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Seconds, in words: "3 seconds", "1.5 seconds", "1 second".
pub fn seconds_words(ms: u32) -> String {
    let tenths = ms.div_ceil(100);
    let (whole, part) = (tenths / 10, tenths % 10);
    match (whole, part) {
        (1, 0) => "1 second".to_string(),
        (w, 0) => format!("{w} seconds"),
        (w, p) => format!("{w}.{p} seconds"),
    }
}

thread_local! {
    static FIXED_SEED: StdCell<Option<u64>> = const { StdCell::new(None) };
}

/// Pins the seed of new runs, for tests. `None` goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new run: the pinned one, or one from the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}
