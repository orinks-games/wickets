use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl RunScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let r = &self.run;
        lanes(&mut s, r.spec.lanes);
        for f in &r.falling {
            let y = 100. + (1. - f.left / f.total.max(0.01)) as f32 * 380.;
            marker(&mut s, lane(f.lane, r.spec.lanes), y, f.kind.name(), GOLD);
        }
        let x = lane(r.lane, r.spec.lanes);
        s.rect(x - 35., 510., 70., 35., TEAL);
        s
    }
}
