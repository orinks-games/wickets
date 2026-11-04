use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl WalkScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let w = &self.walk;
        s.rect(410., 60., 180., 500., [87, 78, 49]);
        for b in &w.birds {
            if !b.left {
                let dx = b.pos.0 - w.x;
                let dy = b.pos.1 - w.y;
                if dx.abs() < 12. && dy.abs() < 12. {
                    marker(
                        &mut s,
                        500. + dx * 30.,
                        300. + dy * 20.,
                        if b.friend { "Bird" } else { "Song" },
                        GOLD,
                    );
                }
            }
        }
        player(&mut s, 500., 300.);
        s
    }
}
