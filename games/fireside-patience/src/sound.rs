//! Sound keys, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound};
use parlor_core::{Mood, Parlor, Stem};

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
pub const TITLE_MUSIC: &str = "fireside-patience/music/title";

/// The table's music (see `parlor_core::score`): the accompaniment from
/// the deal, the tune once the first cards reach the foundations, then
/// the rest of the band as they fill. There is no clock, so no hurry.
pub const SCORE: [Stem; 4] = [
    Stem::always("fireside-patience/score/base", 1.0),
    Stem::between("fireside-patience/score/melody", 1.0, 0.0, 0.1),
    Stem::between("fireside-patience/score/pulse", 1.0, 0.15, 0.4),
    Stem::between("fireside-patience/score/lift", 1.0, 0.6, 0.9),
];

/// The score's mood with `cards` of the 52 on the foundations.
pub fn mood(cards: usize) -> Mood {
    Mood::new(cards as f32 / 52.0)
}

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

/// The title's music, with the fire and the clock under it.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(Some(AMBIENCE));
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The fire and the clock at the table, under its score.
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
