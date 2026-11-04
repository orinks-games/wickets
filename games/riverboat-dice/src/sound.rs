//! Sound keys, where each seat's sounds come from, and the sound check's
//! catalog.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::{Sfx, YOU};

pub const CUP_RATTLE: &str = "riverboat-dice/cup-rattle";
pub const DICE_LAND: &str = "riverboat-dice/dice-land";
pub const ONE_THUD: &str = "riverboat-dice/one-thud";
pub const SNAG_CRASH: &str = "riverboat-dice/snag-crash";
pub const CHAIR_SCRAPE: &str = "riverboat-dice/chair-scrape";
pub const COIN_STACK: &str = "riverboat-dice/coin-stack";
pub const TABLE_GROAN: &str = "riverboat-dice/table-groan";
pub const TABLE_CHEER: &str = "riverboat-dice/table-cheer";
pub const NOPE: &str = "riverboat-dice/nope";
pub const PADDLE_WHEEL: &str = "riverboat-dice/paddle-wheel";
pub const HIGH_WATER: &str = "riverboat-dice/high-water";
pub const MOON_CHIME: &str = "riverboat-dice/moon-chime";
pub const SALOON_BAND: &str = "riverboat-dice/saloon-band";
pub const TITLE_MUSIC: &str = "riverboat-dice/title-music";

/// The dealer sits across the table from you.
pub const DEALER: Vec3 = Vec3::new(0.0, 2.0, 0.0);

/// Where a seat is, heard from yours: with three opponents, the first on
/// your left, the second across, the third on your right. A lone opponent
/// sits across.
pub fn seat_position(seat: usize, opponents: usize) -> Vec3 {
    if seat == YOU {
        return Vec3::new(0.0, 0.0, 0.0);
    }
    if opponents == 1 {
        return Vec3::new(0.0, 2.5, 0.0);
    }
    match seat {
        1 => Vec3::new(-2.0, 1.5, 0.0),
        2 => Vec3::new(0.0, 2.5, 0.0),
        _ => Vec3::new(2.0, 1.5, 0.0),
    }
}

/// Which side a seat is on, in words: "on your left".
pub fn side_words(seat: usize, opponents: usize) -> &'static str {
    let pos = seat_position(seat, opponents);
    if pos.x < -0.5 {
        "on your left"
    } else if pos.x > 0.5 {
        "on your right"
    } else {
        "across the table"
    }
}

pub fn key(sfx: Sfx) -> &'static str {
    match sfx {
        Sfx::Rattle => CUP_RATTLE,
        Sfx::Land => DICE_LAND,
        Sfx::Thud => ONE_THUD,
        Sfx::Snag => SNAG_CRASH,
        Sfx::Coin => COIN_STACK,
        Sfx::Groan => TABLE_GROAN,
        Sfx::Cheer => TABLE_CHEER,
        Sfx::Moon => MOON_CHIME,
    }
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

/// The dice and the cup come from the dealer's side.
pub fn play_sfx(ctx: &mut Ctx<Parlor>, sfx: Sfx) -> Sound {
    match sfx {
        Sfx::Rattle | Sfx::Land | Sfx::Thud | Sfx::Snag | Sfx::Moon => {
            ctx.audio.play_at(key(sfx), DEALER, Bus::EFFECTS)
        }
        _ => play(ctx, key(sfx)),
    }
}

/// A chair scrape from a seat.
pub fn chair(ctx: &mut Ctx<Parlor>, seat: usize, opponents: usize) -> Sound {
    ctx.audio
        .play_at(CHAIR_SCRAPE, seat_position(seat, opponents), Bus::EFFECTS)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The boat around the table: the paddle wheel as the bed, or at high
/// water the rushing river in its place, and the band in the saloon as a
/// quiet loop on the ambience bus. Returns the band's loop so the screen
/// can stop it.
pub fn saloon(ctx: &mut Ctx<Parlor>, high_water: bool) -> Sound {
    ctx.audio.play_music(None, false);
    ctx.audio
        .set_bed(Some(if high_water { HIGH_WATER } else { PADDLE_WHEEL }));
    ctx.audio.start_loop(SALOON_BAND, None, Bus::AMBIENCE)
}

/// Every sound, with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            CUP_RATTLE,
            "Cup rattle: the dealer shakes two dice in a leather cup. A roll is coming.",
        ),
        (
            DICE_LAND,
            "Dice landing: two dice tumble onto the felt. The dealer calls the faces and the sum.",
        ),
        (
            ONE_THUD,
            "One thud: a dull wooden thud. A die shows 1, and the landing is over for everyone still seated.",
        ),
        (
            SNAG_CRASH,
            "Snag crash: a crash of glass and wood. A double 1 with the snag rule on wipes the night total of everyone seated.",
        ),
        (
            CHAIR_SCRAPE,
            "Chair scrape: a player stands and banks. It comes from their seat, left, across or right.",
        ),
        (
            COIN_STACK,
            "Coin stack: you bank the table's total into your night total.",
        ),
        (TABLE_GROAN, "Table groan: the table groans at a 1."),
        (TABLE_CHEER, "Table cheer: you win the night."),
        (NOPE, "Dull tap: nothing to do just now."),
        (
            MOON_CHIME,
            "Showboat chime: on Showboat Saturday, doubles double what the table holds.",
        ),
    ]
}
