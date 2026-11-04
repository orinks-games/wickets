use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Crack {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        dial(&mut s, self.run.dial as f64, 100.);
        s.label(65., 60., format!("Wheel {}", self.run.wheel + 1), GOLD);
        s
    }
}
