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
/// red 8, and the 2 of hearts next in the stock.
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
        stock: vec![card(9, Clubs), card(4, Spades), card(2, Hearts)],
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

/// Waits for what the game says to be heard before a step ends, so the
/// lesson's next words do not cut it off.
#[derive(Default)]
struct Heard {
    /// `ctx.said` when last looked.
    said: u64,
    /// When the lines said so far should be done.
    until: f64,
}

impl Heard {
    /// Whether everything said since `armed` first held has been heard.
    /// Lines said in the frame that arms it count.
    fn after(&mut self, armed: bool, ctx: &Ctx<Parlor>) -> bool {
        if !armed {
            self.said = ctx.said;
            self.until = ctx.clock;
            return false;
        }
        let new = ((ctx.said - self.said) as usize).min(ctx.transcript.len());
        let lines = &ctx.transcript[ctx.transcript.len() - new..];
        for line in lines.iter().filter(|l| !l.starts_with("[event]")) {
            self.until = self.until.max(ctx.clock) + speech_secs(line);
        }
        self.said = ctx.said;
        ctx.clock >= self.until
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
            "Seven piles, stock, then four foundations. {} and {r} move; each place says its top card. Find the ace of hearts.",
            sentence_start(&l)
        ),
        |s: &PlayScreen, _| (s.cursor == 4).then(|| "There it is.".to_string()),
    );

    let ace = screen.step(
        format!(
            "{} picks up the card under you. Move to where it goes, and {p} again places it. A foundation takes one suit, ace first. Put the ace on the hearts foundation, past the stock.",
            sentence_start(&p)
        ),
        move |s: &PlayScreen, _| {
            (s.game.table.foundations[hearts] >= 1)
                .then(|| "The soft tap means placed.".to_string())
        },
    );

    let seven = screen.step(
        "A pile builds down in alternating colours: a black 7 goes on a red 8. Move the 7 of spades from pile 3 onto the 8 of diamonds on pile 2.",
        |s: &PlayScreen, _| {
            (s.game.table.piles[1].top() == Some(card(7, Suit::Spades))).then(|| {
                "The flip means a hidden card turned up.".to_string()
            })
        },
    );

    let stock = screen.step(
        format!(
            "{} turns a card up from the stock. At the stock's place, {p} picks it up. Turn up the 2 of hearts and put it on its foundation.",
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
        "A game of patience waits by the fire: put all 52 cards on the four foundations, by suit, from ace to king. There is no clock and no way to lose. Learn on a small table",
    )
    .step(find)
    .step(ace)
    .step(seven)
    .step(stock)
    .step(undo)
    .outro(format!(
        "Only a king goes on an empty pile, and every relaxed deal can be won. {} gives a useful move. Play deals a full game.",
        sentence_start(&h)
    ))
}
