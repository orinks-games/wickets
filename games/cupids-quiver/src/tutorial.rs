//! The tutorial: the real practice field, one skill at a time.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::field::{Field, Launch};
use crate::rules::Mode;
use crate::ID;

/// The practice field's seed in the lesson, so every player hears the
/// same first heart.
const SEED: u64 = 14;

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
    let h = k(ParlorAction::Hint);
    let field = Live::new(Field::new(Launch::seeded(Mode::Practice, SEED)).in_lesson());

    let mut turned = false;
    let turn = field.step(
        format!(
            "{} and {r} turn the bow, 5 degrees a press. The sky turns with you. Turn the bow, then {u} faces straight ahead again.",
            sentence_start(&l)
        ),
        move |f: &Field, _| {
            if f.run.aim != 0.0 {
                turned = true;
            }
            (turned && f.run.aim == 0.0).then(|| {
                "The bow faces straight ahead again.".to_string()
            })
        },
    );

    let draw = field.step(
        format!(
            "Hold {p} to draw the string. Let go once the bow is at full draw to shoot one arrow. {} eases the string without shooting.",
            sentence_start(&d)
        ),
        |f: &Field, _| (f.run.tally.shots >= 1).then(|| "That is a shot.".to_string()),
    );

    let hit = field.step(
        format!(
            "A paper heart is drifting across the sky. Turn toward it, aim a little ahead of its drift, and shoot when your sights are on it. {} says how far to turn to meet it. Hit one heart.",
            sentence_start(&h)
        ),
        |f: &Field, _| (f.run.tally.hits >= 1).then(|| "A hit.".to_string()),
    );

    Lesson::new(
        ID,
        "Cupid's Quiver",
        "You stand on a rooftop at dusk with a bow, and paper hearts drift across the sky. Learn on the practice field, with endless arrows",
    )
    .step(turn)
    .step(draw)
    .step(hit)
    .outro(format!(
        "Hearts come in pairs of one colour: hit both in a row for a sweetheart pair. {} lists the sky. Play climbs six rooftops, and Help has the scoring.",
        sentence_start(&t)
    ))
}
