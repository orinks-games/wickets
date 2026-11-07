//! The tutorial: a petal caught, a ride on the current and a clean catch,
//! in slow practice water where nothing can go wrong.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::rules::Mode;
use crate::run::{Launch, RunScreen};
use crate::ID;

/// The practice water.
pub fn water() -> Live<RunScreen> {
    let launch = Launch {
        mode: Mode::Practice,
        seed: Some(7),
    };
    Live::new(RunScreen::new(launch, None))
}

pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, water())
}

/// The tutorial on `water`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, water: Live<RunScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, go, s, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Status),
        sentence_start(&k(ParlorAction::Hint)),
    );
    let catch = water.step(
        format!(
            "{} sets the boat off. Hold {l} or {r} to paddle that way: you hear each stroke on that side. A petal is a soft bell floating toward you, rising as it comes. Steer so it is straight ahead as it reaches you.",
            sentence_start(&go)
        ),
        |s: &RunScreen, _| (s.run.caught >= 1).then(|| "Caught. Each catch plays a note in the tune.".to_string()),
    );
    let current = water.step(
        "The hiss ahead of you is the current, the fastest water. Steer into it and stay there for a few beats. Riding it scores points every beat.",
        |s: &RunScreen, _| (s.run.flow_beats >= 6).then(|| "Riding the current.".to_string()),
    );
    let clean = water.step(
        "A catch dead centre is a clean catch: a bright ring, and double points. Catch one clean.",
        |s: &RunScreen, _| (s.run.clean >= 1).then(|| "Clean.".to_string()),
    );
    let quiet = water.clone();
    Lesson::new(
        ID,
        "Little Rapids",
        "You sail a little boat down a stream, catching what floats along with the music. Learn in slow water, where nothing can go wrong",
    )
    .step(catch)
    .step(current)
    .step(clean)
    .outro(format!(
        "On a real stream, steer clear of stones and the banks: a bump costs your combo, and the band drops back. The longer your combo, the more of the band plays. {} gives your combo and multiplier. Press it again straight away for your score and progress. {h} says where the current is and what is coming.",
        sentence_start(&s)
    ))
    .on_exit(move |ctx| {
        quiet.with_mut(|s| s.silence(ctx));
        // The tutorial taught the petal, so a stream need not stop for it.
        let mut p = crate::progress::Progress::load(ctx);
        p.meet(crate::rules::Kind::Petal);
        p.save(ctx);
    })
}
