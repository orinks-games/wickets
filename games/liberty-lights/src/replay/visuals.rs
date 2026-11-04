use super::*;
#[allow(unused_imports)]
use crate::visuals::{burst, draw_round};
use parlor_core::visuals::*;
impl Replay {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [14, 22, 45]);
        if let Some(r) = &self.round {
            draw_round(&mut s, r);
        }
        s
    }
}
