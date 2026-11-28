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
            "{} and {r} move you one lane at a time, announcing your lane as you go. Every drop comes down in its own lane until it lands. Step into an egg's lane and stay there as it lands to catch it.",
            sentence_start(&l)
        ),
        |s: &RunScreen, _| {
            (s.run.caught_total >= 1).then(|| "Caught.".to_string())
        },
    );

    let banked = lanes.step(
        format!(
            "Caught eggs only count once you bank them. The cart sits in an edge lane, and {} rings its bell so you can find it. Head over to that lane and press {bank} to bank your basket.",
            sentence_start(&cart)
        ),
        |s: &RunScreen, _| {
            (s.run.banked_total >= 1).then(|| "Banked as points.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Basket Run",
        "It is Easter morning, and eggs are tumbling down from the trees over the meadow. Catch them in your basket and carry them back to the cart. You will try it out in the practice lanes first, where nothing costs a basket.",
    )
    .step(catch)
    .step(banked)
    .outro(format!(
        "In a real run, step out of the way of mud clods and diving bees. Golden eggs are worth five regular eggs, and you need to bank before the bell ends the round. Press {} for your lane and what lands next, press it again for the clock, and press {h} whenever you want a hint.",
        sentence_start(&s)
    ))
}
