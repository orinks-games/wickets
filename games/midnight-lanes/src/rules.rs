//! The rules of the lane, with no sound or screen: the rack of pins, the
//! pin model, the release, and ten-pin scoring.
//!
//! Positions across the lane are in boards, numbered 1 at the left edge
//! to 39 at the right, with board 20 in the middle. [`pinfall`] is the
//! clean model, the same pins for the same line, which hints and the
//! tutorial use. A ball in play goes through [`pinfall_live`] instead: the
//! lane draws the ball a little off line and each pin flies a little
//! differently, from a seeded [`Rng`], so a perfect pocket hit sometimes
//! leaves a pin, as in real bowling.

use parlor_core::Rng;

/// Boards across the lane.
pub const BOARDS: i32 = 39;
/// The middle board, in line with the head pin.
pub const CENTRE: i32 = 20;
/// The closest boards to the gutters a player can stand on.
pub const STAND_MIN: i32 = 3;
pub const STAND_MAX: i32 = 37;
/// How far the aim turns either way, in steps of one board at the pins.
pub const AIM_MAX: i32 = 10;

/// Half the distance between two pins side by side, in boards.
const HALF_GAP: f64 = 5.64;
/// The distance from one row of pins to the next, in boards.
const ROW_DEPTH: f64 = 9.77;
/// Ball and pin touch when their centres are this close, in boards.
const BALL_CONTACT: f64 = 6.25;
/// Two pins touch when their centres are this close, in boards.
const PIN_CONTACT: f64 = 4.48;
/// A falling pin with less energy than this knocks nothing more down.
const MIN_ENERGY: f64 = 0.35;
/// The steepest a pin flies off sideways, in radians.
const MAX_ANGLE: f64 = 1.05;
/// The lane's length from the foul line to the head pin, in boards, for
/// the ball's angle into the pins. Shorter than the real lane, so that
/// where the player stands changes how the ball meets the pins.
pub const LANE_BOARDS: f64 = 70.0;

/// Where pin `n` (1 to 10) stands: boards from the middle, and boards back
/// from the head pin.
pub fn pin_spot(n: u8) -> (f64, f64) {
    let (col, row) = match n {
        1 => (0.0, 0.0),
        2 => (-1.0, 1.0),
        3 => (1.0, 1.0),
        4 => (-2.0, 2.0),
        5 => (0.0, 2.0),
        6 => (2.0, 2.0),
        7 => (-3.0, 3.0),
        8 => (-1.0, 3.0),
        9 => (1.0, 3.0),
        10 => (3.0, 3.0),
        _ => panic!("no pin {n}"),
    };
    (col * HALF_GAP, row * ROW_DEPTH)
}

/// A set of pins, by number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rack(pub u16);

impl Rack {
    pub const FULL: Rack = Rack(0x3FF);
    pub const EMPTY: Rack = Rack(0);

    pub fn of(pins: &[u8]) -> Rack {
        let mut r = Rack::EMPTY;
        for &p in pins {
            r.add(p);
        }
        r
    }

    pub fn has(self, pin: u8) -> bool {
        (1..=10).contains(&pin) && self.0 & (1 << (pin - 1)) != 0
    }

    pub fn add(&mut self, pin: u8) {
        assert!((1..=10).contains(&pin), "no pin {pin}");
        self.0 |= 1 << (pin - 1);
    }

    pub fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn pins(self) -> Vec<u8> {
        (1..=10).filter(|&p| self.has(p)).collect()
    }

    pub fn without(self, other: Rack) -> Rack {
        Rack(self.0 & !other.0)
    }

    /// A split: the head pin is down and the pins left standing fall into
    /// two or more groups with a gap between them.
    pub fn is_split(self) -> bool {
        if self.has(1) || self.count() < 2 {
            return false;
        }
        let pins = self.pins();
        let mut seen = vec![pins[0]];
        let mut i = 0;
        while i < seen.len() {
            let (x, z) = pin_spot(seen[i]);
            for &q in &pins {
                let (qx, qz) = pin_spot(q);
                let near = ((qx - x).powi(2) + (qz - z).powi(2)).sqrt() < 2.0 * HALF_GAP + 0.5;
                if near && !seen.contains(&q) {
                    seen.push(q);
                }
            }
            i += 1;
        }
        seen.len() < pins.len()
    }
}

/// One ball as it reaches the pins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    /// The board the player stood on.
    pub stand: i32,
    /// The board the ball crosses at the head pin, pull included.
    pub at: f64,
}

impl Line {
    /// The line from `stand` with `aim` steps, pulled by `pull` boards.
    pub fn new(stand: i32, aim: i32, pull: f64) -> Line {
        Line {
            stand,
            at: (stand + aim) as f64 + pull,
        }
    }

    /// Whether the ball leaves the lane before it reaches the pins.
    pub fn gutter(&self) -> bool {
        self.at < 0.5 || self.at > BOARDS as f64 + 0.5
    }

    /// How far down the lane, from 0 to 1, the ball drops into the
    /// gutter, for a gutter ball.
    pub fn gutter_at(&self) -> Option<f64> {
        if !self.gutter() {
            return None;
        }
        let edge = if self.at < 0.5 {
            0.5
        } else {
            BOARDS as f64 + 0.5
        };
        let span = self.at - self.stand as f64;
        Some(((edge - self.stand as f64) / span).clamp(0.0, 1.0))
    }

    /// The board under the ball at `progress`, from 0 at the foul line to
    /// 1 at the head pin.
    pub fn board_at(&self, progress: f64) -> f64 {
        let p = match self.gutter_at() {
            Some(edge) => progress.min(edge),
            None => progress,
        };
        self.stand as f64 + (self.at - self.stand as f64) * p.clamp(0.0, 1.0)
    }

    /// How well the ball angles in towards the head pin, from 0 for a
    /// straight ball or one angling away, to 1 for a good pocket angle.
    pub fn inward(&self) -> f64 {
        let offset = self.at - CENTRE as f64;
        (-offset.signum() * self.angle()).clamp(0.0, GOOD_ANGLE) / GOOD_ANGLE
    }

    /// The ball's angle across the lane, in radians, positive to the right.
    fn angle(&self) -> f64 {
        ((self.at - self.stand as f64) / LANE_BOARDS).atan()
    }
}

/// Something moving through the pins: the ball, or a pin it sent flying.
struct Striker {
    x: f64,
    z: f64,
    angle: f64,
    energy: f64,
    contact: f64,
}

/// The pins a clean ball on `line` knocks down from `standing`: the same
/// pins every time.
pub fn pinfall(standing: Rack, line: &Line) -> Rack {
    fall(standing, line, None)
}

/// How far the lane may draw a ball off its line, in boards either way.
pub const LANE_DRAW: f64 = 0.8;
/// The most a pin's flight may turn by chance at each hit, in radians.
const SCATTER: f64 = 0.3;
/// The most energy a pin may lose by chance at each hit, as a fraction.
const SOFT_HIT: f64 = 0.35;
/// An entry angle towards the head pin this steep, in radians, scatters
/// the pins least.
const GOOD_ANGLE: f64 = 0.08;

/// The pins a ball in play knocks down from `standing`. The lane draws it
/// up to [`LANE_DRAW`] boards off `line`, and each pin flies with a little
/// chance, so the same line does not always give the same pins.
pub fn pinfall_live(standing: Rack, line: &Line, rng: &mut Rng) -> Rack {
    if steady_lane() {
        return pinfall(standing, line);
    }
    let draw = (rng.unit() + rng.unit() - 1.0) * LANE_DRAW;
    let drawn = Line {
        at: line.at + draw,
        ..*line
    };
    // The draw never sends a ball on the lane into the gutter.
    let line = if drawn.gutter() { *line } else { drawn };
    // A ball angling in towards the head pin carries the pins better than
    // one rolled straight: it scatters them less.
    let scatter = 1.3 - 0.6 * line.inward();
    fall(standing, &line, Some((rng, scatter)))
}

/// The pin model. With `noise`, each hit turns and softens a little by
/// chance, and a pocket hit is not an automatic strike.
fn fall(standing: Rack, line: &Line, mut noise: Option<(&mut Rng, f64)>) -> Rack {
    if line.gutter() {
        return Rack::EMPTY;
    }
    let offset = line.at - CENTRE as f64;
    let mut up = standing;
    let mut down = Rack::EMPTY;
    let mut strikers = vec![Striker {
        x: offset,
        z: -1.0,
        angle: line.angle(),
        energy: 1.0,
        contact: BALL_CONTACT,
    }];
    strikers[0].x -= strikers[0].angle.tan();
    loop {
        // The next contact down the deck: the hit on the nearest pin.
        let mut next: Option<(f64, usize, u8, f64)> = None;
        for (i, s) in strikers.iter().enumerate() {
            if s.energy < MIN_ENERGY {
                continue;
            }
            for pin in up.pins() {
                let (px, pz) = pin_spot(pin);
                if pz <= s.z {
                    continue;
                }
                let path = s.x + s.angle.tan() * (pz - s.z);
                let off = (px - path) * s.angle.cos();
                if off.abs() >= s.contact {
                    continue;
                }
                if next.is_none_or(|(z, _, _, _)| pz < z) {
                    next = Some((pz, i, pin, off));
                }
            }
        }
        let Some((_, i, pin, off)) = next else {
            break;
        };
        let s = &strikers[i];
        let mut turn = (off / s.contact).asin();
        let square = turn.cos().max(0.3);
        let mut energy = if i == 0 { 1.0 } else { s.energy * 0.8 * square };
        if let Some((rng, scale)) = noise.as_mut() {
            turn += (rng.unit() * 2.0 - 1.0) * SCATTER * *scale;
            energy *= 1.0 - rng.unit() * SOFT_HIT * *scale;
        }
        let angle = (s.angle + turn).clamp(-MAX_ANGLE, MAX_ANGLE);
        let (px, pz) = pin_spot(pin);
        up = up.without(Rack::of(&[pin]));
        down.add(pin);
        strikers.push(Striker {
            x: px,
            z: pz,
            angle,
            energy,
            contact: PIN_CONTACT,
        });
    }
    down
}

/// The top of the swing, in seconds after the swing starts.
pub const SWING_TOP: f64 = 0.9;
/// The swing lets go by itself this long after it starts.
pub const SWING_END: f64 = 1.8;
/// A release this close to the top is clean: no pull.
pub const CLEAN: f64 = 0.05;
/// Boards of pull for each second the release is off the clean window,
/// before the lane's own draw.
pub const PULL_RATE: f64 = 30.0;

/// The pull of a release `error` seconds off the top of the swing,
/// negative for early. Always draws twice from `rng`, so a clean release
/// leaves the lane's sequence where a pulled one would.
pub fn pull(error: f64, rng: &mut Rng) -> f64 {
    let size = 0.6 + 0.8 * rng.unit();
    let left = rng.chance(0.5);
    let off = (error.abs() - CLEAN).max(0.0);
    if off == 0.0 {
        return 0.0;
    }
    let boards = off * PULL_RATE * size;
    if left {
        -boards
    } else {
        boards
    }
}

/// Ten frames.
pub const FRAMES: usize = 10;

/// One frame as the score sheet shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub rolls: Vec<u8>,
    /// The running total up to this frame, once its bonus rolls are in.
    pub total: Option<u32>,
}

impl Frame {
    pub fn strike(&self) -> bool {
        self.rolls.first() == Some(&10)
    }

    pub fn spare(&self) -> bool {
        !self.strike() && self.rolls.len() >= 2 && self.rolls[0] + self.rolls[1] == 10
    }
}

/// A game of ten-pin scoring, roll by roll.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    pub rolls: Vec<u8>,
}

impl Game {
    pub fn new() -> Game {
        Game::default()
    }

    /// A game from a list of rolls, for tests and replays.
    pub fn from_rolls(rolls: &[u8]) -> Game {
        let mut g = Game::new();
        for &r in rolls {
            g.roll(r);
        }
        g
    }

    /// Adds a roll of `pins`. Panics when the game is over or the roll
    /// knocks more pins than were standing.
    pub fn roll(&mut self, pins: u8) {
        assert!(!self.is_over(), "the game is over");
        assert!(pins <= self.standing(), "only {} pins up", self.standing());
        self.rolls.push(pins);
    }

    /// The frames so far, each with its running total where known.
    pub fn frames(&self) -> Vec<Frame> {
        let r = &self.rolls;
        let at = |i: usize| r.get(i).map(|&p| p as u32);
        let mut frames = Vec::new();
        let mut i = 0;
        let mut total = Some(0u32);
        for f in 0..FRAMES {
            if i >= r.len() {
                break;
            }
            let (rolls, score, used) = if f + 1 < FRAMES {
                if r[i] == 10 {
                    let bonus = at(i + 1).zip(at(i + 2)).map(|(a, b)| 10 + a + b);
                    (vec![10], bonus, 1)
                } else {
                    let end = (i + 2).min(r.len());
                    let rolls = r[i..end].to_vec();
                    let score = match at(i + 1) {
                        Some(b) if r[i] as u32 + b == 10 => at(i + 2).map(|c| 10 + c),
                        Some(b) => Some(r[i] as u32 + b),
                        None => None,
                    };
                    (rolls, score, 2)
                }
            } else {
                let rolls = r[i..].to_vec();
                let needed = Self::tenth_length(&rolls);
                let score = (rolls.len() == needed).then(|| rolls.iter().map(|&p| p as u32).sum());
                (rolls, score, needed)
            };
            total = total.zip(score).map(|(t, s)| t + s);
            frames.push(Frame { rolls, total });
            i += used;
        }
        frames
    }

    /// How many rolls the tenth frame takes, from the rolls in it so far.
    fn tenth_length(rolls: &[u8]) -> usize {
        match rolls {
            [10, ..] => 3,
            [a, b, ..] if a + b == 10 => 3,
            _ => 2,
        }
    }

    /// The frame (1 to 10) and roll in it (1 to 3) of the next roll, or
    /// None when the game is over.
    pub fn next(&self) -> Option<(usize, usize)> {
        let frames = self.frames();
        let Some(last) = frames.last() else {
            return Some((1, 1));
        };
        let n = frames.len();
        if n < FRAMES {
            if last.strike() || last.rolls.len() == 2 {
                Some((n + 1, 1))
            } else {
                Some((n, 2))
            }
        } else if last.rolls.len() < Self::tenth_length(&last.rolls) {
            Some((n, last.rolls.len() + 1))
        } else {
            None
        }
    }

    pub fn is_over(&self) -> bool {
        self.next().is_none()
    }

    /// Whether the next roll faces a full rack.
    pub fn fresh_rack(&self) -> bool {
        self.standing() == 10
    }

    /// Whether the next roll is the first ball at a newly set rack, the
    /// only ball that can be a strike. A full rack is not enough: after a
    /// first ball of 0 all ten still stand, and clearing them is a spare.
    pub fn new_rack(&self) -> bool {
        let frames = self.frames();
        match (self.next(), frames.last()) {
            (None, _) => false,
            (Some((_, 1)), _) | (_, None) => true,
            (Some((f, 2)), Some(last)) if f == FRAMES => last.rolls[0] == 10,
            (Some((f, 3)), Some(last)) if f == FRAMES => {
                let (a, b) = (last.rolls[0], last.rolls[1]);
                (a == 10 && b == 10) || (a != 10 && a + b == 10)
            }
            _ => false,
        }
    }

    /// How many pins stand for the next roll.
    pub fn standing(&self) -> u8 {
        let frames = self.frames();
        match (self.next(), frames.last()) {
            (None, _) | (Some((_, 1)), _) | (_, None) => 10,
            (Some((f, roll)), Some(last)) if f < FRAMES || roll == 2 => {
                if f == FRAMES && last.rolls[0] == 10 {
                    10
                } else {
                    10 - last.rolls[0]
                }
            }
            (Some(_), Some(last)) => {
                let (a, b) = (last.rolls[0], last.rolls[1]);
                if (a == 10 && b == 10) || (a != 10 && a + b == 10) {
                    10
                } else {
                    10 - b
                }
            }
        }
    }

    /// The total of every frame scored so far.
    pub fn score(&self) -> u32 {
        self.frames()
            .iter()
            .filter_map(|f| f.total)
            .next_back()
            .unwrap_or(0)
    }

    /// The final score, once the game is over.
    pub fn final_score(&self) -> Option<u32> {
        self.is_over().then(|| self.score())
    }
}

/// Pins as a spoken list: "4, 7 and 10".
pub fn pin_list(rack: Rack) -> String {
    let pins: Vec<String> = rack.pins().iter().map(|p| p.to_string()).collect();
    match pins.len() {
        0 => "none".to_string(),
        1 => pins[0].clone(),
        n => format!("{} and {}", pins[..n - 1].join(", "), pins[n - 1]),
    }
}

/// "Board 15, left of centre."
pub fn board_words(board: i32) -> String {
    match board.cmp(&CENTRE) {
        std::cmp::Ordering::Less => format!("Board {board}, left of centre."),
        std::cmp::Ordering::Equal => format!("Board {board}, centre."),
        std::cmp::Ordering::Greater => format!("Board {board}, right of centre."),
    }
}

/// "Aim straight", "aim 2 right".
pub fn aim_words(aim: i32) -> String {
    match aim {
        0 => "aim straight".to_string(),
        a if a < 0 => format!("aim {} left", -a),
        a => format!("aim {a} right"),
    }
}

/// "1 pin", "7 pins".
pub fn pins_words(n: u8) -> String {
    if n == 1 {
        "1 pin".to_string()
    } else {
        format!("{n} pins")
    }
}

/// "1 point", "150 points".
pub fn points_words(n: u32) -> String {
    if n == 1 {
        "1 point".to_string()
    } else {
        format!("{n} points")
    }
}

/// How a release felt, in a few words.
pub fn release_words(error: f64) -> &'static str {
    let late = error > 0.0;
    match error.abs() {
        e if e <= CLEAN => "Clean release.",
        e if e <= 0.10 => {
            if late {
                "A little late."
            } else {
                "A little early."
            }
        }
        e if e <= 0.15 => {
            if late {
                "Late."
            } else {
                "Early."
            }
        }
        _ if late => "Very late.",
        _ => "Very early.",
    }
}

thread_local! {
    static FIXED_SEED: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
    static STEADY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// For tests of the screens: balls in play fall by the clean model, so a
/// clean release from the pocket is always a strike.
pub fn set_steady_lane(on: bool) {
    STEADY.with(|s| s.set(on));
}

fn steady_lane() -> bool {
    STEADY.with(|s| s.get())
}

/// Pins the seed new games get, for tests. `None` goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// A seed for a new game: the pinned one, or one from the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}
