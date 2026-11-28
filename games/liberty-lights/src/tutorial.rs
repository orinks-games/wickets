//! The tutorial: one boom, heard and launched back on a real show.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::controls::{is_enter, Pick};
use crate::rules::{timing_word, Kind, Launch, Phase, Pos};
use crate::show::{Mode, Show};
use crate::ID;

/// When the lesson's boom goes up, in milliseconds after the go.
const BOOM_MS: i32 = 1000;

/// A one-boom show, as level 1 plays it. Enter, and finishing from the
/// wheel, are kept back: the lesson ends before a show is scored.
pub fn sky() -> Live<Show> {
    let show = vec![Launch::new(Kind::Boom, Pos::Center, BOOM_MS)];
    let mut sky = Show::with_show(Mode::Level { level: 1, total: 0 }, show);
    sky.lesson = true;
    Live::new(sky).guard(|show: &Show, ctx, event| {
        let finishing = is_enter(event)
            || (ctx.action_for(event) == Some(ParlorAction::Primary)
                && show.round.phase == Phase::Playing
                && show.wheel.current() != Pick::Fire(Kind::Boom));
        finishing.then(|| {
            format!(
                "Launch the boom first, with 1 or {}.",
                ctx.control_name(ParlorAction::Primary)
            )
        })
    })
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, sky())
}

/// The tutorial on `sky`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, sky: Live<Show>) -> Lesson {
    let p = ctx.control_name(ParlorAction::Primary);
    let t = ctx.control_name(ParlorAction::Secondary);
    let kinds: Vec<String> = Kind::ALL
        .iter()
        .map(|k| format!("{}, {}: {}.", k.digit(), k.name(), k.describe()))
        .collect();

    let listen = sky.step(
        format!(
            "{} plays the show, starting with three count-in beats and a go cue that marks time zero. Pay attention to when the boom goes up.",
            sentence_start(&p)
        ),
        |show: &Show, _| {
            (show.round.phase == Phase::YourTurn)
                .then(|| "The show is over, and now it is your turn.".to_string())
        },
    );

    let launch = sky.step(
        format!(
            "Press {p} for your own count-in, then after the go cue press 1 to launch a boom at the same moment. {} replays the show if you want another try.",
            sentence_start(&t)
        ),
        |show: &Show, _| {
            let l = show.round.launches.first()?;
            let word = timing_word(l.ms - BOOM_MS);
            Some(format!(
                "Boom launched, {word} timing. Each firework scores for the right kind, the right spot and accurate timing."
            ))
        },
    );

    Lesson::new(
        ID,
        "Liberty Lights",
        "You are the director of the Fourth of July fireworks show. Each level plays a pattern, and you launch it back on beat. Try it out on a single boom",
    )
    .step(listen)
    .step(launch)
    .outro(format!(
        "There are four fireworks on keys 1 to 4. {} Starting at level 11, fireworks also launch on the left or right. Score 60 percent or higher to clear a level, press Enter when you are done launching, and visit the Practice room anytime to try every firework without scoring.",
        kinds.join(" ")
    ))
}
