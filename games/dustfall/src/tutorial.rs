//! The tutorial: one quiet night at a third of the pace, with two drifters
//! in two waves that never fire. Scan, slide, shoot, then the status and
//! the sky.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::colony::{share_as, Mode};
use crate::night::NightScreen;
use crate::rules::{Input, Run};
use crate::ID;

/// The practice night's seed.
pub const SEED: u64 = 3;

/// A night 1 already under way at the practice pace, with its colony
/// phase skipped.
pub fn night() -> Live<NightScreen> {
    let mut run = Run::practice(SEED);
    run.take_events();
    run.apply(Input::Ready).expect("the practice night starts");
    Live::new(NightScreen::new(share_as(run, Mode::Practice)))
}

pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, night())
}

/// The tutorial on `night`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, night: Live<NightScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (left, right, fire, missile, up, down, status, hint) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );
    let scan = night.step(
        format!(
            "The dome is under you, and the gun runs on a rail along its edge. The raiders come down five lanes, far left to far right, and the first wave has just been called. Press {down} to scan to the nearest raider once you hear its engine. {} goes back through them. Scanning costs nothing.",
            sentence_start(&up)
        ),
        |s: &NightScreen, _| {
            s.scanned.map(|_| {
                "Each raider is named with its lane, its range and the hits it has left. Its engine sounds from where it is, and it is falling toward you."
                    .to_string()
            })
        },
    );
    let slide = night.step(
        format!(
            "Slide the gun into its lane with {left} or {right}. The lane is named as you arrive, and a lock tone starts when a raider is in the gun's lane. The tone pulses faster as it nears."
        ),
        |s: &NightScreen, _| {
            let run = s.run();
            let under = run.raiders.iter().any(|r| r.lane == run.gun);
            under.then(|| "Locked on.".to_string())
        },
    );
    let shoot = night.step(
        format!(
            "Press {fire} to fire up the gun's lane. A drifter has 1 hit, so one shot brings it down. The gun needs a moment to reload between shots."
        ),
        |s: &NightScreen, _| {
            (s.run().downed >= 1).then(|| {
                "Down. Its wreck leaves salvage, which the colony keeps. Raiders come in waves: once the sky is clear, the next wave is called.".to_string()
            })
        },
    );
    let status_step = night.step(
        format!(
            "Press {status} for the gun, the dome, the shield and the wave. Press it again straight away for every raider in the sky."
        ),
        |s: &NightScreen, _| s.status_asked.then(|| "That is the watch at a glance.".to_string()),
    );
    let sky = night.step(
        format!(
            "Press {hint} to hear the sky lane by lane, and what comes next. A scanner building tells it which lane the next raider takes."
        ),
        |s: &NightScreen, _| s.hint_asked.then(|| "The sky, read out.".to_string()),
    );
    Lesson::new(
        ID,
        "Dustfall",
        "You hold a dome through twelve nights of raiders, who come down five lanes in waves. Between nights you spend what you have salvaged on buildings, mines and missiles. Learn the gun on a quiet night first",
    )
    .step(scan)
    .step(slide)
    .step(shoot)
    .step(status_step)
    .step(sky)
    .outro(format!(
        "On a real night the raiders fire back: a rising whine in a lane means slide the gun out of it before the shot lands. When the last wave is down, the colony phase opens, and what you build there shapes the nights to come. {} fires, {missile} fires a missile at the nearest raider, {up} and {down} scan, {status} is the status, and {hint} reads the sky. Good luck on the watch.",
        sentence_start(&fire)
    ))
}
