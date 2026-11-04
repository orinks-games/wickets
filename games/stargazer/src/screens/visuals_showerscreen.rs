use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl ShowerScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        if let Some(m) = &self.watch.current {
            let t = (m.t / m.dur.max(0.01)).clamp(0., 1.) as f32;
            let x = 500. + (m.from + (m.to - m.from) * t) * 400.;
            let y = 120. + t * 300.;
            s.line(
                (x - 70., y - 35.),
                (x, y),
                if m.bright { 8. } else { 3. },
                PAPER,
            );
            s.circle(x, y, 8., GOLD);
        }
        s
    }
}
