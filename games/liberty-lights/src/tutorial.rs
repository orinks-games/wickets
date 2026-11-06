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
            "{} plays the show: three wooden ticks, then a brighter go. The go is time zero. Listen for the boom and when it went up.",
            sentence_start(&p)
        ),
        |show: &Show, _| {
            (show.round.phase == Phase::YourTurn)
                .then(|| "That fizz is the fuse: your turn.".to_string())
        },
    );

    let launch = sky.step(
        format!(
            "Press {p} for your own count-in. After the go, press 1 to launch a boom at the same delay as in the show. {} hears the show again.",
            sentence_start(&t)
        ),
        |show: &Show, _| {
            let l = show.round.launches.first()?;
            let word = timing_word(l.ms - BOOM_MS);
            Some(format!(
                "Boom launched, {word} timing. Each firework scores for the right kind, the right place and the timing."
            ))
        },
    );

    Lesson::new(
        ID,
        "Liberty Lights",
        "You are the fireworks director: each show is played to you, and you launch it back. Learn on a show of one boom",
    )
    .step(listen)
    .step(launch)
    .outro(format!(
        "There are four fireworks, on keys 1 to 4. {} From level 11 they also go up left or right. 60 percent clears a level, and Enter finishes a show. The Practice room tries every firework with nothing to score.",
        kinds.join(" ")
    ))
}
