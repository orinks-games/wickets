//! The tutorial: an egg caught and banked in the practice lanes, where
//! nothing costs a basket.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::rules::Mode;
use crate::run::{Launch, RunScreen};
use crate::ID;

/// The practice lanes: slow, every kind explained as it first falls.
pub fn lanes() -> Live<RunScreen> {
    let launch = Launch {
        mode: Mode::Practice,
        seed: Some(3),
    };
    Live::new(RunScreen::new(launch, None))
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, lanes())
}

/// The tutorial on `lanes`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, lanes: Live<RunScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, bank, cart, s, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );

    let catch = lanes.step(
        format!(
            "{} and {r} step one lane at a time; each step says your lane. Each falling object sounds from its lane: lower and louder means it will land sooner. An egg is a soft whistle: stand in its lane as it lands to catch it.",
            sentence_start(&l)
        ),
        |s: &RunScreen, _| {
            (s.run.caught_total >= 1).then(|| "Caught, with a woody thump into your basket.".to_string())
        },
    );

    let banked = lanes.step(
        format!(
            "Eggs count once banked. The cart's soft bell rings from its edge lane, and {} rings it now. Go to that lane and press {bank} to bank.",
            sentence_start(&cart)
        ),
        |s: &RunScreen, _| {
            (s.run.banked_total >= 1).then(|| "Banked as points.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Basket Run",
        "It is Easter morning, and eggs tumble from the trees over the meadow. Catch them in your basket and carry them to the cart. Learn in the practice lanes, where nothing costs a basket",
    )
    .step(catch)
    .step(banked)
    .outro(format!(
        "In a run, step out of the lane of a gloopy mud clod or a buzzing bee. A golden egg sparkles and is worth five. Bank before the bell ends the round. {} says your lane and what lands next, again for the clock, and {h} says what to do right now.",
        sentence_start(&s)
    ))
}
