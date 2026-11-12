//! The tutorial: one ball, bowled a step at a time on the real lane.

use std::cell::RefCell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::{Bus, Ctx, Sound};
use parlor_core::{Lesson, Parlor, ParlorAction, Reply, Rng, Step};

use crate::lane::{best_line, POCKET_LEFT};
use crate::play::hint_words;
use crate::rules::{
    aim_words, board_words, pinfall, pins_words, pull, release_words, Line, Rack, CENTRE,
    SWING_END, SWING_TOP,
};
use crate::sound;
use crate::{ID, NAME};

/// The board the lesson stands on, and the board it aims for: the pocket
/// line, which a clean ball always strikes here.
const POCKET_STAND: i32 = POCKET_LEFT.0;
const POCKET_AT: i32 = POCKET_LEFT.0 + POCKET_LEFT.1;

#[derive(Default)]
struct Ball {
    stand: i32,
    aim: i32,
    swing: Option<(f64, Sound)>,
    /// The pins the ball knocked down.
    down: Rack,
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
    );
    let (s, h) = (k(ParlorAction::Status), k(ParlorAction::Hint));
    let ball = Rc::new(RefCell::new(Ball {
        stand: CENTRE,
        ..Ball::default()
    }));

    let b = Rc::clone(&ball);
    let stand = Step::new(
        format!(
            "You start on board 20, in line with the head pin. A ball aimed straight at the head pin often leaves a split. Aim from off centre for a better chance of a strike. {} and {r} step one board. Step left to board {POCKET_STAND}.",
            sentence_start(&l)
        ),
        move |ctx, a| {
            let mut ball = b.borrow_mut();
            match a {
                ParlorAction::Left => ball.stand -= 1,
                ParlorAction::Right if ball.stand < CENTRE + 3 => ball.stand += 1,
                _ => return Reply::Nudge,
            }
            sound::play_at(ctx, sound::STEP, ball.stand as f64);
            let words = board_words(ball.stand);
            if ball.stand == POCKET_STAND {
                Reply::done(words)
            } else {
                Reply::say(words)
            }
        },
    );

    let b = Rc::clone(&ball);
    let aim = Step::new(
        format!(
            "{} turns your aim one step right, {d} one step left. The aim tone marks where the ball would cross the pins. Aim right until it crosses board {POCKET_AT}, the pocket beside the head pin.",
            sentence_start(&u)
        ),
        move |ctx, a| {
            let mut ball = b.borrow_mut();
            match a {
                ParlorAction::Up => ball.aim += 1,
                ParlorAction::Down => ball.aim -= 1,
                _ => return Reply::Nudge,
            }
            let target = ball.stand + ball.aim;
            sound::play_at(ctx, sound::AIM_TONE, target as f64);
            let line = format!("{}, board {target} at the pins.", sentence_start(&aim_words(ball.aim)));
            if target == POCKET_AT {
                Reply::done(format!("{line} You are aimed into the pocket at an angle."))
            } else {
                Reply::say(line)
            }
        },
    );

    let b = Rc::clone(&ball);
    let b2 = Rc::clone(&ball);
    let swing = Step::new(
        format!(
            "{} starts the swing. The swing tone rises to the top, then falls. Press {p} again right at the top to let go.",
            sentence_start(&p)
        ),
        move |ctx, a| {
            if a != ParlorAction::Primary {
                return Reply::Nudge;
            }
            let mut ball = b.borrow_mut();
            match ball.swing.take() {
                None => {
                    let s = ctx.audio.start_loop(sound::SWING, None, Bus::EFFECTS);
                    ctx.audio.set_pitch(s, sound::SWING_LOW);
                    ball.swing = Some((0.0, s));
                    Reply::say("Swing.")
                }
                Some((t, s)) => {
                    ctx.audio.stop(s, 40);
                    let error = t - SWING_TOP;
                    let line = Line::new(ball.stand, ball.aim, pull(error, &mut Rng::seeded(9)));
                    bowl(ctx, &mut ball, line, release_words(error))
                }
            }
        },
    )
    .tick(move |ctx, dt| {
        let mut ball = b2.borrow_mut();
        let (t, s) = ball.swing?;
        let t = t + dt;
        ctx.audio.set_pitch(s, sound::swing_pitch(t, SWING_TOP));
        if t < SWING_END {
            ball.swing = Some((t, s));
            return None;
        }
        ctx.audio.stop(s, 40);
        ball.swing = None;
        Some(Reply::say(format!(
            "The swing ran out. Press {} again, and let go as the swing tone stops rising.",
            ctx.control_name(ParlorAction::Primary)
        )))
    });

    let b = Rc::clone(&ball);
    let status = Step::press(
        format!(
            "{} speaks the frame, the pins standing and your score. Press it now.",
            sentence_start(&s)
        ),
        ParlorAction::Status,
        move |_| {
            let down = b.borrow().down.count();
            if down == 10 {
                "Frame 1 done, a strike. Score 10, plus your next two balls.".to_string()
            } else {
                format!(
                    "Frame 1, second ball, {} standing, score {down}.",
                    pins_words(10 - down)
                )
            }
        },
    );

    let b = Rc::clone(&ball);
    let hint = Step::press(
        format!(
            "{} suggests a board and an aim for your next ball. Press it now.",
            sentence_start(&h)
        ),
        ParlorAction::Hint,
        move |_| {
            let ball = b.borrow();
            let left = Rack::FULL.without(ball.down);
            if left.is_empty() {
                let (stand, aim) = best_line(Rack::FULL, ball.stand);
                format!("New rack: board {stand}, {}.", aim_words(aim))
            } else {
                hint_words(left, ball.stand)
            }
        },
    );

    Lesson::new(
        ID,
        NAME,
        "Ten frames, two balls a frame. First, stand and aim",
    )
    .step(stand)
    .step(aim)
    .step(swing)
    .step(status)
    .step(hint)
    .outro("All ten with the first ball is a strike, and with two balls a spare. Play bowls a full game, and Help explains scoring.")
}

/// The ball rolls, and the pins fall by the game's own pin model.
fn bowl(ctx: &mut Ctx<Parlor>, ball: &mut Ball, line: Line, release: &str) -> Reply {
    sound::play_at(ctx, sound::RELEASE, ball.stand as f64);
    if line.gutter() {
        sound::play_at(ctx, sound::GUTTER, line.at);
        return Reply::say(format!(
            "{release} Gutter. The ball left the lane. Press {} to swing again.",
            ctx.control_name(ParlorAction::Primary)
        ));
    }
    ball.down = pinfall(Rack::FULL, &line);
    let down = ball.down.count();
    sound::play(
        ctx,
        if down >= sound::HEAVY {
            sound::PIN_HEAVY
        } else {
            sound::PIN_LIGHT
        },
    );
    if down == 10 {
        sound::play(ctx, sound::STRIKE);
        Reply::done(format!("{release} Strike, all ten pins."))
    } else {
        Reply::done(format!(
            "{release} {} down. In a game, your second ball goes at the pins left.",
            sentence_start(&pins_words(down))
        ))
    }
}
