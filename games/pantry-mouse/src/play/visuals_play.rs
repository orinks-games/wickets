use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Play {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx).into_iter().take(4).collect(), [18, 30, 40]);
        let r = &self.run;
        let (cw, ch) = board(&mut s, r.maze.w as usize, r.maze.h as usize);
        for y in 0..r.maze.h {
            for x in 0..r.maze.w {
                let c = crate::rules::Cell { x, y };
                if r.has_crumb(c) {
                    s.circle(
                        60. + (x as f32 + 0.5) * cw,
                        70. + (y as f32 + 0.5) * ch,
                        cw.min(ch) * 0.09,
                        GOLD,
                    );
                }
                if !r.maze.is_open(c) {
                    s.rect(
                        60. + x as f32 * cw,
                        70. + y as f32 * ch,
                        cw,
                        ch,
                        [92, 79, 67],
                    );
                }
            }
        }
        for c in &r.cats {
            marker(
                &mut s,
                60. + (c.pos.x as f32 + 0.5) * cw,
                70. + (c.pos.y as f32 + 0.5) * ch,
                c.name,
                RED,
            );
        }
        player(
            &mut s,
            60. + (r.mouse.x as f32 + 0.5) * cw,
            70. + (r.mouse.y as f32 + 0.5) * ch,
        );
        s
    }
}
