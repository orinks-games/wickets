//! The tutorial: one quiet night with two drifters that never fire. Scan,
//! slide, shoot, end a turn, then the status and the route finder.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::colony::{share_as, Mode};
use crate::night::NightScreen;
use crate::rules::{Input, Run};
use crate::ID;

/// The practice night's seed: two drifters, one at the centre.
pub const SEED: u64 = 3;

/// A night 1 already under way, with its colony phase skipped.
pub fn night() -> Live<NightScreen> {
    let mut run = Run::new(SEED);
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
    let (left, right, fire, up, down, status, hint) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );
    let scan = night.step(
        format!(
            "Two drifters have come over the horizon, far out on row 6. The dome is under you, and the gun runs on a rail along its edge. Press {down} to scan to the nearest raider. {} goes back through them. Scanning costs nothing.",
            sentence_start(&up)
        ),
        |s: &NightScreen, _| {
            s.scanned.map(|_| {
                "Each raider is named with its column, its row and how many hits it has left. Its sound comes from where it stands."
                    .to_string()
            })
        },
    );
    let slide = night.step(
        format!(
            "Slide the gun under a raider with {left} or {right}. Each slide costs 1 of your 2 actions, and the column is named as you arrive."
        ),
        |s: &NightScreen, _| {
            let run = s.run();
            let under = run.raiders.iter().any(|r| r.at.col == run.gun);
            under.then(|| "Under it.".to_string())
        },
    );
    let shoot = night.step(
        format!(
            "Press {fire} to fire at the nearest raider in the gun's column. A drifter has 1 hit, so one shot brings it down."
        ),
        |s: &NightScreen, _| {
            (s.run().downed >= 1 && !s.busy()).then(|| {
                "Down. Its wreck leaves salvage, which the colony keeps. That was your second action, so the turn ended on its own and the raiders moved.".to_string()
            })
        },
    );
    let end = night.step(
        "Press Enter to end a turn with actions to spare. Each one left charges the shield by 5. Then the raiders move, each heard at its new sector, and your turn comes round again.",
        |s: &NightScreen, _| {
            (s.turns_ended >= 1 && !s.busy())
                .then(|| "The raiders have moved. Your turn again.".to_string())
        },
    );
    let status_step = night.step(
        format!(
            "Press {status} for the gun, the dome, the shield and your actions. Press it again straight away for every raider on the grid."
        ),
        |s: &NightScreen, _| s.status_asked.then(|| "That is the watch at a glance.".to_string()),
    );
    let route = night.step(
        format!(
            "Press {hint} to hear where the scanned raider is heading. Raiders find their own way round your walls and flak, and this tells you the next step. A scanner building tells you the whole route."
        ),
        |s: &NightScreen, _| s.hint_asked.then(|| "The route finder.".to_string()),
    );
    Lesson::new(
        ID,
        "Dustfall",
        "You hold a dome through twelve nights of raiders, one turn at a time. Between nights you spend what you have salvaged on walls, flak posts, mines and buildings. Learn the gun on a quiet night first",
    )
    .step(scan)
    .step(slide)
    .step(shoot)
    .step(end)
    .step(status_step)
    .step(route)
    .outro(format!(
        "On a real night the drifters fire back: a rising whine under a column means move the gun off it before you end the turn. When the last raider is gone, the colony phase opens, and what you build there changes the routes they take. {} fires, Enter ends the turn, {up} and {down} scan, and {hint} says where a raider is heading. Good luck on the watch.",
        sentence_start(&fire)
    ))
}
