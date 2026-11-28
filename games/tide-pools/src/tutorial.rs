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
            "{} leans closer to the water, and {d} steps back. Standing back surveys this pool and its neighbours, while leaning in focuses on just this pool so you can reach in. Lean in close now.",
            sentence_start(&u)
        ),
        |s: &OutingScreen, _| {
            (s.outing.lean == Lean::In).then(String::new)
        },
    );

    let gather = pools.step(
        format!(
            "An anemone lives in this pool. Press {p} right after it calls to gather it."
        ),
        |s: &OutingScreen, _| {
            s.outing
                .gathered
                .contains(&Species::Anemone)
                .then(|| "You gathered the anemone. If you reach in when nothing has called, your hands come up empty.".to_string())
        },
    );

    let shy = pools.step(
        format!(
            "{} and {r} walk from pool to pool. The pool to your right holds a shy shore crab that hides when you lean in. Keep still until it creeps back out, then gather it.",
            sentence_start(&l)
        ),
        |s: &OutingScreen, _| {
            s.outing
                .gathered
                .contains(&Species::Crab)
                .then(|| "You gathered the crab.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Tide Pools",
        "The tide is out and rock pools stretch along the shore in front of you. Practice leaning in and gathering creatures here with no tide to rush you",
    )
    .step(lean)
    .step(gather)
    .step(shy)
    .outro(
        "On a real trip, you gather until the tide comes back in, and everything you find comes home to your aquarium. These practice creatures stay in their pools. Press F1 anytime for the rest of the controls.",
    )
}
