//! The tutorial: walk the lane, sow and water a plot, and talk to a
//! settler, in a practice hamlet that is never kept.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::rules::{Hamlet, Place, Plot};
use crate::screen::HamletScreen;
use crate::ID;

/// The practice hamlet.
pub fn practice() -> Live<HamletScreen> {
    Live::new(HamletScreen::new(Hamlet::practice(7), None))
}

pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, practice())
}

/// The tutorial on `hamlet`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, hamlet: Live<HamletScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (left, right, up, main, status, hint) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Primary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );
    let name = hamlet.with(|s| {
        s.h.borrow()
            .settlers
            .first()
            .map(|s| s.name.clone())
            .unwrap_or_default()
    });
    let walk = hamlet.step(
        format!(
            "You are on the lane, by the cottages. {} and {right} walk it, from the gate to the woods. Walk to the fields, past the green toward the woods.",
            sentence_start(&left)
        ),
        |s: &HamletScreen, _| {
            let at = s.h.borrow().at;
            (at.place() == Place::Fields).then(String::new)
        },
    );
    let sow = hamlet.step(
        format!(
            "{} goes into the fields, a row of plots at a time. Find a plot that is dug and ready to sow, and press {main} on it to sow a seed.",
            sentence_start(&up)
        ),
        |s: &HamletScreen, _| {
            let h = s.h.borrow();
            h.plots
                .iter()
                .any(|p| matches!(p, Plot::Sown { .. }))
                .then(String::new)
        },
    );
    let water = hamlet.step(
        format!(
            "A new seed needs water. {} on the same plot waters it.",
            sentence_start(&main)
        ),
        |s: &HamletScreen, _| {
            let h = s.h.borrow();
            h.plots
                .iter()
                .any(|p| matches!(p, Plot::Sown { watered: true, .. }))
                .then(|| "A watered crop grows overnight.".to_string())
        },
    );
    let talk = hamlet.step(
        format!(
            "{name} lives here too. {} tells you what needs you. Pressed again, it gives the day, then everyone. Find {name} along the lane and press Enter to talk.",
            sentence_start(&status)
        ),
        |s: &HamletScreen, _| {
            let h = s.h.borrow();
            h.settlers
                .iter()
                .any(|st| st.talked_today)
                .then(|| "A word with a settler cheers them up, and tells you what they want.".to_string())
        },
    );
    let quiet = hamlet.clone();
    Lesson::new(
        ID,
        "Larkfield",
        "Larkfield is an empty river hamlet, and you are its first farmer. Settlers come to stay, and each has a story to finish. Learn in a practice hamlet that is never kept",
    )
    .step(walk)
    .step(sow)
    .step(water)
    .step(talk)
    .outro(format!(
        "Settlers work on their own, at what they are best at. Each one plays an instrument in the hamlet's tune, from wherever they are working. At the end of the day the supper bell rings, and everyone eats from the larder. {} says what to do next whenever you are unsure.",
        sentence_start(&hint)
    ))
    .on_exit(move |ctx| {
        quiet.with_mut(|s| s.silence(ctx));
    })
}
