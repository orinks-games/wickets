//! The tutorial: push off, take a lantern, keep clear of a drift and land
//! a spin, on the practice pond where nothing is scored.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::{Mode, SkateScreen};
use crate::ID;

/// The practice pond, going round again for as long as the lesson needs.
pub fn pond(ctx: &mut Ctx<Parlor>) -> Live<SkateScreen> {
    Live::new(SkateScreen::with_seed(ctx, Mode::Practice, 7).laps())
}

pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let p = pond(ctx);
    lesson_on(ctx, p)
}

/// The tutorial on `pond`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, pond: Live<SkateScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, go, s, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Status),
        sentence_start(&k(ParlorAction::Hint)),
    );
    let lantern = pond.step(
        format!(
            "{} pushes off. Hold {l} or {r} to lean that way across the ice. A lantern lights ahead of you in one of five lanes. Be in its lane as it reaches you to take it.",
            sentence_start(&go)
        ),
        |s: &SkateScreen, _| (s.skate.tally.taken >= 1).then(|| "Taken.".to_string()),
    );
    let line = pond.step(
        "Take lanterns one after another to build a line. Each lantern in a line is worth 1 more point, up to 5 points. Take 3 in a row.",
        |s: &SkateScreen, _| (s.skate.line >= 3).then(|| "A line of 3.".to_string()),
    );
    let drift = pond.step(
        "Drifts of snow lie on the ice between lanterns. Skating into one slows you and breaks your line. Lean round the next drift you meet.",
        |s: &SkateScreen, _| {
            (s.skate.tally.patches_cleared >= 1).then(|| "Clear of the drift.".to_string())
        },
    );
    let spin = pond.step(
        format!(
            "Between lanterns you can spin. Hold {go} until the spin is ready, then let go. Hold it too long and you wobble. Land a spin."
        ),
        |s: &SkateScreen, _| (s.skate.tally.spins >= 1).then(|| "Landed.".to_string()),
    );
    let quiet = pond.clone();
    Lesson::new(
        ID,
        crate::NAME,
        "You skate across a pond at night, taking lanterns as they light. Learn on the practice pond, where nothing is scored",
    )
    .step(lantern)
    .step(line)
    .step(drift)
    .step(spin)
    .outro(format!(
        "On a real pond, snow settles as you skate, and the night ends when it covers the ice. Each lantern taken keeps it clear a little longer. Lean round thin ice on the later ponds too. {} gives nearby hazards, the next lantern and your position. Press it again straight away for your score, your line and the snow. {h} tells you what to do next.",
        sentence_start(&s)
    ))
    .on_exit(move |ctx| {
        quiet.with_mut(|s| s.silence(ctx));
    })
}
