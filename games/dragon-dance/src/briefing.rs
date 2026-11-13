//! The briefing before a street from the title or street select: what the
//! street asks of the dragon, and number keys for the cues it plays (see
//! [`parlor_core::briefing`]). A street with no gongs has no gong cues, a
//! calm walk has lanterns in place of envelopes and firecrackers, and the
//! New Year parade adds its midnight countdown and great gong.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::briefing::{Briefing, Cue};
use parlor_core::Parlor;

use crate::dance::{DanceScreen, Launch, Mode};
use crate::rules::{Finale, Street};
use crate::sound;

/// Whether the street strikes gongs.
fn gongs(street: &Street) -> bool {
    street.gong_every > 0 || street.finale != Finale::Calm
}

/// What the street is, in a sentence or two.
pub fn text(mode: Mode) -> String {
    let street = mode.street();
    let mut parts = vec!["keep your head in the pearl's lane on every beat"];
    if gongs(&street) {
        parts.push("rear up on the gong");
    }
    if street.envelope_chance > 0.0 {
        parts.push(if street.calm() {
            "catch the lanterns"
        } else {
            "catch red envelopes"
        });
    }
    if street.firecracker_chance > 0.0 && !street.calm() {
        parts.push("avoid firecrackers");
    }
    let todo = match parts.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{}, and {last}", rest.join(", ")),
        _ => parts.concat(),
    };
    format!("{}. {}.", mode.title(), sentence_start(&todo))
}

/// The sound check's cues this street plays, in the sound check's order.
pub fn cues(mode: Mode) -> Vec<Cue> {
    let street = mode.street();
    let calm = street.calm();
    let midnight = street.finale == Finale::Midnight;
    sound::catalog()
        .into_iter()
        .filter(|(key, _)| match *key {
            sound::DRUM | sound::PEARL | sound::OFF_PEARL => true,
            sound::ROLL | sound::GONG => gongs(&street),
            sound::GREAT_GONG | sound::COUNTDOWN => midnight,
            sound::PEARL_MOVE => street.move_chance > 0.0,
            sound::ENVELOPE => street.envelope_chance > 0.0 && !calm,
            sound::LANTERN => street.envelope_chance > 0.0 && calm,
            sound::CRACKLE => street.firecracker_chance > 0.0 && !calm,
            _ => false,
        })
        .map(|(key, meaning)| {
            let label = meaning.split(':').next().unwrap_or(meaning);
            if key.ends_with("/loop") {
                Cue::looped(label, key)
            } else {
                Cue::new(label, key)
            }
        })
        .collect()
}

/// The briefing for `mode`, which starts the street on Enter.
pub fn briefing(mode: Mode) -> Briefing {
    Briefing::new(text(mode))
        .cues(cues(mode))
        .then(move |ctx| ctx.replace(DanceScreen::new(Launch::new(mode))))
}

/// Opens `mode`'s briefing, or starts the street at once with briefings
/// off.
pub fn open(ctx: &mut Ctx<Parlor>, mode: Mode) {
    briefing(mode).show(ctx);
}
