use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl RunScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let l = &self.lake;
        water(
            &mut s,
            if ctx.settings.display.reduced_motion {
                0.
            } else {
                l.time
            },
        );
        let zoom = 220. / l.plan.radius.max(1.);
        for (i, light) in l.lanterns.iter().enumerate() {
            if l.lit(i) {
                s.circle(
                    500. + light.pos.x * zoom,
                    300. - light.pos.y * zoom,
                    10.,
                    GOLD,
                );
            }
        }
        let x = 500. + l.pos.x * zoom;
        let y = 300. - l.pos.y * zoom;
        s.circle(x, y, 15., PAPER);
        s.line(
            (x, y),
            (
                x + l.heading.to_radians().sin() * 40.,
                y - l.heading.to_radians().cos() * 40.,
            ),
            5.,
            TEAL,
        );
        s
    }
}
