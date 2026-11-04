//! Training: five short lessons, each a page of explanation and then a
//! test ride on a quiet street.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Pages, Parlor, ParlorAction};

use crate::bike::Bike;
use crate::course::{Course, Hazard, HazardKind, DOOR_EDGE, PARKED_EDGE};
use crate::ride::Ride;

/// How many lessons there are.
pub const LESSONS: u32 = 5;
/// Riding this far in the door lesson, with no door crash, passes it.
pub const DOORS_DONE: f64 = 165.0;

/// The quiet street a lesson rides on.
pub fn course(n: u32) -> Course {
    let street = Course::empty("Training street", 400.0);
    match n {
        4 => street
            .with(Hazard {
                kind: HazardKind::Door,
                s: 70.0,
                x0: PARKED_EDGE,
                x1: DOOR_EDGE,
            })
            .with(Hazard {
                kind: HazardKind::Door,
                s: 150.0,
                x0: PARKED_EDGE,
                x1: DOOR_EDGE,
            }),
        5 => street
            .with(Hazard {
                kind: HazardKind::Pothole,
                s: 60.0,
                x0: 2.45,
                x1: 3.15,
            })
            .with(Hazard {
                kind: HazardKind::Pothole,
                s: 120.0,
                x0: 2.45,
                x1: 3.15,
            }),
        _ => street,
    }
}

/// The speed the brake lesson asks for before the stop, metres a second.
pub const BRAKE_FROM: f64 = 5.0;

/// The lesson's page, read before it starts.
pub fn page(ctx: &Ctx<Parlor>, n: u32) -> String {
    let k = |a| ctx.control_name(a);
    let up = k(ParlorAction::Up);
    let down = k(ParlorAction::Down);
    let left = k(ParlorAction::Left);
    let right = k(ParlorAction::Right);
    let p = k(ParlorAction::Primary);
    match n {
        1 => format!(
            "Lesson 1 of {LESSONS}: pedal and coast. Hold {up} to pedal, and the chain whirrs. Let go to coast, and the freewheel ticks, faster the faster you go. Pedal up to {}, then coast for 2 seconds.",
            parlor_core::units::per_hour(20.0)
        ),
        2 => format!(
            "Lesson 2 of {LESSONS}: brake. Hold {down} to brake. The brakes build over a moment, so start early. Pedal up to {}, then brake to a stop.",
            parlor_core::units::per_hour(BRAKE_FROM * 3.6)
        ),
        3 => format!(
            "Lesson 3 of {LESSONS}: steer. {} and {right} slide you across the road while you hold them. A short thrum means you crossed the line between the bike lane and the traffic lane. Get moving, steer out into the traffic lane, then back into the bike lane.",
            sentence_start(&left)
        ),
        4 => format!(
            "Lesson 4 of {LESSONS}: the door zone. A sharp click from a parked car means a door is about to open. {} steers left: move out before it does. Pass both doors.",
            sentence_start(&left)
        ),
        _ => format!(
            "Lesson 5 of {LESSONS}: hop. {} hops while you are moving. A tick ahead means a pothole, and it ticks faster as it nears. A sharp double beep means it is in your line: hop then. Two potholes lie in the bike lane: hop one of them.",
            sentence_start(&p)
        ),
    }
}

/// The line said as the lesson's ride starts.
pub fn goal(ctx: &Ctx<Parlor>, n: u32) -> String {
    let k = |a| ctx.control_name(a);
    match n {
        1 => format!("Lesson 1. Hold {} to pedal.", k(ParlorAction::Up)),
        2 => format!("Lesson 2. Hold {} to get up to speed.", k(ParlorAction::Up)),
        3 => format!(
            "Lesson 3. Get moving, then hold {} to steer into the traffic lane.",
            k(ParlorAction::Left)
        ),
        4 => "Lesson 4. Ride on and listen for the click of a door.".to_string(),
        _ => format!(
            "Lesson 5. Ride on in the bike lane and press {} on the double beep.",
            k(ParlorAction::Primary)
        ),
    }
}

/// Training from the first lesson: its page, then its ride. Escape on the
/// page goes back without starting.
pub fn start(ctx: &Ctx<Parlor>, bike: Bike) -> Pages {
    Pages::briefing([page(ctx, 1)]).then(move |ctx| ctx.replace(Ride::training(1, bike)))
}

/// The pages after the last lesson.
pub fn done_pages(_ctx: &Ctx<Parlor>) -> Vec<String> {
    vec![
        "Training complete: all five lessons passed. You can pedal, coast, brake, steer, keep out of the door zone and hop.".to_string(),
        "Next, try a time trial. Downtown adds cars from behind, curbs, ramps and junction turns. Hint tells you what to do about whatever comes next.".to_string(),
    ]
}
