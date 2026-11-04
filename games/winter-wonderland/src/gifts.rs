//! Gift Sort's rules: a shift on the workshop conveyor, driven by `dt`.
//!
//! A gift rides the belt towards its edge. The player shakes it to hear
//! what is inside, then sends it to the recipient whose wish makes that
//! sound, or behind them into the return sack when nobody wished for it.
//! A wrong match costs clock time; a gift that rides off the edge breaks
//! and costs time too. Sort the shift's quota before the clock runs out.

use parlor_core::Rng;

/// The shifts in a full Gift Sort run.
pub const SHIFTS: u32 = 8;

/// What a gift sounds like when shaken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Rattle,
    Squish,
    Clink,
    Jingle,
    Crinkle,
    Slosh,
    Tick,
    Squeak,
    Thump,
    Chime,
}

impl Kind {
    /// Every kind, in the order shifts bring them in.
    pub const ALL: [Kind; 10] = [
        Kind::Rattle,
        Kind::Squish,
        Kind::Clink,
        Kind::Jingle,
        Kind::Crinkle,
        Kind::Slosh,
        Kind::Tick,
        Kind::Squeak,
        Kind::Thump,
        Kind::Chime,
    ];

    /// "rattle", as a noun.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Rattle => "rattle",
            Kind::Squish => "squish",
            Kind::Clink => "clink",
            Kind::Jingle => "jingle",
            Kind::Crinkle => "crinkle",
            Kind::Slosh => "slosh",
            Kind::Tick => "tick",
            Kind::Squeak => "squeak",
            Kind::Thump => "thump",
            Kind::Chime => "chime",
        }
    }

    /// How the sound check describes the shake.
    pub fn describe(self) -> &'static str {
        match self {
            Kind::Rattle => "dry wooden knocks, like blocks tumbling in a box",
            Kind::Squish => "a soft, low, muffled squeeze",
            Kind::Clink => "short glassy pings",
            Kind::Jingle => "a shake of little bells",
            Kind::Crinkle => "a bright crackle of wrappers",
            Kind::Slosh => "water swishing with a bubble or two",
            Kind::Tick => "a steady, even clock tick",
            Kind::Squeak => "two rubbery squeaks",
            Kind::Thump => "two heavy thuds",
            Kind::Chime => "a little music-box tune",
        }
    }

    /// Things a recipient might wish for that make this sound.
    pub fn items(self) -> &'static [&'static str] {
        match self {
            Kind::Rattle => &["building blocks", "a bag of dice", "a wooden puzzle"],
            Kind::Squish => &["a teddy bear", "fluffy slippers", "a plush penguin"],
            Kind::Clink => &["teacups", "glass ornaments", "a set of mugs"],
            Kind::Jingle => &[
                "bells",
                "a bell for the bike",
                "jingle bell earrings",
            ],
            Kind::Crinkle => &[
                "chocolates in foil",
                "a bag of toffees",
                "candy canes in cellophane",
            ],
            Kind::Slosh => &["a snow globe", "bubble bath", "a bottle of maple syrup"],
            Kind::Tick => &["a pocket watch", "an alarm clock", "a kitchen timer"],
            Kind::Squeak => &["a rubber duck", "a squeaky dog toy", "squeaky boots"],
            Kind::Thump => &["a stack of books", "a heavy cookbook", "a bowling ball"],
            Kind::Chime => &["a music box", "a musical jewellery box", "a tiny carousel"],
        }
    }
}

/// Where a recipient stands, and the key that sends a gift there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spot {
    Left,
    Ahead,
    Right,
    /// The return sack behind the player, for gifts nobody wished for.
    Sack,
}

impl Spot {
    pub const PEOPLE: [Spot; 3] = [Spot::Left, Spot::Ahead, Spot::Right];

    /// "on your left", for sentences.
    pub fn place(self) -> &'static str {
        match self {
            Spot::Left => "on your left",
            Spot::Ahead => "straight ahead",
            Spot::Right => "on your right",
            Spot::Sack => "behind you",
        }
    }

    /// "Left", for the status line and the sighted helper.
    pub fn label(self) -> &'static str {
        match self {
            Spot::Left => "Left",
            Spot::Ahead => "Ahead",
            Spot::Right => "Right",
            Spot::Sack => "Sack",
        }
    }
}

/// Who might be on the list this year.
pub const NAMES: [&str; 6] = [
    "June",
    "Uncle Ned",
    "Little Pip",
    "Aunt Rosa",
    "Cousin Theo",
    "Mrs. Holly",
];

/// One person waiting for a gift.
#[derive(Clone, Debug, PartialEq)]
pub struct Recipient {
    pub name: &'static str,
    pub spot: Spot,
    pub wish: Kind,
    pub item: &'static str,
}

/// How one shift plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShiftSpec {
    pub level: u32,
    /// How many people are waiting (2 or 3).
    pub recipients: usize,
    /// How many gift sounds can turn up.
    pub pool: usize,
    /// Whether unwanted gifts turn up, for the return sack.
    pub sack: bool,
    /// Whether the briefing names each wish's sound.
    pub say_sounds: bool,
    /// Whether one recipient changes their wish half way through.
    pub mind_change: bool,
    /// Gifts to sort to finish the shift.
    pub quota: u32,
    /// Seconds on the shift clock.
    pub time: f64,
    /// Seconds the first gift takes to reach the edge.
    pub travel: f64,
    pub wrong_penalty: f64,
    pub fall_penalty: f64,
}

impl ShiftSpec {
    /// Shift `level`, 1 to [`SHIFTS`].
    pub fn new(level: u32) -> ShiftSpec {
        let l = level.clamp(1, SHIFTS);
        let quota = 4 + l;
        ShiftSpec {
            level: l,
            recipients: if l == 1 { 2 } else { 3 },
            pool: [2, 3, 4, 5, 6, 7, 8, 10][(l - 1) as usize],
            sack: l >= 3,
            say_sounds: l <= 3,
            mind_change: l >= 5,
            quota,
            time: 25.0 + quota as f64 * 7.5,
            travel: 12.0 - (l - 1) as f64 * 0.85,
            wrong_penalty: 5.0 + l as f64 * 0.5,
            fall_penalty: 4.0,
        }
    }
}

/// The gift on the belt.
#[derive(Clone, Debug, PartialEq)]
pub struct Gift {
    pub kind: Kind,
    /// Seconds ridden so far.
    pub ridden: f64,
    /// Seconds from arriving to the edge.
    pub travel: f64,
    pub shakes: u32,
    /// A hint was taken on it: half points.
    pub hinted: bool,
    pub warned: bool,
}

impl Gift {
    /// Seconds before it rides off the edge.
    pub fn left(&self) -> f64 {
        (self.travel - self.ridden).max(0.0)
    }
}

/// Something that happened on the belt, for the screen to voice.
#[derive(Clone, Debug, PartialEq)]
pub enum SortEvent {
    /// A new gift landed on the belt.
    Arrived,
    /// The gift is close to the edge.
    Warn,
    /// The gift rode off the edge and broke.
    Fell { kind: Kind, penalty: f64 },
    /// Sent to the right person (or the sack): points earned.
    Correct { spot: Spot, kind: Kind, points: u64 },
    /// Sent to the wrong place: the gift stays on the belt.
    Wrong { spot: Spot, penalty: f64 },
    /// The shift clock is running low: whole seconds left.
    ClockLow(u32),
    /// Recipient `who` now wishes for something else.
    MindChanged { who: usize },
    /// The quota is sorted.
    Cleared { bonus: u64 },
    /// The shift clock ran out.
    TimeUp,
}

/// What a press of a direction did, when it did not reach the rules.
#[derive(Clone, Debug, PartialEq)]
pub enum SendRefusal {
    /// Nothing on the belt to send.
    NoGift,
    /// Nobody stands there, or there is no sack this shift.
    NoOne,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftState {
    Running,
    Cleared,
    Failed,
}

/// One shift on the belt.
pub struct Shift {
    pub spec: ShiftSpec,
    pub recipients: Vec<Recipient>,
    pub gift: Option<Gift>,
    /// Seconds left on the shift clock.
    pub clock: f64,
    pub sorted: u32,
    pub broken: u32,
    pub wrongs: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub hints: u32,
    /// This shift's points, bonus included once cleared.
    pub score: u64,
    pub state: ShiftState,
    gap: f64,
    last_kind: Option<Kind>,
    changed: bool,
    low_said: u32,
    rng: Rng,
}

/// Seconds between one gift leaving and the next landing.
pub const GAP: f64 = 0.8;

impl Shift {
    pub fn new(level: u32, mut rng: Rng) -> Shift {
        let spec = ShiftSpec::new(level);
        let mut names = NAMES.to_vec();
        rng.shuffle(&mut names);
        let mut wishes: Vec<Kind> = Kind::ALL[..spec.pool].to_vec();
        rng.shuffle(&mut wishes);
        let spots: &[Spot] = if spec.recipients == 2 {
            &[Spot::Left, Spot::Right]
        } else {
            &Spot::PEOPLE
        };
        let recipients = spots
            .iter()
            .enumerate()
            .map(|(i, &spot)| {
                let wish = wishes[i];
                Recipient {
                    name: names[i],
                    spot,
                    wish,
                    item: rng.pick(wish.items()),
                }
            })
            .collect();
        Shift {
            spec,
            recipients,
            gift: None,
            clock: spec.time,
            sorted: 0,
            broken: 0,
            wrongs: 0,
            streak: 0,
            best_streak: 0,
            hints: 0,
            score: 0,
            state: ShiftState::Running,
            gap: 0.3,
            last_kind: None,
            changed: false,
            low_said: u32::MAX,
            rng,
        }
    }

    /// The recipient at `spot`, if anyone stands there.
    pub fn at(&self, spot: Spot) -> Option<&Recipient> {
        self.recipients.iter().find(|r| r.spot == spot)
    }

    /// Where a gift of `kind` belongs: the person who wished for it, else
    /// the sack.
    pub fn home_of(&self, kind: Kind) -> Spot {
        self.recipients
            .iter()
            .find(|r| r.wish == kind)
            .map(|r| r.spot)
            .unwrap_or(Spot::Sack)
    }

    /// Gift sounds nobody on the list wished for, that can still turn up.
    fn unwanted(&self) -> Vec<Kind> {
        Kind::ALL[..self.spec.pool]
            .iter()
            .copied()
            .filter(|k| self.recipients.iter().all(|r| r.wish != *k))
            .collect()
    }

    fn travel_now(&self) -> f64 {
        let ramp = (1.0 - 0.03 * self.sorted as f64).max(0.65);
        self.spec.travel * ramp
    }

    fn next_kind(&mut self) -> Kind {
        let unwanted = self.unwanted();
        let wanted: Vec<Kind> = self.recipients.iter().map(|r| r.wish).collect();
        let sack = self.spec.sack;
        let pick = |rng: &mut Rng| {
            if sack && !unwanted.is_empty() && rng.chance(0.3) {
                *rng.pick(&unwanted)
            } else {
                *rng.pick(&wanted)
            }
        };
        let mut kind = pick(&mut self.rng);
        if Some(kind) == self.last_kind {
            kind = pick(&mut self.rng);
        }
        self.last_kind = Some(kind);
        kind
    }

    /// Runs the belt and the clock for `dt` seconds.
    pub fn update(&mut self, dt: f64) -> Vec<SortEvent> {
        let mut out = Vec::new();
        if self.state != ShiftState::Running {
            return out;
        }
        self.clock -= dt;
        if self.tick_clock(&mut out) {
            return out;
        }
        match self.gift.as_mut() {
            None => {
                self.gap -= dt;
                if self.gap <= 0.0 {
                    let kind = self.next_kind();
                    let travel = self.travel_now();
                    self.gift = Some(Gift {
                        kind,
                        ridden: 0.0,
                        travel,
                        shakes: 0,
                        hinted: false,
                        warned: false,
                    });
                    out.push(SortEvent::Arrived);
                }
            }
            Some(gift) => {
                gift.ridden += dt;
                let warn_at = (gift.travel * 0.3).min(3.0);
                if !gift.warned && gift.left() <= warn_at {
                    gift.warned = true;
                    out.push(SortEvent::Warn);
                }
                if gift.ridden >= gift.travel {
                    let kind = gift.kind;
                    self.gift = None;
                    self.gap = GAP;
                    self.broken += 1;
                    self.streak = 0;
                    let penalty = self.spec.fall_penalty;
                    self.clock -= penalty;
                    out.push(SortEvent::Fell { kind, penalty });
                    self.tick_clock(&mut out);
                }
            }
        }
        out
    }

    /// Says when the clock is low and ends the shift at zero. True once
    /// the shift has failed.
    fn tick_clock(&mut self, out: &mut Vec<SortEvent>) -> bool {
        if self.clock <= 0.0 {
            self.clock = 0.0;
            self.state = ShiftState::Failed;
            self.gift = None;
            out.push(SortEvent::TimeUp);
            return true;
        }
        let whole = self.clock.ceil() as u32;
        // The first warning comes at 10 seconds, or as soon as a penalty
        // drops the clock below it; then a tick each second from 5.
        if whole <= 10 && whole < self.low_said && (self.low_said > 10 || whole <= 5) {
            self.low_said = whole;
            out.push(SortEvent::ClockLow(whole));
        }
        false
    }

    /// Shakes the gift: its kind, or `None` with an empty belt.
    pub fn shake(&mut self) -> Option<Kind> {
        let gift = self.gift.as_mut()?;
        gift.shakes += 1;
        Some(gift.kind)
    }

    /// Marks a hint taken on the gift on the belt (half points for it).
    pub fn take_hint(&mut self) {
        if let Some(gift) = self.gift.as_mut() {
            if !gift.hinted {
                gift.hinted = true;
                self.hints += 1;
            }
        }
    }

    /// Sends the gift on the belt to `spot`.
    pub fn send(&mut self, spot: Spot) -> Result<Vec<SortEvent>, SendRefusal> {
        if self.state != ShiftState::Running {
            return Err(SendRefusal::NoGift);
        }
        let open = match spot {
            Spot::Sack => self.spec.sack,
            _ => self.at(spot).is_some(),
        };
        if !open {
            return Err(SendRefusal::NoOne);
        }
        let Some(gift) = self.gift.clone() else {
            return Err(SendRefusal::NoGift);
        };
        let mut out = Vec::new();
        if self.home_of(gift.kind) != spot {
            self.wrongs += 1;
            self.streak = 0;
            let penalty = self.spec.wrong_penalty;
            self.clock -= penalty;
            out.push(SortEvent::Wrong { spot, penalty });
            self.tick_clock(&mut out);
            return Ok(out);
        }
        let speed = (100.0 * gift.left() / gift.travel).round() as u64;
        let mut points = 100 + speed + 10 * self.streak.min(10) as u64;
        if gift.hinted {
            points /= 2;
        }
        points *= self.spec.level as u64;
        self.score += points;
        self.sorted += 1;
        self.streak += 1;
        self.best_streak = self.best_streak.max(self.streak);
        self.gift = None;
        self.gap = GAP;
        out.push(SortEvent::Correct {
            spot,
            kind: gift.kind,
            points,
        });
        if self.sorted >= self.spec.quota {
            let bonus = (self.clock.floor() as u64) * 10 * self.spec.level as u64
                + 250 * self.spec.level as u64;
            self.score += bonus;
            self.state = ShiftState::Cleared;
            out.push(SortEvent::Cleared { bonus });
        } else if self.spec.mind_change && !self.changed && self.sorted == self.spec.quota / 2 {
            self.changed = true;
            let unwanted = self.unwanted();
            if !unwanted.is_empty() {
                let who = self.rng.below(self.recipients.len() as u32) as usize;
                let wish = *self.rng.pick(&unwanted);
                let item = *self.rng.pick(wish.items());
                let r = &mut self.recipients[who];
                r.wish = wish;
                r.item = item;
                out.push(SortEvent::MindChanged { who });
            }
        }
        Ok(out)
    }
}
