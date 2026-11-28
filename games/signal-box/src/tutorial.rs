//! The tutorial: the five calls by ear, then a few trains in the practice
//! room, where there is no clock.

use hearken::text::{end_sentence, sentence_start};
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction, Step};

use crate::play::{Mode, Signal};
use crate::rules::{window_words, Call, START_WINDOW};
use crate::sound;
use crate::ID;

/// Trains the lesson sends through in the practice room.
const TRAINS: u32 = 5;

/// The practice room.
pub fn room() -> Live<Signal> {
    let mut signal = Signal::with_seed(Mode::Practice, 9);
    signal.lesson = true;
    Live::new(signal)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, room())
}

/// The tutorial on `room`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, room: Live<Signal>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (p, s, rep) = (
        k(ParlorAction::Primary),
        k(ParlorAction::Status),
        k(ParlorAction::Repeat),
    );
    let keys: Vec<String> = Call::ALL
        .iter()
        .map(|c| format!("{} {}", sentence_start(&k(c.action())), c.deed()))
        .collect();

    let calls = Step::each(
        format!(
            "There are five dispatcher calls, and each one has its own key. {}. Press each key to try out its call.",
            keys.join(". ")
        ),
        Call::ALL
            .into_iter()
            .map(|c| {
                let reaction: parlor_core::Reaction = Box::new(move |ctx| {
                    sound::play_call(ctx, c);
                    end_sentence(c.word())
                });
                (c.action(), reaction)
            })
            .collect(),
        "Those are all five.",
    );

    let trains = room.step(
        format!(
            "Now you are in the practice room, where there is no clock. {} opens the line. Match each dispatcher call with its key. {} repeats the call whenever you need it. Send {TRAINS} trains through.",
            sentence_start(&p),
            sentence_start(&rep)
        ),
        |sig: &Signal, _| {
            (sig.shift.passed >= TRAINS).then(|| format!("{TRAINS} trains through."))
        },
    );

    Lesson::new(
        ID,
        "Signal Box",
        "You are working the night shift in a country junction signal box, throwing levers and ringing back before each train reaches the points",
    )
    .step(calls)
    .step(trains)
    .outro(format!(
        "On a real shift, each call has a time window, starting at {} and shrinking as more trains pass. One wrong or late move derails the train and ends the shift. {} reads your shift status and score.",
        window_words(START_WINDOW),
        sentence_start(&s)
    ))
}
