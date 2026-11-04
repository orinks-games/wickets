use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Tower {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        for (i, bell) in self.peal.row.iter().enumerate() {
            let x = lane(i, self.peal.row.len());
            s.rect(
                x - 30.,
                180.,
                60.,
                180.,
                if i == self.peal.place { GOLD } else { TEAL },
            );
            s.circle(x, 370., 15., PAPER);
            s.label(x - 10., 150., bell.to_string(), PAPER);
        }
        s
    }
}
