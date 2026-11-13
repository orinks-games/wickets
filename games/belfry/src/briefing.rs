//! The briefing before a peal from the title: the peal's rules, and number
//! keys for the bells it rings and the answer cues (see
//! [`parlor_core::briefing`]). Practice night adds the called place's cue.

use hearken::Ctx;
use parlor_core::briefing::{Briefing, Cue};
use parlor_core::Parlor;

use crate::rules::{bell_words, Mode};
use crate::sound;
use crate::tower::Tower;

/// What the peal is, in a sentence or two.
pub fn text(mode: Mode) -> String {
    let r = mode.rules();
    let twice = if r.repeats {
        "A bell may come twice."
    } else {
        "No bell comes twice."
    };
    let mut line = format!(
        "{}. Ring rows to find the secret peal of {} bells from {}, in {} rows. {twice}",
        mode.name(),
        r.places,
        r.bells,
        r.rows
    );
    if mode == Mode::PracticeNight {
        line.push_str(" The captain calls one place.");
    }
    line
}

/// The peal's bells, treble first, then the knock and the tap, and on
/// Practice night the called place, named as the sound check names them.
pub fn cues(mode: Mode) -> Vec<Cue> {
    let bells = mode.rules().bells;
    let mut cues: Vec<Cue> = (1..=bells)
        .map(|b| Cue::new(bell_words(b, bells), sound::bell(b)))
        .collect();
    for (key, meaning) in sound::catalog() {
        let wanted = key == sound::KNOCK
            || key == sound::TAP
            || (key == sound::MOON && mode == Mode::PracticeNight);
        if wanted {
            let label = meaning.split(':').next().unwrap_or(meaning);
            cues.push(Cue::new(label, key));
        }
    }
    cues
}

/// The briefing for `mode`, which starts a new peal on Enter.
pub fn briefing(mode: Mode) -> Briefing {
    Briefing::new(text(mode))
        .cues(cues(mode))
        .then(move |ctx| ctx.replace(Tower::new(mode)))
}

/// Opens `mode`'s briefing, or starts the peal at once with briefings off.
pub fn open(ctx: &mut Ctx<Parlor>, mode: Mode) {
    briefing(mode).show(ctx);
}
