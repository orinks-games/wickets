use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Floor {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let d = &self.run.dance;
        for i in 0..5 {
            s.rect(90. + i as f32 * 180., 50., 65., 110., [76, 82, 86]);
        }
        lanes(&mut s, 4);
        if let Some(i) = d.last_call {
            if let Some(c) = d.plan.calls.get(i) {
                let remaining = d.time_of(c.beat as i64) - d.now();
                marker(
                    &mut s,
                    500.,
                    (500. - remaining as f32 * 0.15).clamp(200., 510.),
                    format!("{:?}", c.step),
                    GOLD,
                );
            }
        }
        player(&mut s, 500., 530.);
        s
    }
}
