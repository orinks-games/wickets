//! The tutorial: the two lanterns of the practice pond, gathered by ear.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::rules::Mode;
use crate::screens::{Launch, RunScreen};
use crate::ID;

/// The practice pond. Heading home is kept back: the lesson ends with the
/// second lantern, before the night is summed up.
pub fn pond() -> Live<RunScreen> {
    let launch = Launch {
        mode: Mode::Pond,
        seed: Some(1),
    };
    Live::new(RunScreen::new(launch, None)).guard(|_, ctx, event| {
        (ctx.action_for(event) == Some(ParlorAction::Secondary))
            .then(|| "Stay out a little longer: a lantern is still on the lake.".to_string())
    })
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, pond())
}

/// The tutorial on `pond`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, pond: Live<RunScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
    );

    let paddle = pond.step(
        format!(
            "{} paddles straight ahead toward the first lantern. Hold it to keep moving, and use {d} to slow the canoe down.",
            sentence_start(&u),
        ),
        |s: &RunScreen, _| {
            (s.lake().gathered >= 1).then(String::new)
        },
    );

    let turn = pond.step(
        format!(
            "{} and {r} turn the canoe: tap to turn a little, or hold to keep turning. The second lantern sits off to your right, so turn until it is straight ahead and paddle over to gather it.",
            sentence_start(&l)
        ),
        |s: &RunScreen, _| {
            (s.lake().gathered >= 2).then(String::new)
        },
    );

    Lesson::new(
        ID,
        "Lantern Lake",
        "You are out in a canoe on a night lake, gathering drifting lanterns to build the night's song. There is no timer here, and you can start on the practice pond to get the feel of paddling",
    )
    .step(paddle)
    .step(turn)
    .outro(
        "Frogs and other creatures stay in the same spots all night to serve as your landmarks, and reeds brushing the canoe mean you have reached the shore. Press F1 anytime to review your controls.",
    )
}
