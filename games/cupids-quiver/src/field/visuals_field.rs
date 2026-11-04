use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Field {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        for t in &self.run.sky {
            marker(
                &mut s,
                500. + t.angle as f32 * 5.,
                180.,
                t.kind.name(),
                GOLD,
            );
        }
        let x = 500. + self.run.aim as f32 * 5.;
        s.line((x - 25., 460.), (x + 25., 460.), 4., PAPER);
        s.line((x, 435.), (x, 485.), 4., PAPER);
        for a in &self.run.flying {
            s.line(
                (500. + a.aim as f32 * 5., 480. - a.t as f32 * 200.),
                (500. + a.aim as f32 * 5., 450. - a.t as f32 * 200.),
                5.,
                TEAL,
            );
        }
        s
    }
}
