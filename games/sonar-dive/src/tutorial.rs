//! The tutorial: one chest on a practice dive, found by sonar and taken.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction, Rng};

use crate::rules::{wrap, Config, Dive, Expedition, Kind, Layout, BELL_REACH, V2};
use crate::screens::{DiveScreen, Mode};
use crate::ID;

/// How far off east the sub may point and still face the chest: within
/// this, a straight swim reaches it.
pub const NEAR: f32 = 8.0;
/// Where the chest lies: east of the bell.
pub const CHEST: V2 = V2 { x: 14.0, y: 0.0 };
/// Where the mine lies: south of the chest, beyond ticking range of the
/// way out and the way back.
pub const MINE: V2 = V2 { x: 14.0, y: -18.0 };

/// A practice dive with one chest, east of the bell.
pub fn sea() -> Live<DiveScreen> {
    let config = Config::practice();
    let mut exp = Expedition::new(config.clone(), Rng::seeded(1));
    exp.dive = Dive::with_objects(
        Layout::for_dive(1, &config),
        &config,
        config.hull,
        Vec::new(),
    );
    exp.dive.place(Kind::Chest, CHEST, 60);
    exp.dive.place(Kind::Mine, MINE, 0);
    exp.dive.lesson = true;
    Live::new(DiveScreen::new(exp, Mode::Practice, None))
}

fn dive(s: &DiveScreen) -> Option<&Dive> {
    s.expedition().map(|e| &e.dive)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, sea())
}

/// The tutorial on `sea`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, sea: Live<DiveScreen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p, b, s, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );

    let ping = sea.step(
        format!(
            "{} sends out a sonar ping. Everything in range echoes back from its true direction, and farther targets echo later. Try a ping now.",
            sentence_start(&p)
        ),
        |s: &DiveScreen, _| {
            dive(s)
                .is_some_and(|d| !d.returns.is_empty())
                .then(|| "The chest is on your right.".to_string())
        },
    );

    let turn = sea.step(
        format!(
            "Hold {l} or {r} to turn the sub. When you let go, your heading is spoken. Turn until the chest is straight ahead, and ping to check."
        ),
        |s: &DiveScreen, ctx| {
            let turning =
                ctx.action_held(ParlorAction::Left) || ctx.action_held(ParlorAction::Right);
            // Ahead means toward the chest from wherever the sub has got to.
            let d = dive(s)?;
            let want = (CHEST - d.pos).heading();
            let off = (wrap(d.heading - want + 180.0) - 180.0).abs();
            (!turning && off <= NEAR).then(|| "The chest is ahead of you.".to_string())
        },
    );

    let mut since = None;
    let mine = sea.step(
        "Mines warn you when you get close, and hitting one damages your hull. There is one sitting south of you. Ping to locate it.",
        move |s: &DiveScreen, _| {
            let d = dive(s)?;
            // A mine already bumped into is gone and can never answer.
            if !d.objects.iter().any(|o| o.kind == Kind::Mine && o.active) {
                return Some("You found that mine already, the hard way.".to_string());
            }
            let start = *since.get_or_insert(d.pings);
            // Once every echo of a new ping is back, so the buzz comes first.
            let heard = d.pings > start && !d.echoes_due();
            (heard && d.returns.iter().any(|r| r.kind == Kind::Mine))
                .then(|| "That was the mine.".to_string())
        },
    );

    let bell = sea.step(
        format!(
            "Your treasure is safe once you surface at the boat's bell, and {s} reports where it is. Head back to the bell."
        ),
        |s: &DiveScreen, _| {
            dive(s)
                .is_some_and(|d| {
                    // A chest taken close in still counts as a trip out.
                    (d.left_bell || d.carried > 0) && d.bell_distance() < BELL_REACH
                })
                .then(|| "At the bell.".to_string())
        },
    );

    let swim = sea.step(
        format!(
            "{} and {d} step the engine telegraph between slow astern, all stop, slow, half, and full ahead. Going faster burns more air. Throttle up and swim into the chest.",
            sentence_start(&u)
        ),
        |s: &DiveScreen, _| {
            dive(s)
                .is_some_and(|d| d.carried > 0)
                .then(|| "Chest taken.".to_string())
        },
    );

    Lesson::new(
        ID,
        "Sonar Dive",
        "You are at the helm of a small salvage submarine. This practice dive puts a single chest nearby so you can get a feel for the sonar and controls",
    )
    .step(ping)
    .step(turn)
    .step(swim)
    .step(mine)
    .step(bell)
    .outro(format!(
        "On a real dive, make sure you surface before your air runs out. {} points you to the nearest treasure, and {b} drops a marker buoy. Practice dive gives you endless air.",
        sentence_start(&h)
    ))
}
