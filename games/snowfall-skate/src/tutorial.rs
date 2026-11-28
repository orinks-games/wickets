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
            "{} pushes off onto the ice. Hold {l} or {r} to lean across the five lanes. A lantern lights ahead of you, so line up in its lane as it reaches you to take it.",
            sentence_start(&go)
        ),
        |s: &SkateScreen, _| (s.skate.tally.taken >= 1).then(|| "Taken.".to_string()),
    );
    let line = pond.step(
        "Taking lanterns one after another builds a line, adding 1 more point each time up to 5 points. Take 3 lanterns in a row.",
        |s: &SkateScreen, _| (s.skate.line >= 3).then(|| "A line of 3.".to_string()),
    );
    let drift = pond.step(
        "Snow drifts sit on the ice between some lanterns, slowing you down and breaking your line if you hit them. Lean around the next drift you meet.",
        |s: &SkateScreen, _| {
            (s.skate.tally.patches_cleared >= 1).then(|| "Clear of the drift.".to_string())
        },
    );
    let spin = pond.step(
        format!(
            "Between lanterns you can spin for bonus points. Hold {go} until the spin is ready, then let go before you wobble. Land a spin."
        ),
        |s: &SkateScreen, _| (s.skate.tally.spins >= 1).then(|| "Landed.".to_string()),
    );
    let quiet = pond.clone();
    Lesson::new(
        ID,
        crate::NAME,
        "Glide across a frozen pond at night, catching lanterns as they light ahead of you. Practice here with no snow and no score",
    )
    .step(lantern)
    .step(line)
    .step(drift)
    .step(spin)
    .outro(format!(
        "On a real pond, snow settles as you skate and ends the night once it covers the ice, while each lantern sweeps part of the ice clear. Watch for thin ice on later ponds too. {} checks nearby hazards, the next lantern, and your spot on the ice, and pressing it again gives your score, line, and snow depth. {h} tells you which way to lean.",
        sentence_start(&s)
    ))
    .on_exit(move |ctx| {
        quiet.with_mut(|s| s.silence(ctx));
    })
}
