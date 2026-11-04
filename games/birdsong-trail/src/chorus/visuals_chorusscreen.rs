use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl ChorusScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        for (i, b) in self.chorus.places.iter().enumerate() {
            let x = lane(i, self.chorus.places.len());
            s.line((x, 520.), (x, 180.), 10., TEAL);
            if b.is_some() {
                s.circle(x, 180., 20., GOLD);
            }
            if i == self.chorus.facing as usize {
                player(&mut s, x, 460.);
            }
        }
        s
    }
}
