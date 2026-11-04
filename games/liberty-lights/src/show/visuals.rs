use super::*;
#[allow(unused_imports)]
use crate::visuals::{burst, draw_round};
use parlor_core::visuals::*;
impl Show {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [14, 22, 45]);
        draw_round(&mut s, &self.round);
        s
    }
}
