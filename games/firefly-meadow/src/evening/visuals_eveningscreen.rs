use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl EveningScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let e = &self.evening;
        s.rect(0., 470., 1000., 130., [38, 65, 46]);
        for f in e.flies.iter().filter(|f| f.lit()) {
            let x = 500. + (f.pos.0 - e.pos.0) as f32 * 20.;
            let y = 300. + (f.pos.1 - e.pos.1) as f32 * 15.;
            if (40.0..960.0).contains(&x) && (40.0..460.0).contains(&y) {
                s.circle(x, y, 12., GOLD);
            }
        }
        player(&mut s, 500., 500.);
        s
    }
}
