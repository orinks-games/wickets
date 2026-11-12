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
            "{}, {d}, {l} and {r} walk one step at a time, and {u} is always north. A firefly's tone marks where it drifts; quiet means far. Walk until it is in reach.",
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
            "{} catches a firefly in reach, but only while it glows. Press as its tone plays.",
            sentence_start(&p)
        ),
        // The meadow says what was caught; the lesson goes on without a
        // second line.
        |s: &EveningScreen, _| (s.evening.caught >= 1).then(String::new),
    );

    let release = patch.step(
        format!(
            "{} releases the fireflies from your jar. Their notes ring together as a chord. Set them free now.",
            sentence_start(&t)
        ),
        |s: &EveningScreen, _| (s.evening.releases >= 1).then(String::new),
    );

    Lesson::new(
        ID,
        "Firefly Meadow",
        "It is dusk, nothing here can go wrong, and you catch fireflies in a jar. Learn on the practice patch",
    )
    .step(walk)
    .step(catch)
    .step(release)
    .outro("Some species flash two or three times: press once for every flash. Your jar holds 6, and chords with names go in your journal. Play walks the home meadow.")
}
