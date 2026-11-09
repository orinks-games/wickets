//! The tutorial: the chickadee of the practice glade, met and befriended.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::rules::Phase;
use crate::walk::WalkScreen;
use crate::ID;

/// Said as the chickadee notices you.
const NOTICED: &str = "It noticed you, and the rest of the glade hushes.";

/// About how long a line takes to say.
fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}

/// The practice glade. Heading home is kept back: the lesson ends at the
/// chickadee, before the walk is summed up.
pub fn glade() -> Live<WalkScreen> {
    let mut walk = WalkScreen::practice();
    walk.lesson = true;
    Live::new(walk).guard(|_, ctx, event| {
        (ctx.action_for(event) == Some(ParlorAction::Secondary))
            .then(|| "Stay a moment: the chickadee is waiting for you.".to_string())
    })
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, glade())
}

/// The tutorial on `glade`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, glade: Live<WalkScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p, rep, h, tab) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Repeat),
        k(ParlorAction::Hint),
        k(ParlorAction::Secondary),
    );

    let approach = glade.step(
        format!(
            "{} steps forward and {d} steps back; {l} and {r} turn you. Songs come from their true place; one behind you is softer. Walk toward the chickadee until it sings to you.",
            sentence_start(&u),
            l = sentence_start(&l),
        ),
        |s: &WalkScreen, _| {
            matches!(s.walk.phase, Phase::Listening(_) | Phase::Answering(..))
                .then(|| NOTICED.to_string())
        },
    );

    let answer_prompt = format!(
        "When the song ends, whistle it back note for note. {} is low, {p} is middle, {u} is high. {} asks the bird to sing once more.",
        sentence_start(&d),
        sentence_start(&rep)
    );
    // The chickadee waits for the page to be read before it sings.
    let page = format!("{NOTICED} {answer_prompt}");
    let held = glade.clone();
    let answer = glade
        .step(answer_prompt, |s: &WalkScreen, _| {
            s.walk.birds[0].friend.then(|| {
                "On a trail, every bird you befriend goes in your field guide.".to_string()
            })
        })
        .start(move |_ctx| held.with_mut(|s| s.walk.hold_song(speech_secs(&page))));

    Lesson::new(
        ID,
        "Birdsong Trail",
        "You walk a trail at dawn, and answer birds' songs to befriend them. There is no way to fail and no clock. Learn in the practice glade",
    )
    .step(approach)
    .step(answer)
    .outro(format!(
        "On a trail, {p} names the birds nearest you, and {h} says which way the nearest new bird is. Each trail ends at a sound you can walk toward, or press {tab} twice to head home. The birds follow the real seasons."
    ))
}
