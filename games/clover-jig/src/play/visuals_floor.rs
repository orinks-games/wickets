use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Floor {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let d = &self.run.dance;
        lanes(&mut s, 4);
        if let Some(i) = d.last_call {
            if let Some(c) = d.plan.calls.get(i) {
                let remaining = d.time_of(c.beat as i64) - d.now();
                let y = 500. - remaining as f32 * 0.15;
                marker(
                    &mut s,
                    500.,
                    y.clamp(90., 510.),
                    format!("{:?}", c.step),
                    GOLD,
                );
            }
        }
        player(&mut s, 500., 530.);
        s
    }
}
