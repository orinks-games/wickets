//! The tutorial: a small peal of two places, rung on the real tower rules.

use std::cell::RefCell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Parlor, ParlorAction, Reply, Step};

use crate::rules::{bell_words, row_words, Answer, Cue, Peal, Ringer, Rules, PLAIN};
use crate::sound;
use crate::ID;

/// The lesson's peal: two places from the six bells, no bell twice.
const SMALL: Rules = Rules {
    places: 2,
    bells: 6,
    repeats: false,
    rows: 99,
    hints: false,
};
/// The secret the lesson rings toward.
const CODE: [u8; 2] = [3, 1];

fn new_peal() -> Peal {
    let mut peal = Peal::new(SMALL, 0);
    peal.code = CODE.to_vec();
    peal
}

fn change(ctx: &mut Ctx<Parlor>, peal: &mut Peal, up: bool) -> String {
    let bell = peal.change_bell(up);
    sound::play(ctx, sound::bell(bell));
    format!("{}.", bell_words(bell, SMALL.bells))
}

fn shift(ctx: &mut Ctx<Parlor>, peal: &mut Peal, forward: bool) -> String {
    let place = peal.move_place(forward);
    let bell = peal.bell();
    sound::play(ctx, sound::bell(bell));
    format!("Place {}, bell {bell}.", place + 1)
}

/// Rings the row as play does: the rope, each bell, then one knock per
/// bell in place and one tap per bell misplaced, spaced to count. The
/// answer is spoken once they finish, from the step's tick.
fn ring(ctx: &mut Ctx<Parlor>, peal: &mut Peal, ringer: &mut Ringer) -> Answer {
    sound::play(ctx, sound::ROPE);
    let bells = peal.row.clone();
    let answer = peal.ring().unwrap_or_default();
    let words = format!("{}: {}.", row_words(&bells), answer.words_for(SMALL.places));
    ringer.row(&bells, Some(answer), Some(words));
    let due = ringer.due();
    play_cues(ctx, due);
    answer
}

/// Plays the ringer's sounds, and returns any words it was holding.
fn play_cues(ctx: &mut Ctx<Parlor>, cues: Vec<Cue>) -> Option<String> {
    let mut words = None;
    for cue in cues {
        match cue {
            Cue::Bell(b) => {
                sound::play(ctx, sound::bell(b));
            }
            Cue::Knock => {
                sound::play(ctx, sound::KNOCK);
            }
            Cue::Tap => {
                sound::play(ctx, sound::TAP);
            }
            Cue::Speak(text) => words = Some(text),
        }
    }
    words
}

/// A key while a row rings, as in play: the rest of the row is skipped
/// and its answer spoken at once.
fn skip(ctx: &mut Ctx<Parlor>, ringer: &mut Ringer) -> Option<String> {
    let rest: Vec<Cue> = ringer
        .flush()
        .into_iter()
        .filter(|c| matches!(c, Cue::Speak(_)))
        .collect();
    play_cues(ctx, rest)
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, u, d, p, rep) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Repeat),
    );
    let peal = Rc::new(RefCell::new(new_peal()));
    let ringer = Rc::new(RefCell::new(Ringer::new()));

    let pl = Rc::clone(&peal);
    let bells = Step::new(
        format!(
            "{} moves to the next bell down the scale, {d} back up. Bell 1, the treble, is the highest. Change bells until you reach bell 6, the tenor, the lowest.",
            sentence_start(&u)
        ),
        move |ctx, a| {
            let mut peal = pl.borrow_mut();
            let words = match a {
                ParlorAction::Up => change(ctx, &mut peal, true),
                ParlorAction::Down => change(ctx, &mut peal, false),
                _ => return Reply::Nudge,
            };
            if peal.bell() == SMALL.bells {
                Reply::done(words)
            } else {
                Reply::say(words)
            }
        },
    );

    let pl = Rc::clone(&peal);
    let places = Step::new(
        format!(
            "{} and {r} move between the places in your guess, called a row. This row has 2 places. Move to place 2.",
            sentence_start(&l)
        ),
        move |ctx, a| {
            let mut peal = pl.borrow_mut();
            let words = match a {
                ParlorAction::Left => shift(ctx, &mut peal, false),
                ParlorAction::Right => shift(ctx, &mut peal, true),
                _ => return Reply::Nudge,
            };
            if peal.place == 1 {
                Reply::done(words)
            } else {
                Reply::say(words)
            }
        },
    );

    // The first ring: the step ends once the bells, knocks and taps are
    // done and the answer is spoken.
    let (pl, rg) = (Rc::clone(&peal), Rc::clone(&ringer));
    let rg_tick = Rc::clone(&ringer);
    let first_ring = Step::new(
        format!(
            "{} rings your row, and the apprentice answers. In place means a bell is right and in its place. Misplaced means a bell is in the peal, but in another place. Ring your row now.",
            sentence_start(&p)
        ),
        move |ctx, a| {
            let mut ringer = rg.borrow_mut();
            if ringer.busy() {
                return skip(ctx, &mut ringer).map_or(Reply::Nudge, Reply::done);
            }
            if a != ParlorAction::Primary {
                return Reply::Nudge;
            }
            ring(ctx, &mut pl.borrow_mut(), &mut ringer);
            Reply::say(String::new())
        },
    )
    .tick(move |ctx, dt| {
        let cues = rg_tick.borrow_mut().update(dt);
        play_cues(ctx, cues).map(Reply::done)
    });

    let (pl, rg) = (Rc::clone(&peal), Rc::clone(&ringer));
    let (pl_tick, rg_tick) = (Rc::clone(&peal), Rc::clone(&ringer));
    let p_name = p.clone();
    let solve = Step::new(
        format!(
            "Now find the peal. Change bells, move between places, and ring until both bells are in place. {} reads the rows you have rung so far.",
            sentence_start(&rep)
        ),
        move |ctx, a| {
            let mut peal = pl.borrow_mut();
            let mut ringer = rg.borrow_mut();
            if ringer.busy() {
                let Some(words) = skip(ctx, &mut ringer) else {
                    return Reply::Nudge;
                };
                return answered(ctx, &peal, words);
            }
            let words = match a {
                ParlorAction::Up => change(ctx, &mut peal, true),
                ParlorAction::Down => change(ctx, &mut peal, false),
                ParlorAction::Left => shift(ctx, &mut peal, false),
                ParlorAction::Right => shift(ctx, &mut peal, true),
                ParlorAction::Status => format!("Your row: {}.", row_words(&peal.row)),
                ParlorAction::Repeat if peal.log.is_empty() => format!(
                    "No rows rung yet. {} rings your row.",
                    sentence_start(&p_name)
                ),
                ParlorAction::Repeat => peal.log_lines().join(" "),
                ParlorAction::Primary => {
                    ring(ctx, &mut peal, &mut ringer);
                    String::new()
                }
                _ => return Reply::Nudge,
            };
            Reply::say(words)
        },
    )
    .tick(move |ctx, dt| {
        let cues = rg_tick.borrow_mut().update(dt);
        let words = play_cues(ctx, cues)?;
        Some(answered(ctx, &pl_tick.borrow(), words))
    })
    .own_repeat();

    Lesson::new(
        ID,
        "Belfry",
        "The old bell ringer left a secret sequence of bells, called a peal. Work out which bells he chose and their order. Ring a guess, and the apprentice answers with clues. Start with a practice sequence of 2 bells",
    )
    .step(bells)
    .step(places)
    .step(first_ring)
    .step(solve)
    .outro(format!(
        "Try Plain first. The secret sequence contains {} bells chosen from the {} available. You have {} guesses to solve it. Each bell appears at most once. In Village, bells can repeat. Help explains hints and scoring.",
        PLAIN.places, PLAIN.bells, PLAIN.rows
    ))
}

/// The answer to a row in the last step: the peal found ends the lesson.
fn answered(ctx: &mut Ctx<Parlor>, peal: &Peal, words: String) -> Reply {
    if peal.solved() {
        sound::play(ctx, sound::SOLVED);
        let n = peal.rows_used();
        Reply::done(format!(
            "{}: both in place. The peal is found, in {n} rows.",
            row_words(&peal.row)
        ))
    } else {
        Reply::say(words)
    }
}
