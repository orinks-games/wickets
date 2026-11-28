//! The tutorial: the practice wire, walked with a voice alongside.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::rules::{Mode, Phase};
use crate::screens::{Launch, RunScreen};
use crate::ID;

/// The practice wire, seeded so every first walk meets the same wire.
/// Falling costs nothing on it, and the lesson ends on the far roof,
/// before any results.
pub fn wire() -> Live<RunScreen> {
    let mut screen = RunScreen::new(
        Launch {
            mode: Mode::Training,
            seed: Some(7),
        },
        None,
    );
    screen.in_lesson = true;
    Live::new(screen)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, wire())
}

/// The tutorial on `wire`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, wire: Live<RunScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, p, s, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );

    let steps = wire.step(
        format!(
            "The tone tracks your balance, and centered is safe. {} and {r} each shift your weight once per press, so tap and release rather than holding, always pressing opposite your lean. {} takes a step once you are balanced. Try taking 3 steps.",
            sentence_start(&l),
            sentence_start(&p)
        ),
        |s: &RunScreen, _| (s.run().pos >= 3).then(|| "3 steps out.".to_string()),
    );

    let across = wire.step(
        format!(
            "Keep walking all the way to the garage roof. A gust and then a pigeon will come along, and each is explained as it arrives. Falls cost nothing here. {} reports your lean, and {h} gives advice.",
            sentence_start(&s)
        ),
        // The arrival is already said: nothing to add before the outro.
        |s: &RunScreen, _| matches!(s.run().phase, Phase::Crossed { .. }).then(String::new),
    );

    Lesson::new(
        ID,
        "Tightrope",
        "You are stepping out onto a wire strung between rooftops, high above the city. Start out on a low practice wire over the garden",
    )
    .step(steps)
    .step(across)
    .outro("Long wires sway the hardest near mid-span. You can walk this practice wire again any time from the title menu.")
}
