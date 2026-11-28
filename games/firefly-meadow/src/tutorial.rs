//! The tutorial: the real practice patch, one skill at a time.

use hearken::text::sentence_start;
use hearken::{Ctx, Key};
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::evening::{plan_for, EveningScreen, Mode};
use crate::ID;

const SEED: u64 = 5;

/// The practice patch the lesson runs.
pub fn patch(ctx: &mut Ctx<Parlor>) -> Live<EveningScreen> {
    let plan = plan_for(ctx, Mode::Practice);
    // Heading home is kept back: the lesson ends before the evening is
    // summed up.
    let mut screen = EveningScreen::new(ctx, plan, Mode::Practice, SEED);
    // The lesson's own steps say when a firefly is in reach.
    screen.taught_already(&["near"]);
    Live::new(screen).guard(|_, _, event| {
        matches!(event.pressed(), Some(Key::Return | Key::KpEnter))
            .then(|| "Stay out a little longer: the lesson ends in the meadow.".to_string())
    })
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let patch = patch(ctx);
    lesson_on(ctx, patch)
}

/// The tutorial on `patch`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, patch: Live<EveningScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p, t) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
    );
    let start = patch.with(|s| s.evening.pos);

    // Walking and reach in one step: the walk is toward the firefly, so
    // reaching it never comes before it is taught.
    let walk = patch.step(
        format!(
            "{}, {d}, {l}, and {r} walk one step at a time, with {u} always north. A firefly is drifting out in the meadow. Walk toward it until it is in reach.",
            sentence_start(&u)
        ),
        move |s: &EveningScreen, _| {
            let (x, y) = s.evening.pos;
            let walked = (x - start.0).abs() + (y - start.1).abs() >= 1.0;
            (walked && s.evening.in_reach()).then(|| "In reach.".to_string())
        },
    );

    let catch = patch.step(
        format!(
            "{} catches a firefly in reach, as long as it is glowing. Press it right as the firefly lights up.",
            sentence_start(&p)
        ),
        // The meadow says what was caught; the lesson goes on without a
        // second line.
        |s: &EveningScreen, _| (s.evening.caught >= 1).then(String::new),
    );

    let release = patch.step(
        format!(
            "{} sets the fireflies in your jar free, ringing their notes together as a chord. Release your jar now.",
            sentence_start(&t)
        ),
        |s: &EveningScreen, _| (s.evening.releases >= 1).then(String::new),
    );

    Lesson::new(
        ID,
        "Firefly Meadow",
        "Wander a meadow at dusk and catch blinking fireflies in your jar. Nothing here can go wrong. Try it out on the practice patch.",
    )
    .step(walk)
    .step(catch)
    .step(release)
    .outro("Some species flash two or three times in a row, so press once for every flash. Your jar holds 6 fireflies, and named chords are saved in your journal. Pick Play to walk the home meadow.")
}
