use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Bowl {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let r = &self.session;
        s.rect(240., 60., 520., 500., [145, 111, 72]);
        for i in 0..10 {
            if r.rack.has(i as u8 + 1) {
                let row = if i < 1 {
                    0
                } else if i < 3 {
                    1
                } else if i < 6 {
                    2
                } else {
                    3
                };
                let first = [0, 1, 3, 6][row];
                let x = 500. + ((i - first) as f32 - row as f32 / 2.) * 75.;
                s.circle(x, 210. - row as f32 * 45., 15., PAPER);
            }
        }
        let y = match r.phase {
            crate::lane::Phase::Rolling { t } => 520. - (t as f32 * 120.).min(320.),
            _ => 520.,
        };
        s.circle(240. + r.stand as f32 / 40. * 520., y, 23., TEAL);
        s
    }
}
