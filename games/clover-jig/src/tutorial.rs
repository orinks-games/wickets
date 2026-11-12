//! The tutorial: the real practice room, where the tune waits for each
//! step.

use std::cell::Cell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::{Floor, Mode};
use crate::rules::Tally;
use crate::ID;

const SEED: u64 = 17;

/// About how long a line takes to say.
fn speech_secs(text: &str) -> f64 {
    0.4 + text.chars().count() as f64 / 15.0
}
/// Steps the lesson asks for.
pub const STEPS: u32 = 4;
/// Steps on the moving tune before the lesson goes on without a spot on.
pub const LIVE_STEPS: u32 = 8;

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
    let (l, r, p, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Hint),
    );

    let start = room.step(
        format!(
            "{} starts the dance after four beats of the bodhran drum. Start it now.",
            sentence_start(&p)
        ),
        |f: &Floor, _| {
            f.run
                .dance
                .music_started()
                .then(|| "Here comes the tune.".to_string())
        },
    );

    let dance = room.step(
        format!(
            "The caller says each step one beat before you dance. {} dances left, {r} dances right, and {p} hops. Here the music holds at each step, and its key is named. Dance {STEPS} steps.",
            sentence_start(&l)
        ),
        |f: &Floor, _| {
            (f.run.dance.tally().hits() >= STEPS || f.run.finished()).then(|| format!("{STEPS} steps danced."))
        },
    );

    // One stretch where the tune goes on, as in a real dance, so timing is
    // practised and the spot-on chime is heard.
    // Steps judged before this one began do not count toward it.
    let before = Rc::new(Cell::new(Tally::default()));
    let (live_room, at_start) = (room.clone(), Rc::clone(&before));
    let live_prompt = "Now the tune goes on, as in a real dance, after three drum beats. Step on the beat after each call. Spot on is 3 points.";
    // The tune waits while this is said, then counts in, so its first
    // call is not lost under the words.
    let page = format!("{STEPS} steps danced. {live_prompt}");
    let live = room
        .step(live_prompt, move |f: &Floor, _| {
            let (t, b) = (f.run.dance.tally(), before.get());
            let live = (t.good + t.close + t.missed) - (b.good + b.close + b.missed);
            if t.spot_on > b.spot_on {
                Some("That bright chime is 3 points.".to_string())
            } else if live >= LIVE_STEPS || f.run.finished() {
                Some("Right on the beat scores 3 points.".to_string())
            } else {
                None
            }
        })
        .start(move |ctx| {
            live_room.with_mut(|f| {
                f.run.dance.practice = false;
                at_start.set(f.run.dance.tally());
                f.count_in_after(ctx, speech_secs(&page));
            })
        });

    Lesson::new(
        ID,
        "Clover Jig",
        "You dance at a village ceili, and a caller calls your steps. Learn in the practice room",
    )
    .step(start)
    .step(dance)
    .step(live)
    .outro(format!(
        "Before a dance, {h} plays every step with its call. Play starts the first set."
    ))
}
