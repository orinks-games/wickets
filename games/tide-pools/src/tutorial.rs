//! The tutorial: the first two practice pools, an anemone and a shy crab.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::outing::{outing_for, OutingScreen};
use crate::rules::{Lean, Mode, Species};
use crate::ID;

/// The practice pools, with no tide.
pub fn pools() -> Live<OutingScreen> {
    Live::new(OutingScreen::new(outing_for(Mode::Practice), None).in_lesson())
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, pools())
}

/// The tutorial on `pools`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, pools: Live<OutingScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
    );

    let lean = pools.step(
        format!(
            "{} leans toward the water; {d} steps back. Stand back to take in this pool and its neighbours; lean in for only this one. Lean in close.",
            sentence_start(&u)
        ),
        |s: &OutingScreen, _| {
            (s.outing.lean == Lean::In).then(String::new)
        },
    );

    let gather = pools.step(
        format!(
            "This pool holds an anemone. Press {p} just after it calls."
        ),
        |s: &OutingScreen, _| {
            s.outing
                .gathered
                .contains(&Species::Anemone)
                .then(|| "The anemone is yours. Reach in when nothing has called, and your hands come up with only water.".to_string())
        },
    );

    let shy = pools.step(
        format!(
            "{} and {r} walk from pool to pool. The pool on your right holds a shy shore crab. It darts away when you lean in, so keep still until it creeps back out, then gather it.",
            sentence_start(&l)
        ),
        |s: &OutingScreen, _| {
            s.outing
                .gathered
                .contains(&Species::Crab)
                .then(|| "The crab is yours.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Tide Pools",
        "The tide is out, and rock pools line the shore with the sea ahead of you. There is no way to fail: gather what you find. Learn in the practice pools",
    )
    .step(lean)
    .step(gather)
    .step(shy)
    .outro(
        "A trip lasts until the tide comes back: a bell buoy rings when it turns. On a trip, what you gather goes home to your aquarium. These practice creatures go back in the water. F1 lists the rest.",
    )
}
