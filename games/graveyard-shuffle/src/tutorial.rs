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
            "{} starts the dance after four knocks on a coffin lid. Start it now.",
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
            "Each monster makes its sound one beat before its move. The zombie groans: {d} drags. The skeleton rattles: {u} rattles back. The mummy moans: {p} unwraps. Here the music holds for each move, and its key is named. Dance {STEPS} moves."
        ),
        |f: &Floor, _| {
            (f.run.dance.tally().hits() >= STEPS || f.run.finished()).then(|| {
                "Right on the beat is spot on, for 3 points.".to_string()
            })
        },
    );

    let mut before = None;
    let in_time = room.step(
        "Now one in time: dance the next move on its beat, before the song stops for it.",
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
        "Doctor Volt's monster has started a dance craze, and you lead it. Learn in the practice room",
    )
    .step(start)
    .step(dance)
    .step(in_time)
    .outro(format!(
        "In a real dance the music does not stop. Before a dance, {h} plays every monster with its move. Play starts the first night."
    ))
}
