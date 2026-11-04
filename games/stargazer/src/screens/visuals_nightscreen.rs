use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl NightScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let n = &self.night;
        for i in &n.sky.up {
            let o = &n.sky.objects[*i];
            s.circle(
                50. + o.az as f32 / 360. * 900.,
                530. - o.alt as f32 / 90. * 460.,
                (5. - o.mag).clamp(2., 8.),
                PAPER,
            );
        }
        let x = 50. + n.az as f32 / 360. * 900.;
        let y = 530. - n.alt as f32 / 90. * 460.;
        s.line((x - 25., y), (x + 25., y), 3., GOLD);
        s.line((x, y - 25.), (x, y + 25.), 3., GOLD);
        s
    }
}
