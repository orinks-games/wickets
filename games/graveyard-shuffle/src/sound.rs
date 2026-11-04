//! Sound keys, where they play, and the sound check's catalog.
//!
//! The songs are music, on `Bus::MUSIC`: the game starts each part on its
//! beat. The monsters and the dance cues are effects. The beds are quiet
//! ambience under everything.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::songs::{Monster, Place};

pub const TITLE_MUSIC: &str = "graveyard-shuffle/music/title";
pub const COUNT: &str = "graveyard-shuffle/count";
pub const SPOT_ON: &str = "graveyard-shuffle/judge/spot-on";
pub const CLOSE: &str = "graveyard-shuffle/judge/close";
pub const STUMBLE: &str = "graveyard-shuffle/judge/stumble";
pub const CLAP: &str = "graveyard-shuffle/clap";
pub const APPLAUSE: &str = "graveyard-shuffle/applause";
pub const BED_GATE: &str = "graveyard-shuffle/bed/gate";
pub const BED_CRYPT: &str = "graveyard-shuffle/bed/crypt";
pub const BED_CHURCH: &str = "graveyard-shuffle/bed/church";
pub const BED_LAB: &str = "graveyard-shuffle/bed/lab";

/// How loud a monster plays, over the music.
pub const CALL_VOLUME: f32 = 1.0;
/// How loud a song plays, under the monsters and the moves.
pub const TUNE_VOLUME: f32 = 0.7;
/// How loud a stray move plays: a move with no monster near it.
pub const STRAY_VOLUME: f32 = 0.45;

/// The sound a monster makes as it comes up to dance.
pub fn call_key(m: Monster) -> &'static str {
    match m {
        Monster::Zombie => "graveyard-shuffle/call/zombie",
        Monster::Skeleton => "graveyard-shuffle/call/skeleton",
        Monster::Mummy => "graveyard-shuffle/call/mummy",
        Monster::Vampire => "graveyard-shuffle/call/vampire",
        Monster::Werewolf => "graveyard-shuffle/call/werewolf",
        Monster::Ghost => "graveyard-shuffle/call/ghost",
    }
}

/// The sound of its move, danced.
pub fn step_key(m: Monster) -> &'static str {
    match m {
        Monster::Zombie => "graveyard-shuffle/move/zombie",
        Monster::Skeleton => "graveyard-shuffle/move/skeleton",
        Monster::Mummy => "graveyard-shuffle/move/mummy",
        Monster::Vampire => "graveyard-shuffle/move/vampire",
        Monster::Werewolf => "graveyard-shuffle/move/werewolf",
        Monster::Ghost => "graveyard-shuffle/move/ghost",
    }
}

/// Where a move lands around the listener.
pub fn step_place(m: Monster) -> Vec3 {
    match m {
        Monster::Vampire => Vec3::new(-2.0, 0.5, 0.0),
        Monster::Werewolf => Vec3::new(2.0, 0.5, 0.0),
        Monster::Skeleton => Vec3::new(0.0, 2.0, 0.0),
        Monster::Zombie => Vec3::new(0.0, -1.5, 0.0),
        Monster::Mummy | Monster::Ghost => Vec3::new(0.0, 0.5, 0.0),
    }
}

pub fn bed_key(place: Place) -> &'static str {
    match place {
        Place::Gate => BED_GATE,
        Place::Crypt => BED_CRYPT,
        Place::Church => BED_CHURCH,
        Place::Lab => BED_LAB,
    }
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_vol(ctx: &mut Ctx<Parlor>, key: &str, volume: f32) -> Sound {
    let s = ctx.audio.play(key, Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

/// A monster comes up to dance.
pub fn play_call(ctx: &mut Ctx<Parlor>, m: Monster) -> Sound {
    play_vol(ctx, call_key(m), CALL_VOLUME)
}

/// A move lands, in its place.
pub fn play_step(ctx: &mut Ctx<Parlor>, m: Monster, volume: f32) -> Sound {
    let s = ctx.audio.play_at(step_key(m), step_place(m), Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

/// A part of a song, on the music bus.
pub fn play_part(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    let s = ctx.audio.play(key, Bus::MUSIC);
    ctx.audio.set_volume(s, TUNE_VOLUME);
    s
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The place's bed, the title music off: the song plays from the game.
pub fn place_bed(ctx: &mut Ctx<Parlor>, place: Place) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(bed_key(place)));
}

/// Every cue but the monsters and moves, with its name and meaning, for
/// the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            COUNT,
            "Count: a coffin lid knock. Four of them count you in before each song, and three after a pause.",
        ),
        (
            SPOT_ON,
            "Spot on: a small spooky chime with your move. It landed right on the beat.",
        ),
        (
            CLOSE,
            "Close: a soft dull tick with your move. It landed in time, but a little early or late.",
        ),
        (
            STUMBLE,
            "Stumble: a thud and a clatter of bones. A move was missed, or the wrong move was danced.",
        ),
        (
            CLAP,
            "Bony claps: the crowd claps three times. Another 8 moves in a row, for a bonus.",
        ),
        (APPLAUSE, "Applause: the end of a song, with howls and rattles."),
    ]
}

/// Whether a catalog key is a bed or music, played on a music bus.
pub fn is_music(key: &str) -> bool {
    key == TITLE_MUSIC || key.starts_with("graveyard-shuffle/bed/")
}
