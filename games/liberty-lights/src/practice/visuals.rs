use super::*;
#[allow(unused_imports)]
use crate::visuals::{burst, draw_round};
use parlor_core::visuals::*;
impl Range {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [14, 22, 45]);
        s.label(120., 100., self.wheel.current().spoken(), PAPER);
        s.label(120., 160., self.launched.to_string(), GOLD);
        s
    }
}
