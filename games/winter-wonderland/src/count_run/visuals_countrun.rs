use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl CountRun {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        dial(&mut s, self.night.beat.rem_euclid(12.), 12.);
        s.label(60., 60., self.night.spec.name, PAPER);
        s
    }
}
