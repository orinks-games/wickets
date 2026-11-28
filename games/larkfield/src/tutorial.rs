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
            "You are standing on the lane by the cottages. {} and {right} walk from the gate all the way to the woods. Head past the green to the fields.",
            sentence_start(&left)
        ),
        |s: &HamletScreen, _| {
            let at = s.h.borrow().at;
            (at.place() == Place::Fields).then(String::new)
        },
    );
    let sow = hamlet.step(
        format!(
            "{} steps into the fields, one row of plots at a time. Walk onto a dug plot and press {main} to sow a seed.",
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
            "Freshly sown seeds need water. {} on that same plot waters it.",
            sentence_start(&main)
        ),
        |s: &HamletScreen, _| {
            let h = s.h.borrow();
            h.plots
                .iter()
                .any(|p| matches!(p, Plot::Sown { watered: true, .. }))
                .then(|| "Nice, a watered crop grows overnight.".to_string())
        },
    );
    let talk = hamlet.step(
        format!(
            "{name} lives here too. {} checks what needs attention and how much food is in the larder; press it again for the day and stores, and a third time for everyone's mood and work. Find {name} along the lane and press Enter to say hello.",
            sentence_start(&status)
        ),
        |s: &HamletScreen, _| {
            let h = s.h.borrow();
            h.settlers
                .iter()
                .any(|st| st.talked_today)
                .then(|| "A quick chat cheers a settler up, and they will tell you what they want.".to_string())
        },
    );
    let quiet = hamlet.clone();
    Lesson::new(
        ID,
        "Larkfield",
        "Larkfield starts as a quiet river hamlet, and you are its first farmer. Settlers arrive to build a life here, each with a dream to finish. Try the basics in a practice hamlet that is not saved",
    )
    .step(walk)
    .step(sow)
    .step(water)
    .step(talk)
    .outro(format!(
        "Settlers pick their own work based on what they are best at, and everyone gathers for supper from the larder at dusk. {} suggests what to do next whenever you are not sure.",
        sentence_start(&hint)
    ))
    .on_exit(move |ctx| {
        quiet.with_mut(|s| s.silence(ctx));
    })
}
