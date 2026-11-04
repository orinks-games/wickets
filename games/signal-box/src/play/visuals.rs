use super::*;
use parlor_core::visuals::*;
impl Signal {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        for i in 0..3 {
            let y = 170. + i as f32 * 130.;
            s.line((70., y), (930., y), 7., PAPER);
            for x in (80..930).step_by(30) {
                s.line((x as f32, y - 15.), (x as f32, y + 15.), 3., GOLD);
            }
        }
        if let Some(call) = self.shift.last_call {
            s.label(90., 70., call.word(), GOLD);
        }
        if let crate::rules::Phase::Passing { left } = self.shift.phase {
            let x = 900. - (left as f32 * 150.).clamp(0., 800.);
            s.rect(x, 130., 90., 45., TEAL);
            s.circle(x + 20., 180., 12., PAPER);
            s.circle(x + 70., 180., 12., PAPER);
        }
        s
    }
}
