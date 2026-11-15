//! The tutorial: the real practice street, one skill at a time.

use std::cell::Cell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::dance::{DanceScreen, Launch, Mode};
use crate::rules::{Dance, Mark, Street, COMBO_EVERY};
use crate::streets::PRACTICE;
use crate::ID;

/// The lesson's street: the practice street, long enough to learn on.
const LESSON_STREET: Street = Street {
    bars: 64,
    ..PRACTICE
};
const SEED: u64 = 8;
/// Beats in a row the first step asks for.
pub const FOLLOW: u32 = 4;

/// The practice street the lesson runs.
pub fn street() -> Live<DanceScreen> {
    let mut screen = DanceScreen::new(Launch {
        mode: Mode::Practice,
        seed: SEED,
    });
    screen.dance = Dance::new(LESSON_STREET, SEED);
    // The head starts away from the pearl, and away from the lane it moves
    // to first, so following it always takes a step.
    let pearl = screen.dance.pearl;
    let first_move = screen.dance.plan.marks.iter().find_map(|m| match m.mark {
        Mark::Pearl { lane } => Some(lane),
        _ => None,
    });
    screen.dance.lane = (0..LESSON_STREET.lanes)
        .find(|&l| l != pearl && Some(l) != first_move)
        .unwrap_or(if pearl == 0 { 1 } else { pearl - 1 });
    screen.lesson = true;
    Live::new(screen)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, street())
}

/// The tutorial on `street`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, street: Live<DanceScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, p, t) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
    );

    let follow = street.step(
        format!(
            "{} and {r} move your head one lane. The pearl moves from lane to lane, and {} names its lane. Follow it for {FOLLOW} beats in a row.",
            sentence_start(&l),
            sentence_start(&t)
        ),
        |s: &DanceScreen, _| {
            (s.steps > 0 && s.dance.combo >= FOLLOW).then(|| {
                format!(
                    "{FOLLOW} beats in a row. Each beat in the pearl's lane scores, and every {COMBO_EVERY} bring a cheer."
                )
            })
        },
    );

    // Only a rear after this step begins counts: one on an early gong,
    // before the step was taught, would end it as soon as it opened.
    let before = Rc::new(Cell::new(0));
    let (at_start, gong_street) = (Rc::clone(&before), street.clone());
    let gong = street
        .step(
            format!(
                "Now and then a gong strikes. {} right as the gong strikes rears the dragon up. Keep following, and rear on the next gong.",
                sentence_start(&p)
            ),
            move |s: &DanceScreen, _| {
                (s.dance.perfect + s.dance.good > before.get())
                    .then(|| "A rear on the gong.".to_string())
            },
        )
        .start(move |_ctx| {
            gong_street.with(|s| at_start.set(s.dance.perfect + s.dance.good))
        });

    Lesson::new(
        ID,
        "Dragon Dance",
        "You are the head of a long paper dragon in a street parade, and the dragon follows where you lead. Learn on the practice street",
    )
    .step(follow)
    .step(gong)
    .outro(format!(
        "Be in a red envelope's lane to catch it. Keep out of the firecrackers' lane. {} names what is coming. Play starts the first street.",
        sentence_start(&u)
    ))
}
