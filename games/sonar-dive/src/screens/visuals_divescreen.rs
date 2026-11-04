use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl DiveScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        if let Some(e) = &self.exp {
            let d = &e.dive;
            water(
                &mut s,
                if ctx.settings.display.reduced_motion {
                    0.
                } else {
                    d.time
                },
            );
            s.circle(500., 300., 210., [27, 65, 74]);
            for ret in &d.returns {
                let a = ret.clock as f32 / 12. * std::f32::consts::TAU;
                let r = (ret.dist / d.layout.sonar_range.max(1.)).min(1.) * 190.;
                s.circle(500. + a.sin() * r, 300. - a.cos() * r, 8., GOLD);
            }
            player(&mut s, 500., 300.);
            s.line(
                (500., 300.),
                (
                    500. + d.heading.to_radians().sin() * 65.,
                    300. - d.heading.to_radians().cos() * 65.,
                ),
                5.,
                TEAL,
            );
        }
        s
    }
}
