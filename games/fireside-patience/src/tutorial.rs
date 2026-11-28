//! The tutorial: a small table laid out to teach each move, played on the
//! real play screen.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::PlayScreen;
use crate::rules::{Card, Game, Pile, Suit, Table};
use crate::ID;

fn card(rank: u8, suit: Suit) -> Card {
    Card { rank, suit }
}

fn up(cards: &[Card]) -> Pile {
    Pile {
        down: Vec::new(),
        up: cards.to_vec(),
    }
}

/// The lesson's table: the ace of hearts on pile 5, a black 7 ready for a
/// red 8, the 2 of hearts next in the stock, and the ace of clubs at the
/// bottom of the stock, so finding it takes the whole stock.
pub fn table() -> Table {
    use Suit::*;
    Table {
        piles: vec![
            up(&[card(13, Spades)]),
            up(&[card(8, Diamonds)]),
            Pile {
                down: vec![card(12, Hearts)],
                up: vec![card(7, Spades)],
            },
            up(&[card(10, Clubs)]),
            up(&[card(1, Hearts)]),
            up(&[card(5, Clubs)]),
            up(&[card(11, Diamonds)]),
        ],
        stock: vec![card(1, Clubs), card(9, Clubs), card(4, Spades), card(2, Hearts)],
        waste: Vec::new(),
        foundations: [0; 4],
    }
}

/// The lesson's table on the real play screen.
pub fn screen() -> Live<PlayScreen> {
    Live::new(PlayScreen::practice(Game::from_table(table(), 1, 0)).in_lesson())
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
    lesson_on(ctx, screen())
}

/// The tutorial on `screen`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, screen: Live<PlayScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, p, t, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Hint),
    );
    let hearts = Suit::Hearts.index();

    let find = screen.step(
        format!(
            "The table has 7 piles, the stock, and 4 foundations in one row. {} and {r} move along it, naming each place's top card. Find the ace of hearts.",
            sentence_start(&l)
        ),
        |s: &PlayScreen, _| (s.cursor == 4).then(|| "There it is.".to_string()),
    );

    let ace = screen.step(
        format!(
            "{} picks up the card under you, and pressing {p} again places it. Foundations build one suit from ace up to king. Put the ace on the hearts foundation, past the stock.",
            sentence_start(&p)
        ),
        move |s: &PlayScreen, _| {
            (s.game.table.foundations[hearts] >= 1)
                .then(|| "Placed.".to_string())
        },
    );

    let seven = screen.step(
        "Piles build down in alternating colours, so a black 7 goes on a red 8. Move the 7 of spades from pile 3 onto the 8 of diamonds on pile 2.",
        |s: &PlayScreen, _| {
            (s.game.table.piles[1].top() == Some(card(7, Suit::Spades))).then(|| {
                "A face-down card turns up when you move the cards off it.".to_string()
            })
        },
    );

    let stock = screen.step(
        format!(
            "{} flips a card up from the stock, and {p} picks it up when you are on the stock. Turn up the 2 of hearts and move it onto its foundation.",
            sentence_start(&t)
        ),
        {
            // The game names the card and its foundation: let it be heard.
            let mut heard = Heard::default();
            move |s: &PlayScreen, ctx| {
                heard
                    .after(s.game.table.foundations[hearts] >= 2, ctx)
                    .then(String::new)
            }
        },
    );

    let clubs = Suit::Clubs.index();
    let aces = screen.step(
        format!(
            "The other aces hide face down in the piles or deeper in the stock. Press {t} until the ace of clubs appears, then move it to its foundation."
        ),
        {
            let mut heard = Heard::default();
            move |s: &PlayScreen, ctx| {
                heard
                    .after(s.game.table.foundations[clubs] >= 1, ctx)
                    .then(String::new)
            }
        },
    );

    let refill = screen.step(
        format!(
            "{} flips the stock back over whenever it runs out, as often as you like. Turn it over now.",
            sentence_start(&t)
        ),
        {
            // Ends on the turn that refills the stock, not on a stock that
            // was never used up.
            let mut ran_out = false;
            let mut heard = Heard::default();
            move |s: &PlayScreen, ctx| {
                let t = &s.game.table;
                ran_out |= t.stock.is_empty();
                heard
                    .after(ran_out && !t.stock.is_empty() && t.waste.is_empty(), ctx)
                    .then(String::new)
            }
        },
    );

    let mut most = 0;
    let mut heard = Heard::default();
    let undo = screen.step(
        "Backspace takes back your last move, as many times as you like. Press it now.",
        move |s: &PlayScreen, ctx| {
            most = most.max(s.game.moves);
            heard.after(s.game.moves < most, ctx).then(String::new)
        },
    );

    Lesson::new(
        ID,
        "Fireside Patience",
        "Settle in by the fire for solitaire: sort all 52 cards onto the four suit foundations, ace to king. There is no clock and you cannot lose. Try the basics on a small table.",
    )
    .step(find)
    .step(ace)
    .step(seven)
    .step(stock)
    .step(aces)
    .step(refill)
    .step(undo)
    .outro(format!(
        "Build all 4 foundations from ace to king to win. Only a king can move onto an empty pile, and {} suggests a useful move any time. Pick Play to deal a full game.",
        sentence_start(&h)
    ))
}
