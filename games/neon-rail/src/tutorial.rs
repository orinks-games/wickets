//! The tutorial: lanes, the four tools, and four kinds of traffic, each
//! met in the player's own lane. The guided run then teaches the rest in
//! the corridor.

use std::cell::Cell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, Vec3};
use parlor_core::{Lesson, Parlor, ParlorAction, Reply, Step};

use crate::cues;
use crate::screens::lane_x;
use crate::ID;

const LANES: [&str; 3] = ["Left lane", "Center lane", "Right lane"];

/// Plays `key` from `lane`, a few metres ahead of a skimmer in `at`.
fn ahead(ctx: &mut Ctx<Parlor>, key: &str, lane: usize, at: usize) {
    let pos = Vec3::new(lane_x(lane) - lane_x(at), 3.0, 0.0);
    ctx.audio.play_at(key, pos, Bus::EFFECTS);
}

/// Shifts one lane, as the rail does: a snap, or the edge.
fn shift(ctx: &mut Ctx<Parlor>, lane: &Cell<usize>, a: ParlorAction) -> Option<String> {
    let at = lane.get();
    let to = match a {
        ParlorAction::Left if at > 0 => at - 1,
        ParlorAction::Right if at < 2 => at + 1,
        ParlorAction::Left | ParlorAction::Right => {
            ctx.audio.play(cues::EDGE, Bus::EFFECTS);
            return Some("Rail edge.".to_string());
        }
        _ => return None,
    };
    lane.set(to);
    ctx.audio.play(cues::SHIFT, Bus::EFFECTS);
    Some(format!("{}.", LANES[to]))
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, boost, fire, slow, prio) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Down),
        k(ParlorAction::Up),
    );
    let lane = Rc::new(Cell::new(1));

    let ln = Rc::clone(&lane);
    let mut been_left = false;
    let lanes = Step::new(
        format!(
            "Three lanes; you start in the center. {} shifts left and {r} shifts right, with a magnetic snap. Shift left, then all the way right.",
            sentence_start(&l)
        ),
        move |ctx, a| {
            let Some(line) = shift(ctx, &ln, a) else {
                return Reply::Nudge;
            };
            been_left |= ln.get() == 0;
            if been_left && ln.get() == 2 {
                Reply::done(line)
            } else {
                Reply::say(line)
            }
        },
    );

    let tools = Step::each(
        format!(
            "{} boosts. {} fires the rail gun. {} uses a slowdown pulse. {} uses priority override. Try each tool.",
            sentence_start(&boost), sentence_start(&fire), sentence_start(&slow), sentence_start(&prio)
        ),
        vec![
            (
                ParlorAction::Primary,
                Box::new(|ctx: &mut Ctx<Parlor>| {
                    ctx.audio.play(cues::BOOST, Bus::EFFECTS);
                    "Boost. It smashes through blockers, then needs time to recharge.".to_string()
                }),
            ),
            (
                ParlorAction::Secondary,
                Box::new(|ctx: &mut Ctx<Parlor>| {
                    ctx.audio.play(cues::RAILGUN, Bus::EFFECTS);
                    "Rail gun. It locks onto drones and unstable vehicles in the last second of their approach."
                        .to_string()
                }),
            ),
            (
                ParlorAction::Down,
                Box::new(|ctx: &mut Ctx<Parlor>| {
                    ctx.audio.play(cues::SLOWDOWN, Bus::EFFECTS);
                    "Slowdown pulse. It eases fast passers and priority traffic for a few seconds."
                        .to_string()
                }),
            ),
            (
                ParlorAction::Up,
                Box::new(|ctx: &mut Ctx<Parlor>| {
                    ctx.audio.play(cues::OVERRIDE, Bus::EFFECTS);
                    "Priority override. It holds priority traffic for you, whatever lane you are in.".to_string()
                }),
            ),
        ],
        "Slowdown pulses and priority overrides have limited charges. Boost recharges between uses.",
    );

    let ln = Rc::clone(&lane);
    let ln2 = Rc::clone(&lane);
    let flow = Step::new(
        "That steady hum on your left is a flow vehicle. Flow traffic is safe to meet: be in its lane when it arrives. Shift into its lane.",
        move |ctx, a| {
            let Some(line) = shift(ctx, &ln, a) else {
                return Reply::Nudge;
            };
            if ln.get() == 0 {
                ctx.audio.play(cues::CLEAN, Bus::EFFECTS);
                Reply::done(format!("{line} Clean pass: you met it in its lane"))
            } else {
                Reply::say(line)
            }
        },
    )
    .start(move |ctx| ahead(ctx, cues::FLOW, 0, ln2.get()));

    // The flow step leaves the skimmer in the left lane, so the fast
    // passer comes on the far side: two shifts to join it, or one pulse.
    let ln = Rc::clone(&lane);
    let ln2 = Rc::clone(&lane);
    let fast = Step::new(
        format!(
            "A fast passer on your right: a short warning, then it tears past. Get into its lane, or fire the slowdown pulse with {slow}. Do either now."
        ),
        move |ctx, a| {
            const CLEARED: &str = "Cleared. Meet a fast passer in its lane or use a slowdown pulse.";
            if a == ParlorAction::Down {
                ctx.audio.play(cues::SLOWDOWN, Bus::EFFECTS);
                ctx.audio.play(cues::CLEAN, Bus::EFFECTS);
                return Reply::done(CLEARED);
            }
            let Some(line) = shift(ctx, &ln, a) else {
                return Reply::Nudge;
            };
            if ln.get() == 2 {
                ctx.audio.play(cues::CLEAN, Bus::EFFECTS);
                Reply::done(format!("{line} {CLEARED}"))
            } else {
                Reply::say(line)
            }
        },
    )
    .start(move |ctx| ahead(ctx, cues::FAST, 2, ln2.get()));

    // Debris comes in whatever lane the fast passer left you in.
    let ln = Rc::clone(&lane);
    let ln2 = Rc::clone(&lane);
    let hit = Rc::new(Cell::new(0));
    let hit2 = Rc::clone(&hit);
    let debris = Step::new(
        "Debris is coming straight ahead in your lane. Shift to another lane to avoid it.",
        move |ctx, a| {
            let Some(line) = shift(ctx, &ln, a) else {
                return Reply::Nudge;
            };
            if ln.get() != hit.get() {
                ctx.audio.play(cues::PASS, Bus::EFFECTS);
                Reply::done(format!("{line} Dodged: it went by in another lane"))
            } else {
                Reply::say(line)
            }
        },
    )
    .start(move |ctx| {
        hit2.set(ln2.get());
        ahead(ctx, cues::DEBRIS, ln2.get(), ln2.get());
    });

    let blockade = Step::new(
        format!(
            "Last, a full blockade. It fills all three lanes, so you cannot dodge it. In a run, a bright chime sounds just before it hits: boost then. Boost now with {boost}."
        ),
        |ctx, a| match a {
            ParlorAction::Primary => {
                ctx.audio.play(cues::SMASH, Bus::EFFECTS);
                Reply::done("Breached")
            }
            ParlorAction::Left | ParlorAction::Right => {
                ctx.audio.play(cues::EDGE, Bus::EFFECTS);
                Reply::say("No lane is clear. Boost through it.")
            }
            _ => Reply::Nudge,
        },
    )
    .start(|ctx| {
        ctx.audio.play(cues::BLOCKADE, Bus::EFFECTS);
        ctx.audio.play(cues::BOOST_CHIME, Bus::EFFECTS);
    });

    Lesson::new(
        ID,
        "Neon Rail",
        "You fly a stolen combat skimmer through the dead transit lines of Black Sky City. Learn the skimmer here, at a standstill",
    )
    .step(lanes)
    .step(tools)
    .step(flow)
    .step(fast)
    .step(debris)
    .step(blockade)
    .outro("Too many misses raise stress, and at 100 the network collapses. Practice run flies the guided route, where each new sound is explained as it arrives, and your first Play flies it too.")
}
