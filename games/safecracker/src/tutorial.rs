//! The tutorial: one wheel on a practice dial, found by ear and set.

use std::cell::Cell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Parlor, ParlorAction, Reply, Step};

use crate::sound;
use crate::ID;

/// The practice wheel's secret number.
const SECRET: i32 = 23;

/// The click a number gives as the dial passes it.
fn click(ctx: &mut Ctx<Parlor>, n: i32) {
    let key = if n == 0 {
        sound::ZERO
    } else if n % 10 == 0 {
        sound::DETENT
    } else {
        sound::CLICK
    };
    sound::play(ctx, key);
    if n == SECRET {
        sound::play(ctx, sound::CLUNK);
    }
}

/// One number down or up, wrapping at 100.
fn turn(ctx: &mut Ctx<Parlor>, dial: &Cell<i32>, by: i32) -> i32 {
    let n = (dial.get() + by).rem_euclid(100);
    dial.set(n);
    click(ctx, n);
    n
}

/// Ten numbers at once, with a ratchet. True if the tumbler clunked on
/// the way.
fn spin(ctx: &mut Ctx<Parlor>, dial: &Cell<i32>, up: bool) -> bool {
    let from = dial.get();
    let passed = (1..=10).any(|i| (from + if up { i } else { -i }).rem_euclid(100) == SECRET);
    dial.set((from + if up { 10 } else { -10 }).rem_euclid(100));
    sound::play(ctx, sound::SPIN);
    if passed {
        sound::play(ctx, sound::CLUNK);
    }
    passed
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p, t) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
    );
    let dial = Rc::new(Cell::new(0));

    let dl = Rc::clone(&dial);
    let clicks = Step::new(
        format!(
            "The dial runs from 0 to 99. {} turns it one number down and {r} one number up, with a soft click each. Every tenth number gives a firmer double click. Turn up to 5.",
            sentence_start(&l)
        ),
        move |ctx, a| {
            let n = match a {
                ParlorAction::Left => turn(ctx, &dl, -1),
                ParlorAction::Right => turn(ctx, &dl, 1),
                _ => return Reply::Nudge,
            };
            if n == 5 {
                Reply::done("5")
            } else {
                Reply::say(n.to_string())
            }
        },
    );

    let dl = Rc::clone(&dial);
    let read = Step::press(
        format!(
            "{} speaks the number under the mark, without turning. Press it now.",
            sentence_start(&t)
        ),
        ParlorAction::Secondary,
        move |_| format!("{}.", dl.get()),
    );

    let dl = Rc::clone(&dial);
    let spins = Step::new(
        format!(
            "{} spins the dial ten numbers up and {d} ten down, with a quick ratchet. The tumbler clunk marks a wheel's number. Spin up until it clunks.",
            sentence_start(&u)
        ),
        move |ctx, a| {
            let up = match a {
                ParlorAction::Up => true,
                ParlorAction::Down => false,
                ParlorAction::Left | ParlorAction::Right => {
                    let n = turn(ctx, &dl, if a == ParlorAction::Left { -1 } else { 1 });
                    return Reply::say(n.to_string());
                }
                _ => return Reply::Nudge,
            };
            if spin(ctx, &dl, up) {
                Reply::done(format!(
                    "{}. That was the clunk: the number is in the ten you just spun past.",
                    dl.get()
                ))
            } else {
                Reply::say(format!("{}. No clunk.", dl.get()))
            }
        },
    );

    let dl = Rc::clone(&dial);
    let back = Step::new(
        format!("Now step back one number at a time with {l} until it clunks again."),
        move |ctx, a| {
            let n = match a {
                ParlorAction::Left => turn(ctx, &dl, -1),
                ParlorAction::Right => turn(ctx, &dl, 1),
                ParlorAction::Up => {
                    spin(ctx, &dl, true);
                    dl.get()
                }
                ParlorAction::Down => {
                    spin(ctx, &dl, false);
                    dl.get()
                }
                ParlorAction::Secondary => return Reply::say(format!("{}.", dl.get())),
                _ => return Reply::Nudge,
            };
            if n == SECRET {
                Reply::done(format!("{n}. Clunk. The number is {n}."))
            } else {
                Reply::say(n.to_string())
            }
        },
    );

    let dl = Rc::clone(&dial);
    let set = Step::new(
        format!(
            "{} sets the wheel on the number under the mark. Set it now.",
            sentence_start(&p)
        ),
        move |ctx, a| match a {
            ParlorAction::Primary if dl.get() == SECRET => {
                sound::play(ctx, sound::SET);
                sound::play(ctx, sound::OPEN);
                Reply::done("Wheel set, and the safe swings open")
            }
            ParlorAction::Primary => {
                sound::play(ctx, sound::SPRING);
                Reply::say(format!(
                    "The wheel sprang back: {} is not the number. In a real safe that is a strike, and three strikes jam it. Find the clunk at {SECRET} first.",
                    dl.get()
                ))
            }
            ParlorAction::Left => Reply::say(turn(ctx, &dl, -1).to_string()),
            ParlorAction::Right => Reply::say(turn(ctx, &dl, 1).to_string()),
            _ => Reply::Nudge,
        },
    );

    Lesson::new(
        ID,
        "Safecracker",
        "Turn the dial and listen for the tumbler to find each wheel's number. Learn on a practice dial with one wheel and no time limit",
    )
    .step(clicks)
    .step(read)
    .step(spins)
    .step(back)
    .step(set)
    .outro(format!("In a real safe, turning the dial does not speak each number. Press {t} to hear the current number. Real safes have two to five wheels, and later ones add false gates and a night guard. Play starts with the office lockbox, and Practice dial gives endless single wheels."))
}
