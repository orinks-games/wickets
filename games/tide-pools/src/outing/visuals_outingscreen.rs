use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl OutingScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx).into_iter().take(2).collect(), [18, 30, 40]);
        let o = &self.outing;
        water(
            &mut s,
            if ctx.settings.display.reduced_motion {
                0.
            } else {
                o.elapsed
            },
        );
        for (i, pool) in o.pools.iter().enumerate() {
            let x = lane(i, o.pools.len());
            s.circle(x, 300., 55., if i == o.at { TEAL } else { [63, 91, 105] });
            s.label(x - 10., 390., (i + 1).to_string(), PAPER);
            if i == o.at {
                for (j, _find) in pool
                    .finds
                    .iter()
                    .filter(|f| !f.gathered && f.hiding <= 0.)
                    .enumerate()
                {
                    s.circle(x - 25. + j as f32 * 14., 300., 6., GOLD);
                }
            }
        }
        s
    }
}
