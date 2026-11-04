use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl CountScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        s.rect(50., 440., 900., 70., [50, 85, 64]);
        s.label(380., 300., self.count.guess.to_string(), GOLD);
        s.label(380., 360., "Count", PAPER);
        if let crate::rules::CountPhase::Passing(time) = self.count.phase {
            if let Some(flock) = self.count.flocks.get(self.count.at) {
                for (i, at) in flock.calls.iter().enumerate() {
                    let age = time - at;
                    if (0.0..0.5).contains(&age) {
                        let x = 500. + flock.sides[i] * 350.;
                        s.line((x - 18., 180.), (x, 192.), 5., PAPER);
                        s.line((x, 192.), (x + 18., 180.), 5., PAPER);
                    }
                }
            }
        }
        s
    }
}
