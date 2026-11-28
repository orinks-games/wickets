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
            "{} launches your boat. Hold {l} or {r} to paddle across the stream. A petal is floating toward you, so steer until it is straight ahead as it reaches you.",
            sentence_start(&go)
        ),
        |s: &RunScreen, _| (s.run.caught >= 1).then(|| "Caught.".to_string()),
    );
    let current = water.step(
        "Ahead of you is the current, the fastest water in the stream. Steer into it and ride along for a few beats to score points on every beat.",
        |s: &RunScreen, _| (s.run.flow_beats >= 6).then(|| "Riding the current.".to_string()),
    );
    let clean = water.step(
        "Catching something right at the middle of your boat is a clean catch and scores double. Line one up dead centre.",
        |s: &RunScreen, _| (s.run.clean >= 1).then(|| "Clean.".to_string()),
    );
    let quiet = water.clone();
    Lesson::new(
        ID,
        "Little Rapids",
        "Sail a paper boat down a winding stream, catching petals and scraps on the beat. Practice here in slow water where nothing can go wrong",
    )
    .step(catch)
    .step(current)
    .step(clean)
    .outro(format!(
        "On a real stream, steer clear of stones and the banks, because a bump resets your combo. {} checks your combo and multiplier, and pressing it again right away gives your score and progress. {h} tells you where the current is and what is coming next.",
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
