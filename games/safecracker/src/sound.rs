//! Sound keys, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::{Detent, Noise};

/// The safe's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("safecracker");
pub const CLICK: &str = "safecracker/dial/click";
pub const DETENT: &str = "safecracker/dial/detent";
pub const ZERO: &str = "safecracker/dial/zero";
pub const SPIN: &str = "safecracker/dial/spin";
pub const CLUNK: &str = "safecracker/tumbler/clunk";
pub const FALSE_GATE: &str = "safecracker/tumbler/false";
pub const SET: &str = "safecracker/tumbler/set";
pub const SPRING: &str = "safecracker/tumbler/spring";
pub const OPEN: &str = "safecracker/safe/open";
pub const ALARM: &str = "safecracker/safe/alarm";
pub const STEPS: &str = "safecracker/guard/steps";
pub const STOP: &str = "safecracker/guard/stop";
pub const HEARD: &str = "safecracker/guard/heard";
pub const CAUGHT: &str = "safecracker/guard/caught";
pub const TICK: &str = "safecracker/cue/tick";
pub const HINT: &str = "safecracker/cue/hint";
pub const NOPE: &str = "safecracker/cue/nope";
pub const PIPE: &str = "safecracker/noise/pipe";
pub const CREAK: &str = "safecracker/noise/creak";
pub const CLOCK: &str = "safecracker/noise/clock";
pub const ROOM: &str = "safecracker/ambience/room";
pub const TITLE_MUSIC: &str = "safecracker/music/title";
pub const WIN: &str = "safecracker/music/win";
pub const LOSE: &str = "safecracker/music/lose";
pub const ONWARD: &str = "safecracker/safe/onward";
/// The unlucky safe's black cat, just after the clunk at 13.
pub const YOWL: &str = "safecracker/event/yowl";
/// Rain on the windows: the blackout safe's bed, in place of the room hum.
pub const BLACKOUT_BED: &str = "safecracker/ambience/blackout";

/// The click a number makes.
pub fn detent(d: Detent) -> &'static str {
    match d {
        Detent::Plain => CLICK,
        Detent::Ten => DETENT,
        Detent::Zero => ZERO,
    }
}

pub fn noise(kind: Noise) -> &'static str {
    match kind {
        Noise::Pipe => PIPE,
        Noise::Creak => CREAK,
        Noise::Clock => CLOCK,
    }
}

/// Where the guard's footsteps sit: beyond the door, ahead and a little to
/// the right.
pub const DOOR: Vec3 = Vec3::new(1.5, 6.0, 0.0);

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_vol(ctx: &mut Ctx<Parlor>, key: &str, volume: f32) -> Sound {
    let s = ctx.audio.play(key, Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The quiet room bed on, music off.
pub fn room_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(ROOM));
}

/// Every cue, with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (CLICK, "Dial click: the dial moved one number."),
        (
            DETENT,
            "Ten detent: a firmer double click at every tenth number, 10, 20 and so on, to help you count.",
        ),
        (ZERO, "Zero notch: a deep notch as the dial reaches zero, the top of the dial."),
        (SPIN, "Spin: a quick ratchet as the dial spins ten numbers at once."),
        (
            CLUNK,
            "Tumbler clunk: the dial passed or reached this wheel's number. Deep and solid. Stop, step back to it, and set the wheel. On later safes it is quieter.",
        ),
        (
            FALSE_GATE,
            "False gate: a hollow, rattling tick at a wrong number. Ignore it: it is not the tumbler.",
        ),
        (SET, "Wheel set: a latch and a rising chime. The wheel is set, on to the next."),
        (
            SPRING,
            "Spring: a wrong set. The wheel springs back and you take a strike. Three strikes and the safe jams.",
        ),
        (OPEN, "Safe open: the handle turns and the bolts slide back."),
        (
            ONWARD,
            "Onward: a heavy door and a bell. In a challenge, this safe is open and the next one is waiting.",
        ),
        (ALARM, "Relocker: the safe jams and the alarm rings. The safe is lost."),
        (
            STEPS,
            "Footsteps: the guard is coming down the corridor. When they stop, keep still.",
        ),
        (
            STOP,
            "Guard at the door: the footsteps stop and his keys jingle. Keep still: do not turn or set until he walks away.",
        ),
        (
            HEARD,
            "Who's there: the guard heard you. One more noise while he listens and you are caught.",
        ),
        (CAUGHT, "Whistle: the guard caught you. The safe is lost."),
        (TICK, "Clock tick: time attack is running low."),
        (NOPE, "Dull tap: nothing to do, or no hints left."),
        (
            PIPE,
            "Knocking pipe: a building noise, off to one side. Not the safe.",
        ),
        (
            CREAK,
            "Creaking floor: a building noise, off to one side. Not the safe.",
        ),
        (
            CLOCK,
            "Distant clock: a building noise, off to one side. Not the safe.",
        ),
        (
            YOWL,
            "Cat yowl: a soft, short yowl just after a clunk. On Friday the 13th, the unlucky safe's cat: this wheel's number is 13.",
        ),
    ]
}
