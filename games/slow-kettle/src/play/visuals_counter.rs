use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Counter {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let k = &self.service.kettle;
        s.circle(400., 310., 130., [116, 142, 149]);
        s.rect(340., 160., 120., 25., PAPER);
        s.line((510., 280.), (620., 225.), 30., [116, 142, 149]);
        if k.on {
            for i in 0..4 {
                let x = 350. + i as f32 * 35.;
                s.line(
                    (x, 155.),
                    (
                        x + 10.,
                        100. - (if ctx.settings.display.reduced_motion {
                            0.
                        } else {
                            ctx.clock as f32
                        } * 25.
                            + i as f32 * 15.)
                            % 50.,
                    ),
                    4.,
                    PAPER,
                );
            }
        }
        gauge(
            &mut s,
            520.,
            "Temperature",
            k.temp / 100.,
            format!("{:.0}", k.temp),
        );
        s
    }
}
