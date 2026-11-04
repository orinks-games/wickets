//! Sound keys, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

use crate::rules::Suit;

pub const CARD_HEARTS: &str = "fireside-patience/card/hearts";
pub const CARD_DIAMONDS: &str = "fireside-patience/card/diamonds";
pub const CARD_CLUBS: &str = "fireside-patience/card/clubs";
pub const CARD_SPADES: &str = "fireside-patience/card/spades";
pub const CARD_FLIP: &str = "fireside-patience/card-flip";
pub const PLACE_OK: &str = "fireside-patience/place-ok";
pub const PLACE_REFUSED: &str = "fireside-patience/place-refused";
pub const STOCK_TURN: &str = "fireside-patience/stock-turn";
pub const FOUNDATION_DONE: &str = "fireside-patience/foundation-done";
pub const ROW_CHANGE: &str = "fireside-patience/row-change";
pub const UNDO: &str = "fireside-patience/undo";
pub const NOPE: &str = "fireside-patience/nope";
pub const WIN_FLOURISH: &str = "fireside-patience/win-flourish";
pub const AMBIENCE: &str = "fireside-patience/fireside-ambience";
pub const MOONLIT: &str = "fireside-patience/moonlit-ambience";

/// The soft card sound of a suit.
pub fn card_key(suit: Suit) -> &'static str {
    match suit {
        Suit::Hearts => CARD_HEARTS,
        Suit::Diamonds => CARD_DIAMONDS,
        Suit::Clubs => CARD_CLUBS,
        Suit::Spades => CARD_SPADES,
    }
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn card(ctx: &mut Ctx<Parlor>, suit: Suit) -> Sound {
    play(ctx, card_key(suit))
}

/// The fire and the clock, under the title and the table alike.
pub fn fireside(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(AMBIENCE));
}

/// The Rabbit rabbit deal's evening: the fire low, the clock, and an owl
/// outside.
pub fn moonlit(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(MOONLIT));
}

/// Whether `key` is an ambience bed, which plays on the ambience bus.
pub fn is_bed(key: &str) -> bool {
    key == AMBIENCE || key == MOONLIT
}

/// Every sound, with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            CARD_HEARTS,
            "Hearts card: a paper slide with a warm low bell. Every hearts card you reach or pick up.",
        ),
        (
            CARD_DIAMONDS,
            "Diamonds card: a paper slide with a bright high bell.",
        ),
        (
            CARD_CLUBS,
            "Clubs card: a paper slide with a soft wooden knock.",
        ),
        (
            CARD_SPADES,
            "Spades card: a paper slide with a deep wooden knock.",
        ),
        (
            CARD_FLIP,
            "Card flip: a face-down card turns face up. Its name follows.",
        ),
        (PLACE_OK, "Soft tap: the cards you held are placed."),
        (
            PLACE_REFUSED,
            "Muffled thud: the cards cannot go there. The reason follows, and you keep holding them.",
        ),
        (
            STOCK_TURN,
            "Stock turn: a card is turned up from the stock, or the stock is refilled.",
        ),
        (
            FOUNDATION_DONE,
            "Foundation chime: a suit is complete, ace to king.",
        ),
        (
            ROW_CHANGE,
            "Row chime: you cross from the piles to the stock, or from the stock to the foundations.",
        ),
        (UNDO, "Undo swish: the last move is taken back."),
        (NOPE, "Dull tap: nothing to do there just now."),
    ]
}
