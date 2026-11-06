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
            "Five calls, five keys. {}. Press each to hear its call.",
            keys.join(". ")
        ),
        Call::ALL
            .into_iter()
            .map(|c| {
                let reaction: parlor_core::Reaction = Box::new(move |ctx| {
                    sound::play_call(ctx, c);
                    end_sentence(&format!("{}: {}", c.word(), c.sound_words()))
                });
                (c.action(), reaction)
            })
            .collect(),
        "Those are all five.",
    );

    let trains = room.step(
        format!(
            "Now the practice room, with no clock. {} opens the line. Listen for each call, and answer it before the train reaches the points. {} says the call again. Send {TRAINS} trains through.",
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
        "You work the signal box on a country junction at night, answering the dispatcher's calls before each train arrives",
    )
    .step(calls)
    .step(trains)
    .outro(format!(
        "On a shift each call has a time window, {} at first and shrinking as you go. A wrong or late answer derails the train and ends the shift. {} speaks the shift and your score.",
        window_words(START_WINDOW),
        sentence_start(&s)
    ))
}
