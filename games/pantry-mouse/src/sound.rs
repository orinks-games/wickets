//! Sound keys, where things sit around the mouse, and the sound check's
//! catalog.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

pub const STEP: &str = "pantry-mouse/step-wood";
pub const WALL: &str = "pantry-mouse/wall-bump";
pub const NIBBLE: &str = "pantry-mouse/nibble";
pub const SNIFF_TICK: &str = "pantry-mouse/sniff-tick";
pub const CAT_BELL: &str = "pantry-mouse/cat/bell";
pub const CAT_PAD: &str = "pantry-mouse/cat/pad";
pub const CAT_PURR: &str = "pantry-mouse/cat/purr";
pub const CAT_CLOSE: &str = "pantry-mouse/cat-close";
pub const CAUGHT: &str = "pantry-mouse/caught";
pub const FREEZE: &str = "pantry-mouse/freeze";
pub const THAW: &str = "pantry-mouse/thaw";
pub const CLEAR: &str = "pantry-mouse/pantry-clear";
pub const AMBIENCE: &str = "pantry-mouse/pantry-ambience";
pub const TITLE_MUSIC: &str = "pantry-mouse/title-music";
pub const TURKEY: &str = "pantry-mouse/treat/turkey";
pub const CHEESE_RIND: &str = "pantry-mouse/treat/moon-cheese";

/// A treat's sound: played where it hides on a sniff, and as you eat it.
pub fn treat(t: crate::rules::Treat) -> &'static str {
    match t {
        crate::rules::Treat::Turkey => TURKEY,
        crate::rules::Treat::CheeseRind => CHEESE_RIND,
    }
}

/// A treat's sound, in plain words.
pub fn treat_words(t: crate::rules::Treat) -> &'static str {
    match t {
        crate::rules::Treat::Turkey => "a juicy chew",
        crate::rules::Treat::CheeseRind => "a soft glassy chime",
    }
}

/// Metres between the centres of two cells, for placing sounds.
pub const CELL_METRES: f32 = 2.0;

/// The loop each cat wears, by its place in [`crate::rules::CATS`].
pub fn cat_loop(i: usize) -> &'static str {
    match i {
        0 => CAT_BELL,
        1 => CAT_PAD,
        _ => CAT_PURR,
    }
}

/// A cat's sound, in plain words, by its place in the cat list.
pub fn cat_sound_words(i: usize) -> &'static str {
    match i {
        0 => "a bell on its collar",
        1 => "soft padding paws",
        _ => "a low purr",
    }
}

/// Where a cell offset sits around the listener: right is right, up the
/// maze is ahead.
pub fn place(dx: i32, dy: i32) -> Vec3 {
    Vec3::new(dx as f32 * CELL_METRES, -(dy as f32) * CELL_METRES, 0.0)
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, dx: i32, dy: i32) -> Sound {
    ctx.audio.play_at(key, place(dx, dy), Bus::EFFECTS)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The pantry at night, music off.
pub fn pantry_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(AMBIENCE));
}

/// Every sound of the game with its name and meaning, for the sound check,
/// in the order a player meets them. The name is the text before the
/// colon.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (STEP, "Step: a soft tap on wooden boards. You moved one cell."),
        (
            NIBBLE,
            "Nibble: a quick crunch. You ate a crumb, for 10 points.",
        ),
        (
            WALL,
            "Wall: a dull thud. A wall is in the way, and the open ways are spoken.",
        ),
        (
            SNIFF_TICK,
            "Sniff tick: a tiny bright tick. Each one is a crumb, from its direction, nearest first.",
        ),
        (
            CAT_BELL,
            "Ginger cat: a bell on its collar. It heads straight for you.",
        ),
        (
            CAT_PAD,
            "Tabby cat: soft padding paws. It wanders until it is 4 steps away, then chases.",
        ),
        (
            CAT_PURR,
            "Grey cat: a low purr. It wanders like the tabby, and joins in the monthly challenge.",
        ),
        (
            CAT_CLOSE,
            "Cat close: a short rising meow from the cat's side. A cat is 2 steps away or nearer.",
        ),
        (
            CAUGHT,
            "Caught: a pounce and a squeak. A cat caught you and you lose a whisker.",
        ),
        (
            FREEZE,
            "Freeze: an indrawn breath. You keep still and the nearest cat loses track of you.",
        ),
        (THAW, "Thaw: a soft breath out. You can move again."),
        (
            CLEAR,
            "Pantry clear: a bright rising run of notes. Every crumb is gone.",
        ),
        (
            TURKEY,
            "Turkey scrap: a juicy chew. In the Leftovers pantry at Thanksgiving, a turkey scrap, for 50 points. A sniff plays it from where the nearest one hides.",
        ),
        (
            CHEESE_RIND,
            "Cheese rind: a soft glassy chime. On Bin night, the first Tuesday of every month, a cheese rind in every pantry, for 100 points. A sniff plays it from where it hides.",
        ),
    ]
}

/// The sounds the sound check loops on the music bus rather than playing
/// once.
pub fn is_loop(key: &str) -> bool {
    matches!(key, CAT_BELL | CAT_PAD | CAT_PURR | AMBIENCE | TITLE_MUSIC)
}
