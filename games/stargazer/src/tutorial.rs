//! The tutorial: tonight's sky, a star found by ear, the guide tone, and a
//! star traced.

use hearken::text::sentence_start;
use hearken::{Ctx, Key};
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::screens::NightScreen;
use crate::sky::Body;
use crate::ID;

/// Tonight's sky, as a free night opens it. Enter, which heads home, is
/// kept back: the lesson ends before the night is summed up.
pub fn hill(ctx: &mut Ctx<Parlor>) -> Live<NightScreen> {
    Live::new(NightScreen::free(ctx).for_tutorial()).guard(|_, _, event| {
        matches!(event.pressed(), Some(Key::Return | Key::KpEnter))
            .then(|| "Stay a little longer: the tutorial ends with a traced star.".to_string())
    })
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let hill = hill(ctx);
    lesson_on(ctx, hill)
}

/// The tutorial on `hill`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, hill: Live<NightScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p, t, s, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );

    let sweep = hill.step(
        format!(
            "{} and {r} sweep your listening point around the sky 5 degrees at a time, and {u} and {d} raise and lower it. {} tells you the next star and how many steps away it is. Move until your point rests on a star.",
            sentence_start(&l),
            sentence_start(&h)
        ),
        |s: &NightScreen, _| {
            let n = &s.night;
            // A faint star counts: it chimes like any other, and the
            // trace step says which stars belong to a pattern.
            match n.sky.object_at(n.point()).map(|o| o.body) {
                Some(Body::Star { .. }) => {
                    Some("You are on a star.".to_string())
                }
                Some(Body::Faint) => Some(
                    "You are on a faint star. It belongs to no pattern, but you found it."
                        .to_string(),
                ),
                _ => None,
            }
        },
    );

    let guide = hill.step(
        format!(
            "{} toggles the guide tone, which leads you straight toward the next star. Try turning it on now.",
            sentence_start(&t)
        ),
        // The game has just said "Guide tone on, toward …", so the step
        // ends without a line of its own.
        |s: &NightScreen, _| s.night.guide.then(String::new),
    );

    let trace = hill.step(
        format!(
            "Every constellation has its stars in order, and the one you need is announced as next. Land on it and press {p} to trace it. Try tracing one star now."
        ),
        |s: &NightScreen, _| {
            (s.night.traced.values().sum::<usize>() >= 1).then(|| {
                "Trace every star of a pattern in order, and it goes into your star journal.".to_string()
            })
        },
    );

    Lesson::new(
        ID,
        "Stargazer",
        "You are lying back on a dark hilltop under the night sky with no clock to worry about. Try out the basics under tonight's sky",
    )
    .step(sweep)
    .step(guide)
    .step(trace)
    .outro(format!(
        "The sky follows the real calendar, and more constellations rise as your journal grows. {} reports where your point is, and pressing Enter twice heads home.",
        sentence_start(&s)
    ))
}
