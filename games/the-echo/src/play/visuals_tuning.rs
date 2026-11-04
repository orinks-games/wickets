use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Tuning {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let r = &self.rank;
        for (i, p) in r.pipes.iter().enumerate() {
            let x = lane(i, r.pipes.len());
            let height = 180. + (p.note as f32 * 5.).rem_euclid(230.);
            s.rect(
                x - 20.,
                490. - height,
                40.,
                height,
                if p.tuned { TEAL } else { PAPER },
            );
            if i == r.cur {
                s.circle(x, 535., 15., GOLD);
            }
        }
        s
    }
}
