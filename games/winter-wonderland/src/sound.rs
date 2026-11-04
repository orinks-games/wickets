//! Sound keys, and where Gift Sort's places are heard around the player.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::gifts::{Kind, Spot};
use crate::night::Inst;

pub const BELT: &str = "winter-wonderland/sort/belt";
pub const ARRIVE: &str = "winter-wonderland/sort/arrive";
pub const SEND: &str = "winter-wonderland/sort/send";
pub const CORRECT: &str = "winter-wonderland/sort/correct";
pub const WRONG: &str = "winter-wonderland/sort/wrong";
pub const FALL: &str = "winter-wonderland/sort/fall";
pub const WARN: &str = "winter-wonderland/sort/warn";
pub const SACK: &str = "winter-wonderland/sort/sack";
pub const CLOCK: &str = "winter-wonderland/sort/clock";
pub const CHANGE: &str = "winter-wonderland/sort/change";
pub const CLEAR: &str = "winter-wonderland/sort/clear";
pub const HINT: &str = "winter-wonderland/sort/hint";
pub const EMPTY: &str = "winter-wonderland/sort/empty";

pub const TICK: &str = "winter-wonderland/count/tick";
pub const TOCK: &str = "winter-wonderland/count/tock";
pub const BASS: &str = "winter-wonderland/count/bass";
pub const CHORD: &str = "winter-wonderland/count/chord";
pub const MISS: &str = "winter-wonderland/count/miss";
pub const OOPS: &str = "winter-wonderland/count/wrong";
pub const STRAY: &str = "winter-wonderland/count/stray";
pub const COMBO: &str = "winter-wonderland/count/combo";
pub const CLEAN: &str = "winter-wonderland/count/perfect";
pub const LOW: &str = "winter-wonderland/count/low";
pub const POP: &str = "winter-wonderland/count/pop";
pub const TOLL: &str = "winter-wonderland/count/toll";
pub const CHEER: &str = "winter-wonderland/count/cheer";
pub const FIREWORKS: &str = "winter-wonderland/count/fireworks";

pub const FIRE: &str = "winter-wonderland/ambience/fire";
/// Gift Sort's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("winter-wonderland");
pub const TITLE_MUSIC: &str = "winter-wonderland/music/title";
pub const WIN: &str = "winter-wonderland/music/win";
pub const LOSE: &str = "winter-wonderland/music/lose";

/// The shake of a gift of `kind`.
pub fn gift(kind: Kind) -> &'static str {
    match kind {
        Kind::Rattle => "winter-wonderland/gifts/rattle",
        Kind::Squish => "winter-wonderland/gifts/squish",
        Kind::Clink => "winter-wonderland/gifts/clink",
        Kind::Jingle => "winter-wonderland/gifts/jingle",
        Kind::Crinkle => "winter-wonderland/gifts/crinkle",
        Kind::Slosh => "winter-wonderland/gifts/slosh",
        Kind::Tick => "winter-wonderland/gifts/tick",
        Kind::Squeak => "winter-wonderland/gifts/squeak",
        Kind::Thump => "winter-wonderland/gifts/thump",
        Kind::Chime => "winter-wonderland/gifts/chime",
    }
}

/// The band's (and the player's) note on `inst`.
pub fn inst(inst: Inst) -> &'static str {
    match inst {
        Inst::Bell => "winter-wonderland/count/bell",
        Inst::Drum => "winter-wonderland/count/drum",
        Inst::Horn => "winter-wonderland/count/horn",
    }
}

/// Where a recipient (or the sack) is heard: left, ahead, right, behind.
pub fn spot_pos(spot: Spot) -> Vec3 {
    match spot {
        Spot::Left => Vec3 {
            x: -2.0,
            y: 0.5,
            z: 0.0,
        },
        Spot::Ahead => Vec3 {
            x: 0.0,
            y: 2.5,
            z: 0.0,
        },
        Spot::Right => Vec3 {
            x: 2.0,
            y: 0.5,
            z: 0.0,
        },
        Spot::Sack => Vec3 {
            x: 0.0,
            y: -1.5,
            z: 0.0,
        },
    }
}

/// Where the gift on the belt is heard: just in front of the player.
pub const GIFT_POS: Vec3 = Vec3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};

/// A cue on the effects bus.
pub fn cue(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

/// A cue placed at a recipient's side.
pub fn cue_at(ctx: &mut Ctx<Parlor>, key: &str, spot: Spot) -> Sound {
    ctx.audio.play_at(key, spot_pos(spot), Bus::EFFECTS)
}

/// A stinger on the music bus.
pub fn stinger(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::MUSIC)
}
