//! The tutorial: one whole deal, played on the real table, with a guide
//! that names each move as it comes up and teaches each rule the first
//! time it is needed.
//!
//! The guide is a small state machine ([`Phase`]) the table runs after
//! every key. It follows a plan, the winning line for the table as it
//! stands: the opening is written out so the rules come in a good order,
//! and the solver plans the rest. When the player makes a move of their
//! own, the guide plans again from there; when no plan can be found, it
//! says so and points to Backspace. Undo, the table reading and the hint
//! are taught by asking for them once each, after the first king.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::PlayScreen;
use crate::rules::{moves_words, Card, Game, Move, Pile, Spot, Step, Suit, Table};
use crate::ID;

fn card(rank: u8, suit: Suit) -> Card {
    Card { rank, suit }
}

fn pile(down: &[Card], up: Card) -> Pile {
    Pile {
        down: down.to_vec(),
        up: vec![up],
    }
}

/// The tutorial's deal: a full Klondike table, fixed and winnable. The
/// ace of hearts lies alone on pile 1, the ace of clubs under the 7 of
/// spades, and the king of spades under the 8 of diamonds, so the opening
/// teaches the foundations, building, the stock, a run and an empty pile
/// for a king. Face-down cards are listed bottom first, so the last one
/// turns up first; the stock's last card turns up first.
pub fn deal() -> Table {
    use Suit::*;
    let (h, d, c, s) = (Hearts, Diamonds, Clubs, Spades);
    Table {
        piles: vec![
            pile(&[], card(1, h)),
            pile(&[card(2, d)], card(9, h)),
            pile(&[card(10, c), card(1, c)], card(7, s)),
            pile(&[card(1, s), card(6, d), card(13, s)], card(8, d)),
            pile(
                &[card(5, c), card(4, d), card(10, d), card(2, c)],
                card(9, c),
            ),
            pile(
                &[card(8, s), card(9, s), card(12, s), card(4, h), card(2, s)],
                card(8, c),
            ),
            pile(
                &[
                    card(13, h),
                    card(9, d),
                    card(3, d),
                    card(4, c),
                    card(10, s),
                    card(5, d),
                ],
                card(11, d),
            ),
        ],
        stock: vec![
            card(8, h),
            card(5, s),
            card(6, s),
            card(12, c),
            card(5, h),
            card(12, h),
            card(11, s),
            card(13, c),
            card(1, d),
            card(3, c),
            card(4, s),
            card(6, c),
            card(7, d),
            card(11, c),
            card(6, h),
            card(12, d),
            card(7, h),
            card(13, d),
            card(11, h),
            card(10, h),
            card(7, c),
            card(3, s),
            card(3, h),
            card(2, h),
        ],
        waste: Vec::new(),
        foundations: [0; 4],
    }
}

/// The opening, written out: each rule in turn, up to the first king on
/// an empty pile. The solver plans the rest of the deal from there.
pub fn opening() -> Vec<Step> {
    use Spot::{Foundation, Pile, Waste};
    let hearts = Suit::Hearts.index();
    let clubs = Suit::Clubs.index();
    vec![
        Step::Move(Move::new(Pile(0), 1, Foundation(hearts))),
        Step::Move(Move::new(Pile(2), 1, Pile(3))),
        Step::Move(Move::new(Pile(2), 1, Foundation(clubs))),
        Step::Turn,
        Step::Move(Move::new(Waste, 1, Foundation(hearts))),
        Step::Move(Move::new(Pile(3), 2, Pile(4))),
        Step::Move(Move::new(Pile(3), 1, Pile(0))),
    ]
}

/// How many tables the guide's solver may visit when the player leaves
/// the plan. The deal is easy, so a winnable table is found well within.
const REPLAN_LIMIT: usize = 4_000;

/// The winning line from a table: each table, and the step played from
/// it. `tables` has one more entry than `steps`: the table it ends on.
#[derive(Clone, Debug)]
struct Plan {
    tables: Vec<Table>,
    steps: Vec<Step>,
}

impl Plan {
    /// The steps from `table`, played out to keep each table on the way.
    fn new(table: &Table, steps: Vec<Step>) -> Plan {
        let mut game = Game::from_table(table.clone(), 1, 0);
        let mut tables = vec![table.clone()];
        let mut kept = Vec::new();
        for step in steps {
            if game.step(step).is_err() {
                break;
            }
            tables.push(game.table.clone());
            kept.push(step);
        }
        Plan {
            tables,
            steps: kept,
        }
    }

    /// The solver's winning line from `table`, if it finds one.
    fn solve(table: &Table, limit: usize) -> Option<Plan> {
        crate::solver::solve(table, 1, limit).map(|steps| Plan::new(table, steps))
    }

    /// Where `table` is in the plan, if it is on it.
    fn find(&self, table: &Table) -> Option<usize> {
        self.tables.iter().position(|t| t == table)
    }
}

/// A rule the guide teaches the first time a move needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    /// A card up on a foundation, and how to pick up and place.
    Foundation,
    /// A card on a pile, one lower in the other colour.
    Build,
    /// A face-down card turning up, said once one has.
    Uncover,
    /// Turning the stock and taking its card.
    Stock,
    /// Several cards moved at once.
    Run,
    /// A king onto an empty pile.
    King,
    /// Turning the used-up stock back over.
    Refill,
}

/// A key the deal teaches by asking for it, once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    Undo,
    Table,
    Hint,
}

/// The keys the deal asks for, in order, after the first king.
const ASKS: [Ask; 3] = [Ask::Undo, Ask::Table, Ask::Hint];

/// Where the guide is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The deal has just been laid out: the first move is still to name.
    Opening,
    /// After each move, the guide names the next.
    Guiding,
    /// One of the keys the deal teaches: the guide waits for it.
    Asking(Ask),
    /// Every card is up.
    Won,
}

/// What the player just did at the table, as the guide hears it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Did {
    /// Space: a card picked up, placed, or put back.
    Place,
    /// The stock key.
    Turn,
    /// Backspace.
    Undo,
    /// The table reading.
    Table,
    /// The hint key.
    Hint,
}

/// The player's own keys, named as the lesson says them.
#[derive(Clone, Debug)]
pub struct Keys {
    pub up: String,
    pub pick: String,
    pub turn: String,
    pub table: String,
    pub hint: String,
}

impl Keys {
    pub fn of(ctx: &Ctx<Parlor>) -> Keys {
        let k = |a| ctx.control_name(a);
        Keys {
            up: k(ParlorAction::Up),
            pick: k(ParlorAction::Primary),
            turn: k(ParlorAction::Secondary),
            table: k(ParlorAction::Repeat),
            hint: k(ParlorAction::Hint),
        }
    }
}

/// The guide through the tutorial's deal. See the module docs.
#[derive(Clone, Debug)]
pub struct Guide {
    keys: Keys,
    phase: Phase,
    plan: Plan,
    /// Rules taught so far.
    taught: Vec<Topic>,
    /// Keys still to ask for.
    asks: Vec<Ask>,
    /// The table when the guide last looked.
    seen: Table,
    /// The last move named, so a run of turns is named once.
    named: Option<String>,
}

impl Guide {
    /// The guide for the tutorial's deal.
    pub fn new(keys: Keys) -> Guide {
        let table = deal();
        let mut plan = Plan::new(&table, opening());
        let last = plan.tables.last().cloned().unwrap_or_else(|| table.clone());
        if let Some(rest) = Plan::solve(&last, crate::rules::HINT_LIMIT) {
            plan.steps.extend(rest.steps);
            plan.tables.extend(rest.tables.into_iter().skip(1));
        }
        Guide {
            keys,
            phase: Phase::Opening,
            plan,
            taught: Vec::new(),
            asks: ASKS.to_vec(),
            seen: table,
            named: None,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The first frame: name the first move.
    pub fn begin(&mut self, game: &Game, ctx: &mut Ctx<Parlor>) {
        if self.phase != Phase::Opening {
            return;
        }
        self.phase = Phase::Guiding;
        self.seen = game.table.clone();
        if let Some(line) = self.next_move(&game.table, true) {
            ctx.say_queued(line);
        }
    }

    /// After a key at the table: move the guide on and say what comes next.
    pub fn after(&mut self, did: Did, game: &Game, ctx: &mut Ctx<Parlor>) {
        if matches!(self.phase, Phase::Opening | Phase::Won) {
            return;
        }
        if game.won() {
            self.phase = Phase::Won;
            return;
        }
        let changed = game.table != self.seen;
        let before = std::mem::replace(&mut self.seen, game.table.clone());
        let line = match self.phase {
            Phase::Asking(ask) if asked_by(did) == Some(ask) => self.answered(ask, game),
            Phase::Asking(ask) => {
                // Another key: the asked key is still to press. After a
                // move, the plan follows the table and the ask comes again.
                self.used(did);
                if changed {
                    self.follow(&game.table);
                    Some(self.ask_line(ask))
                } else {
                    None
                }
            }
            Phase::Guiding => {
                self.used(did);
                if !changed {
                    return;
                }
                let flipped = game.table.face_down() < before.face_down();
                let lead = (flipped && self.learn(Topic::Uncover))
                    .then(|| self.topic_words(Topic::Uncover));
                let moved = matches!(did, Did::Place | Did::Turn);
                // A step along the plan: a move already named, such as one
                // of a run of turns, is not named again.
                let along = self.plan.find(&before).zip(self.plan.find(&game.table));
                let along = along.is_some_and(|(was, now)| now == was + 1);
                match self.asks.first() {
                    // After the first king, one key a move.
                    Some(&ask) if moved && self.taught.contains(&Topic::King) => {
                        self.follow(&game.table);
                        self.phase = Phase::Asking(ask);
                        join(lead, Some(self.ask_line(ask)))
                    }
                    _ => join(lead, self.next_move(&game.table, along)),
                }
            }
            Phase::Opening | Phase::Won => None,
        };
        say(ctx, line);
    }

    /// Whether every rule of the opening and every key has been taught.
    pub fn taught_all(&self) -> bool {
        self.asks.is_empty() && self.taught.contains(&Topic::King)
    }

    /// A key the deal would ask for, pressed before it was asked: there
    /// is no need to ask for it.
    fn used(&mut self, did: Did) {
        if let Some(ask) = asked_by(did) {
            self.asks.retain(|a| *a != ask);
        }
    }

    /// The asked key was pressed: back to guiding. The hint has named the
    /// next move itself; after the others, the guide names it.
    fn answered(&mut self, ask: Ask, game: &Game) -> Option<String> {
        self.asks.retain(|a| *a != ask);
        self.phase = Phase::Guiding;
        let next = match ask {
            Ask::Hint => {
                self.named = self.move_key(&game.table);
                None
            }
            Ask::Undo | Ask::Table => self.next_move(&game.table, false),
        };
        let done = self.asks.is_empty().then(|| {
            "That is every key. Finish the deal. F1 repeats the next move, and Escape leaves the tutorial.".to_string()
        });
        join(done, next)
    }

    fn ask_line(&self, ask: Ask) -> String {
        match ask {
            Ask::Undo => {
                "Backspace undoes your last move, as often as you like. Press it now.".to_string()
            }
            Ask::Table => format!(
                "{} reads each place's top card. Press it now.",
                sentence_start(&self.keys.table)
            ),
            Ask::Hint => format!(
                "{} gives a hint whenever you need one. Press it now.",
                sentence_start(&self.keys.hint)
            ),
        }
    }

    /// Keeps the plan on the table: where the table is on it, or a new
    /// plan from it. Returns whether the deal can still be won.
    fn follow(&mut self, table: &Table) -> bool {
        if self.plan.find(table).is_some() {
            return true;
        }
        match Plan::solve(table, REPLAN_LIMIT) {
            Some(plan) => {
                self.plan = plan;
                true
            }
            None => false,
        }
    }

    /// Teaches `topic` if it is new. Returns whether it was.
    fn learn(&mut self, topic: Topic) -> bool {
        if self.taught.contains(&topic) {
            return false;
        }
        self.taught.push(topic);
        true
    }

    /// The next move, with the rule it needs the first time. `fresh`
    /// keeps a move already named from being named again, as in a run of
    /// turns of the stock; otherwise it is said whatever came before.
    fn next_move(&mut self, table: &Table, fresh: bool) -> Option<String> {
        if !self.follow(table) {
            self.named = None;
            return Some("You cannot win from here. Backspace undoes a move.".to_string());
        }
        let words = self.move_words(table)?;
        let key = self.move_key(table);
        if fresh && self.named.is_some() && self.named == key {
            return None;
        }
        self.named = key;
        let at = self.plan.find(table)?;
        let lead = self
            .topic_of(table, self.plan.steps[at])
            .and_then(|topic| self.learn(topic).then(|| self.topic_words(topic)));
        join(lead, Some(words))
    }

    /// The rule a step needs.
    fn topic_of(&self, table: &Table, step: Step) -> Option<Topic> {
        match step {
            Step::Turn if table.stock.is_empty() => Some(Topic::Refill),
            Step::Turn => Some(Topic::Stock),
            Step::Move(mv) => match mv.to {
                _ if mv.count > 1 => Some(Topic::Run),
                Spot::Foundation(_) => Some(Topic::Foundation),
                Spot::Pile(q) if table.piles[q].is_empty() => Some(Topic::King),
                Spot::Pile(_) if mv.from == Spot::Waste => Some(Topic::Stock),
                Spot::Pile(_) => Some(Topic::Build),
                Spot::Waste => None,
            },
        }
    }

    fn topic_words(&self, topic: Topic) -> String {
        let k = &self.keys;
        match topic {
            Topic::Foundation => format!(
                "Each foundation takes one suit, from ace to king. {} picks up a card. {} again places it.",
                sentence_start(&k.pick),
                sentence_start(&k.pick)
            ),
            Topic::Build => {
                "Build piles down by rank, alternating red and black. A black 7 goes on a red 8."
                    .to_string()
            }
            Topic::Uncover => {
                "A face-down card turns up when you move all cards off it.".to_string()
            }
            Topic::Stock => format!(
                "{} turns up the next stock card. {} picks it up at the stock.",
                sentence_start(&k.turn),
                sentence_start(&k.pick)
            ),
            Topic::Run => format!(
                "{} moves deeper into a pile's face-up cards. {} picks up the chosen card and all cards on it.",
                sentence_start(&k.up),
                sentence_start(&k.pick)
            ),
            Topic::King => "Only a king goes on an empty pile.".to_string(),
            Topic::Refill => format!(
                "The stock is used up. {} turns it back over, as often as you like.",
                sentence_start(&k.turn)
            ),
        }
    }

    /// Where a run of turns of the stock from plan position `at` ends,
    /// before any turn that turns the stock back over.
    fn turns_end(&self, at: usize) -> usize {
        let mut end = at;
        while self.plan.steps.get(end) == Some(&Step::Turn)
            && !self.plan.tables[end].stock.is_empty()
        {
            end += 1;
        }
        end
    }

    /// What names the next move, so it is named only once: a run of turns
    /// is one move, toward the card it turns up.
    fn move_key(&self, table: &Table) -> Option<String> {
        let at = self.plan.find(table)?;
        let step = *self.plan.steps.get(at)?;
        if step == Step::Turn && !table.stock.is_empty() {
            let end = self.turns_end(at);
            let card = self.plan.tables[end].waste.last().map(|c| c.name());
            return Some(format!("turn to {}", card.unwrap_or_default()));
        }
        self.move_words(table)
    }

    /// The plan's next move from `table`, in words, with a run of turns
    /// named once by the card it is for.
    fn move_words(&self, table: &Table) -> Option<String> {
        let at = self.plan.find(table)?;
        let step = *self.plan.steps.get(at)?;
        Some(match step {
            Step::Turn if table.stock.is_empty() => "Turn the stock back over.".to_string(),
            Step::Turn => {
                // The turns in a row, up to the card they turn up.
                let end = self.turns_end(at);
                let turns = end - at;
                let refill_next = self.plan.steps.get(end) == Some(&Step::Turn);
                match self.plan.tables[end].waste.last() {
                    _ if turns == 1 => "Turn the stock.".to_string(),
                    _ if refill_next => "Turn through the rest of the stock.".to_string(),
                    Some(c) => format!("Turn the stock until the {} turns up.", c.name()),
                    None => "Turn the stock.".to_string(),
                }
            }
            Step::Move(mv) => move_words(table, mv),
        })
    }

    /// The plan's next step from the table, for a test or an agent that
    /// follows the guide.
    pub fn next_step(&self, table: &Table) -> Option<Step> {
        let at = self.plan.find(table)?;
        self.plan.steps.get(at).copied()
    }

    /// What to do now, for F1 and the hint: the key asked for, or the
    /// next move.
    pub fn now(&self, game: &Game) -> String {
        match self.phase {
            Phase::Asking(ask) => self.ask_line(ask),
            Phase::Won => "Every card is on the foundations.".to_string(),
            _ => self.move_words(&game.table).unwrap_or_else(|| {
                "You cannot win from here. Backspace undoes a move.".to_string()
            }),
        }
    }

    /// The hint key: the next move.
    pub fn hint(&self, game: &Game) -> String {
        match self.move_words(&game.table) {
            Some(words) => words,
            None if game.won() => "Every card is on the foundations.".to_string(),
            None => "You cannot win from here. Backspace undoes a move.".to_string(),
        }
    }
}

/// The ask a key answers, if any.
fn asked_by(did: Did) -> Option<Ask> {
    match did {
        Did::Undo => Some(Ask::Undo),
        Did::Table => Some(Ask::Table),
        Did::Hint => Some(Ask::Hint),
        Did::Place | Did::Turn => None,
    }
}

/// A move from the plan, as the guide names it.
fn move_words(table: &Table, mv: Move) -> String {
    let cards = table.cards_at(mv.from);
    let card = cards[cards.len() - mv.count];
    let what = match mv.count {
        1 => format!("the {}", card.name()),
        2 => format!("the {} and the card on it", card.name()),
        _ => format!("the {} and the cards on it", card.name()),
    };
    let from = match mv.from {
        Spot::Pile(p) => format!("pile {}", p + 1),
        Spot::Waste => "the stock".to_string(),
        Spot::Foundation(_) => "its foundation".to_string(),
    };
    match mv.to {
        Spot::Foundation(_) => format!("Move {what} from {from} to its foundation."),
        Spot::Pile(q) => match table.piles[q].top() {
            Some(onto) => format!(
                "Move {what} from {from} onto the {} in pile {}.",
                onto.name(),
                q + 1
            ),
            None => format!("Move {what} from {from} to empty pile {}.", q + 1),
        },
        Spot::Waste => String::new(),
    }
}

fn join(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(format!("{a} {b}")),
        (a, b) => a.or(b),
    }
}

fn say(ctx: &mut Ctx<Parlor>, line: Option<String>) {
    if let Some(line) = line {
        ctx.say_queued(line);
    }
}

/// The tutorial's deal on the real play screen, with its guide.
pub fn screen(ctx: &Ctx<Parlor>) -> Live<PlayScreen> {
    let game = Game::from_table(deal(), 1, 0);
    Live::new(
        PlayScreen::practice(game)
            .in_lesson()
            .guided(Guide::new(Keys::of(ctx))),
    )
}

/// Seconds a line takes to say, near enough.
fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// Waits for what the game says about the move that finished a step, so
/// the lesson's next words do not cut it off. Only that move's lines are
/// waited for: a key pressed after it has already cut them off, so the
/// next step comes at once, and the player's presses never hold it back.
#[derive(Default)]
struct Heard {
    /// `ctx.said` when last looked; none before the step's first frame.
    said: Option<u64>,
    /// When the finishing move's lines should be done, once it is made.
    until: Option<f64>,
}

impl Heard {
    /// Whether the step can end: `done` holds, and what the move that made
    /// it hold said has been heard or cut off by a later key.
    fn after(&mut self, done: bool, ctx: &Ctx<Parlor>) -> bool {
        if !done {
            self.said = Some(ctx.said);
            self.until = None;
            return false;
        }
        match self.until {
            None => {
                // The frame of the move: its lines start now. A step that
                // begins already done waits for its own prompt instead.
                let since = self.said.unwrap_or(ctx.said.saturating_sub(1));
                let new = ((ctx.said - since) as usize).min(ctx.transcript.len());
                let lines = &ctx.transcript[ctx.transcript.len() - new..];
                let until = lines
                    .iter()
                    .filter(|l| !l.starts_with("[event]"))
                    .fold(ctx.clock, |t, line| t + speech_secs(line));
                self.said = Some(ctx.said);
                self.until = Some(until);
                ctx.clock >= until
            }
            Some(until) => self.said.is_some_and(|s| ctx.said > s) || ctx.clock >= until,
        }
    }
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let screen = screen(ctx);
    lesson_on(ctx, screen)
}

/// The tutorial on `screen`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, screen: Live<PlayScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r) = (k(ParlorAction::Left), k(ParlorAction::Right));

    let mut heard = Heard::default();
    let mut marked = false;
    let watch = screen.clone();
    let deal = screen
        .step(
            format!(
                "The row has 7 piles, the stock, then 4 foundations. {} and {r} move between them.",
                sentence_start(&l)
            ),
            move |s: &PlayScreen, ctx| {
                if !marked && s.guide().is_some_and(Guide::taught_all) {
                    // Every key taught: leaving now still counts.
                    marked = true;
                    parlor_core::tutorial::mark_tutorial_done(ctx, ID);
                }
                heard
                    .after(s.game.won(), ctx)
                    .then(|| format!("Deal won in {}.", moves_words(s.game.moves)))
            },
        )
        .now(move |_ctx| {
            watch.with(|s| match s.guide() {
                Some(g) => g.now(&s.game),
                None => String::new(),
            })
        });

    Lesson::new(
        ID,
        "Fireside Patience",
        "A game of patience by the fire: put all 52 cards on the four foundations by suit, ace to king. There is no clock, and any move can be taken back. Learn by playing a whole deal",
    )
    .step(deal)
    .outro("Play deals a new game.")
}
