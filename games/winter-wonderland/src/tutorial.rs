//! The tutorial: two gifts sorted on the first shift's belt, then a few
//! answers to the band on a practice night.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::count_run::CountRun;
use crate::mode::Mode;
use crate::sort_run::SortRun;
use crate::ID;

/// Gifts the lesson sorts before moving on.
const GIFTS: u32 = 2;
/// Notes the lesson answers on the beat before it ends.
const NOTES: u32 = 4;

/// The first shift of Gift Sort.
pub fn belt() -> Live<SortRun> {
    let mut run = SortRun::new(Mode::Free, 1);
    run.lesson = true;
    Live::new(run)
}

/// About how long a line takes to say, so the belt waits for it.
fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// A practice night of Midnight Countdown: the party never fizzles.
pub fn party() -> Live<CountRun> {
    let mut run = CountRun::new(Mode::Practice, 1);
    run.lesson = true;
    Live::new(run)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, belt(), party())
}

/// The tutorial on `belt` and `party`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, belt: Live<SortRun>, party: Live<CountRun>) -> Lesson {
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

    // The wishes are said in the step, and the belt waits until they have
    // been heard.
    let wishes = belt.with(|s| s.wishes(ctx));
    let brief_prompt = format!(
        "Gift Sort. Here is who is waiting, and how each wish sounds in a box. {wishes} {} starts the belt.",
        sentence_start(&p)
    );
    // The lesson's opening line comes first: about five seconds.
    let brief_wait = 5.0 + speech_secs(&brief_prompt);
    let held = belt.clone();
    let brief = belt
        .step(brief_prompt, |s: &SortRun, _| {
            s.playing()
                .then(|| "The belt is running. A soft thump means a gift has landed.".to_string())
        })
        .start(move |_| held.with_mut(|s| s.hold_for(brief_wait)));

    let shake = belt.step(
        format!(
            "{} shakes the gift, and you hear what is inside. Shake it now.",
            sentence_start(&p)
        ),
        |s: &SortRun, _| {
            s.shift()
                .gift
                .as_ref()
                .is_some_and(|g| g.shakes > 0)
                .then(|| MATCHED.to_string())
        },
    );

    const MATCHED: &str = "Now match that sound to a wish.";
    let sort_prompt = format!(
        "{} sends the gift to the person on your left, and {r} to the person on your right. Rising bells mean you got it right, and a low buzz costs clock time. {} says the wishes again. Sort {GIFTS} gifts.",
        sentence_start(&l),
        sentence_start(&rep)
    );
    // The shaken gift waits on the belt until this has been heard.
    let sort_wait = speech_secs(&format!("{MATCHED} {sort_prompt}"));
    let held = belt.clone();
    let sort = belt
        .step(sort_prompt, |s: &SortRun, _| {
            (s.shift().sorted >= GIFTS).then(|| {
                format!("{GIFTS} gifts sorted. Quick beeping means a gift is near the edge, and if it falls it breaks.")
            })
        })
        .start(move |_| held.with_mut(|s| s.hold_for(sort_wait)));

    let rhythm = party.step(
        format!(
            "Midnight Countdown, on New Year's Eve. The band plays a call one bar long, and you answer it in the next bar on the same beats. A clock ticks each beat, with a lower tock on the first beat of a bar. Bells are {p}, and {p} starts the music. Answer {NOTES} notes."
        ),
        |s: &CountRun, _| {
            let n = s.night();
            (n.perfects + n.goods >= NOTES).then(|| "On the beat. Every answer in a row raises your combo.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Winter Wonderland",
        "Two parts, and the story plays both in order. Learn each in turn",
    )
    .step(brief)
    .step(shake)
    .step(sort)
    .step(rhythm)
    .outro(format!(
        "From shift 3, {d} sends unwanted gifts to the sack. {}: clock and score. {}: hint.",
        sentence_start(&s),
        sentence_start(&h)
    ))
}
