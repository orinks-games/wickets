//! The tutorial: the real practice room, where the song waits for each
//! monster's move.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::{Floor, Mode};
use crate::ID;

/// Brings up the zombie, the skeleton and the mummy, in that order, first.
pub const SEED: u64 = 12;
/// Steps the lesson asks for while the song waits.
pub const STEPS: u32 = 4;

/// The practice room the lesson runs.
pub fn room(ctx: &mut Ctx<Parlor>) -> Live<Floor> {
    Live::new(Floor::with_seed(ctx, Mode::Practice(0), SEED))
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let room = room(ctx);
    lesson_on(ctx, room)
}

/// The tutorial on `room`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, room: Live<Floor>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (u, d, p, h) = (
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Hint),
    );

    let start = room.step(
        format!(
            "{} starts the song after four count-in beats. Press it whenever you are ready.",
            sentence_start(&p)
        ),
        |f: &Floor, _| {
            f.run
                .dance
                .music_started()
                .then(|| "Here comes the band.".to_string())
        },
    );

    let dance = room.step(
        format!(
            "Each monster steps up one beat before its move. {} dances the zombie's drag, {u} dances the skeleton's rattle, and {p} dances the mummy's unwrap. In the practice room, the music pauses for each move and names its key. Try dancing {STEPS} moves.",
            sentence_start(&d)
        ),
        |f: &Floor, _| {
            (f.run.dance.tally().hits() >= STEPS || f.run.finished()).then(|| {
                "Right on the beat is spot on, for 3 points.".to_string()
            })
        },
    );

    let mut before = None;
    let in_time = room.step(
        "Now try one in time: hit the next move right on its beat before the music pauses for it.",
        move |f: &Floor, _| {
            let t = f.run.dance.tally();
            let on_beat = t.hits() - t.waited;
            let start = *before.get_or_insert(on_beat);
            if on_beat > start {
                Some("In time.".to_string())
            } else {
                f.run
                    .finished()
                    .then(|| "That was the last move.".to_string())
            }
        },
    );

    Lesson::new(
        ID,
        "Graveyard Shuffle",
        "Doctor Volt's monster has sparked a midnight dance craze, and you are leading the graveyard floor. You will warm up in the practice room first, where the music pauses for every move.",
    )
    .step(start)
    .step(dance)
    .step(in_time)
    .outro(format!(
        "In a real night, the music never stops. Before any song starts, press {h} to preview every monster and its move. Choose Play on the title when you are ready for the first night."
    ))
}
