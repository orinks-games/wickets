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
            .then(|| "Stay out a little longer: a lantern is still waiting.".to_string())
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
            "{} paddles toward the first lantern, directly ahead. Hold it to keep paddling. Listen for the lantern's chime to ring faster as you approach. {d} slows the canoe.",
            sentence_start(&u),
            d = sentence_start(&d),
        ),
        |s: &RunScreen, _| {
            (s.lake().gathered >= 1).then(String::new)
        },
    );

    let turn = pond.step(
        format!(
            "{} and {r} turn the canoe: a short press turns a little, and holding keeps turning. A chime behind you is muffled. The second lantern is off to your right. Turn until its chime is straight ahead, in both ears, then paddle to it.",
            sentence_start(&l)
        ),
        |s: &RunScreen, _| {
            (s.lake().gathered >= 2).then(String::new)
        },
    );

    Lesson::new(
        ID,
        "Lantern Lake",
        "You sit in a canoe on a quiet lake at night, gathering lanterns by ear. There is no timer and no way to fail. Learn on the practice pond",
    )
    .step(paddle)
    .step(turn)
    .outro(
        "Listen for frogs and other creatures to find your way around the lake. They stay in the same places all night. Reeds brushing the canoe mean you have reached the shore. F1 explains your controls.",
    )
}
