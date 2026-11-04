use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl SortRun {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        s.rect(50., 280., 900., 150., [75, 97, 107]);
        if let Some(g) = &self.shift.gift {
            let x = 100. + (g.ridden / g.travel.max(0.01)).clamp(0., 1.) as f32 * 800.;
            s.rect(x - 30., 310., 60., 60., GOLD);
            s.line((x, 310.), (x, 370.), 6., RED);
        }
        s
    }
}
