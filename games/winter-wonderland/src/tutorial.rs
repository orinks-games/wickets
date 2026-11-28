//! The tutorial: two gifts sorted on the first shift's belt, then one
//! letter answered at the Wish Desk.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::desk_run::DeskRun;
use crate::mode::Mode;
use crate::sort_run::SortRun;
use crate::ID;

/// Gifts the lesson sorts before moving on.
const GIFTS: u32 = 2;

/// The first shift of Gift Sort.
pub fn belt() -> Live<SortRun> {
    let mut run = SortRun::new(Mode::Free, 1);
    run.lesson = true;
    Live::new(run)
}

/// The first stack at the Wish Desk.
pub fn desk() -> Live<DeskRun> {
    let mut run = DeskRun::new(Mode::Free, 1);
    run.lesson = true;
    Live::new(run)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, belt(), desk())
}

/// The tutorial on `belt` and `desk`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, belt: Live<SortRun>, desk: Live<DeskRun>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, d, p, rep, s, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Repeat),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );

    // The wishes are said in the step, and the belt cannot start until
    // they have been said: the run's speech hold covers the lesson's
    // opening line and this prompt.
    let wishes = belt.with(|s| s.wishes(ctx));
    let brief_prompt = format!(
        "First up is Gift Sort. Here is who is waiting and how each wish sounds inside a box. {wishes} Press {} to start the belt.",
        sentence_start(&p)
    );
    let held = belt.clone();
    let brief = belt
        .step(brief_prompt, |s: &SortRun, _| {
            // Done once the first gift has landed, so the next prompt's
            // "Shake it now" comes with a gift there to shake.
            (s.playing() && s.shift().gift.is_some())
                .then(|| "The belt is running. A gift has landed.".to_string())
        })
        .start(move |_| held.with_mut(|s| s.lesson_prompt()));

    // The gift waits on the belt while this is said, then rides its whole
    // way.
    let held = belt.clone();
    let shake = belt
        .step(
            format!(
                "{} shakes the box so you can tell what is inside. Try shaking it now.",
                sentence_start(&p)
            ),
            |s: &SortRun, _| {
                s.shift()
                    .gift
                    .as_ref()
                    .is_some_and(|g| g.shakes > 0)
                    .then(|| MATCHED.to_string())
            },
        )
        .start(move |_| held.with_mut(|s| s.lesson_prompt()));

    const MATCHED: &str = "Now match that gift to a wish.";
    let sort_prompt = format!(
        "{} sends the gift to the person on your left, and {r} sends it to the person on your right. A wrong delivery costs clock time. {} repeats the wishes. Sort {GIFTS} gifts to move on.",
        sentence_start(&l),
        sentence_start(&rep)
    );
    // The shaken gift waits on the belt while this is said, then rides its
    // whole way.
    let held = belt.clone();
    let sort = belt
        .step(sort_prompt, |s: &SortRun, _| {
            (s.shift().sorted >= GIFTS).then(|| {
                format!("{GIFTS} gifts sorted. A gift left too long rides off the edge of the belt, and if it falls it breaks.")
            })
        })
        .start(move |_| held.with_mut(|s| s.lesson_prompt()));

    let open = desk.step(
        format!(
            "Next is the Wish Desk, where there is no clock. Children write in for one gift each, and the boxes wait on a shelf in front of you. {} opens the first letter.",
            sentence_start(&p)
        ),
        |s: &DeskRun, _| s.playing().then(|| "The first letter is open.".to_string()),
    );
    let find = desk.step(
        format!(
            "{} and {r} walk along the shelf, and {} shakes the box in front of you. Find the box that matches the wish, or press {} to read the letter again.",
            sentence_start(&l),
            sentence_start(&p),
            sentence_start(&rep)
        ),
        |s: &DeskRun, _| {
            let st = s.stack();
            st.answer()
                .is_some_and(|a| a == st.at && st.shelf[a].shakes > 0)
                .then(|| "That is the one.".to_string())
        },
    );
    let wrap = desk.step(
        format!(
            "Hold {p} to wrap it. Keep holding until it is wrapped. Let go early and nothing is wrapped."
        ),
        |s: &DeskRun, _| {
            (s.stack().answered() >= 1).then(|| "Wrapped and stamped. A wrong box goes back on the shelf and costs points, never time.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Winter Wonderland",
        "Winter Wonderland pairs Gift Sort on the workshop belt with the Wish Desk, and story mode plays both in order. Try out each part in turn",
    )
    .step(brief)
    .step(shake)
    .step(sort)
    .step(open)
    .step(find)
    .step(wrap)
    .outro(format!(
        "From shift 3, {d} drops unwanted gifts in the return sack. {} reports your status, reading the gift and clock first in Gift Sort and the shift, gifts sorted, and score on a second press. {} gives a hint. Midnight Countdown, the New Year's Eve rhythm game, teaches itself on its first night.",
        sentence_start(&s),
        sentence_start(&h)
    ))
}
