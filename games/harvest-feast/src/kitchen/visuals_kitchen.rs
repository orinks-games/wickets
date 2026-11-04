use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Kitchen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        for (i, d) in self.meal.dishes.iter().enumerate() {
            let x = lane(i, self.meal.dishes.len());
            s.rect(x - 45., 250., 90., 100., [117, 132, 139]);
            s.circle(x, 250., 45., PAPER);
            s.label(x - 30., 420., format!("{}", d.step + 1), GOLD);
            let ratio = d
                .current()
                .map_or(0., |step| d.clock / step.wait.max(0.01))
                .clamp(0., 1.);
            let label = match d.stage {
                crate::rules::Stage::Queued => "Waiting",
                crate::rules::Stage::Cooking => "Cooking",
                crate::rules::Stage::Due => "Ready",
                crate::rules::Stage::Burning => "Burning",
                crate::rules::Stage::Done => "Served",
            };
            s.label(x - 45., 190., label, PAPER);
            s.rect(x - 35., 470., 70. * ratio as f32, 12., TEAL);
        }
        s
    }
}
