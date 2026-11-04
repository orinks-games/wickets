//! The rules of patience, as plain data: the deck, the deal, legal moves,
//! the stock, undo, hints and the stuck check. No sound or speech here
//! beyond the words for a card and a refused move.
//!
//! The table is Klondike: seven piles of 1 to 7 cards with the top card
//! face up, a stock of the other 24, and four foundations, one per suit,
//! built up from the ace. Piles build down in alternating colours, and
//! only a king goes on an empty pile. The stock turns one card at a time,
//! or three in draw three, as often as the player likes.

use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use hearken::text::sentence_start;
use parlor_core::Rng;

thread_local! {
    static FIXED_SEED: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Pins which deal of the pool a new game gets, for tests. `None` goes
/// back to the clock.
pub fn fix_deal(index: Option<usize>) {
    FIXED_SEED.with(|s| s.set(index));
}

/// The seed a new relaxed game deals from: a seed from the proved pool,
/// the pinned one or one picked by the clock.
pub fn run_seed() -> u64 {
    let pool = crate::pool::RELAXED;
    let index = FIXED_SEED
        .with(|s| s.get())
        .unwrap_or_else(|| Rng::from_time().below(pool.len() as u32) as usize);
    pool[index % pool.len()]
}

/// The number of tableau piles.
pub const PILES: usize = 7;
/// Cards in a deck.
pub const DECK: usize = 52;
/// The highest rank, the king.
pub const KING: u8 = 13;

/// The four suits, in the foundations' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades];

    /// The suit's place among the foundations, from 0.
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Suit {
        Suit::ALL[i % 4]
    }

    /// "hearts".
    pub fn name(self) -> &'static str {
        match self {
            Suit::Hearts => "hearts",
            Suit::Diamonds => "diamonds",
            Suit::Clubs => "clubs",
            Suit::Spades => "spades",
        }
    }

    pub fn red(self) -> bool {
        matches!(self, Suit::Hearts | Suit::Diamonds)
    }

    /// "red" or "black".
    pub fn colour(self) -> &'static str {
        if self.red() {
            "red"
        } else {
            "black"
        }
    }
}

/// One card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Card {
    /// 1 for the ace up to 13 for the king.
    pub rank: u8,
    pub suit: Suit,
}

/// A rank in words, as it reads mid-sentence: "ace", "7", "queen".
pub fn rank_words(rank: u8) -> String {
    match rank {
        1 => "ace".to_string(),
        11 => "jack".to_string(),
        12 => "queen".to_string(),
        13 => "king".to_string(),
        n => n.to_string(),
    }
}

impl Card {
    pub const fn new(rank: u8, suit: Suit) -> Card {
        Card { rank, suit }
    }

    pub fn red(self) -> bool {
        self.suit.red()
    }

    /// "6 of hearts", "ace of spades", as it reads mid-sentence.
    pub fn name(self) -> String {
        format!("{} of {}", rank_words(self.rank), self.suit.name())
    }

    /// "6 of hearts, red", as navigation reads a card.
    pub fn name_and_colour(self) -> String {
        format!(
            "{} of {}, {}",
            rank_words(self.rank),
            self.suit.name(),
            self.suit.colour()
        )
    }

    /// Whether this card may go on `under` in a pile: one lower, in the
    /// other colour.
    pub fn fits_on(self, under: Card) -> bool {
        self.rank + 1 == under.rank && self.red() != under.red()
    }
}

/// Every card once, hearts ace to king, then diamonds, clubs and spades.
pub fn full_deck() -> Vec<Card> {
    Suit::ALL
        .into_iter()
        .flat_map(|suit| (1..=KING).map(move |rank| Card::new(rank, suit)))
        .collect()
}

/// One tableau pile: its face-down cards, then its face-up run on top.
/// The last card of `up` is the top card.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pile {
    pub down: Vec<Card>,
    pub up: Vec<Card>,
}

impl Pile {
    pub fn top(&self) -> Option<Card> {
        self.up.last().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.down.is_empty() && self.up.is_empty()
    }
}

/// A place a card can be taken from or put on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spot {
    /// A tableau pile, from 0.
    Pile(usize),
    /// The turned-up cards beside the stock. Only the top one moves.
    Waste,
    /// A suit's foundation, by `Suit::index`.
    Foundation(usize),
}

/// Moving the top `count` cards of `from` onto `to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Spot,
    pub count: usize,
    pub to: Spot,
}

impl Move {
    pub const fn new(from: Spot, count: usize, to: Spot) -> Move {
        Move { from, count, to }
    }
}

/// Why a move cannot be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Nothing to take from there.
    Empty(Spot),
    /// More cards than are face up there.
    TooMany,
    /// Only one card moves to or from a foundation or the stock.
    OneAtATime,
    /// From a place onto itself.
    SameSpot,
    /// Only a king goes on an empty pile.
    KingOnly(Card),
    /// The card does not build on the pile's top card.
    Mismatch { card: Card, onto: Card },
    /// The card is not next on its foundation.
    NotNext { card: Card, next: u8 },
    /// A card goes on its own suit's foundation.
    WrongSuit { card: Card },
    /// The stock and the turned-up cards are both empty.
    StockEmpty,
    /// Every card is on the foundations.
    Won,
}

impl Refusal {
    /// The rule broken, in words.
    pub fn words(&self) -> String {
        match *self {
            Refusal::Empty(_) => "Nothing there to move.".to_string(),
            Refusal::TooMany => "Only face-up cards move.".to_string(),
            Refusal::OneAtATime => "Only one card at a time goes on a foundation.".to_string(),
            Refusal::SameSpot => "That is where the cards came from.".to_string(),
            Refusal::KingOnly(card) => format!(
                "{} cannot go on an empty pile. Only a king goes there.",
                sentence_start(&format!("the {}", card.name()))
            ),
            Refusal::Mismatch { card, onto } => {
                let lead = format!(
                    "{} cannot go on the {}.",
                    sentence_start(&format!("the {}", card.name())),
                    onto.name()
                );
                if card.rank == KING {
                    format!("{lead} A king goes only on an empty pile.")
                } else {
                    let need = if card.red() { "black" } else { "red" };
                    format!("{lead} It needs a {need} {}.", rank_words(card.rank + 1))
                }
            }
            Refusal::NotNext { card, next } => format!(
                "{} cannot go up yet. The {} foundation needs the {} next.",
                sentence_start(&format!("the {}", card.name())),
                card.suit.name(),
                rank_words(next)
            ),
            Refusal::WrongSuit { card } => format!(
                "{} goes on the {} foundation.",
                sentence_start(&format!("the {}", card.name())),
                card.suit.name()
            ),
            Refusal::StockEmpty => "The stock is empty, and nothing is turned up.".to_string(),
            Refusal::Won => "Every card is on the foundations.".to_string(),
        }
    }
}

/// Everything on the table.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Table {
    pub piles: Vec<Pile>,
    /// Face down; the last card is turned next.
    pub stock: Vec<Card>,
    /// Turned up beside the stock; the last card is the one that moves.
    pub waste: Vec<Card>,
    /// How far each suit's foundation has climbed: 0 for empty, 13 when
    /// complete. Indexed by `Suit::index`.
    pub foundations: [u8; 4],
}

impl Table {
    /// The Klondike layout from a deck: pile `n` gets `n + 1` cards, the
    /// last face up, and the rest form the stock. The deck's first card
    /// goes to pile 1.
    pub fn lay_out(deck: &[Card]) -> Table {
        assert_eq!(deck.len(), DECK, "a deal needs the whole deck");
        let mut cards = deck.iter().copied();
        let mut piles: Vec<Pile> = (0..PILES).map(|_| Pile::default()).collect();
        for row in 0..PILES {
            for (p, pile) in piles.iter_mut().enumerate().skip(row) {
                let card = cards.next().expect("enough cards");
                if p == row {
                    pile.up.push(card);
                } else {
                    pile.down.push(card);
                }
            }
        }
        let mut stock: Vec<Card> = cards.collect();
        stock.reverse();
        Table {
            piles,
            stock,
            waste: Vec::new(),
            foundations: [0; 4],
        }
    }

    /// A shuffled deal from `seed`.
    pub fn deal(seed: u64) -> Table {
        let mut deck = full_deck();
        Rng::seeded(seed).shuffle(&mut deck);
        Table::lay_out(&deck)
    }

    /// Every card on the table, wherever it is.
    pub fn all_cards(&self) -> Vec<Card> {
        let mut cards: Vec<Card> = Vec::new();
        for p in &self.piles {
            cards.extend(&p.down);
            cards.extend(&p.up);
        }
        cards.extend(&self.stock);
        cards.extend(&self.waste);
        for suit in Suit::ALL {
            cards.extend((1..=self.foundations[suit.index()]).map(|r| Card::new(r, suit)));
        }
        cards
    }

    /// The face-up cards at a spot, bottom first, top last.
    pub fn cards_at(&self, spot: Spot) -> Vec<Card> {
        match spot {
            Spot::Pile(i) => self.piles[i].up.clone(),
            Spot::Waste => self.waste.last().copied().into_iter().collect(),
            Spot::Foundation(i) => match self.foundations[i] {
                0 => Vec::new(),
                r => vec![Card::new(r, Suit::from_index(i))],
            },
        }
    }

    /// The top card at a spot.
    pub fn top(&self, spot: Spot) -> Option<Card> {
        self.cards_at(spot).last().copied()
    }

    pub fn face_down(&self) -> usize {
        self.piles.iter().map(|p| p.down.len()).sum()
    }

    pub fn on_foundations(&self) -> usize {
        self.foundations.iter().map(|&r| r as usize).sum()
    }

    pub fn won(&self) -> bool {
        self.foundations.iter().all(|&r| r == KING)
    }

    /// A short hash of the whole table, for the solver and the tests.
    pub fn hash64(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }

    /// Whether the move follows the rules.
    pub fn check(&self, mv: Move) -> Result<(), Refusal> {
        if self.won() {
            return Err(Refusal::Won);
        }
        if mv.from == mv.to {
            return Err(Refusal::SameSpot);
        }
        let from = self.cards_at(mv.from);
        if from.is_empty() || mv.count == 0 {
            return Err(Refusal::Empty(mv.from));
        }
        if !matches!(mv.from, Spot::Pile(_)) && mv.count > 1 {
            return Err(Refusal::OneAtATime);
        }
        if mv.count > from.len() {
            return Err(Refusal::TooMany);
        }
        let card = from[from.len() - mv.count];
        match mv.to {
            Spot::Waste => Err(Refusal::SameSpot),
            Spot::Foundation(i) => {
                if mv.count > 1 {
                    return Err(Refusal::OneAtATime);
                }
                if card.suit.index() != i {
                    return Err(Refusal::WrongSuit { card });
                }
                let next = self.foundations[i] + 1;
                if card.rank != next {
                    return Err(Refusal::NotNext { card, next });
                }
                Ok(())
            }
            Spot::Pile(p) => match self.piles[p].top() {
                None if card.rank == KING => Ok(()),
                None => Err(Refusal::KingOnly(card)),
                Some(onto) if card.fits_on(onto) => Ok(()),
                Some(onto) => Err(Refusal::Mismatch { card, onto }),
            },
        }
    }

    /// Makes a move the caller has checked. Returns what it did.
    pub(crate) fn apply(&mut self, mv: Move) -> Played {
        let onto = self.top(mv.to);
        let cards: Vec<Card> = match mv.from {
            Spot::Pile(i) => {
                let up = &mut self.piles[i].up;
                up.split_off(up.len() - mv.count)
            }
            Spot::Waste => vec![self.waste.pop().expect("checked")],
            Spot::Foundation(i) => {
                let r = self.foundations[i];
                self.foundations[i] -= 1;
                vec![Card::new(r, Suit::from_index(i))]
            }
        };
        let mut completed = None;
        match mv.to {
            Spot::Pile(p) => self.piles[p].up.extend(&cards),
            Spot::Foundation(i) => {
                self.foundations[i] += 1;
                if self.foundations[i] == KING {
                    completed = Some(Suit::from_index(i));
                }
            }
            Spot::Waste => unreachable!("checked"),
        }
        let mut flipped = None;
        if let Spot::Pile(i) = mv.from {
            let pile = &mut self.piles[i];
            if pile.up.is_empty() {
                if let Some(card) = pile.down.pop() {
                    pile.up.push(card);
                    flipped = Some(card);
                }
            }
        }
        Played {
            mv,
            cards,
            onto,
            flipped,
            completed,
            won: self.won(),
        }
    }

    /// Turns the stock: `draw` cards onto the turned-up pile, or, when the
    /// stock is empty, the turned-up cards back over as the new stock.
    pub(crate) fn turn(&mut self, draw: usize) -> Result<Turned, Refusal> {
        if self.stock.is_empty() {
            if self.waste.is_empty() {
                return Err(Refusal::StockEmpty);
            }
            let mut back = std::mem::take(&mut self.waste);
            back.reverse();
            let n = back.len();
            self.stock = back;
            return Ok(Turned::Refilled(n));
        }
        let mut drawn = Vec::new();
        for _ in 0..draw {
            match self.stock.pop() {
                Some(card) => {
                    self.waste.push(card);
                    drawn.push(card);
                }
                None => break,
            }
        }
        Ok(Turned::Drew(drawn))
    }
}

/// What a move did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Played {
    pub mv: Move,
    /// The cards moved, bottom first.
    pub cards: Vec<Card>,
    /// The card they went on, if any.
    pub onto: Option<Card>,
    /// A face-down card the move turned up.
    pub flipped: Option<Card>,
    /// A foundation the move completed.
    pub completed: Option<Suit>,
    pub won: bool,
}

/// What turning the stock did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Turned {
    /// These cards were turned up, the last one on top.
    Drew(Vec<Card>),
    /// The turned-up cards went back as the stock, this many.
    Refilled(usize),
}

/// How many tables the hint's search may visit before it falls back to a
/// plain useful move.
pub const HINT_LIMIT: usize = 1_000;

/// One player action, as the solver plans and the tests replay them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Turn,
    Move(Move),
}

/// What the hint suggests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    /// A move, and whether it turns up a face-down card.
    Move(Move),
    /// Turn the stock: a card further in can be played.
    Turn,
    /// Nothing helps: undo, or deal again.
    Stuck,
    Won,
}

/// A game of patience: the table, the draw, the moves made and the undo
/// history.
#[derive(Clone, Debug)]
pub struct Game {
    pub table: Table,
    /// Cards turned at a time: 1, or 3 in draw three.
    pub draw: usize,
    /// The deal's seed.
    pub seed: u64,
    /// Moves made: every card move and every turn of the stock. Undo
    /// takes one back.
    pub moves: u32,
    /// Turning the stock costs no move, as in the Rabbit rabbit deal.
    pub free_turns: bool,
    history: Vec<(Table, u32)>,
}

impl Game {
    /// A fresh deal from `seed`, turning `draw` cards at a time.
    pub fn deal(seed: u64, draw: usize) -> Game {
        Game::from_table(Table::deal(seed), draw, seed)
    }

    /// A game from a table set up by the caller.
    pub fn from_table(table: Table, draw: usize, seed: u64) -> Game {
        Game {
            table,
            draw: draw.max(1),
            seed,
            moves: 0,
            free_turns: false,
            history: Vec::new(),
        }
    }

    pub fn won(&self) -> bool {
        self.table.won()
    }

    pub fn check(&self, mv: Move) -> Result<(), Refusal> {
        self.table.check(mv)
    }

    /// Makes a move, if the rules allow it.
    pub fn play(&mut self, mv: Move) -> Result<Played, Refusal> {
        self.table.check(mv)?;
        self.history.push((self.table.clone(), self.moves));
        self.moves += 1;
        Ok(self.table.apply(mv))
    }

    /// Turns the stock.
    pub fn turn(&mut self) -> Result<Turned, Refusal> {
        if self.won() {
            return Err(Refusal::Won);
        }
        let before = self.table.clone();
        let turned = self.table.turn(self.draw)?;
        self.history.push((before, self.moves));
        if !self.free_turns {
            self.moves += 1;
        }
        Ok(turned)
    }

    /// Plays one step.
    pub fn step(&mut self, step: Step) -> Result<(), Refusal> {
        match step {
            Step::Turn => self.turn().map(|_| ()),
            Step::Move(mv) => self.play(mv).map(|_| ()),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Takes back the last move, face-down cards and all. Returns whether
    /// there was one.
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some((table, moves)) => {
                self.table = table;
                self.moves = moves;
                true
            }
            None => false,
        }
    }

    /// Every card is face up and the stock is used up, so the rest can go
    /// to the foundations on their own.
    pub fn can_finish(&self) -> bool {
        !self.won()
            && self.table.stock.is_empty()
            && self.table.waste.is_empty()
            && self.table.face_down() == 0
    }

    /// Plays every card to the foundations, lowest first, once
    /// `can_finish` holds. Each card counts as a move.
    pub fn finish(&mut self) -> Vec<Played> {
        let mut done = Vec::new();
        while self.can_finish() {
            let next = (0..PILES).find_map(|p| {
                let card = self.table.piles[p].top()?;
                let mv = Move::new(Spot::Pile(p), 1, Spot::Foundation(card.suit.index()));
                self.table.check(mv).is_ok().then_some(mv)
            });
            match next {
                Some(mv) => match self.play(mv) {
                    Ok(p) => done.push(p),
                    Err(_) => break,
                },
                None => break,
            }
        }
        done
    }

    /// Every legal move on the table now, foundation to pile included.
    pub fn legal_moves(&self) -> Vec<Move> {
        let t = &self.table;
        let mut out = Vec::new();
        let mut sources: Vec<(Spot, usize)> = Vec::new();
        for p in 0..PILES {
            for count in 1..=t.piles[p].up.len() {
                sources.push((Spot::Pile(p), count));
            }
        }
        if !t.waste.is_empty() {
            sources.push((Spot::Waste, 1));
        }
        for f in 0..4 {
            if t.foundations[f] > 0 {
                sources.push((Spot::Foundation(f), 1));
            }
        }
        for (from, count) in sources {
            let mut targets: Vec<Spot> = (0..PILES).map(Spot::Pile).collect();
            targets.extend((0..4).map(Spot::Foundation));
            for to in targets {
                let mv = Move::new(from, count, to);
                if t.check(mv).is_ok() {
                    out.push(mv);
                }
            }
        }
        out
    }

    /// Each card the stock can bring to the top of the turned-up pile
    /// within one full turn of it, with the turns it takes: 0 for the card
    /// on top now. A card shows once, at its first chance.
    pub fn stock_reach(&self) -> Vec<(usize, Card, Table)> {
        stock_reach(&self.table, self.draw)
    }

    /// Moves that make progress: they put a card up, turn a face-down card,
    /// free a card for a foundation, or bring the top turned-up card into
    /// play. Moves that only shuffle a run between piles are left out.
    pub fn useful_moves(&self) -> Vec<Move> {
        useful_moves(&self.table)
    }

    /// The next step of a winning line from here, when the solver finds
    /// one; else one useful move, the most helpful kind first. Following
    /// the hint never walks a winnable deal into a dead end.
    pub fn hint(&self) -> Hint {
        if self.won() {
            return Hint::Won;
        }
        match crate::solver::solve(&self.table, self.draw, HINT_LIMIT)
            .and_then(|steps| steps.first().copied())
        {
            Some(Step::Move(mv)) => Hint::Move(mv),
            Some(Step::Turn) => Hint::Turn,
            None => self.quick_hint(),
        }
    }

    /// One useful move, the most helpful kind first, with no look-ahead.
    fn quick_hint(&self) -> Hint {
        if self.won() {
            return Hint::Won;
        }
        let moves = self.useful_moves();
        if let Some(&mv) = moves.first() {
            return Hint::Move(mv);
        }
        let deeper = self
            .stock_reach()
            .into_iter()
            .any(|(turns, _, table)| turns > 0 && !useful_from_waste(&table).is_empty());
        if deeper {
            Hint::Turn
        } else {
            Hint::Stuck
        }
    }

    /// No useful move is left, now or anywhere in the stock.
    pub fn stuck(&self) -> bool {
        self.quick_hint() == Hint::Stuck
    }
}

/// Each card the stock can bring to the top of the turned-up pile within
/// one full turn of it, with the turns it takes and the table it leaves.
pub fn stock_reach(table: &Table, draw: usize) -> Vec<(usize, Card, Table)> {
    let mut out: Vec<(usize, Card, Table)> = Vec::new();
    let mut t = table.clone();
    let limit = t.stock.len() + t.waste.len() + 2;
    for turns in 0..=limit {
        if let Some(&card) = t.waste.last() {
            if !out.iter().any(|(_, c, _)| *c == card) {
                out.push((turns, card, t.clone()));
            }
        }
        if t.turn(draw).is_err() {
            break;
        }
        if t.stock == table.stock && t.waste == table.waste {
            break;
        }
    }
    out
}

/// A card whose foundation is far enough along that no pile could need it
/// to build on: an ace or 2, or a card whose two lower cards of the other
/// colour are already up.
pub fn safe_to_put_up(t: &Table, card: Card) -> bool {
    if t.foundations[card.suit.index()] + 1 != card.rank {
        return false;
    }
    if card.rank <= 2 {
        return true;
    }
    Suit::ALL
        .into_iter()
        .filter(|s| s.red() != card.red())
        .all(|s| t.foundations[s.index()] + 1 >= card.rank)
}

fn useful_from_waste(t: &Table) -> Vec<Move> {
    let mut out = Vec::new();
    if let Some(card) = t.waste.last() {
        let up = Move::new(Spot::Waste, 1, Spot::Foundation(card.suit.index()));
        if t.check(up).is_ok() {
            out.push(up);
        }
        for p in 0..PILES {
            let mv = Move::new(Spot::Waste, 1, Spot::Pile(p));
            if t.check(mv).is_ok() {
                out.push(mv);
            }
        }
    }
    out
}

/// The useful moves on a table, best first: cards to the foundations, runs
/// that turn a face-down card, the turned-up card into play, kings to an
/// empty pile from over face-down cards, and part runs that free a card
/// for its foundation.
pub fn useful_moves(t: &Table) -> Vec<Move> {
    let mut out = Vec::new();
    // Up to the foundations, piles first.
    for p in 0..PILES {
        if let Some(card) = t.piles[p].top() {
            let mv = Move::new(Spot::Pile(p), 1, Spot::Foundation(card.suit.index()));
            if t.check(mv).is_ok() {
                out.push(mv);
            }
        }
    }
    let waste = useful_from_waste(t);
    out.extend(
        waste
            .iter()
            .filter(|m| matches!(m.to, Spot::Foundation(_)))
            .copied(),
    );
    // Whole runs that uncover a face-down card.
    for p in 0..PILES {
        let pile = &t.piles[p];
        if pile.down.is_empty() || pile.up.is_empty() {
            continue;
        }
        for q in 0..PILES {
            let mv = Move::new(Spot::Pile(p), pile.up.len(), Spot::Pile(q));
            if q != p && t.check(mv).is_ok() {
                out.push(mv);
                break;
            }
        }
    }
    // The turned-up card onto a pile.
    out.extend(
        waste
            .iter()
            .filter(|m| matches!(m.to, Spot::Pile(_)))
            .copied(),
    );
    // Part runs that free a card for its foundation.
    for p in 0..PILES {
        let up = &t.piles[p].up;
        for count in 1..up.len() {
            let freed = up[up.len() - count - 1];
            if t.foundations[freed.suit.index()] + 1 != freed.rank {
                continue;
            }
            for q in 0..PILES {
                let mv = Move::new(Spot::Pile(p), count, Spot::Pile(q));
                if q != p && t.check(mv).is_ok() && t.piles[q].top().is_some() {
                    out.push(mv);
                    break;
                }
            }
        }
    }
    out
}

/// "first", "second"... up to "thirteenth", for a card's place from the
/// top of a pile.
pub fn ordinal(n: usize) -> &'static str {
    const WORDS: [&str; 13] = [
        "first",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "eighth",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
    ];
    WORDS[(n.max(1) - 1).min(12)]
}

/// "1 card", "3 cards".
pub fn cards_words(n: usize) -> String {
    if n == 1 {
        "1 card".to_string()
    } else {
        format!("{n} cards")
    }
}

/// "1 move", "34 moves".
pub fn moves_words(n: u32) -> String {
    if n == 1 {
        "1 move".to_string()
    } else {
        format!("{n} moves")
    }
}

/// Points for a won deal: a base, less one for each move.
pub const POINTS: u32 = 1000;

/// The score for winning in `moves`: never below 1.
pub fn points(moves: u32) -> u64 {
    POINTS.saturating_sub(moves).max(1) as u64
}
