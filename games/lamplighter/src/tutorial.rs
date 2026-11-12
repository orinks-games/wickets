//! The tutorial: a short hall with no taper, walked and lit by ear.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::Round;
use crate::rules::{Alcove, Layout, Side};
use crate::ID;

/// The tutorial's hall.
pub const LESSON_HALL: Layout = Layout {
    length: 6,
    out: 3,
    taper: None,
    draughts: 0,
};

/// The tutorial hall's seed: its first lamp out is two steps in, so the
/// walk step waits for the player to walk.
const LESSON_SEED: u64 = 2;

/// The hall the lesson walks.
pub fn hall() -> Live<Round> {
    Live::new(Round::lesson_hall(LESSON_HALL, LESSON_SEED))
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, hall())
}

/// Whether a lamp that is out is at the player's step.
fn beside_out(r: &Round) -> bool {
    Side::BOTH
        .into_iter()
        .any(|side| r.hall.alcove(r.hall.pos, side) == Alcove::Out)
}

/// The tutorial on `hall`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, hall: Live<Round>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (up, down, left, right) = (
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Left),
        k(ParlorAction::Right),
    );
    let (p, s, r, h) = (
        k(ParlorAction::Primary),
        k(ParlorAction::Status),
        k(ParlorAction::Repeat),
        k(ParlorAction::Hint),
    );

    let walk = hall.step(
        format!(
            "You stand at the lobby end of the Long Hall, with a wall on each side. {} walks toward the far wall and {down} walks back. A thin whistle across empty glass is a lamp that is out, on the wall it comes from. Walk until the whistle is right beside you.",
            sentence_start(&up)
        ),
        |r: &Round, _| beside_out(r).then(|| "A lamp that is out is right beside you.".to_string()),
    );
    let light = hall.step(
        format!(
            "{} reaches to the left wall and {right} to the right. Reach toward the whistle to light the lamp.",
            sentence_start(&left)
        ),
        |r: &Round, _| (r.hall.lit >= 1).then(String::new),
    );
    let rest = hall.step(
        format!(
            "Now light the rest. {} finds the nearest lamp out, and {r} says what is at your step.",
            sentence_start(&h)
        ),
        |r: &Round, _| (r.hall.out() == 0).then(String::new),
    );

    Lesson::new(
        ID,
        "Lamplighter",
        "The lamps along the Long Hall have gone out, and Mabel has handed you her old taper to light them again",
    )
    .step(walk)
    .step(light)
    .step(rest)
    .outro(format!(
        "In the evening round a taper burns as you go, and {p} lights it to start. Light every lamp before it burns down, and the seconds left are a bonus. If it runs out, Mabel lights you a fresh one, so the hall always gets lit. {} reads the lamps still out, your step and the taper time left.",
        sentence_start(&s)
    ))
}
