//! The card table: moving along the piles, picking up and placing cards,
//! turning the stock, undo, and the end of a deal.

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Date, Pages, Parlor, ParlorAction, Period, Records, ScoreResult};

use crate::challenge::{self, Job, CHALLENGES_NAME};
use crate::events::{self, EventDeal};
use crate::progress::{self, Saved};
use crate::rules::{
    cards_words, moves_words, ordinal, points, rank_words, run_seed, Card, Game, Hint, Move,
    Played, Spot, Suit, Turned, DECK, KING, PILES,
};
use crate::{sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const PLAY_NAME: &str = "fireside_patience_table";

/// The game's own key: Backspace takes back the last move.
pub const UNDO_KEY: Key = Key::Backspace;

/// The places along the row: the seven piles, the stock, then the four
/// foundations.
pub const PLACES: usize = PILES + 1 + 4;
/// The stock's place in the row.
pub const STOCK_PLACE: usize = PILES;

/// The spot a place in the row stands for. The stock's place holds the
/// turned-up card.
pub fn spot_of(place: usize) -> Spot {
    match place {
        p if p < PILES => Spot::Pile(p),
        STOCK_PLACE => Spot::Waste,
        p => Spot::Foundation(p - PILES - 1),
    }
}

/// The place in the row of a spot.
pub fn place_of(spot: Spot) -> usize {
    match spot {
        Spot::Pile(p) => p,
        Spot::Waste => STOCK_PLACE,
        Spot::Foundation(f) => PILES + 1 + f,
    }
}

/// "the 6 of hearts", as it reads mid-sentence.
fn the(card: Card) -> String {
    format!("the {}", card.name())
}

/// "Hearts foundation".
fn foundation_name(f: usize) -> String {
    format!("{} foundation", sentence_start(Suit::from_index(f).name()))
}

/// A place's name: "Pile 3", "Stock", "Hearts foundation".
pub fn place_name(place: usize) -> String {
    match spot_of(place) {
        Spot::Pile(p) => format!("Pile {}", p + 1),
        Spot::Waste => "Stock".to_string(),
        Spot::Foundation(f) => foundation_name(f),
    }
}

/// "1 card to turn" or "none left to turn".
fn to_turn(n: usize) -> String {
    match n {
        0 => "none left to turn".to_string(),
        n => format!("{} to turn", cards_words(n)),
    }
}

pub struct PlayScreen {
    pub game: Game,
    /// A challenge, deal after deal, and the challenge it counts for.
    job: Option<Job>,
    challenge: Option<Challenge>,
    /// The monthly event's deal.
    pub event: Option<EventDeal>,
    /// The place in the row, from 0.
    pub cursor: usize,
    /// How far into the pile's face-up cards, from 0 for the top card.
    pub depth: usize,
    /// The cards picked up: where from, and how many.
    pub held: Option<(Spot, usize)>,
    /// The results are on their way: the deal is won.
    ended: bool,
    told_row: bool,
    told_hold: bool,
    told_stuck: bool,
    /// Cards on the foundations when the dead end was told: only more
    /// than that makes a new table.
    stuck_up: usize,
    /// Whether the deal counts in the records. The tutorial's table does not.
    keep: bool,
    /// Inside the tutorial: the lesson's prompts teach, so the table keeps
    /// its intro, controls and stuck notice to itself.
    lesson: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl PlayScreen {
    /// A relaxed deal from the proved pool.
    pub fn new() -> PlayScreen {
        PlayScreen::with_game(Game::deal(run_seed(), 1))
    }

    /// A game set up by the caller.
    pub fn with_game(game: Game) -> PlayScreen {
        PlayScreen {
            status_key: Default::default(),
            game,
            job: None,
            challenge: None,
            event: None,
            cursor: 0,
            depth: 0,
            held: None,
            ended: false,
            told_row: false,
            told_hold: false,
            told_stuck: false,
            stuck_up: 0,
            keep: true,
            lesson: false,
        }
    }

    /// A table set up for the tutorial, kept out of the records.
    pub fn practice(game: Game) -> PlayScreen {
        PlayScreen {
            status_key: Default::default(),
            keep: false,
            ..PlayScreen::with_game(game)
        }
    }

    /// The table inside the tutorial.
    pub fn in_lesson(mut self) -> PlayScreen {
        self.lesson = true;
        self
    }

    /// The challenge's deals, from its seed.
    pub fn challenge(ch: Challenge) -> PlayScreen {
        let job = Job::new(&ch);
        let mut screen = PlayScreen::with_game(job.game());
        screen.job = Some(job);
        screen.challenge = Some(ch);
        screen
    }

    pub fn is_challenge(&self) -> bool {
        self.job.is_some()
    }

    /// An event's deal.
    pub fn event(deal: EventDeal, date: Date) -> PlayScreen {
        let seed = events::rabbit_seed(date);
        let mut game = Game::deal(seed, 1);
        game.free_turns = deal == EventDeal::Rabbit;
        let mut screen = PlayScreen::with_game(game);
        screen.event = Some(deal);
        screen
    }

    fn lucky(&self) -> bool {
        self.event == Some(EventDeal::Rabbit)
    }

    /// The fire, or the Rabbit rabbit evening.
    fn bed(&self, ctx: &mut Ctx<Parlor>) {
        if self.lucky() {
            sound::moonlit(ctx);
        } else {
            sound::fireside(ctx);
        }
    }

    /// The challenge's period, if this is one.
    pub fn period(&self) -> Option<Period> {
        self.job.as_ref().map(|j| j.period)
    }

    fn intro(&self) -> String {
        let what = match &self.job {
            Some(job) if job.deals() > 1 => format!(
                "{}. {}",
                challenge::name(job.period),
                challenge::next_words(job)
            ),
            Some(job) => format!("{}.", challenge::name(job.period)),
            None => match self.event {
                Some(EventDeal::Rabbit) => format!(
                    "{}. The fire burns low and an owl calls outside. Turning the stock costs no moves today.",
                    events::RABBIT_NAME
                ),
                None => "Relaxed.".to_string(),
            },
        };
        let draw = if self.game.draw == 1 {
            "Draw one"
        } else {
            "Draw three"
        };
        format!("{what} {draw}. This deal can always be won.")
    }

    fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let l = ctx.control_name(ParlorAction::Left);
        let r = ctx.control_name(ParlorAction::Right);
        let u = ctx.control_name(ParlorAction::Up);
        let d = ctx.control_name(ParlorAction::Down);
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let rep = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        format!(
            "{} and {r} move along the piles, the stock and the foundations. {u} and {d} move within a pile. {p} picks up and places. {t} turns the stock. Backspace undoes. Home and End jump to the first pile and the last foundation. {s} says what you hold and where you are, twice for the count, {rep} reads the table, {h} gives a hint.",
            sentence_start(&l),
            u = sentence_start(&u),
        )
    }

    /// The cards held, bottom first.
    fn held_cards(&self) -> Vec<Card> {
        match self.held {
            Some((spot, count)) => {
                let cards = self.game.table.cards_at(spot);
                cards[cards.len().saturating_sub(count)..].to_vec()
            }
            None => Vec::new(),
        }
    }

    fn held_words(&self) -> String {
        let cards = self.held_cards();
        match cards.as_slice() {
            [] => String::new(),
            [one] => the(*one),
            [first, .., last] => format!(
                "{}, {} down to {}",
                cards_words(cards.len()),
                the(*first),
                the(*last)
            ),
        }
    }

    /// Where the held cards would go from the place under the cursor.
    fn target(&self) -> Option<Move> {
        let (from, count) = self.held?;
        let to = match spot_of(self.cursor) {
            Spot::Waste => return None,
            Spot::Foundation(f) => match self.held_cards().first() {
                Some(card) if count == 1 => Spot::Foundation(card.suit.index()),
                _ => Spot::Foundation(f),
            },
            spot => spot,
        };
        Some(Move::new(from, count, to))
    }

    /// A place, as moving onto it reads it.
    pub fn place_words(&self, place: usize) -> String {
        let t = &self.game.table;
        let mut text = match spot_of(place) {
            Spot::Pile(p) => {
                let pile = &t.piles[p];
                match pile.top() {
                    None => format!("Pile {}, empty.", p + 1),
                    Some(card) => {
                        let mut counts = Vec::new();
                        if pile.up.len() > 1 {
                            counts.push(format!("{} face up", pile.up.len()));
                        }
                        if !pile.down.is_empty() {
                            counts.push(format!("{} face down", pile.down.len()));
                        }
                        let tail = if counts.is_empty() {
                            String::new()
                        } else {
                            format!(" {}.", sentence_start(&counts.join(", ")))
                        };
                        format!("Pile {}: {}.{tail}", p + 1, card.name_and_colour())
                    }
                }
            }
            Spot::Waste => self.stock_words(),
            Spot::Foundation(f) => match t.foundations[f] {
                0 => format!("{}, empty.", foundation_name(f)),
                KING => format!("{}, complete.", foundation_name(f)),
                r => format!("{}: up to the {}.", foundation_name(f), rank_words(r)),
            },
        };
        if self.cursor == place {
            if let Some(mv) = self.target() {
                if let (Spot::Foundation(here), Spot::Foundation(to)) = (spot_of(place), mv.from) {
                    if mv.from == mv.to && here != to {
                        text.push_str(&format!(" Goes back to {}.", Suit::from_index(to).name()));
                    }
                }
                if mv.from != mv.to && self.game.check(mv).is_ok() {
                    // On a foundation, a single card goes to its own suit's.
                    match (spot_of(place), mv.to) {
                        (Spot::Foundation(here), Spot::Foundation(to)) if here != to => {
                            text.push_str(&format!(" Goes to {}.", Suit::from_index(to).name()));
                        }
                        _ => text.push_str(" Fits here."),
                    }
                }
            }
        }
        text
    }

    fn stock_words(&self) -> String {
        let t = &self.game.table;
        match t.waste.last() {
            None if t.stock.is_empty() => "Stock, empty.".to_string(),
            None => format!("Stock: nothing turned up, {}.", to_turn(t.stock.len())),
            Some(card) => format!(
                "Stock: {}, turned up, {}.",
                card.name_and_colour(),
                to_turn(t.stock.len())
            ),
        }
    }

    /// Status, in a fixed order: what you hold, where you are, the
    /// foundations, the stock, face-down cards, then moves.
    /// The status key, in layers. First: what you hold and where you are.
    /// A second press: the foundations, the stock, face-down cards and
    /// moves.
    pub fn status_layers(&self) -> Vec<String> {
        let t = &self.game.table;
        let mut first = Vec::new();
        if self.held.is_some() {
            first.push(format!("holding {}", self.held_words()));
        }
        first.push(match spot_of(self.cursor) {
            Spot::Pile(_) => format!("on {}", place_name(self.cursor).to_lowercase()),
            _ => format!("on the {}", place_name(self.cursor).to_lowercase()),
        });
        let mut more = vec![
            format!("{} of {DECK} on the foundations", t.on_foundations()),
            to_turn(t.stock.len()),
            format!("{} face down", t.face_down()),
            moves_words(self.game.moves),
        ];
        if self.game.free_turns {
            more.push("turns are free".to_string());
        }
        if let Some(job) = &self.job {
            if job.deals() > 1 {
                more.push(format!("deal {} of {}", job.index + 1, job.deals()));
            }
        }
        vec![
            format!("{}.", sentence_start(&first.join(", "))),
            format!("{}.", sentence_start(&more.join(", "))),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }

    /// The whole table, top cards only, as Repeat reads it.
    pub fn table_words(&self) -> String {
        let t = &self.game.table;
        let mut out: Vec<String> = Vec::new();
        for (p, pile) in t.piles.iter().enumerate() {
            out.push(match pile.top() {
                None => format!("Pile {}: empty.", p + 1),
                Some(card) if pile.down.is_empty() => format!("Pile {}: {}.", p + 1, card.name()),
                Some(card) => format!(
                    "Pile {}: {}, {} face down.",
                    p + 1,
                    card.name(),
                    pile.down.len()
                ),
            });
        }
        out.push(self.stock_words());
        let founds: Vec<String> = Suit::ALL
            .into_iter()
            .map(|s| match t.foundations[s.index()] {
                0 => format!("{} empty", s.name()),
                r => format!("{} up to {}", s.name(), rank_words(r)),
            })
            .collect();
        out.push(format!("Foundations: {}.", founds.join(", ")));
        out.join(" ")
    }

    /// A move, as the hint names it.
    fn move_words(&self, mv: Move) -> String {
        let t = &self.game.table;
        let cards = t.cards_at(mv.from);
        let card = cards[cards.len() - mv.count];
        let what = match mv.from {
            Spot::Pile(p) => format!("{} in pile {}", the(card), p + 1),
            Spot::Waste => format!("the turned-up {}", card.name()),
            Spot::Foundation(_) => format!("{} on its foundation", the(card)),
        };
        let place = match mv.to {
            Spot::Foundation(_) => "go on its foundation".to_string(),
            Spot::Pile(q) => match t.piles[q].top() {
                Some(onto) => format!("go on {} in pile {}", the(onto), q + 1),
                None => format!("go to the empty pile {}", q + 1),
            },
            Spot::Waste => String::new(),
        };
        let mut text = format!("{} can {place}.", sentence_start(&what));
        if let Spot::Pile(p) = mv.from {
            if mv.count == cards.len() && !t.piles[p].down.is_empty() {
                text.push_str(" That turns up a face-down card.");
            }
        }
        text
    }

    fn stuck_words() -> String {
        "No helpful move left. Backspace undoes.".to_string()
    }

    fn hint_words(&self, ctx: &Ctx<Parlor>) -> String {
        match self.game.hint() {
            Hint::Move(mv) => self.move_words(mv),
            Hint::Turn => format!(
                "{} turns the stock. A card further in can be played.",
                sentence_start(&ctx.control_name(ParlorAction::Secondary))
            ),
            Hint::Stuck => {
                let mut text = PlayScreen::stuck_words();
                if !self.lesson && !self.is_challenge() && self.event.is_none() {
                    text.push_str(" For a fresh deal, Escape, then Back to the title.");
                }
                text
            }
            Hint::Won => "Every card is on the foundations.".to_string(),
        }
    }

    fn nope(&self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        sound::play(ctx, sound::NOPE);
        ctx.say(text.into());
    }

    /// The suit sound of the card under the cursor, if there is one.
    fn card_sound(&self, ctx: &mut Ctx<Parlor>) {
        let card = match spot_of(self.cursor) {
            Spot::Pile(p) => {
                let up = &self.game.table.piles[p].up;
                up.len()
                    .checked_sub(1 + self.depth)
                    .and_then(|i| up.get(i))
                    .copied()
            }
            spot => self.game.table.top(spot),
        };
        if let Some(card) = card {
            sound::card(ctx, card.suit);
        }
    }

    fn step(&mut self, ctx: &mut Ctx<Parlor>, right: bool) {
        let next = if right {
            (self.cursor + 1).min(PLACES - 1)
        } else {
            self.cursor.saturating_sub(1)
        };
        if next == self.cursor {
            let text = format!("End of the row. {}", self.place_words(self.cursor));
            self.nope(ctx, text);
            return;
        }
        self.go(ctx, next);
    }

    /// Home and End: the first pile, or the last foundation.
    fn jump(&mut self, ctx: &mut Ctx<Parlor>, place: usize) {
        if place == self.cursor {
            self.card_sound(ctx);
            let text = self.place_words(place);
            ctx.say(text);
            return;
        }
        self.go(ctx, place);
    }

    fn go(&mut self, ctx: &mut Ctx<Parlor>, next: usize) {
        let crossed = (self.cursor < STOCK_PLACE) != (next < STOCK_PLACE)
            || (self.cursor > STOCK_PLACE) != (next > STOCK_PLACE);
        self.cursor = next;
        self.depth = 0;
        if crossed {
            sound::play(ctx, sound::ROW_CHANGE);
        }
        self.card_sound(ctx);
        let text = self.place_words(next);
        ctx.say(text);
        if crossed && !self.told_row {
            self.told_row = true;
            ctx.say_queued("That chime marks a new part of the row.");
        }
    }

    fn within(&mut self, ctx: &mut Ctx<Parlor>, deeper: bool) {
        let Spot::Pile(p) = spot_of(self.cursor) else {
            let text = format!(
                "{} and {} move within a pile.",
                sentence_start(&ctx.control_name(ParlorAction::Up)),
                ctx.control_name(ParlorAction::Down)
            );
            self.nope(ctx, text);
            return;
        };
        let pile = &self.game.table.piles[p];
        if pile.up.is_empty() {
            let text = format!("Pile {} is empty.", p + 1);
            self.nope(ctx, text);
            return;
        }
        let len = pile.up.len();
        let card_at = |depth: usize| pile.up[len - 1 - depth];
        if deeper {
            if self.depth + 1 >= len {
                let card = card_at(self.depth);
                let text = match pile.down.len() {
                    0 => format!(
                        "{} is the bottom card of the pile.",
                        sentence_start(&the(card))
                    ),
                    n => format!(
                        "{} is the deepest face-up card, with {n} face down under it.",
                        sentence_start(&the(card))
                    ),
                };
                self.nope(ctx, text);
                return;
            }
            self.depth += 1;
        } else {
            if self.depth == 0 {
                let text = format!("Top card: {}.", card_at(0).name_and_colour());
                self.nope(ctx, text);
                return;
            }
            self.depth -= 1;
        }
        let card = card_at(self.depth);
        sound::card(ctx, card.suit);
        let place = match self.depth {
            0 => "the top card".to_string(),
            d => format!("{} from the top", ordinal(d + 1)),
        };
        ctx.say(format!(
            "{}, {place}.",
            sentence_start(&card.name_and_colour())
        ));
    }

    fn primary(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.held.is_some() {
            self.place(ctx);
        } else {
            self.pick_up(ctx);
        }
    }

    fn pick_up(&mut self, ctx: &mut Ctx<Parlor>) {
        let spot = spot_of(self.cursor);
        let cards = self.game.table.cards_at(spot);
        if cards.is_empty() {
            let text = match spot {
                Spot::Pile(p) => format!("Pile {} is empty.", p + 1),
                Spot::Waste if self.game.table.stock.is_empty() => {
                    "The stock is empty, and nothing is turned up.".to_string()
                }
                Spot::Waste => format!(
                    "Nothing turned up. {} turns the stock.",
                    sentence_start(&ctx.control_name(ParlorAction::Secondary))
                ),
                Spot::Foundation(f) => format!("{} is empty.", sentence_start(&foundation_name(f))),
            };
            self.nope(ctx, text);
            return;
        }
        let count = match spot {
            Spot::Pile(_) => (self.depth + 1).min(cards.len()),
            _ => 1,
        };
        self.held = Some((spot, count));
        let first = cards[cards.len() - count];
        sound::card(ctx, first.suit);
        ctx.say(format!("Holding {}.", self.held_words()));
        if !self.told_hold {
            self.told_hold = true;
            let p = ctx.control_name(ParlorAction::Primary);
            let them = if count == 1 { "it" } else { "them" };
            ctx.say_queued(format!(
                "Move to a pile or a foundation, and {p} places {them}. Backspace puts {them} back."
            ));
        }
    }

    fn put_back(&mut self, ctx: &mut Ctx<Parlor>) {
        let words = self.held_words();
        self.held = None;
        sound::play(ctx, sound::PLACE_OK);
        ctx.say(format!("Put back: {words}."));
    }

    fn place(&mut self, ctx: &mut Ctx<Parlor>) {
        let Some((from, _)) = self.held else {
            return;
        };
        if spot_of(self.cursor) == from {
            self.put_back(ctx);
            return;
        }
        let Some(mv) = self.target() else {
            sound::play(ctx, sound::PLACE_REFUSED);
            ctx.say("Not on the stock.");
            return;
        };
        // A card off a foundation, placed on another, goes back to its own.
        if mv.from == mv.to {
            self.put_back(ctx);
            return;
        }
        match self.game.play(mv) {
            Ok(played) => {
                self.held = None;
                self.depth = 0;
                self.cursor = place_of(mv.to);
                sound::play(ctx, sound::PLACE_OK);
                ctx.say(placed_words(&played));
                self.after_play(ctx, &played);
            }
            Err(refusal) => {
                sound::play(ctx, sound::PLACE_REFUSED);
                ctx.say(refusal.words());
            }
        }
    }

    /// What follows a move: a card turned up, a suit completed, the
    /// finish, or a gentle word when nothing helps.
    fn after_play(&mut self, ctx: &mut Ctx<Parlor>, played: &Played) {
        // A card turned up or put up makes a new table: a dead end after
        // it is news, so say it again.
        if played.flipped.is_some() || self.game.table.on_foundations() > self.stuck_up {
            self.told_stuck = false;
        }
        if let Some(card) = played.flipped {
            sound::play(ctx, sound::CARD_FLIP);
            if let Spot::Pile(p) = played.mv.from {
                ctx.say_queued(format!("Turned up in pile {}: {}.", p + 1, card.name()));
            }
        }
        if let Some(suit) = played.completed {
            sound::play(ctx, sound::FOUNDATION_DONE);
            ctx.say_queued(format!(
                "{} foundation complete.",
                sentence_start(suit.name())
            ));
        }
        if self.game.can_finish() {
            self.game.finish();
            ctx.say_queued("All face up. The rest go up on their own.");
        }
        if self.game.won() {
            self.win(ctx);
            return;
        }
        self.check_stuck(ctx);
    }

    fn check_stuck(&mut self, ctx: &mut Ctx<Parlor>) {
        // The quick check misses some lines, such as emptying a pile for
        // a king, so the solver has the last word. It only runs on tables
        // the quick check already calls dead.
        if !self.lesson && !self.told_stuck && self.game.stuck() && self.game.hint() == Hint::Stuck
        {
            self.told_stuck = true;
            self.stuck_up = self.game.table.on_foundations();
            ctx.say_queued(PlayScreen::stuck_words());
        }
    }

    fn turn(&mut self, ctx: &mut Ctx<Parlor>) {
        let back = if self.held.take().is_some() {
            "Put back. "
        } else {
            ""
        };
        match self.game.turn() {
            Ok(Turned::Drew(cards)) => {
                sound::play(ctx, sound::STOCK_TURN);
                let left = to_turn(self.game.table.stock.len());
                let top = cards.last().copied();
                let left = sentence_start(&left);
                let text = match (cards.len(), top) {
                    (1, Some(card)) => {
                        format!("{back}Turned up: {}. {left}.", card.name_and_colour())
                    }
                    (n, Some(card)) => {
                        // Draw three fans the cards: the ones under the top
                        // come into play next, so name them too.
                        let under: Vec<String> =
                            cards.iter().rev().skip(1).map(|c| c.name()).collect();
                        format!(
                            "{back}{n} turned up, {} on top. Under it, {}. {left}.",
                            card.name_and_colour(),
                            under.join(" and ")
                        )
                    }
                    _ => format!("{back}{left}."),
                };
                ctx.say(text);
                self.check_stuck(ctx);
            }
            Ok(Turned::Refilled(n)) => {
                sound::play(ctx, sound::STOCK_TURN);
                ctx.say(format!("{back}Stock refilled, {}.", to_turn(n)));
            }
            Err(refusal) => {
                let text = format!("{back}{}", refusal.words());
                self.nope(ctx, text);
            }
        }
    }

    fn undo(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.held.is_some() {
            self.put_back(ctx);
            return;
        }
        if self.game.undo() {
            self.depth = 0;
            self.told_stuck = false;
            sound::play(ctx, sound::UNDO);
            ctx.say(format!(
                "Move taken back. {}.",
                sentence_start(&moves_words(self.game.moves))
            ));
        } else {
            self.nope(ctx, "Nothing to take back.");
        }
    }

    fn win(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        self.held = None;
        if let Some(deal) = self.event {
            self.win_event(ctx, deal);
            return;
        }
        sound::play(ctx, sound::WIN_FLOURISH);
        if self.job.is_some() {
            self.finish_job_deal(ctx);
            return;
        }
        let moves = self.game.moves;
        let saved = progress::save_win(ctx, moves);
        let pages = result_pages(moves, &saved);
        let summary = format!("Deal won in {}", moves_words(moves));
        let mut results = Some(after_deal(summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    /// An event's deal is won on its own board.
    fn win_event(&mut self, ctx: &mut Ctx<Parlor>, deal: EventDeal) {
        let moves = self.game.moves;
        let score = Records::record(ctx, ID, deal.board(), points(moves));
        let mut pages = vec![event_first_page(deal, moves, score)];
        sound::play(ctx, sound::WIN_FLOURISH);
        pages.push("Turning the stock cost no moves, for a lucky month.".to_string());
        let summary = format!("{} won in {}", deal.name(), moves_words(moves));
        let mut results = Some(after_event(deal, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    /// A challenge deal is won: on to the next, or the challenge is over.
    fn finish_job_deal(&mut self, ctx: &mut Ctx<Parlor>) {
        let won = self.game.clone();
        let Some(job) = self.job.as_mut() else {
            return;
        };
        if let Some(next) = job.advance(&won) {
            let intro = challenge::next_words(job);
            self.game = next;
            self.cursor = 0;
            self.depth = 0;
            self.ended = false;
            self.told_stuck = false;
            ctx.say(format!("Deal won in {}.", moves_words(won.moves)));
            ctx.say_queued(intro);
            let first = self.place_words(0);
            ctx.say_queued(first);
            return;
        }
        let (Some(job), Some(ch)) = (self.job.take(), self.challenge.take()) else {
            return;
        };
        let pages = challenge::finish_job(ctx, &ch, &job);
        let summary = format!("{} result: {} points", ch.title(), job.score);
        let mut results = Some(after_job(ch.period, summary, pages.clone()));
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }
}

impl Default for PlayScreen {
    fn default() -> Self {
        PlayScreen::new()
    }
}

/// A move made, in words.
fn placed_words(played: &Played) -> String {
    let what = if played.cards.len() == 1 {
        the(played.cards[0])
    } else {
        cards_words(played.cards.len())
    };
    let what = sentence_start(&what);
    match played.mv.to {
        Spot::Foundation(f) => format!("{what} on the {} foundation.", Suit::from_index(f).name()),
        Spot::Pile(q) => match played.onto {
            Some(onto) => format!("{what} on {}, pile {}.", the(onto), q + 1),
            None => format!("{what} to pile {}.", q + 1),
        },
        Spot::Waste => what,
    }
}

/// The pages after a won relaxed deal.
pub fn result_pages(moves: u32, saved: &Saved) -> Vec<String> {
    let score = points(moves);
    let record = match saved.score {
        ScoreResult::First => "your first win".to_string(),
        ScoreResult::NewBest { previous } => format!("a new best, beating {previous}"),
        ScoreResult::Short { best } => format!("your best is {best}"),
    };
    let first = format!(
        "Deal won in {}. {score} points, {record}.",
        moves_words(moves)
    );
    let fewest = match saved.previous_fewest {
        Some(f) if moves < f => format!("A new fewest, {} under {f}.", moves_words(f - moves)),
        Some(f) => format!("Fewest moves: {f}."),
        None => format!("Fewest moves: {moves}."),
    };
    let wins = match saved.wins {
        1 => "1 deal won.".to_string(),
        n => format!("{n} deals won."),
    };
    vec![first, format!("{fewest} {wins}")]
}

/// The first page after an event's deal.
pub fn event_first_page(deal: EventDeal, moves: u32, score: ScoreResult) -> String {
    let points = points(moves);
    let record = match score {
        ScoreResult::First => format!("your first {} score", deal.board_words()),
        ScoreResult::NewBest { previous } => format!("a new best, beating {previous}"),
        ScoreResult::Short { best } => format!("your best is {best}"),
    };
    format!(
        "{} won in {}. {points} points, {record}.",
        deal.name(),
        moves_words(moves)
    )
}

/// The results menu after an event's deal. Once the event's window has closed, it
/// says the event has ended and goes back to the title.
pub fn after_event(deal: EventDeal, summary: String, pages: Vec<String>) -> Results {
    let help = "The same Rabbit rabbit deal again. Only your best score is kept.";
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        help,
        move |ctx| {
            let today = parlor_core::challenges::today(ctx);
            let next = events::RABBIT.is_open(today).then_some(deal);
            match next {
                Some(next) => ctx.replace(PlayScreen::event(next, today)),
                // The window closed while the results were up.
                None => ctx.pop_to_with(TITLE_NAME, events::ended_words(deal)),
            }
        },
    )
}

/// The results menu after a relaxed deal.
pub fn after_deal(summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        "A new deal, one that can always be won.",
        |ctx| ctx.replace(PlayScreen::new()),
    )
}

/// The results menu after a challenge.
pub fn after_job(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same cards, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(PlayScreen::challenge(ch))
        },
    )
}

impl State<Parlor> for PlayScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        PLAY_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        self.bed(ctx);
        if !self.is_challenge() && self.keep && self.event.is_none() {
            progress::note_deal(ctx);
        }
        if !self.lesson {
            let intro = self.intro();
            ctx.say(intro);
            // The full controls until the tutorial or a first deal has
            // taught them; after that, F1 has them.
            let taught = parlor_core::tutorial::tutorial_done(ctx, crate::ID)
                || progress::Progress::load(ctx).deals > 1;
            if taught {
                ctx.say_queued("F1 lists the controls.");
            } else {
                let c = self.controls(ctx);
                ctx.say_queued(c);
            }
        }
        self.card_sound(ctx);
        let first = self.place_words(self.cursor);
        ctx.say_queued(first);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        self.bed(ctx);
        ctx.say(format!(
            "Back at the table. {}",
            self.place_words(self.cursor)
        ));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ended {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if event.is_repeat() {
                return;
            }
            let leave = if self.is_challenge() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        let moving = matches!(
            action,
            Some(ParlorAction::Left | ParlorAction::Right | ParlorAction::Up | ParlorAction::Down)
        );
        if event.is_repeat() && !moving {
            return;
        }
        match action {
            Some(ParlorAction::Left) => self.step(ctx, false),
            Some(ParlorAction::Right) => self.step(ctx, true),
            Some(ParlorAction::Up) => self.within(ctx, true),
            Some(ParlorAction::Down) => self.within(ctx, false),
            Some(ParlorAction::Primary) => self.primary(ctx),
            Some(ParlorAction::Secondary) => self.turn(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status_layers();
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                let text = self.table_words();
                ctx.say(text);
            }
            Some(ParlorAction::Hint) => {
                let text = self.hint_words(ctx);
                ctx.say(text);
            }
            None => {
                if let InputEvent::KeyDown {
                    key, repeat: false, ..
                } = event
                {
                    if *key == UNDO_KEY {
                        self.undo(ctx);
                    } else if *key == Key::Home {
                        self.jump(ctx, 0);
                    } else if *key == Key::End {
                        self.jump(ctx, PLACES - 1);
                    }
                }
            }
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let t = &self.game.table;
        let mut out = vec![match &self.job {
            Some(job) => format!(
                "{}, deal {} of {}",
                challenge::name(job.period),
                job.index + 1,
                job.deals()
            ),
            None => match self.event {
                Some(deal) => format!("Fireside Patience: {}", deal.name()),
                None => "Fireside Patience: Relaxed".to_string(),
            },
        }];
        for (p, pile) in t.piles.iter().enumerate() {
            let up: Vec<String> = pile.up.iter().map(|c| c.name()).collect();
            let marker = if self.cursor == p { "> " } else { "" };
            out.push(format!(
                "{marker}Pile {}: {} face down, {}",
                p + 1,
                pile.down.len(),
                if up.is_empty() {
                    "empty".to_string()
                } else {
                    up.join(", ")
                }
            ));
        }
        out.push(self.stock_words());
        out.push(format!(
            "{} of {DECK} on the foundations, {}",
            t.on_foundations(),
            moves_words(self.game.moves)
        ));
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut now = format!(
            "{} Build the four foundations up by suit, from ace to king.",
            self.status()
        );
        if let Some(job) = &self.job {
            now.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count it.",
                job.period.label().to_lowercase(),
                challenge::name(job.period)
            ));
        }
        format!("{now} {} F1 helps, Escape pauses.", self.controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_playscreen;
