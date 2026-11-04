use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl RunScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let g = &self.garden;
        let (cw, ch) = board(&mut s, g.cols, g.rows);
        for (i, plant) in g.plots.iter().enumerate() {
            let x = 60. + (i % g.cols) as f32 * cw;
            let y = 70. + (i / g.cols) as f32 * ch;
            s.rect(x + 5., y + 5., cw - 10., ch - 10., [69, 52, 40]);
            if let Some(p) = plant {
                let height = 20. + p.growth.clamp(0., 1.) * ch * 0.5;
                s.line(
                    (x + cw / 2., y + ch - 12.),
                    (x + cw / 2., y + ch - 12. - height),
                    7.,
                    TEAL,
                );
                s.circle(
                    x + cw / 2.,
                    y + ch - 12. - height,
                    12.,
                    if p.wilted { RED } else { GOLD },
                );
            }
            if i == g.row * g.cols + g.col {
                s.line((x + 5., y + ch - 4.), (x + cw - 5., y + ch - 4.), 6., PAPER);
            }
        }
        s
    }
}
