//! The tutorial: one cup of Sencha, brewed on the real kettle.

use std::cell::RefCell;
use std::rc::Rc;

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Parlor, ParlorAction, Reply, Step};

use crate::rules::{strength_words, Cue, Service};
use crate::sound::{self, KettleSong};
use crate::teas::{Tea, TEAS};
use crate::ID;

/// The tea the lesson brews: a green tea, so the player pours before the
/// boil.
const SENCHA: usize = 1;
/// How far from the tea's heat a learner's pour still counts.
const NEAR: f64 = 10.0;

struct Brew {
    shelf: usize,
    service: Service,
    song: KettleSong,
}

fn starters() -> Vec<&'static Tea> {
    TEAS.iter().filter(|t| t.price().is_none()).collect()
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (u, d, p, t, s, h) = (
        k(ParlorAction::Up),
        k(ParlorAction::Down),
        k(ParlorAction::Primary),
        k(ParlorAction::Secondary),
        k(ParlorAction::Status),
        k(ParlorAction::Hint),
    );
    let sencha = &TEAS[SENCHA];
    let brew = Rc::new(RefCell::new(Brew {
        shelf: 0,
        service: Service::new(false),
        song: KettleSong::default(),
    }));

    let b = Rc::clone(&brew);
    let shelf = Step::new(
        format!(
            "{} and {d} move along your shelf; each tea is named. Find Sencha, then press {p} to put it in the pot.",
            sentence_start(&u)
        ),
        move |ctx, a| {
            let mut b = b.borrow_mut();
            let teas = starters();
            match a {
                ParlorAction::Down => b.shelf = (b.shelf + 1) % teas.len(),
                ParlorAction::Up => b.shelf = (b.shelf + teas.len() - 1) % teas.len(),
                ParlorAction::Primary => {
                    let tea = teas[b.shelf];
                    if tea.id != sencha.id {
                        sound::play(ctx, sound::NOPE);
                        return Reply::say("This lesson uses Sencha. Choose Sencha from the shelf.");
                    }
                    sound::play(ctx, sound::TIN);
                    b.service.choose(tea);
                    return Reply::done(format!("Sencha in the pot. {}", tea.needs()));
                }
                _ => return Reply::Nudge,
            }
            sound::play(ctx, sound::SHELF);
            let tea = teas[b.shelf];
            Reply::say(format!("{}. {}", tea.name, tea.about))
        },
    );

    let b = Rc::clone(&brew);
    let b2 = Rc::clone(&brew);
    let kettle = Step::new(
        format!(
            "{} puts the kettle on, and it heats toward the boil. Sencha wants water at 75 degrees, well before the boil: press {p} again then. {} says the heat.",
            sentence_start(&p),
            sentence_start(&s)
        ),
        move |ctx, a| {
            let mut b = b.borrow_mut();
            let heating = b.song.playing();
            match a {
                ParlorAction::Status => Reply::say(format!(
                    "{} degrees, {}.",
                    b.service.kettle.degrees(),
                    b.service.kettle.stage().words()
                )),
                ParlorAction::Primary if !heating => {
                    sound::play(ctx, sound::KETTLE_ON);
                    b.service.heat();
                    let temp = b.service.kettle.temp;
                    b.song.start(ctx, temp);
                    Reply::say("Kettle on.")
                }
                ParlorAction::Primary => {
                    sound::play(ctx, sound::POUR);
                    b.song.stop(ctx);
                    let heat = b.service.pour();
                    let deg = heat.round() as u32;
                    let want = sencha.heat as f64;
                    if (heat - want).abs() <= NEAR {
                        Reply::done(format!("Poured at {deg} degrees, right for Sencha. The leaves are steeping."))
                    } else {
                        let why = if heat > want {
                            "too hot: green tea turns bitter"
                        } else {
                            "too cool: the tea stays thin"
                        };
                        b.service.kettle.temp = crate::rules::ROOM;
                        b.service.choose(sencha);
                        Reply::say(format!(
                            "Poured at {deg} degrees, {why}. Fresh water in the kettle. Press {} to try again.",
                            ctx.control_name(ParlorAction::Primary)
                        ))
                    }
                }
                _ => Reply::Nudge,
            }
        },
    )
    .tick(move |ctx, dt| {
        let mut b = b2.borrow_mut();
        if !b.song.playing() {
            return None;
        }
        let cues = b.service.update(dt);
        let temp = b.service.kettle.temp;
        b.song.follow(ctx, temp);
        cues.into_iter().find_map(|c| match c {
            Cue::Stage(stage) => Some(Reply::say(stage.call_out())),
            _ => None,
        })
    });

    let b = Rc::clone(&brew);
    let b2 = Rc::clone(&brew);
    let steep = Step::new(
        format!(
            "Sencha wants a 6 second steep. {} breathes in the steam to check how strong it is, and {p} lifts the leaves out.",
            sentence_start(&t)
        ),
        move |ctx, a| {
            let mut b = b.borrow_mut();
            let words = strength_words(b.service.steeped, sencha.steep as f64);
            match a {
                ParlorAction::Secondary => Reply::say(format!("You breathe in the steam: {words}.")),
                ParlorAction::Primary => {
                    sound::play(ctx, sound::LIFT);
                    let secs = b.service.seconds();
                    b.service.lift();
                    // Praise only what the strength words call ready.
                    let close = words == strength_words(sencha.steep as f64, sencha.steep as f64);
                    let after = if close {
                        "A guest would love it."
                    } else {
                        "Each tea likes its own steep, and the guest still enjoys the cup."
                    };
                    Reply::done(format!(
                        "Leaves out at {}, {words}. {after}",
                        crate::talk::seconds_words(secs)
                    ))
                }
                _ => Reply::Nudge,
            }
        },
    )
    .tick(move |ctx, dt| {
        let mut b = b2.borrow_mut();
        for cue in b.service.update(dt) {
            match cue {
                Cue::Tick(_) => {
                    sound::play(ctx, sound::TICK);
                }
                Cue::LongSteep => {
                    sound::play(ctx, sound::LIFT);
                    return Some(Reply::done(
                        "The leaves steeped so long they came out on their own. Very strong.",
                    ));
                }
                _ => {}
            }
        }
        None
    })
    .start(|ctx| {
        sound::play(ctx, sound::TICK);
    });

    let hint = Step::press(
        format!(
            "{} gives a hint at any time, such as where a guest's tea is on your shelf. Press it now.",
            sentence_start(&h)
        ),
        ParlorAction::Hint,
        move |ctx| {
            sound::play(ctx, sound::HINT);
            sencha.needs()
        },
    );

    let b = Rc::clone(&brew);
    Lesson::new(
        ID,
        "Slow Kettle",
        "You keep a small tea house, at your own pace, and nothing here can go wrong. Make a cup of Sencha now",
    )
    .step(shelf)
    .step(kettle)
    .step(steep)
    .step(hint)
    .outro("Guests come in and say what they would like. Play opens the tea house, and Practice brewing is a quiet place to learn the kettle.")
    .on_exit(move |ctx| b.borrow_mut().song.stop(ctx))
}
