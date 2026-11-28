//! The tutorial: crumbs and a freeze in the practice pantry, where the one
//! slow cat costs no whiskers.

use std::cell::Cell;
use std::rc::Rc;

use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::{frozen_words, Mode, Play};
use crate::rules::CRUMB_POINTS;
use crate::ID;

/// Crumbs the lesson eats before it moves on to the cat.
const CRUMBS: u32 = 3;

/// The practice pantry.
pub fn pantry() -> Live<Play> {
    Live::new(Play::with_seed(Mode::Practice, 5))
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, pantry())
}

/// The tutorial on `pantry`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, pantry: Live<Play>) -> Lesson {
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
    // How long the cat wanders off the scent while the freeze step is read.
    const LESSON_GRACE_MS: u32 = 8000;
    // Freezes left when the freeze step opens: one used in the crumb step
    // must not pass it.
    let freezes = Rc::new(Cell::new(0));
    let at_start = Rc::clone(&freezes);

    let crumbs = pantry.step(
        format!(
            "Move one cell at a time with {l}, {r}, {u}, and {d}. Press {p} to sniff out the nearest crumbs, or {h} to point toward a safe crumb. Go eat {CRUMBS} crumbs."
        ),
        |p: &Play, _| {
            (p.run.eaten >= CRUMBS).then(|| {
                format!("{CRUMBS} crumbs eaten, worth {CRUMB_POINTS} points each. Clear every crumb to finish a pantry.")
            })
        },
    );

    let freeze = pantry.step(
        format!(
            "Track the cat as it prowls nearby; a short warning means it is within 2 steps. In a real run, getting caught costs a whisker. Press {t} to freeze in place for 3 seconds so the nearest cat loses your scent. Try freezing now."
        ),
        move |p: &Play, _| {
            // The game keeps quiet on this step, so the line is said once.
            (p.run.freezes < at_start.get()).then(|| frozen_words(p.run.freezes))
        },
    );
    let watch = pantry.clone();
    let freeze = freeze.start(move |_| {
        watch.with_mut(|p| {
            p.lesson_says_freeze = true;
            // Freezes spent while eating would leave nothing to practise.
            p.run.freezes = p.run.freezes.max(1);
            // Send the cat home, off the scent while the step is read, so
            // the lesson is heard before it can arrive.
            let mouse = p.run.mouse;
            for cat in &mut p.run.cats {
                if cat.den != mouse {
                    cat.pos = cat.den;
                    cat.heading = None;
                }
                cat.lost = LESSON_GRACE_MS.div_ceil(crate::rules::STEP_MS);
            }
            freezes.set(p.run.freezes);
        })
    });

    Lesson::new(
        ID,
        "Pantry Mouse",
        "You are a mouse sneaking through a farmhouse pantry at night, hunting for crumbs while keeping out of the cats' reach. Try the practice pantry first, where getting caught costs nothing",
    )
    .step(crumbs)
    .step(freeze)
    .outro(format!(
        "The ginger cat chases straight after you, while the tabby wanders until you get close. Press {s} whenever you want to check where every cat is."
    ))
}
