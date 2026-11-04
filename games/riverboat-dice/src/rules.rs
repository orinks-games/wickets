//! The rules of Riverboat Dice, as plain structs with no screen or sound:
//! the dice, the three honest players at the table, a night of landings,
//! the careful player's advice, and the dealer that spaces a roll's sounds
//! and words out in time.
//!
//! A night is a number of landings. Each landing, everyone sits at the
//! table. The dealer rolls two dice and adds their sum to the table's
//! total for the landing, which every seated player stands to win. Before
//! the next roll, anyone may stand up and bank the table's total into
//! their night total. A 1 on either die ends the landing, and everyone
//! still seated gets nothing for it. With the snag rule on, a double 1
//! also wipes the night total of everyone still seated. At the full moon
//! table, doubles with no 1 double what the table holds.

use std::cell::Cell;
use std::collections::VecDeque;

use hearken::text::sentence_start;
use parlor_core::{FixedStep, Rng};

thread_local! {
    static FIXED_SEED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Pins every new night's seed, for tests. `None` goes back to the clock.
pub fn fix_seed(seed: Option<u64>) {
    FIXED_SEED.with(|s| s.set(seed));
}

/// The seed a new night uses: the pinned one, else the clock.
pub fn run_seed() -> u64 {
    FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().next_u64())
}

/// Landings in a saloon night.
pub const LANDINGS: u32 = 5;
/// Landings at the Captain's table.
pub const CAPTAIN_LANDINGS: u32 = 10;
/// Rolls out of 36 that show a 1.
pub const ONE_ROLLS: u32 = 11;
/// The average sum of a roll with no 1 in it.
pub const SAFE_AVERAGE: u32 = 8;
/// The Widow banks once the table holds this much.
pub const WIDOW_BANKS: u32 = 15;
/// The Gambler rides until the table holds this much.
pub const GAMBLER_BANKS: u32 = 40;
/// Rolls out of 36 that are doubles with no 1: 2 and 2 up to 6 and 6.
pub const MOON_DOUBLES: u32 = 5;
/// Those five doubles' sums, added: 4, 6, 8, 10 and 12.
pub const MOON_DOUBLES_SUM: u32 = 40;
/// The Captain banks when he leads by this much...
pub const CAPTAIN_LEAD: u32 = 20;
/// ...or once the landing has had this many rolls.
pub const CAPTAIN_ROLLS: u32 = 4;

/// Two dice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Roll {
    pub a: u8,
    pub b: u8,
}

impl Roll {
    pub const fn new(a: u8, b: u8) -> Roll {
        Roll { a, b }
    }

    pub fn sum(self) -> u32 {
        self.a as u32 + self.b as u32
    }

    /// Either die shows a 1: the landing is over.
    pub fn has_one(self) -> bool {
        self.a == 1 || self.b == 1
    }

    /// Both dice show a 1.
    pub fn double_one(self) -> bool {
        self.a == 1 && self.b == 1
    }

    /// Both dice show the same face, and it is not a 1.
    pub fn doubles(self) -> bool {
        self.a == self.b && !self.has_one()
    }

    /// The faces and sum, as the dealer calls them: "4 and 3, 7". A roll
    /// with a 1 has no sum worth calling: "1 and 5".
    pub fn words(self) -> String {
        if self.has_one() {
            format!("{} and {}", self.a, self.b)
        } else {
            format!("{} and {}, {}", self.a, self.b, self.sum())
        }
    }
}

/// Where the rolls come from: a seeded source, or a fixed list for tests.
#[derive(Clone, Debug)]
pub enum Dice {
    Seeded(Rng),
    /// These rolls in order, then a seeded source.
    Scripted(VecDeque<Roll>, Rng),
}

impl Dice {
    pub fn seeded(seed: u64) -> Dice {
        Dice::Seeded(Rng::seeded(seed))
    }

    pub fn scripted(rolls: impl IntoIterator<Item = Roll>) -> Dice {
        Dice::Scripted(rolls.into_iter().collect(), Rng::seeded(0))
    }

    pub fn roll(&mut self) -> Roll {
        fn die(rng: &mut Rng) -> u8 {
            rng.below(6) as u8 + 1
        }
        match self {
            Dice::Seeded(rng) => Roll::new(die(rng), die(rng)),
            Dice::Scripted(list, rng) => list
                .pop_front()
                .unwrap_or_else(|| Roll::new(die(rng), die(rng))),
        }
    }
}

/// The three passengers who play against you. Each follows one plain rule,
/// and none of them ever knows the next roll.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Opponent {
    Widow,
    Gambler,
    Captain,
}

impl Opponent {
    /// "the Widow", for the middle of a sentence.
    pub fn name(self) -> &'static str {
        match self {
            Opponent::Widow => "the Widow",
            Opponent::Gambler => "the Gambler",
            Opponent::Captain => "the Captain",
        }
    }

    /// "The Widow", for the start of one.
    pub fn title(self) -> String {
        sentence_start(self.name())
    }

    /// The rule, as a sentence: "She banks once the table holds 15."
    pub fn rule(self) -> String {
        match self {
            Opponent::Widow => format!("She banks once the table holds {WIDOW_BANKS}."),
            Opponent::Gambler => format!("He rides until the table holds {GAMBLER_BANKS}."),
            Opponent::Captain => {
                format!("He banks when he leads by {CAPTAIN_LEAD}, or after {CAPTAIN_ROLLS} rolls.")
            }
        }
    }
}

/// Who sits in a seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    You,
    Ai(Opponent),
}

impl Who {
    /// "you" or "the Widow".
    pub fn name(self) -> &'static str {
        match self {
            Who::You => "you",
            Who::Ai(o) => o.name(),
        }
    }
}

/// One seat at the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seat {
    pub who: Who,
    /// Banked over the night so far.
    pub night: u32,
    /// Still in the landing, standing to win the table's total.
    pub seated: bool,
    /// What this seat banked this landing, once it stands.
    pub banked: Option<u32>,
}

/// A kind of night.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    pub landings: u32,
    /// A double 1 wipes the night total of everyone still seated.
    pub snag: bool,
    /// Showboat Saturday: doubles with no 1 double what the table holds.
    pub moon: bool,
    /// Your opponents, in their seats from your left.
    pub field: &'static [Opponent],
}

/// The saloon: three passengers, five landings, no snag.
pub const SALOON: Rules = Rules {
    landings: LANDINGS,
    snag: false,
    moon: false,
    field: &[Opponent::Widow, Opponent::Gambler, Opponent::Captain],
};

/// The saloon with the snag rule on.
pub const SNAG_SALOON: Rules = Rules {
    snag: true,
    ..SALOON
};

/// Showboat Saturday: the saloon, where doubles double the table.
pub const MOON_TABLE: Rules = Rules {
    moon: true,
    ..SALOON
};

/// The Captain's table: the Captain alone, ten landings.
pub const CAPTAINS_TABLE: Rules = Rules {
    landings: CAPTAIN_LANDINGS,
    snag: false,
    moon: false,
    field: &[Opponent::Captain],
};

/// How a landing ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandingEnd {
    /// A 1 came up.
    One,
    /// A double 1 with the snag rule on.
    Snag,
    /// Nobody was left seated.
    AllBanked,
}

/// What a roll did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollReport {
    pub roll: Roll,
    /// The landing it was rolled in.
    pub landing: u32,
    /// The table's total after the roll, before anyone stood.
    pub pot: u32,
    /// Seats that stood up after the roll, and what each banked.
    pub stood: Vec<(usize, u32)>,
    /// Seats still seated when a 1 came, and what each lost.
    pub lost: Vec<(usize, u32)>,
    /// Seats whose night total the snag wiped, and what each had.
    pub wiped: Vec<(usize, u32)>,
    /// Doubles on Showboat Saturday doubled what the table holds.
    pub doubled: bool,
    pub end: Option<LandingEnd>,
}

/// What banking did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BankReport {
    pub amount: u32,
    /// Your night total after banking.
    pub night: u32,
    pub end: Option<LandingEnd>,
}

/// Why an action was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// You banked this landing already.
    Standing,
    /// The table holds nothing yet.
    NothingToBank,
    /// The night is over.
    Over,
}

/// One night at the table.
#[derive(Clone, Debug)]
pub struct Night {
    pub rules: Rules,
    /// Seat 0 is you; the rest follow `rules.field`.
    pub seats: Vec<Seat>,
    /// The landing being played, from 1.
    pub landing: u32,
    /// The table's total this landing.
    pub pot: u32,
    /// Rolls this landing that had no 1.
    pub rolls: u32,
    dice: Dice,
    /// Every roll of the night, in order.
    pub history: Vec<Roll>,
    pub last: Option<RollReport>,
    /// The most rolls anyone stayed seated through in one landing.
    pub longest_stay: u32,
    pub over: bool,
}

/// Your seat.
pub const YOU: usize = 0;

impl Night {
    pub fn new(rules: Rules, seed: u64) -> Night {
        Night::with_dice(rules, Dice::seeded(seed))
    }

    pub fn with_dice(rules: Rules, dice: Dice) -> Night {
        let mut seats = vec![Seat {
            who: Who::You,
            night: 0,
            seated: true,
            banked: None,
        }];
        seats.extend(rules.field.iter().map(|&o| Seat {
            who: Who::Ai(o),
            night: 0,
            seated: true,
            banked: None,
        }));
        Night {
            rules,
            seats,
            landing: 1,
            pot: 0,
            rolls: 0,
            dice,
            history: Vec::new(),
            last: None,
            longest_stay: 0,
            over: false,
        }
    }

    /// Your seat.
    pub fn you(&self) -> &Seat {
        &self.seats[YOU]
    }

    /// Whether anyone is still seated this landing.
    pub fn anyone_seated(&self) -> bool {
        self.seats.iter().any(|s| s.seated)
    }

    /// The seats still seated, in seat order.
    pub fn seated(&self) -> Vec<usize> {
        (0..self.seats.len())
            .filter(|&i| self.seats[i].seated)
            .collect()
    }

    /// Whether this is the night's last landing.
    pub fn last_landing(&self) -> bool {
        self.landing == self.rules.landings
    }

    /// What `seat` would have if it banked now.
    pub fn if_banked(&self, seat: usize) -> u32 {
        let s = &self.seats[seat];
        s.night + if s.seated { self.pot } else { 0 }
    }

    /// The best of everyone else's totals if they all banked now.
    pub fn best_other(&self, seat: usize) -> u32 {
        (0..self.seats.len())
            .filter(|&i| i != seat)
            .map(|i| self.if_banked(i))
            .max()
            .unwrap_or(0)
    }

    /// Whether the player in `seat` stands up now, by their rule.
    pub fn wants_to_bank(&self, seat: usize) -> bool {
        let Who::Ai(o) = self.seats[seat].who else {
            return false;
        };
        if !self.seats[seat].seated || self.pot == 0 {
            return false;
        }
        match o {
            Opponent::Widow => self.pot >= WIDOW_BANKS,
            Opponent::Gambler => self.pot >= GAMBLER_BANKS,
            Opponent::Captain => {
                self.rolls >= CAPTAIN_ROLLS
                    || self.if_banked(seat) >= self.best_other(seat) + CAPTAIN_LEAD
            }
        }
    }

    fn stand(&mut self, seat: usize) -> u32 {
        let pot = self.pot;
        let s = &mut self.seats[seat];
        s.seated = false;
        s.night += pot;
        s.banked = Some(pot);
        pot
    }

    /// Ends the landing and sets up the next, or ends the night.
    fn end_landing(&mut self) {
        if self.landing >= self.rules.landings {
            self.over = true;
            for s in &mut self.seats {
                s.seated = false;
            }
            return;
        }
        self.landing += 1;
        self.pot = 0;
        self.rolls = 0;
        for s in &mut self.seats {
            s.seated = true;
            s.banked = None;
        }
    }

    /// The dealer rolls for everyone still seated.
    pub fn roll(&mut self) -> Result<RollReport, Refusal> {
        if self.over {
            return Err(Refusal::Over);
        }
        let roll = self.dice.roll();
        self.history.push(roll);
        let landing = self.landing;
        let mut report = RollReport {
            roll,
            landing,
            pot: self.pot,
            stood: Vec::new(),
            lost: Vec::new(),
            wiped: Vec::new(),
            doubled: false,
            end: None,
        };
        if roll.has_one() {
            let snag = self.rules.snag && roll.double_one();
            for i in self.seated() {
                report.lost.push((i, self.pot));
                if snag {
                    report.wiped.push((i, self.seats[i].night));
                    self.seats[i].night = 0;
                }
            }
            report.end = Some(if snag {
                LandingEnd::Snag
            } else {
                LandingEnd::One
            });
            self.end_landing();
        } else {
            self.pot += roll.sum();
            if self.rules.moon && roll.doubles() {
                self.pot *= 2;
                report.doubled = true;
            }
            self.rolls += 1;
            self.longest_stay = self.longest_stay.max(self.rolls);
            report.pot = self.pot;
            // Everyone decides on the same table, before anyone stands.
            let standing: Vec<usize> = (0..self.seats.len())
                .filter(|&i| self.wants_to_bank(i))
                .collect();
            for i in standing {
                let amount = self.stand(i);
                report.stood.push((i, amount));
            }
            if !self.anyone_seated() {
                report.end = Some(LandingEnd::AllBanked);
                self.end_landing();
            }
        }
        self.last = Some(report.clone());
        Ok(report)
    }

    /// You stand up and bank the table's total.
    pub fn bank(&mut self) -> Result<BankReport, Refusal> {
        if self.over {
            return Err(Refusal::Over);
        }
        if !self.seats[YOU].seated {
            return Err(Refusal::Standing);
        }
        if self.pot == 0 {
            return Err(Refusal::NothingToBank);
        }
        let amount = self.stand(YOU);
        let night = self.seats[YOU].night;
        let end = (!self.anyone_seated()).then(|| {
            self.end_landing();
            LandingEnd::AllBanked
        });
        Ok(BankReport { amount, night, end })
    }

    /// The seats with the highest night total: more than one is a shared win.
    /// A night where nobody banked anything has no winner.
    pub fn winners(&self) -> Vec<usize> {
        let best = self.seats.iter().map(|s| s.night).max().unwrap_or(0);
        if best == 0 {
            return Vec::new();
        }
        (0..self.seats.len())
            .filter(|&i| self.seats[i].night == best)
            .collect()
    }

    /// Your place: 1 plus the seats strictly ahead of you.
    pub fn place(&self) -> usize {
        let mine = self.seats[YOU].night;
        1 + self.seats.iter().filter(|s| s.night > mine).count()
    }

    /// Everyone's night total, highest first, ties in seat order:
    /// "the Captain 41, you 30, the Widow 15, the Gambler 0".
    pub fn standings(&self) -> String {
        let mut order: Vec<usize> = (0..self.seats.len()).collect();
        order.sort_by(|&a, &b| self.seats[b].night.cmp(&self.seats[a].night));
        order
            .iter()
            .map(|&i| format!("{} {}", self.seats[i].who.name(), self.seats[i].night))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// "you and the Gambler".
    pub fn names(&self, seats: &[usize]) -> String {
        let names: Vec<&str> = seats.iter().map(|&i| self.seats[i].who.name()).collect();
        and_list(&names)
    }
}

/// "a", "a and b", "a, b and c".
pub fn and_list(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The odds, in words.
pub fn odds_words(rules: &Rules) -> String {
    let mut text = format!(
        "{ONE_ROLLS} of 36 rolls have a 1 and end the landing. The other 25 add {SAFE_AVERAGE} on average."
    );
    if rules.snag {
        text.push_str(" 1 of 36 is the snag, a double 1, which also wipes the night total.");
    }
    if rules.moon {
        text.push_str(&format!(
            " On the showboat, {MOON_DOUBLES} of 36 rolls are doubles with no 1, and they double what the table holds."
        ));
    }
    text
}

/// What a careful player would do now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Advice {
    /// The landing has not started: nothing is at risk.
    FirstRoll,
    Stay(String),
    Bank(String),
    /// You have banked; nothing to decide.
    Standing,
    Over,
}

/// The careful player's choice, with a short reason. It weighs what one
/// more roll adds on average against what a 1 would cost, and on the last
/// landing it rides on while banking would still leave you behind.
pub fn advice(night: &Night) -> Advice {
    if night.over {
        return Advice::Over;
    }
    if !night.you().seated {
        return Advice::Standing;
    }
    if night.pot == 0 {
        return Advice::FirstRoll;
    }
    let pot = night.pot;
    let mine = night.if_banked(YOU);
    if night.last_landing() {
        let others: Vec<usize> = (0..night.seats.len()).filter(|&i| i != YOU).collect();
        // What the others have already banked: a seated player's share of
        // the table is not theirs yet, since a 1 wipes it.
        let secured = others
            .iter()
            .map(|&i| night.seats[i].night)
            .max()
            .unwrap_or(0);
        let top: Vec<usize> = others
            .iter()
            .copied()
            .filter(|&i| night.seats[i].night == secured)
            .collect();
        let seated_top = top.iter().copied().find(|&i| night.seats[i].seated);
        let stood_top = top.iter().any(|&i| !night.seats[i].seated);
        if mine < secured && stood_top {
            let gap = secured - mine + 1;
            return Advice::Stay(format!(
                "This is the last landing, and banking {pot} still leaves you behind. You need {gap} more to lead."
            ));
        }
        if mine <= secured {
            if let Some(leader) = seated_top {
                // Every roll adds the same to you both, so the gap cannot
                // close until the leader banks.
                let name = night.seats[leader].who.name();
                return Advice::Stay(format!(
                    "This is the last landing, and {name} is still in, so banking cannot put you ahead. You gain only once {name} banks."
                ));
            }
            return Advice::Bank(format!(
                "This is the last landing, and banking {pot} shares the win. A roll could still take it alone."
            ));
        }
        if others.iter().all(|&i| !night.seats[i].seated) {
            return Advice::Bank(format!(
                "Nobody else is in, and banking {pot} wins the night."
            ));
        }
    }
    let at_risk = ONE_ROLLS * pot
        + if night.rules.snag {
            night.you().night
        } else {
            0
        };
    let gain = (36 - ONE_ROLLS) * SAFE_AVERAGE
        + if night.rules.moon {
            MOON_DOUBLES * pot + MOON_DOUBLES_SUM
        } else {
            0
        };
    if gain > at_risk && night.rules.moon {
        Advice::Stay(format!(
            "The table holds {pot}. Doubles would double it, so a roll is worth the risk until it holds 40."
        ))
    } else if gain > at_risk {
        Advice::Stay(format!(
            "The table holds {pot}. A roll adds {SAFE_AVERAGE} on average, and that is worth the risk."
        ))
    } else if night.rules.snag && night.you().night > 0 {
        Advice::Bank(format!(
            "The table holds {pot}, and the snag could also take your night total of {}.",
            night.you().night
        ))
    } else {
        Advice::Bank(format!(
            "The table holds {pot}. {ONE_ROLLS} of 36 rolls would lose it all, more than a roll adds on average."
        ))
    }
}

/// A sound the dealer's table makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sfx {
    Rattle,
    Land,
    Thud,
    Snag,
    Coin,
    Groan,
    Cheer,
    /// Showboat doubles: the table doubles.
    Moon,
}

/// A prompt the screen words with the player's own keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Prompt {
    /// Stay in or bank.
    Decide,
    /// A new landing: call its first roll.
    FirstRoll,
}

/// One step of what the table says and plays, in time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cue {
    Sound(Sfx),
    /// A chair scrape from a seat, as its player stands.
    Chair(usize),
    /// Words that cut off what is speaking.
    Say(String),
    /// Words that wait their turn.
    Queue(String),
    Prompt(Prompt),
    /// The night is over: the results follow.
    NightOver,
    /// The dealer rolls on his own, while you stand. Handled by `Table`.
    AutoRoll,
}

/// The dealer's clock, in steps a second.
pub const DEALER_HZ: f64 = 60.0;
/// From the cup's rattle to the dice landing.
pub const RATTLE: f64 = 0.5;
/// From the dice landing to the call.
pub const LAND: f64 = 0.15;
/// Between one stand and the next.
pub const STAND_GAP: f64 = 0.4;
/// Before the next landing is announced.
pub const LANDING_GAP: f64 = 1.0;
/// Before the dealer rolls on his own, while you stand.
pub const AUTO_GAP: f64 = 1.4;
/// A hurried roll, after a key skips the wait.
pub const SKIP_GAP: f64 = 0.2;

/// Spaces cues out in time, on a fixed step so it runs the same at any
/// frame rate.
#[derive(Clone, Debug)]
pub struct Dealer {
    clock: FixedStep,
    queue: Vec<(u64, Cue)>,
    tail: u64,
}

impl Default for Dealer {
    fn default() -> Self {
        Dealer::new()
    }
}

impl Dealer {
    pub fn new() -> Dealer {
        Dealer {
            clock: FixedStep::hz(DEALER_HZ),
            queue: Vec::new(),
            tail: 0,
        }
    }

    fn steps(secs: f64) -> u64 {
        (secs * DEALER_HZ).round() as u64
    }

    /// Whether any cue is still waiting.
    pub fn busy(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Adds `cue` `gap` seconds after the last one queued, or after now.
    pub fn after(&mut self, gap: f64, cue: Cue) {
        let now = self.clock.steps();
        let base = if self.queue.is_empty() {
            now
        } else {
            self.tail
        };
        self.tail = base + Self::steps(gap);
        self.queue.push((self.tail, cue));
    }

    /// The cues due now, in order, without advancing time.
    pub fn due(&mut self) -> Vec<Cue> {
        let now = self.clock.steps();
        let mut out = Vec::new();
        self.queue.retain(|(at, cue)| {
            if *at <= now {
                out.push(cue.clone());
                false
            } else {
                true
            }
        });
        out
    }

    /// Time passing.
    pub fn tick(&mut self, dt: f64) {
        self.clock.advance(dt, |_| {});
    }

    /// Drops every waiting cue and returns them.
    pub fn flush(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.queue)
            .into_iter()
            .map(|(_, c)| c)
            .collect()
    }

    /// Seconds since the dealer started.
    pub fn elapsed(&self) -> f64 {
        self.clock.steps() as f64 / DEALER_HZ
    }
}

/// A night at the table with the dealer's timing: what the play screen
/// drives. Every state change happens at once; the cues that tell of it
/// come out over time from `update`.
#[derive(Clone, Debug)]
pub struct Table {
    pub night: Night,
    dealer: Dealer,
    /// "The table rolls on without you" has been said this night.
    told_rolls_on: bool,
    /// The tutorial's table: it stops once the first landing is over.
    pub lesson: bool,
}

impl Table {
    pub fn new(night: Night) -> Table {
        Table {
            night,
            dealer: Dealer::new(),
            told_rolls_on: false,
            lesson: false,
        }
    }

    /// Whether cues are still on their way.
    pub fn busy(&self) -> bool {
        self.dealer.busy()
    }

    /// The night is over and told.
    pub fn finished(&self) -> bool {
        self.night.over && !self.dealer.busy()
    }

    /// Waiting on you: you are seated and the dealer is quiet.
    pub fn your_call(&self) -> bool {
        !self.night.over && self.night.you().seated && !self.dealer.busy()
    }

    /// You stay in: the dealer rolls.
    pub fn call_roll(&mut self) -> Result<RollReport, Refusal> {
        if self.night.over {
            return Err(Refusal::Over);
        }
        if !self.night.you().seated {
            return Err(Refusal::Standing);
        }
        let report = self.night.roll()?;
        self.tell_roll(&report, true);
        Ok(report)
    }

    /// You bank the table's total.
    pub fn bank(&mut self) -> Result<BankReport, Refusal> {
        let report = self.night.bank()?;
        self.dealer.after(0.0, Cue::Sound(Sfx::Coin));
        self.dealer.after(
            0.0,
            Cue::Say(format!(
                "You bank {}. Night total {}.",
                report.amount, report.night
            )),
        );
        match report.end {
            Some(_) => {
                self.dealer.after(
                    0.0,
                    Cue::Queue("Everyone has banked. The landing is over.".to_string()),
                );
                self.tell_next();
            }
            None => {
                if !self.told_rolls_on {
                    self.told_rolls_on = true;
                    self.dealer.after(
                        0.0,
                        Cue::Queue("The table rolls on without you.".to_string()),
                    );
                }
                self.dealer.after(AUTO_GAP, Cue::AutoRoll);
            }
        }
        Ok(report)
    }

    /// The words for a roll and its stands, the landing's end, and what
    /// comes next.
    fn tell_roll(&mut self, report: &RollReport, interrupt: bool) {
        let say = |t: String| {
            if interrupt {
                Cue::Say(t)
            } else {
                Cue::Queue(t)
            }
        };
        let night = &self.night;
        self.dealer.after(0.0, Cue::Sound(Sfx::Rattle));
        self.dealer.after(RATTLE, Cue::Sound(Sfx::Land));
        let mut cues: Vec<(f64, Cue)> = Vec::new();
        match report.end {
            Some(LandingEnd::One) => {
                cues.push((LAND, Cue::Sound(Sfx::Thud)));
                cues.push((0.0, Cue::Sound(Sfx::Groan)));
                cues.push((
                    0.0,
                    say(format!(
                        "{}. A one. The landing is over.",
                        report.roll.words()
                    )),
                ));
                let losers: Vec<usize> = report.lost.iter().map(|&(i, _)| i).collect();
                if !losers.is_empty() && report.pot > 0 {
                    cues.push((
                        0.0,
                        Cue::Queue(format!(
                            "{} lost {}.",
                            sentence_start(&night_names(night, &losers)),
                            report.pot
                        )),
                    ));
                }
            }
            Some(LandingEnd::Snag) => {
                cues.push((LAND, Cue::Sound(Sfx::Snag)));
                cues.push((0.0, Cue::Sound(Sfx::Groan)));
                cues.push((
                    0.0,
                    say("1 and 1, the snag. The landing is over.".to_string()),
                ));
                let wiped: Vec<String> = report
                    .wiped
                    .iter()
                    .map(|&(i, had)| format!("{} {had}", night.seats[i].who.name()))
                    .collect();
                if !wiped.is_empty() {
                    cues.push((
                        0.0,
                        Cue::Queue(format!(
                            "The snag wipes the night totals of {}.",
                            and_list(&wiped.iter().map(String::as_str).collect::<Vec<_>>())
                        )),
                    ));
                }
            }
            _ if report.doubled => {
                cues.push((LAND, Cue::Sound(Sfx::Moon)));
                cues.push((
                    0.0,
                    say(format!(
                        "{}. Showboat doubles. The table doubles to {}.",
                        report.roll.words(),
                        report.pot
                    )),
                ));
                self.tell_stands(report, &mut cues);
            }
            _ => {
                cues.push((
                    LAND,
                    say(format!(
                        "{}. The table holds {}.",
                        report.roll.words(),
                        report.pot
                    )),
                ));
                self.tell_stands(report, &mut cues);
            }
        }
        for (gap, cue) in cues {
            self.dealer.after(gap, cue);
        }
        if report.end.is_some() {
            self.tell_next();
        } else if self.night.you().seated {
            self.dealer.after(0.0, Cue::Prompt(Prompt::Decide));
        } else {
            self.dealer.after(AUTO_GAP, Cue::AutoRoll);
        }
    }

    /// Who stood after a roll, and whether that ended the landing.
    fn tell_stands(&self, report: &RollReport, cues: &mut Vec<(f64, Cue)>) {
        for &(i, amount) in &report.stood {
            cues.push((STAND_GAP, Cue::Chair(i)));
            cues.push((
                0.0,
                Cue::Queue(format!(
                    "{} stands with {amount}.",
                    sentence_start(self.night.seats[i].who.name())
                )),
            ));
        }
        if report.end == Some(LandingEnd::AllBanked) {
            cues.push((
                0.0,
                Cue::Queue("Everyone has banked. The landing is over.".to_string()),
            ));
        }
    }

    /// After a landing: the next one, or the end of the night.
    fn tell_next(&mut self) {
        if self.lesson {
            return;
        }
        if self.night.over {
            self.dealer.after(LANDING_GAP, Cue::NightOver);
            return;
        }
        let n = &self.night;
        self.dealer.after(
            LANDING_GAP,
            Cue::Queue(format!(
                "Landing {} of {}. You have {}.",
                n.landing,
                n.rules.landings,
                n.you().night
            )),
        );
        self.dealer.after(0.0, Cue::Prompt(Prompt::FirstRoll));
    }

    fn take(&mut self, cues: Vec<Cue>, out: &mut Vec<Cue>) -> bool {
        let mut rolled = false;
        for cue in cues {
            if cue == Cue::AutoRoll {
                if let Ok(report) = self.night.roll() {
                    self.tell_roll(&report, false);
                    rolled = true;
                }
            } else {
                out.push(cue);
            }
        }
        rolled
    }

    /// Time passing: the cues now due, in order. Rolls the dealer makes
    /// on his own, while you stand, happen in here.
    pub fn update(&mut self, dt: f64) -> Vec<Cue> {
        self.dealer.tick(dt);
        let mut out = Vec::new();
        loop {
            let due = self.dealer.due();
            if due.is_empty() || !self.take(due, &mut out) {
                break;
            }
        }
        out
    }

    /// A key while the dealer talks: every waiting word comes at once, and
    /// a roll the dealer would make on his own comes sooner.
    pub fn skip(&mut self) -> Vec<Cue> {
        let mut out = Vec::new();
        let mut auto = false;
        for cue in self.dealer.flush() {
            match cue {
                Cue::AutoRoll => auto = true,
                Cue::Sound(_) | Cue::Chair(_) => {}
                other => out.push(other),
            }
        }
        if auto {
            self.dealer.after(SKIP_GAP, Cue::AutoRoll);
        }
        out
    }

    /// The dealer's clock, for tests.
    pub fn elapsed(&self) -> f64 {
        self.dealer.elapsed()
    }
}

fn night_names(night: &Night, seats: &[usize]) -> String {
    night.names(seats)
}
