use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl DeskRun {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [30, 22, 18]);
        let shelf = &self.stack.shelf;
        let n = shelf.len().max(1) as f32;
        s.rect(50., 300., 900., 12., [120, 84, 52]);
        for (i, p) in shelf.iter().enumerate() {
            let x = 70. + i as f32 * (860. / n);
            if !p.gone {
                s.rect(
                    x,
                    240.,
                    60.,
                    60.,
                    if i == self.stack.at { GOLD } else { RED },
                );
            }
        }
        let x = 70. + self.stack.at as f32 * (860. / n);
        player(&mut s, x + 30., 360.);
        s
    }
}
