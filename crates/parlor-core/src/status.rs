//! The status key (S), in layers: Hearken's `StatusKey`, re-exported at
//! the path every game uses.
//!
//! One press answers the question a player asks most: what is wrong, and
//! the one or two numbers they track. Pressing it again straight away reads
//! the next layer, the fuller picture. Build the layers most urgent first,
//! and leave out a danger when there is none rather than saying "no
//! ghost". Keep each layer's order fixed so players learn where to stop
//! listening.

pub use hearken::text::sentences;
/// The status key, in layers, from Hearken: one press for what is wrong
/// and the numbers the player tracks, a quick second press for the rest.
pub use hearken::StatusKey;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::Base;
    use crate::Parlor;
    use hearken::status_key::AGAIN_SECONDS;
    use hearken::Headless;

    fn layers() -> Vec<String> {
        vec!["Short.".into(), String::new(), "More.".into()]
    }

    fn last(h: &Headless<Parlor>) -> String {
        h.spoken().last().cloned().unwrap_or_default()
    }

    #[test]
    fn a_quick_second_press_reads_the_next_layer_then_wraps() {
        let mut h = Headless::new(Default::default(), Base);
        let mut key = StatusKey::new();
        key.speak(&mut h.app.ctx, layers());
        assert_eq!(last(&h), "Short.");
        key.speak(&mut h.app.ctx, layers());
        assert_eq!(last(&h), "More.");
        key.speak(&mut h.app.ctx, layers());
        assert_eq!(last(&h), "Short.");
    }

    #[test]
    fn anything_said_in_between_starts_over() {
        let mut h = Headless::new(Default::default(), Base);
        let mut key = StatusKey::new();
        key.speak(&mut h.app.ctx, layers());
        h.app.ctx.say("A step.");
        key.speak(&mut h.app.ctx, layers());
        assert_eq!(last(&h), "Short.");
    }

    #[test]
    fn a_slow_second_press_starts_over() {
        let mut h = Headless::new(Default::default(), Base);
        let mut key = StatusKey::new();
        key.speak(&mut h.app.ctx, layers());
        h.app.ctx.clock += AGAIN_SECONDS + 1.0;
        key.speak(&mut h.app.ctx, layers());
        assert_eq!(last(&h), "Short.");
    }

    #[test]
    fn sentences_skips_empty_parts() {
        assert_eq!(sentences(["A.", "", "B."]), "A. B.");
    }
}
