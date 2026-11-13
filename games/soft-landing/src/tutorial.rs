//! The tutorial: four short lessons on the practice pad, with endless
//! fuel, ending in one guided landing.
//!
//! 1. The fall: start the descent and hear the tone climb.
//! 2. The engine: hold it down to the safe speed, then let go.
//! 3. The hint: ask when to burn.
//! 4. The landing: the lesson says when to burn and when to let go, and
//!    after a crash, what to change.

use std::cell::Cell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::play::{Descent, Mode};
use crate::rules::{advice, speed_words, Advice, Band, Outcome, SURVEY};
use crate::ID;

/// The falling speed that ends lesson 1: well past the safe speed, so the
/// tone has climbed.
pub const FAST: f64 = 8.0;

/// Below this share of the safe speed, a burn has done its work: the
/// landing lesson says to let go.
pub const EASED: f64 = 0.7;

/// Metres between call-outs in the tutorial: fewer than the practice
/// pad's, so the lessons' lines are heard.
pub const CALL_EVERY: u32 = 50;

/// The practice pad. A crash there starts over from the top by itself.
pub fn pad() -> Live<Descent> {
    let mut d = Descent::new(Mode::Practice);
    d.flight.set_call_every(CALL_EVERY);
    // Too fast waits for a lesson's line instead of cutting it off.
    d.lesson_says.insert("prompts");
    Live::new(d)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, pad())
}

/// What the landing lesson has said on this descent.
#[derive(Default)]
struct Coach {
    burn: bool,
    ease: bool,
    crash: bool,
}

/// The tutorial on `pad`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, pad: Live<Descent>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (p, t, s, h) = (
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );
    let moon = SURVEY[0];
    let safe = speed_words(moon.safe);

    let fall = pad.step(
        format!(
            "Lesson 1 of 4, the fall. You wait {} above the pad. {} starts the descent. Let go and fall freely.",
            parlor_core::units::length(moon.height),
            sentence_start(&p)
        ),
        |d: &Descent, _| {
            let f = &d.flight;
            (f.started() && !f.engine && f.speed() >= FAST).then(|| {
                format!("Falling {}. Too fast to land.", speed_words(f.speed()))
            })
        },
    );

    let engine = pad.step(
        format!(
            "Lesson 2 of 4, the engine. Hold {p} to slow down. Let go at Safe speed."
        ),
        |d: &Descent, _| {
            let f = &d.flight;
            (f.started() && !f.finished() && !f.engine && f.band == Some(Band::Safe))
                .then(|| "Once you let go, gravity speeds you up again.".to_string())
        },
    );

    // Hints asked before this lesson began, set as it begins.
    let asked = Rc::new(Cell::new(0));
    let (base, watch) = (Rc::clone(&asked), pad.clone());
    let hint = pad
        .step(
            format!(
                "Lesson 3 of 4, the hint. {} says when to burn. Press it now.",
                sentence_start(&h)
            ),
            move |d: &Descent, ctx| {
                // The lesson's next line would cut the game's hint off, so it
                // carries the hint itself.
                (d.hints > asked.get()).then(|| d.hint_words(ctx))
            },
        )
        .start(move |_| {
            base.set(watch.with(|d| d.hints));
            // The step speaks the hint itself; the game's own would be
            // cut off by the next prompt.
            watch.with_mut(|d| d.lesson_says.insert("hint"));
        });

    let mut coach = Coach::default();
    let burn_now = format!("Burn now. Hold {p} until Safe speed.");
    let ease = format!("Let go. If the game says Too fast, hold {p} until Safe speed.");
    let late = format!("Hold {p} as soon as the game says Burn now.");
    let tap = format!("Each time the game says Too fast, hold {p} until Safe speed.");
    let land = pad.step(
        "Lesson 4 of 4, the landing. Fall freely until the game says Burn now.",
        move |d: &Descent, ctx| {
            let f = &d.flight;
            match f.outcome {
                Some(Outcome::Landed { speed }) => {
                    return Some(format!("Down safely at {}.", speed_words(speed)));
                }
                Some(Outcome::Crashed { .. }) => {
                    if !coach.crash {
                        coach.crash = true;
                        // Said after the crash: what to change next time.
                        let tip = if coach.ease { &tap } else { &late };
                        ctx.say_queued(tip.clone());
                    }
                    return None;
                }
                None => {}
            }
            if !f.started() {
                // Back at the top after a crash: coach the next descent.
                coach = Coach::default();
                return None;
            }
            if !coach.burn && !f.engine && matches!(advice(f), Advice::Burn { .. }) {
                coach.burn = true;
                ctx.say(burn_now.clone());
            } else if coach.burn && !coach.ease && f.engine && f.speed() <= f.moon.safe * EASED {
                coach.ease = true;
                ctx.say(ease.clone());
            }
            None
        },
    );

    let watch = pad.clone();
    let land = land.start(move |_| {
        watch.with_mut(|d| {
            d.lesson_says.remove("hint");
            d.lesson_says.insert("landed");
        });
    });

    Lesson::new(
        ID,
        "Soft Landing",
        format!(
            "Touch the pad slower than {safe} to land. Four short lessons on the practice pad, with endless fuel"
        ),
    )
    .step(fall)
    .step(engine)
    .step(hint)
    .step(land)
    .outro(format!(
        "On a moon the tank is small: fall freely, then burn once. {} reads height, speed and fuel. {} asks for a call-out.",
        sentence_start(&s),
        sentence_start(&t)
    ))
}
