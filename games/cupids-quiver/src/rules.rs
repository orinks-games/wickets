//! The rules: the sky, its drifting targets, the bow and its arrows.
//!
//! No screens and no sound. The play screen feeds input in (aim, draw,
//! release) and turns the [`Event`]s that come out into speech and sound,
//! so tests drive a whole round directly. Time runs on a
//! [`FixedStep`], so the same play gives the same result at any frame rate.
//!
//! Angles are degrees, 0 straight ahead, positive to the right. A target
//! drifts across the sky from one edge to the other at a near, middle or
//! far range. An arrow takes longer to reach a farther range, so a drifting
//! heart must be met a little ahead of where it is.

use parlor_core::{FixedStep, Rng, Seats};

/// How far the bow turns either way, in degrees.
pub const AIM_LIMIT: f64 = 70.0;
/// One press of Left or Right turns the bow this far.
pub const AIM_STEP: f64 = 5.0;
/// Where targets enter and leave the sky, either side.
pub const SKY_EDGE: f64 = 80.0;
/// Seconds from first pull to a full draw.
pub const DRAW_TIME: f64 = 0.8;
/// Substeps a second.
pub const STEP_HZ: f64 = 120.0;
/// Seconds before the first target of a round drifts in.
pub const LEAD_IN: f64 = 1.5;

/// Points for hitting both hearts of a pair one after the other.
pub const SWEETHEART_BONUS: u64 = 50;
/// Points a rose adds on top of its range.
pub const ROSE_BONUS: u64 = 25;
/// Points for a golden heart.
pub const GOLDEN_POINTS: u64 = 100;
/// Points for a folded love note.
pub const LETTER_POINTS: u64 = 40;
/// Each hit in a row past the first adds this, up to [`STREAK_CAP`].
pub const STREAK_STEP: u64 = 5;
pub const STREAK_CAP: u64 = 25;

/// The colours paired hearts come in. Both hearts of a pair share a colour
/// and hum the same note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Colour {
    Red,
    Pink,
    Peach,
    Violet,
    White,
}

impl Colour {
    pub const ALL: [Colour; 5] = [
        Colour::Red,
        Colour::Pink,
        Colour::Peach,
        Colour::Violet,
        Colour::White,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Colour::Red => "red",
            Colour::Pink => "pink",
            Colour::Peach => "peach",
            Colour::Violet => "violet",
            Colour::White => "white",
        }
    }

    /// The hum's pitch, a step of a major pentatonic scale: red lowest,
    /// white highest.
    pub fn pitch(self) -> f32 {
        match self {
            Colour::Red => 1.0,
            Colour::Pink => 1.122,
            Colour::Peach => 1.26,
            Colour::Violet => 1.498,
            Colour::White => 1.682,
        }
    }

    /// Where the colour sits in the scale, for the sound check: "the lowest".
    pub fn note_words(self) -> &'static str {
        match self {
            Colour::Red => "the lowest note",
            Colour::Pink => "the second note up",
            Colour::Peach => "the middle note",
            Colour::Violet => "the fourth note up",
            Colour::White => "the highest note",
        }
    }
}

/// What drifts in the sky.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A paper heart, one of a pair.
    Heart(Colour),
    /// A rose, in the Valentine's season: bonus points.
    Rose,
    /// A golden heart, on Valentine's night.
    Golden,
    /// A folded love note, drifting slowly. It holds a note to keep.
    Letter,
}

impl Kind {
    /// "pink heart", "rose", "golden heart", "love note".
    pub fn name(self) -> String {
        match self {
            Kind::Heart(c) => format!("{} heart", c.name()),
            Kind::Rose => "rose".to_string(),
            Kind::Golden => "golden heart".to_string(),
            Kind::Letter => "love note".to_string(),
        }
    }
}

/// How far off a target drifts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Range {
    Near,
    Middle,
    Far,
}

impl Range {
    pub const ALL: [Range; 3] = [Range::Near, Range::Middle, Range::Far];

    pub fn name(self) -> &'static str {
        match self {
            Range::Near => "near",
            Range::Middle => "middle distance",
            Range::Far => "far",
        }
    }

    /// Metres away, for placing its sound.
    pub fn metres(self) -> f32 {
        match self {
            Range::Near => 3.0,
            Range::Middle => 6.0,
            Range::Far => 10.0,
        }
    }

    /// Seconds an arrow takes to reach this range.
    pub fn flight(self) -> f64 {
        match self {
            Range::Near => 0.25,
            Range::Middle => 0.45,
            Range::Far => 0.7,
        }
    }

    /// Points for a heart at this range.
    pub fn points(self) -> u64 {
        match self {
            Range::Near => 10,
            Range::Middle => 20,
            Range::Far => 30,
        }
    }

    /// How far off the aim, either side, still hits at this range, before
    /// the round's tightening.
    pub fn tolerance(self) -> f64 {
        match self {
            Range::Near => 10.0,
            Range::Middle => 8.0,
            Range::Far => 6.0,
        }
    }
}

/// One thing drifting in the sky.
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub id: u32,
    pub kind: Kind,
    pub angle: f64,
    /// Degrees a second, positive drifting right.
    pub speed: f64,
    pub range: Range,
    /// The pair it belongs to: both hearts of a pair share it.
    pub pair: Option<u32>,
}

impl Target {
    /// Where it will be after `secs`.
    pub fn angle_after(&self, secs: f64) -> f64 {
        self.angle + self.speed * secs
    }

    fn leaving(&self) -> bool {
        (self.speed > 0.0 && self.angle > SKY_EDGE) || (self.speed < 0.0 && self.angle < -SKY_EDGE)
    }
}

/// An arrow in flight.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrow {
    pub aim: f64,
    pub t: f64,
    /// Ranges it has passed, from near.
    passed: usize,
    /// The seat that loosed it, in the game for two.
    pub shooter: usize,
}

/// The game's modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    /// The main game: six rooftops, each needing enough hearts.
    Rooftops,
    /// Endless rounds, opened by clearing the rooftops.
    Moonlight,
    /// A calm field: one slow heart at a time, endless arrows.
    Practice,
    /// Valentine's night, on the day only: golden hearts and roses.
    Night,
    /// Valentine's night for two, taking turns with one bow.
    ForTwo,
    /// The challenges.
    Daily,
    Weekly,
    Monthly,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Rooftops => "The rooftops",
            Mode::Moonlight => "Moonlight",
            Mode::Practice => "Practice field",
            Mode::Night => "Valentine's night",
            Mode::ForTwo => "For two",
            Mode::Daily => "Today's sky",
            Mode::Weekly => "The week's rooftops",
            Mode::Monthly => "The full moon",
        }
    }

    /// How many rounds, or `None` for no end.
    pub fn rounds(self) -> Option<u32> {
        match self {
            Mode::Rooftops => Some(ROOFTOPS.len() as u32),
            Mode::Moonlight | Mode::Practice => None,
            Mode::Night | Mode::ForTwo => Some(NIGHT_ROOFS.len() as u32),
            Mode::Daily => Some(1),
            Mode::Weekly => Some(3),
            Mode::Monthly => Some(5),
        }
    }

    /// Whether a round ends the run when too few hearts are hit.
    pub fn needs_goal(self) -> bool {
        matches!(self, Mode::Rooftops | Mode::Moonlight)
    }

    /// Whether this is a challenge.
    pub fn is_challenge(self) -> bool {
        matches!(self, Mode::Daily | Mode::Weekly | Mode::Monthly)
    }

    /// Whether this is one of the Valentine's Day modes.
    pub fn is_valentine(self) -> bool {
        matches!(self, Mode::Night | Mode::ForTwo)
    }

    /// Whether love notes can drift by.
    pub fn letters(self) -> bool {
        matches!(
            self,
            Mode::Rooftops | Mode::Moonlight | Mode::Night | Mode::ForTwo
        )
    }

    /// The records board, or `None` for practice and the challenges.
    pub fn board(self) -> Option<&'static str> {
        match self {
            Mode::Rooftops => Some("rooftops"),
            Mode::Moonlight => Some("moonlight"),
            Mode::Night => Some("night"),
            Mode::ForTwo => Some("for two"),
            _ => None,
        }
    }

    /// The difficulty level of `round` (from 1).
    pub fn level(self, round: u32) -> u32 {
        match self {
            Mode::Rooftops => round,
            Mode::Moonlight => round + 3,
            Mode::Practice => 1,
            Mode::Night | Mode::ForTwo => round + 1,
            Mode::Daily => 4,
            Mode::Weekly => round * 2,
            Mode::Monthly => round + 2,
        }
    }
}

/// The main game's rooftops, with a line of what each is like.
pub const ROOFTOPS: [(&str, &str); 6] = [
    (
        "The bakery roof",
        "Warm air rises from the ovens, and pigeons doze in the gutter.",
    ),
    (
        "The post office",
        "Sacks of valentines wait below, and the wind smells of ink.",
    ),
    (
        "The clock tower",
        "The great clock ticks behind you, slow and steady.",
    ),
    (
        "The old theatre",
        "Music drifts up through the skylight from the evening show.",
    ),
    (
        "The flower market",
        "The stalls are packing up, and petals blow across the tiles.",
    ),
    (
        "The cathedral roof",
        "The highest roof in town. The whole sky is yours.",
    ),
];

/// Valentine's night's four rooftops.
pub const NIGHT_ROOFS: [(&str, &str); 4] = [
    (
        "The bridge",
        "Lanterns sway on the bridge, and the river hums below.",
    ),
    (
        "The fountain square",
        "Couples throw coins in the fountain. Every one is a wish.",
    ),
    (
        "The rooftop cafe",
        "A violin plays somewhere below, and cups clink.",
    ),
    (
        "The highest roof in town",
        "The moon is full and the sky is gold with hearts.",
    ),
];

/// Moonlight's rooftops, in turn, round after round.
pub const MOON_ROOFS: [(&str, &str); 4] = [
    (
        "The chimney tops",
        "The moon climbs, and the hearts drift faster.",
    ),
    ("The weather vane", "Owls call from the chimneys."),
    (
        "The water tower",
        "Clouds slide past the moon, and the sky goes silver.",
    ),
    (
        "The old mill",
        "The town is asleep. Only the hearts are awake.",
    ),
];

/// A challenge's twist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Twist {
    /// Hearts drift half again as fast.
    Breeze,
    /// Every target is far.
    FarSky,
    /// Two more targets in the sky at once.
    Crowded,
    /// Sweetheart pairs are worth double.
    Sweethearts,
}

impl Twist {
    pub const ALL: [Twist; 4] = [
        Twist::Breeze,
        Twist::FarSky,
        Twist::Crowded,
        Twist::Sweethearts,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Twist::Breeze => "a breeze",
            Twist::FarSky => "a far sky",
            Twist::Crowded => "a crowded sky",
            Twist::Sweethearts => "sweethearts",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Twist::Breeze => "hearts drift half again as fast",
            Twist::FarSky => "every heart is far off, worth 30 each",
            Twist::Crowded => "two more hearts share the sky",
            Twist::Sweethearts => "sweetheart pairs are worth double",
        }
    }
}

/// One round's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub arrows: u32,
    /// Hits needed to go on, 0 when there is no need.
    pub goal: u32,
    /// Targets in the sky at once.
    pub live: usize,
    pub speed_lo: f64,
    pub speed_hi: f64,
    /// How likely a target is to be far, from 0 to 1.
    pub far: f64,
    /// Narrows every range's hit window.
    pub tolerance: f64,
    pub roses: bool,
    pub golden: f64,
    pub letters: bool,
    pub only_far: bool,
    pub bonus: u64,
}

impl Plan {
    /// The settings for `mode`'s `round`, with roses in season and the
    /// challenge's twist.
    pub fn new(mode: Mode, round: u32, roses: bool, twist: Option<Twist>) -> Plan {
        let level = mode.level(round).max(1);
        let l = level as f64;
        let arrows = 9 + level.min(8);
        let mut plan = Plan {
            arrows,
            goal: if mode.needs_goal() {
                (arrows as f64 * (0.38 + 0.025 * l.min(10.0))).round() as u32
            } else {
                0
            },
            live: (1 + (level as usize).div_ceil(2)).min(6),
            speed_lo: 5.0 + l,
            speed_hi: (9.0 + 2.0 * l).min(30.0),
            far: (l / (l + 4.0)).min(0.7),
            tolerance: (1.0 - 0.04 * l).max(0.6),
            roses: roses && !mode.is_challenge(),
            golden: if mode.is_valentine() { 0.18 } else { 0.0 },
            letters: mode.letters(),
            only_far: false,
            bonus: SWEETHEART_BONUS,
        };
        match mode {
            Mode::Practice => {
                plan.live = 1;
                plan.speed_lo = 4.0;
                plan.speed_hi = 7.0;
                plan.far = 0.0;
                plan.tolerance = 1.0;
                plan.letters = false;
            }
            Mode::Daily => plan.arrows = 15,
            Mode::ForTwo => plan.arrows *= 2,
            Mode::Monthly => plan.roses = true,
            _ => {}
        }
        match twist {
            Some(Twist::Breeze) => {
                plan.speed_lo *= 1.5;
                plan.speed_hi *= 1.5;
            }
            Some(Twist::FarSky) => plan.only_far = true,
            Some(Twist::Crowded) => plan.live += 2,
            Some(Twist::Sweethearts) => plan.bonus *= 2,
            None => {}
        }
        plan
    }

    /// The hit window at `range`, either side of the aim.
    pub fn window(&self, range: Range) -> f64 {
        range.tolerance() * self.tolerance
    }
}

/// Where the run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Playing,
    /// A round is over and the next waits for the player.
    Between,
    /// The run is over.
    Over,
}

/// What letting go of the string did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Release {
    /// An arrow is on its way.
    Shot,
    /// Not drawn far enough; the arrow stays in hand. Holds the draw, 0 to 1.
    TooSoft(f64),
    /// Nothing was drawn.
    NotDrawing,
}

/// Why the bow would not draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Empty,
    NotPlaying,
    Drawing,
}

/// Something that happened, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A target drifted in. `partner` when its pair's other heart is out.
    Spawn {
        id: u32,
        kind: Kind,
        from_left: bool,
        partner: bool,
    },
    /// A target drifted out of the sky unhit.
    Gone { id: u32, kind: Kind },
    /// The string reached a full draw.
    FullDraw,
    /// A hit.
    Hit {
        id: u32,
        kind: Kind,
        range: Range,
        angle: f64,
        points: u64,
        sweetheart: bool,
        streak: u32,
        shooter: usize,
    },
    /// An arrow passed every range without a hit.
    Miss { aim: f64, shooter: usize },
    /// Which target a shot let go now would meet, when that changes.
    Sights(Option<u32>),
    /// The round's quiver is spent and its last arrow down.
    RoundOver {
        round: u32,
        hits: u32,
        goal: u32,
        /// Whether another round follows.
        more: bool,
    },
}

/// Totals over a run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub shots: u32,
    pub hits: u32,
    pub pairs: u32,
    pub roses: u32,
    pub golden: u32,
    pub letters: u32,
    pub best_streak: u32,
    /// Hits by seat, in the game for two.
    pub by_seat: [u32; 2],
}

/// A run: its rounds, the sky, the bow and the score.
#[derive(Clone, Debug)]
pub struct Run {
    pub mode: Mode,
    pub twist: Option<Twist>,
    /// Roses in season, set before play starts.
    pub roses: bool,
    pub round: u32,
    pub plan: Plan,
    pub phase: Phase,
    pub aim: f64,
    /// The draw, 0 to 1, while the string is pulled.
    pub draw: Option<f64>,
    pub arrows_left: u32,
    pub sky: Vec<Target>,
    pub flying: Vec<Arrow>,
    pub score: u64,
    pub round_hits: u32,
    pub round_points: u64,
    pub streak: u32,
    pub tally: Tally,
    /// Partners waiting to drift in: (seconds, pair, colour).
    pending: Vec<(f64, u32, Colour)>,
    /// The pair of the last hit heart, while a sweetheart pair is possible.
    pub last_pair: Option<u32>,
    pub seats: Option<Seats>,
    sights: Option<u32>,
    spawn_in: f64,
    /// In practice, how long a partner waits once the sky is empty. The
    /// tutorial sets it, so a hit that ends the lesson never brings a new
    /// heart in the same moment.
    pub partner_gap: f64,
    /// How long the sky has been empty.
    empty_for: f64,
    letter_this_round: bool,
    next_id: u32,
    rng: Rng,
    clock: FixedStep,
    events: Vec<Event>,
    full_told: bool,
}

impl Run {
    pub fn new(mode: Mode, rng: Rng) -> Run {
        Run::with_twist(mode, None, rng)
    }

    pub fn with_twist(mode: Mode, twist: Option<Twist>, rng: Rng) -> Run {
        let plan = Plan::new(mode, 1, false, twist);
        Run {
            mode,
            twist,
            roses: false,
            round: 1,
            plan,
            phase: Phase::Playing,
            aim: 0.0,
            draw: None,
            arrows_left: plan.arrows,
            sky: Vec::new(),
            flying: Vec::new(),
            score: 0,
            round_hits: 0,
            round_points: 0,
            streak: 0,
            tally: Tally::default(),
            pending: Vec::new(),
            last_pair: None,
            seats: (mode == Mode::ForTwo).then(|| Seats::new(2)),
            sights: None,
            spawn_in: LEAD_IN,
            partner_gap: 0.0,
            empty_for: 0.0,
            letter_this_round: false,
            next_id: 1,
            rng,
            clock: FixedStep::hz(STEP_HZ),
            events: Vec::new(),
            full_told: false,
        }
    }

    /// Turns roses on or off, and replans the round in play.
    pub fn set_roses(&mut self, on: bool) {
        self.roses = on;
        self.plan = Plan::new(self.mode, self.round, on, self.twist);
    }

    /// Holds off the next target for `secs`, so a test can place its own.
    pub fn delay_spawns(&mut self, secs: f64) {
        self.spawn_in = secs;
    }

    /// Whether the quiver never empties.
    pub fn endless_arrows(&self) -> bool {
        self.mode == Mode::Practice
    }

    /// The rounds in this run, or `None` for no end.
    pub fn rounds(&self) -> Option<u32> {
        self.mode.rounds()
    }

    /// The round's name and line.
    pub fn round_place(&self) -> (String, &'static str) {
        let i = self.round.saturating_sub(1) as usize;
        match self.mode {
            Mode::Rooftops => {
                let (name, line) = ROOFTOPS[i.min(ROOFTOPS.len() - 1)];
                (name.to_string(), line)
            }
            Mode::Weekly => {
                let (name, line) = ROOFTOPS[(2 * i + 1).min(ROOFTOPS.len() - 1)];
                (name.to_string(), line)
            }
            Mode::Night | Mode::ForTwo => {
                let (name, line) = NIGHT_ROOFS[i.min(NIGHT_ROOFS.len() - 1)];
                (name.to_string(), line)
            }
            Mode::Moonlight | Mode::Monthly => {
                let (name, line) = MOON_ROOFS[i % MOON_ROOFS.len()];
                (name.to_string(), line)
            }
            Mode::Daily => ("The town square".to_string(), "One long round."),
            Mode::Practice => (
                "The practice field".to_string(),
                "A quiet field with one slow heart at a time.",
            ),
        }
    }

    /// The targets in the sky, by id.
    pub fn target(&self, id: u32) -> Option<&Target> {
        self.sky.iter().find(|t| t.id == id)
    }

    /// The other heart of a target's pair, if it is in the sky.
    pub fn partner_of(&self, id: u32) -> Option<&Target> {
        let pair = self.target(id)?.pair?;
        self.sky.iter().find(|t| t.id != id && t.pair == Some(pair))
    }

    /// The heart that would make a sweetheart pair if hit next.
    pub fn sweetheart_target(&self) -> Option<&Target> {
        let pair = self.last_pair?;
        self.sky.iter().find(|t| t.pair == Some(pair))
    }

    /// Whether a partner of the last pair is still to come.
    pub fn partner_pending(&self) -> bool {
        self.last_pair
            .is_some_and(|p| self.pending.iter().any(|&(_, pair, _)| pair == p))
    }

    /// The target a shot let go now would meet, nearest range first.
    pub fn in_sights(&self) -> Option<u32> {
        self.predict(self.aim)
    }

    fn predict(&self, aim: f64) -> Option<u32> {
        for range in Range::ALL {
            let window = self.plan.window(range);
            let best = self
                .sky
                .iter()
                .filter(|t| t.range == range)
                .map(|t| (t.id, (t.angle_after(range.flight()) - aim).abs()))
                .filter(|&(_, d)| d <= window)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((id, _)) = best {
                return Some(id);
            }
        }
        None
    }

    /// Turns the bow one step. Returns false when it is already at the
    /// limit that way.
    pub fn turn(&mut self, right: bool) -> bool {
        let next =
            (self.aim + if right { AIM_STEP } else { -AIM_STEP }).clamp(-AIM_LIMIT, AIM_LIMIT);
        let moved = next != self.aim;
        self.aim = next;
        moved
    }

    /// Faces straight ahead.
    pub fn centre(&mut self) {
        self.aim = 0.0;
    }

    /// Starts drawing the string.
    pub fn begin_draw(&mut self) -> Result<(), Refusal> {
        if self.phase != Phase::Playing {
            return Err(Refusal::NotPlaying);
        }
        if self.draw.is_some() {
            return Err(Refusal::Drawing);
        }
        if self.arrows_left == 0 && !self.endless_arrows() {
            return Err(Refusal::Empty);
        }
        self.draw = Some(0.0);
        self.full_told = false;
        Ok(())
    }

    /// Eases the string back without shooting. Returns whether it was drawn.
    pub fn cancel_draw(&mut self) -> bool {
        self.draw.take().is_some()
    }

    pub fn drawing(&self) -> bool {
        self.draw.is_some()
    }

    /// Lets go of the string: a full draw shoots.
    pub fn release(&mut self) -> Release {
        let Some(draw) = self.draw.take() else {
            return Release::NotDrawing;
        };
        if draw < 1.0 {
            return Release::TooSoft(draw);
        }
        let shooter = self.seats.as_ref().map_or(0, Seats::current);
        self.flying.push(Arrow {
            aim: self.aim,
            t: 0.0,
            passed: 0,
            shooter,
        });
        if !self.endless_arrows() {
            self.arrows_left = self.arrows_left.saturating_sub(1);
        }
        self.tally.shots += 1;
        if let Some(seats) = self.seats.as_mut() {
            seats.pass();
        }
        Release::Shot
    }

    /// Starts the next round, after [`Phase::Between`].
    pub fn next_round(&mut self) -> bool {
        if self.phase != Phase::Between {
            return false;
        }
        self.round += 1;
        self.plan = Plan::new(self.mode, self.round, self.roses, self.twist);
        self.arrows_left = self.plan.arrows;
        self.round_hits = 0;
        self.round_points = 0;
        self.spawn_in = LEAD_IN;
        self.letter_this_round = false;
        self.last_pair = None;
        self.streak = 0;
        self.phase = Phase::Playing;
        true
    }

    /// Runs time on, and returns what happened.
    pub fn update(&mut self, dt: f64) -> Vec<Event> {
        let mut clock = self.clock;
        clock.advance(dt, |h| self.step(h));
        self.clock = clock;
        std::mem::take(&mut self.events)
    }

    fn step(&mut self, h: f64) {
        if self.phase != Phase::Playing {
            return;
        }
        if let Some(draw) = self.draw.as_mut() {
            let was = *draw;
            *draw = (*draw + h / DRAW_TIME).min(1.0);
            if was < 1.0 && *draw >= 1.0 && !self.full_told {
                self.full_told = true;
                self.events.push(Event::FullDraw);
            }
        }
        for t in self.sky.iter_mut() {
            t.angle += t.speed * h;
        }
        let mut gone = Vec::new();
        self.sky.retain(|t| {
            if t.leaving() {
                gone.push(Event::Gone {
                    id: t.id,
                    kind: t.kind,
                });
                false
            } else {
                true
            }
        });
        self.events.extend(gone);
        self.fly(h);
        self.spawn(h);
        let sights = self.in_sights();
        if sights != self.sights {
            self.sights = sights;
            self.events.push(Event::Sights(sights));
        }
        if self.arrows_left == 0 && self.flying.is_empty() && !self.endless_arrows() {
            self.end_round();
        }
    }

    fn fly(&mut self, h: f64) {
        let mut arrows = std::mem::take(&mut self.flying);
        arrows.retain_mut(|a| {
            a.t += h;
            while a.passed < Range::ALL.len() && a.t >= Range::ALL[a.passed].flight() {
                let range = Range::ALL[a.passed];
                a.passed += 1;
                if self.strike(a, range) {
                    return false;
                }
            }
            if a.passed == Range::ALL.len() {
                self.miss(a);
                return false;
            }
            true
        });
        self.flying = arrows;
    }

    /// The arrow reaching `range`: hits the closest target there in its window.
    fn strike(&mut self, a: &Arrow, range: Range) -> bool {
        let window = self.plan.window(range);
        let Some(at) = self
            .sky
            .iter()
            .enumerate()
            .filter(|(_, t)| t.range == range)
            .map(|(i, t)| (i, (t.angle - a.aim).abs()))
            .filter(|&(_, d)| d <= window)
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .map(|(i, _)| i)
        else {
            return false;
        };
        let t = self.sky.remove(at);
        let sweetheart = t.pair.is_some() && t.pair == self.last_pair;
        self.streak += 1;
        let mut points = match t.kind {
            Kind::Heart(_) => range.points(),
            Kind::Rose => range.points() + ROSE_BONUS,
            Kind::Golden => GOLDEN_POINTS,
            Kind::Letter => LETTER_POINTS,
        };
        points += (STREAK_STEP * u64::from(self.streak.saturating_sub(1))).min(STREAK_CAP);
        if sweetheart {
            points += self.plan.bonus;
            self.tally.pairs += 1;
            self.last_pair = None;
        } else {
            self.last_pair = t.pair;
        }
        match t.kind {
            Kind::Rose => self.tally.roses += 1,
            Kind::Golden => self.tally.golden += 1,
            Kind::Letter => self.tally.letters += 1,
            Kind::Heart(_) => {}
        }
        self.tally.hits += 1;
        self.tally.best_streak = self.tally.best_streak.max(self.streak);
        if let Some(n) = self.tally.by_seat.get_mut(a.shooter) {
            *n += 1;
        }
        self.round_hits += 1;
        self.round_points += points;
        self.score += points;
        self.events.push(Event::Hit {
            id: t.id,
            kind: t.kind,
            range,
            angle: t.angle,
            points,
            sweetheart,
            streak: self.streak,
            shooter: a.shooter,
        });
        true
    }

    fn miss(&mut self, a: &Arrow) {
        self.streak = 0;
        self.last_pair = None;
        self.events.push(Event::Miss {
            aim: a.aim,
            shooter: a.shooter,
        });
    }

    fn end_round(&mut self) {
        self.draw = None;
        self.pending.clear();
        for t in self.sky.drain(..) {
            self.events.push(Event::Gone {
                id: t.id,
                kind: t.kind,
            });
        }
        let goal_met = self.round_hits >= self.plan.goal;
        let last = self.rounds().is_some_and(|n| self.round >= n);
        let more = goal_met && !last;
        self.phase = if more { Phase::Between } else { Phase::Over };
        self.events.push(Event::RoundOver {
            round: self.round,
            hits: self.round_hits,
            goal: self.plan.goal,
            more,
        });
    }

    /// Whether the run ended with every round's goal met.
    pub fn cleared(&self) -> bool {
        self.phase == Phase::Over && self.round_hits >= self.plan.goal
    }

    fn spawning(&self) -> bool {
        self.endless_arrows() || self.arrows_left > 0
    }

    fn spawn(&mut self, h: f64) {
        if !self.spawning() {
            return;
        }
        // In practice a partner waits for the sky to be empty, so only one
        // heart drifts at a time.
        if self.sky.is_empty() {
            self.empty_for += h;
        } else {
            self.empty_for = 0.0;
        }
        let hold = self.mode == Mode::Practice
            && (!self.sky.is_empty() || self.empty_for < self.partner_gap);
        let mut due = Vec::new();
        self.pending.retain_mut(|p| {
            p.0 -= h;
            if p.0 <= 0.0 && !hold {
                due.push((p.1, p.2));
                false
            } else {
                true
            }
        });
        for (pair, colour) in due {
            self.add(Kind::Heart(colour), Some(pair), true);
        }
        self.spawn_in -= h;
        if self.spawn_in > 0.0 {
            return;
        }
        if self.sky.len() + self.pending.len() >= self.plan.live {
            self.spawn_in = 0.5;
            return;
        }
        self.spawn_in = 1.2 + self.rng.unit() * 1.3;
        let plan = self.plan;
        if plan.golden > 0.0 && self.rng.chance(plan.golden) {
            self.add(Kind::Golden, None, false);
        } else if plan.roses && self.rng.chance(0.22) {
            self.add(Kind::Rose, None, false);
        } else if plan.letters && !self.letter_this_round && self.rng.chance(0.06) {
            self.letter_this_round = true;
            self.add(Kind::Letter, None, false);
        } else {
            let colour = self.free_colour();
            let pair = self.next_id;
            self.add(Kind::Heart(colour), Some(pair), false);
            let delay = 0.6 + self.rng.unit() * 1.9;
            self.pending.push((delay, pair, colour));
        }
    }

    /// A colour no heart in the sky has, if there is one.
    fn free_colour(&mut self) -> Colour {
        let used: Vec<Colour> = self
            .sky
            .iter()
            .filter_map(|t| match t.kind {
                Kind::Heart(c) => Some(c),
                _ => None,
            })
            .chain(self.pending.iter().map(|p| p.2))
            .collect();
        let free: Vec<Colour> = Colour::ALL
            .into_iter()
            .filter(|c| !used.contains(c))
            .collect();
        if free.is_empty() {
            *self.rng.pick(&Colour::ALL)
        } else {
            *self.rng.pick(&free)
        }
    }

    fn pick_range(&mut self, kind: Kind) -> Range {
        let plan = self.plan;
        if plan.only_far {
            return Range::Far;
        }
        if self.mode == Mode::Practice {
            return if self.rng.chance(0.5) {
                Range::Near
            } else {
                Range::Middle
            };
        }
        if kind == Kind::Golden {
            return if self.rng.chance(0.6) {
                Range::Far
            } else {
                Range::Middle
            };
        }
        let u = self.rng.unit();
        if u < plan.far {
            Range::Far
        } else if u < plan.far + (1.0 - plan.far) / 2.0 {
            Range::Middle
        } else {
            Range::Near
        }
    }

    fn add(&mut self, kind: Kind, pair: Option<u32>, partner: bool) {
        let id = self.next_id;
        self.next_id += 1;
        let range = self.pick_range(kind);
        let from_left = self.rng.chance(0.5);
        let plan = self.plan;
        let mut speed = plan.speed_lo + self.rng.unit() * (plan.speed_hi - plan.speed_lo);
        match kind {
            Kind::Golden => speed *= 1.3,
            Kind::Letter => speed *= 0.6,
            _ => {}
        }
        let (angle, speed) = if from_left {
            (-SKY_EDGE, speed)
        } else {
            (SKY_EDGE, -speed)
        };
        self.sky.push(Target {
            id,
            kind,
            angle,
            speed,
            range,
            pair,
        });
        self.events.push(Event::Spawn {
            id,
            kind,
            from_left,
            partner,
        });
    }

    /// The share of shots that hit, as a whole percent.
    pub fn accuracy(&self) -> u32 {
        (self.tally.hits * 100)
            .checked_div(self.tally.shots)
            .unwrap_or(0)
    }
}

/// The bow's aim in words: "straight ahead", "15 degrees right".
pub fn aim_words(aim: f64) -> String {
    let n = aim.round() as i64;
    match n {
        0 => "straight ahead".to_string(),
        n if n > 0 => format!("{} right", degrees_words(n)),
        n => format!("{} left", degrees_words(-n)),
    }
}

/// Where a target is from the aim: "20 degrees right of your aim".
pub fn offset_words(from: f64, to: f64) -> String {
    let d = (to - from).round() as i64;
    match d {
        0 => "dead ahead of your aim".to_string(),
        d if d > 0 => format!("{} right of your aim", degrees_words(d)),
        d => format!("{} left of your aim", degrees_words(-d)),
    }
}

/// How to turn from the aim to an angle: "turn 35 degrees left", "hold your aim".
pub fn turn_words(from: f64, to: f64) -> String {
    let d = (to - from).round() as i64;
    match d {
        0 => "hold your aim".to_string(),
        d if d > 0 => format!("turn {} right", degrees_words(d)),
        d => format!("turn {} left", degrees_words(-d)),
    }
}

/// "1 degree", "15 degrees".
pub fn degrees_words(n: i64) -> String {
    match n {
        1 => "1 degree".to_string(),
        n => format!("{n} degrees"),
    }
}

/// "1 arrow", "7 arrows".
pub fn arrows_words(n: u32) -> String {
    match n {
        1 => "1 arrow".to_string(),
        n => format!("{n} arrows"),
    }
}

/// "1 heart", "4 hearts".
pub fn hearts_words(n: u32) -> String {
    match n {
        1 => "1 heart".to_string(),
        n => format!("{n} hearts"),
    }
}
