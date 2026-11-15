//! The tutorial: the five training rides in a row, on the player's bike.
//! Each ride's page becomes its step, and the ride itself is the test.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::bike::Bike;
use crate::progress::Choices;
use crate::ride::Ride;
use crate::training::{self, LESSONS};
use crate::ID;

/// Training ride `n`, run inside the tutorial.
pub fn ride(n: u32, bike: Bike) -> Live<Ride> {
    Live::new(Ride::for_lesson(n, bike))
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let bike = Choices::load(ctx).bike;
    let rides: Vec<Live<Ride>> = (1..=LESSONS).map(|n| ride(n, bike)).collect();
    lesson_on(ctx, rides)
}

/// The tutorial on `rides`, one a lesson, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, rides: Vec<Live<Ride>>) -> Lesson {
    let s = ctx.control_name(ParlorAction::Status);
    let h = ctx.control_name(ParlorAction::Hint);
    let mut lesson = Lesson::new(
        ID,
        "Freewheel",
        "You are a new bike courier in Courier City. Parked cars line your right, then comes the bike lane, then the traffic lane. Learn on a quiet training street, one skill at a time",
    );
    for (i, live) in rides.iter().enumerate() {
        let n = i as u32 + 1;
        // "Lesson 1 of 5: pedal and coast. ..." without its heading.
        let page = training::page(ctx, n);
        let text = page.split_once(": ").map_or(page.as_str(), |(_, t)| t);
        let step = live.step(sentence_start(text), |r: &Ride, _| {
            r.passed.then(|| "Passed.".to_string())
        });
        lesson = lesson.step(step);
    }
    lesson.outro(format!(
        "In Downtown, cars come up behind you, and junctions name your turn. {} names the next danger, then your speed and lane, and {h} says how to deal with the next hazard. Ride them again from Practice lessons.",
        sentence_start(&s)
    ))
}
