use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Descent {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let f = &self.flight;
        s.rect(0., 535., 1000., 65., [105, 99, 91]);
        let y = 490. - (f.altitude / f.moon.height.max(1.)).clamp(0., 1.) as f32 * 380.;
        s.rect(450., y - 35., 100., 60., PAPER);
        s.line((450., y + 25.), (425., y + 55.), 7., PAPER);
        s.line((550., y + 25.), (575., y + 55.), 7., PAPER);
        if f.engine {
            s.line((500., y + 25.), (500., y + 65.), 18., GOLD);
        }
        s.label(60., 70., format!("{:.0}", f.altitude), GOLD);
        s
    }
}
