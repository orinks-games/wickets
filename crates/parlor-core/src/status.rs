//! The status key (S), in layers.
//!
//! One press answers the question a player asks most: what is wrong, and
//! the one or two numbers they track. Pressing it again straight away reads
//! the next layer, the fuller picture. This follows Freight Fate's Space
//! key, which says speed and gear and leaves what is normal unsaid, while a
//! separate report lists everything. A wall of facts on every press makes
//! players wait through what they already know to reach what changed.
//!
//! Build the layers most urgent first, and leave out a danger when there is
//! none rather than saying "no ghost". Keep each layer's order fixed so
//! players learn where to stop listening.

use crate::Parlor;
use hearken::Ctx;

/// How long after a status line a second press still counts as "again".
const AGAIN_SECONDS: f64 = 6.0;

/// Remembers the last status press, so a quick second press reads the next
/// layer. Keep one on each play screen and pass every S press to
/// [`StatusKey::speak`].
#[derive(Clone, Debug, Default)]
pub struct StatusKey {
    /// `ctx.said` and `ctx.clock` just after the last status line, and
    /// which layer it was.
    last: Option<(u64, f64, usize)>,
}

impl StatusKey {
    pub fn new() -> Self {
        Self::default()
    }

    /// Speak the next layer. The first press, or any press after something
    /// else was said or a few seconds went by, speaks the first layer.
    /// Empty layers are skipped, and after the last layer it starts over.
    /// Returns the line it spoke, for a game that keeps it for R.
    pub fn speak(&mut self, ctx: &mut Ctx<Parlor>, layers: Vec<String>) -> String {
        let layers: Vec<String> = layers.into_iter().filter(|l| !l.is_empty()).collect();
        if layers.is_empty() {
            return String::new();
        }
        let next = match self.last {
            Some((said, at, layer)) if said == ctx.said && ctx.clock - at <= AGAIN_SECONDS => {
                (layer + 1) % layers.len()
            }
            _ => 0,
        };
        let line = layers[next].clone();
        ctx.say(line.clone());
        self.last = Some((ctx.said, ctx.clock, next));
        line
    }

    /// Forget the last press, so the next one starts at the first layer.
    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// Join the non-empty parts with spaces: each part is a whole sentence.
pub fn sentences<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parts
        .into_iter()
        .filter(|p| !p.as_ref().is_empty())
        .map(|p| p.as_ref().trim().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::Base;
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
