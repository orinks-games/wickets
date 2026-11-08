//! The rules of a skate, as plain structs with no sound or speech.
//!
//! You skate across the ice from bank to bank: your place is `x`, from
//! -100 at the left bank to 100 at the right, 0 in the middle. Left and
//! Right lean you across. Lanterns come towards you in five lanes, and one
//! taken as it passes adds to your line: the longer the line, the more it
//! is worth. Drifts hush and thin ice creaks ahead. Your partner skates the
//! easy path a little ahead. Hold the main key for a spin, and let go once
//! it is ready.
//!
//! Snow falls all night and settles on the ice. Each lantern you take
//! keeps a stretch of ice clear a while longer. Drifts lie on the ice:
//! skate into one and you slow down and lose your line. On the later
//! ponds, thin ice creaks before it cracks, and a crack lets the snow in.
//! A skate on a pond ends when the snow has covered the ice, never on a
//! clock. The Frost Fair, the lantern skate and practice are courses with
//! an end instead.
//!
//! Time is in milliseconds from the push-off. Everything comes from a
//! seeded [`Rng`], so a test can skate the same pond twice.

use parlor_core::Rng;

/// The banks: `x` runs from minus this to this.
pub const BANK: f64 = 100.0;
/// The width of a lane.
pub const LANE: f64 = 30.0;
/// How far a lean carries you each second.
pub const LEAN_RATE: f64 = 80.0;
/// A lantern within this of you as it passes is taken.
pub const CATCH: f64 = 16.0;
/// The most a lantern in a line is worth.
pub const LINE_TOP: u32 = 5;
/// How clear the ice is at the push-off.
pub const CLEAR_FULL: f64 = 100.0;
/// How much clear ice a crack costs: water comes up, and snow settles in it.
pub const CRACK_SNOW: f64 = 12.0;
/// How long a crack leaves you finding your feet, with no lean.
pub const STUMBLE_MS: i64 = 1200;
/// How long a drift slows you.
pub const DRIFT_MS: i64 = 1000;
/// How much a drift slows a lean.
pub const DRIFT_LEAN: f64 = 0.4;
/// How long the main key is held before a spin is ready.
pub const SPIN_READY_MS: i64 = 1000;
/// How long a ready spin can be held before it wobbles.
pub const SPIN_HOLD_MS: i64 = 1400;
/// A drift or thin ice is heard this long before it.
pub const PATCH_LEAD_MS: i64 = 2400;
/// In a lantern skate, the second lantern of a pair lights this long
/// after the first.
pub const PAIR_GAP_MS: i64 = 700;
/// After the last lantern of a course, the skate runs on this long.
pub const RUN_OUT_MS: i64 = 2500;
/// Your partner reaches each lantern's lane this long before it passes,
/// and stays in it until just after.
pub const PARTNER_EARLY_MS: i64 = 1500;
const PARTNER_AFTER_MS: i64 = 150;
/// How long a detour round thin ice is held.
const DETOUR_BEFORE_MS: i64 = 500;
const DETOUR_AFTER_MS: i64 = 250;
/// How far ahead of now a pond with no end is planned.
const HORIZON_MS: i64 = 20_000;

/// Where a lantern stands, as you hear it: one of five lanes.
pub fn lane_x(lane: i32) -> f64 {
    lane as f64 * LANE
}

/// "far left", "left", "the middle", "right", "far right".
pub fn lane_words(lane: i32) -> &'static str {
    match lane {
        i32::MIN..=-2 => "far left",
        -1 => "left",
        0 => "the middle",
        1 => "right",
        _ => "far right",
    }
}

/// The lane nearest `x`.
pub fn nearest_lane(x: f64) -> i32 {
    (x / LANE).round().clamp(-2.0, 2.0) as i32
}

/// What a skate is like, apart from where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    /// Milliseconds between lanterns, at least.
    pub gap_ms: i64,
    /// How long before a lantern it lights and is heard.
    pub lead_ms: i64,
    /// Snow settled each second, out of [`CLEAR_FULL`]. 0: the snow never
    /// covers the ice.
    pub snow: f64,
    /// Clear ice each lantern taken keeps.
    pub gain: f64,
    /// The chance of a drift between two lanterns.
    pub drifts: f64,
    /// The chance of thin ice between two lanterns, before a drift.
    pub ice: f64,
    /// Half the width of a drift or a thin patch.
    pub patch_width: f64,
    /// How many lanes the next lantern may be from the last.
    pub spread: i32,
    /// Whether your partner skates with you.
    pub partner: bool,
    /// Lanterns on the course, for a skate with an end. `None` runs until
    /// the snow covers the ice.
    pub count: Option<u32>,
    /// Lanterns in pairs, one lit after the other: take the first lit.
    pub pairs: bool,
    /// Stalls along the bank: the Frost Fair.
    pub stalls: bool,
}

/// One lantern on the ice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lantern {
    /// When it passes you.
    pub at: i64,
    /// When it lights and is first heard.
    pub lit: i64,
    pub lane: i32,
    /// Its note, 0 to 7, up the scale.
    pub note: usize,
    /// In a lantern skate, the one lit second: the wrong one to take.
    pub decoy: bool,
}

impl Lantern {
    pub fn x(&self) -> f64 {
        lane_x(self.lane)
    }
}

/// What lies on the ice between lanterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PatchKind {
    /// A heap of snow: it slows you and breaks your line.
    Drift,
    /// Thin ice: it cracks, and the snow gets in.
    Thin,
}

impl PatchKind {
    /// "Drift", "Thin ice".
    pub fn words(self) -> &'static str {
        match self {
            PatchKind::Drift => "Drift",
            PatchKind::Thin => "Thin ice",
        }
    }
}

/// A drift or a patch of thin ice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Patch {
    pub kind: PatchKind,
    /// When it passes under you.
    pub at: i64,
    pub x: f64,
    pub width: f64,
}

impl Patch {
    pub fn heard_at(&self) -> i64 {
        self.at - PATCH_LEAD_MS
    }

    /// Whether a skater at `x` is on it.
    pub fn under(&self, x: f64) -> bool {
        (x - self.x).abs() < self.width
    }
}

/// A stall on the bank at the Frost Fair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StallKind {
    Chestnuts,
    Cider,
    Band,
    Gingerbread,
}

impl StallKind {
    pub const ALL: [StallKind; 4] = [
        StallKind::Chestnuts,
        StallKind::Cider,
        StallKind::Band,
        StallKind::Gingerbread,
    ];

    /// "The chestnut brazier."
    pub fn words(self) -> &'static str {
        match self {
            StallKind::Chestnuts => "The chestnut brazier",
            StallKind::Cider => "The cider urn",
            StallKind::Band => "The brass band",
            StallKind::Gingerbread => "The gingerbread stall",
        }
    }
}

/// A stall you skate past, on one bank.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stall {
    pub at: i64,
    pub kind: StallKind,
    /// -1 the left bank, 1 the right.
    pub side: i32,
}

/// A place your partner holds until a time: a lantern's lane, or the way
/// round thin ice.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hold {
    from: i64,
    to: i64,
    x: f64,
}

/// What lies ahead on the ice, planned as the skate goes.
#[derive(Clone, Debug)]
pub struct Course {
    pub level: Level,
    /// The seed the course was planned from.
    pub seed: u64,
    rng: Rng,
    pub lanterns: Vec<Lantern>,
    pub patches: Vec<Patch>,
    pub stalls: Vec<Stall>,
    holds: Vec<Hold>,
    /// When the last lantern planned passes.
    planned_to: i64,
    last_lane: i32,
    /// The tune a lantern skate's first-lit lanterns play, a note each.
    pub tune: Vec<usize>,
    /// Lanterns planned, counting a pair as one.
    made: u32,
    /// When a course with an end is over.
    pub end: Option<i64>,
}

impl Course {
    /// A course at `level`, from `seed`. A lantern skate plays `tune`.
    pub fn new(level: Level, seed: u64, tune: Vec<usize>) -> Course {
        let mut c = Course {
            level,
            seed,
            rng: Rng::seeded(seed),
            lanterns: Vec::new(),
            patches: Vec::new(),
            stalls: Vec::new(),
            holds: vec![Hold {
                from: 0,
                to: 0,
                x: 0.0,
            }],
            // The first lantern passes once its lead has run, after a beat.
            planned_to: 1200 + level.lead_ms - level.gap_ms,
            last_lane: 0,
            tune,
            made: 0,
            end: None,
        };
        c.extend(HORIZON_MS);
        c
    }

    /// Plans the course on to `until`, or to its end.
    pub fn extend(&mut self, until: i64) {
        while self.end.is_none() && self.planned_to < until {
            if let Some(n) = self.level.count {
                if self.made >= n {
                    self.end = Some(self.planned_to + RUN_OUT_MS);
                    break;
                }
            }
            self.plan_next();
        }
    }

    fn plan_next(&mut self) {
        let level = self.level;
        let from = self.last_lane;
        let lane = if level.pairs {
            0
        } else {
            let step = self.rng.range(-level.spread, level.spread);
            let mut lane = (from + step).clamp(-2, 2);
            // Never the same lane three times running in a lane: a lean
            // each time keeps the skate moving.
            if lane == from && self.made > 0 && self.rng.chance(0.6) {
                lane = if from >= 1 { from - 1 } else { from + 1 };
            }
            lane
        };
        let prev_at = self.planned_to;
        let prev_x = lane_x(from);
        let x = lane_x(lane);
        // Thin ice sits on the straight line from the last lantern to this
        // one, so the straight way crosses it; the partner goes round.
        let patch = if self.made == 0 || level.pairs {
            None
        } else if level.ice > 0.0 && self.rng.chance(level.ice) {
            Some(PatchKind::Thin)
        } else if level.drifts > 0.0 && self.rng.chance(level.drifts) {
            Some(PatchKind::Drift)
        } else {
            None
        };
        let move_ms = |dx: f64| (dx.abs() / LEAN_RATE * 1000.0) as i64 + 250;
        let mut gap = level.gap_ms + self.rng.range(0, 400) as i64;
        let detour = if let Some(kind) = patch {
            let mid = (prev_x + x) / 2.0 + self.rng.range(-8, 8) as f64;
            let w = level.patch_width;
            // Round the side nearer the middle, unless that is a bank.
            let left = mid - w - 12.0;
            let right = mid + w + 12.0;
            let way = if mid > 0.0 { left } else { right };
            let way = if way.abs() > BANK - 5.0 {
                if way < 0.0 {
                    right
                } else {
                    left
                }
            } else {
                way
            };
            let need = PARTNER_AFTER_MS
                + move_ms(way - prev_x)
                + DETOUR_BEFORE_MS
                + DETOUR_AFTER_MS
                + move_ms(x - way)
                + PARTNER_EARLY_MS;
            gap = gap.max(need);
            Some((kind, mid.clamp(-BANK + w, BANK - w), w, way))
        } else {
            let need = PARTNER_AFTER_MS + move_ms(x - prev_x) + PARTNER_EARLY_MS;
            gap = gap.max(need);
            None
        };
        let at = prev_at + gap;
        if let Some((kind, mid, w, way)) = detour {
            // Halfway in time between the partner leaving one lane and
            // arriving at the next, so both moves fit.
            let leave = prev_at + PARTNER_AFTER_MS + move_ms(way - prev_x);
            let arrive = at - PARTNER_EARLY_MS - move_ms(x - way);
            let ice_at = (leave + DETOUR_BEFORE_MS + arrive - DETOUR_AFTER_MS) / 2;
            self.patches.push(Patch {
                kind,
                at: ice_at,
                x: mid,
                width: w,
            });
            self.holds.push(Hold {
                from: ice_at - DETOUR_BEFORE_MS,
                to: ice_at + DETOUR_AFTER_MS,
                x: way,
            });
        }
        let lit = at - level.lead_ms;
        if level.pairs {
            // Two lanterns, two lanes apart or more. The first lit is the
            // one to take; its note is the tune's next.
            let i = self.made as usize;
            let note = self.tune.get(i).copied().unwrap_or(0);
            let a = self.rng.range(-2, 2);
            let mut b = self.rng.range(-2, 2);
            while (a - b).abs() < 2 {
                b = self.rng.range(-2, 2);
            }
            let mut decoy_note = (note + 2 + self.rng.below(5) as usize) % 8;
            if decoy_note == note {
                decoy_note = (note + 3) % 8;
            }
            self.lanterns.push(Lantern {
                at,
                lit,
                lane: a,
                note,
                decoy: false,
            });
            self.lanterns.push(Lantern {
                at,
                lit: lit + PAIR_GAP_MS,
                lane: b,
                note: decoy_note,
                decoy: true,
            });
            self.last_lane = a;
        } else {
            let note = (lane + 2) as usize + if self.rng.chance(0.4) { 3 } else { 0 };
            self.lanterns.push(Lantern {
                at,
                lit,
                lane,
                note: note.min(7),
                decoy: false,
            });
            self.last_lane = lane;
            if level.partner {
                self.holds.push(Hold {
                    from: at - PARTNER_EARLY_MS,
                    to: at + PARTNER_AFTER_MS,
                    x,
                });
            }
        }
        if level.stalls && self.made % 5 == 2 {
            let kind = StallKind::ALL[(self.made as usize / 5) % StallKind::ALL.len()];
            let side = if (self.made / 5).is_multiple_of(2) {
                -1
            } else {
                1
            };
            self.stalls.push(Stall {
                at: at - gap / 2,
                kind,
                side,
            });
        }
        self.planned_to = at;
        self.made += 1;
    }

    /// Where your partner is at `t`: holding a lantern's lane, or gliding
    /// between them.
    pub fn partner_x(&self, t: i64) -> f64 {
        let i = self.holds.partition_point(|h| h.from <= t);
        let Some(prev) = i.checked_sub(1).map(|i| self.holds[i]) else {
            return 0.0;
        };
        if t <= prev.to {
            return prev.x;
        }
        match self.holds.get(i) {
            Some(next) => {
                let span = (next.from - prev.to).max(1) as f64;
                let k = ((t - prev.to) as f64 / span).clamp(0.0, 1.0);
                prev.x + (next.x - prev.x) * k
            }
            None => prev.x,
        }
    }

    pub fn has_ice(&self) -> bool {
        self.level.ice > 0.0
    }

    pub fn has_drifts(&self) -> bool {
        self.level.drifts > 0.0
    }

    /// Lanterns to take, counting a pair as one.
    pub fn to_take(&self) -> usize {
        self.lanterns.iter().filter(|l| !l.decoy).count()
    }
}

/// How deep the snow lies on the ice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cover {
    /// The ice is covered: the night is over.
    Covered,
    Deep,
    Settling,
    Clear,
}

impl Cover {
    pub fn of(clear: f64) -> Cover {
        if clear <= 0.0 {
            Cover::Covered
        } else if clear < 30.0 {
            Cover::Deep
        } else if clear < 60.0 {
            Cover::Settling
        } else {
            Cover::Clear
        }
    }

    /// "the ice clear".
    pub fn words(self) -> &'static str {
        match self {
            Cover::Covered => "the ice covered",
            Cover::Deep => "the snow deep",
            Cover::Settling => "the snow settling",
            Cover::Clear => "the ice clear",
        }
    }
}

/// How a spin went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spin {
    /// Let go once ready: points, more as the snow deepens.
    Landed(u32),
    /// Let go before it was ready: only a stride.
    Stride,
    /// Held too long: a wobble, and the line goes.
    Wobble,
    /// A second spin before the next lantern: no points.
    TooSoon,
}

/// How a skate ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// The snow covered the ice.
    SnowedOver,
    /// The course is skated to its end.
    Done,
}

/// What happened, for the screen to sound and say.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ev {
    /// A lantern lights ahead and is first heard.
    Lit {
        lantern: usize,
    },
    /// A lantern taken as it passed.
    Taken {
        lantern: usize,
        points: u32,
        line: u32,
    },
    /// The wrong lantern of a pair taken: the line goes.
    Wrong {
        lantern: usize,
    },
    /// A lantern went by out of reach.
    Passed {
        lantern: usize,
        line_lost: bool,
    },
    /// A drift hushes, or thin ice creaks, ahead.
    PatchHeard {
        patch: usize,
    },
    /// Into a drift: slowed, and the line goes.
    Drifted {
        patch: usize,
    },
    /// Thin ice cracked under you.
    Cracked {
        patch: usize,
    },
    /// A drift or thin ice went by beside you.
    PatchClear {
        patch: usize,
    },
    /// The spin is wound and ready to let go.
    SpinReady,
    Spun(Spin),
    /// The snow deepened past a step.
    Deeper(Cover),
    /// A stall at the fair is first heard.
    Stall {
        stall: usize,
    },
    Ended(End),
}

/// What a skate came to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub taken: u32,
    pub passed: u32,
    pub wrong: u32,
    pub drifts: u32,
    pub cracks: u32,
    pub patches_cleared: u32,
    pub spins: u32,
    pub wobbles: u32,
    pub best_line: u32,
    /// Points from spins.
    pub spin_points: u32,
}

/// A skate in progress.
#[derive(Clone, Debug)]
pub struct Skate {
    pub course: Course,
    t: f64,
    pub x: f64,
    /// How much of the ice is clear of snow, out of [`CLEAR_FULL`].
    pub clear: f64,
    pub score: u32,
    pub line: u32,
    pub tally: Tally,
    /// How each lantern went: taken, or not.
    pub done: Vec<Option<bool>>,
    lit: Vec<bool>,
    patch_heard: Vec<bool>,
    patch_done: Vec<bool>,
    stall_heard: Vec<bool>,
    cover: Cover,
    started: bool,
    ended: Option<End>,
    stumble_until: i64,
    slow_until: i64,
    /// When the main key went down for a spin, if it is held.
    spin_from: Option<i64>,
    spin_ready: bool,
    /// The lanterns passed when the last spin landed: one spin a lantern.
    spun_at: Option<usize>,
}

impl Skate {
    pub fn new(course: Course) -> Skate {
        let mut s = Skate {
            course,
            t: 0.0,
            x: 0.0,
            clear: CLEAR_FULL,
            score: 0,
            line: 0,
            tally: Tally::default(),
            done: Vec::new(),
            lit: Vec::new(),
            patch_heard: Vec::new(),
            patch_done: Vec::new(),
            stall_heard: Vec::new(),
            cover: Cover::Clear,
            started: false,
            ended: None,
            stumble_until: 0,
            slow_until: 0,
            spin_from: None,
            spin_ready: false,
            spun_at: None,
        };
        s.grow();
        s
    }

    fn grow(&mut self) {
        let n = self.course.lanterns.len();
        self.done.resize(n, None);
        self.lit.resize(n, false);
        let i = self.course.patches.len();
        self.patch_heard.resize(i, false);
        self.patch_done.resize(i, false);
        self.stall_heard.resize(self.course.stalls.len(), false);
    }

    pub fn start(&mut self) {
        self.started = true;
    }

    /// The same course from the start, skated on from where you are: the
    /// tutorial goes round again until each step is done.
    pub fn again(&mut self) {
        let mut next = Skate::new(Course::new(
            self.course.level,
            self.course.seed.wrapping_add(1),
            self.course.tune.clone(),
        ));
        next.x = self.x;
        next.tally = self.tally;
        next.started = true;
        *self = next;
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn ended(&self) -> Option<End> {
        self.ended
    }

    pub fn now(&self) -> i64 {
        self.t as i64
    }

    pub fn cover(&self) -> Cover {
        self.cover
    }

    pub fn lane(&self) -> i32 {
        nearest_lane(self.x)
    }

    /// Whether a spin is being held.
    pub fn spinning(&self) -> bool {
        self.spin_from.is_some()
    }

    pub fn spin_ready(&self) -> bool {
        self.spin_ready
    }

    pub fn stumbling(&self) -> bool {
        self.now() < self.stumble_until
    }

    /// Whether a drift is slowing you.
    pub fn slowed(&self) -> bool {
        self.now() < self.slow_until
    }

    /// Whether the snow can cover the ice on this skate.
    pub fn snows(&self) -> bool {
        self.course.level.snow > 0.0
    }

    /// The lantern coming now: lit, not yet passed, the first to come. In a
    /// pair, the first lit.
    pub fn next_lantern(&self) -> Option<usize> {
        (0..self.course.lanterns.len())
            .find(|&i| self.done[i].is_none() && self.lit[i] && !self.course.lanterns[i].decoy)
    }

    /// The other lantern of the pair coming now, once lit.
    pub fn next_decoy(&self) -> Option<usize> {
        (0..self.course.lanterns.len())
            .find(|&i| self.done[i].is_none() && self.lit[i] && self.course.lanterns[i].decoy)
    }

    /// A drift or thin ice heard and not yet passed.
    pub fn patch_ahead(&self) -> Option<usize> {
        (0..self.course.patches.len()).find(|&i| self.patch_heard[i] && !self.patch_done[i])
    }

    pub fn partner_x(&self) -> f64 {
        self.course.partner_x(self.now())
    }

    /// Lanterns to take that have passed.
    pub fn passed_count(&self) -> usize {
        self.course
            .lanterns
            .iter()
            .zip(&self.done)
            .filter(|(l, d)| !l.decoy && d.is_some())
            .count()
    }

    /// Points a landed spin earns now: 3 on clear ice, up to 5 as the snow
    /// deepens.
    pub fn spin_points(&self) -> u32 {
        if !self.snows() {
            return 3;
        }
        3 + ((CLEAR_FULL - self.clear).max(0.0) / 34.0).floor().min(2.0) as u32
    }

    /// The main key goes down: a spin starts winding.
    pub fn spin_down(&mut self) {
        if !self.started || self.ended.is_some() || self.stumbling() {
            return;
        }
        if self.spin_from.is_none() {
            self.spin_from = Some(self.now());
            self.spin_ready = false;
        }
    }

    /// The main key comes up: the spin lands, or was only a stride.
    pub fn spin_up(&mut self) -> Vec<Ev> {
        let Some(from) = self.spin_from.take() else {
            return Vec::new();
        };
        let held = self.now() - from;
        self.spin_ready = false;
        if held < SPIN_READY_MS {
            return vec![Ev::Spun(Spin::Stride)];
        }
        let passed = self.passed_count();
        if self.spun_at == Some(passed) {
            return vec![Ev::Spun(Spin::TooSoon)];
        }
        self.spun_at = Some(passed);
        let points = self.spin_points();
        self.score += points;
        self.tally.spins += 1;
        self.tally.spin_points += points;
        vec![Ev::Spun(Spin::Landed(points))]
    }

    /// Runs `dt` seconds, leaning by `lean`: -1 while Left is held, 1 while
    /// Right is, 0 otherwise.
    pub fn update(&mut self, dt: f64, lean: i32) -> Vec<Ev> {
        let mut evs = Vec::new();
        if !self.started || self.ended.is_some() {
            return evs;
        }
        self.t += dt * 1000.0;
        let now = self.now();
        self.course.extend(now + HORIZON_MS);
        self.grow();
        if !self.stumbling() && self.spin_from.is_none() {
            let rate = if self.slowed() {
                LEAN_RATE * DRIFT_LEAN
            } else {
                LEAN_RATE
            };
            self.x = (self.x + lean.signum() as f64 * rate * dt).clamp(-BANK, BANK);
        }
        if let Some(from) = self.spin_from {
            let held = now - from;
            if !self.spin_ready && held >= SPIN_READY_MS {
                self.spin_ready = true;
                evs.push(Ev::SpinReady);
            }
            if held >= SPIN_READY_MS + SPIN_HOLD_MS {
                self.spin_from = None;
                self.spin_ready = false;
                self.line = 0;
                self.tally.wobbles += 1;
                evs.push(Ev::Spun(Spin::Wobble));
            }
        }
        // The snow settles.
        let snow = self.course.level.snow;
        if snow > 0.0 {
            self.clear = (self.clear - snow * dt).max(0.0);
        }
        for i in 0..self.course.stalls.len() {
            if !self.stall_heard[i] && now >= self.course.stalls[i].at - 3000 {
                self.stall_heard[i] = true;
                evs.push(Ev::Stall { stall: i });
            }
        }
        for i in 0..self.course.patches.len() {
            let p = self.course.patches[i];
            if !self.patch_heard[i] && now >= p.heard_at() {
                self.patch_heard[i] = true;
                evs.push(Ev::PatchHeard { patch: i });
            }
            if self.patch_done[i] || now < p.at {
                continue;
            }
            self.patch_done[i] = true;
            if !p.under(self.x) {
                self.tally.patches_cleared += 1;
                evs.push(Ev::PatchClear { patch: i });
                continue;
            }
            self.line = 0;
            self.spin_from = None;
            self.spin_ready = false;
            match p.kind {
                PatchKind::Drift => {
                    self.tally.drifts += 1;
                    self.slow_until = now + DRIFT_MS;
                    evs.push(Ev::Drifted { patch: i });
                }
                PatchKind::Thin => {
                    self.tally.cracks += 1;
                    self.clear = (self.clear - CRACK_SNOW).max(0.0);
                    self.stumble_until = now + STUMBLE_MS;
                    evs.push(Ev::Cracked { patch: i });
                }
            }
        }
        for i in 0..self.course.lanterns.len() {
            let l = self.course.lanterns[i];
            if !self.lit[i] && now >= l.lit {
                self.lit[i] = true;
                evs.push(Ev::Lit { lantern: i });
            }
            if self.done[i].is_some() || now < l.at {
                continue;
            }
            let reach = (self.x - l.x()).abs() <= CATCH;
            if reach && l.decoy {
                self.done[i] = Some(true);
                self.tally.wrong += 1;
                self.line = 0;
                evs.push(Ev::Wrong { lantern: i });
            } else if reach {
                self.done[i] = Some(true);
                self.line += 1;
                self.tally.best_line = self.tally.best_line.max(self.line);
                let points = self.line.min(LINE_TOP);
                self.score += points;
                self.tally.taken += 1;
                self.clear = (self.clear + self.course.level.gain).min(CLEAR_FULL);
                evs.push(Ev::Taken {
                    lantern: i,
                    points,
                    line: self.line,
                });
            } else {
                self.done[i] = Some(false);
                if !l.decoy {
                    let line_lost = self.line > 0;
                    self.line = 0;
                    self.tally.passed += 1;
                    evs.push(Ev::Passed {
                        lantern: i,
                        line_lost,
                    });
                }
            }
        }
        let cover = Cover::of(self.clear);
        if cover < self.cover {
            evs.push(Ev::Deeper(cover));
        }
        self.cover = cover;
        if snow > 0.0 && self.clear <= 0.0 {
            self.end(End::SnowedOver, &mut evs);
        } else if self.course.end.is_some_and(|e| now >= e) {
            self.end(End::Done, &mut evs);
        }
        evs
    }

    fn end(&mut self, end: End, evs: &mut Vec<Ev>) {
        self.ended = Some(end);
        self.spin_from = None;
        evs.push(Ev::Ended(end));
    }
}

/// "no stars", "1 star", "3 stars".
pub fn stars_words(n: u32) -> String {
    match n {
        0 => "no stars".to_string(),
        1 => "1 star".to_string(),
        n => format!("{n} stars"),
    }
}

/// "1 point", "12 points".
pub fn points_words(n: u32) -> String {
    if n == 1 {
        "1 point".to_string()
    } else {
        format!("{n} points")
    }
}

/// "1 lantern", "12 lanterns".
pub fn lanterns_words(n: u32) -> String {
    if n == 1 {
        "1 lantern".to_string()
    } else {
        format!("{n} lanterns")
    }
}

/// Stars for `score` against a pond's three marks.
pub fn stars_for(score: u32, marks: [u32; 3]) -> u32 {
    marks.iter().filter(|&&m| score >= m).count() as u32
}
