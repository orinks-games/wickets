use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl RunScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let r = &self.run;
        s.rect(30., 240., 120., 360., [83, 99, 113]);
        s.rect(850., 240., 120., 360., [83, 99, 113]);
        s.line((150., 320.), (850., 320.), 4., PAPER);
        let steps = r.levels[r.index].steps.max(1);
        let x = 150. + r.pos as f32 / steps as f32 * 700.;
        player(&mut s, x + r.lean as f32 * 25., 280.);
        s.line(
            (x - 60., 285.),
            (x + 60., 285. + r.lean as f32 * 30.),
            5.,
            GOLD,
        );
        s
    }
}
